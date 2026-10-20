//! Layout (layer 1, steps 1b-1 and 1b-2): the box tree, block layout, inline layout, replaced
//! elements, lists, positioning and work metering.
//!
//! Coordinates are [`LayoutUnit`]s (1/64 px). A block box's `x`/`y` are relative to the border box of
//! its parent block; lines, text fragments and atomic inlines (`inline-block`, images) are relative to
//! the border box of the block container that owns them; out-of-flow boxes (`position: absolute |
//! fixed`) are relative to the block container they sit in in the tree (their static-position
//! parent). [`dump::dump_layout`] prints absolute positions; [`LayoutResult::page_pos`] gives them for
//! one box.
//!
//! What it does: block widths, heights, min/max/fit-content, percentages, `box-sizing`, margin
//! collapsing, `overflow` clipping flags, anonymous blocks, inline formatting (white space, line
//! breaking, `line-height`, `vertical-align`, inline box edges, `text-align` incl. `justify`,
//! `text-indent`, spacing, `text-transform`, decorations, `tab-size`), atomic inlines, images with
//! `srcset`, lists, relative / absolute / fixed positioning with paint-order data, the canvas
//! background, form controls and placeholder boxes, and a per-page work budget.

pub mod bidi;
pub mod block;
pub mod boxtree;
pub mod dump;
pub mod flex;
pub mod floats;
pub mod grid;
pub(crate) mod generated;
pub mod image;
pub mod inline;
pub mod intrinsic;
pub mod lists;
pub mod position;
pub mod replaced;
pub mod table;
pub mod units;
pub mod vertical;

use std::rc::Rc;

use crate::css::cascade::StyleTree;
use crate::css::color::Color;
use crate::css::types::{Position, TextDecorationStyle};
use crate::html::atoms::{FxHashMap, a};
use crate::html::dom::{Document, NodeId};
use crate::platform::{Glyph, Platform};

/// A shaped glyph as layout and the display list keep it: the platform's [`Glyph`] without the cluster range (only
/// unit building reads that), and with 16-bit font and glyph ids (a glyph index is 16 bits in OpenType; the platform
/// keeps fewer than 65536 fonts). 16 bytes instead of 28, for every glyph of the page.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlacedGlyph {
    pub font: u16,
    /// Glyph index in `font`; 0 is the font's missing-glyph box.
    pub glyph: u16,
    /// Horizontal advance, px (letter-, word-spacing and justification included).
    pub advance: f32,
    pub offset_x: f32,
    pub offset_y: f32,
}

impl From<&Glyph> for PlacedGlyph {
    fn from(g: &Glyph) -> PlacedGlyph {
        PlacedGlyph { font: u16::try_from(g.font.0).unwrap_or(0), glyph: u16::try_from(g.glyph).unwrap_or(0), advance: g.advance, offset_x: g.offset_x, offset_y: g.offset_y }
    }
}

pub use boxtree::{BoxId, BoxKind, BoxTree, NO_BOX, build_box_tree};
pub use image::{ImageRequest, ImageSizes, ImageState};
pub use table::TableInfo;
pub use units::LayoutUnit;

/// Layout work allowed per page, in work units (about one per glyph cluster, a few per line and per
/// box). A page that needs more is laid out as far as the budget goes; see [`LayoutResult::incomplete`].
/// A 2.4 MB Chinese Wikipedia article uses about a tenth of it.
pub const DEFAULT_WORK_BUDGET: u64 = 20_000_000;

/// Inline formatting contexts nested in one another (an `inline-block` in a line of an `inline-block`
/// in a line ...) are laid out recursively with big stack frames; deeper ones are left empty.
/// (Nested blocks are bounded by `boxtree::MAX_BOX_DEPTH`.) Page threads have 8 MB of stack in
/// release builds; the limit also keeps a 1 MB debug main thread safe.
pub const MAX_PARAGRAPH_DEPTH: u32 = 24;

/// Characters of text shaped per page (a character costs about 130 bytes while its paragraph is laid
/// out, so this keeps hostile pages -- 10 MB without a space -- near 130 MB). Text beyond it is dropped
/// and the layout is flagged incomplete. The Chinese Wikipedia article has about a fifth of it.
pub const MAX_SHAPED_CHARS: u64 = 1_000_000;

