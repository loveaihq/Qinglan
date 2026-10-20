//! Block layout: widths (CSS2 10.3.3), min/max (10.4, 10.7), heights (10.6.3), percentages (10.2, 10.5),
//! `box-sizing` (css-sizing-3), intrinsic-size keywords (css-sizing-3 3), margin collapsing (CSS2 8.3.1),
//! relative positioning (CSS2 9.3.2) and list markers (css-lists-3 3).
//!
//! A block is laid out knowing only its containing block's width (and height when definite); its own
//! position is set by the parent afterwards. That works because margin collapsing can be described
//! bottom-up: a block reports the margin that sticks out of its top and bottom edge (`mt`, `mb`,
//! already merged with those of the first/last child where CSS2 8.3.1 says they are adjoining) and
//! whether its own margins collapse through it (`through`); the parent then merges margins between
//! siblings and places the children.

use super::boxtree::{BoxId, BoxKind, Control, LayoutBox};
use super::units::{LayoutUnit as LU, Margin};
use super::{BoxOut, Fragment, FragKind, Layouter};
use crate::css::computed::{BoxStyle, slant_of};
use crate::css::types::{BoxSizing, Clear, Dim, Display, ListStylePosition, Overflow, Position};
use crate::css::values::Lp;
use crate::platform::FontQuery;

pub(crate) struct BlockResult {
    /// Border-box height (the width is in `out[id].w`).
    pub h: LU,
    /// Used margin-left.
    pub ml: LU,
    /// Margin at the top and bottom edge as seen by the parent.
    pub mt: Margin,
    pub mb: Margin,
    /// For a block that collapses through: the top margin alone, which decides where it is placed
    /// ("as if it had a non-zero bottom border", CSS2 8.3.1). Equal to `mt` otherwise.
    pub top_only: Margin,
    /// The block's own bottom margin alone (what is left of it when clearance keeps it from collapsing through).
    pub bot_only: Margin,
    /// An empty block whose top and bottom margins are adjoining (CSS2 8.3.1 last paragraphs).
    pub through: bool,
    /// The `position: relative` offset (CSS2 9.3.2), added to the box's position by whoever places it.
    pub rel: (LU, LU),
}

/// The edges and size specifications of a block (see `block_geometry`).
pub(crate) struct Geom {
    pub(super) pad: [LU; 4],
    pub(super) bor: [LU; 4],
    pub(super) h_extra: LU,
    pub(super) v_extra: LU,
    pub(super) m_top: LU,
    pub(super) m_bot: LU,
    pub(super) content_w: LU,
    pub(super) ml: LU,
    pub(super) spec_h: Option<LU>,
    /// The computed `height` is `auto` or 0 (CSS2 8.3.1: such a box can collapse through).
    pub(super) h_zero_or_auto: bool,
    pub(super) min_h: LU,
    pub(super) max_h: Option<LU>,
    pub(super) repl: Option<(LU, LU)>,
}

/// How a block is laid out beyond the ordinary rules.
#[derive(Clone, Copy, Default)]
pub(crate) struct Flow {
    pub root: bool,
    /// Content width fixed by the caller (atomic inlines, out-of-flow boxes).
    pub forced_w: Option<LU>,
    /// Content height fixed by the caller (out-of-flow boxes with `top` and `bottom`).
    pub forced_h: Option<LU>,
    /// A table cell (table.rs): these are its used border widths. A cell has no margins and its height is its content's.
    pub cell_bor: Option<[LU; 4]>,
    /// Lay the box out as if its `height`, `min-height` and `max-height` were `auto` / 0 / `none` (a flex item's content size, flex.rs).
    pub ignore_h: bool,
    /// The content of a frame root (vertical.rs) is laid out: its geometry is `Layouter::inner_geom`, or the box's own.
    pub inner: bool,
}

pub(crate) fn lp_has_percent(l: &Lp) -> bool {
    match l {
        Lp::Len(..) => false,
        Lp::Pct(_) => true,
        Lp::Calc(c) => c.has_percent(),
    }
}

