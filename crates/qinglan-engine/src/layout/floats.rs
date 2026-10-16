//! Floats (step 2a-1): CSS2 9.5 (the nine placement rules of 9.5.1), 9.5.2 (`clear`, clearance), 9.4.1 and 10.6.7
//! (a block formatting context contains its floats), and the line boxes that float intrusion shortens (9.5, 9.4.2).
//!
//! Block layout (`block.rs`) is bottom-up: a block is laid out knowing its containing block's width and not its own
//! position, which its parent decides afterwards (margin collapsing). Floats need the position, so a block is laid out
//! at a *guessed* position in its block formatting context ([`Layouter::pos`]): the one its parent expects from the
//! margins it can see ahead of time. When the real position differs and the layout looked at floats on the way
//! (`touched`), the layout of that block is undone ([`Mark`]) and made again at the real one. Pages without floats
//! never touch any of this.
//!
//! Coordinates here are those of the block formatting context: x and y from the border-box corner of its root.

use super::boxtree::{BoxId, BoxKind};
use super::units::{LayoutUnit as LU, Margin};
use super::{Layouter, block};
use crate::css::types::{Clear, Dim, Display, Float};

/// A float's margin box in the coordinates of its block formatting context.
#[derive(Clone, Copy, Debug)]
pub(crate) struct FloatBox {
    pub right: bool,
    pub l: LU,
    pub t: LU,
    pub r: LU,
    pub b: LU,
}

/// A float laid out, not placed yet: its margin box and the left and top margin.
#[derive(Clone, Copy, Debug)]
pub(crate) struct FloatDims {
    pub w: LU,
    pub h: LU,
    pub ml: LU,
    pub mt: LU,
}

/// The floats of one block formatting context, in the order of the document.
#[derive(Default)]
pub(crate) struct FloatSpace {
    pub list: Vec<FloatBox>,
    /// Top of the last float placed: no float goes higher (9.5.1 rule 5).
    pub last_top: LU,
}

/// A float without a proper size or position is limited to this many per block formatting context (a hostile page can
/// make thousands; every placement looks at the earlier ones).
pub(crate) const MAX_FLOATS: usize = 4000;

impl FloatSpace {
    /// The room left for content in `[cl, cr]` at the heights `y .. y + h`: the floats that overlap those heights
    /// narrow it from both sides (9.5, 9.4.2).
    pub fn band(&self, y: LU, h: LU, cl: LU, cr: LU) -> (LU, LU) {
        let h = h.max(LU::from_raw(1));
        let (mut l, mut r) = (cl, cr);
        for f in &self.list {
            if f.t < y + h && f.b > y {
                if f.right {
                    r = r.min(f.l);
                } else {
                    l = l.max(f.r);
                }
            }
        }
        (l, r.max(l))
    }

    /// The smallest bottom edge, below `y`, of a float that overlaps the heights `y .. y + h`.
    pub fn next_bottom(&self, y: LU, h: LU) -> Option<LU> {
        let h = h.max(LU::from_raw(1));
        self.list.iter().filter(|f| f.t < y + h && f.b > y).map(|f| f.b).min()
    }

    /// The bottom of the lowest float on the sides `clear` names (9.5.2).
    pub fn clear_y(&self, clear: Clear) -> Option<LU> {
        let (left, right) = (matches!(clear, Clear::Left | Clear::Both), matches!(clear, Clear::Right | Clear::Both));
        self.list.iter().filter(|f| if f.right { right } else { left }).map(|f| f.b).max()
    }

    /// The lowest bottom of any float (what a block formatting context root grows to contain, 10.6.7).
    pub fn max_bottom(&self) -> Option<LU> {
        self.list.iter().map(|f| f.b).max()
    }

    /// Place a float whose margin box is `w` x `h` (9.5.1): as high as possible, not higher than `y_min` or the float
    /// before it, as far to its side as possible inside `[cl, cr]`; below the floats in the way when it does not fit
    /// beside them. Returns the margin box's corner.
    pub fn place(&mut self, right: bool, w: LU, h: LU, y_min: LU, cl: LU, cr: LU) -> (LU, LU) {
        let mut y = y_min.max(self.last_top);
        for _ in 0..MAX_FLOATS + 2 {
            let (mut l, mut r) = self.band(y, h, cl, cr);
            let mut fits = r - l >= w;
            if !fits && w > cr - cl {
                // Wider than the container, a float sticks out on its far side, but stays clear of the floats there
                // (9.5.1 rules 3 and 7: it may not reach past a float of the other side).
                let (l2, r2) = if right { self.band(y, h, cl - w, cr) } else { self.band(y, h, cl, cr + w) };
                // Only when no float of its own side is beside it, in the container or not (rule 7).
                if r2 - l2 >= w && !self.list.iter().any(|f| f.right == right && f.t < y + h && f.b > y) {
                    (l, r) = (l2, r2);
                    fits = true;
                }
            }
            let next = if fits { None } else { self.next_bottom(y, h) };
            if fits || next.is_none() {
                // Fits, or nothing is in the way any more (too wide for the container: it overflows on its side).
                let x = if right { r - w } else { l };
                if self.list.len() < MAX_FLOATS {
                    self.list.push(FloatBox { right, l: x, t: y, r: x + w, b: y + h });
                }
                self.last_top = y;
                return (x, y);
            }
            y = next.unwrap_or(y);
        }
        (cl, y)
    }
}

