//! Display list builder: walks the layout in paint order (L1 task 10): positioned boxes with a negative
//! `z-index`, then the normal flow in tree order, then the other positioned boxes in document order.
//! Positioned boxes and everything they contain are painted at their turn in that order, not inside the flow;
//! `position: fixed` boxes (and what they contain) go to the fixed list.

use super::{DisplayList, ImagePlace, ImageSpot, Item, ItemKind, NO_CLIP, PaintGlyph, Rect, TextRun};
use crate::css::color::Color;
use crate::css::computed::{ComputedStyle, slant_of};
use crate::css::types::{Position, Visibility};
use crate::html::atoms::FxHashMap;
use crate::html::dom::{Document, NodeId};
use crate::layout::block::{border_px, pad_px};
use crate::layout::boxtree::{BoxId, BoxKind, NO_BOX, ReplacedKind};
use crate::layout::floats::is_float;
use crate::layout::units::LayoutUnit as LU;
use crate::layout::{BoxOut, FragKind, Fragment, LayoutResult, Positioned};
use crate::platform::{FontQuery, Platform};

/// Ancestor walks (clip, fixed) never go further than this.
const MAX_WALK: usize = 400;
/// Pages cannot be taller than this for scrolling purposes (CSS px); hostile offsets are cut here.
const MAX_PAGE_HEIGHT_PX: i32 = 1 << 22;

const GREY_FILL: Color = Color::rgb(0xe0, 0xe0, 0xe0);
const GREY_LINE: Color = Color::rgb(0x76, 0x76, 0x76);
const GREY_TEXT: Color = Color::rgb(0x50, 0x50, 0x50);

fn positioned(style: &ComputedStyle) -> bool {
    !matches!(style.boxs.position, Position::Static | Position::Sticky)
}

/// Boxes with a position of their own in `LayoutResult::boxes`.
fn has_box_out(kind: BoxKind) -> bool {
    matches!(kind, BoxKind::Block | BoxKind::InlineBlock | BoxKind::Replaced)
}

/// Build the display list of a layout. `doc` is read for the `checked` and `loading` attributes; the
/// platform shapes the labels of placeholder boxes.
pub fn build_display_list(doc: &Document, res: &LayoutResult, platform: &dyn Platform) -> DisplayList {
    let mut b = Builder { doc, res, platform, list: DisplayList::empty(), clip: NO_CLIP, fixed: false, canvas_from: res.canvas.map_or(NO_BOX, |c| c.from), floats: Vec::new() };
    b.list.canvas = res.canvas.map(|c| c.color);
    b.list.partial = res.partial;
    let root = res.tree.root;
    for p in &res.below {
        b.positioned(p, root);
    }
    b.list.fixed_below = b.list.fixed.len();
    let (rx, ry) = res.page_pos(root);
    b.fixed = false;
    b.clip = NO_CLIP;
    b.paint_box(root, rx, ry);
    for p in &res.above {
        b.positioned(p, root);
    }
    b.finish()
}

struct Builder<'a> {
    doc: &'a Document,
    res: &'a LayoutResult,
    platform: &'a dyn Platform,
    list: DisplayList,
    /// The clip of the items being added (an index into `list.clips`).
    clip: u32,
    /// Items go to the fixed list.
    fixed: bool,
    canvas_from: BoxId,
    /// Floats met on the way, waiting for the end of their stacking context: (box, page position, clip, fixed).
    floats: Vec<(BoxId, LU, LU, u32, bool)>,
}

