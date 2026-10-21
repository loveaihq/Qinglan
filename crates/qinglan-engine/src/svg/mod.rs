//! SVG (L2 step 2e-1), written for this engine on top of [`crate::vector`] and the CSS engine.
//!
//! Three kinds of source, one pipeline: an inline `<svg>` of an HTML page is already in the DOM (foreign content); a standalone
//! SVG file (`<img src=x.svg>`, CSS backgrounds, `data:` URLs) is read by [`xml::parse`] into the same kind of [`Document`] and
//! styled by [`crate::css::style_document`]. [`build::scene`] walks the styled tree once and flattens it into a [`Scene`]: a
//! list of shapes with their transforms, paints, clips and opacity groups, in painting order, in the user space of the outermost
//! `svg`. [`render::render`] then draws a scene into a premultiplied BGRA [`render::Canvas`] of any size (the viewBox
//! is mapped onto it), which is what `<img>` and backgrounds get as their decoded bitmap, and what an inline `<svg>` is
//! drawn from (cached by size).
//!
//! Work is bounded in three places (docs/decisions.md "2e-1"): the XML parser (nodes, depth), the scene (element visits, `use`
//! expansions and nesting, operations, path segments, gradient stops) and the renderer (pixels filled, group layers).

pub mod build;
pub mod data;
pub mod render;
pub mod xml;

use std::rc::Rc;

use crate::css::color::Color;
use crate::html::dom::{Document, NodeId};
use crate::css::types::FontFamily;
use crate::platform::image::{DecodedImage, ImageError, MAX_DECODED_PIXELS, MAX_IMAGE_PIXELS};
use crate::vector::{FillRule, Path, Stroke, Transform};

pub use build::{Ids, scene};
pub use render::{Canvas, render};

/// `preserveAspectRatio` (SVG 2 coordinate-systems 8.8): alignment per axis (0 min, 1 mid, 2 max), `none`, `slice`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Par {
    pub none: bool,
    pub ax: u8,
    pub ay: u8,
    pub slice: bool,
}

impl Par {
    pub const DEFAULT: Par = Par { none: false, ax: 1, ay: 1, slice: false };

    pub fn parse(s: &str) -> Par {
        let mut it = s.split_ascii_whitespace().filter(|t| *t != "defer");
        let mut par = Par::DEFAULT;
        match it.next() {
            Some("none") => par.none = true,
            Some(a) if a.len() == 8 => {
                let axis = |p: &str| match p {
                    "Min" => Some(0),
                    "Mid" => Some(1),
                    "Max" => Some(2),
                    _ => None,
                };
                match (a.get(..1), a.get(1..4).and_then(axis), a.get(4..5), a.get(5..8).and_then(axis)) {
                    (Some("x"), Some(x), Some("Y"), Some(y)) => (par.ax, par.ay) = (x, y),
                    _ => return Par::DEFAULT,
                }
            }
            _ => return Par::DEFAULT,
        }
        par.slice = it.next() == Some("slice");
        par
    }
}

/// The transform that maps the viewBox `vb` (x, y, w, h) into a viewport of `w` x `h` at the origin
/// (SVG 2 coordinate-systems 8.8 "viewBox to viewport transformation").
pub fn view_transform(vb: [f32; 4], par: Par, w: f32, h: f32) -> Transform {
    let [x, y, vw, vh] = vb;
    let (sx, sy) = (w / vw, h / vh);
    if par.none {
        return Transform { a: sx, b: 0.0, c: 0.0, d: sy, e: -x * sx, f: -y * sy };
    }
    let s = if par.slice { sx.max(sy) } else { sx.min(sy) };
    let tx = (w - vw * s) * f32::from(par.ax) / 2.0;
    let ty = (h - vh * s) * f32::from(par.ay) / 2.0;
    Transform { a: s, b: 0.0, c: 0.0, d: s, e: tx - x * s, f: ty - y * s }
}

/// What the root `svg` says about its own size (SVG 2 geometry 8.? / coordinate-systems 8.2 and css-images-3 5.3).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Intrinsic {
    pub w: Option<f32>,
    pub h: Option<f32>,
    pub ratio: Option<f32>,
    pub vb: Option<[f32; 4]>,
    pub par: Option<Par>,
}

