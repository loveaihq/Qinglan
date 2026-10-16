//! Replaced elements: images, frames and other placeholder boxes, check boxes (CSS2 10.3.2, 10.3.4,
//! 10.6.2, 10.4 "constraint violation" table; HTML 15.4 and 15.5).
//!
//! Images use their natural size (decoded pixels / density of the chosen `srcset` candidate) when the
//! host has reported it; before that, the `width` / `height` attributes give the box (and its aspect
//! ratio) and an image with no size at all is 0x0. Frames and plug-ins are 300x150 like any replaced
//! element without an intrinsic size (CSS2 10.3.2).

use super::block::resolve_dim;
use super::boxtree::{BoxId, ReplacedKind, parse_html_int};
use super::image::ImageState;
use super::units::LayoutUnit as LU;
use super::Layouter;
use crate::css::types::BoxSizing;

/// What is known about the size of a replaced element before CSS is applied.
struct Natural {
    w: Option<f64>,
    h: Option<f64>,
    ratio: Option<f64>,
    default_w: f64,
    default_h: f64,
}

fn ratio_of(w: f64, h: f64) -> Option<f64> {
    (w > 0.0 && h > 0.0).then(|| w / h)
}

/// CSS2 10.4: the used size of a replaced element with an intrinsic size and ratio when both
/// `width` and `height` are `auto`, so that min/max constraints keep the ratio.
fn constrain(w: f64, h: f64, min_w: f64, max_w: f64, min_h: f64, max_h: f64) -> (f64, f64) {
    if w <= 0.0 || h <= 0.0 {
        return (w.clamp(min_w, max_w.max(min_w)), h.clamp(min_h, max_h.max(min_h)));
    }
    let (over_w, under_w, over_h, under_h) = (w > max_w, w < min_w, h > max_h, h < min_h);
    match (over_w, under_w, over_h, under_h) {
        (false, false, false, false) => (w, h),
        (true, _, true, _) => {
            if max_w / w <= max_h / h {
                (max_w, min_h.max(max_w * h / w))
            } else {
                (min_w.max(max_h * w / h), max_h)
            }
        }
        (_, true, _, true) => {
            if min_w / w <= min_h / h {
                (max_w.min(min_h * w / h), min_h)
            } else {
                (min_w, max_h.min(min_w * h / w))
            }
        }
        (_, true, true, _) => (min_w, max_h),
        (true, _, _, true) => (max_w, min_h),
        (true, ..) => (max_w, min_h.max(max_w * h / w)),
        (_, true, ..) => (min_w, max_h.min(min_w * h / w)),
        (_, _, true, _) => (min_w.max(max_h * w / h), max_h),
        _ => (max_w.min(min_h * w / h), min_h),
    }
}

