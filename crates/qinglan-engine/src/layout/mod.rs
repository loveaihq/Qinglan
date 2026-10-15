//! Layout (layer 1, step 1b-1): the box tree, block layout and basic inline layout.
//!
//! Coordinates are [`LayoutUnit`]s (1/64 px). A block box's `x`/`y` are relative to the border box of
//! its parent; lines and text fragments are relative to the border box of the block container that
//! owns them. [`dump::dump_layout`] prints absolute positions.
//!
//! What 1b-1 does: block widths, heights, min/max, percentages, `box-sizing`, margin collapsing,
//! `overflow` clipping flags, anonymous blocks, and inline formatting with white-space processing,
//! line breaking, `line-height`, the strut, `text-align: left/center/right` and `<br>`.
//! What 1b-2 adds (the code is arranged for it): `vertical-align`, inline box padding/border/margin,
//! `justify`, `text-indent`, `letter-spacing`, `word-spacing`, `inline-block`, replaced elements, lists,
//! positioning, form controls and work metering.

pub mod block;
pub mod boxtree;
pub mod dump;
pub mod inline;
pub mod units;

use std::rc::Rc;

use crate::css::cascade::StyleTree;
use crate::html::atoms::FxHashMap;
use crate::html::dom::Document;
use crate::platform::{Glyph, Platform};

pub use boxtree::{BoxId, BoxKind, BoxTree, NO_BOX, build_box_tree};
pub use units::LayoutUnit;

#[derive(Clone, Copy, Debug)]
pub struct LayoutOptions {
    pub viewport_w: f32,
    pub viewport_h: f32,
}

impl Default for LayoutOptions {
    fn default() -> Self {
        LayoutOptions { viewport_w: 1280.0, viewport_h: 1000.0 }
    }
}

/// Where a block box ended up. `x`/`y` are relative to the parent's border box, `w`/`h` are the
/// size of the border box.
#[derive(Clone, Copy, Default, Debug)]
pub struct BoxOut {
    pub x: LayoutUnit,
    pub y: LayoutUnit,
    pub w: LayoutUnit,
    pub h: LayoutUnit,
    /// `overflow: hidden/clip`: content is clipped to the padding box.
    pub clip: bool,
    /// The lines of the block's inline formatting context: `LayoutResult::lines[a..b]`.
    pub lines: (u32, u32),
}

/// One line box. Positions are relative to the border box of the owning block.
#[derive(Clone, Copy, Debug)]
pub struct Line {
    pub x: LayoutUnit,
    pub y: LayoutUnit,
    pub w: LayoutUnit,
    pub h: LayoutUnit,
    /// Distance from the top of the line to the baseline.
    pub baseline: LayoutUnit,
    /// The fragments of the line: `LayoutResult::fragments[a..b]`.
    pub frags: (u32, u32),
    /// The line ends with a forced break (`<br>`, a preserved line feed).
    pub forced: bool,
}

#[derive(Clone, Copy, Debug)]
pub enum FragKind {
    /// A piece of text from one text box on one line: bytes `start..end` of `LayoutResult::texts[text]`,
    /// shaped into `LayoutResult::glyphs[glyphs.0..glyphs.1]`.
    Text { text: u32, start: u32, end: u32, glyphs: (u32, u32) },
    /// The forced break itself (zero width).
    Br,
}

/// A text fragment (the content area of the text: font ascent + descent high) or a `<br>`.
/// Positions are relative to the border box of the block that owns the line.
#[derive(Clone, Copy, Debug)]
pub struct Fragment {
    pub box_id: BoxId,
    pub kind: FragKind,
    pub x: LayoutUnit,
    pub y: LayoutUnit,
    pub w: LayoutUnit,
    pub h: LayoutUnit,
}

pub struct LayoutResult {
    pub tree: BoxTree,
    /// Parallel to `tree.boxes`; only block boxes get positions.
    pub boxes: Vec<BoxOut>,
    pub lines: Vec<Line>,
    pub fragments: Vec<Fragment>,
    pub glyphs: Vec<Glyph>,
    /// The text of every text item after white-space processing (what fragments point into).
    pub texts: Vec<String>,
    pub viewport: (LayoutUnit, LayoutUnit),
}

impl LayoutResult {
    pub fn box_out(&self, id: BoxId) -> BoxOut {
        self.boxes.get(id as usize).copied().unwrap_or_default()
    }
}

/// A font as layout needs it: metrics rounded to whole px (like Blink) and the line-height parts.
pub(crate) struct ResolvedFont {
    pub size: f32,
    pub asc: LayoutUnit,
    pub desc: LayoutUnit,
    /// Half-leading above and below the content area for the element's `line-height`.
    pub lead_top: LayoutUnit,
    pub lead_bottom: LayoutUnit,
}

pub(crate) struct Layouter<'a> {
    pub doc: &'a Document,
    pub tree: &'a BoxTree,
    pub platform: &'a dyn Platform,
    pub out: Vec<BoxOut>,
    pub lines: Vec<Line>,
    pub fragments: Vec<Fragment>,
    pub glyphs: Vec<Glyph>,
    pub texts: Vec<String>,
    pub fonts: FxHashMap<(usize, Option<Rc<str>>), Rc<ResolvedFont>>,
    /// Per box: the number of the last line the box contributed to (see `inline.rs`).
    pub stamp: Vec<u32>,
    pub line_no: u32,
    pub viewport: (LayoutUnit, LayoutUnit),
}

/// Lay out a styled document. The platform supplies fonts, shaping and line-break opportunities.
pub fn layout_document(doc: &Document, styles: &StyleTree, platform: &dyn Platform, opts: &LayoutOptions) -> LayoutResult {
    let tree = build_box_tree(doc, styles);
    let viewport = (LayoutUnit::from_f32(opts.viewport_w).max0(), LayoutUnit::from_f32(opts.viewport_h).max0());
    let (boxes, lines, fragments, glyphs, texts) = {
        let n = tree.boxes.len();
        let mut l = Layouter {
            doc,
            tree: &tree,
            platform,
            out: vec![BoxOut::default(); n],
            lines: Vec::new(),
            fragments: Vec::new(),
            glyphs: Vec::new(),
            texts: Vec::new(),
            fonts: FxHashMap::default(),
            stamp: vec![0; n],
            line_no: 0,
            viewport,
        };
        l.layout_root();
        (l.out, l.lines, l.fragments, l.glyphs, l.texts)
    };
    LayoutResult { tree, boxes, lines, fragments, glyphs, texts, viewport }
}
