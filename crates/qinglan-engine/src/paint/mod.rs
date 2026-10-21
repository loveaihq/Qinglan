//! Painting (L1 task 16), step 1c-2a: the display list, its spatial index and the software rasterizer.
//!
//! 1. [`build_display_list`] turns a [`crate::layout::LayoutResult`] into a [`DisplayList`]: solid rectangles
//!    (backgrounds, borders, decoration lines), images, text runs (glyph ids, advances, colour) and clip
//!    rectangles, in paint order, in page coordinates (CSS px as 1/64 units). `position: fixed` boxes and
//!    everything inside them go into a separate list in viewport coordinates. The list does not depend on the
//!    scroll position, the device pixel ratio or the platform's pixels, so it is built once per layout.
//! 2. [`DisplayList::index`] finds the items that touch a band of the page without looking at the others.
//! 3. [`raster::render_frame`] draws the part of the list that is in the viewport into a premultiplied BGRA
//!    [`raster::Frame`] in device pixels: rectangle fills, bilinear image scaling, glyph masks from the platform
//!    blended with the text colour, clipping. The canvas background (CSS2 14.2) is the first thing painted.
//! 4. [`images`] decides which images to request and which decoded bitmaps to drop (lazy loading, L1 task 15)
//!    and keeps the decoded bitmaps the rasterizer draws.
//!
//! Choices: all border styles are drawn solid, each side is a rectangle (no mitred corners), all decoration
//! styles are single solid lines, text is blended in sRGB space with the platform's grayscale mask.

mod borders;
pub mod build;
mod fx;
pub mod decode;
pub mod gradient;
pub mod images;
pub mod raster;

use crate::css::color::Color;
use crate::css::types::BorderLineStyle;
use crate::html::dom::NodeId;
use crate::layout::units::LayoutUnit as LU;
use crate::vector::Transform;

pub use build::build_display_list;
pub use images::{Band, ImageSource, ImageStore, NoBitmaps, far_images, wanted_images};
pub use raster::{Frame, RasterStats, device_size, render_frame};

/// A rectangle in layout units. Page coordinates for the normal list, viewport coordinates for the fixed list.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Rect {
    pub x: LU,
    pub y: LU,
    pub w: LU,
    pub h: LU,
}

impl Rect {
    pub fn new(x: LU, y: LU, w: LU, h: LU) -> Rect {
        Rect { x, y, w, h }
    }
    pub fn right(&self) -> LU {
        self.x + self.w
    }
    pub fn bottom(&self) -> LU {
        self.y + self.h
    }
    pub fn is_empty(&self) -> bool {
        self.w <= LU::ZERO || self.h <= LU::ZERO
    }
    pub fn intersects(&self, o: &Rect) -> bool {
        self.x < o.right() && o.x < self.right() && self.y < o.bottom() && o.y < self.bottom()
    }
    pub fn intersect(&self, o: &Rect) -> Rect {
        let (x, y) = (self.x.max(o.x), self.y.max(o.y));
        let (r, b) = (self.right().min(o.right()), self.bottom().min(o.bottom()));
        Rect { x, y, w: (r - x).max0(), h: (b - y).max0() }
    }
}

/// A rounded rectangle in page coordinates; radii in CSS px (horizontal, vertical) for the top-left, top-right,
/// bottom-right and bottom-left corner.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RRect {
    pub rect: Rect,
    pub radii: [(f32, f32); 4],
}

/// The rounded part of a clip: its own shape (if any) and the nearest clip below it that has rounded parts. The
/// rectangle of every clip is in [`DisplayList::clips`] (already the intersection with all the clips below).
#[derive(Clone, Debug)]
pub struct Round {
    pub shape: Option<RRect>,
    pub prev: u32,
}

/// One background layer, resolved: where its tiles are and how big they are (CSS px, page coordinates).
#[derive(Clone, Copy, Debug)]
pub struct BgDraw {
    /// The image (`NONE` for a gradient).
    pub node: NodeId,
    /// `DisplayList::grads[i]` or `u32::MAX`.
    pub grad: u32,
    pub tile: (f32, f32),
    /// The top left corner of tile (0, 0).
    pub origin: (f32, f32),
    /// The distance between tiles; 0: this axis does not repeat.
    pub step: (f32, f32),
}