impl Layouter<'_> {
    fn natural(&self, id: BoxId) -> Natural {
        let bx = self.tree.get(id);
        let attr = |name: &str| self.doc.atoms.lookup(name).and_then(|a| self.doc.attr(bx.node, a)).and_then(parse_html_int);
        let mut n = Natural { w: None, h: None, ratio: None, default_w: 300.0, default_h: 150.0 };
        match bx.replaced() {
            Some(ReplacedKind::Image) => {
                n.default_w = 0.0;
                n.default_h = 0.0;
                match (self.images.get(bx.node), bx.image()) {
                    (Some(ImageState::Loaded { width, height }), Some(src)) => {
                        let d = (src.density as f64).max(0.0001);
                        let (w, h) = (width as f64 / d, height as f64 / d);
                        n.w = Some(w);
                        n.h = Some(h);
                        n.ratio = ratio_of(w, h);
                    }
                    // Not there yet: the attributes' ratio reserves the space (HTML 15.4.3).
                    _ => {
                        if let (Some(w), Some(h)) = (attr("width"), attr("height")) {
                            n.ratio = ratio_of(w as f64, h as f64);
                        }
                    }
                }
            }
            Some(ReplacedKind::Placeholder(kind)) => match kind {
                "canvas" => {
                    let w = attr("width").filter(|v| *v >= 0).map_or(300.0, f64::from);
                    let h = attr("height").filter(|v| *v >= 0).map_or(150.0, f64::from);
                    n.w = Some(w);
                    n.h = Some(h);
                    n.ratio = ratio_of(w, h);
                }
                "audio" => n.default_h = 54.0,
                _ => {}
            },
            Some(ReplacedKind::Check) => {
                n.w = Some(13.0);
                n.h = Some(13.0);
                n.ratio = Some(1.0);
            }
            Some(ReplacedKind::Fixed(w, h, _)) => {
                n.w = Some(w as f64);
                n.h = Some(h as f64);
                n.ratio = ratio_of(w as f64, h as f64);
            }
            None => {}
        }
        n
    }

    /// The content-box size of replaced element `id` in a containing block `cb_w` wide (`cb_h` high
    /// when definite).
    pub(crate) fn replaced_size(&mut self, id: BoxId, cb_w: LU, cb_h: Option<LU>) -> (LU, LU) {
        self.replaced_size_of(id, cb_w, cb_h, false)
    }

    /// The size the box is laid out with: like [`Self::replaced_size`], and the way it was reached is kept for
    /// [`ImageFit`].
    pub(crate) fn replaced_size_final(&mut self, id: BoxId, cb_w: LU, cb_h: Option<LU>) -> (LU, LU) {
        self.replaced_size_of(id, cb_w, cb_h, true)
    }

    fn replaced_size_of(&mut self, id: BoxId, cb_w: LU, cb_h: Option<LU>, record: bool) -> (LU, LU) {
        let bx = self.tree.get(id);
        let bs = &bx.style.boxs;
        let bd = &bx.style.border;
        let pad = |i: usize| bs.padding.get(i).map_or(LU::ZERO, |l| super::block::pad_px(l, cb_w));
        let bor = |i: usize| LU::from_f32(bd.width.get(i).copied().unwrap_or(0.0)).max0();
        let (h_extra, v_extra) = (pad(1) + pad(3) + bor(1) + bor(3), pad(0) + pad(2) + bor(0) + bor(2));
        let border_box = bs.box_sizing == BoxSizing::BorderBox;
        let to_cw = |v: LU| if border_box { (v - h_extra).max0() } else { v };
        let to_ch = |v: LU| if border_box { (v - v_extra).max0() } else { v };
        let px = |v: LU| v.to_f32() as f64;
        let spec_w = resolve_dim(&bs.width, Some(cb_w)).map(|v| px(to_cw(v)));
        let spec_h = resolve_dim(&bs.height, cb_h).map(|v| px(to_ch(v)));
        let min_w = resolve_dim(&bs.min_width, Some(cb_w)).map_or(0.0, |v| px(to_cw(v)));
        let max_w = resolve_dim(&bs.max_width, Some(cb_w)).map_or(f64::INFINITY, |v| px(to_cw(v)));
        let min_h = resolve_dim(&bs.min_height, cb_h).map_or(0.0, |v| px(to_ch(v)));
        let max_h = resolve_dim(&bs.max_height, cb_h).map_or(f64::INFINITY, |v| px(to_ch(v)));
        let nat = self.natural(id);
        let (w, h) = solve(spec_w, spec_h, (min_w, max_w, min_h, max_h), &nat);
        let used = (LU::from_f32(w as f32).max0(), LU::from_f32(h as f32).max0());
        if record && let Some(src) = bx.image() {
            let fit = ImageFit { spec_w: spec_w.map(|v| v as f32), spec_h: spec_h.map(|v| v as f32), limits: [min_w as f32, max_w as f32, min_h as f32, max_h as f32], density: src.density, used };
            self.fits.insert(bx.node, fit);
        }
        used
    }
}

/// CSS2 10.3.2 / 10.6.2 for a replaced element: the content size from the specified sizes (already resolved, `None`
/// for `auto`), the `(min-width, max-width, min-height, max-height)` and what is known of the element itself.
fn solve(spec_w: Option<f64>, spec_h: Option<f64>, limits: (f64, f64, f64, f64), nat: &Natural) -> (f64, f64) {
    let (min_w, max_w, min_h, max_h) = limits;
    let cw = |v: f64| v.clamp(min_w, max_w.max(min_w));
    let ch = |v: f64| v.clamp(min_h, max_h.max(min_h));
    match (spec_w, spec_h) {
        (Some(w), Some(h)) => (cw(w), ch(h)),
        (Some(w), None) => {
            let w = cw(w);
            (w, ch(nat.ratio.map_or_else(|| nat.h.unwrap_or(nat.default_h), |r| w / r)))
        }
        (None, Some(h)) => {
            let h = ch(h);
            (cw(nat.ratio.map_or_else(|| nat.w.unwrap_or(nat.default_w), |r| h * r)), h)
        }
        (None, None) => match (nat.w, nat.h) {
            (Some(w), Some(h)) => constrain(w, h, min_w, max_w, min_h, max_h),
            (Some(w), None) => {
                let w = cw(w);
                (w, ch(nat.ratio.map_or(nat.default_h, |r| w / r)))
            }
            (None, Some(h)) => {
                let h = ch(h);
                (cw(nat.ratio.map_or(nat.default_w, |r| h * r)), h)
            }
            (None, None) => match nat.ratio {
                Some(r) => {
                    let w = cw(nat.default_w.max(300.0));
                    (w, ch(w / r))
                }
                None => (cw(nat.default_w), ch(nat.default_h)),
            },
        },
    }
}

