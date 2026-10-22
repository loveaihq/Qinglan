//! Scroll containers and sticky boxes (css-overflow-3 section 3, css-position-3 section 3.4).
//!
//! A list that has either keeps [`Scope`]s: the box, the rectangle it shows through and, for a scroll container, how far its content
//! reaches and where it is scrolled to. What is inside a scope is not in `DisplayList::items` but in a [`Pane`], one item of the
//! list that draws its own items shifted (by the scroll offset, or by the sticky shift) and clipped to the view, straight into the
//! frame, no layer. The scroll offsets are the only part of a list that changes after it is built (they are cells), so the list is
//! still built once per layout and drawn at any position; [`DisplayList::inherit`] carries them over to the next layout.

use crate::fmath::Fclamp;
use std::cell::Cell;

use super::raster::{Ctx, Frame, device_rect, fill, premul};
use super::{DisplayList, Item, Rect};
use crate::css::color::Color;
use crate::html::dom::NodeId;
use crate::layout::units::LayoutUnit as LU;
use crate::vector::IRect;

/// "No scope": the page itself.
pub const NO_SCOPE: u32 = u32::MAX;
/// The width of the strip along an edge of a scroll container where its bar is drawn and found by the pointer (CSS px).
pub const LANE: f32 = 12.0;
const THUMB: f32 = 6.0;
const THUMB_MIN: f32 = 24.0;
const MAX_CHAIN: usize = 64;

pub enum ScopeKind {
    /// A scroll container; for x and y: 0 the user cannot scroll it (`hidden`), 1 `auto`, 2 `scroll`.
    Scroll([u8; 2]),
    Sticky(StickyRule),
}

/// css-position-3 3.4: the containing block and the insets of a sticky box.
#[derive(Clone, Copy)]
pub struct StickyRule {
    /// The containing block (content box), page coordinates.
    pub cb: Rect,
    /// top, right, bottom, left in px; NaN for `auto`.
    pub inset: [f32; 4],
    /// Where the box can be drawn (for the index).
    pub reach: Rect,
    /// The containing block is the scroll container the box is in: it reaches as far as the container's content does.
    pub cb_scroll: bool,
}

pub struct Scope {
    pub node: NodeId,
    /// The scope this one is in (`NO_SCOPE`: the page).
    pub parent: u32,
    /// A scroll container: its padding box. A sticky box: its border box. In the coordinates of the parent scope's content.
    pub view: Rect,
    /// A scroll container: the right and bottom edge of its scrollable overflow area.
    pub ext: (f32, f32),
    pub kind: ScopeKind,
    /// How far a scroll container is scrolled (CSS px).
    pub off: Cell<(f32, f32)>,
    /// Inside a `position: fixed` box: `view` is in viewport coordinates, which do not move with the page, so a pointer is
    /// matched against it by its place in the viewport, not in the page.
    pub fixed: bool,
}

/// The items of one scope, drawn together.
pub struct Pane {
    pub scope: u32,
    pub items: Vec<Item>,
    /// The scroll bars go on top of this one.
    pub bars: bool,
}

/// The size of a thumb and its place on a lane of `lane` px, for `view` px of `content` scrolled `off` of at most `max`.
pub fn thumb(lane: f32, view: f32, content: f32, off: f32, max: f32) -> (f32, f32) {
    let len = (lane * view / content.max(1.0)).fclamp(THUMB_MIN.min(lane), lane.max(1.0));
    let start = if max > 0.0 { (lane - len) * (off / max).fclamp(0.0, 1.0) } else { 0.0 };
    (start, len)
}

fn contains(r: &Rect, x: f32, y: f32) -> bool {
    x >= r.x.to_f32() && x < r.right().to_f32() && y >= r.y.to_f32() && y < r.bottom().to_f32()
}