/// A border or outline: the outer rectangle, the widths (top, right, bottom, left), colors, styles and outer radii (CSS px).
#[derive(Clone, Copy, Debug)]
pub struct BorderDraw {
    pub rect: Rect,
    pub w: [LU; 4],
    pub color: [Color; 4],
    pub style: [BorderLineStyle; 4],
    pub radii: [(f32, f32); 4],
}

/// "No clip" in [`Item::clip`].
pub const NO_CLIP: u32 = u32::MAX;

/// One glyph of a text run. `advance` already includes letter-spacing, word-spacing and justification.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaintGlyph {
    /// The platform's font id.
    pub font: u16,
    pub glyph: u16,
    pub advance: f32,
    /// Offset of the drawn glyph from its pen position (marks, kerning), CSS px, y down.
    pub dx: f32,
    pub dy: f32,
}

/// Glyphs of one style on one line: `DisplayList::glyphs[glyphs.0..glyphs.1]`, the pen starting at `x`.
#[derive(Clone, Copy, Debug)]
pub struct TextRun {
    pub x: LU,
    pub baseline: LU,
    /// Font size, CSS px.
    pub size: f32,
    pub color: Color,
    pub glyphs: (u32, u32),
    /// The blur radius of a text shadow, CSS px (0: plain text).
    pub blur: f32,
    /// Sideways text in vertical writing: `x` is the baseline, `baseline` where the pen starts; the pen runs down and the
    /// glyphs are turned a quarter clockwise.
    pub rot: bool,
}

#[derive(Clone, Copy, Debug)]
pub enum ItemKind {
    /// A solid rectangle filling `Item::bounds`.
    Rect(Color),
    /// The image of this DOM node, stretched over `Item::bounds`.
    Image { node: NodeId },
    Text(TextRun),
    /// A background layer: `DisplayList::bgs[i]`, drawn in `Item::bounds` (the painting area).
    Bg(u32),
    /// A border or outline that is not a plain solid rectangle: `DisplayList::borders[i]`.
    Border(u32),
    /// A box shadow: `DisplayList::shadows[i]`, drawn in `Item::bounds`.
    Shadow(u32),
    /// A group (`opacity`, `transform`): `DisplayList::groups[i]`, drawn in `Item::bounds` (its transformed extent).
    Group(u32),
}

/// A `box-shadow` (css-backgrounds-3 6.1). Outer: `rect` is the border box and the shadow is cast outside it; inset: `rect` is
/// the padding box and the shadow is cast inside it. Offsets, blur and spread in CSS px.
#[derive(Clone, Copy, Debug)]
pub struct ShadowDraw {
    pub rect: Rect,
    pub radii: [(f32, f32); 4],
    pub color: Color,
    pub dx: f32,
    pub dy: f32,
    pub blur: f32,
    pub spread: f32,
    pub inset: bool,
}

/// Things painted together as one: through an offscreen layer with `opacity`, and about `xf` (page CSS px to page CSS px).
/// The items are in the page coordinates of the untransformed box, with a clip chain of their own.
pub struct Group {
    pub items: Vec<Item>,
    pub opacity: u8,
    pub xf: Transform,
    /// The union of the bounds of `items`.
    pub local: Rect,
}

#[derive(Clone, Copy, Debug)]
pub struct Item {
    /// The rectangle the item draws in (text: the line fragment grown by half an em, to cover glyph overhang).
    pub bounds: Rect,
    /// An index into [`DisplayList::clips`], or [`NO_CLIP`].
    pub clip: u32,
    pub kind: ItemKind,
}

/// Where an image is drawn (for decoding at the right size and for the distance to the viewport).
#[derive(Clone, Copy, Debug)]
pub struct ImagePlace {
    pub node: NodeId,
    pub rect: Rect,
    /// The page rows the image is drawn in (for the distance to the viewport); for `<img>` the rows of `rect`.
    pub span: (LU, LU),
    /// In the fixed list (always near).
    pub fixed: bool,
}

