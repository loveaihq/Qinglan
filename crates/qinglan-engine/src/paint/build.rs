//! Display list builder: walks the layout in paint order (CSS2 appendix E.2). Every stacking context (the root, a
//! positioned box with a `z-index`, a `position: fixed` box) paints its own background, the contexts of negative
//! `z-index`, the in-flow blocks, the floats, the in-flow inline content, then the positioned boxes of `z-index` auto
//! or 0 in document order and the positive ones. A positioned box and everything in its flow is painted at its turn
//! in the context it belongs to, not inside the flow; `position: fixed` boxes (and what they contain) go to the fixed list.

use std::rc::Rc;

use super::{BgDraw, BorderDraw, DisplayList, Group, ImagePlace, ImageSpot, Item, ItemKind, NO_CLIP, NO_SCOPE, Pane, PaintGlyph, RRect, Rect, Round, Scope, ScopeKind, ScrollAxis, ShadowDraw, StickyRule, TextRun, gradient};
use std::cell::Cell;

mod controls;
use crate::css::fx::Shadow;
use crate::vector::Transform;
use crate::css::bg::{Backgrounds, BgBox, BgImage, BgRepeat, BgSize, Pos1, Rep};
use crate::css::color::Color;
use crate::css::computed::{ComputedStyle, slant_of};
use crate::css::types::{BorderCollapse, BorderLineStyle, Dim, Overflow, Position, Visibility};
use crate::css::values::Lp;
use crate::html::atoms::FxHashMap;
use crate::html::dom::{Document, NONE, NodeId};
use crate::layout::block::{border_px, pad_px};
use crate::layout::boxtree::{BgRef, BoxId, BoxKind, Control, NO_BOX, ReplacedKind, Role, role_of};
use crate::layout::units::LayoutUnit as LU;
use crate::layout::{BoxOut, FragKind, Fragment, LayoutResult, Positioned, has_effect, has_transform, makes_context};
use crate::platform::{FontQuery, Platform};

/// Ancestor walks (clip, fixed) never go further than this.
const MAX_WALK: usize = 400;
/// Pages cannot be taller than this for scrolling purposes (CSS px); hostile offsets are cut here.
const MAX_PAGE_HEIGHT_PX: i32 = 1 << 22;

const GREY_FILL: Color = Color::rgb(0xe0, 0xe0, 0xe0);
const GREY_LINE: Color = Color::rgb(0x76, 0x76, 0x76);
const GREY_TEXT: Color = Color::rgb(0x50, 0x50, 0x50);

/// Is the box painted at its own turn, among the positioned boxes of its stacking context (not in the flow)? Positioned boxes
/// are, and so are those with an `opacity` below 1 or a `transform` (css-color-4 12.1, css-transforms-1 2).
/// A scroll container: `overflow` is `scroll` or `auto` on some axis (css-overflow-3 3).
fn scroller(style: &ComputedStyle) -> bool {
    matches!(style.boxs.overflow_x, Overflow::Scroll | Overflow::Auto) || matches!(style.boxs.overflow_y, Overflow::Scroll | Overflow::Auto)
}

fn has_inset(style: &ComputedStyle) -> bool {
    style.boxs.inset.iter().any(|d| matches!(d, Dim::Lp(_)))
}

fn positioned(style: &ComputedStyle) -> bool {
    style.boxs.position != Position::Static || has_effect(style)
}

/// Is the box the containing block of absolutely positioned descendants?
fn is_cb(style: &ComputedStyle) -> bool {
    style.boxs.position != Position::Static || has_transform(style)
}

/// Boxes with a position of their own in `LayoutResult::boxes`.
fn has_box_out(kind: BoxKind) -> bool {
    matches!(kind, BoxKind::Block | BoxKind::InlineBlock | BoxKind::Replaced)
}