#[derive(Clone, Copy, Debug)]
pub struct LayoutOptions {
    pub viewport_w: f32,
    pub viewport_h: f32,
    /// Device pixels per CSS px (picks the `srcset` candidate).
    pub dpr: f32,
    /// Work units allowed for this page.
    pub work_budget: u64,
    /// First-screen-first (L1 task 17): stop normal-flow layout once it reaches this y (CSS px, page
    /// coordinates; the caller passes scroll position + viewport height + one screen). The result is
    /// then [`LayoutResult::partial`]; run the layout again without it for the whole page.
    pub stop_y: Option<f32>,
}

impl Default for LayoutOptions {
    fn default() -> Self {
        LayoutOptions { viewport_w: 1280.0, viewport_h: 1000.0, dpr: 1.0, work_budget: DEFAULT_WORK_BUDGET, stop_y: None }
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
    /// A table cell that is empty with `empty-cells: hide`: no background or border.
    pub hide: bool,
    /// The lines of the block's inline formatting context: `LayoutResult::lines[a..b]`.
    pub lines: (u32, u32),
    /// The list marker of an outside-marker list item: `LayoutResult::markers[m - 1]`, 0 for none.
    pub marker: u32,
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
    /// shaped into `LayoutResult::glyphs[glyphs.0..glyphs.1]` (advances include letter- and
    /// word-spacing and justification), with the decoration lines `LayoutResult::decorations[decor.0..decor.1]`.
    /// `asc` is the distance from the top of the fragment (the content area) to the baseline.
    ///
    /// `v` is the orientation in vertical text (css-writing-modes-3 5): 0 horizontal, 1 sideways (glyphs turned a quarter clockwise,
    /// the baseline `asc` from the left edge of the fragment), 2 upright (`asc` is the offset of the glyphs' baseline below the
    /// middle of their cell), 3 `text-combine-upright` (one horizontal cell; `fs` is the font size it was shaped at).
    Text { text: u32, start: u32, end: u32, glyphs: (u32, u32), decor: (u32, u32), asc: LayoutUnit, v: u8, fs: f32 },
    /// The text of an inside list marker (same fields as `Text`).
    Marker { text: u32, start: u32, end: u32, glyphs: (u32, u32), asc: LayoutUnit },
    /// The forced break itself (zero width).
    Br,
    /// An atomic inline (`inline-block`, image ...): its box is `box_id`, positioned in `LayoutResult::boxes`.
    Atomic,
    /// The border box of an inline element on this line, for its background and border;
    /// `first` / `last`: this line holds the start / end of the element (the edges are drawn there).
    InlineBox { first: bool, last: bool },
}

/// A text fragment (the content area of the text: font ascent + descent high), a `<br>`, an atomic
/// inline or an inline element's box. Positions are relative to the border box of the block that owns the line.
#[derive(Clone, Copy, Debug)]
pub struct Fragment {
    pub box_id: BoxId,
    pub kind: FragKind,
    pub x: LayoutUnit,
    pub y: LayoutUnit,
    pub w: LayoutUnit,
    pub h: LayoutUnit,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DecorKind {
    Underline,
    Overline,
    LineThrough,
}

/// One decoration line of a text fragment: a rectangle `w` wide and `thickness` high at (`x`, `y`)
/// (owner block coordinates), drawn in `style` and `color`.
#[derive(Clone, Copy, Debug)]
pub struct Decoration {
    pub kind: DecorKind,
    pub style: TextDecorationStyle,
    pub color: Color,
    pub x: LayoutUnit,
    pub y: LayoutUnit,
    pub w: LayoutUnit,
    pub thickness: LayoutUnit,
}

/// A positioned box in the order it is painted (L1 task 10): `below` before the normal flow
/// (`z-index` < 0), `above` after it (`z-index` auto / 0 in document order, then positive ones).
/// [`LayoutResult::stacked`] has them per stacking context (CSS2 appendix E).
#[derive(Clone, Copy, Debug)]
pub struct Positioned {
    pub box_id: BoxId,
    pub position: Position,
    pub z_index: i32,
    /// `z-index: auto`: the box paints as if it made a stacking context, but the positioned boxes inside it take part
    /// in its `ctx` (CSS2 appendix E.2).
    pub auto: bool,
    /// The stacking context the box takes part in: the nearest ancestor that makes one (the root, or a positioned box
    /// with a `z-index` or `position: fixed`).
    pub ctx: BoxId,
}

/// The canvas background (CSS2 14.2): the colour and the box it came from (that box does not paint it again).
#[derive(Clone, Copy, Debug)]
pub struct Canvas {
    pub color: Color,
    pub from: BoxId,
}

pub struct LayoutResult {
    pub tree: BoxTree,
    /// Parallel to `tree.boxes`; every block-level, atomic and replaced box gets a position.
    pub boxes: Vec<BoxOut>,
    pub lines: Vec<Line>,
    pub fragments: Vec<Fragment>,
    pub glyphs: Vec<PlacedGlyph>,
    /// The text of every text item after white-space processing (what fragments point into).
    pub texts: Vec<String>,
    pub decorations: Vec<Decoration>,
    /// Outside list markers (see `BoxOut::marker`), relative to the border box of their list item.
    pub markers: Vec<Fragment>,
    /// Out-of-flow boxes: (block container they are positioned in, the box), in document order.
    pub oof: Vec<(BoxId, BoxId)>,
    pub below: Vec<Positioned>,
    pub above: Vec<Positioned>,
    /// The same boxes sorted by (stacking context, `z-index`, document order); see [`LayoutResult::stacked_in`].
    pub stacked: Vec<Positioned>,
    /// Tables that need more than their box (captions, collapsed borders), by box.
    pub tables: FxHashMap<BoxId, TableInfo>,
    /// The used border widths (top, right, bottom, left) of the cells and tables with collapsed borders: their halves of the
    /// edges, which the background origin (the padding box) is inset by.
    pub halves: FxHashMap<BoxId, [LayoutUnit; 4]>,
    pub canvas: Option<Canvas>,
    /// The images the page wants (see [`image`]).
    pub images: Vec<ImageRequest>,
    /// How each image box got its size (see [`replaced::ImageFit`]), by DOM node.
    pub fits: FxHashMap<NodeId, replaced::ImageFit>,
    pub viewport: (LayoutUnit, LayoutUnit),
    /// Some box floats (the painter looks for floats only then).
    pub has_floats: bool,
    /// The work budget ran out: the rest of the page was not laid out (1c shows
    /// "页面太复杂，只显示了一部分").
    pub incomplete: bool,
    /// Work units used.
    pub work: u64,
    /// First-screen-first stopped the layout early (see [`LayoutOptions::stop_y`]): the page below the
    /// stop line is missing and the page height is only what was laid out.
    pub partial: bool,
}

impl LayoutResult {
    /// Free the shaped glyphs and the white-space-processed texts. Only [`crate::paint::build_display_list`] (and the
    /// dumps) read them; hit testing and scroll anchoring use boxes, lines and fragments. A window calls this once the
    /// display list is built.
    pub fn release_text_data(&mut self) {
        self.glyphs = Vec::new();
        self.texts = Vec::new();
    }

