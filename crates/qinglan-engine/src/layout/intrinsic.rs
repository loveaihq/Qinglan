//! Intrinsic sizes (css-sizing-3 4, CSS2 10.3.5): min-content and max-content widths of boxes, the
//! `min-content` / `max-content` / `fit-content` keywords and the shrink-to-fit width of atomic
//! inline-level boxes.

use super::block::{lp_has_percent, pad_px, resolve_dim, resolve_margin};
use super::boxtree::{BoxId, BoxKind, Control};
use super::units::LayoutUnit as LU;
use super::Layouter;
use crate::css::types::{BoxSizing, Dim};

fn fixed_len(d: &Dim) -> Option<LU> {
    match d {
        Dim::Lp(l) if !lp_has_percent(l) => Some(LU::from_f32(l.resolve(0.0))),
        _ => None,
    }
}

impl Layouter<'_> {
    /// The value of an intrinsic-size keyword for box `id`, or `None` if `d` is not one. `avail` is
    /// the available content width (for `fit-content`).
    pub(crate) fn kw_size(&mut self, id: BoxId, d: &Dim, avail: LU) -> Option<LU> {
        if !matches!(d, Dim::MinContent | Dim::MaxContent | Dim::FitContent) {
            return None;
        }
        let (min, max) = self.content_widths(id);
        Some(match d {
            Dim::MinContent => min,
            Dim::MaxContent => max,
            _ => min.max(avail).min(max),
        })
    }

    /// Min-content and max-content width of the content box of `id` (memoised).
    pub(crate) fn content_widths(&mut self, id: BoxId) -> (LU, LU) {
        if let Some(w) = self.widths.get(&id) {
            return *w;
        }
        if !self.charge(2) {
            return (LU::ZERO, LU::ZERO);
        }
        let tree = self.tree;
        let bx = tree.get(id);
        let r = match bx.control() {
            Control::TextInput(n) | Control::TextArea(n, _) => {
                let rf = self.font_for(&bx.style);
                let w = LU::from_raw(rf.zero_adv.raw() as i64 * n as i64);
                (w, w)
            }
            _ if bx.kind == BoxKind::Replaced => {
                let (w, _) = self.replaced_size(id, LU::ZERO, None);
                (w, w)
            }
            _ if bx.inline_children => self.measure_inline(id),
            _ => {
                let (mut min, mut max) = (LU::ZERO, LU::ZERO);
                for &c in &bx.children {
                    if self.exhausted {
                        break;
                    }
                    let (a, b) = self.outer_widths(c);
                    min = min.max(a);
                    max = max.max(b);
                }
                (min, max)
            }
        };
        self.widths.insert(id, r);
        r
    }

    /// Min-content and max-content contribution of box `id` to its container: the margin box, with a
    /// fixed `width` (and `min-width` / `max-width`) honoured. Percentages count as `auto`.
    pub(crate) fn outer_widths(&mut self, id: BoxId) -> (LU, LU) {
        let tree = self.tree;
        let bx = tree.get(id);
        let bs = &bx.style.boxs;
        let bd = &bx.style.border;
        let edge = |d: &Dim| match d {
            Dim::Lp(l) if !lp_has_percent(l) => LU::from_f32(l.resolve(0.0)),
            _ => LU::ZERO,
        };
        let pad = |i: usize| bs.padding.get(i).map_or(LU::ZERO, |l| if lp_has_percent(l) { LU::ZERO } else { pad_px(l, LU::ZERO) });
        let bor = |i: usize| super::block::border_px(bd.width.get(i).copied().unwrap_or(0.0));
        let h_extra = pad(1) + pad(3) + bor(1) + bor(3);
        let margins = edge(&bs.margin[1]) + edge(&bs.margin[3]);
        let to_cw = |v: LU| if bs.box_sizing == BoxSizing::BorderBox { (v - h_extra).max0() } else { v };
        let (mut min, mut max) = match fixed_len(&bs.width) {
            Some(w) => (to_cw(w), to_cw(w)),
            None => self.content_widths(id),
        };
        if let Some(mx) = fixed_len(&bs.max_width).map(to_cw) {
            min = min.min(mx);
            max = max.min(mx);
        }
        if let Some(mn) = fixed_len(&bs.min_width).map(to_cw) {
            min = min.max(mn);
            max = max.max(mn);
        }
        (min + h_extra + margins, max + h_extra + margins)
    }

    /// The used content width of an atomic inline-level box in a line of a block `cb_w` wide:
    /// `width` if given, else the intrinsic width of a text field, else shrink-to-fit (CSS2 10.3.9).
    pub(crate) fn atomic_width(&mut self, id: BoxId, cb_w: LU) -> LU {
        let tree = self.tree;
        let bx = tree.get(id);
        let bs = &bx.style.boxs;
        let bd = &bx.style.border;
        let pad = |i: usize| bs.padding.get(i).map_or(LU::ZERO, |l| pad_px(l, cb_w));
        let bor = |i: usize| super::block::border_px(bd.width.get(i).copied().unwrap_or(0.0));
        let h_extra = pad(1) + pad(3) + bor(1) + bor(3);
        let mrg = |i: usize| bs.margin.get(i).and_then(|d| resolve_margin(d, cb_w)).unwrap_or(LU::ZERO);
        let avail = (cb_w - mrg(1) - mrg(3) - h_extra).max0();
        let to_cw = |v: LU| if bs.box_sizing == BoxSizing::BorderBox { (v - h_extra).max0() } else { v };
        let spec = match self.kw_size(id, &bs.width, avail) {
            Some(v) => Some(v),
            None => resolve_dim(&bs.width, Some(cb_w)).map(to_cw),
        };
        let mut w = match spec {
            Some(w) => w,
            None => {
                let (min, max) = self.content_widths(id);
                if matches!(bx.control(), Control::TextInput(_) | Control::TextArea(..)) { min } else { min.max(avail).min(max) }
            }
        };
        let max_w = match self.kw_size(id, &bs.max_width, avail) {
            Some(v) => Some(v),
            None => resolve_dim(&bs.max_width, Some(cb_w)).map(to_cw),
        };
        let min_w = match self.kw_size(id, &bs.min_width, avail) {
            Some(v) => v,
            None => resolve_dim(&bs.min_width, Some(cb_w)).map_or(LU::ZERO, to_cw),
        };
        if let Some(m) = max_w {
            w = w.min(m);
        }
        w.max(min_w)
    }
}
