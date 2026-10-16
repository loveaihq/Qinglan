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

pub mod build;
pub mod decode;
pub mod images;
pub mod raster;

use crate::css::color::Color;
use crate::html::dom::NodeId;
use crate::layout::units::LayoutUnit as LU;

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
}

#[derive(Clone, Copy, Debug)]
pub enum ItemKind {
    /// A solid rectangle filling `Item::bounds`.
    Rect(Color),
    /// The image of this DOM node, stretched over `Item::bounds`.
    Image { node: NodeId },
    Text(TextRun),
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
    /// In the fixed list (always near).
    pub fixed: bool,
}

/// Where one of `LayoutResult::images` sits on the page (same order), for lazy loading.
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
    pub glyphs: Vec<PaintGlyph>,
    /// The canvas background colour (None: white).
    pub canvas: Option<Color>,
    /// The page's height as far as it is laid out (a partial layout gives a temporary value).
    pub height: LU,
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
            glyphs: Vec::new(),
            canvas: None,
            height: LU::ZERO,
            images: Vec::new(),
            spots: Vec::new(),
            partial: false,
            index: Index::default(),
        }
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