    /// The positioned boxes that take part in the stacking context `ctx`, in the order CSS2 appendix E.2 paints them:
    /// negative `z-index` first, then `auto` and 0 in document order, then the positive ones.
    pub fn stacked_in(&self, ctx: BoxId) -> &[Positioned] {
        let (a, b) = (self.stacked.partition_point(|p| p.ctx < ctx), self.stacked.partition_point(|p| p.ctx <= ctx));
        self.stacked.get(a..b).unwrap_or(&[])
    }

    pub fn box_out(&self, id: BoxId) -> BoxOut {
        self.boxes.get(id as usize).copied().unwrap_or_default()
    }

    /// Top-left corner of the border box of `id` in page coordinates (for `position: fixed` boxes:
    /// viewport coordinates at scroll position 0).
    pub fn page_pos(&self, id: BoxId) -> (LayoutUnit, LayoutUnit) {
        let (mut x, mut y) = (LayoutUnit::ZERO, LayoutUnit::ZERO);
        let mut cur = id;
        let mut steps = 0;
        while cur != NO_BOX && steps < 100_000 {
            let o = self.box_out(cur);
            x += o.x;
            y += o.y;
            cur = self.tree.layout_parent(cur);
            steps += 1;
        }
        (x, y)
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
    /// Height of an "x" and advance of "0" (`ex`, `ch`).
    pub x_height: LayoutUnit,
    pub zero_adv: LayoutUnit,
    /// Decoration line positions, px: underline below the baseline, strike-through above it.
    pub underline_offset: f32,
    pub underline_thickness: f32,
    pub strike_offset: f32,
    pub strike_thickness: f32,
}

/// An out-of-flow box waiting for its containing block to be finished (position.rs).
/// How an out-of-flow child of a flex container sits in its static-position rectangle (css-flexbox-1 4.1): the
/// rectangle is `rw` x `rh` from the static position; `ax` / `ay` are 0 (start), 1 (middle) or 2 (end).
#[derive(Clone, Copy)]
pub(crate) struct StaticAlign {
    pub rw: LayoutUnit,
    pub rh: LayoutUnit,
    pub ax: u8,
    pub ay: u8,
}

pub(crate) struct Pending {
    pub id: BoxId,
    /// The block container the box sits in; `(sx, sy)` is the static position in its coordinates.
    pub owner: BoxId,
    pub sx: LayoutUnit,
    pub sy: LayoutUnit,
    /// The containing block: a positioned box, or `None` for the initial containing block.
    pub cb: Option<BoxId>,
    pub align: Option<StaticAlign>,
}

pub(crate) struct Layouter<'a> {
    pub doc: &'a Document,
    pub tree: &'a BoxTree,
    pub platform: &'a dyn Platform,
    pub out: Vec<BoxOut>,
    pub lines: Vec<Line>,
    pub fragments: Vec<Fragment>,
    pub glyphs: Vec<PlacedGlyph>,
    pub texts: Vec<String>,
    pub decorations: Vec<Decoration>,
    pub markers: Vec<Fragment>,
    /// Keyed by the addresses of the shared font group and the language string of the style.
    pub fonts: FxHashMap<(usize, usize), Rc<ResolvedFont>>,
    /// Per box: does the box or an ancestor it passes decoration lines down from declare any?
    pub decorated: Vec<bool>,
    /// Per box: (number of the last line the box was seen on, its entry in that line) -- see `inline.rs`.
    pub stamp: Vec<(u32, u32)>,
    pub line_no: u32,
    pub viewport: (LayoutUnit, LayoutUnit),
    pub work: u64,
    pub budget: u64,
    pub exhausted: bool,
    /// Paragraphs already shaped while measuring (inline.rs).
    pub paras: FxHashMap<BoxId, inline::Prepared>,
    /// Min-content and max-content widths of boxes (intrinsic.rs).
    pub widths: FxHashMap<BoxId, (LayoutUnit, LayoutUnit)>,
    pub pending: Vec<Pending>,
    /// First-screen-first left out an invisible out-of-flow box (its layout waits for the full layout).
    pub deferred_oof: bool,
    pub cb_stack: Vec<BoxId>,
    pub oof_list: Vec<(BoxId, BoxId)>,
    pub scratch: Vec<BoxId>,
    pub images: &'a ImageSizes,
    /// How image boxes got their sizes (replaced.rs).
    pub fits: FxHashMap<NodeId, replaced::ImageFit>,
    /// Nesting of inline formatting contexts being laid out or measured (see `MAX_PARAGRAPH_DEPTH`).
    pub depth: u32,
    /// Characters that may still be shaped on this page.
    pub text_left: u64,
    /// Text was dropped because of the page's text allowance.
    pub truncated: bool,
    /// First-screen-first: the page y (layout units) at which normal-flow layout stops, and whether it did.
    pub stop_at: Option<LayoutUnit>,
    pub stopped: bool,
    /// Estimated page y of the border-box top of the block being laid out (a lower bound: margins that
    /// collapse through it are not known yet). Only the stop line is decided with it.
    pub origin_y: LayoutUnit,
    /// Greater than 0 inside atomic inlines and out-of-flow boxes, which are always laid out whole.
    pub no_stop: u32,
    /// Blocks whose children were cut off by the stop line (their heights are not final).
    pub cut: Vec<BoxId>,
    /// The floats of the block formatting context being laid out (floats.rs).
    pub fs: floats::FloatSpace,
    /// Where the border box of the block being laid out starts, in the coordinates of that formatting context: set by
    /// whoever lays the block out (a guess for blocks in the flow: the real position is only known afterwards).
    pub pos: (LayoutUnit, LayoutUnit),
    /// The layout looked at, or added to, the floats of its formatting context (so it depends on `pos`).
    pub touched: bool,
    /// The page has floats (when it has none none of the float machinery runs).
    pub has_floats: bool,
    /// The grids of the tables (table.rs), by table box.
    pub grids: FxHashMap<BoxId, Rc<table::Grid>>,
    /// The painter's extra data for tables.
    pub tinfo: FxHashMap<BoxId, TableInfo>,
    /// `LayoutResult::halves`.
    pub halves: FxHashMap<BoxId, [LayoutUnit; 4]>,
    /// Flex items measured by a throw-away layout, by box: (the content width it was measured at, its border-box height)
    /// (flex.rs). The type is that of `widths`, which keeps the binary from carrying one more hash map.
    pub measured: FxHashMap<BoxId, (LayoutUnit, LayoutUnit)>,
    /// Per box: 0 not looked at, 1 nothing below it depends on its height, 2 something does (flex.rs).
    pub flex_sens: Vec<u8>,
    /// Work spent laying stretched flex items out a second time.
    pub flex_spent: u64,
    /// The first and last baseline of flex containers (css-flexbox-1 8.5), relative to their border boxes; `LayoutUnit::MIN`
    /// is "none".
    pub flex_base: FxHashMap<BoxId, (LayoutUnit, LayoutUnit)>,
    /// The geometry `layout_frame` computed for the frame root about to be laid out (vertical.rs).
    pub inner_geom: Option<block::Geom>,
    /// The frame root whose content widths are being measured (intrinsic.rs).
    pub in_frame: BoxId,
}

impl Layouter<'_> {
    /// The containing block of a `position: fixed` box: the viewport (`None`), or the nearest transformed ancestor on the
    /// containing-block stack (css-transforms-1 2).
    pub(crate) fn fixed_cb(&self) -> Option<BoxId> {
        self.cb_stack.iter().rev().copied().find(|&b| has_transform(&self.tree.get(b).style))
    }