impl StickyRule {
    /// How far the box `n` is shifted when the scrollport is `port` (x, y, width, height, same coordinates as `n`).
    pub fn delta(&self, n: &Rect, port: (f32, f32, f32, f32), grow: (f32, f32)) -> (f32, f32) {
        let axis = |n0: f32, n1: f32, c0: f32, c1: f32, p0: f32, p1: f32, i0: f32, i1: f32| {
            let v0 = p0 + if i0.is_nan() { 0.0 } else { i0 };
            let mut v1 = p1 - if i1.is_nan() { 0.0 } else { i1 };
            // A view smaller than the box: the end inset gives way.
            v1 = v1.max(v0 + (n1 - n0));
            let mut d = 0.0;
            if !i0.is_nan() && n0 < v0 {
                d = v0 - n0;
            }
            if !i1.is_nan() && n1 + d > v1 {
                d = v1 - n1;
            }
            // Only as far as the box stays in its containing block.
            if d > 0.0 { d.min(c1 - n1).max(0.0) } else { d.max(c0 - n0).min(0.0) }
        };
        let (c, [t, r, b, l]) = (&self.cb, self.inset);
        let dx = axis(n.x.to_f32(), n.right().to_f32(), c.x.to_f32(), c.right().to_f32().max(grow.0), port.0, port.0 + port.2, l, r);
        let dy = axis(n.y.to_f32(), n.bottom().to_f32(), c.y.to_f32(), c.bottom().to_f32().max(grow.1), port.1, port.1 + port.3, t, b);
        (dx, dy)
    }
}

impl Scope {
    /// How far a scroll container can be scrolled in x and y.
    pub fn max(&self) -> (f32, f32) {
        ((self.ext.0 - self.view.right().to_f32()).max(0.0), (self.ext.1 - self.view.bottom().to_f32()).max(0.0))
    }

    /// The bar of axis `a` (0 x, 1 y) if there is one: the length of its lane, the start and the length of its thumb.
    pub fn bar(&self, a: usize) -> Option<(f32, f32, f32)> {
        let ScopeKind::Scroll(show) = &self.kind else { return None };
        let m = self.max();
        let has = |a: usize| show.get(a).is_some_and(|&s| s != 0) && (if a == 0 { m.0 } else { m.1 }) > 0.0;
        if !has(a) {
            return None;
        }
        let (view, max, off) = if a == 0 { (self.view.w.to_f32(), m.0, self.off.get().0) } else { (self.view.h.to_f32(), m.1, self.off.get().1) };
        let lane = view - if has(1 - a) { LANE } else { 0.0 };
        let (s, l) = thumb(lane, view, view + max, off, max);
        Some((lane, s, l))
    }

    fn scrolls(&self) -> bool {
        matches!(self.kind, ScopeKind::Scroll(_))
    }
}

/// What a press on a scroll bar did.
pub enum Press {
    /// It took hold of a thumb: `DisplayList::bar_drag` follows the pointer.
    Thumb { scope: u32, axis: u8, grab: f32 },
    /// It paged the container towards the pointer.
    Track,
}

impl DisplayList {
    /// How far the sticky scope `i` is shifted when the scrollport of what it is in is `port`.
    pub fn sticky_delta(&self, i: u32, port: (f32, f32, f32, f32)) -> (f32, f32) {
        let Some(s) = self.scopes.get(i as usize) else { return (0.0, 0.0) };
        let ScopeKind::Sticky(rule) = &s.kind else { return (0.0, 0.0) };
        let grow = if rule.cb_scroll { self.scopes.get(s.parent as usize).map_or((0.0, 0.0), |p| p.ext) } else { (0.0, 0.0) };
        rule.delta(&s.view, port, grow)
    }

    /// How far the scroll containers around scope `i` (not itself) are scrolled.
    pub fn shift_of(&self, i: u32) -> (f32, f32) {
        let (mut x, mut y) = (0.0, 0.0);
        let mut cur = i;
        for _ in 0..MAX_CHAIN {
            let Some(s) = self.scopes.get(cur as usize) else { break };
            cur = s.parent;
            if let Some(p) = self.scopes.get(cur as usize).filter(|p| p.scrolls()) {
                let o = p.off.get();
                (x, y) = (x + o.0, y + o.1);
            }
        }
        (x, y)
    }

