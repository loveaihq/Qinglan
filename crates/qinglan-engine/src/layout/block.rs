//! Block layout: widths (CSS2 10.3.3), min/max (10.4, 10.7), heights (10.6.3), percentages (10.2, 10.5),
//! `box-sizing` (css-sizing-3) and margin collapsing (CSS2 8.3.1).
//!
//! A block is laid out knowing only its containing block's width (and height when definite); its own
//! position is set by the parent afterwards. That works because margin collapsing can be described
//! bottom-up: a block reports the margin that sticks out of its top and bottom edge (`mt`, `mb`,
//! already merged with those of the first/last child where CSS2 8.3.1 says they are adjoining) and
//! whether its own margins collapse through it (`through`); the parent then merges margins between
//! siblings and places the children.

use super::boxtree::{BoxId, LayoutBox};
use super::units::{LayoutUnit as LU, Margin};
use super::{BoxOut, Layouter};
use crate::css::computed::BoxStyle;
use crate::css::types::{BoxSizing, Dim, Display, Overflow};
use crate::css::values::Lp;

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
    /// An empty block whose top and bottom margins are adjoining (CSS2 8.3.1 last paragraphs).
    pub through: bool,
}

fn lp_has_percent(l: &Lp) -> bool {
    match l {
        Lp::Len(..) => false,
        Lp::Pct(_) => true,
        Lp::Calc(c) => c.has_percent(),
    }
}

/// A size property as a length; `None` for `auto`, `none`, the intrinsic-size keywords (not laid out
/// in 1b-1: they act like `auto`) and for percentages without a definite base.
fn resolve_dim(d: &Dim, base: Option<LU>) -> Option<LU> {
    match d {
        Dim::Lp(l) if !lp_has_percent(l) => Some(LU::from_f32(l.resolve(0.0))),
        Dim::Lp(l) => base.map(|b| LU::from_f32(l.resolve(b.to_f32()))),
        _ => None,
    }
}

/// A margin; `None` for `auto`.
fn resolve_margin(d: &Dim, base: LU) -> Option<LU> {
    match d {
        Dim::Auto => None,
        Dim::Lp(l) => Some(LU::from_f32(l.resolve(base.to_f32()))),
        _ => Some(LU::ZERO),
    }
}

fn pad_px(l: &Lp, base: LU) -> LU {
    LU::from_f32(l.resolve(base.to_f32())).max0()
}

/// Border widths are whole px, and a thin border does not vanish (like Blink).
fn border_px(w: f32) -> LU {
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
    overflow_bfc(bs) || bs.display == Display::FlowRoot || bx.fallback.is_some()
}

/// CSS2 10.3.3 and 10.4: content width and the two horizontal margins.
fn solve_width(cb: LU, w: Option<LU>, ml: Option<LU>, mr: Option<LU>, extra: LU, min: LU, max: Option<LU>) -> (LU, LU, LU) {
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
                            // Over-constrained: auto margins count as 0, margin-right takes the rest.
                            (w, LU::ZERO, rest)
                        }
                    }
                    (None, Some(r)) => (w, cb - used - r, r),
                    (Some(l), None) => (w, l, cb - used - l),
                    // Over-constrained (ltr): margin-right is ignored and recomputed.
                    (Some(l), Some(_)) => (w, l, cb - used - l),
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