    /// Count `n` work units. Returns false once the budget is used up.
    pub(crate) fn charge(&mut self, n: u64) -> bool {
        self.work = self.work.saturating_add(n);
        if self.work > self.budget {
            self.exhausted = true;
        }
        !self.exhausted
    }

    /// First-screen-first inside a flex or grid container: run `f`, the layout of one item whose border-box top is at
    /// estimated page y `est`, with a stop line of its own (items laid out side by side must not cut each other off), and
    /// say whether it was cut.
    pub(crate) fn item_stop<R>(&mut self, est: LayoutUnit, f: impl FnOnce(&mut Self) -> R) -> (R, bool) {
        let (was, origin) = (self.stopped, self.origin_y);
        self.stopped = false;
        self.origin_y = est;
        let out = f(self);
        let hit = self.stopped;
        self.stopped = was;
        self.origin_y = origin;
        (out, hit)
    }

    /// Would content starting at estimated page y `y` be left out? (Does not set `stopped`.)
    pub(crate) fn below_stop(&self, y: LayoutUnit) -> bool {
        self.no_stop == 0 && self.stop_at.is_some_and(|s| y >= s)
    }

    /// First-screen-first: should the content starting at estimated page y `y` be left out? Sets
    /// `stopped` the first time it says yes; after that it says yes for everything.
    pub(crate) fn past_stop(&mut self, y: LayoutUnit) -> bool {
        if self.stopped {
            return true;
        }
        match self.stop_at {
            Some(s) if self.no_stop == 0 && y >= s => {
                self.stopped = true;
                true
            }
            _ => false,
        }
    }
}

/// Lay out a styled document with no image sizes known. The platform supplies fonts, shaping and
/// line-break opportunities.
pub fn layout_document(doc: &Document, styles: &StyleTree, platform: &dyn Platform, opts: &LayoutOptions) -> LayoutResult {
    layout_document_with(doc, styles, platform, opts, &ImageSizes::new())
}

/// Lay out a styled document; `images` holds the image sizes the host has learned so far.
pub fn layout_document_with(doc: &Document, styles: &StyleTree, platform: &dyn Platform, opts: &LayoutOptions, images: &ImageSizes) -> LayoutResult {
    let env = boxtree::BuildEnv { viewport_w: opts.viewport_w, viewport_h: opts.viewport_h, dpr: opts.dpr, images };
    let mut tree = boxtree::build_box_tree_with(doc, styles, &env);
    let viewport = (LayoutUnit::from_f32(opts.viewport_w).max0(), LayoutUnit::from_f32(opts.viewport_h).max0());
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
        decorations: Vec::new(),
        markers: Vec::new(),
        fonts: FxHashMap::default(),
        decorated: inline::decorated_boxes(&tree),
        stamp: vec![(0, 0); n],
        line_no: 0,
        viewport,
        work: 0,
        budget: opts.work_budget,
        exhausted: false,
        paras: FxHashMap::default(),
        widths: FxHashMap::default(),
        pending: Vec::new(),
        deferred_oof: false,
        cb_stack: Vec::new(),
        oof_list: Vec::new(),
        scratch: Vec::new(),
        images,
        fits: FxHashMap::default(),
        depth: 0,
        text_left: (opts.work_budget / 2).min(MAX_SHAPED_CHARS),
        truncated: false,
        stop_at: opts.stop_y.map(|y| LayoutUnit::from_f32(y.max(0.0))),
        stopped: false,
        origin_y: LayoutUnit::ZERO,
        no_stop: 0,
        cut: Vec::new(),
        fs: floats::FloatSpace::default(),
        pos: (LayoutUnit::ZERO, LayoutUnit::ZERO),
        touched: false,
        has_floats: tree.boxes.iter().any(floats::is_float),
        grids: FxHashMap::default(),
        tinfo: FxHashMap::default(),
        halves: FxHashMap::default(),
        measured: FxHashMap::default(),
        flex_sens: Vec::new(),
        flex_spent: 0,
        flex_base: FxHashMap::default(),
        inner_geom: None,
        in_frame: NO_BOX,
    };
    l.prefetch_text();
    l.layout_root();
    let has_floats = l.has_floats;
    let tables = std::mem::take(&mut l.tinfo);
    let halves = std::mem::take(&mut l.halves);
    let (boxes, mut lines, mut fragments, glyphs, texts, mut decorations, mut markers, oof, work, incomplete, partial) =
        (l.out, l.lines, l.fragments, l.glyphs, l.texts, l.decorations, l.markers, l.oof_list, l.work, l.exhausted || l.truncated || tree.cut, l.stopped || l.deferred_oof);
    let mut fits = l.fits;
    fits.shrink_to_fit();
    // The arrays grew by doubling; what is kept for the life of the page should not carry the slack.
    tree.boxes.shrink_to_fit();
    lines.shrink_to_fit();
    fragments.shrink_to_fit();
    decorations.shrink_to_fit();
    markers.shrink_to_fit();
    let canvas = canvas_background(doc, &tree);
    let (below, above, stacked) = paint_order(&tree);
    let images = tree.images.clone();
    LayoutResult { tree, boxes, lines, fragments, glyphs, texts, decorations, markers, oof, below, above, stacked, tables, halves, canvas, images, fits, viewport, has_floats, incomplete, work, partial }
}

