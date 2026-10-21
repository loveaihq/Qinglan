//! The scene builder: walks a styled SVG tree and flattens it into a [`Scene`] (SVG 2 painting 13, struct 5, coordinate
//! systems 8, pservers 13.2-13.3, masking 14.3 (`clipPath`), text 11 (simple)).
//!
//! Styles come from the CSS engine. The SVG-only presentation properties (`fill`, `stroke`, ...) are kept by the cascade as
//! custom-property-like entries under their own names (see `css::svg_props`), so an entry that differs from the parent's is a
//! declaration of the element itself; the builder resolves inheritance itself (`St`), which is also what makes `use`
//! instances inherit from the `use` element and not from where the referenced element sits.

use crate::fmath::Fclamp;
use std::rc::Rc;

use super::data::{self, Unit};
use super::*;
use crate::css::color::{self, ColorValue};
use crate::css::computed::{ComputedStyle, slant_of};
use crate::css::syntax;
use crate::css::types::{Display, Overflow, Visibility};
use crate::css::values::Lp;
use crate::html::atoms::{Atom, FxHashMap};
use crate::html::dom::{NONE, Namespace, NodeData};
use crate::vector::{Cap, Join, Pt};

/// Element visits per scene (a `use` bomb expands into visits, not memory).
pub const MAX_VISITS: u64 = 120_000;
/// Operations per scene.
pub const MAX_OPS: usize = 40_000;
/// Path segments per path and per scene.
pub const MAX_PATH_SEGS: usize = 250_000;
pub const MAX_SCENE_SEGS: usize = 600_000;
/// Nesting of elements plus `use` expansions.
pub const MAX_DEPTH: u32 = 96;
/// Gradient stops kept per gradient, gradients and clips per scene.
pub const MAX_STOPS: usize = 256;
const MAX_GRADS: usize = 4096;
const MAX_CLIPS: usize = 4096;
/// Opacity groups open at once.
const MAX_GROUPS: u32 = 8;

// The SVG-only properties, in the order of `PROPS`.
const P_FILL: usize = 0;
const P_STROKE: usize = 1;
const P_FILL_OP: usize = 2;
const P_STROKE_OP: usize = 3;
const P_FILL_RULE: usize = 4;
const P_CLIP_RULE: usize = 5;
const P_WIDTH: usize = 6;
const P_CAP: usize = 7;
const P_JOIN: usize = 8;
const P_MITER: usize = 9;
const P_DASH: usize = 10;
const P_DASH_OFF: usize = 11;
const P_ANCHOR: usize = 12;
const P_CLIP_PATH: usize = 13;
const P_STOP_COLOR: usize = 14;
const P_STOP_OP: usize = 15;