/// Everything layout appends to, as it was when a block's layout started (see the module comment).
#[derive(Clone, Copy)]
pub(crate) struct Mark {
    lines: usize,
    fragments: usize,
    glyphs: usize,
    texts: usize,
    decorations: usize,
    markers: usize,
    oof: usize,
    pending: usize,
    cut: usize,
    floats: usize,
    last_top: LU,
}

/// The floats whose placeholders are in the inline content of `owner`: its children, and the children of the inline
/// boxes in it.
pub(crate) fn floats_in(tree: &super::boxtree::BoxTree, owner: BoxId, out: &mut Vec<BoxId>) {
    for &c in &tree.get(owner).children {
        let b = tree.get(c);
        if is_float(b) {
            out.push(c);
        } else if b.kind == BoxKind::Inline {
            floats_in(tree, c, out);
        }
    }
}

pub(crate) fn is_float(b: &super::boxtree::LayoutBox) -> bool {
    // (Text and inline boxes carry the style of their element, `float` included; only the box the element makes floats.)
    matches!(b.kind, BoxKind::Block | BoxKind::InlineBlock | BoxKind::Replaced) && !b.oof && b.style.boxs.float != Float::None
}

impl Layouter<'_> {
    pub(crate) fn mark(&self) -> Mark {
        Mark {
            lines: self.lines.len(),
            fragments: self.fragments.len(),
            glyphs: self.glyphs.len(),
            texts: self.texts.len(),
            decorations: self.decorations.len(),
            markers: self.markers.len(),
            oof: self.oof_list.len(),
            pending: self.pending.len(),
            cut: self.cut.len(),
            floats: self.fs.list.len(),
            last_top: self.fs.last_top,
        }
    }

    pub(crate) fn restore(&mut self, m: Mark) {
        self.lines.truncate(m.lines);
        self.fragments.truncate(m.fragments);
        self.glyphs.truncate(m.glyphs);
        self.texts.truncate(m.texts);
        self.decorations.truncate(m.decorations);
        self.markers.truncate(m.markers);
        self.oof_list.truncate(m.oof);
        self.pending.truncate(m.pending);
        self.cut.truncate(m.cut);
        self.fs.list.truncate(m.floats);
        self.fs.last_top = m.last_top;
    }

    /// The top margin that will show above block `id` once the margins of its first children that touch it have
    /// collapsed into it (8.3.1), as far as it can be told without laying it out. Only a guess: when it is wrong the
    /// block is laid out again.
    pub(crate) fn est_top_margin(&self, id: BoxId, cb_w: LU, depth: u32) -> Margin {
        let tree = self.tree;
        let bx = tree.get(id);
        let bs = &bx.style.boxs;
        let own = Margin::from(block::resolve_margin(&bs.margin[0], cb_w).unwrap_or(LU::ZERO));
        let solid = bx.kind != BoxKind::Block
            || bx.inline_children
            || bx.fallback.is_some()
            || bx.oof
            || bs.display == Display::FlowRoot
            || block::border_px(bx.style.border.width[0]) != LU::ZERO
            || block::pad_px(&bs.padding[0], cb_w) != LU::ZERO
            || matches!(bs.overflow_x, crate::css::types::Overflow::Hidden | crate::css::types::Overflow::Scroll | crate::css::types::Overflow::Auto)
            || matches!(bs.overflow_y, crate::css::types::Overflow::Hidden | crate::css::types::Overflow::Scroll | crate::css::types::Overflow::Auto);
        if solid || depth > 24 {
            return own;
        }
        match bx.children.first() {
            Some(&c) => own.join(self.est_top_margin(c, cb_w, depth + 1)),
            None => own,
        }
    }

    /// Lay out the float `id` for a line of an owner block `owner_cbw` wide, as a block formatting context of its own
    /// (an inline-block's layout: `auto` width is shrink-to-fit, 10.3.5). Nothing is placed yet.
    pub(crate) fn measure_float(&mut self, id: BoxId, owner_cbw: LU) -> FloatDims {
        self.touched = true;
        let (w, _, _, ml, mt, _) = self.layout_atomic(id, owner_cbw);
        let o = self.out.get(id as usize).copied().unwrap_or_default();
        let bs = &self.tree.get(id).style.boxs;
        let mb = block::resolve_margin(&bs.margin[2], owner_cbw).unwrap_or(LU::ZERO);
        FloatDims { w, h: mt + o.h + mb, ml, mt }
    }

    /// Place the float `id` (measured as `d`) in the context (CSS2 9.5.1), no higher than the height `y` (relative to
    /// the border box of the owner block, whose content box starts `owner_left` from its left edge and is
    /// `owner_cbw` wide). `self.pos` is the owner's position.
    pub(crate) fn put_float(&mut self, id: BoxId, d: FloatDims, owner_cbw: LU, owner_left: LU, y: LU) {
        self.touched = true;
        let bs = &self.tree.get(id).style.boxs;
        let (cl, cr) = (self.pos.0 + owner_left, self.pos.0 + owner_left + owner_cbw);
        let mut y_min = self.pos.1 + y;
        if bs.clear != Clear::None
            && let Some(c) = self.fs.clear_y(bs.clear)
        {
            y_min = y_min.max(c);
        }
        let (fx, fy) = self.fs.place(bs.float == Float::Right, d.w, d.h, y_min, cl, cr);
        if let Some(out) = self.out.get_mut(id as usize) {
            out.x = fx + d.ml - self.pos.0;
            out.y = fy + d.mt - self.pos.1;
        }
    }

    /// The band of a line at BFC height `y` (page-flow height of the owner's content top + `y_local`): the left offset
    /// from the content box edge and the width.
    pub(crate) fn line_band(&mut self, owner_left: LU, owner_cbw: LU, y_local: LU, h: LU) -> (LU, LU) {
        if self.fs.list.is_empty() {
            return (LU::ZERO, owner_cbw);
        }
        self.touched = true;
        let (cl, cr) = (self.pos.0 + owner_left, self.pos.0 + owner_left + owner_cbw);
        let (l, r) = self.fs.band(self.pos.1 + y_local, h, cl, cr);
        (l - cl, r - l)
    }

    /// The next height (relative to the owner's border box) below `y_local` at which floats stop being in the way of a
    /// line `h` high.
    pub(crate) fn line_next_bottom(&self, y_local: LU, h: LU) -> Option<LU> {
        self.fs.next_bottom(self.pos.1 + y_local, h).map(|b| b - self.pos.1)
    }

    /// A block formatting context root in the flow (`overflow: hidden`, `flow-root` ...) may not overlap the floats
    /// of the context it sits in (9.5): its width is what the floats leave at height `gy`, and if it does not fit
    /// beside them it goes below. `base` is the BFC position of the parent's border box, `left` the parent's content
    /// left, `cbw` its content width. Returns `(width to lay out in, x shift, y in the parent)`.
    pub(crate) fn avoid_floats(&mut self, cid: BoxId, base: (LU, LU), left: LU, cbw: LU, gy: LU) -> (LU, LU, LU) {
        self.touched = true;
        let (cl, cr) = (base.0 + left, base.0 + left + cbw);
        let bx = self.tree.get(cid);
        // Its margin box must fit: an `auto` width fills the room; a given width needs `width + margins`.
        let bs = &bx.style.boxs;
        let want = match &bs.width {
            Dim::Lp(l) if !block::lp_has_percent(l) => {
                let m = |i: usize| bs.margin.get(i).and_then(|d| block::resolve_margin(d, cbw)).unwrap_or(LU::ZERO);
                let w = LU::from_f32(l.resolve(0.0));
                let extra = block::pad_px(&bs.padding[1], cbw) + block::pad_px(&bs.padding[3], cbw) + block::border_px(bx.style.border.width[1]) + block::border_px(bx.style.border.width[3]);
                // A given width over-constrains the margins, and the right one is the one given up (10.3.3): only the left margin counts.
                Some(if bs.box_sizing == crate::css::types::BoxSizing::BorderBox { w } else { w + extra } + m(3))
            }
            _ => None,
        };
        let mut y = gy;
        for _ in 0..MAX_FLOATS + 2 {
            let (l, r) = self.fs.band(base.1 + y, LU::from_raw(1), cl, cr);
            let fits = (l == cl && r == cr) || (r > l && want.is_none_or(|w| w <= r - l));
            if fits {
                return (r - l, l - cl, y);
            }
            match self.fs.next_bottom(base.1 + y, LU::from_raw(1)) {
                Some(b) => y = b - base.1,
                None => return (r - l, l - cl, y),
            }
        }
        (cbw, LU::ZERO, y)
    }
}