/// CSS2 14.2: the root element's background covers the canvas; when it has none (transparent), the
/// `body`'s does, and then the `body` does not paint it again.
fn canvas_background(doc: &Document, tree: &BoxTree) -> Option<Canvas> {
    let bg_of = |id: BoxId| -> Option<Color> {
        let s = &tree.get(id).style;
        let c = s.misc.background_color.resolve(s.text.color);
        // A background image alone also takes the canvas (its color is then transparent).
        let image = s.misc.bg.images.iter().any(|i| !matches!(i, crate::css::bg::BgImage::None));
        (c.a != 0 || image).then_some(c)
    };
    let root = tree.root;
    if let Some(color) = bg_of(root) {
        return Some(Canvas { color, from: root });
    }
    if tree.get(root).anonymous {
        return None;
    }
    let body = tree.get(root).children.iter().copied().find(|&c| {
        let b = tree.get(c);
        b.kind == BoxKind::Block && !b.anonymous && doc.is_html_element(b.node, a::BODY)
    })?;
    bg_of(body).map(|color| Canvas { color, from: body })
}

/// The stacking context a box takes part in (CSS2 appendix E.1): the nearest ancestor that makes one.
fn stacking_context(tree: &BoxTree, id: BoxId) -> BoxId {
    let mut p = tree.get(id).parent;
    for _ in 0..100_000 {
        if p == NO_BOX {
            break;
        }
        let b = tree.get(p);
        if matches!(b.kind, BoxKind::Block | BoxKind::InlineBlock | BoxKind::Replaced) && makes_context(&b.style) {
            return p;
        }
        p = b.parent;
    }
    tree.root
}