/// Read the `width`, `height`, `viewBox` and `preserveAspectRatio` of the `svg` element `root`. Lengths in `%` and
/// anything that is not a length are "auto" (`None`); `em` and `ex` are taken at the default 16 px.
pub fn intrinsic(doc: &Document, root: NodeId) -> Intrinsic {
    let attr = |name: &str| doc.element(root).and_then(|e| e.attrs.iter().find(|a| a.ns == crate::html::dom::Namespace::None && doc.atoms.get(a.name) == name).map(|a| doc.value(a)));
    let abs = |name: &str| {
        let (v, u) = data::length(attr(name)?)?;
        let v = match u {
            data::Unit::Px => v,
            data::Unit::Em => v * 16.0,
            data::Unit::Ex => v * 8.0,
            data::Unit::Pct => return None,
        };
        (v >= 0.0).then_some(v)
    };
    let (w, h) = (abs("width"), abs("height"));
    let vb = attr("viewBox").and_then(|s| data::numbers(s, 5)).and_then(|v| match v.as_slice() {
        &[x, y, w, h] if w > 0.0 && h > 0.0 => Some([x, y, w, h]),
        _ => None,
    });
    let ratio = match (w, h, vb) {
        (Some(w), Some(h), _) if w > 0.0 && h > 0.0 => Some(w / h),
        (_, _, Some(v)) => Some(v[2] / v[3]),
        _ => None,
    };
    Intrinsic { w, h, ratio, vb, par: attr("preserveAspectRatio").map(Par::parse) }
}

impl Intrinsic {
    /// The size an image gets when nothing else decides it: the given width and height, a missing one from the ratio, and the
    /// "default object size" 300 x 150 for what is still missing (css-images-3 5.3: a ratio alone is contained in it).
    pub fn object_size(&self) -> (f32, f32) {
        let r = self.ratio.filter(|r| *r > 0.0);
        match (self.w, self.h, r) {
            (Some(w), Some(h), _) => (w, h),
            (Some(w), None, Some(r)) => (w, w / r),
            (None, Some(h), Some(r)) => (h * r, h),
            (Some(w), None, None) => (w, 150.0),
            (None, Some(h), None) => (300.0, h),
            (None, None, Some(r)) => {
                if r >= 2.0 {
                    (300.0, 300.0 / r)
                } else {
                    (150.0 * r, 150.0)
                }
            }
            (None, None, None) => (300.0, 150.0),
        }
    }
}

/// A paint: a color (the paint's alpha is in [`Fill::alpha`]) or an index into [`Scene::grads`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Paint {
    Color(Color),
    Grad(u32),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fill {
    pub paint: Paint,
    /// `fill-opacity` / `stroke-opacity` and the element's own opacity when it is folded in, 0..=255.
    pub alpha: u8,
}

/// `linearGradient` / `radialGradient` resolved (SVG 2 pservers 13.2, 13.3), with the attributes of the `href` chain.
#[derive(Clone, Debug)]
pub struct Grad {
    pub radial: bool,
    /// `gradientUnits="objectBoundingBox"`: the geometry is in fractions of the painted shape's bounding box.
    pub bbox: bool,
    pub xf: Transform,
    /// 0 pad, 1 reflect, 2 repeat.
    pub spread: u8,
    /// Linear: x1 y1 x2 y2. Radial: cx cy r fx fy fr.
    pub p: [f32; 6],
    pub stops: Vec<(f32, Color)>,
}

#[derive(Clone, Debug)]
pub struct Shape {
    pub path: Path,
    pub ctm: Transform,
    pub fill: Option<Fill>,
    pub stroke: Option<(Fill, Stroke)>,
    pub rule: FillRule,
    /// 1 + an index into [`Scene::clips`]; 0 none.
    pub clip: u32,
}

/// A clip path instance: the shapes (with their transforms to the root's user space) whose union is the visible part, and
/// the clip it is nested in (1 + index, 0 none): the visible part is the intersection of the chain.
#[derive(Clone, Debug)]
pub struct Clip {
    pub parts: Vec<(Path, Transform, FillRule)>,
    pub next: u32,
}

#[derive(Clone, Debug)]
pub struct TextRun {
    pub text: String,
    pub fill: Fill,
    pub size: f32,
    pub weight: f32,
    pub slant: crate::platform::FontSlant,
    pub families: Rc<Vec<FontFamily>>,
    pub dx: f32,
    pub dy: f32,
}

/// One text chunk (SVG 2 text 11.? "text chunk"): the runs follow each other from (`x`, `y`); `anchor` -1 start, 0 middle, 1 end.
#[derive(Clone, Debug)]
pub struct TextOp {
    pub x: f32,
    pub y: f32,
    pub ctm: Transform,
    pub anchor: i8,
    pub runs: Vec<TextRun>,
    pub clip: u32,
}

#[derive(Clone, Debug)]
pub enum Op {
    Shape(Box<Shape>),
    Text(Box<TextOp>),
    /// Start a group that is composited with this alpha (0..=255) at its `End`.
    Begin(u8),
    End,
}