impl Layouter<'_> {
    pub(super) fn layout_root(&mut self) {
        let root = self.tree.root;
        let (vw, vh) = self.viewport;
        let r = self.layout_block(root, vw, Some(vh), true);
        if let Some(o) = self.out.get_mut(root as usize) {
            o.x = r.ml;
            o.y = r.mt.resolve();
        }
    }

    /// Lay out the block `id` inside a containing block of content width `cb_w` (and content height
    /// `cb_h` when it is definite). Fills in `out[id]` except for `x`/`y`.
    pub(super) fn layout_block(&mut self, id: BoxId, cb_w: LU, cb_h: Option<LU>, is_root: bool) -> BlockResult {
        let tree = self.tree;
        let bx = tree.get(id);
        let style = bx.style.clone();
        let bs: &BoxStyle = &style.boxs;
        let bd = &style.border;

        // Edges. Percentages of padding and margins, vertical ones too, refer to the containing
        // block's width (CSS2 8.3, 8.4).
        let pad = [pad_px(&bs.padding[0], cb_w), pad_px(&bs.padding[1], cb_w), pad_px(&bs.padding[2], cb_w), pad_px(&bs.padding[3], cb_w)];
        let bor = [border_px(bd.width[0]), border_px(bd.width[1]), border_px(bd.width[2]), border_px(bd.width[3])];
        let marg = [
            resolve_margin(&bs.margin[0], cb_w),
            resolve_margin(&bs.margin[1], cb_w),
            resolve_margin(&bs.margin[2], cb_w),
            resolve_margin(&bs.margin[3], cb_w),
        ];
        let h_extra = pad[1] + pad[3] + bor[1] + bor[3];
        let v_extra = pad[0] + pad[2] + bor[0] + bor[2];
        let border_box = bs.box_sizing == BoxSizing::BorderBox;
        let to_cw = |v: LU| if border_box { (v - h_extra).max0() } else { v };
        let to_ch = |v: LU| if border_box { (v - v_extra).max0() } else { v };

        // Width (10.3.3) with min/max (10.4).
        let spec_w = resolve_dim(&bs.width, Some(cb_w)).map(to_cw);
        let min_w = resolve_dim(&bs.min_width, Some(cb_w)).map_or(LU::ZERO, to_cw);
        let max_w = resolve_dim(&bs.max_width, Some(cb_w)).map(to_cw);
        let (content_w, ml, _mr) = solve_width(cb_w, spec_w, marg[3], marg[1], h_extra, min_w, max_w);

        // Height: a percentage needs a definite containing block height, otherwise it is `auto`
        // (10.5). Min/max (10.7) apply to the result.
        let spec_h = resolve_dim(&bs.height, cb_h).map(to_ch);
        let min_h = resolve_dim(&bs.min_height, cb_h).map_or(LU::ZERO, to_ch);
        let max_h = resolve_dim(&bs.max_height, cb_h).map(to_ch);
        let clamp_h = |h: LU| max_h.map_or(h, |m| h.min(m)).max(min_h);
        let spec_h = spec_h.map(clamp_h);

        let content_left = bor[3] + pad[3];
        let content_top = bor[0] + pad[0];
        let m_top = marg[0].unwrap_or(LU::ZERO);
        let m_bot = marg[2].unwrap_or(LU::ZERO);

        // Which margins of this box may collapse with its content (8.3.1).
        let bfc = is_root || establishes_bfc(bx, bs);
        let top_collapses = !bfc && bor[0] == LU::ZERO && pad[0] == LU::ZERO;
        let bottom_collapses = !bfc && bor[2] == LU::ZERO && pad[2] == LU::ZERO && spec_h.is_none() && min_h == LU::ZERO;

        let mut mt = Margin::from(m_top);
        let mut mb = Margin::from(m_bot);
        let mut top_only = mt;
        let mut through = false;
        let content_h_auto;

        if bx.inline_children {
            content_h_auto = self.layout_inline(id, content_w, content_left, content_top);
            let has_lines = self.out.get(id as usize).is_some_and(|o| o.lines.1 > o.lines.0);
            if top_collapses && bottom_collapses && !has_lines {
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
                let r = self.layout_block(cid, content_w, spec_h, false);
                let y;
                if r.through {
                    // Placed where it would be if it had a bottom border (8.3.1).
                    if at_top {
                        y = content_top;
                    } else {
                        y = cursor + strut.join(r.top_only).resolve();
                    }
                    strut = strut.join(r.mt);
                } else {
                    if at_top {
                        strut = strut.join(r.mt);
                        top_margin = strut;
                        y = content_top;
                        at_top = false;
                    } else {
                        y = cursor + strut.join(r.mt).resolve();
                    }
                    cursor = y + r.h;
                    strut = r.mb;
                }
                if let Some(o) = self.out.get_mut(cid as usize) {
                    o.x = content_left + r.ml;
                    o.y = y;
                }
            }
            if at_top {
                // No child with a box of its own: nothing but empty blocks (or no blocks at all).
                if top_collapses {
                    top_margin = strut;
                }
                if top_collapses && bottom_collapses {
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

        let content_h = match spec_h {
            Some(h) => h,
            None => clamp_h(content_h_auto),
        };
        let w = content_w + h_extra;
        let h = content_h + v_extra;
        let clip = matches!(bs.overflow_x, Overflow::Hidden | Overflow::Clip) || matches!(bs.overflow_y, Overflow::Hidden | Overflow::Clip);
        if let Some(o) = self.out.get_mut(id as usize) {
            let lines = o.lines;
            *o = BoxOut { x: o.x, y: o.y, w, h, clip, lines };
        }
        BlockResult { h, ml, mt, mb, top_only, through }
    }
}