    /// The innermost scroll container under the point (`pt` in page coordinates, `at` the same place in the viewport, which is what
    /// the containers of fixed boxes are matched with), or `NO_SCOPE`.
    pub fn scope_at(&self, pt: (f32, f32), at: (f32, f32)) -> u32 {
        let mut inside = vec![false; self.scopes.len()];
        let mut best = NO_SCOPE;
        for (i, s) in self.scopes.iter().enumerate() {
            let up = s.parent == NO_SCOPE || inside.get(s.parent as usize) == Some(&true);
            let ok = up && (!s.scrolls() || {
                let (sx, sy) = self.shift_of(i as u32);
                let p = if s.fixed { at } else { pt };
                contains(&s.view, p.0 + sx, p.1 + sy)
            });
            if let Some(slot) = inside.get_mut(i) {
                *slot = ok;
            }
            if ok && s.scrolls() {
                best = i as u32;
            }
        }
        best
    }

    /// Scroll the containers under the point by (`dx`, `dy`) CSS px, the innermost that still can first and then those around
    /// it; what none of them could take is returned (for the page).
    pub fn wheel(&self, pt: (f32, f32), at: (f32, f32), mut dx: f32, mut dy: f32) -> (f32, f32) {
        // A NaN or infinite delta would put NaN in an offset for good (and hand an infinite rest to the page): it is no scroll.
        if !dx.is_finite() {
            dx = 0.0;
        }
        if !dy.is_finite() {
            dy = 0.0;
        }
        let mut i = self.scope_at(pt, at);
        for _ in 0..MAX_CHAIN {
            let Some(s) = self.scopes.get(i as usize) else { break };
            if let ScopeKind::Scroll(show) = &s.kind {
                let ((ox, oy), (mx, my)) = (s.off.get(), s.max());
                let nx = if show.first() == Some(&0) { ox } else { (ox + dx).fclamp(0.0, mx) };
                let ny = if show.get(1) == Some(&0) { oy } else { (oy + dy).fclamp(0.0, my) };
                (dx, dy) = (dx - (nx - ox), dy - (ny - oy));
                s.off.set((nx, ny));
            }
            i = s.parent;
        }
        (dx, dy)
    }

    /// A press at the point (page coordinates, and `at` in the viewport as for [`DisplayList::scope_at`]) on the bar of a scroll container, if it is on one.
    pub fn bar_press(&self, pt: (f32, f32), at: (f32, f32)) -> Option<Press> {
        for (i, s) in self.scopes.iter().enumerate().rev() {
            if !s.scrolls() {
                continue;
            }
            let (sx, sy) = self.shift_of(i as u32);
            let (w, h) = (s.view.w.to_f32(), s.view.h.to_f32());
            let p = if s.fixed { at } else { pt };
            let (x, y) = (p.0 + sx - s.view.x.to_f32(), p.1 + sy - s.view.y.to_f32());
            if x < 0.0 || y < 0.0 || x >= w || y >= h {
                continue;
            }
            for a in 0..2usize {
                let Some((lane, ts, tl)) = s.bar(a) else { continue };
                let (along, across, extent) = if a == 1 { (y, x, w) } else { (x, y, h) };
                if across < extent - LANE || along >= lane {
                    continue;
                }
                if along >= ts && along < ts + tl {
                    return Some(Press::Thumb { scope: i as u32, axis: a as u8, grab: along - ts });
                }
                let step = (if along < ts { -0.875 } else { 0.875 }) * if a == 1 { h } else { w };
                let (o, m) = (s.off.get(), s.max());
                s.off.set(if a == 1 { (o.0, (o.1 + step).fclamp(0.0, m.1)) } else { ((o.0 + step).fclamp(0.0, m.0), o.1) });
                return Some(Press::Track);
            }
        }
        None
    }

    /// The thumb held by `grab` follows the pointer at `pt` (page coordinates; `at` in the viewport).
    pub fn bar_drag(&self, scope: u32, axis: u8, grab: f32, pt: (f32, f32), at: (f32, f32)) {
        let Some(s) = self.scopes.get(scope as usize) else { return };
        let Some((lane, _, tl)) = s.bar(usize::from(axis)) else { return };
        let (sx, sy) = self.shift_of(scope);
        let m = s.max();
        let p = if s.fixed { at } else { pt };
        let (along, max) = if axis == 1 { (p.1 + sy - s.view.y.to_f32(), m.1) } else { (p.0 + sx - s.view.x.to_f32(), m.0) };
        let v = ((along - grab) / (lane - tl).max(1.0) * max).fclamp(0.0, max);
        let o = s.off.get();
        s.off.set(if axis == 1 { (o.0, v) } else { (v, o.1) });
    }