/// A positioned box with a `z-index` other than `auto`, or `position: fixed`, or a box with an `opacity` below 1 or a
/// `transform`, starts a stacking context (CSS2 appendix E, css-color-4 12.1, css-transforms-1 2).
pub fn makes_context(style: &crate::css::computed::ComputedStyle) -> bool {
    let b = &style.boxs;
    (!matches!(b.position, Position::Static | Position::Sticky) && (b.z_index.is_some() || b.position == Position::Fixed)) || has_effect(style)
}

/// `opacity` below 1 or a `transform`: the box is painted as a group and, if it is not positioned, in the layer of the
/// positioned boxes with `z-index: 0` (css-color-4 12.1; css-transforms-1 2).
pub fn has_effect(style: &crate::css::computed::ComputedStyle) -> bool {
    style.misc.fx.as_deref().is_some_and(|f| f.opacity < 255 || !f.xform.is_empty())
}

/// A `transform` makes the box the containing block of its absolutely positioned and fixed descendants (css-transforms-1 2).
pub fn has_transform(style: &crate::css::computed::ComputedStyle) -> bool {
    style.misc.fx.as_deref().is_some_and(|f| !f.xform.is_empty())
}

/// Is the box the containing block of absolutely positioned descendants (positioned, or transformed)?
pub fn makes_cb(style: &crate::css::computed::ComputedStyle) -> bool {
    style.boxs.position != Position::Static || has_transform(style)
}