/// Build the display list of a layout. `doc` is read for the `checked` and `loading` attributes; the
/// platform shapes the labels of placeholder boxes.
pub fn build_display_list(doc: &Document, res: &LayoutResult, platform: &dyn Platform) -> DisplayList {
    let mut b = Builder { doc, res, platform, list: DisplayList::empty(), clip: NO_CLIP, fixed: false, canvas_from: res.canvas.map_or(NO_BOX, |c| c.from), floats: Vec::new(), outlines: Vec::new(), sinks: Vec::new(), inline_fx: res.tree.boxes.iter().any(|b| b.kind == BoxKind::Inline && b.style.misc.fx.as_deref().is_some_and(|x| x.opacity < 255)), frame: NO_SCOPE, frames: Vec::new(), own: FxHashMap::default(), own_sticky: FxHashMap::default(), pane_clip: Vec::new(), ext: Vec::new(), page_right: LU::ZERO };
    b.list.vp = (res.viewport.0.to_f32(), res.viewport.1.to_f32());
    b.list.hide_x = res.tree.viewport_overflow.0 == Overflow::Hidden;
    b.find_scopes();
    b.list.canvas = res.canvas.map(|c| c.color);
    b.list.partial = res.partial;
    b.list.axis = ScrollAxis::of(res.tree.get(res.tree.root).style.text.writing_mode);
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
    floats: Vec<(BoxId, LU, LU, u32, bool, u32)>,
    /// Outlines met on the way, waiting for the end of their stacking context: (outline, clip, fixed).
    outlines: Vec<(BorderDraw, u32, bool)>,
    /// Open groups: the items of each so far and the clip outside it.
    sinks: Vec<(Vec<Item>, u32)>,
    /// Some inline box has an `opacity` below 1 (the lines are checked for groups only then).
    inline_fx: bool,
    /// The innermost scope the items being added are in (`NO_SCOPE`: the page), and the ones it was opened inside.
    frame: u32,
    frames: Vec<u32>,
    /// The scope each box opened (a scroll container, a sticky box).
    own: FxHashMap<BoxId, u32>,
    own_sticky: FxHashMap<BoxId, u32>,
    /// Per scope: the clip outside its pane, and the right and bottom edge of what it holds.
    pane_clip: Vec<u32>,
    ext: Vec<(LU, LU)>,
    /// The right edge of what the page holds (its scrollable overflow).
    page_right: LU,
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
        if let Some((items, _)) = self.sinks.last_mut() {
            items.push(item);
        } else if self.fixed {
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
        let mut fixed = false;
        for _ in 0..MAX_WALK {
            if cur == NO_BOX {
                break;
            }
            let b = tree.get(cur);
            // A transformed ancestor of a fixed box is its containing block: it scrolls with the page (css-transforms-1 2).
            if fixed && has_transform(&b.style) {
                return false;
            }
            fixed |= b.style.boxs.position == Position::Fixed;
            cur = b.parent;
        }
        fixed
    }

    /// The padding box of a box whose border box is `r`.
    fn padding_box(&self, id: BoxId, r: Rect) -> Rect {
        let bd = &self.res.tree.get(id).pstyle().border;
        let b = |i: usize| border_px(bd.width.get(i).copied().unwrap_or(0.0));
        Rect::new(r.x + b(3), r.y + b(0), (r.w - b(1) - b(3)).max0(), (r.h - b(0) - b(2)).max0())
    }

    /// The clips a positioned box inherits from its ancestors, each with the box whose padding box it is. An overflow clip of an
    /// ancestor applies when the ancestor is, or contains, the box's containing block; for `absolute` that means the containing
    /// block (the nearest positioned ancestor) and everything above it. `fixed` boxes are not clipped.
    fn ancestor_walk(&self, id: BoxId, pos: Position, mut f: impl FnMut(BoxId, Rect)) {
        if pos == Position::Fixed {
            return;
        }
        let res = self.res;
        let tree = &res.tree;
        let mut counting = pos != Position::Absolute;
        let mut cur = tree.get(id).parent;
        for _ in 0..MAX_WALK {
            if cur == NO_BOX {
                break;
            }
            let b = tree.get(cur);
            if !counting && is_cb(&b.style) {
                counting = true;
            }
            if counting && has_box_out(b.kind) {
                let o = res.box_out(cur);
                if o.clip {
                    let (x, y) = res.page_pos(cur);
                    f(cur, self.clip_area(cur, Rect::new(x, y, o.w, o.h)));
                }
            }
            if b.style.boxs.position == Position::Fixed {
                break;
            }
            cur = b.parent;
        }
    }

    fn ancestor_clip(&self, id: BoxId, pos: Position) -> Option<Rect> {
        let mut acc: Option<Rect> = None;
        self.ancestor_walk(id, pos, |_, r| acc = Some(acc.map_or(r, |a| a.intersect(&r))));
        acc
    }

    /// [`Self::ancestor_clip`] by scope: the clip rectangles in the content coordinates of each scope they are in.
    fn ancestor_levels(&self, id: BoxId, pos: Position) -> Vec<(u32, Rect)> {
        let mut levels: Vec<(u32, Rect)> = Vec::new();
        self.ancestor_walk(id, pos, |a, r| {
            // A scroll container's padding box clips from outside it; any other box's from inside the scope it is in.
            let lvl = match self.own.get(&a).and_then(|&s| self.list.scopes.get(s as usize)).filter(|s| matches!(s.kind, ScopeKind::Scroll(_))) {
                Some(s) => s.parent,
                None => self.list.scope_of.get(a as usize).copied().unwrap_or(NO_SCOPE),
            };
            match levels.iter_mut().find(|l| l.0 == lvl) {
                Some(l) => l.1 = l.1.intersect(&r),
                None => levels.push((lvl, r)),
            }
        });
        levels
    }

    // ---- positioned boxes ----------------------------------------------------------------------------

    /// Does `id` make a stacking context (the root, or a positioned box with a `z-index`, or a fixed one)?
    fn is_context(&self, id: BoxId) -> bool {
        id == self.res.tree.root || makes_context(self.res.tree.get(id).pstyle())
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
        if self.list.scopes.is_empty() {
            if let Some(r) = self.ancestor_clip(id, p.position) {
                self.push_clip(r);
            }
            self.paint_box(id, x, y);
        } else {
            self.positioned_in_scopes(id, p.position, x, y);
        }
        (self.fixed, self.clip) = saved;
    }

    /// A positioned box on a page with scopes: it is painted inside the panes of the scopes it is in that the painting so far is
    /// not in, each clipped by the ancestors' clips of its level.
    #[inline(never)]
    fn positioned_in_scopes(&mut self, id: BoxId, pos: Position, x: LU, y: LU) {
        let levels = self.ancestor_levels(id, pos);
        // A fixed box is not in what scrolls; it goes to the fixed list, past the panes that are open.
        let detached = self.fixed.then(|| (std::mem::take(&mut self.sinks), std::mem::take(&mut self.frames), std::mem::replace(&mut self.frame, NO_SCOPE)));
        let target = if self.fixed {
            NO_SCOPE
        } else {
            match self.own_sticky.get(&id).or_else(|| self.own.get(&id)) {
                Some(&s) => self.list.scopes.get(s as usize).map_or(NO_SCOPE, |s| s.parent),
                None => self.list.scope_of.get(id as usize).copied().unwrap_or(NO_SCOPE),
            }
        };
        let path = self.path_to(target);
        let rect_of = |lvl: u32| levels.iter().find(|l| l.0 == lvl).map(|l| l.1);
        let mut lvl = self.frame;
        for &s in &path {
            self.clip = NO_CLIP;
            if let Some(r) = rect_of(lvl) {
                self.push_clip(r);
            }
            self.open_scope(s, true);
            lvl = s;
        }
        self.clip = NO_CLIP;
        if let Some(r) = rect_of(lvl) {
            self.push_clip(r);
        }
        self.paint_box(id, x, y);
        for _ in &path {
            self.close_scope(false);
        }
        if let Some((sinks, frames, frame)) = detached {
            (self.sinks, self.frames, self.frame) = (sinks, frames, frame);
        }
    }

    /// The scopes between the one being painted and `target` (inside it), outermost first; none when `target` is not inside it.
    fn path_to(&self, target: u32) -> Vec<u32> {
        let mut path = Vec::new();
        let mut s = target;
        while s != self.frame && s != NO_SCOPE && path.len() < 64 {
            path.push(s);
            s = self.list.scopes.get(s as usize).map_or(NO_SCOPE, |sc| sc.parent);
        }
        if s != self.frame {
            path.clear();
        }
        path.reverse();
        path
    }

    /// Begin the pane of scope `s`: what is added goes to it until [`Self::close_scope`]. `set`: the clip now is the one the pane
    /// item will carry (and is remembered for panes opened again later).
    fn open_scope(&mut self, s: u32, set: bool) {
        if set && let Some(c) = self.pane_clip.get_mut(s as usize) {
            *c = self.clip;
        }
        self.sinks.push((Vec::new(), self.clip));
        self.frames.push(self.frame);
        self.frame = s;
        self.clip = NO_CLIP;
    }

    /// End the pane that is open: an item of the scope's rectangle in what was open before it. `bars`: the bars go on top.
    fn close_scope(&mut self, bars: bool) {
        let Some((items, outer)) = self.sinks.pop() else { return };
        let s = self.frame;
        self.frame = self.frames.pop().unwrap_or(NO_SCOPE);
        self.clip = outer;
        let Some(sc) = self.list.scopes.get(s as usize) else { return };
        if items.is_empty() && !bars {
            return;
        }
        let bounds = match &sc.kind {
            ScopeKind::Sticky(r) => r.reach,
            ScopeKind::Scroll(_) => sc.view,
        };
        self.list.panes.push(Pane { scope: s, items, bars });
        let pi = (self.list.panes.len() - 1) as u32;
        self.push(bounds, ItemKind::Pane(pi));
    }

    /// Make the scopes of the page: a scroll container for each box with `overflow: scroll | auto`, a sticky one for each
    /// `position: sticky` box with an inset, unless its nearest scroll container is `overflow: hidden` (that never scrolls).
    #[inline(never)]
    fn find_scopes(&mut self) {
        let res = self.res;
        let tree = &res.tree;
        if !tree.boxes.iter().any(|b| scroller(&b.style) || (b.style.boxs.position == Position::Sticky && has_inset(&b.style))) {
            return;
        }
        // The `overflow: hidden` boxes that are the scrollport of a sticky box: they never scroll but the box sticks to them.
        let mut hosts: FxHashMap<BoxId, ()> = FxHashMap::default();
        for (i, b) in tree.boxes.iter().enumerate() {
            if b.style.boxs.position == Position::Sticky && has_inset(&b.style) {
                let mut c = b.parent;
                for _ in 0..MAX_WALK {
                    let Some(a) = (c != NO_BOX).then(|| tree.get(c)) else { break };
                    if a.style.boxs.overflow_x == Overflow::Hidden || a.style.boxs.overflow_y == Overflow::Hidden || scroller(&a.style) {
                        if !scroller(&a.style) {
                            hosts.insert(c, ());
                        }
                        break;
                    }
                    c = a.parent;
                }
            }
            let _ = i;
        }
        // Per box: the scope its content is in, the one its absolutely positioned descendants are in, done.
        let mut st = vec![(NO_SCOPE, NO_SCOPE, false, false); tree.boxes.len()];
        let mut chain: Vec<BoxId> = Vec::new();
        for start in 0..tree.boxes.len() as BoxId {
            if st.get(start as usize).is_none_or(|s| s.3) {
                continue;
            }
            chain.clear();
            let mut cur = start;
            while cur != NO_BOX && chain.len() < 100_000 && st.get(cur as usize).is_some_and(|s| !s.3) {
                chain.push(cur);
                cur = tree.get(cur).parent;
            }
            let mut up = st.get(cur as usize).copied().unwrap_or((NO_SCOPE, NO_SCOPE, false, true));
            for &i in chain.iter().rev() {
                let b = tree.get(i);
                let bs = &b.style.boxs;
                let lives = match bs.position {
                    Position::Fixed => NO_SCOPE,
                    Position::Absolute => up.1,
                    _ => up.0,
                };
                let mut me = lives;
                if matches!(b.kind, BoxKind::Block | BoxKind::InlineBlock) {
                    let sticky = bs.position == Position::Sticky && has_inset(&b.style);
                    let host = scroller(&b.style) || hosts.contains_key(&i);
                    // A box that is both (a sticky sidebar that scrolls) has the sticky scope outside the scroll one.
                    if sticky {
                        me = self.new_scope(i, lives, true);
                        self.own_sticky.insert(i, me);
                    }
                    if host {
                        me = self.new_scope(i, me, false);
                        self.own.insert(i, me);
                    }
                }
                up = (me, if is_cb(&b.style) { me } else { up.1 }, false, true);
                if let Some(slot) = st.get_mut(i as usize) {
                    *slot = up;
                }
            }
        }
        self.list.scope_of = st.iter().map(|s| s.0).collect();
    }

    fn new_scope(&mut self, i: BoxId, parent: u32, sticky: bool) -> u32 {
        let res = self.res;
        let b = res.tree.get(i);
        let bs = &b.style.boxs;
        let o = res.box_out(i);
        let (x, y) = res.page_pos(i);
        let r = Rect::new(x, y, o.w, o.h);
        let (kind, view) = if !sticky {
            let m = |o: Overflow| match o {
                Overflow::Scroll => 2,
                Overflow::Auto => 1,
                _ => 0,
            };
            (ScopeKind::Scroll([m(bs.overflow_x), m(bs.overflow_y)]), self.padding_box(i, r))
        } else {
            // css-position-3 3.4: the containing block is the content box of the parent; percentages are of the scrollport.
            let p = res.tree.layout_parent(i);
            let po = res.box_out(p);
            let (px, py) = res.page_pos(p);
            let pb = self.padding_box(p, Rect::new(px, py, po.w, po.h));
            let pad = |k: usize| res.tree.get(p).pstyle().boxs.padding.get(k).map_or(LU::ZERO, |l| pad_px(l, po.w));
            let cb = Rect::new(pb.x + pad(3), pb.y + pad(0), (pb.w - pad(3) - pad(1)).max0(), (pb.h - pad(0) - pad(2)).max0());
            let (pw, ph) = self.list.scopes.get(parent as usize).map_or(self.list.vp, |s| (s.view.w.to_f32(), s.view.h.to_f32()));
            let base = [ph, pw, ph, pw];
            let inset: [f32; 4] = std::array::from_fn(|k| match bs.inset.get(k) {
                Some(Dim::Lp(l)) => l.resolve(base.get(k).copied().unwrap_or(0.0)),
                _ => f32::NAN,
            });
            let (hx, hy) = (!inset[1].is_nan() || !inset[3].is_nan(), !inset[0].is_nan() || !inset[2].is_nan());
            // In a scroll container, the containing block reaches as far as the container's content: not known yet, so any distance.
            let cb_scroll = self.own.get(&p).is_some_and(|&s| self.list.scopes.get(s as usize).is_some_and(|s| matches!(s.kind, ScopeKind::Scroll(_))));
            let far = LU::px(MAX_PAGE_HEIGHT_PX);
            let (x0, x1) = if hx { (r.x.min(cb.x), if cb_scroll { far } else { r.right().max(cb.right()) }) } else { (r.x, r.right()) };
            let (y0, y1) = if hy { (r.y.min(cb.y), if cb_scroll { far } else { r.bottom().max(cb.bottom()) }) } else { (r.y, r.bottom()) };
            (ScopeKind::Sticky(StickyRule { cb, inset, reach: Rect::new(x0, y0, (x1 - x0).max0(), (y1 - y0).max0()), cb_scroll }), r)
        };
        self.ext.push((view.x, view.y));
        self.pane_clip.push(NO_CLIP);
        self.list.scopes.push(Scope { node: b.node, parent, view, ext: (view.right().to_f32(), view.bottom().to_f32()), kind, off: Cell::new((0.0, 0.0)) });
        (self.list.scopes.len() - 1) as u32
    }

    /// Add `r` (a box or a line piece the painting has reached, in the coordinates of the current scope) to the scrollable overflow
    /// of the scroll container or the page it is in, as far as the clips let it be seen.
    fn extend(&mut self, r: Rect) {
        let mut s = self.frame;
        let mut scroll = NO_SCOPE;
        for _ in 0..64 {
            let Some(sc) = self.list.scopes.get(s as usize) else { break };
            if matches!(sc.kind, ScopeKind::Scroll(_)) {
                scroll = s;
                break;
            }
            s = sc.parent;
        }
        if scroll == NO_SCOPE && self.fixed {
            return;
        }
        let r = self.list.clip_rect(self.clip).map_or(r, |c| r.intersect(&c));
        if r.is_empty() {
            return;
        }
        match self.ext.get_mut(scroll as usize) {
            Some(e) => *e = (e.0.max(r.right()), e.1.max(r.bottom())),
            None => self.page_right = self.page_right.max(r.right()),
        }
    }

    /// The extent along the scroll axis of something at `r` in scope `s`, as the page sees it: that of the outermost scope around it
    /// (a sticky one moves with the viewport: its pictures are always near).
    fn span_of(&self, mut s: u32, r: &Rect) -> (LU, LU) {
        let mut v = *r;
        let mut sticky = false;
        for _ in 0..64 {
            let Some(sc) = self.list.scopes.get(s as usize) else { break };
            sticky = matches!(sc.kind, ScopeKind::Sticky(_));
            (v, s) = (sc.view, sc.parent);
        }
        if sticky { (LU::MIN, LU::MAX) } else { self.along(&v) }
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
        let sticky = if self.own_sticky.is_empty() { None } else { self.own_sticky.get(&id).copied() };
        if let Some(s) = sticky {
            self.open_scope(s, true);
        }
        let grouped = self.open_group(id);
        self.paint_box_inner(id, ax, ay);
        if grouped {
            self.close_group(id, ax, ay);
        }
        if sticky.is_some() {
            self.close_scope(false);
        }
    }

    /// A box with an `opacity` below 1 or a `transform` is painted into a group of its own (see [`Group`]); the items it
    /// pushes go to the group and carry a clip chain of their own.
    fn open_group(&mut self, id: BoxId) -> bool {
        if id == self.res.tree.root || !has_effect(self.res.tree.get(id).pstyle()) {
            return false;
        }
        self.sinks.push((Vec::new(), self.clip));
        self.clip = NO_CLIP;
        true
    }

    fn close_group(&mut self, id: BoxId, ax: LU, ay: LU) {
        let Some(fx) = self.res.tree.get(id).pstyle().misc.fx.clone() else { return };
        let o = self.res.box_out(id);
        let (x, y) = (ax.to_f32(), ay.to_f32());
        let xf = Transform::translate(x, y).compose(&fx.matrix(o.w.to_f32(), o.h.to_f32())).compose(&Transform::translate(-x, -y));
        self.finish_group(fx.opacity, xf);
    }

    /// End the innermost open group: its items are drawn with `opacity` about `xf`.
    #[inline(never)]
    fn finish_group(&mut self, opacity: u8, xf: Transform) {
        let Some((items, outer)) = self.sinks.pop() else { return };
        self.clip = outer;
        let Some(local) = items.iter().map(|i| i.bounds).reduce(|a, b| {
            let (x, y) = (a.x.min(b.x), a.y.min(b.y));
            Rect::new(x, y, a.right().max(b.right()) - x, a.bottom().max(b.bottom()) - y)
        }) else {
            return;
        };
        let det = xf.a * xf.d - xf.b * xf.c;
        // `opacity: 0` paints nothing; a transform that cannot be inverted draws nothing (css-transforms-1 "singular").
        if opacity == 0 || !(det.abs() > 1e-9 && det.is_finite() && [xf.e, xf.f].iter().all(|v| v.is_finite())) {
            return;
        }
        let corners = [(local.x, local.y), (local.right(), local.y), (local.x, local.bottom()), (local.right(), local.bottom())].map(|(cx, cy)| xf.apply(crate::vector::Pt::new(cx.to_f32(), cy.to_f32())));
        let lim = (MAX_PAGE_HEIGHT_PX / 2) as f32;
        let fold = |f: fn(f32, f32) -> f32, g: fn(&crate::vector::Pt) -> f32, init: f32| corners.iter().map(g).fold(init, f).clamp(-lim, lim);
        let (x0, x1) = (fold(f32::min, |p| p.x, f32::MAX), fold(f32::max, |p| p.x, f32::MIN));
        let (y0, y1) = (fold(f32::min, |p| p.y, f32::MAX), fold(f32::max, |p| p.y, f32::MIN));
        let bounds = Rect::new(LU::from_f32(x0), LU::from_f32(y0), LU::from_f32(x1 - x0), LU::from_f32(y1 - y0));
        self.list.groups.push(Group { items, opacity, xf, local });
        let gi = (self.list.groups.len() - 1) as u32;
        self.extend(bounds);
        self.push(bounds, ItemKind::Group(gi));
    }

    fn paint_box_inner(&mut self, id: BoxId, ax: LU, ay: LU) {
        let first = self.floats.len();
        let first_outline = self.outlines.len();
        self.walk(id, ax, ay, false);
        let mut i = first;
        while let Some(&(c, x, y, clip, fixed, fr)) = self.floats.get(i) {
            i += 1;
            let saved = (self.clip, self.fixed);
            let path = self.path_to(fr);
            for &s in &path {
                self.clip = self.pane_clip.get(s as usize).copied().unwrap_or(NO_CLIP);
                self.open_scope(s, false);
            }
            (self.clip, self.fixed) = (clip, fixed);
            self.paint_box(c, x, y);
            for _ in &path {
                self.close_scope(false);
            }
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
            self.extend(rect);
            if !o.hide {
                self.box_decor(id, rect);
                if bx.control() == Control::Select && bx.style.misc.appearance == crate::css::types::Appearance::Auto && Self::visible_style(&bx.style) {
                    self.select_arrow(rect);
                }
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
        let pane = if self.own.is_empty() { None } else { self.own.get(&id).copied().filter(|&s| self.list.scopes.get(s as usize).is_some_and(|s| matches!(s.kind, ScopeKind::Scroll(_)))) };
        if let Some(s) = pane {
            self.open_scope(s, true);
        }
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
                    if has_effect(res.tree.get(c).pstyle()) {
                        continue;
                    }
                    let co = res.box_out(c);
                    self.floats.push((c, ax + co.x, ay + co.y, self.clip, self.fixed, self.frame));
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
        if pane.is_some() {
            self.close_scope(inline);
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
        let shadows = m.fx.as_deref().filter(|f| !f.shadows.is_empty()).map(|f| f.shadows.as_slice());
        if let Some(sh) = shadows {
            self.box_shadows(sh, style, r, radii, sides, false);
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
        if let Some(sh) = shadows {
            self.box_shadows(sh, style, r, radii, sides, true);
        }
        if borders {
            self.border_items(style, r, sides, radii);
        }
        self.outline(style, r, radii);
    }

    /// The outer (below the background) or inset (above it) shadows of a box, the first one on top (css-backgrounds-3 6.1.3).
    #[inline(never)]
    fn box_shadows(&mut self, shadows: &[Shadow], style: &ComputedStyle, r: Rect, radii: [(f32, f32); 4], sides: [bool; 4], inset: bool) {
        let bd = &style.border;
        let bw: [LU; 4] = std::array::from_fn(|i| if sides.get(i).copied().unwrap_or(false) { border_px(bd.width.get(i).copied().unwrap_or(0.0)) } else { LU::ZERO });
        let pb = Rect::new(r.x + bw[3], r.y + bw[0], (r.w - bw[1] - bw[3]).max0(), (r.h - bw[0] - bw[2]).max0());
        for s in shadows.iter().rev().filter(|s| s.inset == inset) {
            let [dx, dy, blur, spread] = s.px();
            let color = s.color.resolve(style.text.color);
            if color.a == 0 || ![dx, dy, blur, spread].iter().all(|v| v.is_finite()) {
                continue;
            }
            let (dx, dy, blur, spread) = (dx.clamp(-1.0e6, 1.0e6), dy.clamp(-1.0e6, 1.0e6), blur.min(1.0e5), spread.clamp(-1.0e6, 1.0e6));
            let (rect, radii, bounds) = if inset {
                (pb, inset_radii(radii, bw.map(LU::to_f32)), pb)
            } else {
                let g = LU::from_f32(spread.max(0.0) + blur * 1.5 + 1.0);
                (r, radii, Rect::new(r.x + LU::from_f32(dx) - g, r.y + LU::from_f32(dy) - g, r.w + g + g, r.h + g + g))
            };
            // (An empty box still casts a shadow, but has no padding box to cast it into.)
            if inset && rect.is_empty() {
                continue;
            }
            let i = self.list.shadows.len() as u32;
            self.list.shadows.push(ShadowDraw { rect, radii, color, dx, dy, blur, spread, inset });
            self.push(bounds, ItemKind::Shadow(i));
        }
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
                let span = self.span_of(self.frame, &clip_rect);
                self.list.images.push(ImagePlace { node, rect: tile, span, area: clip_rect, fixed: self.fixed, scope: self.frame });
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

    /// What box `id` clips to: its padding box, on the axes where `overflow` is not `visible` (`clip` on one axis only leaves the
    /// other unclipped, css-overflow-3 3).
    fn clip_area(&self, id: BoxId, rect: Rect) -> Rect {
        let mut pb = self.padding_box(id, rect);
        let bs = &self.res.tree.get(id).pstyle().boxs;
        let far = LU::px(MAX_PAGE_HEIGHT_PX);
        if bs.overflow_x == Overflow::Visible {
            (pb.x, pb.w) = (-far, far + far);
        }
        if bs.overflow_y == Overflow::Visible {
            (pb.y, pb.h) = (-far, far + far);
        }
        pb
    }

    /// Clip to the padding box of box `id`, rounded by the inner border edge where the box has a radius (css-backgrounds-3 5.3).
    fn clip_box(&mut self, id: BoxId, rect: Rect) -> u32 {
        let pb = self.clip_area(id, rect);
        let style = &self.res.tree.get(id).pstyle();
        let radii = resolve_radii(style.misc.radius.as_deref(), rect.w.to_f32(), rect.h.to_f32());
        let bd = &style.border;
        let bwf: [f32; 4] = std::array::from_fn(|i| border_px(bd.width.get(i).copied().unwrap_or(0.0)).to_f32());
        self.push_round(pb, inset_radii(radii, bwf))
    }

    fn lines(&mut self, o: &BoxOut, ax: LU, ay: LU) {
        let res = self.res;
        // The inline boxes with an `opacity` below 1 that the fragments being painted are in, outermost first: each is a group
        // around its consecutive fragments (a line at a time), css-color-4 12.1.
        let mut open: Vec<BoxId> = Vec::new();
        for li in o.lines.0..o.lines.1 {
            let Some(l) = res.lines.get(li as usize) else { continue };
            for fi in l.frags.0..l.frags.1 {
                if let Some(f) = res.fragments.get(fi as usize) {
                    if !self.inline_fx {
                        self.fragment(f, ax, ay);
                        continue;
                    }
                    let mut want: Vec<BoxId> = Vec::new();
                    let mut cur = f.box_id;
                    for _ in 0..MAX_WALK {
                        if cur == NO_BOX {
                            break;
                        }
                        let b = res.tree.get(cur);
                        if !matches!(b.kind, BoxKind::Inline | BoxKind::Text | BoxKind::LineBreak) {
                            break;
                        }
                        if b.kind == BoxKind::Inline && b.style.misc.fx.as_deref().is_some_and(|x| x.opacity < 255) {
                            want.push(cur);
                        }
                        cur = b.parent;
                    }
                    want.reverse();
                    self.sync_inline_groups(&mut open, &want);
                    self.fragment(f, ax, ay);
                }
            }
        }
        self.sync_inline_groups(&mut open, &[]);
    }

    /// Close the open inline groups that are not in `want` (and those inside them), then open the missing ones.
    #[inline(never)]
    fn sync_inline_groups(&mut self, open: &mut Vec<BoxId>, want: &[BoxId]) {
        let keep = open.iter().zip(want).take_while(|(a, b)| a == b).count();
        while open.len() > keep {
            if let Some(b) = open.pop() {
                let op = self.res.tree.get(b).pstyle().misc.fx.as_deref().map_or(255, |x| x.opacity);
                self.finish_group(op, Transform::IDENTITY);
            }
        }
        for &b in want.iter().skip(keep) {
            self.sinks.push((Vec::new(), self.clip));
            self.clip = NO_CLIP;
            open.push(b);
        }
    }

    fn fragment(&mut self, f: &Fragment, ax: LU, ay: LU) {
        let res = self.res;
        match f.kind {
            FragKind::Text { glyphs, decor, asc, v, fs, .. } => {
                self.extend(Rect::new(ax + f.x, ay + f.y, f.w, f.h));
                self.text(f, ax, ay, glyphs, asc, v, fs);
                let style = res.tree.get(f.box_id).pstyle().clone();
                if Self::visible_style(&style) {
                    for di in decor.0..decor.1 {
                        if let Some(d) = res.decorations.get(di as usize) {
                            self.rect(Rect::new(ax + d.x, ay + d.y, d.w, d.thickness), d.color);
                        }
                    }
                }
            }
            FragKind::Marker { glyphs, asc, .. } => self.text(f, ax, ay, glyphs, asc, 0, 0.0),
            FragKind::InlineBox { first, last } => {
                let style = res.tree.get(f.box_id).pstyle().clone();
                if Self::visible_style(&style) {
                    let r = Rect::new(ax + f.x, ay + f.y, f.w, f.h);
                    self.decor(f.box_id, &style, r, [true, last, true, first], true);
                }
            }
            FragKind::Atomic => {
                if !positioned(res.tree.get(f.box_id).pstyle()) {
                    self.paint_box(f.box_id, ax + f.x, ay + f.y);
                }
            }
            FragKind::Br => {}
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn text(&mut self, f: &Fragment, ax: LU, ay: LU, glyphs: (u32, u32), asc: LU, v: u8, fs: f32) {
        let res = self.res;
        let style = res.tree.get(f.box_id).pstyle();
        let color = style.text.color;
        if !Self::visible_style(style) || (color.a == 0 && style.text.shadow.is_none()) {
            return;
        }
        let Some(gs) = res.glyphs.get(glyphs.0 as usize..glyphs.1 as usize) else { return };
        if gs.is_empty() {
            return;
        }
        let size = if fs > 0.0 { fs } else { style.font.size };
        let half = LU::from_f32(size * 0.5);
        let (x, top) = (ax + f.x, ay + f.y);
        let bounds = Rect::new(x - half, top - half, f.w + half + half, f.h + half + half);
        let paint = gs.iter().map(|g| PaintGlyph { font: g.font, glyph: g.glyph, advance: g.advance, dx: g.offset_x, dy: g.offset_y });
        let shadows = style.text.shadow.clone();
        match v {
            0 => self.push_text(bounds, x, top + asc, size, color, paint, shadows.as_deref().map(Vec::as_slice), false),
            // Sideways (css-writing-modes-3 5.1): the pen runs down the line, `asc` from the left edge is the baseline; no shadows.
            1 => self.push_text(bounds, x + asc, top, size, color, paint, None, true),
            // Upright: every glyph in a cell of its own, centred across the line, its baseline `asc` below the middle of the cell
            // (the pen does not move; the glyphs are placed by their offsets).
            2 => {
                let xc = f.w.to_f32() / 2.0;
                let mid = asc.to_f32();
                let mut cum = 0.0f32;
                let placed: Vec<PaintGlyph> = gs
                    .iter()
                    .map(|g| {
                        let p = PaintGlyph { font: g.font, glyph: g.glyph, advance: 0.0, dx: xc - g.offset_x * 0.5, dy: cum + g.advance * 0.5 + mid };
                        cum += g.advance;
                        p
                    })
                    .collect();
                self.push_text(bounds, x, top, size, color, placed.into_iter(), None, false);
            }
            // `text-combine-upright`: one horizontal cell, centred in the cell.
            _ => {
                let w: f32 = gs.iter().map(|g| g.advance).sum();
                let x0 = x + LU::from_f32((f.w.to_f32() - w) * 0.5);
                self.push_text(bounds, x0, top + LU::from_f32(f.h.to_f32() * 0.5) + asc, size, color, paint, None, false);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn push_text(&mut self, bounds: Rect, x: LU, baseline: LU, size: f32, color: Color, glyphs: impl Iterator<Item = PaintGlyph>, shadows: Option<&[Shadow]>, rot: bool) {
        // A shadow can reach into view from outside the bounds of the text.
        let reach = LU::from_f32(shadows.map_or(0.0, |sh| sh.iter().map(Shadow::px).map(|[dx, dy, b, _]| dx.abs().max(dy.abs()) + b * 1.5 + 1.0).fold(0.0, f32::max)).min(1.0e5));
        let wide = Rect::new(bounds.x - reach, bounds.y - reach, bounds.w + reach + reach, bounds.h + reach + reach);
        if bounds.is_empty() || !self.visible(&wide) {
            return;
        }
        let start = self.list.glyphs.len() as u32;
        self.list.glyphs.extend(glyphs);
        let glyphs = (start, self.list.glyphs.len() as u32);
        // `text-shadow` (css-text-decor-3 4.1): the last shadow at the back, all below the text.
        for s in shadows.unwrap_or(&[]).iter().rev() {
            let [dx, dy, blur, _] = s.px();
            let col = s.color.resolve(color);
            if col.a == 0 || ![dx, dy, blur].iter().all(|v| v.is_finite()) {
                continue;
            }
            let (dx, dy, blur) = (dx.clamp(-1.0e5, 1.0e5), dy.clamp(-1.0e5, 1.0e5), blur.min(1.0e4));
            let g = LU::from_f32(blur * 1.5 + 1.0);
            let b = Rect::new(bounds.x + LU::from_f32(dx) - g, bounds.y + LU::from_f32(dy) - g, bounds.w + g + g, bounds.h + g + g);
            self.push(b, ItemKind::Text(TextRun { x: x + LU::from_f32(dx), baseline: baseline + LU::from_f32(dy), size, color: col, glyphs, blur, rot }));
        }
        if color.a != 0 {
            self.push(bounds, ItemKind::Text(TextRun { x, baseline, size, color, glyphs, blur: 0.0, rot }));
        }
    }

    // ---- replaced boxes -------------------------------------------------------------------------------

    fn replaced(&mut self, id: BoxId, r: Rect) {
        let res = self.res;
        let bx = res.tree.get(id);
        let style = bx.pstyle().clone();
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
                    let span = self.span_of(self.frame, &content);
                    self.list.images.push(ImagePlace { node: bx.node, rect: content, span, area: content, fixed: self.fixed, scope: self.frame });
                    if let Some(p) = prev {
                        self.restore(p);
                    }
                }
            }
            Some(ReplacedKind::Svg) => {
                if let Some(sc) = res.tree.svgs.get(&bx.node)
                    && !content.is_empty()
                    && self.visible(&content)
                {
                    self.list.svgs.push(sc.clone());
                    let i = (self.list.svgs.len() - 1) as u32;
                    self.push(content, ItemKind::Svg(i));
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
            Some(ReplacedKind::Check) => self.check(bx.node, content, &style),
            // A control that has no look of its own (): a grey frame.
            Some(ReplacedKind::Fixed(_, _, kind)) if !self.gauge(bx.node, content, kind, &style) => {
                self.rect(content, GREY_LINE);
                let one = LU::px(1);
                self.rect(Rect::new(content.x + one, content.y + one, (content.w - one - one).max0(), (content.h - one - one).max0()), GREY_FILL);
            }
            Some(ReplacedKind::Fixed(..)) => {}
            None => {}
        }
    }

    /// Write `text` in the font of box `id` near the top left corner of `r`, clipped to `r`.
    fn label(&mut self, id: BoxId, r: Rect, text: &str) {
        let style = self.res.tree.get(id).pstyle().clone();
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
        self.push_text(bounds, x, base, f.size, GREY_TEXT, glyphs, None, false);
        self.restore(prev);
    }

    // ---- the end ---------------------------------------------------------------------------------------

    fn finish(mut self) -> DisplayList {
        let res = self.res;
        let cap = LU::px(MAX_PAGE_HEIGHT_PX);
        let root = res.box_out(res.tree.root);
        let mut height = (root.y + root.h).min(cap);
        // A vertical page also measures how far it reaches to the sides (the scroll range is horizontal).
        let vertical = self.list.axis != ScrollAxis::Down;
        let (mut x_min, mut x_max) = if vertical { (root.x.max(-cap), (root.x + root.w).min(cap)) } else { (LU::ZERO, LU::ZERO) };
        for it in &self.list.items {
            let clip = self.list.clip_rect(it.clip);
            let bottom = match clip {
                Some(c) => it.bounds.bottom().min(c.bottom()),
                None => it.bounds.bottom(),
            };
            height = height.max(bottom.min(cap));
            if vertical {
                let (l, r) = match clip {
                    Some(c) => (it.bounds.x.max(c.x), it.bounds.right().min(c.right())),
                    None => (it.bounds.x, it.bounds.right()),
                };
                if l < r {
                    x_min = x_min.min(l.max(-cap));
                    x_max = x_max.max(r.min(cap));
                }
            }
        }
        self.list.height = height;
        if !vertical {
            x_max = self.page_right.min(cap);
        }
        (self.list.x_min, self.list.x_max) = (x_min, x_max);
        self.finish_scopes();
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

    /// The scrollable overflow of each scroll container: its padding box and what its content reached, with the end padding when
    /// the content goes past the content box (css-overflow-3 2.2, as browsers do).
    fn finish_scopes(&mut self) {
        for (&b, &s) in &self.own {
            let (Some(sc), Some(e)) = (self.list.scopes.get_mut(s as usize), self.ext.get(s as usize)) else { continue };
            if matches!(sc.kind, ScopeKind::Scroll(_)) {
                let (pad, w) = (&self.res.tree.get(b).pstyle().boxs.padding, self.res.box_out(b).w);
                let p = |k: usize| pad.get(k).map_or(0.0, |l| pad_px(l, w).to_f32());
                let (vr, vb) = (sc.view.right().to_f32(), sc.view.bottom().to_f32());
                let (cr, cb) = (e.0.to_f32(), e.1.to_f32());
                sc.ext = (if cr > vr - p(1) { cr + p(1) } else { vr }.max(vr), if cb > vb - p(2) { cb + p(2) } else { vb }.max(vb));
            }
        }
        self.list.panes.shrink_to_fit();
    }

    /// The extent of `r` along the scroll axis (fixed items are never measured: they are always near).
    fn along(&self, r: &Rect) -> (LU, LU) {
        self.list.axis.span(self.res.viewport.0, r)
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
                let (x, y) = res.page_pos(owner);
                let sc = self.list.scope_of.get(owner as usize).copied().unwrap_or(NO_SCOPE);
                let (top, bottom) = self.span_of(sc, &Rect::new(x, y, o.w, o.h));
                ImageSpot { top, bottom, lazy }
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