/// A size property as a length; `None` for `auto`, `none`, the intrinsic-size keywords (see
/// `Layouter::kw_size`) and for percentages without a definite base.
pub(crate) fn resolve_dim(d: &Dim, base: Option<LU>) -> Option<LU> {
    match d {
        Dim::Lp(l) if !lp_has_percent(l) => Some(LU::from_f32(l.resolve(0.0))),
        Dim::Lp(l) => base.map(|b| LU::from_f32(l.resolve(b.to_f32()))),
        _ => None,
    }
}

/// A margin; `None` for `auto`.
pub(crate) fn resolve_margin(d: &Dim, base: LU) -> Option<LU> {
    match d {
        Dim::Auto => None,
        Dim::Lp(l) => Some(LU::from_f32(l.resolve(base.to_f32()))),
        _ => Some(LU::ZERO),
    }
}

pub(crate) fn pad_px(l: &Lp, base: LU) -> LU {
    LU::from_f32(l.resolve(base.to_f32())).max0()
}

/// Border widths are whole px, and a thin border does not vanish (like Blink).
pub(crate) fn border_px(w: f32) -> LU {
    if w >= 1.0 {
        LU::from_f32(w.floor())
    } else if w > 0.0 {
        LU::px(1)
    } else {
        LU::ZERO
    }
}

/// `overflow` values that make the box a scroll container, which starts a new block formatting
/// context. (Layer 1 does not scroll or clip for `auto`/`scroll`, but they still contain margins.)
fn overflow_bfc(bs: &BoxStyle) -> bool {
    let is = |o: Overflow| matches!(o, Overflow::Hidden | Overflow::Scroll | Overflow::Auto);
    is(bs.overflow_x) || is(bs.overflow_y)
}

/// Boxes that start a new block formatting context: their margins do not collapse with their
/// children's (CSS2 9.4.1, 8.3.1).
fn establishes_bfc(bx: &LayoutBox, bs: &BoxStyle) -> bool {
    overflow_bfc(bs) || bs.display == Display::FlowRoot || super::floats::is_float(bx) || bx.fallback.is_some() || bx.oof || bx.flex_item || matches!(bx.kind, BoxKind::InlineBlock | BoxKind::Replaced)
}

/// CSS2 10.3.3 and 10.4: content width and the two horizontal margins.
#[allow(clippy::too_many_arguments)]
pub(crate) fn solve_width(cb: LU, w: Option<LU>, ml: Option<LU>, mr: Option<LU>, extra: LU, min: LU, max: Option<LU>, rtl: bool) -> (LU, LU, LU) {
    let inner = |w: Option<LU>| -> (LU, LU, LU) {
        match w {
            None => {
                let (l, r) = (ml.unwrap_or(LU::ZERO), mr.unwrap_or(LU::ZERO));
                ((cb - l - r - extra).max0(), l, r)
            }
            Some(w) => {
                let used = w + extra;
                match (ml, mr) {
                    (None, None) => {
                        let rest = cb - used;
                        if rest > LU::ZERO {
                            let l = rest.half();
                            (w, l, rest - l)
                        } else {
                            // Over-constrained: auto margins count as 0, the margin at the end takes the rest.
                            if rtl { (w, rest, LU::ZERO) } else { (w, LU::ZERO, rest) }
                        }
                    }
                    (None, Some(r)) => (w, cb - used - r, r),
                    (Some(l), None) => (w, l, cb - used - l),
                    // Over-constrained: the margin at the end (right for ltr, left for rtl) is ignored and recomputed.
                    (Some(l), Some(r)) => {
                        if rtl {
                            (w, cb - used - r, r)
                        } else {
                            (w, l, cb - used - l)
                        }
                    }
                }
            }
        }
    };
    let mut r = inner(w);
    if let Some(m) = max
        && r.0 > m
    {
        r = inner(Some(m));
    }
    if r.0 < min {
        r = inner(Some(min));
    }
    r
}

