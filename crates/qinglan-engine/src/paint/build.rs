//! Display list builder: walks the layout in paint order (CSS2 appendix E.2). Every stacking context (the root, a
//! positioned box with a `z-index`, a `position: fixed` box) paints its own background, the contexts of negative
//! `z-index`, the in-flow blocks, the floats, the in-flow inline content, then the positioned boxes of `z-index` auto
//! or 0 in document order and the positive ones. A positioned box and everything in its flow is painted at its turn
//! in the context it belongs to, not inside the flow; `position: fixed` boxes (and what they contain) go to the fixed list.

use std::rc::Rc;

use super::{BgDraw, BorderDraw, DisplayList, ImagePlace, ImageSpot, Item, ItemKind, NO_CLIP, PaintGlyph, RRect, Rect, Round, TextRun, gradient};
use crate::css::bg::{Backgrounds, BgBox, BgImage, BgRepeat, BgSize, Pos1, Rep};
use crate::css::color::Color;
use crate::css::computed::{ComputedStyle, slant_of};
use crate::css::types::{BorderCollapse, BorderLineStyle, Position, Visibility};
use crate::css::values::Lp;
use crate::html::atoms::FxHashMap;
use crate::html::dom::{Document, NONE, NodeId};
use crate::layout::block::{border_px, pad_px};
use crate::layout::boxtree::{BgRef, BoxId, BoxKind, NO_BOX, ReplacedKind, Role, role_of};
use crate::layout::units::LayoutUnit as LU;
use crate::layout::{BoxOut, FragKind, Fragment, LayoutResult, Positioned, makes_context};
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
    let mut b = Builder { doc, res, platform, list: DisplayList::empty(), clip: NO_CLIP, fixed: false, canvas_from: res.canvas.map_or(NO_BOX, |c| c.from), floats: Vec::new(), outlines: Vec::new() };
    b.list.canvas = res.canvas.map(|c| c.color);
    b.list.partial = res.partial;
    let root = res.tree.root;
    let (rx, ry) = res.page_pos(root);
    b.fixed = false;
    b.clip = NO_CLIP;
    b.paint_box(root, rx, ry);
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
    /// Outlines met on the way, waiting for the end of their stacking context: (outline, clip, fixed).
    outlines: Vec<(BorderDraw, u32, bool)>,
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
        let round = self.has_round(self.clip).then(|| Box::new(Round { shape: None, prev: self.clip }));
        self.list.rounds.push(round);
        self.list.clips.push(merged);
        std::mem::replace(&mut self.clip, (self.list.clips.len() - 1) as u32)
    }

    fn has_round(&self, clip: u32) -> bool {
        self.list.rounds.get(clip as usize).is_some_and(Option::is_some)
    }

    /// Clip further to the rounded rectangle `r` (corner radii in CSS px); a rectangle when no corner is round.
    fn push_round(&mut self, r: Rect, radii: [(f32, f32); 4]) -> u32 {
        let prev = self.push_clip(r);
        if radii.iter().any(|(a, b)| *a > 0.0 && *b > 0.0) {
            let below = if self.has_round(prev) { prev } else { NO_CLIP };
            if let Some(slot) = self.list.rounds.get_mut(self.clip as usize) {
                *slot = Some(Box::new(Round { shape: Some(RRect { rect: r, radii }), prev: below }));
            }
        }
        prev
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

    /// Does `id` make a stacking context (the root, or a positioned box with a `z-index`, or a fixed one)?
    fn is_context(&self, id: BoxId) -> bool {
        id == self.res.tree.root || makes_context(&self.res.tree.get(id).style)
    }

    /// The positioned boxes of the stacking context `ctx` that go below its flow (`neg`: negative `z-index`) or above it.
    fn stacked(&mut self, ctx: BoxId, neg: bool) {
        let res = self.res;
        for p in res.stacked_in(ctx) {
            if (p.z_index < 0) == neg {
                self.positioned(p, res.tree.root);
            }
        }
    }

    fn positioned(&mut self, p: &Positioned, root: BoxId) {
        let id = p.box_id;
        if id == root {
            return;
        }
        let saved = (self.fixed, self.clip);
        let res = self.res;
        let (x, y) = res.page_pos(id);
        self.fixed = self.in_fixed(id);
        self.clip = NO_CLIP;
        if let Some(r) = self.ancestor_clip(id, p.position) {
            self.push_clip(r);
        }
        self.paint_box(id, x, y);
        (self.fixed, self.clip) = saved;
    }

    // ---- boxes ----------------------------------------------------------------------------------------

    /// Paint box `id`, the root of a stacking context (the root, a positioned box, a float, an atomic inline or a
    /// replaced box; a plain in-flow block is only reached through [`Self::walk`]), whose border box starts at page
    /// position (`ax`, `ay`), and its flow descendants. Positioned descendants are painted later, at their own turn.
    ///
    /// CSS2 appendix E.2: the backgrounds and borders of the in-flow blocks first (step 3), then the floats (step 4,
    /// each one as a context of its own), then the in-flow inline content, text included (step 7). Two walks of the
    /// block tree make that order without a buffer: one for the blocks, one for the lines.
    fn paint_box(&mut self, id: BoxId, ax: LU, ay: LU) {
        let first = self.floats.len();
        let first_outline = self.outlines.len();
        self.walk(id, ax, ay, false);
        let mut i = first;
        while let Some(&(c, x, y, clip, fixed)) = self.floats.get(i) {
            i += 1;
            let saved = (self.clip, self.fixed);
            (self.clip, self.fixed) = (clip, fixed);
            self.paint_box(c, x, y);
            (self.clip, self.fixed) = saved;
        }
        self.floats.truncate(first);
        self.walk(id, ax, ay, true);
        if self.is_context(id) {
            self.stacked(id, false);
        }
        self.flush_outlines(first_outline);
    }

    /// One walk over the in-flow blocks of a stacking context: `inline` false paints the decoration of the blocks
    /// and queues the floats, true paints the lines (and list markers, atomic inlines).
    fn walk(&mut self, id: BoxId, ax: LU, ay: LU, inline: bool) {
        let res = self.res;
        let bx = res.tree.get(id);
        let o = res.box_out(id);
        if res.partial && o.w == LU::ZERO && o.h == LU::ZERO && o.x == LU::ZERO && o.y == LU::ZERO && o.lines.0 == o.lines.1 {
            // Never laid out (below the first-screen stop line).
            return;
        }
        let rect = Rect::new(ax, ay, o.w, o.h);
        if !inline {
            if !o.hide {
                self.box_decor(id, rect);
            }
            if bx.replaced().is_some() {
                self.replaced(id, rect);
            }
            if self.is_context(id) {
                self.stacked(id, true);
                if id == res.tree.root {
                    self.list.fixed_below = self.list.fixed.len();
                }
            }
        }
        let prev = o.clip.then(|| self.clip_box(id, rect));
        if inline
            && o.marker > 0
            && let Some(m) = res.markers.get(o.marker as usize - 1)
        {
            self.fragment(m, ax, ay);
        }
        if bx.inline_children {
            if inline {
                self.lines(&o, ax, ay);
            } else if res.has_floats {
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
                if cb.flex_item {
                    // A flex or grid item paints atomically like an inline-block (css-flexbox-1 4.3, css-grid-1 9, CSS2 E.2
                    // step 7): background, floats and content together, in the pass of the inline content, in the
                    // `order`-modified document order the items are in already.
                    if inline {
                        self.paint_box(c, ax + co.x, ay + co.y);
                    }
                } else if cb.kind == BoxKind::Block {
                    self.walk(c, ax + co.x, ay + co.y, inline);
                } else if !inline {
                    // A block-level replaced box is a context of its own, painted whole at its place among the blocks.
                    self.paint_box(c, ax + co.x, ay + co.y);
                }
            }
            // The collapsed borders of a table go over its cells' backgrounds (css-tables-3 "collapsed borders").
            if !inline
                && let Some(t) = res.tables.get(&id)
                && Self::visible_style(&bx.style)
            {
                for s in &t.segs {
                    self.rect(Rect::new(ax + s.0, ay + s.1, s.2, s.3), s.4);
                }
            }
        }
        if let Some(p) = prev {
            self.restore(p);
        }
    }

    fn visible_style(style: &ComputedStyle) -> bool {
        style.text.visibility == Visibility::Visible
    }

    /// Background, border and outline of a block-level or atomic box.
    fn box_decor(&mut self, id: BoxId, r: Rect) {
        let res = self.res;
        let bx = res.tree.get(id);
        let style = bx.style.clone();
        if !Self::visible_style(&style) {
            return;
        }
        // A table with captions: the background and border are those of the grid box (table.rs).
        let r = res.tables.get(&id).map_or(r, |t| Rect::new(r.x, r.y + t.y, r.w, t.h));
        // In the collapsing border model the parts of a table have no borders of their own; the table draws the edges.
        let borders = style.text.border_collapse != BorderCollapse::Collapse || matches!(role_of(bx), Role::Other | Role::Caption);
        self.decor(id, &style, r, [true; 4], borders);
    }

    /// Background, borders and outline of box `id` (or of one fragment of an inline box) whose border box is `r`; `sides`
    /// says which borders exist (top, right, bottom, left): a box broken over lines has no border, and no rounded
    /// corner, where it is cut (css-backgrounds-3 `box-decoration-break: slice`, approximated: every fragment gets its
    /// own copy of the background image).
    fn decor(&mut self, id: BoxId, style: &Rc<ComputedStyle>, r: Rect, sides: [bool; 4], borders: bool) {
        let m = &style.misc;
        let canvas = id == self.canvas_from;
        let mut radii = resolve_radii(m.radius.as_deref(), r.w.to_f32(), r.h.to_f32());
        if !sides[3] {
            radii[0] = (0.0, 0.0);
            radii[3] = (0.0, 0.0);
        }
        if !sides[1] {
            radii[1] = (0.0, 0.0);
            radii[2] = (0.0, 0.0);
        }
        let has_img = m.bg.images.iter().any(|i| !matches!(i, BgImage::None));
        let color = m.background_color.resolve(style.text.color);
        if (color.a != 0 && !canvas) || has_img {
            // A plain color in a square border box (nearly every background) is one rectangle.
            if !has_img && radii == [(0.0, 0.0); 4] && matches!(Backgrounds::nth(&m.bg.clip, m.bg.images.len().saturating_sub(1)), Some(BgBox::Border | BgBox::Text)) {
                self.rect(r, color);
            } else {
                self.backgrounds(id, style, r, radii, sides);
            }
        }
        if borders {
            self.border_items(style, r, sides, radii);
        }
        self.outline(style, r, radii);
    }

    /// The background color and layers (css-backgrounds-3 2, 3): the color first, then the layers from the last to the first.
    fn backgrounds(&mut self, id: BoxId, style: &Rc<ComputedStyle>, r: Rect, radii: [(f32, f32); 4], sides: [bool; 4]) {
        let res = self.res;
        let bg = &style.misc.bg;
        let bd = &style.border;
        let canvas = id == self.canvas_from;
        // In the collapsing border model the parts of a table have no borders of their own: the table draws the edges, and the
        // background reaches to the middle of them (the border box). A cell and the table are inset by their halves of the
        // edges for the padding box (where the image is placed); rows and columns have none.
        let collapsed = style.text.border_collapse == BorderCollapse::Collapse && !matches!(role_of(res.tree.get(id)), Role::Other | Role::Caption);
        let halves = if collapsed { res.halves.get(&id) } else { None };
        let bw: [LU; 4] = std::array::from_fn(|i| match (sides.get(i).copied().unwrap_or(false), halves) {
            (false, _) => LU::ZERO,
            (true, Some(h)) => h.get(i).copied().unwrap_or(LU::ZERO),
            (true, None) if collapsed => LU::ZERO,
            (true, None) => border_px(bd.width.get(i).copied().unwrap_or(0.0)),
        });
        let pb = Rect::new(r.x + bw[3], r.y + bw[0], (r.w - bw[1] - bw[3]).max0(), (r.h - bw[0] - bw[2]).max0());
        let cb_w = res.box_out(res.tree.layout_parent(id)).w;
        let pad: [LU; 4] = std::array::from_fn(|i| if sides.get(i).copied().unwrap_or(false) { style.boxs.padding.get(i).map_or(LU::ZERO, |l| pad_px(l, cb_w)) } else { LU::ZERO });
        let content = Rect::new(pb.x + pad[3], pb.y + pad[0], (pb.w - pad[1] - pad[3]).max0(), (pb.h - pad[0] - pad[2]).max0());
        let bwf = bw.map(LU::to_f32);
        let bpf: [f32; 4] = std::array::from_fn(|i| bwf.get(i).copied().unwrap_or(0.0) + pad.get(i).map_or(0.0, |p| p.to_f32()));
        let box_of = |b: BgBox| match b {
            BgBox::Border | BgBox::Text => (r, radii),
            BgBox::Padding => (pb, inset_radii(radii, bwf)),
            BgBox::Content => (content, inset_radii(radii, bpf)),
        };
        let n = bg.images.len();
        let nth_box = |list: &[BgBox], i: usize| Backgrounds::nth(list, i).copied().unwrap_or(BgBox::Border);
        if !canvas {
            let color = style.misc.background_color.resolve(style.text.color);
            if color.a != 0 {
                let (rect, rad) = box_of(nth_box(&bg.clip, n.saturating_sub(1)));
                let prev = self.push_round(rect, rad);
                self.rect(rect, color);
                self.restore(prev);
            }
        }
        // The background of the root (or the body) belongs to the canvas: positioned by the root element's box, painted over the
        // whole canvas (CSS2 14.2).
        let (root_rect, canvas_area) = {
            let (rx, ry) = res.page_pos(res.tree.root);
            let ro = res.box_out(res.tree.root);
            (Rect::new(rx, ry, ro.w, ro.h), Rect::new(LU::ZERO, LU::ZERO, res.viewport.0.max(rx + ro.w), res.viewport.1.max(ry + ro.h)))
        };
        let refs = res.tree.bgs.get(&id);
        for i in (0..n).rev() {
            let Some(img) = bg.images.get(i) else { continue };
            let (nat, node) = match img {
                BgImage::None => continue,
                BgImage::Gradient(_) => (None, NONE),
                BgImage::Url(_) => match refs.and_then(|v| v.iter().find(|b| usize::from(b.layer) == i)) {
                    Some(BgRef { nat: Some(n), node, .. }) => (Some((n.0 as f32, n.1 as f32)), *node),
                    _ => continue,
                },
            };
            let (clip_rect, clip_rad) = if canvas { (canvas_area, [(0.0, 0.0); 4]) } else { box_of(nth_box(&bg.clip, i)) };
            let orig = if canvas { root_rect } else { box_of(nth_box(&bg.origin, i)).0 };
            if clip_rect.is_empty() || !self.visible(&clip_rect) {
                continue;
            }
            let (pw, ph) = (orig.w.to_f32(), orig.h.to_f32());
            let size = Backgrounds::nth(&bg.size, i).unwrap_or(&BgSize::AUTO);
            let (tw, th) = match (size, nat) {
                (BgSize::Cover | BgSize::Contain, Some((nw, nh))) if nw > 0.0 && nh > 0.0 => {
                    let (sx, sy) = (pw / nw, ph / nh);
                    let s = if *size == BgSize::Cover { sx.max(sy) } else { sx.min(sy) };
                    (nw * s, nh * s)
                }
                (BgSize::Cover | BgSize::Contain, _) => (pw, ph),
                (BgSize::Lens(a, b), nat) => {
                    let (aw, ah) = (a.as_ref().map(|l| l.resolve(pw)), b.as_ref().map(|l| l.resolve(ph)));
                    match (aw, ah, nat) {
                        (Some(w), Some(h), _) => (w, h),
                        (Some(w), None, Some((nw, nh))) if nw > 0.0 => (w, w * nh / nw),
                        (None, Some(h), Some((nw, nh))) if nh > 0.0 => (h * nw / nh, h),
                        (None, None, Some((nw, nh))) => (nw, nh),
                        (Some(w), None, _) => (w, ph),
                        (None, Some(h), _) => (pw, h),
                        (None, None, None) => (pw, ph),
                    }
                }
            };
            if !(tw.is_finite() && th.is_finite() && tw > 0.0 && th > 0.0) {
                continue;
            }
            let rep = Backgrounds::nth(&bg.repeat, i).copied().unwrap_or(BgRepeat(Rep::Repeat, Rep::Repeat));
            let (px, py) = (Backgrounds::nth(&bg.pos_x, i).unwrap_or(&Pos1::ZERO), Backgrounds::nth(&bg.pos_y, i).unwrap_or(&Pos1::ZERO));
            // Tile size, offset of the first tile from the positioning area and distance between tiles (0: no repeat), per axis.
            let axis = |mode: Rep, area: f32, t: f32, pos: &Pos1| -> (f32, f32, f32) {
                match mode {
                    Rep::Repeat => (t, pos.resolve(area, t), t),
                    Rep::NoRepeat => (t, pos.resolve(area, t), 0.0),
                    Rep::Round => {
                        let t2 = area / (area / t).round().max(1.0);
                        (t2, pos.resolve(area, t2), t2)
                    }
                    Rep::Space => {
                        let k = (area / t).floor();
                        if k >= 2.0 { (t, 0.0, t + (area - k * t) / (k - 1.0)) } else { (t, pos.resolve(area, t), 0.0) }
                    }
                }
            };
            // With `round` on one axis only and an `auto` size on the other, the other axis keeps the image's ratio (3.5).
            let (mut tw, mut th) = (tw, th);
            // A repeating tile is at least one CSS px: a 0.2px tile makes a solid area, as in Chromium.
            if rep.0 != Rep::NoRepeat {
                tw = tw.max(1.0);
            }
            if rep.1 != Rep::NoRepeat {
                th = th.max(1.0);
            }
            let snap = |a: f32, t: f32| a / (a / t).round().max(1.0);
            if nat.is_some() && rep.0 == Rep::Round && rep.1 != Rep::Round && matches!(size, BgSize::Lens(_, None)) {
                let t2 = snap(pw, tw);
                (tw, th) = (t2, th * t2 / tw);
            } else if nat.is_some() && rep.1 == Rep::Round && rep.0 != Rep::Round && matches!(size, BgSize::Lens(None, _)) {
                let t2 = snap(ph, th);
                (tw, th) = (tw * t2 / th, t2);
            }
            let ((tw, ox, sx), (th, oy, sy)) = (axis(rep.0, pw, tw, px), axis(rep.1, ph, th, py));
            let mut draw = BgDraw { node, grad: u32::MAX, tile: (tw, th), origin: (orig.x.to_f32() + ox, orig.y.to_f32() + oy), step: (sx, sy) };
            if let BgImage::Gradient(g) = img {
                let Some(gp) = gradient::resolve(g, tw, th, style.text.color) else { continue };
                draw.grad = self.list.grads.len() as u32;
                self.list.grads.push(gp);
            } else {
                let tile = Rect::new(LU::ZERO, LU::ZERO, LU::from_f32(tw), LU::from_f32(th));
                self.list.images.push(ImagePlace { node, rect: tile, span: (clip_rect.y, clip_rect.bottom()), fixed: self.fixed });
            }
            let bi = self.list.bgs.len() as u32;
            self.list.bgs.push(draw);
            let prev = self.push_round(clip_rect, clip_rad);
            self.push(clip_rect, ItemKind::Bg(bi));
            self.restore(prev);
        }
    }

    /// The borders of a box. Plain solid ones are rectangles (top and bottom run the full width, left and right fill what
    /// is between them); a radius, another style or two neighbouring sides of different colors make a [`BorderDraw`].
    #[allow(clippy::indexing_slicing)] // the indices are the sides 0..4 of arrays of four
    fn border_items(&mut self, style: &ComputedStyle, r: Rect, sides: [bool; 4], radii: [(f32, f32); 4]) {
        let bd = &style.border;
        let w: [LU; 4] = std::array::from_fn(|i| if sides.get(i).copied().unwrap_or(false) { border_px(bd.width.get(i).copied().unwrap_or(0.0)) } else { LU::ZERO });
        let col: [Color; 4] = std::array::from_fn(|i| bd.color.get(i).map_or(Color::TRANSPARENT, |c| c.resolve(style.text.color)));
        let st: [BorderLineStyle; 4] = std::array::from_fn(|i| bd.style.get(i).copied().unwrap_or(BorderLineStyle::None));
        let (t, rt, b, l) = (w[0], w[1], w[2], w[3]);
        if t + rt + b + l == LU::ZERO {
            return;
        }
        let radius = radii.iter().any(|(a, c)| *a > 0.0 && *c > 0.0);
        let solid = (0..4).all(|i| w[i] == LU::ZERO || st[i] == BorderLineStyle::Solid);
        let joined = (0..4).all(|i| {
            let j = (i + 1) % 4;
            w[i] == LU::ZERO || w[j] == LU::ZERO || col[i] == col[j]
        });
        if radius || !solid || !joined {
            if (0..4).any(|i| w[i] > LU::ZERO && col[i].a != 0) {
                self.border_item(BorderDraw { rect: r, w, color: col, style: st, radii });
            }
            return;
        }
        if t > LU::ZERO {
            self.rect(Rect::new(r.x, r.y, r.w, t), col[0]);
        }
        if b > LU::ZERO {
            self.rect(Rect::new(r.x, r.bottom() - b, r.w, b), col[2]);
        }
        let mid = (r.h - t - b).max0();
        if l > LU::ZERO {
            self.rect(Rect::new(r.x, r.y + t, l, mid), col[3]);
        }
        if rt > LU::ZERO {
            self.rect(Rect::new(r.right() - rt, r.y + t, rt, mid), col[1]);
        }
    }

    fn border_item(&mut self, b: BorderDraw) {
        let bounds = b.rect;
        if bounds.is_empty() || !self.visible(&bounds) {
            return;
        }
        let i = self.list.borders.len() as u32;
        self.list.borders.push(b);
        self.push(bounds, ItemKind::Border(i));
    }

    /// `outline` (css-ui-3 6): outside the border box by `outline-offset`, painted on top of the stacking context
    /// (CSS2 E.2 step 10), so it is queued until the context is done.
    fn outline(&mut self, style: &ComputedStyle, r: Rect, radii: [(f32, f32); 4]) {
        let o = &style.misc.outline();
        let color = o.color.resolve(style.text.color);
        if o.width <= 0.0 || o.style == BorderLineStyle::None || color.a == 0 {
            return;
        }
        let w = border_px(o.width);
        let grow = LU::from_f32(o.offset.clamp(-1.0e5, 1.0e5)) + w;
        let rect = Rect::new(r.x - grow, r.y - grow, (r.w + grow + grow).max0(), (r.h + grow + grow).max0());
        let g = grow.to_f32();
        let radii = radii.map(|(a, b)| if a > 0.0 && b > 0.0 { (a + g, b + g) } else { (0.0, 0.0) });
        let d = BorderDraw { rect, w: [w; 4], color: [color; 4], style: [o.style; 4], radii };
        self.outlines.push((d, self.clip, self.fixed));
    }

    fn flush_outlines(&mut self, first: usize) {
        if self.outlines.len() <= first {
            return;
        }
        let queued: Vec<_> = self.outlines.drain(first..).collect();
        let saved = (self.clip, self.fixed);
        for (b, clip, fixed) in queued {
            (self.clip, self.fixed) = (clip, fixed);
            self.border_item(b);
        }
        (self.clip, self.fixed) = saved;
    }

    /// Clip to the padding box of box `id`, rounded by the inner border edge where the box has a radius (css-backgrounds-3 5.3).
    fn clip_box(&mut self, id: BoxId, rect: Rect) -> u32 {
        let pb = self.padding_box(id, rect);
        let style = &self.res.tree.get(id).style;
        let radii = resolve_radii(style.misc.radius.as_deref(), rect.w.to_f32(), rect.h.to_f32());
        let bd = &style.border;
        let bwf: [f32; 4] = std::array::from_fn(|i| border_px(bd.width.get(i).copied().unwrap_or(0.0)).to_f32());
        self.push_round(pb, inset_radii(radii, bwf))
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
                    self.decor(f.box_id, &style, r, [true, last, true, first], true);
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
                    // A radius clips the picture to the rounded inner edge (css-backgrounds-3 5.4).
                    let radii = resolve_radii(style.misc.radius.as_deref(), r.w.to_f32(), r.h.to_f32());
                    let inner = inset_radii(radii, [t.to_f32(), rt.to_f32(), bt.to_f32(), l.to_f32()]);
                    let prev = inner.iter().any(|(a, b)| *a > 0.0 && *b > 0.0).then(|| self.push_round(content, inner));
                    self.push(content, ItemKind::Image { node: bx.node });
                    self.list.images.push(ImagePlace { node: bx.node, rect: content, span: (content.y, content.bottom()), fixed: self.fixed });
                    if let Some(p) = prev {
                        self.restore(p);
                    }
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
        let bg_owner: FxHashMap<NodeId, BoxId> = res.tree.bgs.iter().flat_map(|(b, v)| v.iter().map(move |r| (r.node, *b))).collect();
        res.images
            .iter()
            .enumerate()
            .map(|(i, r)| {
                let lazy = lazy_atom.and_then(|a| self.doc.attr(r.node, a)).is_some_and(|v| v.trim().eq_ignore_ascii_case("lazy"));
                let always = ImageSpot { top: LU::MIN, bottom: LU::MAX, lazy };
                let Some(bid) = found.get(i).copied().flatten().or_else(|| bg_owner.get(&r.node).copied()) else { return always };
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

/// css-backgrounds-3 5.1, 5.5: the radii of a border box of `w` x `h` (percentages resolved against it), all scaled down by the
/// smallest ratio side / (sum of the two radii on it) when some pair would overlap. A corner with a zero radius is square.
fn resolve_radii(r: Option<&[(Lp, Lp); 4]>, w: f32, h: f32) -> [(f32, f32); 4] {
    let mut out = [(0.0f32, 0.0f32); 4];
    let Some(r) = r else { return out };
    for (o, (a, b)) in out.iter_mut().zip(r.iter()) {
        *o = (a.resolve(w).max(0.0), b.resolve(h).max(0.0));
    }
    let [tl, tr, br, bl] = out;
    let mut f = 1.0f32;
    for (side, sum) in [(w, tl.0 + tr.0), (w, bl.0 + br.0), (h, tl.1 + bl.1), (h, tr.1 + br.1)] {
        if sum > side && sum > 0.0 {
            f = f.min(side.max(0.0) / sum);
        }
    }
    out.map(|(a, b)| if a > 0.0 && b > 0.0 { (a * f, b * f) } else { (0.0, 0.0) })
}

/// The radii of an inner edge: each reduced by the widths `d` (top, right, bottom, left) next to the corner, at least 0 (5.3).
fn inset_radii(r: [(f32, f32); 4], d: [f32; 4]) -> [(f32, f32); 4] {
    let [t, rt, b, l] = d;
    let [tl, tr, br, bl] = r;
    let s = |v: f32, d: f32| (v - d).max(0.0);
    let fix = |(a, b): (f32, f32)| if a > 0.0 && b > 0.0 { (a, b) } else { (0.0, 0.0) };
    [fix((s(tl.0, l), s(tl.1, t))), fix((s(tr.0, rt), s(tr.1, t))), fix((s(br.0, rt), s(br.1, b))), fix((s(bl.0, l), s(bl.1, b)))]
}
