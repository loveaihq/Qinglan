//! Backgrounds (css-backgrounds-3 3), gradients (css-images-3 3, css-images-4 3), `border-radius` (css-backgrounds-3 5)
//! and `outline` (css-ui-3 6): the value types and their parsers.
//!
//! Choices (decisions.md, 2c-1): `background-attachment` is accepted and ignored (`fixed` and `local` paint as `scroll`);
//! `background-clip: text` is parsed and painted as `border-box`; the `in <color-space>` part of gradients is accepted and
//! ignored (every gradient interpolates in premultiplied sRGB, the rule for the legacy color syntaxes, CSS Color 4 12.3);
//! vendor-prefixed gradients and `image-set()` are invalid.

use std::rc::Rc;

use super::color::{ColorValue, parse_color_cv};
use super::parse_prop::{ParseEnv, p_lp};
use super::syntax::{Cv, split_commas, trim_ws};
use super::tokenizer::Token;
use super::types::{PropId, Value};
use super::values::{Calc, LenCtx, Lp, parse_length_percentage};

/// At most this many layers are kept per element, and stops per gradient.
pub const MAX_LAYERS: usize = 16;
pub const MAX_STOPS: usize = 64;

/// One axis of a `<position>`: `pct` percent of (container - object) plus `off` (a length, or a percentage of that same
/// difference). Edge offsets are folded in: `right 10px` is 100% and -10px (css-backgrounds-3 3.6).
#[derive(Clone, Debug, PartialEq)]
pub struct Pos1 {
    pub pct: f32,
    pub off: Lp,
}

impl Pos1 {
    pub const ZERO: Pos1 = Pos1 { pct: 0.0, off: Lp::ZERO };
    pub const CENTER: Pos1 = Pos1 { pct: 50.0, off: Lp::ZERO };

    pub fn compute(&self, ctx: &LenCtx) -> Pos1 {
        Pos1 { pct: self.pct, off: self.off.compute(ctx) }
    }

    /// Where the object starts along an axis of length `area`, for an object of length `size`.
    pub fn resolve(&self, area: f32, size: f32) -> f32 {
        let d = area - size;
        d * self.pct / 100.0 + self.off.resolve(d)
    }

