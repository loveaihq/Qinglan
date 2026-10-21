//! Form control looks (L2 task 2e-2), in a Windows 11 light style: the replaced controls (check boxes, radio buttons, `progress`,
//! `meter`, `input[type=range]`) and the arrow of a `select`. Text fields, buttons and the `select` box are ordinary boxes
//! styled by the UA style sheet (a 1px `#767676` border, small radius, white or `#efefef` fill), so author borders and
//! backgrounds replace them; `appearance: none` also removes what is painted here. The controls cannot be used yet (layer 3).

use super::*;
use crate::css::types::Appearance;

const ACCENT: Color = Color::rgb(0x00, 0x75, 0xff);
const EDGE: Color = Color::rgb(0x76, 0x76, 0x76);
const TRACK: Color = Color::rgb(0xe6, 0xe6, 0xe6);

impl Builder<'_> {
    fn attr_of(&self, node: NodeId, name: &str) -> Option<&str> {
        self.doc.atoms.lookup(name).and_then(|a| self.doc.attr(node, a))
    }

    fn num_attr(&self, node: NodeId, name: &str) -> Option<f32> {
        self.attr_of(node, name).and_then(|v| v.trim().parse::<f32>().ok()).filter(|v| v.is_finite())
    }

    /// `r` filled with `color`, corners rounded by `radius` CSS px.
    fn pill(&mut self, r: Rect, radius: f32, color: Color) {
        let prev = self.push_round(r, [(radius, radius); 4]);
        self.rect(r, color);
        self.restore(prev);
    }

    /// Pixel art on a grid of CSS px with its corner at `(x, y)`: `(dx, dy, width)` runs one px high.
    fn runs(&mut self, x: LU, y: LU, runs: &[(i32, i32, i32)], color: Color) {
        for &(dx, dy, w) in runs {
            self.rect(Rect::new(x + LU::px(dx), y + LU::px(dy), LU::px(w), LU::px(1)), color);
        }
    }

    /// A check box (square, 2px radius) or radio button (round), 13 x 13 CSS px; checked from the `checked` attribute.
    pub(super) fn check(&mut self, node: NodeId, c: Rect, style: &ComputedStyle) {
        if style.misc.appearance == Appearance::None {
            return;
        }
        let radio = self.attr_of(node, "type").is_some_and(|t| t.eq_ignore_ascii_case("radio"));
        let checked = self.attr_of(node, "checked").is_some();
        let inset = |r: Rect, d: i32| Rect::new(r.x + LU::px(d), r.y + LU::px(d), (r.w - LU::px(2 * d)).max0(), (r.h - LU::px(2 * d)).max0());
        let rad = if radio { c.w.to_f32().min(c.h.to_f32()) / 2.0 } else { 2.0 };
        self.pill(c, rad, if checked { ACCENT } else { EDGE });
        self.pill(inset(c, 1), (rad - 1.0).max(0.0), if checked && !radio { ACCENT } else { Color::WHITE });
        if checked {
            if radio {
                self.pill(inset(c, 3), (rad - 3.0).max(0.0), ACCENT);
            } else {
                // The tick: two strokes two px thick.
                self.runs(c.x, c.y, &[(3, 6, 2), (4, 7, 2), (5, 8, 2), (6, 7, 2), (7, 6, 2), (8, 5, 2), (9, 4, 2)], Color::WHITE);
            }
        }
    }

    /// `progress`, `meter` and `input[type=range]`: a rounded track and the part up to the value. False: not one of them.
    pub(super) fn gauge(&mut self, node: NodeId, c: Rect, kind: &str, style: &ComputedStyle) -> bool {
        if style.misc.appearance == Appearance::None {
            return true;
        }
        let (min, max) = (self.num_attr(node, "min"), self.num_attr(node, "max"));
        let value = self.num_attr(node, "value");
        let (h, w) = (c.h.to_f32(), c.w.to_f32());
        match kind {
            "progress" => {
                let top = max.filter(|m| *m > 0.0).unwrap_or(1.0);
                self.pill(c, h / 2.0, TRACK);
                if let Some(v) = value {
                    let prev = self.push_round(c, [(h / 2.0, h / 2.0); 4]);
                    self.rect(Rect::new(c.x, c.y, LU::from_f32(w * (v / top).clamp(0.0, 1.0)), c.h), ACCENT);
                    self.restore(prev);
                }
            }
            "meter" => {
                let lo = min.unwrap_or(0.0);
                let hi = max.unwrap_or(1.0).max(lo);
                let at = |v: f32| v.clamp(lo, hi);
                let (l, hg) = (at(self.num_attr(node, "low").unwrap_or(lo)), at(self.num_attr(node, "high").unwrap_or(hi)));
                let region = |v: f32| -> i32 { if v < l { 0 } else if v > hg { 2 } else { 1 } };
                let v = at(value.unwrap_or(0.0));
                let o = at(self.num_attr(node, "optimum").unwrap_or((lo + hi) / 2.0));
                // css-forms: the same region as the optimum is green, the next one yellow, the far one red.
                let color = match (region(v) - region(o)).abs() {
                    0 => Color::rgb(0x1f, 0xb0, 0x31),
                    1 => Color::rgb(0xff, 0xc4, 0x00),
                    _ => Color::rgb(0xe0, 0x30, 0x30),
                };
                self.pill(c, 4.0, TRACK);
                let f = if hi > lo { (v - lo) / (hi - lo) } else { 0.0 };
                let prev = self.push_round(c, [(4.0, 4.0); 4]);
                self.rect(Rect::new(c.x, c.y, LU::from_f32(w * f), c.h), color);
                self.restore(prev);
            }
            "range" => {
                let (lo, hi) = (min.unwrap_or(0.0), max.unwrap_or(100.0));
                let v = value.unwrap_or((lo + hi) / 2.0);
                let f = if hi > lo { ((v - lo) / (hi - lo)).clamp(0.0, 1.0) } else { 0.0 };
                let mid = c.y + LU::from_f32((h - 4.0) / 2.0);
                let x = LU::from_f32((w - h) * f + h / 2.0);
                self.pill(Rect::new(c.x, mid, c.w, LU::px(4)), 2.0, Color::rgb(0xc4, 0xc4, 0xc4));
                self.pill(Rect::new(c.x, mid, x, LU::px(4)), 2.0, ACCENT);
                self.pill(Rect::new(c.x + x - LU::from_f32(h / 2.0), c.y, c.h, c.h), h / 2.0, ACCENT);
            }
            _ => return false,
        }
        true
    }

    /// The arrow of a `select` over the right of its border box `r`.
    pub(super) fn select_arrow(&mut self, r: Rect) {
        let (x, y) = (r.right() - LU::px(15), r.y + LU::from_f32((r.h.to_f32() - 4.0).max(0.0) / 2.0));
        self.runs(x, y, &[(0, 0, 2), (6, 0, 2), (1, 1, 2), (5, 1, 2), (2, 2, 2), (4, 2, 2), (3, 3, 2)], Color::rgb(0x40, 0x40, 0x40));
    }
}