    /// The point in the content coordinates of scope `sc`, given the scrollport of the page `port` (x, y, w, h): back through the
    /// sticky shifts and scroll offsets of the scopes from the outermost one. `None` when a scroll container does not show it.
    pub fn to_content(&self, sc: u32, mut pt: (f32, f32), mut port: (f32, f32, f32, f32)) -> Option<(f32, f32)> {
        let mut chain = [NO_SCOPE; MAX_CHAIN];
        let mut n = 0;
        let mut cur = sc;
        while let Some(s) = self.scopes.get(cur as usize) {
            *chain.get_mut(n)? = cur;
            n += 1;
            cur = s.parent;
            if n == MAX_CHAIN {
                break;
            }
        }
        for &i in chain.get(..n)?.iter().rev() {
            let s = self.scopes.get(i as usize)?;
            match &s.kind {
                ScopeKind::Scroll(_) => {
                    if !contains(&s.view, pt.0, pt.1) {
                        return None;
                    }
                    let o = s.off.get();
                    pt = (pt.0 + o.0, pt.1 + o.1);
                    port = (s.view.x.to_f32() + o.0, s.view.y.to_f32() + o.1, s.view.w.to_f32(), s.view.h.to_f32());
                }
                ScopeKind::Sticky(_) => {
                    let d = self.sticky_delta(i, port);
                    pt = (pt.0 - d.0, pt.1 - d.1);
                    port = (port.0 - d.0, port.1 - d.1, port.2, port.3);
                }
            }
        }
        Some(pt)
    }

    /// How far (CSS px) the rectangle `r` in the content of scope `s` is from what the scroll container around it shows now
    /// (0: inside it, or on the page).
    pub fn scope_gap(&self, s: u32, r: &Rect) -> f32 {
        let mut cur = s;
        for _ in 0..MAX_CHAIN {
            let Some(sc) = self.scopes.get(cur as usize) else { return 0.0 };
            if sc.scrolls() {
                let (o, v) = (sc.off.get(), &sc.view);
                let gap = |a0: f32, a1: f32, v0: f32, v1: f32| (v0 - a1).max(a0 - v1).max(0.0);
                let dx = gap(r.x.to_f32() - o.0, r.right().to_f32() - o.0, v.x.to_f32(), v.right().to_f32());
                let dy = gap(r.y.to_f32() - o.1, r.bottom().to_f32() - o.1, v.y.to_f32(), v.bottom().to_f32());
                return dx.max(dy);
            }
            cur = sc.parent;
        }
        0.0
    }

    /// How far the scroll containers that are scrolled are, by DOM node (to carry over to the list of the next layout).
    pub fn offsets(&self) -> Vec<(NodeId, (f32, f32))> {
        self.scopes.iter().filter(|s| s.scrolls() && s.off.get() != (0.0, 0.0)).map(|s| (s.node, s.off.get())).collect()
    }

    /// Scroll the containers as [`DisplayList::offsets`] of the layout before said (clamped to what they hold now).
    pub fn restore(&self, offs: &[(NodeId, (f32, f32))]) {
        for &(node, (x, y)) in offs {
            self.set_scroll(node, x, y);
        }
    }

    /// Scroll the container of `node` to (`x`, `y`) CSS px (clamped).
    pub fn set_scroll(&self, node: NodeId, x: f32, y: f32) {
        if let Some(s) = self.scopes.iter().find(|s| s.node == node && s.scrolls()) {
            let m = s.max();
            s.off.set((x.fclamp(0.0, m.0), y.fclamp(0.0, m.1)));
        }
    }