/// A link inside the SVG: the `a` element and its box in the root's user space (x, y, w, h).
#[derive(Clone, Copy, Debug)]
pub struct Link {
    pub node: NodeId,
    pub bbox: [f32; 4],
}

#[derive(Debug)]
pub struct Scene {
    /// Unique, so that renderings of it can be cached by id.
    pub id: u64,
    pub intrinsic: Intrinsic,
    pub ops: Vec<Op>,
    pub grads: Vec<Grad>,
    pub clips: Vec<Clip>,
    pub links: Vec<Link>,
    /// Work spent building it (element visits, operations), for the page's work meter.
    pub work: u64,
    /// False when a limit stopped the scene short.
    pub complete: bool,
}

/// Header of a standalone SVG for `<img>` and backgrounds: its natural size in px.
pub fn probe(bytes: &[u8]) -> Result<(u32, u32), ImageError> {
    let doc = xml::parse(bytes).map_err(|_| ImageError::Corrupt)?;
    let (w, h) = intrinsic(&doc, doc.document_element()).object_size();
    let (w, h) = (w.round().max(1.0), h.round().max(1.0));
    if f64::from(w) * f64::from(h) > MAX_IMAGE_PIXELS as f64 {
        return Err(ImageError::TooLarge { width: w as u32, height: h as u32 });
    }
    Ok((w as u32, h as u32))
}

/// Draw a standalone SVG into `width` x `height` device pixels. Text is not drawn (it needs the platform, which lives on
/// the UI thread).
pub fn rasterize(bytes: &[u8], width: u32, height: u32) -> Result<DecodedImage, ImageError> {
    let mut doc = xml::parse(bytes).map_err(|_| ImageError::Corrupt)?;
    let root = doc.document_element();
    let (nw, nh) = intrinsic(&doc, root).object_size();
    // A request over the pixel cap is shrunk, keeping the ratio (the decoder contract).
    let (mut w, mut h) = (width.max(1), height.max(1));
    if u64::from(w) * u64::from(h) > MAX_DECODED_PIXELS {
        let k = (MAX_DECODED_PIXELS as f64 / (f64::from(w) * f64::from(h))).sqrt();
        (w, h) = (((f64::from(w) * k) as u32).max(1), ((f64::from(h) * k) as u32).max(1));
    }
    let opts = crate::css::StyleOptions { viewport_w: nw, viewport_h: nh, profile: false };
    let out = crate::css::style_document(&mut doc, None, &mut crate::css::NoResources, &opts);
    let scene = build::scene(&doc, &out.tree.styles, root, &mut Ids::default(), None);
    let canvas = render::render(&scene, w, h, None, None).ok_or(ImageError::Corrupt)?;
    Ok(DecodedImage { width: canvas.w, height: canvas.h, natural_width: nw as u32, natural_height: nh as u32, bgra: canvas.data })
}

/// The bitmaps of inline SVGs drawn lately, kept so that a scroll repaints a page of icons without rendering each again.
const CACHE_BYTES: usize = 3 << 19;

thread_local! {
    static CACHE: std::cell::RefCell<Vec<(u64, u32, u32, std::sync::Arc<DecodedImage>)>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// The rendering of `sc` at `w` x `h` device pixels (`dpr` device pixels per px of its user space when it has no viewBox),
/// from the cache or made now (and kept, oldest dropped, up to 1.5 MB). `None` when it is too big or empty.
pub fn cached_bitmap(sc: &Scene, w: u32, h: u32, platform: &dyn crate::platform::Platform, dpr: f32) -> Option<std::sync::Arc<DecodedImage>> {
    if let Some(hit) = CACHE.with(|c| c.borrow().iter().find(|e| e.0 == sc.id && e.1 == w && e.2 == h).map(|e| e.3.clone())) {
        return Some(hit);
    }
    let canvas = render::render(sc, w, h, Some(platform), Some(dpr))?;
    let img = std::sync::Arc::new(DecodedImage { width: canvas.w, height: canvas.h, natural_width: w, natural_height: h, bgra: canvas.data });
    CACHE.with(|c| {
        let mut c = c.borrow_mut();
        let cost = |e: &(u64, u32, u32, std::sync::Arc<DecodedImage>)| e.3.bgra.len();
        while !c.is_empty() && c.iter().map(cost).sum::<usize>() + img.bgra.len() > CACHE_BYTES {
            c.remove(0);
        }
        if img.bgra.len() <= CACHE_BYTES {
            c.push((sc.id, w, h, img.clone()));
        }
    });
    Some(img)
}