/// The way a page scrolls: down for horizontal writing, along the lines for a vertical-rl (to the left, the start is the
/// right edge) or vertical-lr (to the right) root. Scroll positions, the image bands and the anchors are all measured
/// along it ("progress"), so everything that is not about pixels works the same for the three.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScrollAxis {
    #[default]
    Down,
    Left,
    Right,
}

impl ScrollAxis {
    pub fn of(mode: crate::css::types::WritingMode) -> ScrollAxis {
        match (mode.vertical(), mode.lr()) {
            (false, _) => ScrollAxis::Down,
            (true, false) => ScrollAxis::Left,
            (true, true) => ScrollAxis::Right,
        }
    }

    /// Where the viewport's top left corner is on the page (CSS px) after scrolling `p` px along the axis.
    pub fn offset(self, p: f32) -> (f32, f32) {
        match self {
            ScrollAxis::Down => (0.0, p),
            ScrollAxis::Left => (-p, 0.0),
            ScrollAxis::Right => (p, 0.0),
        }
    }

    /// The extent of the rectangle `x, y, w, h` (page coordinates) along the axis, measured from the start of the page: for
    /// `Left` from the right edge of the first viewport (`vw` wide).
    pub fn span_f(self, vw: f32, x: f32, y: f32, w: f32, h: f32) -> (f32, f32) {
        match self {
            ScrollAxis::Down => (y, y + h),
            ScrollAxis::Left => (vw - x - w, vw - x),
            ScrollAxis::Right => (x, x + w),
        }
    }

    /// [`ScrollAxis::span_f`] in layout units.
    pub fn span(self, vw: LU, r: &Rect) -> (LU, LU) {
        match self {
            ScrollAxis::Down => (r.y, r.bottom()),
            ScrollAxis::Left => (vw - r.right(), vw - r.x),
            ScrollAxis::Right => (r.x, r.right()),
        }
    }
}

/// Where one of `LayoutResult::images` sits on the page (same order), for lazy loading. `top` and `bottom` are along the
/// scroll axis.
#[derive(Clone, Copy, Debug)]
pub struct ImageSpot {
    pub top: LU,
    pub bottom: LU,
    /// `loading="lazy"`.
    pub lazy: bool,
}

/// The display list of one layout. Paint order is: fixed items below the flow (`fixed[..fixed_below]`),
/// `items` in order, then the rest of `fixed`.
pub struct DisplayList {
    pub items: Vec<Item>,
    pub fixed: Vec<Item>,
    pub fixed_below: usize,
    pub clips: Vec<Rect>,
    /// Parallel to `clips`: `Some` where the clip or one below it is rounded.
    pub rounds: Vec<Option<Box<Round>>>,
    pub bgs: Vec<BgDraw>,
    pub grads: Vec<gradient::GradPaint>,
    pub borders: Vec<BorderDraw>,
    pub shadows: Vec<ShadowDraw>,
    pub groups: Vec<Group>,
    pub glyphs: Vec<PaintGlyph>,
    /// The canvas background colour (None: white).
    pub canvas: Option<Color>,
    /// The page's height as far as it is laid out (a partial layout gives a temporary value).
    pub height: LU,
    /// The way the page scrolls (the root's writing mode) and, for a vertical root, how far its content reaches on the
    /// left and the right (the root's box and what is drawn, clipped); both zero for a horizontal one.
    pub axis: ScrollAxis,
    pub x_min: LU,
    pub x_max: LU,
    pub images: Vec<ImagePlace>,
    pub spots: Vec<ImageSpot>,
    /// The layout this was built from was stopped early (first-screen-first).
    pub partial: bool,
    index: Index,
}

impl DisplayList {
    pub(crate) fn empty() -> DisplayList {
        DisplayList {
            items: Vec::new(),
            fixed: Vec::new(),
            fixed_below: 0,
            clips: Vec::new(),
            rounds: Vec::new(),
            bgs: Vec::new(),
            grads: Vec::new(),
            borders: Vec::new(),
            shadows: Vec::new(),
            groups: Vec::new(),
            glyphs: Vec::new(),
            canvas: None,
            height: LU::ZERO,
            axis: ScrollAxis::Down,
            x_min: LU::ZERO,
            x_max: LU::ZERO,
            images: Vec::new(),
            spots: Vec::new(),
            partial: false,
            index: Index::default(),
        }
    }