/// Positioned boxes in painting order (L1 task 10; CSS2 appendix E.2): `below` / `above` are all of them in one list
/// (`z-index` < 0 first, then auto / 0, then positive ones); `stacked` is the same sorted per stacking context.
fn paint_order(tree: &BoxTree) -> (Vec<Positioned>, Vec<Positioned>, Vec<Positioned>) {
    let mut all: Vec<(i32, u32, Positioned)> = Vec::new();
    for (i, b) in tree.boxes.iter().enumerate() {
        if !matches!(b.kind, BoxKind::Block | BoxKind::InlineBlock | BoxKind::Replaced) {
            continue;
        }
        let mut pos = b.style.boxs.position;
        let plain = matches!(pos, Position::Static | Position::Sticky);
        if plain {
            if !has_effect(&b.style) {
                continue;
            }
            // Not positioned, but painted with the `z-index: 0` positioned boxes (it ignores `z-index`).
            pos = Position::Relative;
        }
        let z = if plain { 0 } else { b.style.boxs.z_index.unwrap_or(0) };
        let p = Positioned { box_id: i as BoxId, position: pos, z_index: z, auto: !makes_context(&b.style), ctx: stacking_context(tree, i as BoxId) };
        all.push((z, b.order, p));
    }
    // Boxes were pushed in box order, so equal (z, order) already keep the box-id order.
    crate::sorting::sort_by_u64_key(&mut all, |&(z, order, _)| (u64::from((z as u32) ^ 0x8000_0000) << 32) | u64::from(order));
    let mut stacked: Vec<Positioned> = all.iter().map(|t| t.2).collect();
    crate::sorting::sort_by_u64_key(&mut stacked, |p| u64::from(p.ctx));
    let (neg, rest): (Vec<_>, Vec<_>) = all.into_iter().partition(|(z, ..)| *z < 0);
    (neg.into_iter().map(|t| t.2).collect(), rest.into_iter().map(|t| t.2).collect(), stacked)
}
