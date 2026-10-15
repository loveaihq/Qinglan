//! Absolutely positioned boxes (CSS2 9.6, 10.3.7, 10.3.8, 10.6.4, 10.6.5) and `position: fixed`.
//!
//! An out-of-flow box is a placeholder in the inline run it sits in; when that run is laid out the
//! box is queued with its *static position* (CSS2 9.6.4: where it would have been in flow). Its
//! containing block is the padding box of the nearest positioned ancestor (the initial containing
//! block, the size of the viewport at the page origin, if there is none; for `fixed` always the
//! viewport). The box is laid out when its containing block is finished, because `right`, `bottom`
//! and percentages need the containing block's size.
//!
//! A `position: relative` inline element is the containing block of the out-of-flow boxes inside it
//! (CSS2 10.1 item 4): `inline::lay_lines` works out its padding box from the first and last piece of
//! the element and calls [`Layouter::layout_oof_in_rect`] once the paragraph is laid out. (An
//! out-of-flow box inside an inline-block inside such an element still uses the nearest positioned
//! block-level ancestor.) A `fixed` box is positioned in viewport coordinates at scroll position 0
//! (the host adds the scroll offset when it paints fixed boxes).

use super::block::{Flow, border_px, lp_has_percent, pad_px, resolve_dim, resolve_margin};
use super::boxtree::{BoxId, BoxKind, NO_BOX};
use super::units::LayoutUnit as LU;
use super::{Layouter, Pending};
use crate::css::types::{BoxSizing, Dim};

fn inset(d: &Dim, base: LU) -> Option<LU> {
    match d {
        Dim::Lp(l) => Some(LU::from_f32(l.resolve(if lp_has_percent(l) { base.to_f32() } else { 0.0 }))),
        _ => None,
    }
}