    /// How far the page scrolls along its axis (CSS px) in a viewport of `vw` x `vh`: its height below the first screen, or
    /// how far its lines reach past the edge the first screen starts at (the right edge for vertical-rl).
    pub fn max_progress(&self, vw: f32, vh: f32) -> f32 {
        match self.axis {
            ScrollAxis::Down => (self.height.to_f32() - vh).max(0.0),
            ScrollAxis::Left => (-self.x_min.to_f32()).max(0.0),
            ScrollAxis::Right => (self.x_max.to_f32() - vw).max(0.0),
        }
    }

    /// The part of the page on screen when `progress` px are scrolled along the axis, for the image decisions.
    pub fn band(&self, progress: f32, vw: f32, vh: f32) -> Band {
        Band::new(progress, if self.axis == ScrollAxis::Down { vh } else { vw })
    }

    pub(crate) fn build_index(&mut self) {
        self.index = Index::build(&self.items);
    }

    /// Indices into `items` of the items that may touch the page band `top..bottom`, in paint order.
    pub fn query(&self, top: LU, bottom: LU, out: &mut Vec<u32>) {
        self.index.query(top, bottom, out);
    }

    pub fn clip_rect(&self, clip: u32) -> Option<Rect> {
        self.clips.get(clip as usize).copied()
    }

    /// The rounded shapes that clip `clip` (its own and those of the clips below it), innermost first.
    pub fn round_shapes(&self, clip: u32) -> Vec<(u32, RRect)> {
        let mut out = Vec::new();
        let mut c = clip;
        for _ in 0..64 {
            let Some(Some(r)) = self.rounds.get(c as usize) else { break };
            if let Some(s) = r.shape {
                out.push((c, s));
            }
            c = r.prev;
        }
        out
    }
}

/// A band index: the page is cut into bands 256 px high; each holds the items that touch it. Items that
/// span many bands (page backgrounds) live in a list of their own that every query includes.
#[derive(Default)]
struct Index {
    bands: Vec<Vec<u32>>,
    tall: Vec<u32>,
}

/// 256 px in layout units.
const BAND: i32 = 256 * 64;
/// Items taller than this many bands go to the `tall` list.
const MAX_SPAN: i32 = 8;
/// Bands beyond this (4 million px) are not indexed: such items go to the `tall` list.
const MAX_BANDS: i32 = 1 << 14;

impl Index {
    fn build(items: &[Item]) -> Index {
        let mut idx = Index::default();
        for (i, it) in items.iter().enumerate() {
            let (top, bottom) = (it.bounds.y.raw(), it.bounds.bottom().raw());
            if bottom < 0 {
                continue;
            }
            let (lo, hi) = (top.max(0) / BAND, bottom / BAND);
            if hi >= MAX_BANDS || hi - lo > MAX_SPAN {
                idx.tall.push(i as u32);
                continue;
            }
            let need = hi as usize + 1;
            if idx.bands.len() < need {
                idx.bands.resize_with(need, Vec::new);
            }
            for b in idx.bands.iter_mut().skip(lo as usize).take((hi - lo + 1) as usize) {
                b.push(i as u32);
            }
        }
        idx
    }

    fn query(&self, top: LU, bottom: LU, out: &mut Vec<u32>) {
        out.clear();
        if bottom.raw() >= 0 {
            let lo = (top.raw().max(0) / BAND) as usize;
            let hi = ((bottom.raw() / BAND) as usize).min(self.bands.len().saturating_sub(1));
            if lo <= hi {
                for b in self.bands.iter().skip(lo).take(hi - lo + 1) {
                    out.extend_from_slice(b);
                }
            }
        }
        out.extend_from_slice(&self.tall);
        out.sort_unstable();
        out.dedup();
    }
}