/// The `position: relative` offset (CSS2 9.3.2): `left` wins over `right`, `top` over `bottom`;
/// percentages refer to the containing block (vertical ones only when its height is definite).
pub(super) fn relative_offset(bs: &BoxStyle, cb_w: LU, cb_h: Option<LU>) -> (LU, LU) {
    if bs.position != Position::Relative {
        return (LU::ZERO, LU::ZERO);
    }
    let get = |d: &Dim, base: Option<LU>| match d {
        Dim::Lp(l) if !lp_has_percent(l) => Some(LU::from_f32(l.resolve(0.0))),
        Dim::Lp(l) => base.map(|b| LU::from_f32(l.resolve(b.to_f32()))),
        _ => None,
    };
    let [top, right, bottom, left] = &bs.inset;
    let dx = get(left, Some(cb_w)).or_else(|| get(right, Some(cb_w)).map(|v| -v)).unwrap_or(LU::ZERO);
    let dy = get(top, cb_h).or_else(|| get(bottom, cb_h).map(|v| -v)).unwrap_or(LU::ZERO);
    (dx, dy)
}

impl Layouter<'_> {
    pub(super) fn layout_root(&mut self) {
        let root = self.tree.root;
        let (vw, vh) = self.viewport;
        let r = self.layout_block(root, vw, Some(vh), Flow { root: true, ..Flow::default() });
        if let Some(o) = self.out.get_mut(root as usize) {
            o.x = r.ml + r.rel.0;
            // A `vertical-rl` root starts at the right edge of the viewport (css-writing-modes-3 3.1: the initial containing block takes its mode).
            if self.tree.get(root).froot && matches!(self.tree.get(root).style.text.writing_mode, crate::css::types::WritingMode::VerticalRl | crate::css::types::WritingMode::SidewaysRl) {
                o.x = vw - o.w;
            }
            o.y = r.mt.resolve() + r.rel.1;
        }
        self.resolve_pending(None, 0);
    }

    /// Lay out the block `id` inside a containing block of content width `cb_w` (and content height
    /// `cb_h` when it is definite). Fills in `out[id]` except for `x`/`y`.
    pub(super) fn layout_block(&mut self, id: BoxId, cb_w: LU, cb_h: Option<LU>, flow: Flow) -> BlockResult {
        let tree = self.tree;
        let bx = tree.get(id);
        let is_root = flow.root;
        if bx.froot && !flow.inner {
            return self.layout_frame(id, cb_w, cb_h, flow);
        }
        if matches!(bx.fallback, Some(Display::Table | Display::InlineTable)) {
            return self.layout_table(id, cb_w, cb_h, flow);
        }
        if matches!(bx.fallback, Some(Display::Flex | Display::InlineFlex)) {
            return self.layout_flex(id, cb_w, cb_h, flow);
        }
        if matches!(bx.fallback, Some(Display::Grid | Display::InlineGrid)) {
            return self.layout_grid(id, cb_w, cb_h, flow);
        }

        if !self.charge(4 + bx.children.len() as u64) {
            return BlockResult { h: LU::ZERO, ml: LU::ZERO, mt: Margin::default(), mb: Margin::default(), top_only: Margin::default(), bot_only: Margin::default(), through: false, rel: (LU::ZERO, LU::ZERO) };
        }
        // The geometry is worked out in a function of its own so that its many temporaries do not
        // sit in this frame while the children (the recursion) are laid out.
        let g = match self.inner_geom.take().filter(|_| flow.inner) {
            Some(g) => g,
            None => self.block_geometry(id, cb_w, cb_h, flow),
        };

        // Floats (floats.rs): a block formatting context root has floats of its own, and its position in it is its own
        // corner; any other block lays out at the position `self.pos` its parent guessed for it.
        let bfc = is_root || flow.inner || establishes_bfc(bx, &bx.style.boxs);
        let saved_pos = self.pos;
        let saved_touched = self.touched;
        let saved_fs = bfc.then(|| std::mem::take(&mut self.fs));
        let base = if bfc { (LU::ZERO, LU::ZERO) } else { (self.pos.0 + g.ml, self.pos.1) };
        self.pos = base;

        let positioned = super::makes_cb(&bx.style);
        if positioned {
            self.cb_stack.push(id);
        }
        let pending_mark = self.pending.len();

        // Which margins of this box may collapse with its content (8.3.1).
        let top_collapses = !bfc && g.bor[0] == LU::ZERO && g.pad[0] == LU::ZERO;
        let bottom_collapses = !bfc && g.bor[2] == LU::ZERO && g.pad[2] == LU::ZERO && g.spec_h.is_none() && g.min_h == LU::ZERO;
        // A box's own top and bottom margins collapse when its height is 0 or auto (CSS2 8.3.1); only with `auto`
        // does the bottom margin also merge with that of the last child.
        let collapses_through = !bfc && g.bor[2] == LU::ZERO && g.pad[2] == LU::ZERO && g.h_zero_or_auto && g.min_h == LU::ZERO;
        let (content_left, content_top) = (g.bor[3] + g.pad[3], g.bor[0] + g.pad[0]);
        let (m_top, m_bot) = (g.m_top, g.m_bot);

        let mut mt = Margin::from(m_top);
        let mut mb = Margin::from(m_bot);
        let mut top_only = mt;
        let mut through = false;
        let content_h_auto;

        if g.repl.is_some() {
            content_h_auto = LU::ZERO;
        } else if bx.inline_children {
            content_h_auto = self.layout_inline(id, g.content_w, content_left, content_top);
            let has_lines = self.out.get(id as usize).is_some_and(|o| o.lines.1 > o.lines.0);
            if top_collapses && collapses_through && !has_lines {
                through = true;
                top_only = mt;
                mt = mt.join(mb);
                mb = mt;
            }
        } else {
            let mut cursor = content_top;
            // The margin waiting between `cursor` and the next child. While `at_top`, it is this box's
            // own top margin merged with those of the children that touch it.
            let mut strut = if top_collapses { Margin::from(m_top) } else { Margin::default() };
            let mut at_top = top_collapses;
            let mut top_margin = Margin::from(m_top);
            for &cid in &bx.children {
                if self.exhausted {
                    break;
                }
                // First-screen-first: a child that would start at or below the stop line is left out, and
                // so is everything after it. The estimate is the child's top without the margin it may still collapse with.
                let est = self.origin_y + if at_top { content_top } else { cursor + strut.resolve() };
                if self.past_stop(est) {
                    self.cut.push(id);
                    break;
                }
                let saved_origin = std::mem::replace(&mut self.origin_y, est);
                // Where the child will be, as far as the margins in sight tell (floats.rs); the real place is known only
                // once it is laid out.
                let cbx = tree.get(cid);
                let floats_on = self.has_floats;
                let gy = if at_top {
                    content_top
                } else if floats_on {
                    cursor + strut.join(self.est_top_margin(cid, g.content_w, 0)).resolve()
                } else {
                    cursor
                };
                let avoid = floats_on && !self.fs.list.is_empty() && cbx.kind == BoxKind::Block && establishes_bfc(cbx, &cbx.style.boxs);
                let (mut cw, mut shift, mut gy2) = (g.content_w, LU::ZERO, gy);
                let mark = self.mark();
                // The floats it clears are the ones before it, not those inside it.
                let clear_b = if floats_on && cbx.style.boxs.clear != Clear::None { self.fs.clear_y(cbx.style.boxs.clear) } else { None };
                let outer_touched = std::mem::replace(&mut self.touched, false);
                if avoid {
                    (cw, shift, gy2) = self.avoid_floats(cid, base, content_left, g.content_w, gy);
                }
                self.pos = (base.0 + content_left + shift, base.1 + gy2);
                let mut r = self.layout_block(cid, cw, g.spec_h, Flow::default());
                let place = |r: &BlockResult, at_top: bool, cursor: LU, strut: Margin| -> LU {
                    if at_top {
                        content_top
                    } else if r.through {
                        // Placed where it would be if it had a bottom border (8.3.1).
                        cursor + strut.join(r.top_only).resolve()
                    } else {
                        cursor + strut.join(r.mt).resolve()
                    }
                };
                let mut y = place(&r, at_top, cursor, strut);
                let mut cleared = false;
                if floats_on {
                    if avoid {
                        y = y.max(gy2);
                        // It may not overlap a float anywhere along its height (9.5): where one comes in later and takes
                        // too much room it goes down to where that float ends.
                        for _ in 0..64 {
                            let h = r.h.max(LU::from_raw(1));
                            let width = self.out.get(cid as usize).map_or(LU::ZERO, |o| o.w);
                            let (cl, cr) = (base.0 + content_left, base.0 + content_left + g.content_w);
                            let (l2, r2) = self.fs.band(base.1 + y, h, cl, cr);
                            let at = cl + shift;
                            if (l2 <= at && at + r.ml + width <= r2) || (l2 == cl && r2 == cr) {
                                break;
                            }
                            let Some(nb) = self.fs.next_bottom(base.1 + y, h) else { break };
                            self.restore(mark);
                            (cw, shift, y) = self.avoid_floats(cid, base, content_left, g.content_w, nb - base.1);
                            self.pos = (base.0 + content_left + shift, base.1 + y);
                            self.touched = false;
                            r = self.layout_block(cid, cw, g.spec_h, Flow::default());
                            gy2 = y;
                        }
                    }
                    // Clearance (9.5.2): the top border edge goes down to the bottom of the floats it clears.
                    if let Some(b) = clear_b {
                        self.touched = true;
                        // Clearance also keeps an empty block's margins from collapsing through it (CSS2 8.3.1).
                        cleared = b - base.1 > y;
                        y = y.max(b - base.1);
                    }
                    if self.touched && y != gy2 {
                        if avoid {
                            // Clearance moved it: the room beside the floats is not where it was.
                            (cw, shift, _) = self.avoid_floats(cid, base, content_left, g.content_w, y);
                        }
                        self.restore(mark);
                        self.pos = (base.0 + content_left + shift, base.1 + y);
                        self.touched = false;
                        r = self.layout_block(cid, cw, g.spec_h, Flow::default());
                    }
                }
                self.touched |= outer_touched;
                self.origin_y = saved_origin;
                if r.through && !cleared {
                    strut = strut.join(r.mt);
                } else {
                    let (top_m, bot_m) = if r.through { (r.top_only, r.bot_only) } else { (r.mt, r.mb) };
                    if at_top {
                        strut = strut.join(top_m);
                        top_margin = strut;
                        at_top = false;
                    }
                    cursor = y + r.h;
                    strut = bot_m;
                }
                if let Some(o) = self.out.get_mut(cid as usize) {
                    o.x = content_left + shift + r.ml + r.rel.0;
                    o.y = y + r.rel.1;
                }
            }
            if at_top {
                // No child with a box of its own: nothing but empty blocks (or no blocks at all).
                if top_collapses {
                    top_margin = strut;
                }
                if top_collapses && collapses_through {
                    through = true;
                    top_only = top_margin;
                    mt = top_margin.join(Margin::from(m_bot));
                    mb = mt;
                } else {
                    mt = top_margin;
                    top_only = mt;
                }
                content_h_auto = LU::ZERO;
            } else {
                mt = top_margin;
                top_only = mt;
                let end;
                if bottom_collapses {
                    mb = Margin::from(m_bot).join(strut);
                    end = cursor;
                } else {
                    end = cursor + strut.resolve();
                }
                content_h_auto = (end - content_top).max0();
            }
        }

        // A block formatting context root reaches down to the floats in it (10.6.7).
        let content_h_auto = match (bfc, self.fs.max_bottom()) {
            (true, Some(fb)) if g.repl.is_none() => content_h_auto.max(fb - content_top),
            _ => content_h_auto,
        };
        self.finish_box(id, &g, content_h_auto, content_top);
        self.pos = saved_pos;
        if let Some(fs) = saved_fs {
            self.fs = fs;
            self.touched = saved_touched;
        }
        if positioned {
            self.cb_stack.pop();
            self.resolve_pending(Some(id), pending_mark);
        }
        let rel = relative_offset(&bx.style.boxs, cb_w, cb_h);
        BlockResult { h: self.out.get(id as usize).map_or(LU::ZERO, |o| o.h), ml: g.ml, mt, mb, top_only, bot_only: Margin::from(m_bot), through, rel }
    }

    /// Edges, width and height specifications of block `id` in a containing block `cb_w` wide.
    pub(super) fn block_geometry(&mut self, id: BoxId, cb_w: LU, cb_h: Option<LU>, flow: Flow) -> Geom {
        let tree = self.tree;
        let bx = tree.get(id);
        let style = bx.style.clone();
        let bs: &BoxStyle = &style.boxs;
        let bd = &style.border;

        // Edges. Percentages of padding and margins, vertical ones too, refer to the containing
        // block's width (CSS2 8.3, 8.4).
        let pad = [pad_px(&bs.padding[0], cb_w), pad_px(&bs.padding[1], cb_w), pad_px(&bs.padding[2], cb_w), pad_px(&bs.padding[3], cb_w)];
        let cell = flow.cell_bor.is_some();
        let bor = flow.cell_bor.unwrap_or([border_px(bd.width[0]), border_px(bd.width[1]), border_px(bd.width[2]), border_px(bd.width[3])]);
        let marg = if cell {
            [Some(LU::ZERO); 4]
        } else {
            [resolve_margin(&bs.margin[0], cb_w), resolve_margin(&bs.margin[1], cb_w), resolve_margin(&bs.margin[2], cb_w), resolve_margin(&bs.margin[3], cb_w)]
        };
        let h_extra = pad[1] + pad[3] + bor[1] + bor[3];
        let v_extra = pad[0] + pad[2] + bor[0] + bor[2];
        let border_box = bs.box_sizing == BoxSizing::BorderBox;
        let to_cw = |v: LU| if border_box { (v - h_extra).max0() } else { v };
        let to_ch = |v: LU| if border_box { (v - v_extra).max0() } else { v };

        // Replaced elements: the content size comes from the element (CSS2 10.3.2, 10.6.2).
        let repl = (bx.kind == BoxKind::Replaced).then(|| self.replaced_size_final(id, cb_w, cb_h));

        // Width (10.3.3) with min/max (10.4). The intrinsic-size keywords (`min-content` ...) give a
        // content-box size whatever `box-sizing` says.
        let avail = (cb_w - marg[3].unwrap_or(LU::ZERO) - marg[1].unwrap_or(LU::ZERO) - h_extra).max0();
        let spec_w = match self.kw_size(id, &bs.width, avail) {
            Some(v) => Some(v),
            None => resolve_dim(&bs.width, Some(cb_w)).map(to_cw),
        };
        let min_w = match self.kw_size(id, &bs.min_width, avail) {
            Some(v) => v,
            None => resolve_dim(&bs.min_width, Some(cb_w)).map_or(LU::ZERO, to_cw),
        };
        let max_w = match self.kw_size(id, &bs.max_width, avail) {
            Some(v) => Some(v),
            None => resolve_dim(&bs.max_width, Some(cb_w)).map(to_cw),
        };
        // The direction of the containing block decides which margin is dropped when they are over-constrained (10.3.3).
        let cb_rtl = bx.parent != super::boxtree::NO_BOX && self.tree.get(bx.parent).style.text.direction == crate::css::types::Direction::Rtl;
        let (content_w, ml, _mr) = if let Some(fw) = flow.forced_w {
            (fw, marg[3].unwrap_or(LU::ZERO), marg[1].unwrap_or(LU::ZERO))
        } else if let Some((rw, _)) = repl {
            solve_width(cb_w, Some(rw), marg[3], marg[1], h_extra, LU::ZERO, None, cb_rtl)
        } else {
            solve_width(cb_w, spec_w, marg[3], marg[1], h_extra, min_w, max_w, cb_rtl)
        };

        // Height: a percentage needs a definite containing block height, otherwise it is `auto`
        // (10.5). Min/max (10.7) apply to the result.
        let sized = !cell && !flow.ignore_h;
        let spec_h = if sized { resolve_dim(&bs.height, cb_h).map(to_ch) } else { None };
        let h_zero_or_auto = flow.forced_h.is_none() && repl.is_none() && spec_h.is_none_or(|h| h == LU::ZERO);
        let min_h = if sized { resolve_dim(&bs.min_height, cb_h).map_or(LU::ZERO, to_ch) } else { LU::ZERO };
        let max_h = if sized { resolve_dim(&bs.max_height, cb_h).map(to_ch) } else { None };
        let clamp_h = |h: LU| max_h.map_or(h, |m| h.min(m)).max(min_h);
        let spec_h = match (flow.forced_h, repl) {
            (Some(h), _) => Some(clamp_h(h)),
            (None, Some((_, rh))) => Some(rh),
            (None, None) => spec_h.map(clamp_h),
        };
        Geom { pad, bor, h_extra, v_extra, m_top: marg[0].unwrap_or(LU::ZERO), m_bot: marg[2].unwrap_or(LU::ZERO), content_w, ml, spec_h, h_zero_or_auto, min_h, max_h, repl }
    }

    /// Final height, size and flags of block `id` once its content is laid out; list marker.
    pub(super) fn finish_box(&mut self, id: BoxId, g: &Geom, content_h_auto: LU, content_top: LU) {
        let tree = self.tree;
        let bx = tree.get(id);
        let bs = &bx.style.boxs;
        let mut content_h_auto = content_h_auto;
        // Text fields, selects and text areas are at least their rows high.
        if !matches!(bx.control(), Control::None) {
            let rf = self.font_for(&bx.style);
            let line_h = rf.asc + rf.desc + rf.lead_top + rf.lead_bottom;
            content_h_auto = match bx.control() {
                Control::TextArea(_, rows) => LU::from_raw(line_h.raw() as i64 * rows as i64),
                _ => content_h_auto.max(line_h),
            };
        }
        let content_h = match g.spec_h {
            Some(h) => h,
            None => g.max_h.map_or(content_h_auto, |m| content_h_auto.min(m)).max(g.min_h),
        };
        let w = g.content_w + g.h_extra;
        let h = content_h + g.v_extra;
        let clip = bx.force_clip
            || matches!(bx.control(), Control::TextArea(..))
            || matches!(bs.overflow_x, Overflow::Hidden | Overflow::Clip)
            || matches!(bs.overflow_y, Overflow::Hidden | Overflow::Clip);
        if let Some(o) = self.out.get_mut(id as usize) {
            let (lines, marker) = (o.lines, o.marker);
            *o = BoxOut { x: o.x, y: o.y, w, h, clip, hide: o.hide, lines, marker };
        }
        if bx.ordinal().is_some() {
            self.layout_marker(id, content_top);
        }
    }

    /// The baseline of the first line box in `id`'s subtree, relative to its border box.
    pub(super) fn first_baseline(&self, id: BoxId) -> Option<LU> {
        if let Some(b) = self.flex_base.get(&id) {
            return (b.0 != LU::MIN).then_some(b.0);
        }
        let bx = self.tree.get(id);
        let o = self.out.get(id as usize)?;
        if bx.inline_children {
            let l = self.lines.get(o.lines.0 as usize).filter(|_| o.lines.1 > o.lines.0)?;
            return Some(l.y + l.baseline);
        }
        bx.children.iter().find_map(|&c| {
            let b = self.first_baseline(c)?;
            Some(b + self.out.get(c as usize)?.y)
        })
    }

    /// The baseline of the last line box in `id`'s subtree, relative to its border box.
    pub(super) fn last_baseline(&self, id: BoxId) -> Option<LU> {
        if let Some(b) = self.flex_base.get(&id) {
            return (b.1 != LU::MIN).then_some(b.1);
        }
        let bx = self.tree.get(id);
        let o = self.out.get(id as usize)?;
        if bx.inline_children {
            let l = self.lines.get((o.lines.1 as usize).checked_sub(1)?).filter(|_| o.lines.1 > o.lines.0)?;
            return Some(l.y + l.baseline);
        }
        bx.children.iter().rev().find_map(|&c| {
            let b = self.last_baseline(c)?;
            Some(b + self.out.get(c as usize)?.y)
        })
    }

    /// The marker text of list item `id` (css-lists-3 3): the counter in the item's `list-style-type`
    /// plus its suffix, shaped in the item's font.
    pub(super) fn marker_shape(&mut self, id: BoxId) -> Option<(String, Vec<crate::platform::Glyph>, LU)> {
        let bx = self.tree.get(id);
        let mut text = super::lists::marker_string(bx)?;
        if bx.style.text.direction == crate::css::types::Direction::Rtl {
            // The marker of a right-to-left item reads right to left: its words (runs of letters and digits) keep their order,
            // the rest is turned around, so that "1. " is shown as " .1" (UAX #9 would do the same).
            let mut words: Vec<String> = Vec::new();
            let mut prev_word = false;
            for c in text.chars() {
                if c.is_alphanumeric() && prev_word {
                    if let Some(w) = words.last_mut() {
                        w.push(c);
                    }
                } else {
                    words.push(c.to_string());
                }
                prev_word = c.is_alphanumeric();
            }
            text = words.into_iter().rev().collect();
        }
        let rf = self.font_for(&bx.style);
        let f = &bx.style.font;
        let q = FontQuery { families: &f.family, weight: f.weight, slant: slant_of(f.style), lang: bx.style.text.lang.as_deref() };
        let glyphs = self.platform.shape(&text, &q, rf.size);
        let width = LU::from_f32(glyphs.iter().map(|g| g.advance).sum());
        Some((text, glyphs, width))
    }

    /// An outside marker sits just left of the list item's border box, on the baseline of its first
    /// line (an `inside` marker on an item without inline content of its own is placed the same way).
    fn layout_marker(&mut self, id: BoxId, content_top: LU) {
        let bx = self.tree.get(id);
        let inside = bx.style.text.list_style_position == ListStylePosition::Inside && bx.inline_children;
        if inside {
            return;
        }
        let Some((text, glyphs, width)) = self.marker_shape(id) else { return };
        let rf = self.font_for(&bx.style);
        let baseline = self.first_baseline(id).unwrap_or(content_top + rf.lead_top + rf.asc);
        let gbase = self.glyphs.len() as u32;
        self.glyphs.extend(glyphs.iter().map(super::PlacedGlyph::from));
        let tid = self.texts.len() as u32;
        let len = text.len() as u32;
        self.texts.push(text);
        self.markers.push(Fragment {
            box_id: id,
            kind: FragKind::Marker { text: tid, start: 0, end: len, glyphs: (gbase, self.glyphs.len() as u32), asc: rf.asc },
            // Just outside the border box: on the left, or on the right for a right-to-left item.
            x: if bx.style.text.direction == crate::css::types::Direction::Rtl { self.out.get(id as usize).map_or(LU::ZERO, |o| o.w) } else { -width },
            y: baseline - rf.asc,
            w: width,
            h: rf.asc + rf.desc,
        });
        if let Some(o) = self.out.get_mut(id as usize) {
            o.marker = self.markers.len() as u32;
        }
    }
}