    /// The horizontal bar of the page, if it has one when it is scrolled `p` px along the x axis (a vertical page: along its
    /// axis): the length of the lane, the start and the length of the thumb, and the most it can scroll.
    pub fn page_bar(&self, p: f32) -> Option<(f32, f32, f32, f32)> {
        let (vw, vh) = self.vp;
        let max = if self.axis == super::ScrollAxis::Down { (self.x_max.to_f32() - vw).max(0.0) } else { self.max_progress(vw, vh) };
        if max <= 0.0 || self.hide_x {
            return None;
        }
        let (s, l) = thumb(vw, vw, vw + max, p.abs().min(max), max);
        Some((vw, if self.axis == super::ScrollAxis::Left { vw - s - l } else { s }, l, max))
    }
}

impl Ctx<'_> {
    /// Draw a pane: its items, shifted and clipped to the pane's rectangle `r`; then the bars of a scroll container.
    pub(super) fn pane(&mut self, frame: &mut Frame, p: &Pane, r: &IRect, clip: &IRect) {
        let list = self.list;
        let Some(s) = list.scopes.get(p.scope as usize) else { return };
        let area = r.intersect(clip);
        if area.is_empty() || self.depth >= 60 {
            return;
        }
        let (shift, port) = match &s.kind {
            ScopeKind::Scroll(_) => {
                let (o, v) = (s.off.get(), &s.view);
                (o, Some((v.x.to_f32() + o.0, v.y.to_f32() + o.1, v.w.to_f32(), v.h.to_f32())))
            }
            ScopeKind::Sticky(_) => {
                let d = self.port.map_or((0.0, 0.0), |pt| list.sticky_delta(p.scope, pt));
                ((-d.0, -d.1), self.port.map(|(x, y, w, h)| (x - d.0, y - d.1, w, h)))
            }
        };
        let k = self.dpr;
        let (dx, dy) = ((f64::from(shift.0) * k).round() as i32, (f64::from(shift.1) * k).round() as i32);
        let mut child = self.child(self.dpr, self.sx + dx, self.sy + dy, area);
        child.port = port;
        for it in &p.items {
            child.item(frame, it);
        }
        let st = child.stats;
        self.merge(st);
        if p.bars && s.scrolls() {
            for a in 0..2usize {
                if let Some((_, ts, tl)) = s.bar(a) {
                    let v = &s.view;
                    let pad = (LANE - THUMB) / 2.0;
                    let r = if a == 1 { (v.right().to_f32() - LANE + pad, v.y.to_f32() + ts, THUMB, tl) } else { (v.x.to_f32() + ts, v.bottom().to_f32() - LANE + pad, tl, THUMB) };
                    self.pill(frame, r, clip, a == 1);
                }
            }
        }
    }

    /// A thumb at `(x, y, w, h)` CSS px (in this context's coordinates) with the corners cut.
    pub(super) fn pill(&mut self, frame: &mut Frame, (x, y, w, h): (f32, f32, f32, f32), clip: &IRect, _vertical: bool) {
        let rect = Rect::new(LU::from_f32(x), LU::from_f32(y), LU::from_f32(w), LU::from_f32(h));
        let mut d = device_rect(&rect, self.dpr);
        (d.x0, d.x1, d.y0, d.y1) = (d.x0 - self.sx, d.x1 - self.sx, d.y0 - self.sy, d.y1 - self.sy);
        let px = premul(Color::rgb(0x8a, 0x8a, 0x8a));
        fill(frame, &IRect { x0: d.x0 + 1, x1: d.x1 - 1, ..d }.intersect(clip), px);
        fill(frame, &IRect { y0: d.y0 + 1, y1: d.y1 - 1, ..d }.intersect(clip), px);
        // Both fills cover the middle; an opaque colour does not mind.
    }

    /// The horizontal bar of the page, over the bottom edge of the viewport.
    pub(super) fn page_bar(&mut self, frame: &mut Frame, p: f32) {
        let Some((_, ts, tl, _)) = self.list.page_bar(p) else { return };
        let (_, vh) = self.list.vp;
        let clip = self.bounds;
        self.pill(frame, (ts, vh - LANE + (LANE - THUMB) / 2.0, tl, THUMB), &clip, false);
    }
}