impl Builder<'_> {
    // ---- bookkeeping -------------------------------------------------------------------------------

    /// Is anything of `bounds` visible given the current clip?
    fn visible(&self, bounds: &Rect) -> bool {
        match self.list.clip_rect(self.clip) {
            Some(c) => c.intersects(bounds),
            None => true,
        }
    }

    fn push(&mut self, bounds: Rect, kind: ItemKind) {
        if bounds.is_empty() || !self.visible(&bounds) {
            return;
        }
        let item = Item { bounds, clip: self.clip, kind };
        if self.fixed {
            self.list.fixed.push(item);
        } else {
            self.list.items.push(item);
        }
    }

    fn rect(&mut self, bounds: Rect, color: Color) {
        if color.a != 0 {
            self.push(bounds, ItemKind::Rect(color));
        }
    }

    /// Clip further to `r`; returns the previous clip index for [`Self::restore`].
    fn push_clip(&mut self, r: Rect) -> u32 {
        let merged = match self.list.clip_rect(self.clip) {
            Some(c) => c.intersect(&r),
            None => r,
        };
        self.list.clips.push(merged);
        std::mem::replace(&mut self.clip, (self.list.clips.len() - 1) as u32)
    }

    fn restore(&mut self, prev: u32) {
        self.clip = prev;
    }

    fn in_fixed(&self, id: BoxId) -> bool {
        let tree = &self.res.tree;
        let mut cur = id;
        for _ in 0..MAX_WALK {
            if cur == NO_BOX {
                return false;
            }
            let b = tree.get(cur);
            if b.style.boxs.position == Position::Fixed {
                return true;
            }
            cur = b.parent;
        }
        false
    }

    /// The padding box of a box whose border box is `r`.
    fn padding_box(&self, id: BoxId, r: Rect) -> Rect {
        let bd = &self.res.tree.get(id).style.border;
        let b = |i: usize| border_px(bd.width.get(i).copied().unwrap_or(0.0));
        Rect::new(r.x + b(3), r.y + b(0), (r.w - b(1) - b(3)).max0(), (r.h - b(0) - b(2)).max0())
    }

    /// The clip a positioned box inherits from its ancestors. An overflow clip of an ancestor applies
    /// when the ancestor is, or contains, the box's containing block; for `absolute` that means the
    /// containing block (the nearest positioned ancestor) and everything above it. `fixed` boxes are not clipped.
    fn ancestor_clip(&self, id: BoxId, pos: Position) -> Option<Rect> {
        if pos == Position::Fixed {
            return None;
        }
        let res = self.res;
        let tree = &res.tree;
        let mut acc: Option<Rect> = None;
        let mut counting = pos != Position::Absolute;
        let mut cur = tree.get(id).parent;
        for _ in 0..MAX_WALK {
            if cur == NO_BOX {
                break;
            }
            let b = tree.get(cur);
            if !counting && positioned(&b.style) {
                counting = true;
            }
            if counting && has_box_out(b.kind) {
                let o = res.box_out(cur);
                if o.clip {
                    let (x, y) = res.page_pos(cur);
                    let r = self.padding_box(cur, Rect::new(x, y, o.w, o.h));
                    acc = Some(acc.map_or(r, |a| a.intersect(&r)));
                }
            }
            if b.style.boxs.position == Position::Fixed {
                break;
            }
            cur = b.parent;
        }
        if counting { acc } else { None }
    }

    // ---- positioned boxes ----------------------------------------------------------------------------

    fn positioned(&mut self, p: &Positioned, root: BoxId) {
        let id = p.box_id;
        if id == root {
            return;
        }
        let res = self.res;
        let (x, y) = res.page_pos(id);
        self.fixed = self.in_fixed(id);
        self.clip = NO_CLIP;
        if let Some(r) = self.ancestor_clip(id, p.position) {
            self.push_clip(r);
        }
        self.paint_box(id, x, y);
        self.fixed = false;
        self.clip = NO_CLIP;
    }

    // ---- boxes ----------------------------------------------------------------------------------------

    /// Paint box `id` (block, atomic inline or replaced) whose border box starts at page position (`ax`, `ay`),
    /// and its flow descendants. Positioned descendants are painted later, at their own turn.
    fn paint_box(&mut self, id: BoxId, ax: LU, ay: LU) {
        let first = self.floats.len();
        self.paint_box_inner(id, ax, ay);
        // Floats are painted above the in-flow blocks of their stacking context (CSS2 appendix E.2 step 5, simplified:
        // the text of those blocks is below them too). A float, an atomic inline, a replaced box, a positioned box
        // and the root are such contexts themselves: their floats go on top of them, at their end.
        let bx = self.res.tree.get(id);
        if self.floats.len() > first && (bx.kind != BoxKind::Block || id == self.res.tree.root || positioned(&bx.style) || is_float(bx)) {
            let mut i = first;
            while let Some(&(c, x, y, clip, fixed)) = self.floats.get(i) {
                i += 1;
                let saved = (self.clip, self.fixed);
                (self.clip, self.fixed) = (clip, fixed);
                self.paint_box(c, x, y);
                (self.clip, self.fixed) = saved;
            }
            self.floats.truncate(first);
        }
    }

    fn paint_box_inner(&mut self, id: BoxId, ax: LU, ay: LU) {
        let res = self.res;
        let bx = res.tree.get(id);
        let o = res.box_out(id);
        if res.partial && o.w == LU::ZERO && o.h == LU::ZERO && o.x == LU::ZERO && o.y == LU::ZERO && o.lines.0 == o.lines.1 {
            // Never laid out (below the first-screen stop line).
            return;
        }
        let rect = Rect::new(ax, ay, o.w, o.h);
        self.box_decor(id, rect);
        if bx.replaced().is_some() {
            self.replaced(id, rect);
        }
        let prev = o.clip.then(|| {
            let pb = self.padding_box(id, rect);
            self.push_clip(pb)
        });
        if o.marker > 0
            && let Some(m) = res.markers.get(o.marker as usize - 1)
        {
            self.fragment(m, ax, ay);
        }
        if bx.inline_children {
            self.lines(&o, ax, ay);
            if res.has_floats {
                let mut floats = Vec::new();
                crate::layout::floats::floats_in(&res.tree, id, &mut floats);
                for c in floats {
                    let co = res.box_out(c);
                    self.floats.push((c, ax + co.x, ay + co.y, self.clip, self.fixed));
                }
            }
        } else {
            for &c in &bx.children {
                let cb = res.tree.get(c);
                if cb.oof || !has_box_out(cb.kind) || positioned(&cb.style) {
                    continue;
                }
                let co = res.box_out(c);
                self.paint_box(c, ax + co.x, ay + co.y);
            }
        }
        if let Some(p) = prev {
            self.restore(p);
        }
    }

    fn visible_style(style: &ComputedStyle) -> bool {
        style.text.visibility == Visibility::Visible
    }

    /// Background and borders of a block-level or atomic box.
    fn box_decor(&mut self, id: BoxId, r: Rect) {
        let style = self.res.tree.get(id).style.clone();
        if !Self::visible_style(&style) {
            return;
        }
        if id != self.canvas_from {
            self.rect(r, style.misc.background_color.resolve(style.text.color));
        }
        self.borders(&style, r, [true; 4]);
    }

    /// The four border rectangles of a box; `sides` is top, right, bottom, left. Top and bottom run the
    /// full width, left and right fill what is between them (no mitred corners).
    fn borders(&mut self, style: &ComputedStyle, r: Rect, sides: [bool; 4]) {
        let bd = &style.border;
        let w = |i: usize| {
            if sides.get(i).copied().unwrap_or(false) { border_px(bd.width.get(i).copied().unwrap_or(0.0)) } else { LU::ZERO }
        };
        let col = |i: usize| bd.color.get(i).map_or(Color::TRANSPARENT, |c| c.resolve(style.text.color));
        let (t, rt, b, l) = (w(0), w(1), w(2), w(3));
        if t > LU::ZERO {
            self.rect(Rect::new(r.x, r.y, r.w, t), col(0));
        }
        if b > LU::ZERO {
            self.rect(Rect::new(r.x, r.bottom() - b, r.w, b), col(2));
        }
        let mid = (r.h - t - b).max0();
        if l > LU::ZERO {
            self.rect(Rect::new(r.x, r.y + t, l, mid), col(3));
        }
        if rt > LU::ZERO {
            self.rect(Rect::new(r.right() - rt, r.y + t, rt, mid), col(1));
        }
    }

    fn lines(&mut self, o: &BoxOut, ax: LU, ay: LU) {
        let res = self.res;
        for li in o.lines.0..o.lines.1 {
            let Some(l) = res.lines.get(li as usize) else { continue };
            for fi in l.frags.0..l.frags.1 {
                if let Some(f) = res.fragments.get(fi as usize) {
                    self.fragment(f, ax, ay);
                }
            }
        }
    }

    fn fragment(&mut self, f: &Fragment, ax: LU, ay: LU) {
        let res = self.res;
        match f.kind {
            FragKind::Text { glyphs, decor, asc, .. } => {
                self.text(f, ax, ay, glyphs, asc);
                let style = res.tree.get(f.box_id).style.clone();
                if Self::visible_style(&style) {
                    for di in decor.0..decor.1 {
                        if let Some(d) = res.decorations.get(di as usize) {
                            self.rect(Rect::new(ax + d.x, ay + d.y, d.w, d.thickness), d.color);
                        }
                    }
                }
            }
            FragKind::Marker { glyphs, asc, .. } => self.text(f, ax, ay, glyphs, asc),
            FragKind::InlineBox { first, last } => {
                let style = res.tree.get(f.box_id).style.clone();
                if Self::visible_style(&style) {
                    let r = Rect::new(ax + f.x, ay + f.y, f.w, f.h);
                    self.rect(r, style.misc.background_color.resolve(style.text.color));
                    self.borders(&style, r, [true, last, true, first]);
                }
            }
            FragKind::Atomic => {
                if !positioned(&res.tree.get(f.box_id).style) {
                    self.paint_box(f.box_id, ax + f.x, ay + f.y);
                }
            }
            FragKind::Br => {}
        }
    }

    fn text(&mut self, f: &Fragment, ax: LU, ay: LU, glyphs: (u32, u32), asc: LU) {
        let res = self.res;
        let style = &res.tree.get(f.box_id).style;
        let color = style.text.color;
        if !Self::visible_style(style) || color.a == 0 {
            return;
        }
        let Some(gs) = res.glyphs.get(glyphs.0 as usize..glyphs.1 as usize) else { return };
        if gs.is_empty() {
            return;
        }
        let size = style.font.size;
        let half = LU::from_f32(size * 0.5);
        let (x, top) = (ax + f.x, ay + f.y);
        let bounds = Rect::new(x - half, top - half, f.w + half + half, f.h + half + half);
        let paint = gs.iter().map(|g| PaintGlyph { font: g.font, glyph: g.glyph, advance: g.advance, dx: g.offset_x, dy: g.offset_y });
        self.push_text(bounds, x, top + asc, size, color, paint);
    }

    fn push_text(&mut self, bounds: Rect, x: LU, baseline: LU, size: f32, color: Color, glyphs: impl Iterator<Item = PaintGlyph>) {
        if bounds.is_empty() || !self.visible(&bounds) {
            return;
        }
        let start = self.list.glyphs.len() as u32;
        self.list.glyphs.extend(glyphs);
        let run = TextRun { x, baseline, size, color, glyphs: (start, self.list.glyphs.len() as u32) };
        self.push(bounds, ItemKind::Text(run));
    }

    // ---- replaced boxes -------------------------------------------------------------------------------

    fn replaced(&mut self, id: BoxId, r: Rect) {
        let res = self.res;
        let bx = res.tree.get(id);
        let style = bx.style.clone();
        if !Self::visible_style(&style) {
            return;
        }
        // The content box: the border box without borders and padding. Percentage padding refers to the
        // containing block's width; the parent block's border-box width stands in for it.
        let cb_w = res.box_out(res.tree.layout_parent(id)).w;
        let (bd, bs) = (&style.border, &style.boxs);
        let b = |i: usize| border_px(bd.width.get(i).copied().unwrap_or(0.0));
        let p = |i: usize| bs.padding.get(i).map_or(LU::ZERO, |l| pad_px(l, cb_w));
        let (l, t, rt, bt) = (b(3) + p(3), b(0) + p(0), b(1) + p(1), b(2) + p(2));
        let content = Rect::new(r.x + l, r.y + t, (r.w - l - rt).max0(), (r.h - t - bt).max0());
        match bx.replaced() {
            Some(ReplacedKind::Image) => {
                if bx.image().is_some() && !content.is_empty() && self.visible(&content) {
                    self.push(content, ItemKind::Image { node: bx.node });
                    self.list.images.push(ImagePlace { node: bx.node, rect: content, fixed: self.fixed });
                }
            }
            Some(ReplacedKind::Placeholder(name)) => {
                // A thin border and the name, no fill: a placeholder must never hide what is under it (mdBook's
                // `position: fixed` sidebar frame covered 300 px of the text).
                let one = LU::px(1);
                if content.w > one + one && content.h > one + one {
                    self.rect(Rect::new(content.x, content.y, content.w, one), GREY_LINE);
                    self.rect(Rect::new(content.x, content.y + content.h - one, content.w, one), GREY_LINE);
                    self.rect(Rect::new(content.x, content.y + one, one, content.h - one - one), GREY_LINE);
                    self.rect(Rect::new(content.x + content.w - one, content.y + one, one, content.h - one - one), GREY_LINE);
                }
                self.label(id, content, name);
            }
            Some(ReplacedKind::Check) => {
                self.rect(content, GREY_LINE);
                let one = LU::px(1);
                let inner = Rect::new(content.x + one, content.y + one, (content.w - one - one).max0(), (content.h - one - one).max0());
                self.rect(inner, Color::WHITE);
                let checked = self.doc.atoms.lookup("checked").is_some_and(|a| self.doc.attr(bx.node, a).is_some());
                if checked {
                    let three = LU::px(3);
                    self.rect(Rect::new(content.x + three, content.y + three, (content.w - three - three).max0(), (content.h - three - three).max0()), Color::rgb(0x33, 0x33, 0x33));
                }
            }
            Some(ReplacedKind::Fixed(..)) => {
                self.rect(content, GREY_LINE);
                let one = LU::px(1);
                self.rect(Rect::new(content.x + one, content.y + one, (content.w - one - one).max0(), (content.h - one - one).max0()), GREY_FILL);
            }
            None => {}
        }
    }

    /// Write `text` in the font of box `id` near the top left corner of `r`, clipped to `r`.
    fn label(&mut self, id: BoxId, r: Rect, text: &str) {
        let style = self.res.tree.get(id).style.clone();
        let f = &style.font;
        let q = FontQuery { families: &f.family, weight: f.weight, slant: slant_of(f.style), lang: style.text.lang.as_deref() };
        let Some(font) = self.platform.resolve_font(&q) else { return };
        let ascent = self.platform.font_metrics(font, f.size).ascent;
        let gs = self.platform.shape(text, &q, f.size);
        let glyphs = gs.iter().map(crate::layout::PlacedGlyph::from).map(|g| PaintGlyph { font: g.font, glyph: g.glyph, advance: g.advance, dx: g.offset_x, dy: g.offset_y });
        let pad = LU::px(4);
        let (x, base) = (r.x + pad, r.y + pad + LU::from_f32(ascent).round_px());
        let prev = self.push_clip(r);
        let bounds = Rect::new(r.x, r.y, r.w, r.h);
        self.push_text(bounds, x, base, f.size, GREY_TEXT, glyphs);
        self.restore(prev);
    }

    // ---- the end ---------------------------------------------------------------------------------------

    fn finish(mut self) -> DisplayList {
        let res = self.res;
        let cap = LU::px(MAX_PAGE_HEIGHT_PX);
        let root = res.box_out(res.tree.root);
        let mut height = (root.y + root.h).min(cap);
        for it in &self.list.items {
            let bottom = match self.list.clip_rect(it.clip) {
                Some(c) => it.bounds.bottom().min(c.bottom()),
                None => it.bounds.bottom(),
            };
            height = height.max(bottom.min(cap));
        }
        self.list.height = height;
        self.list.spots = self.spots();
        // The arrays grew by doubling; the list lives as long as the page.
        let l = &mut self.list;
        l.items.shrink_to_fit();
        l.fixed.shrink_to_fit();
        l.clips.shrink_to_fit();
        l.glyphs.shrink_to_fit();
        l.images.shrink_to_fit();
        self.list.build_index();
        self.list
    }

    /// Where each requested image sits on the page (same order as `LayoutResult::images`).
    fn spots(&self) -> Vec<ImageSpot> {
        let res = self.res;
        if res.images.is_empty() {
            return Vec::new();
        }
        let mut want: FxHashMap<NodeId, usize> = FxHashMap::default();
        for (i, r) in res.images.iter().enumerate() {
            want.entry(r.node).or_insert(i);
        }
        let mut found: Vec<Option<BoxId>> = vec![None; res.images.len()];
        for (bi, b) in res.tree.boxes.iter().enumerate() {
            if !matches!(b.kind, BoxKind::Replaced | BoxKind::Inline) {
                continue;
            }
            if let Some(&i) = want.get(&b.node)
                && let Some(slot) = found.get_mut(i)
                && slot.is_none()
            {
                *slot = Some(bi as BoxId);
            }
        }
        let lazy_atom = self.doc.atoms.lookup("loading");
        res.images
            .iter()
            .enumerate()
            .map(|(i, r)| {
                let lazy = lazy_atom.and_then(|a| self.doc.attr(r.node, a)).is_some_and(|v| v.trim().eq_ignore_ascii_case("lazy"));
                let always = ImageSpot { top: LU::MIN, bottom: LU::MAX, lazy };
                let Some(Some(bid)) = found.get(i).copied() else { return always };
                if self.in_fixed(bid) {
                    return always;
                }
                // A box that has no position of its own (the alt-text box of a broken image) sits in its block.
                let owner = if has_box_out(res.tree.get(bid).kind) { bid } else { res.tree.layout_parent(bid) };
                let o = res.box_out(owner);
                if res.partial && o.w == LU::ZERO && o.h == LU::ZERO && o.x == LU::ZERO && o.y == LU::ZERO {
                    // Below the first-screen stop line: not placed yet, so not near anything.
                    return ImageSpot { top: LU::MAX, bottom: LU::MAX, lazy };
                }
                let (_, y) = res.page_pos(owner);
                ImageSpot { top: y, bottom: y + o.h, lazy }
            })
            .collect()
    }
}