impl Layouter<'_> {
    /// Position of `from`'s border box relative to the border box of `to` (the page origin for `None`),
    /// following the layout parents.
    fn rel_pos(&self, from: BoxId, to: Option<BoxId>) -> (LU, LU) {
        let (mut x, mut y) = (LU::ZERO, LU::ZERO);
        let mut cur = from;
        let mut steps = 0;
        while cur != NO_BOX && Some(cur) != to && steps < 100_000 {
            if let Some(o) = self.out.get(cur as usize) {
                x += o.x;
                y += o.y;
            }
            cur = self.tree.layout_parent(cur);
            steps += 1;
        }
        (x, y)
    }

    /// Lay out the queued out-of-flow boxes whose containing block is `cb` and that were queued
    /// at or after index `mark` of the queue.
    pub(super) fn resolve_pending(&mut self, cb: Option<BoxId>, mark: usize) {
        if self.pending.len() <= mark {
            return;
        }
        let tail = self.pending.split_off(mark);
        let mut mine = Vec::new();
        for p in tail {
            if p.cb == cb {
                mine.push(p);
            } else {
                self.pending.push(p);
            }
        }
        for p in mine {
            self.layout_oof(p);
        }
    }

    fn layout_oof(&mut self, p: Pending) {
        self.layout_oof_with(p, None);
    }

    /// Lay out a box whose containing block is the padding box of a positioned inline element:
    /// `rect` is that box (x, y, w, h) in the coordinates of the owner block's border box.
    pub(super) fn layout_oof_in_rect(&mut self, p: Pending, rect: [LU; 4]) {
        self.layout_oof_with(p, Some(rect));
    }

    fn layout_oof_with(&mut self, p: Pending, inline_cb: Option<[LU; 4]>) {
        if !self.charge(8) || self.depth >= super::MAX_PARAGRAPH_DEPTH {
            return;
        }
        self.depth += 1;
        self.layout_oof_inner(p, inline_cb);
        self.depth -= 1;
    }

    fn layout_oof_inner(&mut self, p: Pending, inline_cb: Option<[LU; 4]>) {
        let tree = self.tree;
        let bx = tree.get(p.id);
        let style = bx.style.clone();
        let bs = &style.boxs;
        let bd = &style.border;

        // The containing block (its padding box) and where the owner of the static position sits in it.
        let (cw, ch, ox, oy) = match inline_cb {
            Some([x, y, w, h]) => (w, h, -x, -y),
            None => {
                let (cw, ch, bl, bt) = match p.cb {
                    Some(c) => {
                        let o = self.out.get(c as usize).copied().unwrap_or_default();
                        let cbd = &tree.get(c).style.border;
                        let b = |i: usize| border_px(cbd.width.get(i).copied().unwrap_or(0.0));
                        ((o.w - b(1) - b(3)).max0(), (o.h - b(0) - b(2)).max0(), b(3), b(0))
                    }
                    None => (self.viewport.0, self.viewport.1, LU::ZERO, LU::ZERO),
                };
                let (ox, oy) = self.rel_pos(p.owner, p.cb);
                (cw, ch, ox - bl, oy - bt)
            }
        };
        let (sx, sy) = (p.sx + ox, p.sy + oy);

        let pad = [pad_px(&bs.padding[0], cw), pad_px(&bs.padding[1], cw), pad_px(&bs.padding[2], cw), pad_px(&bs.padding[3], cw)];
        let bor = [border_px(bd.width[0]), border_px(bd.width[1]), border_px(bd.width[2]), border_px(bd.width[3])];
        let marg = [resolve_margin(&bs.margin[0], cw), resolve_margin(&bs.margin[1], cw), resolve_margin(&bs.margin[2], cw), resolve_margin(&bs.margin[3], cw)];
        let h_extra = pad[1] + pad[3] + bor[1] + bor[3];
        let v_extra = pad[0] + pad[2] + bor[0] + bor[2];
        let border_box = bs.box_sizing == BoxSizing::BorderBox;
        let to_cw = |v: LU| if border_box { (v - h_extra).max0() } else { v };
        let (top, right, bottom, left) = (inset(&bs.inset[0], ch), inset(&bs.inset[1], cw), inset(&bs.inset[2], ch), inset(&bs.inset[3], cw));
        let zero = LU::ZERO;

        // Horizontal (CSS2 10.3.7; 10.3.8 for replaced elements, whose width is never `auto`).
        let replaced_w = (bx.kind == BoxKind::Replaced).then(|| self.replaced_size(p.id, cw, Some(ch)).0);
        let kw = self.kw_size(p.id, &bs.width, cw);
        let spec_w = replaced_w.or(kw).or_else(|| resolve_dim(&bs.width, Some(cw)).map(to_cw));
        let min_w = self.kw_size(p.id, &bs.min_width, cw).unwrap_or_else(|| resolve_dim(&bs.min_width, Some(cw)).map_or(zero, to_cw));
        let max_w = self.kw_size(p.id, &bs.max_width, cw).or_else(|| resolve_dim(&bs.max_width, Some(cw)).map(to_cw));
        let (ml_s, mr_s) = (marg[3], marg[1]);
        let solve = |this: &mut Self, w: Option<LU>| -> (LU, LU, LU, LU) {
            // Returns (content width, margin-left, margin-right, left).
            let (ml0, mr0) = (ml_s.unwrap_or(zero), mr_s.unwrap_or(zero));
            let fit = |this: &mut Self, avail: LU| {
                let (mn, mx) = this.content_widths(p.id);
                mn.max(avail.max0()).min(mx)
            };
            match (left, w, right) {
                (None, None, None) => {
                    let w = fit(this, cw - sx - ml0 - mr0 - h_extra);
                    (w, ml0, mr0, sx)
                }
                (Some(l), Some(w), Some(r)) => {
                    let rest = cw - l - r - w - h_extra;
                    let (ml, mr) = match (ml_s, mr_s) {
                        (None, None) if rest >= zero => (rest.half(), rest - rest.half()),
                        (None, None) => (zero, rest),
                        (None, Some(mr)) => (rest - mr, mr),
                        (Some(ml), _) => (ml, rest - ml),
                    };
                    (w, ml, mr, l)
                }
                (None, None, Some(r)) => {
                    let w = fit(this, cw - r - ml0 - mr0 - h_extra);
                    (w, ml0, mr0, cw - r - mr0 - w - h_extra - ml0)
                }
                (None, Some(w), None) => (w, ml0, mr0, sx),
                (Some(l), None, None) => {
                    let w = fit(this, cw - l - ml0 - mr0 - h_extra);
                    (w, ml0, mr0, l)
                }
                (None, Some(w), Some(r)) => (w, ml0, mr0, cw - r - mr0 - w - h_extra - ml0),
                (Some(l), None, Some(r)) => ((cw - l - r - ml0 - mr0 - h_extra).max0(), ml0, mr0, l),
                (Some(l), Some(w), None) => (w, ml0, mr0, l),
            }
        };
        let mut r = solve(self, spec_w);
        if let Some(m) = max_w
            && r.0 > m
        {
            r = solve(self, Some(m));
        }
        if r.0 < min_w {
            r = solve(self, Some(min_w));
        }
        let (content_w, ml, _mr, left_f) = r;

        // Vertical (CSS2 10.6.4): a box with `top` and `bottom` and an auto height fills the space between.
        let auto_h = replaced_w.is_none() && resolve_dim(&bs.height, Some(ch)).is_none();
        let (mt0, mb0) = (marg[0].unwrap_or(zero), marg[2].unwrap_or(zero));
        let forced_h = match (auto_h, top, bottom) {
            (true, Some(t), Some(b)) => Some((ch - t - b - mt0 - mb0 - v_extra).max0()),
            _ => None,
        };
        let res = self.layout_block(p.id, cw, Some(ch), Flow { root: false, forced_w: Some(content_w), forced_h });
        let h = res.h;
        let (top_f, mt) = match (top, bottom) {
            (Some(t), Some(b)) if !auto_h || forced_h.is_none() => {
                let rest = ch - t - b - h;
                match (marg[0], marg[2]) {
                    (None, None) => (t, rest.half()),
                    (None, Some(mb)) => (t, rest - mb),
                    (Some(mt), _) => (t, mt),
                }
            }
            (Some(t), _) => (t, mt0),
            (None, Some(b)) => (ch - b - mb0 - h - mt0, mt0),
            (None, None) => (sy, mt0),
        };
        let (x_cb, y_cb) = (left_f + ml, top_f + mt);
        if let Some(o) = self.out.get_mut(p.id as usize) {
            o.x = x_cb - ox;
            o.y = y_cb - oy;
        }
        self.oof_list.push((p.owner, p.id));
    }
}