/// How the size of an image box was reached in the last layout: the inputs that do not depend on what the image turns
/// out to be, and the result. The window asks [`ImageFit::keeps_size`] when an image's header arrives: a box whose
/// width is given and whose size comes out the same (the height within [`HEIGHT_SLACK`]) needs no new layout, only a
/// repaint.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImageFit {
    spec_w: Option<f32>,
    spec_h: Option<f32>,
    /// `min-width`, `max-width`, `min-height`, `max-height` (content box, px).
    limits: [f32; 4],
    /// Image pixels per CSS px of the chosen source.
    density: f32,
    /// The content size the layout gave the box.
    used: (LU, LU),
}

/// A height that follows from the width and the picture's ratio may differ this much (layout units, 1 px) from what the
/// `width` and `height` attributes reserved: the attributes are whole numbers (a thumbnail 250 x 180 shown 180 wide has
/// `height="130"`, the ratio gives 129.6), so a picture almost always leaves a fraction of a pixel. A new layout of the
/// whole page for that would cost far more than the page can show; the box keeps the height it has until the next layout.
pub const HEIGHT_SLACK: i32 = 64;

impl ImageFit {
    /// Would an image of `width` x `height` pixels leave the box the size it has (the width exactly, the height to
    /// within [`HEIGHT_SLACK`])? Only when the `width` is specified (otherwise the box's width, and so the widths of
    /// what contains it, follow the picture).
    pub fn keeps_size(&self, width: u32, height: u32) -> bool {
        if self.spec_w.is_none() {
            return false;
        }
        let d = f64::from(self.density).max(0.0001);
        let (w, h) = (f64::from(width) / d, f64::from(height) / d);
        let nat = Natural { w: Some(w), h: Some(h), ratio: ratio_of(w, h), default_w: 0.0, default_h: 0.0 };
        let [a, b, c, e] = self.limits.map(f64::from);
        let (w, h) = solve(self.spec_w.map(f64::from), self.spec_h.map(f64::from), (a, b, c, e), &nat);
        let (w, h) = (LU::from_f32(w as f32).max0(), LU::from_f32(h as f32).max0());
        w == self.used.0 && (i64::from(h.raw()) - i64::from(self.used.1.raw())).abs() <= i64::from(HEIGHT_SLACK)
    }
}

#[cfg(test)]
mod tests {
    use super::{ImageFit, constrain};
    use crate::layout::units::LayoutUnit as LU;

    #[test]
    fn constraint_violation_table() {
        // 200x100 with max-width 100: width limited, height keeps the ratio.
        assert_eq!(constrain(200.0, 100.0, 0.0, 100.0, 0.0, f64::INFINITY), (100.0, 50.0));
        // Both too large: the tighter constraint wins (max-width/w = 0.5 <= max-height/h = 0.8).
        assert_eq!(constrain(200.0, 100.0, 0.0, 100.0, 0.0, 80.0), (100.0, 50.0));
        // Too small in both: min-width/w = 2 <= min-height/h = 3, so width = min-height * w / h.
        assert_eq!(constrain(10.0, 10.0, 20.0, f64::INFINITY, 30.0, f64::INFINITY), (30.0, 30.0));
    }

    fn fit(spec_w: Option<f32>, spec_h: Option<f32>, used: (f32, f32)) -> ImageFit {
        ImageFit { spec_w, spec_h, limits: [0.0, f32::INFINITY, 0.0, f32::INFINITY], density: 1.0, used: (LU::from_f32(used.0), LU::from_f32(used.1)) }
    }

    #[test]
    fn a_box_with_a_given_width_keeps_its_size_when_the_picture_fits_the_ratio() {
        // `width="180" height="130"` with `height: auto`: the box is 180 x 130 before the picture is known.
        let f = fit(Some(180.0), None, (180.0, 130.0));
        // A 250 x 180 thumbnail gives 180 x 129.6: within the slack, no new layout.
        assert!(f.keeps_size(250, 180));
        // The ratio of the attributes itself.
        assert!(f.keeps_size(180, 130));
        // A picture of another shape changes the height by more than a pixel.
        assert!(!f.keeps_size(250, 100));
        assert!(!f.keeps_size(180, 180));
    }

    #[test]
    fn a_given_width_and_height_never_depend_on_the_picture() {
        let f = fit(Some(100.0), Some(40.0), (100.0, 40.0));
        assert!(f.keeps_size(1, 1));
        assert!(f.keeps_size(4000, 3000));
    }

    #[test]
    fn a_width_that_follows_the_picture_always_needs_a_layout() {
        assert!(!fit(None, Some(40.0), (100.0, 40.0)).keeps_size(100, 40));
        assert!(!fit(None, None, (0.0, 0.0)).keeps_size(0, 0));
    }
}