fn id_counter() -> u64 {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

/// `id` attribute → element, for a whole document, built when the first reference needs it.
#[derive(Default)]
pub struct Ids(Option<FxHashMap<Box<str>, NodeId>>);

impl Ids {
    pub fn get(&mut self, doc: &Document, id: &str) -> Option<NodeId> {
        // `url(#a%20b)` and `href="#%E2%9C%93"` name the element whose id is the decoded text.
        let decoded;
        let id = if id.contains('%') {
            decoded = String::from_utf8_lossy(&crate::url::percent_decode(id.as_bytes())).into_owned();
            decoded.as_str()
        } else {
            id
        };
        let map = self.0.get_or_insert_with(|| {
            let mut m: FxHashMap<Box<str>, NodeId> = FxHashMap::default();
            if let Some(idat) = doc.atoms.lookup("id") {
                for n in 0..doc.nodes.len() as NodeId {
                    if let Some(e) = doc.element(n)
                        && let Some(a) = e.attrs.iter().find(|a| a.ns == Namespace::None && a.name == idat)
                    {
                        m.entry(doc.value(a).into()).or_insert(n);
                    }
                }
            }
            m
        });
        map.get(id).copied()
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Spec {
    None,
    Color(Color),
    Current,
    Url(Rc<str>, Option<Rc<Spec>>),
}

/// The inherited presentation state while walking.
#[derive(Clone)]
struct St {
    fill: Rc<Spec>,
    stroke: Rc<Spec>,
    fill_op: f32,
    stroke_op: f32,
    width: f32,
    cap: Cap,
    join: Join,
    miter: f32,
    dash: Rc<Vec<f32>>,
    dash_off: f32,
    rule: FillRule,
    clip_rule: FillRule,
    color: Color,
    anchor: i8,
    /// The clip (1 + index) every descendant is drawn through.
    clip: u32,
}

#[derive(Clone, Copy)]
enum GradRef {
    Solid(Color),
    Index(u32),
}

struct B<'a> {
    doc: &'a Document,
    styles: &'a [Option<Rc<ComputedStyle>>],
    ids: &'a mut Ids,
    sc: Scene,
    props: [Option<Atom>; 23],
    visits: u64,
    segs: usize,
    groups: u32,
    /// Targets of the `use` elements being expanded (cycle check).
    active: Vec<NodeId>,
    grads: FxHashMap<NodeId, Option<GradRef>>,
    /// Text whitespace state: the last character written was a space.
    space: bool,
}

/// Build the scene of the `svg` element `root`. `ref_size` is what percentages refer to when the root has no viewBox (the
/// natural size when `None`).
pub fn scene(doc: &Document, styles: &[Option<Rc<ComputedStyle>>], root: NodeId, ids: &mut Ids, ref_size: Option<(f32, f32)>) -> Scene {
    let mut props = [None; 23];
    for (p, n) in props.iter_mut().zip(crate::css::svg_props::NAMES) {
        *p = doc.atoms.lookup(n);
    }
    let intr = intrinsic(doc, root);
    let sc = Scene { id: id_counter(), intrinsic: intr, outer: Transform::IDENTITY, ops: Vec::new(), grads: Vec::new(), clips: Vec::new(), links: Vec::new(), work: 0, complete: true };
    let mut b = B { doc, styles, ids, sc, props, visits: 0, segs: 0, groups: 0, active: Vec::new(), grads: FxHashMap::default(), space: true };
    if let Some(style) = styles.get(root as usize).and_then(Option::as_ref) {
        let vp = intr.vb.map_or_else(|| ref_size.unwrap_or_else(|| intr.object_size()), |v| (v[2], v[3]));
        let mut st = St {
            fill: Rc::new(Spec::Color(Color::BLACK)),
            stroke: Rc::new(Spec::None),
            fill_op: 1.0,
            stroke_op: 1.0,
            width: 1.0,
            cap: Cap::Butt,
            join: Join::Miter,
            miter: 4.0,
            dash: Rc::new(Vec::new()),
            dash_off: 0.0,
            rule: FillRule::NonZero,
            clip_rule: FillRule::NonZero,
            color: style.text.color,
            anchor: -1,
            clip: 0,
        };
        b.apply(&mut st, style, None, vp);
        // The `transform` attribute of the outermost svg (the CSS property is the layout box's business).
        if style.misc.fx.as_deref().is_none_or(|f| f.xform.is_empty()) {
            b.sc.outer = b.attr(root, "transform").and_then(data::transform).unwrap_or(Transform::IDENTITY);
        }
        if style.boxs.display != Display::None && !intr.zero_vb {
            b.children(root, style, &st, &Transform::IDENTITY, vp, 0);
        }
    }
    b.sc.work = b.visits + b.sc.ops.len() as u64;
    b.sc
}

fn mul255(a: u32, b: u32) -> u32 {
    (a * b + 127) / 255
}

fn alpha_of(op: f32) -> u8 {
    (op.fclamp(0.0, 1.0) * 255.0 + 0.5) as u8
}

#[inline(never)]
fn parse_paint(t: &str) -> Option<Spec> {
    let t = t.trim();
    if t.eq_ignore_ascii_case("none") {
        return Some(Spec::None);
    }
    if t.len() >= 4 && t.get(..4).is_some_and(|p| p.eq_ignore_ascii_case("url(")) {
        let close = t.find(')')?;
        let inner = t.get(4..close)?.trim().trim_matches(['"', '\'']);
        let id: Rc<str> = inner.strip_prefix('#')?.into();
        let rest = t.get(close + 1..).unwrap_or("").trim();
        let fb = if rest.is_empty() { None } else { parse_paint(rest).map(Rc::new) };
        return Some(Spec::Url(id, fb));
    }
    match color::parse_color(&syntax::parse_cvs(t))? {
        ColorValue::Color(c) => Some(Spec::Color(c)),
        ColorValue::CurrentColor => Some(Spec::Current),
    }
}

/// The id of a `url(#id)` reference.
fn url_id(t: &str) -> Option<&str> {
    let t = t.trim();
    let inner = t.get(..4).filter(|p| p.eq_ignore_ascii_case("url("))?;
    let _ = inner;
    let close = t.find(')')?;
    t.get(4..close)?.trim().trim_matches(['"', '\'']).strip_prefix('#')
}

fn opacity_value(t: &str) -> Option<f32> {
    let mut c = data::Cur::new(t.trim());
    let v = c.number()?;
    let v = if c.rest() == "%" { v / 100.0 } else if c.rest().is_empty() { v } else { return None };
    Some(v.fclamp(0.0, 1.0))
}

impl<'a> B<'a> {
    fn style(&self, n: NodeId) -> Option<&'a Rc<ComputedStyle>> {
        self.styles.get(n as usize).and_then(Option::as_ref)
    }

    #[inline(never)]
    fn attr(&self, n: NodeId, name: &str) -> Option<&'a str> {
        let doc = self.doc;
        let e = doc.element(n)?;
        e.attrs.iter().find(|a| a.ns == Namespace::None && doc.atoms.get(a.name) == name).map(|a| doc.value(a))
    }

    #[inline(never)]
    fn href(&self, n: NodeId) -> Option<&'a str> {
        let doc = self.doc;
        let e = doc.element(n)?;
        let find = |ns| e.attrs.iter().find(|a| a.ns == ns && doc.atoms.get(a.name) == "href").map(|a| doc.value(a));
        find(Namespace::None).or_else(|| find(Namespace::XLink))?.trim().strip_prefix('#')
    }

    #[inline(never)]
    fn name(&self, n: NodeId) -> &'a str {
        self.doc.element(n).map_or("", |e| self.doc.atoms.get(e.name))
    }

    fn is_svg(&self, n: NodeId) -> bool {
        self.doc.element(n).is_some_and(|e| e.ns == Namespace::Svg)
    }

    /// The element's own declaration of SVG property `p` (not what it inherits from `parent`).
    #[inline(never)]
    fn own(&self, style: &ComputedStyle, parent: Option<&ComputedStyle>, p: usize) -> Option<String> {
        let at = self.props.get(p).copied().flatten()?;
        let v = style.custom.get(&at)?;
        if parent.is_some_and(|ps| ps.custom.get(&at).is_some_and(|pv| Rc::ptr_eq(pv, v))) {
            return None;
        }
        Some(syntax::serialize_cvs(v))
    }

    /// A length attribute in user units: `%` of `refv`, `em` of the element's font size.
    #[inline(never)]
    fn len(&self, n: NodeId, name: &str, style: &ComputedStyle, refv: f32) -> Option<f32> {
        // The CSS property of the same name (SVG 2 geometry 10) wins over the attribute.
        let p = match name {
            "cx" => 16,
            "cy" => 17,
            "r" => 18,
            "rx" => 19,
            "ry" => 20,
            "x" => 21,
            "y" => 22,
            _ => 99,
        };
        let css = if p < 99 { self.own(style, self.style(self.doc.parent(n)).map(|s| &**s), p) } else { None };
        let (v, u) = data::length(match &css {
            Some(t) => t,
            None => self.attr(n, name)?,
        })?;
        Some(unit_px(v, u, style.font.size, refv))
    }

    #[inline(never)]
    fn apply(&self, st: &mut St, style: &ComputedStyle, parent: Option<&ComputedStyle>, vp: (f32, f32)) {
        if parent.is_none_or(|p| p.text.color != style.text.color) {
            st.color = style.text.color;
        }
        let diag = ((vp.0 * vp.0 + vp.1 * vp.1) / 2.0).sqrt();
        for p in 0..P_CLIP_PATH {
            let Some(t) = self.own(style, parent, p) else { continue };
            let t = t.trim();
            match p {
                P_FILL => {
                    if let Some(s) = parse_paint(t) {
                        st.fill = Rc::new(s);
                    }
                }
                P_STROKE => {
                    if let Some(s) = parse_paint(t) {
                        st.stroke = Rc::new(s);
                    }
                }
                P_FILL_OP => st.fill_op = opacity_value(t).unwrap_or(st.fill_op),
                P_STROKE_OP => st.stroke_op = opacity_value(t).unwrap_or(st.stroke_op),
                P_FILL_RULE | P_CLIP_RULE => {
                    let r = match t {
                        "evenodd" => Some(FillRule::EvenOdd),
                        "nonzero" => Some(FillRule::NonZero),
                        _ => None,
                    };
                    if let Some(r) = r {
                        if p == P_FILL_RULE {
                            st.rule = r;
                        } else {
                            st.clip_rule = r;
                        }
                    }
                }
                P_WIDTH => {
                    if let Some((v, u)) = data::length(t).filter(|(v, _)| *v >= 0.0) {
                        st.width = unit_px(v, u, style.font.size, diag);
                    }
                }
                P_CAP => {
                    st.cap = match t {
                        "round" => Cap::Round,
                        "square" => Cap::Square,
                        "butt" => Cap::Butt,
                        _ => st.cap,
                    }
                }
                P_JOIN => {
                    st.join = match t {
                        "round" => Join::Round,
                        "bevel" => Join::Bevel,
                        "miter" | "miter-clip" | "arcs" => Join::Miter,
                        _ => st.join,
                    }
                }
                P_MITER => {
                    if let Some(v) = data::numbers(t, 1).and_then(|v| v.first().copied()).filter(|v| *v >= 1.0) {
                        st.miter = v;
                    }
                }
                P_DASH => {
                    if t == "none" {
                        st.dash = Rc::new(Vec::new());
                    } else {
                        let list: Option<Vec<f32>> = t
                            .split([',', ' ', '\t', '\n'])
                            .filter(|s| !s.is_empty())
                            .take(64)
                            .map(|s| data::length(s).filter(|(v, _)| *v >= 0.0).map(|(v, u)| unit_px(v, u, style.font.size, diag)))
                            .collect();
                        if let Some(l) = list {
                            st.dash = Rc::new(l);
                        }
                    }
                }
                P_DASH_OFF => {
                    if let Some((v, u)) = data::length(t) {
                        st.dash_off = unit_px(v, u, style.font.size, diag);
                    }
                }
                P_ANCHOR => {
                    st.anchor = match t {
                        "start" => -1,
                        "middle" => 0,
                        "end" => 1,
                        _ => st.anchor,
                    }
                }
                _ => {}
            }
        }
    }

    fn tick(&mut self) -> bool {
        self.visits += 1;
        if self.visits > MAX_VISITS || self.sc.ops.len() >= MAX_OPS {
            self.sc.complete = false;
            return false;
        }
        true
    }

    fn push(&mut self, op: Op) {
        if self.sc.ops.len() < MAX_OPS {
            self.sc.ops.push(op);
        } else {
            self.sc.complete = false;
        }
    }

    fn children(&mut self, n: NodeId, style: &Rc<ComputedStyle>, st: &St, ctm: &Transform, vp: (f32, f32), depth: u32) {
        let doc = self.doc;
        for c in doc.children(n) {
            if self.visits > MAX_VISITS {
                break;
            }
            self.elem(c, Some(style), st, ctm, vp, depth + 1);
        }
    }

    /// The transform of an element: the CSS `transform` property (about `transform-origin`, which is `0 0` in SVG unless
    /// set), else the `transform` attribute.
    #[inline(never)]
    fn xform(&self, n: NodeId, style: &ComputedStyle, vp: (f32, f32)) -> Transform {
        if let Some(fx) = style.misc.fx.as_deref().filter(|f| !f.xform.is_empty()) {
            let mut f = fx.clone();
            if f.origin == crate::css::fx::Fx::initial().origin {
                f.origin = (Lp::px(0.0), Lp::px(0.0));
            }
            return f.matrix(vp.0, vp.1);
        }
        self.attr(n, "transform").and_then(data::transform).unwrap_or(Transform::IDENTITY)
    }

    fn elem(&mut self, n: NodeId, pstyle: Option<&Rc<ComputedStyle>>, st: &St, ctm: &Transform, vp: (f32, f32), depth: u32) {
        if depth > MAX_DEPTH || !self.is_svg(n) || !self.tick() {
            return;
        }
        let Some(style) = self.style(n) else { return };
        if style.boxs.display == Display::None {
            return;
        }
        let name = self.name(n);
        let kind = match name {
            "g" | "a" | "svg" | "use" | "switch" | "text" => 1,
            "path" | "rect" | "circle" | "ellipse" | "line" | "polyline" | "polygon" => 2,
            _ => return,
        };
        let parent = pstyle.map(|p| &**p);
        let mut s2 = st.clone();
        self.apply(&mut s2, style, parent, vp);
        let ctm2 = ctm.compose(&self.xform(n, style, vp));
        if !ctm2.a.is_finite() || !ctm2.d.is_finite() || !ctm2.e.is_finite() || !ctm2.f.is_finite() {
            return;
        }
        if let Some(c) = self.clip_of(n, style, parent, &ctm2, s2.clip, vp, 0) {
            s2.clip = c;
        }
        let op = style.misc.fx.as_deref().map_or(255, |f| f.opacity);
        // One paint can take the element's opacity into its alpha; two overlapping ones need a layer.
        let single = (*s2.fill == Spec::None) || (*s2.stroke == Spec::None);
        let group = op < 255 && (kind == 1 || !single) && self.groups < MAX_GROUPS;
        if group {
            self.groups += 1;
            self.push(Op::Begin(op));
        }
        let fold = if group { 255 } else { op };
        let ops_before = self.sc.ops.len();
        let is_link = name == "a";
        match name {
            "g" | "a" => self.children(n, style, &s2, &ctm2, vp, depth),
            "switch" => {
                let first = self.doc.children(n).find(|c| self.is_svg(*c) && self.style(*c).is_some_and(|s| s.boxs.display != Display::None));
                if let Some(c) = first {
                    self.elem(c, Some(style), &s2, &ctm2, vp, depth + 1);
                }
            }
            "svg" => self.nested(n, n, style, &s2, ctm, &ctm2, vp, depth, None),
            "use" => self.use_elem(n, style, &s2, &ctm2, vp, depth),
            "text" => self.text(n, style, &s2, &ctm2, vp),
            _ => {
                let path = self.shape_path(n, name, style, vp);
                // `pathLength` scales dash lengths: real units per author unit (SVG 2 paths 9.? "pathLength").
                let pl = self.attr(n, "pathLength").and_then(|t| data::numbers(t, 1)).and_then(|v| v.first().copied()).filter(|v| *v >= 0.0);
                // `pathLength="0"` is no dashing at all (the line is solid).
                if pl == Some(0.0) {
                    s2.dash = Rc::new(Vec::new());
                }
                let scale = pl.filter(|v| *v > 0.0).map_or(1.0, |pl| {
                    let len: f32 = path
                        .flatten(0.02)
                        .iter()
                        .map(|p| {
                            let open: f32 = p.pts.windows(2).map(|w| if let [a, b] = w { (b.x - a.x).hypot(b.y - a.y) } else { 0.0 }).sum();
                            match (p.closed, p.pts.first(), p.pts.last()) {
                                (true, Some(a), Some(b)) => open + (b.x - a.x).hypot(b.y - a.y),
                                _ => open,
                            }
                        })
                        .sum();
                    len / pl
                });
                if scale != 1.0 {
                    s2.dash = Rc::new(s2.dash.iter().map(|d| d * scale).collect());
                    s2.dash_off *= scale;
                }
                self.shape(path, &ctm2, &s2, style, fold);
            }
        }
        if is_link && self.sc.links.len() < 4096 {
            self.link(n, ops_before);
        }
        if group {
            self.push(Op::End);
            self.groups -= 1;
        }
    }

    /// The union of the boxes of the operations since `from`, as a link target.
    #[inline(never)]
    fn link(&mut self, n: NodeId, from: usize) {
        let mut bb: Option<[f32; 4]> = None;
        for op in self.sc.ops.iter().skip(from) {
            let r = match op {
                Op::Shape(s) => path_bounds(&s.path, &s.ctm),
                Op::Text(t) => {
                    let size = t.runs.first().map_or(16.0, |r| r.size);
                    let w: f32 = t.runs.iter().map(|r| r.text.chars().count() as f32 * r.size * 0.6).sum();
                    let x = t.x - w * (t.anchor + 1) as f32 / 2.0;
                    let p = t.ctm.apply(Pt::new(x, t.y - size));
                    Some([p.x, p.y, w * t.ctm.a.abs(), size * 1.2 * t.ctm.d.abs()])
                }
                _ => None,
            };
            if let Some(r) = r {
                bb = Some(match bb {
                    None => r,
                    Some(b) => {
                        let (x0, y0) = (b[0].min(r[0]), b[1].min(r[1]));
                        [x0, y0, (b[0] + b[2]).max(r[0] + r[2]) - x0, (b[1] + b[3]).max(r[1] + r[3]) - y0]
                    }
                });
            }
        }
        if let Some(bbox) = bb {
            self.sc.links.push(Link { node: n, bbox });
        }
    }

    // ---- shapes ----------------------------------------------------------------------------------

    #[inline(never)]
    fn shape_path(&mut self, n: NodeId, name: &str, style: &ComputedStyle, vp: (f32, f32)) -> Path {
        let l = |s: &Self, a: &str, r: f32| s.len(n, a, style, r).unwrap_or(0.0);
        let (x, y) = (l(self, "x", vp.0), l(self, "y", vp.1));
        match name {
            "path" => {
                let (p, _) = data::path(self.attr(n, "d").unwrap_or(""), MAX_PATH_SEGS.min(MAX_SCENE_SEGS.saturating_sub(self.segs)));
                self.segs += p.segs.len();
                p
            }
            "rect" => {
                let (w, h) = (l(self, "width", vp.0), l(self, "height", vp.1));
                if w <= 0.0 || h <= 0.0 {
                    return Path::new();
                }
                // SVG 2 geometry 10.2: an unset radius takes the other one; both are clamped to half the side.
                let (rx, ry) = (self.len(n, "rx", style, vp.0).filter(|v| *v >= 0.0), self.len(n, "ry", style, vp.1).filter(|v| *v >= 0.0));
                let (rx, ry) = match (rx, ry) {
                    (None, None) => (0.0, 0.0),
                    (Some(a), None) => (a, a),
                    (None, Some(b)) => (b, b),
                    (Some(a), Some(b)) => (a, b),
                };
                let (rx, ry) = (rx.min(w / 2.0), ry.min(h / 2.0));
                if rx > 0.0 && ry > 0.0 { Path::rounded_rect(x, y, w, h, [(rx, ry); 4]) } else { Path::rect(x, y, w, h) }
            }
            "circle" => {
                let r = self.len(n, "r", style, ((vp.0 * vp.0 + vp.1 * vp.1) / 2.0).sqrt()).unwrap_or(0.0);
                if r > 0.0 { ellipse_path(l(self, "cx", vp.0), l(self, "cy", vp.1), r, r) } else { Path::new() }
            }
            "ellipse" => {
                let (rx, ry) = (self.len(n, "rx", style, vp.0), self.len(n, "ry", style, vp.1));
                let (rx, ry) = match (rx, ry) {
                    (Some(a), Some(b)) => (a, b),
                    (Some(a), None) | (None, Some(a)) => (a, a),
                    _ => (0.0, 0.0),
                };
                if rx > 0.0 && ry > 0.0 { ellipse_path(l(self, "cx", vp.0), l(self, "cy", vp.1), rx, ry) } else { Path::new() }
            }
            "line" => {
                let mut p = Path::new();
                p.move_to(l(self, "x1", vp.0), l(self, "y1", vp.1));
                p.line_to(l(self, "x2", vp.0), l(self, "y2", vp.1));
                p
            }
            _ => {
                // polyline / polygon: pairs of numbers; an odd trailing number is dropped (SVG 2 shapes 10.5).
                let pts = data::numbers(self.attr(n, "points").unwrap_or(""), MAX_PATH_SEGS * 2).unwrap_or_default();
                let mut p = Path::new();
                for (i, xy) in pts.as_chunks::<2>().0.iter().enumerate() {
                    let (a, b) = (xy.first().copied().unwrap_or(0.0), xy.get(1).copied().unwrap_or(0.0));
                    if i == 0 {
                        p.move_to(a, b);
                    } else {
                        p.line_to(a, b);
                    }
                }
                if name == "polygon" && !p.is_empty() {
                    p.close();
                }
                self.segs += p.segs.len();
                p
            }
        }
    }

    #[inline(never)]
    fn resolve(&mut self, spec: &Spec, st: &St, op: f32, fold: u8) -> Option<Fill> {
        let alpha = mul255(u32::from(alpha_of(op)), u32::from(fold)) as u8;
        match spec {
            Spec::None => None,
            Spec::Color(c) => Some(Fill { paint: Paint::Color(*c), alpha }),
            Spec::Current => Some(Fill { paint: Paint::Color(st.color), alpha }),
            Spec::Url(id, fb) => {
                let node = self.ids.get(self.doc, id);
                match node.and_then(|n| self.gradient(n, 0)) {
                    Some(GradRef::Solid(c)) => Some(Fill { paint: Paint::Color(c), alpha }),
                    Some(GradRef::Index(i)) => Some(Fill { paint: Paint::Grad(i), alpha }),
                    None => match fb {
                        Some(f) => self.resolve(&f.clone(), st, op, fold),
                        None => None,
                    },
                }
            }
        }
    }

    fn shape(&mut self, path: Path, ctm: &Transform, st: &St, style: &ComputedStyle, fold: u8) {
        if path.is_empty() || style.text.visibility != Visibility::Visible {
            return;
        }
        let fill = self.resolve(&st.fill.clone(), st, st.fill_op, fold);
        let stroke = if st.width > 0.0 {
            self.resolve(&st.stroke.clone(), st, st.stroke_op, fold).map(|f| {
                let mut s = Stroke::new(st.width);
                (s.cap, s.join, s.miter_limit, s.dashes, s.dash_offset) = (st.cap, st.join, st.miter, (*st.dash).clone(), st.dash_off);
                (f, s)
            })
        } else {
            None
        };
        if fill.is_none() && stroke.is_none() {
            return;
        }
        self.push(Op::Shape(Box::new(Shape { path, ctm: *ctm, fill, stroke, rule: st.rule, clip: st.clip })));
    }

    // ---- viewports and use -----------------------------------------------------------------------

    /// A nested `svg` (or a `symbol` instance): establish its viewport at (`x`, `y`) with the size from the element or the `use`
    /// that instantiates it, map its viewBox, clip, and draw its children. `ctm` is the parent's, `ctm2` the parent's with
    /// the element's own transform.
    #[allow(clippy::too_many_arguments)]
    fn nested(&mut self, n: NodeId, from: NodeId, style: &Rc<ComputedStyle>, st: &St, _ctm: &Transform, ctm2: &Transform, vp: (f32, f32), depth: u32, size: Option<(Option<f32>, Option<f32>)>) {
        let symbol = self.name(n) == "symbol";
        let (x, y) = if symbol { (0.0, 0.0) } else { (self.len(n, "x", style, vp.0).unwrap_or(0.0), self.len(n, "y", style, vp.1).unwrap_or(0.0)) };
        let w = size.and_then(|s| s.0).or_else(|| self.len(n, "width", style, vp.0)).unwrap_or(vp.0);
        let h = size.and_then(|s| s.1).or_else(|| self.len(n, "height", style, vp.1)).unwrap_or(vp.1);
        if w <= 0.0 || h <= 0.0 {
            return;
        }
        let _ = from;
        let nums = self.attr(n, "viewBox").and_then(|s| data::numbers(s, 5));
        // A viewBox with a zero width or height turns the element's rendering off (SVG 2 coordinate-systems 8.2).
        if matches!(nums.as_deref(), Some(&[_, _, c, d]) if (c == 0.0 || d == 0.0) && c >= 0.0 && d >= 0.0) {
            return;
        }
        let vb = nums.and_then(|v| match v.as_slice() {
            &[a, b, c, d] if c > 0.0 && d > 0.0 => Some([a, b, c, d]),
            _ => None,
        });
        let par = self.attr(n, "preserveAspectRatio").map_or(Par::DEFAULT, Par::parse);
        let inner = match vb {
            Some(v) => view_transform(v, par, w, h),
            None => Transform::IDENTITY,
        };
        let base = ctm2.compose(&Transform::translate(x, y));
        let mut s2 = st.clone();
        // overflow other than visible (the UA sheet makes it hidden) clips to the viewport.
        if style.boxs.overflow_x != Overflow::Visible && self.sc.clips.len() < MAX_CLIPS {
            let part = (Path::rect(0.0, 0.0, w, h), base, FillRule::NonZero);
            self.sc.clips.push(Clip { parts: vec![part], next: st.clip });
            s2.clip = self.sc.clips.len() as u32;
        }
        let vp2 = vb.map_or((w, h), |v| (v[2], v[3]));
        self.children(n, style, &s2, &base.compose(&inner), vp2, depth);
    }

    fn use_elem(&mut self, n: NodeId, style: &Rc<ComputedStyle>, st: &St, ctm2: &Transform, vp: (f32, f32), depth: u32) {
        let Some(id) = self.href(n) else { return };
        let Some(target) = self.ids.get(self.doc, id) else { return };
        if self.active.contains(&target) || self.active.len() as u32 >= 24 || !self.is_svg(target) {
            return;
        }
        let Some(tstyle) = self.style(target).cloned() else { return };
        if tstyle.boxs.display == Display::None {
            return;
        }
        let (x, y) = (self.len(n, "x", style, vp.0).unwrap_or(0.0), self.len(n, "y", style, vp.1).unwrap_or(0.0));
        let (w, h) = (self.len(n, "width", style, vp.0), self.len(n, "height", style, vp.1));
        self.active.push(target);
        let tname = self.name(target);
        let parent = self.doc.parent(target);
        let pstyle = self.style(parent).cloned();
        match tname {
            "symbol" | "svg" => {
                let mut s2 = st.clone();
                self.apply(&mut s2, &tstyle, pstyle.as_deref(), vp);
                let t = ctm2.compose(&Transform::translate(x, y));
                self.nested(target, n, &tstyle, &s2, &t, &t, vp, depth + 1, Some((w, h)));
            }
            _ => {
                let t = ctm2.compose(&Transform::translate(x, y));
                self.elem(target, pstyle.as_ref(), st, &t, vp, depth + 1);
            }
        }
        self.active.pop();
    }

    // ---- clip paths ------------------------------------------------------------------------------

    /// The clip made by `clip-path: url(#id)` on `n` (1 + index), nested in `outer`.
    #[allow(clippy::too_many_arguments)]
    #[inline(never)]
    fn clip_of(&mut self, n: NodeId, style: &ComputedStyle, parent: Option<&ComputedStyle>, ctm: &Transform, outer: u32, vp: (f32, f32), depth: u32) -> Option<u32> {
        let t = self.own(style, parent, P_CLIP_PATH)?;
        let id = url_id(&t)?;
        let cp = self.ids.get(self.doc, id)?;
        if self.name(cp) != "clipPath" || depth > 4 || self.sc.clips.len() >= MAX_CLIPS || self.active.contains(&cp) {
            return None;
        }
        let cstyle = self.style(cp)?.clone();
        self.active.push(cp);
        let mut m = *ctm;
        if self.attr(cp, "clipPathUnits") == Some("objectBoundingBox") {
            let bb = self.bbox(n, vp, 0);
            match bb {
                Some(b) if b[2] > 0.0 && b[3] > 0.0 => m = m.compose(&Transform { a: b[2], b: 0.0, c: 0.0, d: b[3], e: b[0], f: b[1] }),
                _ => {
                    self.active.pop();
                    return None;
                }
            }
        }
        m = m.compose(&self.xform(cp, &cstyle, vp));
        // A clipPath's own clip-path nests below it.
        let cparent = self.style(self.doc.parent(cp)).cloned();
        let base = self.clip_of(cp, &cstyle, cparent.as_deref(), ctm, outer, vp, depth + 1).unwrap_or(outer);
        let crule = self.own(&cstyle, None, P_CLIP_RULE).map_or(FillRule::NonZero, |t| rule_of(&t));
        let mut parts = Vec::new();
        for c in self.doc.children(cp) {
            if !self.is_svg(c) {
                continue;
            }
            let Some(s) = self.style(c).cloned() else { continue };
            if s.boxs.display == Display::None {
                continue;
            }
            let (target, tstyle, extra) = if self.name(c) == "use" {
                let Some(t) = self.href(c).and_then(|i| self.ids.get(self.doc, i)) else { continue };
                let Some(ts) = self.style(t).cloned() else { continue };
                let (x, y) = (self.len(c, "x", &s, vp.0).unwrap_or(0.0), self.len(c, "y", &s, vp.1).unwrap_or(0.0));
                (t, ts, self.xform(c, &s, vp).compose(&Transform::translate(x, y)))
            } else {
                (c, s.clone(), Transform::IDENTITY)
            };
            let name = self.name(target);
            if !matches!(name, "path" | "rect" | "circle" | "ellipse" | "line" | "polyline" | "polygon") {
                continue;
            }
            let path = self.shape_path(target, name, &tstyle, vp);
            let rule = self.own(&tstyle, None, P_CLIP_RULE).map_or(crule, |t| rule_of(&t));
            let t = m.compose(&extra).compose(&self.xform(target, &tstyle, vp));
            parts.push((path, t, rule));
            if parts.len() >= 256 {
                break;
            }
        }
        self.active.pop();
        self.sc.clips.push(Clip { parts, next: base });
        Some(self.sc.clips.len() as u32)
    }

    /// The bounding box (x, y, w, h) of a shape or a group of shapes in the element's own user space.
    #[inline(never)]
    fn bbox(&mut self, n: NodeId, vp: (f32, f32), depth: u32) -> Option<[f32; 4]> {
        let style = self.style(n)?.clone();
        let name = self.name(n);
        if matches!(name, "path" | "rect" | "circle" | "ellipse" | "line" | "polyline" | "polygon") {
            let p = self.shape_path(n, name, &style, vp);
            return path_bounds(&p, &Transform::IDENTITY);
        }
        if name != "g" || depth > 6 {
            return None;
        }
        let mut bb: Option<[f32; 4]> = None;
        let kids: Vec<NodeId> = self.doc.children(n).filter(|c| self.is_svg(*c)).take(256).collect();
        for c in kids {
            let Some(cs) = self.style(c).cloned() else { continue };
            let Some(r) = self.bbox(c, vp, depth + 1) else { continue };
            let t = self.xform(c, &cs, vp);
            let (p0, p1) = (t.apply(Pt::new(r[0], r[1])), t.apply(Pt::new(r[0] + r[2], r[1] + r[3])));
            let r = [p0.x.min(p1.x), p0.y.min(p1.y), (p1.x - p0.x).abs(), (p1.y - p0.y).abs()];
            bb = Some(match bb {
                None => r,
                Some(b) => {
                    let (x0, y0) = (b[0].min(r[0]), b[1].min(r[1]));
                    [x0, y0, (b[0] + b[2]).max(r[0] + r[2]) - x0, (b[1] + b[3]).max(r[1] + r[3]) - y0]
                }
            });
        }
        bb
    }

    // ---- gradients -------------------------------------------------------------------------------

    /// The attribute `name` of gradient `n` or, when it has none, of the gradients its `href` chain reaches.
    fn grad_attr(&self, n: NodeId, name: &str, same_kind: bool) -> Option<&'a str> {
        let kind = self.name(n);
        let mut cur = n;
        for _ in 0..8 {
            // An empty `gradientTransform` is as good as none, so the one of the gradient `href` points at is inherited.
            if let Some(v) = self.attr(cur, name).filter(|v| name != "gradientTransform" || !v.trim().is_empty()) {
                return Some(v);
            }
            let next = self.href(cur)?;
            let t = self.ids_peek(next)?;
            let tn = self.name(t);
            if !matches!(tn, "linearGradient" | "radialGradient") || (same_kind && tn != kind && !matches!(name, "gradientUnits" | "gradientTransform" | "spreadMethod")) {
                return None;
            }
            cur = t;
        }
        None
    }

    /// `Ids::get` for code that cannot borrow `self` mutably (the map is built by the first call).
    fn ids_peek(&self, id: &str) -> Option<NodeId> {
        self.ids.0.as_ref().and_then(|m| m.get(id).copied())
    }

    fn gradient(&mut self, n: NodeId, depth: u32) -> Option<GradRef> {
        if let Some(r) = self.grads.get(&n) {
            return *r;
        }
        let r = self.build_gradient(n, depth);
        self.grads.insert(n, r);
        r
    }

    #[inline(never)]
    fn build_gradient(&mut self, n: NodeId, _depth: u32) -> Option<GradRef> {
        let radial = match self.name(n) {
            "linearGradient" => false,
            "radialGradient" => true,
            _ => return None,
        };
        if self.sc.grads.len() >= MAX_GRADS {
            return None;
        }
        // The stops come from the first gradient of the chain that has any.
        let mut stops: Vec<(f32, Color)> = Vec::new();
        let mut cur = n;
        for _ in 0..8 {
            for c in self.doc.children(cur) {
                if self.name(c) != "stop" || !self.is_svg(c) || stops.len() >= MAX_STOPS {
                    continue;
                }
                let Some(style) = self.style(c).cloned() else { continue };
                let pstyle = self.style(cur).cloned();
                let off = self.attr(c, "offset").and_then(|t| {
                    let mut k = data::Cur::new(t.trim());
                    let v = k.number()?;
                    Some(if k.rest() == "%" { v / 100.0 } else { v })
                });
                let off = off.unwrap_or(0.0).fclamp(0.0, 1.0).max(stops.last().map_or(0.0, |s| s.0));
                let col = match self.own(&style, pstyle.as_deref(), P_STOP_COLOR).and_then(|t| parse_paint(&t)) {
                    Some(Spec::Color(c)) => c,
                    Some(Spec::Current) => style.text.color,
                    _ => Color::BLACK,
                };
                let op = self.own(&style, pstyle.as_deref(), P_STOP_OP).and_then(|t| opacity_value(&t)).unwrap_or(1.0);
                let a = mul255(u32::from(col.a), u32::from(alpha_of(op))) as u8;
                stops.push((off, Color { a, ..col }));
            }
            if !stops.is_empty() {
                break;
            }
            let Some(next) = self.href(cur).and_then(|i| self.ids.get(self.doc, i)) else { break };
            if !matches!(self.name(next), "linearGradient" | "radialGradient") || next == n {
                break;
            }
            cur = next;
        }
        match stops.len() {
            0 => return None,
            1 => return stops.first().map(|s| GradRef::Solid(s.1)),
            _ => {}
        }
        let bbox = self.grad_attr(n, "gradientUnits", false) != Some("userSpaceOnUse");
        let (vw, vh) = self.ref_vp();
        let coord = |s: &Self, name: &str, def: f32, axis: u8| -> f32 {
            let Some((v, u)) = s.grad_attr(n, name, true).and_then(data::length) else { return def };
            match (u, bbox) {
                (Unit::Pct, true) => v / 100.0,
                (Unit::Pct, false) => v * [vw, vh, ((vw * vw + vh * vh) / 2.0).sqrt()].get(usize::from(axis)).copied().unwrap_or(vw) / 100.0,
                (u, _) => unit_px(v, u, 16.0, 0.0),
            }
        };
        let d = |s: f32| if bbox { s } else { s * vw };
        let p = if radial {
            let (cx, cy) = (coord(self, "cx", if bbox { 0.5 } else { d(0.5) }, 0), coord(self, "cy", if bbox { 0.5 } else { vh * 0.5 }, 1));
            let r = coord(self, "r", if bbox { 0.5 } else { ((vw * vw + vh * vh) / 2.0).sqrt() * 0.5 }, 2);
            let (fx, fy) = (coord(self, "fx", cx, 0), coord(self, "fy", cy, 1));
            let fr = coord(self, "fr", 0.0, 2);
            [cx, cy, r, fx, fy, fr]
        } else {
            [coord(self, "x1", 0.0, 0), coord(self, "y1", 0.0, 1), coord(self, "x2", if bbox { 1.0 } else { vw }, 0), coord(self, "y2", 0.0, 1), 0.0, 0.0]
        };
        let spread = match self.grad_attr(n, "spreadMethod", false) {
            Some("reflect") => 1,
            Some("repeat") => 2,
            _ => 0,
        };
        let xf = self.grad_attr(n, "gradientTransform", false).and_then(data::transform).unwrap_or(Transform::IDENTITY);
        // A transform that cannot be inverted makes the paint server invalid: the fallback is used.
        if !xf.inverse().a.is_finite() {
            return None;
        }
        self.sc.grads.push(Grad { radial, bbox, xf, spread, p, stops });
        Some(GradRef::Index(self.sc.grads.len() as u32 - 1))
    }

    fn ref_vp(&self) -> (f32, f32) {
        self.sc.intrinsic.vb.map_or_else(|| self.sc.intrinsic.object_size(), |v| (v[2], v[3]))
    }

    // ---- text ------------------------------------------------------------------------------------

    fn text(&mut self, n: NodeId, style: &Rc<ComputedStyle>, st: &St, ctm: &Transform, vp: (f32, f32)) {
        let pos = |s: &Self, node: NodeId, name: &str, r: f32, sty: &ComputedStyle| -> Option<f32> {
            let first = s.attr(node, name)?.split([',', ' ']).find(|t| !t.is_empty())?;
            let (v, u) = data::length(first)?;
            Some(unit_px(v, u, sty.font.size, r))
        };
        let (x, y) = (pos(self, n, "x", vp.0, style).unwrap_or(0.0), pos(self, n, "y", vp.1, style).unwrap_or(0.0));
        let mut chunks: Vec<TextOp> = Vec::new();
        let mut cur = TextOp { x, y, ctm: *ctm, anchor: st.anchor, runs: Vec::new(), clip: st.clip };
        self.space = true;
        self.text_run(n, style, st, vp, &mut cur, &mut chunks, 0);
        if let Some(r) = cur.runs.last_mut()
            && r.text.ends_with(' ')
        {
            r.text.pop();
        }
        chunks.push(cur);
        for c in chunks {
            if c.runs.iter().any(|r| !r.text.is_empty()) {
                self.push(Op::Text(Box::new(c)));
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    #[inline(never)]
    fn text_run(&mut self, n: NodeId, style: &Rc<ComputedStyle>, st: &St, vp: (f32, f32), cur: &mut TextOp, chunks: &mut Vec<TextOp>, depth: u32) {
        if depth > 16 {
            return;
        }
        let doc = self.doc;
        let (mut dx, mut dy) = (0.0f32, 0.0f32);
        if depth > 0 {
            let first = |s: &Self, name: &str, r: f32| s.attr(n, name).and_then(|v| v.split([',', ' ']).find(|t| !t.is_empty())).and_then(data::length).map(|(v, u)| unit_px(v, u, style.font.size, r));
            let (ax, ay) = (first(self, "x", vp.0), first(self, "y", vp.1));
            if ax.is_some() || ay.is_some() {
                let mut next = TextOp { x: ax.unwrap_or(cur.x), y: ay.unwrap_or(cur.y), ctm: cur.ctm, anchor: st.anchor, runs: Vec::new(), clip: cur.clip };
                std::mem::swap(cur, &mut next);
                chunks.push(next);
            }
            dx = first(self, "dx", vp.0).unwrap_or(0.0);
            dy = first(self, "dy", vp.1).unwrap_or(0.0);
        }
        let fill = self.resolve(&st.fill.clone(), st, st.fill_op, 255);
        let mut pending = (dx, dy);
        for c in doc.children(n) {
            match doc.node(c).map(|x| &x.data) {
                Some(NodeData::Text(t)) => {
                    let Some(fill) = fill else { continue };
                    if style.text.visibility != Visibility::Visible {
                        continue;
                    }
                    let mut s = String::new();
                    for ch in t.chars() {
                        let ch = if matches!(ch, '\n' | '\t' | '\r') { ' ' } else { ch };
                        if ch == ' ' {
                            if self.space {
                                continue;
                            }
                            self.space = true;
                        } else {
                            self.space = false;
                        }
                        s.push(ch);
                    }
                    if s.is_empty() || cur.runs.iter().map(|r| r.text.len()).sum::<usize>() > 100_000 {
                        continue;
                    }
                    let f = &style.font;
                    cur.runs.push(TextRun { text: s, fill, size: f.size, weight: f.weight, slant: slant_of(f.style), families: f.family.clone(), dx: pending.0, dy: pending.1 });
                    pending = (0.0, 0.0);
                }
                Some(NodeData::Element(_)) if self.is_svg(c) && matches!(self.name(c), "tspan" | "a") => {
                    let Some(cs) = self.style(c).cloned() else { continue };
                    if cs.boxs.display == Display::None || !self.tick() {
                        continue;
                    }
                    let mut s2 = st.clone();
                    self.apply(&mut s2, &cs, Some(style), vp);
                    self.text_run(c, &cs, &s2, vp, cur, chunks, depth + 1);
                }
                _ => {}
            }
        }
    }
}

/// The path of a circle or ellipse: from (cx + rx, cy), clockwise, four arcs (SVG 2 shapes 10.3, 10.4: where the dashes start).
fn ellipse_path(cx: f32, cy: f32, rx: f32, ry: f32) -> Path {
    let k = crate::vector::KAPPA;
    let mut p = Path::new();
    p.move_to(cx + rx, cy);
    p.cubic_to(cx + rx, cy + k * ry, cx + k * rx, cy + ry, cx, cy + ry);
    p.cubic_to(cx - k * rx, cy + ry, cx - rx, cy + k * ry, cx - rx, cy);
    p.cubic_to(cx - rx, cy - k * ry, cx - k * rx, cy - ry, cx, cy - ry);
    p.cubic_to(cx + k * rx, cy - ry, cx + rx, cy - k * ry, cx + rx, cy);
    p.close();
    p
}

fn rule_of(t: &str) -> FillRule {
    if t.trim() == "evenodd" { FillRule::EvenOdd } else { FillRule::NonZero }
}

fn unit_px(v: f32, u: Unit, font: f32, refv: f32) -> f32 {
    match u {
        Unit::Px => v,
        Unit::Pct => v * refv / 100.0,
        Unit::Em => v * font,
        Unit::Ex => v * font / 2.0,
    }
}

/// The box (x, y, w, h) of a path's points after `t`; control points count, so curves may come out a little large.
pub(super) fn path_bounds(p: &Path, t: &Transform) -> Option<[f32; 4]> {
    use crate::vector::Seg;
    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    let mut add = |q: &Pt| {
        let q = t.apply(*q);
        if q.x.is_finite() && q.y.is_finite() {
            (x0, y0, x1, y1) = (x0.min(q.x), y0.min(q.y), x1.max(q.x), y1.max(q.y));
        }
    };
    for s in &p.segs {
        match s {
            Seg::Move(a) | Seg::Line(a) => add(a),
            Seg::Quad(a, b) => {
                add(a);
                add(b);
            }
            Seg::Cubic(a, b, c) => {
                add(a);
                add(b);
                add(c);
            }
            Seg::Close => {}
        }
    }
    (x0 <= x1).then_some([x0, y0, x1 - x0, y1 - y0])
}

#[allow(dead_code)]
fn _unused() -> NodeId {
    NONE
}