    pub fn to_css(&self) -> String {
        if self.off == Lp::ZERO { format!("{}%", super::values::fmt_num(self.pct)) } else { format!("calc({}% + {})", super::values::fmt_num(self.pct), self.off.to_css()) }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum BgSize {
    Cover,
    Contain,
    /// `None` is `auto`.
    Lens(Option<Lp>, Option<Lp>),
}

impl BgSize {
    pub const AUTO: BgSize = BgSize::Lens(None, None);

    pub fn compute(&self, ctx: &LenCtx) -> BgSize {
        match self {
            BgSize::Lens(a, b) => BgSize::Lens(a.as_ref().map(|l| l.compute(ctx)), b.as_ref().map(|l| l.compute(ctx))),
            other => other.clone(),
        }
    }

    pub fn to_css(&self) -> String {
        match self {
            BgSize::Cover => "cover".into(),
            BgSize::Contain => "contain".into(),
            BgSize::Lens(a, b) => {
                let f = |l: &Option<Lp>| l.as_ref().map_or("auto".to_string(), Lp::to_css);
                format!("{} {}", f(a), f(b))
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rep {
    Repeat,
    Space,
    Round,
    NoRepeat,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BgRepeat(pub Rep, pub Rep);

impl BgRepeat {
    pub fn to_css(self) -> String {
        let f = |r: Rep| match r {
            Rep::Repeat => "repeat",
            Rep::Space => "space",
            Rep::Round => "round",
            Rep::NoRepeat => "no-repeat",
        };
        format!("{} {}", f(self.0), f(self.1))
    }
}

/// `background-origin` / `background-clip` boxes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BgBox {
    Border,
    Padding,
    Content,
    /// `background-clip: text`, painted as `border-box`.
    Text,
}

impl BgBox {
    pub fn to_css(self) -> &'static str {
        match self {
            BgBox::Border => "border-box",
            BgBox::Padding => "padding-box",
            BgBox::Content => "content-box",
            BgBox::Text => "text",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum BgImage {
    None,
    /// The address, resolved against the style sheet when it was parsed.
    Url(Rc<str>),
    Gradient(Rc<Gradient>),
}

impl BgImage {
    pub fn to_css(&self) -> String {
        match self {
            BgImage::None => "none".into(),
            BgImage::Url(u) => format!("url(\"{u}\")"),
            BgImage::Gradient(g) => g.to_css(),
        }
    }
}

/// The background longhands of an element (not inherited). The lists have at least one entry; the number of layers is
/// the length of `images`, the other lists repeat or are cut to it (css-backgrounds-3 2.1).
#[derive(Clone, Debug, PartialEq)]
pub struct Backgrounds {
    pub images: Vec<BgImage>,
    pub pos_x: Vec<Pos1>,
    pub pos_y: Vec<Pos1>,
    pub size: Vec<BgSize>,
    pub repeat: Vec<BgRepeat>,
    pub origin: Vec<BgBox>,
    pub clip: Vec<BgBox>,
    /// Some layer is a `url()`.
    pub has_url: bool,
}

impl Backgrounds {
    pub fn initial() -> Backgrounds {
        Backgrounds {
            images: vec![BgImage::None],
            pos_x: vec![Pos1::ZERO],
            pos_y: vec![Pos1::ZERO],
            size: vec![BgSize::AUTO],
            repeat: vec![BgRepeat(Rep::Repeat, Rep::Repeat)],
            origin: vec![BgBox::Padding],
            clip: vec![BgBox::Border],
            has_url: false,
        }
    }

    /// Entry `i` of a list, repeating the list.
    pub fn nth<T>(list: &[T], i: usize) -> Option<&T> {
        list.get(i % list.len().max(1))
    }
}

/// A parsed value of a background longhand.
#[derive(Clone, Debug, PartialEq)]
pub enum BgVal {
    Images(Vec<BgImage>),
    Pos(Vec<Pos1>),
    Sizes(Vec<BgSize>),
    Repeats(Vec<BgRepeat>),
    Boxes(Vec<BgBox>),
}

// ------------------------------------------------------------------------------------------------------------------
// Gradients

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LinearDir {
    /// Degrees clockwise from "up".
    Angle(f32),
    /// `to <side-or-corner>`: horizontal -1 (left) / 0 / 1 (right), vertical -1 (top) / 0 / 1 (bottom).
    To(i8, i8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Extent {
    ClosestSide,
    FarthestSide,
    ClosestCorner,
    FarthestCorner,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RadSize {
    Extent(Extent),
    /// A circle has only the first radius.
    Lens(Lp, Lp),
}

#[derive(Clone, Debug, PartialEq)]
pub enum GradKind {
    Linear(LinearDir),
    Radial { circle: bool, size: RadSize, at: (Pos1, Pos1) },
    /// `from` in degrees.
    Conic { from: f32, at: (Pos1, Pos1) },
}

/// A color stop, or a color hint (`hint`: only `pos` is meaningful). Positions are lengths or percentages; for conic
/// gradients angles are stored as percentages of a turn.
#[derive(Clone, Debug, PartialEq)]
pub struct Stop {
    pub color: ColorValue,
    pub pos: Option<Lp>,
    pub hint: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Gradient {
    pub kind: GradKind,
    pub repeating: bool,
    pub stops: Vec<Stop>,
}

impl Gradient {
    pub fn compute(&self, ctx: &LenCtx) -> Gradient {
        let kind = match &self.kind {
            GradKind::Linear(d) => GradKind::Linear(*d),
            GradKind::Radial { circle, size, at } => GradKind::Radial {
                circle: *circle,
                size: match size {
                    RadSize::Lens(a, b) => RadSize::Lens(a.compute(ctx), b.compute(ctx)),
                    e => e.clone(),
                },
                at: (at.0.compute(ctx), at.1.compute(ctx)),
            },
            GradKind::Conic { from, at } => GradKind::Conic { from: *from, at: (at.0.compute(ctx), at.1.compute(ctx)) },
        };
        let stops = self.stops.iter().map(|s| Stop { color: s.color, pos: s.pos.as_ref().map(|l| l.compute(ctx)), hint: s.hint }).collect();
        Gradient { kind, repeating: self.repeating, stops }
    }

    pub fn to_css(&self) -> String {
        let name = match (&self.kind, self.repeating) {
            (GradKind::Linear(_), false) => "linear-gradient",
            (GradKind::Linear(_), true) => "repeating-linear-gradient",
            (GradKind::Radial { .. }, false) => "radial-gradient",
            (GradKind::Radial { .. }, true) => "repeating-radial-gradient",
            (GradKind::Conic { .. }, false) => "conic-gradient",
            (GradKind::Conic { .. }, true) => "repeating-conic-gradient",
        };
        let stops: Vec<String> = self
            .stops
            .iter()
            .map(|s| match (&s.pos, s.hint) {
                (Some(p), true) => p.to_css(),
                (Some(p), false) => format!("{} {}", s.color.to_css(), p.to_css()),
                (None, _) => s.color.to_css(),
            })
            .collect();
        format!("{name}({})", stops.join(", "))
    }
}

// ------------------------------------------------------------------------------------------------------------------
// Parsing

fn lower(c: &Cv) -> Option<String> {
    c.ident().map(str::to_ascii_lowercase)
}

fn neg(l: &Lp) -> Lp {
    match l {
        Lp::Len(v, u) => Lp::Len(-v, *u),
        Lp::Pct(p) => Lp::Pct(-p),
        Lp::Calc(c) => Lp::Calc(Rc::new(Calc::Mul(Box::new(Calc::Num(-1.0)), Box::new((**c).clone())))),
    }
}

#[derive(Clone, Debug, PartialEq)]
enum PTok {
    Left,
    Right,
    Top,
    Bottom,
    Center,
    Lp(Lp),
}

fn pos_tok(c: &Cv, q: bool) -> Option<PTok> {
    if let Some(id) = lower(c) {
        return match id.as_str() {
            "left" => Some(PTok::Left),
            "right" => Some(PTok::Right),
            "top" => Some(PTok::Top),
            "bottom" => Some(PTok::Bottom),
            "center" => Some(PTok::Center),
            _ => None,
        };
    }
    p_lp(c, q, false).map(PTok::Lp)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Axis {
    H,
    V,
    Any,
}

fn lp_pos(l: &Lp) -> Pos1 {
    match l {
        Lp::Pct(p) => Pos1 { pct: *p, off: Lp::ZERO },
        other => Pos1 { pct: 0.0, off: other.clone() },
    }
}

/// A keyword with its optional edge offset.
fn kw_pos(t: &PTok, off: Option<Lp>) -> Option<(Axis, Pos1)> {
    let o = off.clone().unwrap_or(Lp::ZERO);
    Some(match t {
        PTok::Left => (Axis::H, Pos1 { pct: 0.0, off: o }),
        PTok::Right => (Axis::H, Pos1 { pct: 100.0, off: neg(&o) }),
        PTok::Top => (Axis::V, Pos1 { pct: 0.0, off: o }),
        PTok::Bottom => (Axis::V, Pos1 { pct: 100.0, off: neg(&o) }),
        PTok::Center if off.is_none() => (Axis::Any, Pos1::CENTER),
        _ => return None,
    })
}

/// css-values-4 `<position>`: one to four values.
fn position(t: &[PTok]) -> Option<(Pos1, Pos1)> {
    let pair = |(a1, pa): (Axis, Pos1), (b1, pb): (Axis, Pos1)| {
        if a1 != Axis::V && b1 != Axis::H {
            Some((pa, pb))
        } else if a1 != Axis::H && b1 != Axis::V {
            Some((pb, pa))
        } else {
            None
        }
    };
    match t {
        [PTok::Lp(l)] => Some((lp_pos(l), Pos1::CENTER)),
        [k] => {
            let (ax, p) = kw_pos(k, None)?;
            Some(match ax {
                Axis::H => (p, Pos1::CENTER),
                Axis::V => (Pos1::CENTER, p),
                Axis::Any => (Pos1::CENTER, Pos1::CENTER),
            })
        }
        [PTok::Lp(x), PTok::Lp(y)] => Some((lp_pos(x), lp_pos(y))),
        [PTok::Lp(x), k] => {
            let (ax, p) = kw_pos(k, None)?;
            (ax != Axis::H).then(|| (lp_pos(x), p))
        }
        [k, PTok::Lp(y)] => {
            let (ax, p) = kw_pos(k, None)?;
            (ax != Axis::V).then(|| (p, lp_pos(y)))
        }
        [a, b] => pair(kw_pos(a, None)?, kw_pos(b, None)?),
        [_, _, _] | [_, _, _, _] => {
            let mut groups = Vec::new();
            let mut i = 0;
            while let Some(k) = t.get(i) {
                let off = match t.get(i + 1) {
                    Some(PTok::Lp(l)) => {
                        i += 1;
                        Some(l.clone())
                    }
                    _ => None,
                };
                groups.push(kw_pos(k, off)?);
                i += 1;
            }
            let [a, b] = <[(Axis, Pos1); 2]>::try_from(groups).ok()?;
            pair(a, b)
        }
        _ => None,
    }
}

fn pos_toks(items: &[&Cv], q: bool) -> Option<Vec<PTok>> {
    items.iter().map(|c| pos_tok(c, q)).collect()
}

fn parse_angle(c: &Cv) -> Option<f32> {
    if let Cv::Tok(Token::Dimension { value, unit, .. }) = c {
        let v = *value as f32;
        return match unit.to_ascii_lowercase().as_str() {
            "deg" => Some(v),
            "grad" => Some(v * 0.9),
            "rad" => Some(v.to_degrees()),
            "turn" => Some(v * 360.0),
            _ => None,
        };
    }
    None
}

fn resolve_url(u: &str, env: &ParseEnv) -> Rc<str> {
    match env.base.as_ref().and_then(|b| b.join(u).ok()) {
        Some(x) => Rc::from(x.href()),
        None => Rc::from(u),
    }
}

/// `<image>` (without `none`): `url()` or a gradient function.
fn parse_image(c: &Cv, env: &ParseEnv) -> Option<BgImage> {
    match c {
        Cv::Tok(Token::Url(u)) => Some(BgImage::Url(resolve_url(u, env))),
        Cv::Func(name, args) => {
            let n = name.to_ascii_lowercase();
            if n == "url" {
                return match trim_ws(args) {
                    [Cv::Tok(Token::Str(u))] => Some(BgImage::Url(resolve_url(u, env))),
                    _ => None,
                };
            }
            parse_gradient(&n, args, env.quirks).map(|g| BgImage::Gradient(Rc::new(g)))
        }
        _ => None,
    }
}

fn parse_gradient(name: &str, args: &[Cv], q: bool) -> Option<Gradient> {
    let (repeating, base) = match name.strip_prefix("repeating-") {
        Some(b) => (true, b),
        None => (false, name),
    };
    if !matches!(base, "linear-gradient" | "radial-gradient" | "conic-gradient") {
        return None;
    }
    let groups = split_commas(args);
    let first: Vec<&Cv> = trim_ws(groups.first()?).iter().filter(|c| !c.is_ws()).collect();
    let conic = base == "conic-gradient";
    // The first group is the configuration unless it starts with a color.
    let is_config = first.first().is_some_and(|c| parse_color_cv(c).is_none() && !(matches!(c, Cv::Tok(Token::Percentage { .. })) || is_lp_start(c, q) && !base.starts_with("radial")));
    let (kind, rest) = if is_config {
        let k = match base {
            "linear-gradient" => parse_linear(&first)?,
            "radial-gradient" => parse_radial(&first, q)?,
            _ => parse_conic(&first, q)?,
        };
        (k, groups.get(1..)?)
    } else {
        let k = match base {
            "linear-gradient" => GradKind::Linear(LinearDir::To(0, 1)),
            "radial-gradient" => GradKind::Radial { circle: false, size: RadSize::Extent(Extent::FarthestCorner), at: (Pos1::CENTER, Pos1::CENTER) },
            _ => GradKind::Conic { from: 0.0, at: (Pos1::CENTER, Pos1::CENTER) },
        };
        (k, groups.as_slice())
    };
    let mut stops: Vec<Stop> = Vec::new();
    for g in rest {
        let items: Vec<&Cv> = trim_ws(g).iter().filter(|c| !c.is_ws()).collect();
        let pos_of = |c: &Cv| -> Option<Lp> {
            if conic {
                if let Some(a) = parse_angle(c) {
                    return Some(Lp::Pct(a / 3.6));
                }
                return match c {
                    Cv::Tok(Token::Percentage { value }) => Some(Lp::Pct(*value as f32)),
                    // A unitless zero is an angle too (browsers take it).
                    Cv::Tok(Token::Number { value, .. }) if *value == 0.0 => Some(Lp::Pct(0.0)),
                    _ => None,
                };
            }
            parse_length_percentage(c, true)
        };
        match items.as_slice() {
            [] => return None,
            [p] if parse_color_cv(p).is_none() => {
                let pos = pos_of(p)?;
                if stops.last().is_none_or(|s| s.hint) {
                    return None;
                }
                stops.push(Stop { color: ColorValue::CurrentColor, pos: Some(pos), hint: true });
            }
            [c, rest @ ..] => {
                let color = parse_color_cv(c)?;
                let pos: Vec<Lp> = rest.iter().map(|x| pos_of(x)).collect::<Option<Vec<_>>>()?;
                match pos.as_slice() {
                    [] => stops.push(Stop { color, pos: None, hint: false }),
                    [a] => stops.push(Stop { color, pos: Some(a.clone()), hint: false }),
                    [a, b] => {
                        stops.push(Stop { color, pos: Some(a.clone()), hint: false });
                        stops.push(Stop { color, pos: Some(b.clone()), hint: false });
                    }
                    _ => return None,
                }
            }
        }
        if stops.len() > MAX_STOPS {
            return None;
        }
    }
    let colors = stops.iter().filter(|s| !s.hint).count();
    // One color stop is enough (css-images-4 3.5): the gradient is that color.
    (colors >= 1 && stops.last().is_some_and(|s| !s.hint)).then_some(Gradient { kind, repeating, stops })
}

fn is_lp_start(c: &Cv, q: bool) -> bool {
    p_lp(c, q, false).is_some()
}

/// Skip `in <colorspace> [<hue-interpolation>]` (recorded: ignored).
fn drop_in<'a>(items: &[&'a Cv]) -> Option<Vec<&'a Cv>> {
    let mut out = Vec::new();
    let mut it = items.iter();
    while let Some(c) = it.next() {
        if c.is_ident("in") {
            for x in it.by_ref() {
                if !x.ident().is_some_and(|i| i.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '-')) {
                    return None;
                }
            }
            break;
        }
        out.push(*c);
    }
    Some(out)
}

fn parse_linear(items: &[&Cv]) -> Option<GradKind> {
    let items = drop_in(items)?;
    match items.as_slice() {
        [] => Some(GradKind::Linear(LinearDir::To(0, 1))),
        [a] if parse_angle(a).is_some() => Some(GradKind::Linear(LinearDir::Angle(parse_angle(a)?))),
        [t, rest @ ..] if t.is_ident("to") && (1..=2).contains(&rest.len()) => {
            let (mut h, mut v) = (0i8, 0i8);
            for c in rest {
                match lower(c)?.as_str() {
                    "left" if h == 0 => h = -1,
                    "right" if h == 0 => h = 1,
                    "top" if v == 0 => v = -1,
                    "bottom" if v == 0 => v = 1,
                    _ => return None,
                }
            }
            Some(GradKind::Linear(LinearDir::To(h, v)))
        }
        _ => None,
    }
}

fn parse_at(items: &[&Cv], q: bool) -> Option<(Pos1, Pos1)> {
    position(&pos_toks(items, q)?)
}

fn parse_radial(items: &[&Cv], q: bool) -> Option<GradKind> {
    let items = drop_in(items)?;
    let at_idx = items.iter().position(|c| c.is_ident("at"));
    let (head, at) = match at_idx {
        Some(i) => (items.get(..i)?, Some(parse_at(items.get(i + 1..)?, q)?)),
        None => (items.as_slice(), None),
    };
    let at = at.unwrap_or((Pos1::CENTER, Pos1::CENTER));
    let mut circle = None;
    let mut ext = None;
    let mut lens: Vec<Lp> = Vec::new();
    for c in head {
        if let Some(id) = lower(c) {
            match id.as_str() {
                "circle" if circle.is_none() => circle = Some(true),
                "ellipse" if circle.is_none() => circle = Some(false),
                "closest-side" => ext = Some(Extent::ClosestSide),
                "farthest-side" => ext = Some(Extent::FarthestSide),
                "closest-corner" => ext = Some(Extent::ClosestCorner),
                "farthest-corner" => ext = Some(Extent::FarthestCorner),
                _ => return None,
            }
        } else {
            let l = p_lp(c, q, true)?;
            lens.push(l);
        }
    }
    if lens.len() > 2 || (ext.is_some() && !lens.is_empty()) {
        return None;
    }
    let circle = circle.unwrap_or(lens.len() == 1);
    let size = match (ext, lens.as_slice()) {
        (Some(e), []) => RadSize::Extent(e),
        (None, []) => RadSize::Extent(Extent::FarthestCorner),
        (None, [a]) if circle && !matches!(a, Lp::Pct(_)) => RadSize::Lens(a.clone(), a.clone()),
        (None, [a, b]) if !circle => RadSize::Lens(a.clone(), b.clone()),
        _ => return None,
    };
    Some(GradKind::Radial { circle, size, at })
}

fn parse_conic(items: &[&Cv], q: bool) -> Option<GradKind> {
    let items = drop_in(items)?;
    let mut from = 0.0;
    let mut at = (Pos1::CENTER, Pos1::CENTER);
    let mut i = 0;
    let mut seen_from = false;
    while let Some(c) = items.get(i) {
        if c.is_ident("from") && !seen_from {
            from = parse_angle(items.get(i + 1)?)?;
            seen_from = true;
            i += 2;
        } else if c.is_ident("at") {
            at = parse_at(items.get(i + 1..)?, q)?;
            break;
        } else {
            return None;
        }
    }
    Some(GradKind::Conic { from, at })
}

fn comma_layers(cvs: &[Cv]) -> Option<Vec<Vec<&Cv>>> {
    let layers: Vec<Vec<&Cv>> = split_commas(cvs).into_iter().map(|l| trim_ws(l).iter().filter(|c| !c.is_ws()).collect::<Vec<_>>()).collect();
    (!layers.is_empty() && layers.len() <= MAX_LAYERS && layers.iter().all(|l| !l.is_empty())).then_some(layers)
}

fn bg_repeat(items: &[&Cv]) -> Option<BgRepeat> {
    let one = |s: &str| match s {
        "repeat" => Some(Rep::Repeat),
        "space" => Some(Rep::Space),
        "round" => Some(Rep::Round),
        "no-repeat" => Some(Rep::NoRepeat),
        _ => None,
    };
    match items {
        [a] => match lower(a)?.as_str() {
            "repeat-x" => Some(BgRepeat(Rep::Repeat, Rep::NoRepeat)),
            "repeat-y" => Some(BgRepeat(Rep::NoRepeat, Rep::Repeat)),
            s => one(s).map(|r| BgRepeat(r, r)),
        },
        [a, b] => Some(BgRepeat(one(&lower(a)?)?, one(&lower(b)?)?)),
        _ => None,
    }
}

fn bg_size(items: &[&Cv], q: bool) -> Option<BgSize> {
    let one = |c: &Cv| -> Option<Option<Lp>> {
        if c.is_ident("auto") { Some(None) } else { p_lp(c, q, true).map(Some) }
    };
    match items {
        [a] if a.is_ident("cover") => Some(BgSize::Cover),
        [a] if a.is_ident("contain") => Some(BgSize::Contain),
        [a] => Some(BgSize::Lens(one(a)?, None)),
        [a, b] => Some(BgSize::Lens(one(a)?, one(b)?)),
        _ => None,
    }
}

fn bg_box(c: &Cv, clip: bool) -> Option<BgBox> {
    match lower(c)?.as_str() {
        "border-box" => Some(BgBox::Border),
        "padding-box" => Some(BgBox::Padding),
        "content-box" => Some(BgBox::Content),
        "text" if clip => Some(BgBox::Text),
        _ => None,
    }
}

/// One background longhand (comma separated lists).
pub fn parse_longhand(prop: PropId, cvs: &[Cv], env: &ParseEnv) -> Option<Value> {
    let layers = comma_layers(cvs)?;
    let q = env.quirks;
    let val = match prop {
        PropId::BackgroundImage => BgVal::Images(
            layers
                .iter()
                .map(|l| match l.as_slice() {
                    [c] if c.is_ident("none") => Some(BgImage::None),
                    [c] => parse_image(c, env),
                    _ => None,
                })
                .collect::<Option<Vec<_>>>()?,
        ),
        PropId::BackgroundPositionX | PropId::BackgroundPositionY => {
            let want = if prop == PropId::BackgroundPositionX { Axis::H } else { Axis::V };
            BgVal::Pos(
                layers
                    .iter()
                    .map(|l| match pos_toks(l, q)?.as_slice() {
                        [PTok::Lp(l)] => Some(lp_pos(l)),
                        [PTok::Center] => Some(Pos1::CENTER),
                        [k] => kw_pos(k, None).filter(|(ax, _)| *ax == want).map(|(_, p)| p),
                        [k, PTok::Lp(o)] => kw_pos(k, Some(o.clone())).filter(|(ax, _)| *ax == want).map(|(_, p)| p),
                        _ => None,
                    })
                    .collect::<Option<Vec<_>>>()?,
            )
        }
        PropId::BackgroundSize => BgVal::Sizes(layers.iter().map(|l| bg_size(l, q)).collect::<Option<Vec<_>>>()?),
        PropId::BackgroundRepeat => BgVal::Repeats(layers.iter().map(|l| bg_repeat(l)).collect::<Option<Vec<_>>>()?),
        PropId::BackgroundOrigin | PropId::BackgroundClip => {
            let clip = prop == PropId::BackgroundClip;
            BgVal::Boxes(
                layers
                    .iter()
                    .map(|l| match l.as_slice() {
                        [c] => bg_box(c, clip),
                        _ => None,
                    })
                    .collect::<Option<Vec<_>>>()?,
            )
        }
        _ => return None,
    };
    Some(Value::Bg(Rc::new(val)))
}

/// `background-position`: both axes of each layer.
pub fn parse_position_shorthand(cvs: &[Cv], env: &ParseEnv) -> Option<Vec<(PropId, Value)>> {
    let layers = comma_layers(cvs)?;
    let mut xs = Vec::new();
    let mut ys = Vec::new();
    for l in &layers {
        let (x, y) = position(&pos_toks(l, env.quirks)?)?;
        xs.push(x);
        ys.push(y);
    }
    Some(vec![(PropId::BackgroundPositionX, Value::Bg(Rc::new(BgVal::Pos(xs)))), (PropId::BackgroundPositionY, Value::Bg(Rc::new(BgVal::Pos(ys))))])
}

/// The `background` shorthand (css-backgrounds-3 2.10): `<bg-layer>#, <final-bg-layer>`. The color may only be in the last layer.
pub fn parse_background(cvs: &[Cv], env: &ParseEnv) -> Option<Vec<(PropId, Value)>> {
    let layers = comma_layers(cvs)?;
    let q = env.quirks;
    let n = layers.len();
    let mut images = Vec::new();
    let (mut xs, mut ys, mut sizes, mut reps, mut origins, mut clips) = (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let mut color = ColorValue::Color(super::color::Color::TRANSPARENT);
    for (li, l) in layers.iter().enumerate() {
        let (mut image, mut pos, mut size, mut rep, mut attach) = (None, None, None, None, false);
        let mut boxes: Vec<BgBox> = Vec::new();
        let mut layer_color = None;
        let mut i = 0;
        while let Some(&c) = l.get(i) {
            i += 1;
            let id = lower(c);
            if let Some(v) = parse_color_cv(c) {
                if layer_color.replace(v).is_some() {
                    return None;
                }
            } else if image.is_none() && (matches!(c, Cv::Tok(Token::Url(_))) || matches!(c, Cv::Func(n, _) if n.eq_ignore_ascii_case("url") || n.to_ascii_lowercase().ends_with("gradient"))) {
                image = Some(parse_image(c, env)?);
            } else if image.is_none() && id.as_deref() == Some("none") {
                image = Some(BgImage::None);
            } else if !attach && matches!(id.as_deref(), Some("scroll" | "fixed" | "local")) {
                attach = true;
            } else if rep.is_none() && matches!(id.as_deref(), Some("repeat-x" | "repeat-y" | "repeat" | "no-repeat" | "space" | "round")) {
                // A second keyword belongs to the same repeat-style.
                let two = l.get(i).and_then(|x| lower(x)).is_some_and(|s| matches!(s.as_str(), "repeat" | "no-repeat" | "space" | "round")) && !id.as_deref().is_some_and(|s| s.starts_with("repeat-"));
                let take = if two { 2 } else { 1 };
                rep = Some(bg_repeat(l.get(i - 1..i - 1 + take)?)?);
                i += take - 1;
            } else if boxes.len() < 2 && matches!(id.as_deref(), Some("border-box" | "padding-box" | "content-box" | "text")) {
                boxes.push(bg_box(c, true)?);
            } else if pos.is_none() && pos_tok(c, q).is_some() {
                let start = i - 1;
                let mut end = i;
                while end < l.len() && end - start < 4 && l.get(end).is_some_and(|x| pos_tok(x, q).is_some()) {
                    end += 1;
                }
                pos = Some(position(&pos_toks(l.get(start..end)?, q)?)?);
                i = end;
                if l.get(i).is_some_and(|x| x.is_delim('/')) {
                    let (s, mut e) = (i + 1, i + 1);
                    while e < l.len() && e - s < 2 && l.get(e).is_some_and(|x| x.is_ident("auto") || p_lp(x, q, true).is_some() || x.is_ident("cover") || x.is_ident("contain")) {
                        e += 1;
                    }
                    size = Some(bg_size(l.get(s..e)?, q)?);
                    i = e;
                }
            } else {
                return None;
            }
        }
        if let Some(v) = layer_color {
            if li + 1 != n {
                return None;
            }
            color = v;
        }
        let (px, py) = pos.unwrap_or((Pos1::ZERO, Pos1::ZERO));
        images.push(image.unwrap_or(BgImage::None));
        xs.push(px);
        ys.push(py);
        sizes.push(size.unwrap_or(BgSize::AUTO));
        reps.push(rep.unwrap_or(BgRepeat(Rep::Repeat, Rep::Repeat)));
        // One box sets origin and clip; two: origin, then clip. `text` is only a clip (css-backgrounds-3 2.9).
        let (o, c) = match boxes.as_slice() {
            [] => (BgBox::Padding, BgBox::Border),
            [b] => (if *b == BgBox::Text { BgBox::Padding } else { *b }, *b),
            [a, b] if *a != BgBox::Text => (*a, *b),
            _ => return None,
        };
        origins.push(o);
        clips.push(c);
    }
    // The parts left at their initial value are `Value::Ignored` (no allocation, and no `Backgrounds` made for the element
    // when nothing else is set): `background: #fff` is by far the most common form.
    let b = |v: BgVal, initial: bool| if initial { Value::Ignored } else { Value::Bg(Rc::new(v)) };
    let (i_img, i_x, i_y, i_size, i_rep, i_org, i_clip) = (
        images.as_slice() == [BgImage::None],
        xs.as_slice() == [Pos1::ZERO],
        ys.as_slice() == [Pos1::ZERO],
        sizes.as_slice() == [BgSize::AUTO],
        reps.as_slice() == [BgRepeat(Rep::Repeat, Rep::Repeat)],
        origins.as_slice() == [BgBox::Padding],
        clips.as_slice() == [BgBox::Border],
    );
    Some(vec![
        (PropId::BackgroundImage, b(BgVal::Images(images), i_img)),
        (PropId::BackgroundPositionX, b(BgVal::Pos(xs), i_x)),
        (PropId::BackgroundPositionY, b(BgVal::Pos(ys), i_y)),
        (PropId::BackgroundSize, b(BgVal::Sizes(sizes), i_size)),
        (PropId::BackgroundRepeat, b(BgVal::Repeats(reps), i_rep)),
        (PropId::BackgroundOrigin, b(BgVal::Boxes(origins), i_org)),
        (PropId::BackgroundClip, b(BgVal::Boxes(clips), i_clip)),
        (PropId::BackgroundColor, Value::Color(color)),
    ])
}

/// `border-radius`: `<length-percentage>{1,4} [ / <length-percentage>{1,4} ]?` in the order top-left, top-right,
/// bottom-right, bottom-left (css-backgrounds-3 5.1).
pub fn parse_radius_shorthand(items: &[&Cv], q: bool) -> Option<Vec<(PropId, Value)>> {
    let slash = items.iter().position(|c| c.is_delim('/'));
    let (h, v) = match slash {
        Some(i) => (items.get(..i)?, items.get(i + 1..)?),
        None => (items, items),
    };
    let expand = |part: &[&Cv]| -> Option<[Lp; 4]> {
        if part.is_empty() || part.len() > 4 {
            return None;
        }
        let vals: Vec<Lp> = part.iter().map(|c| p_lp(c, q, true)).collect::<Option<Vec<_>>>()?;
        let g = |i: usize| vals.get(i).cloned();
        let [a, b, c, d] = match vals.len() {
            1 => [g(0)?, g(0)?, g(0)?, g(0)?],
            2 => [g(0)?, g(1)?, g(0)?, g(1)?],
            3 => [g(0)?, g(1)?, g(2)?, g(1)?],
            _ => [g(0)?, g(1)?, g(2)?, g(3)?],
        };
        Some([a, b, c, d])
    };
    let (hs, vs) = (expand(h)?, expand(v)?);
    let props = [PropId::BorderTopLeftRadius, PropId::BorderTopRightRadius, PropId::BorderBottomRightRadius, PropId::BorderBottomLeftRadius];
    Some(props.into_iter().zip(hs.into_iter().zip(vs)).map(|(p, (a, b))| (p, Value::Radius(a, b))).collect())
}

/// One radius corner: one or two `<length-percentage>`.
pub fn parse_radius(items: &[&Cv], q: bool) -> Option<Value> {
    match items {
        [a] => {
            let l = p_lp(a, q, true)?;
            Some(Value::Radius(l.clone(), l))
        }
        [a, b] => Some(Value::Radius(p_lp(a, q, true)?, p_lp(b, q, true)?)),
        _ => None,
    }
}

#[cfg(test)]
#[allow(clippy::indexing_slicing, clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::css::syntax::parse_cvs;

    fn bg(v: &str) -> Option<Vec<(PropId, Value)>> {
        parse_background(&parse_cvs(v), &ParseEnv::default())
    }

    fn get(list: &[(PropId, Value)], p: PropId) -> Value {
        list.iter().find(|(q, _)| *q == p).unwrap().1.clone()
    }

    #[test]
    fn the_shorthand_sets_every_longhand() {
        let l = bg("url(a.png) 10px 20% / cover no-repeat content-box border-box #f00").unwrap();
        let Value::Bg(v) = get(&l, PropId::BackgroundImage) else { panic!() };
        assert_eq!(*v, BgVal::Images(vec![BgImage::Url("a.png".into())]));
        let Value::Bg(v) = get(&l, PropId::BackgroundSize) else { panic!() };
        assert_eq!(*v, BgVal::Sizes(vec![BgSize::Cover]));
        let Value::Bg(v) = get(&l, PropId::BackgroundOrigin) else { panic!() };
        assert_eq!(*v, BgVal::Boxes(vec![BgBox::Content]));
        // The clip is the initial value: it is left out.
        assert!(matches!(get(&l, PropId::BackgroundClip), Value::Ignored));
        assert!(matches!(get(&l, PropId::BackgroundColor), Value::Color(ColorValue::Color(c)) if c.r == 255));
    }

    #[test]
    fn positions_fold_edge_offsets() {
        let (x, y) = position(&pos_toks(&parse_cvs("right 10px bottom 20%").iter().filter(|c| !c.is_ws()).collect::<Vec<_>>(), false).unwrap()).unwrap();
        assert_eq!(x.pct, 100.0);
        assert_eq!(x.off, Lp::px(-10.0));
        assert_eq!((y.pct, y.off), (100.0, Lp::Pct(-20.0)));
        assert_eq!(x.resolve(100.0, 40.0), 50.0);
    }

    #[test]
    fn a_color_in_an_early_layer_is_invalid() {
        assert!(bg("red, url(a.png)").is_none());
        assert!(bg("url(a.png), red").is_some());
        assert!(bg("url(a.png) bogus").is_none());
    }

    #[test]
    fn gradients_parse_their_configuration_and_stops() {
        let g = |s: &str| {
            let cvs = parse_cvs(s);
            let Some(Cv::Func(n, a)) = cvs.first() else { panic!() };
            parse_gradient(&n.to_ascii_lowercase(), a, false)
        };
        let l = g("linear-gradient(to top right, red, 50%, blue 30% 60%)").unwrap();
        assert_eq!(l.kind, GradKind::Linear(LinearDir::To(1, -1)));
        assert_eq!(l.stops.len(), 4);
        assert!(l.stops[1].hint);
        assert!(matches!(g("linear-gradient(45deg, red, blue)").unwrap().kind, GradKind::Linear(LinearDir::Angle(a)) if a == 45.0));
        assert!(matches!(g("radial-gradient(circle 30px at left 10px top, red, blue)").unwrap().kind, GradKind::Radial { circle: true, .. }));
        assert!(matches!(g("conic-gradient(from 0.25turn at 10% 20%, red 0deg, blue 90deg)").unwrap().kind, GradKind::Conic { from, .. } if from == 90.0));
        assert!(g("linear-gradient(red)").is_some());
        assert!(g("linear-gradient(10%, red)").is_none());
        assert!(g("linear-gradient(red, 10%)").is_none());
        assert!(g("radial-gradient(circle 10px 20px, red, blue)").is_none());
        assert!(g("-webkit-linear-gradient(red, blue)").is_none());
    }
}
