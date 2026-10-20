//! Gradients as the rasterizer draws them (css-images-3 3, css-images-4 3).
//!
//! [`resolve`] turns a computed [`Gradient`] and the size of its box into a [`GradPaint`]: the geometry (gradient line,
//! ending shape, center), the color stops after the fix-up of css-images-3 3.5.3 and a lookup table of premultiplied
//! colors along the gradient. Colors are interpolated premultiplied in sRGB, the rule for the legacy color syntaxes
//! (CSS Color 4 12.3); every color this engine parses is already sRGB, so that is the only space.

use crate::css::bg::{Extent, GradKind, Gradient, LinearDir, RadSize};
use crate::css::color::Color;

/// Most entries of a lookup table; a gradient longer than this many px gets a coarser table.
const MAX_LUT: usize = 4096;

#[derive(Clone, Debug, PartialEq)]
pub enum Geom {
    /// The gradient line starts at (`x0`, `y0`) and goes the way of the unit vector (`dx`, `dy`); positions are px along it.
    Linear { x0: f32, y0: f32, dx: f32, dy: f32 },
    /// Positions are px along the x radius.
    Radial { cx: f32, cy: f32, rx: f32, ry: f32 },
    /// Positions are turns clockwise from `from` (turns), starting at the top.
    Conic { cx: f32, cy: f32, from: f32 },
}

#[derive(Clone, Debug, PartialEq)]
pub struct GradPaint {
    pub geom: Geom,
    pub repeating: bool,
    /// The position of the first stop and the distance to the last one, in the unit of `geom`.
    pub first: f32,
    pub span: f32,
    /// Premultiplied BGRA at the centers of `lut.len()` equal steps of the span.
    pub lut: Vec<[u8; 4]>,
    /// The colors before the first and after the last stop (the lookup table only covers what is between them).
    pub before: [u8; 4],
    pub after: [u8; 4],
}

impl GradPaint {
    /// The premultiplied BGRA color at (`x`, `y`) in the box of the gradient (CSS px).
    pub fn at(&self, x: f32, y: f32) -> [u8; 4] {
        let s = match self.geom {
            Geom::Linear { x0, y0, dx, dy } => (x - x0) * dx + (y - y0) * dy,
            Geom::Radial { cx, cy, rx, ry } => {
                let (a, b) = ((x - cx) / rx, (y - cy) / ry);
                (a * a + b * b).sqrt() * rx
            }
            Geom::Conic { cx, cy, from } => wrap((x - cx).atan2(cy - y) / std::f32::consts::TAU - from, 1.0),
        };
        self.lookup(s)
    }

    pub fn lookup(&self, s: f32) -> [u8; 4] {
        let n = self.lut.len();
        let t = s - self.first;
        let t = if self.repeating {
            wrap(t, self.span)
        } else if t < 0.0 {
            return self.before;
        } else if t >= self.span {
            return self.after;
        } else {
            t
        };
        let i = ((t * n as f32 / self.span) as usize).min(n.saturating_sub(1));
        self.lut.get(i).copied().unwrap_or([0; 4])
    }
}

type Rgba = [f32; 4];

/// `x` modulo `m` in `[0, m)` (the C library's `fmod` is not worth its bytes here).
fn wrap(x: f32, m: f32) -> f32 {
    x - (x / m).floor() * m
}

/// `t^e` for `0 < t < 1`: `exp2(e * log2(t))` with short polynomials, good to about 1e-3, below one level of an 8-bit
/// channel. (The C library's `powf` alone is 3.7 KB of the executable.)
fn pow_unit(t: f32, e: f32) -> f32 {
    let bits = t.max(1.0e-30).to_bits();
    let ex = ((bits >> 23) & 0xff) as i32 - 127;
    let m = f32::from_bits((bits & 0x007f_ffff) | 0x3f80_0000); // 1 <= m < 2
    // ln(m) = 2 atanh(z), z = (m - 1) / (m + 1) <= 1/3: four terms are good to 1e-5.
    let z = (m - 1.0) / (m + 1.0);
    let z2 = z * z;
    let ln_m = 2.0 * z * (1.0 + z2 * (1.0 / 3.0 + z2 * (0.2 + z2 / 7.0)));
    let l = ex as f32 + ln_m * std::f32::consts::LOG2_E;
    let y = (e * l).clamp(-100.0, 100.0);
    let yi = y.floor();
    let yf = y - yi;
    let p = 1.0 + yf * (std::f32::consts::LN_2 + yf * (0.240_226_5 + yf * (0.055_504_11 + yf * (0.009_618_13 + yf * 0.001_333_36))));
    p * f32::from_bits(((yi as i32 + 127).clamp(1, 254) as u32) << 23)
}

fn premul(c: Color) -> Rgba {
    let a = f32::from(c.a) / 255.0;
    [f32::from(c.r) / 255.0 * a, f32::from(c.g) / 255.0 * a, f32::from(c.b) / 255.0 * a, a]
}

/// A stop after the fix-up: position, color and the relative position of the color hint that follows it (0: none).
#[derive(Clone, Copy)]
struct Fixed {
    pos: f32,
    color: Rgba,
    hint: f32,
}

fn color_at(stops: &[Fixed], s: f32) -> Rgba {
    let (Some(first), Some(last)) = (stops.first(), stops.last()) else { return [0.0; 4] };
    if s <= first.pos {
        return first.color;
    }
    if s >= last.pos {
        return last.color;
    }
    let i = stops.partition_point(|st| st.pos <= s);
    let (Some(a), Some(b)) = (i.checked_sub(1).and_then(|k| stops.get(k)), stops.get(i)) else { return last.color };
    let mut t = (s - a.pos) / (b.pos - a.pos);
    if a.hint > 0.0 {
        // css-images-4 3.5.4: the hint is where the color is halfway.
        t = if a.hint <= 1.0e-6 {
            1.0
        } else if a.hint >= 1.0 - 1.0e-6 {
            0.0
        } else {
            pow_unit(t, -std::f32::consts::LN_2 / a.hint.ln())
        };
    }
    let mut out = [0.0; 4];
    for (o, (x, y)) in out.iter_mut().zip(a.color.iter().zip(b.color.iter())) {
        *o = x + (y - x) * t;
    }
    out
}

/// Resolve `g` for a box of `w` x `h` CSS px. `text` is the color `currentcolor` stands for. `None` for a box without area.
pub fn resolve(g: &Gradient, w: f32, h: f32, text: Color) -> Option<GradPaint> {
    if !(w > 0.0 && h > 0.0 && w.is_finite() && h.is_finite()) {
        return None;
    }
    let (geom, total) = match &g.kind {
        GradKind::Linear(dir) => {
            let (dx, dy) = match *dir {
                LinearDir::Angle(a) => {
                    let r = a.to_radians();
                    (r.sin(), -r.cos())
                }
                // The "magic corners": the 50% line passes through the other two corners (css-images-3 3.1.1).
                LinearDir::To(hx, vy) => {
                    let (x, y) = (f32::from(hx) * h, f32::from(vy) * w);
                    let n = x.hypot(y);
                    if n > 0.0 { (x / n, y / n) } else { (0.0, 1.0) }
                }
            };
            let len = (w * dx).abs() + (h * dy).abs();
            (Geom::Linear { x0: w / 2.0 - dx * len / 2.0, y0: h / 2.0 - dy * len / 2.0, dx, dy }, len)
        }
        GradKind::Radial { circle, size, at } => {
            let (cx, cy) = (at.0.resolve(w, 0.0), at.1.resolve(h, 0.0));
            let (l, r, t, b) = (cx.abs(), (w - cx).abs(), cy.abs(), (h - cy).abs());
            let corners = [l.hypot(t), r.hypot(t), l.hypot(b), r.hypot(b)];
            let (min_c, max_c) = (corners.iter().copied().fold(f32::MAX, f32::min), corners.iter().copied().fold(0.0, f32::max));
            let (rx, ry) = match size {
                RadSize::Lens(a, b) => {
                    if *circle {
                        let r = a.resolve(w);
                        (r, r)
                    } else {
                        (a.resolve(w), b.resolve(h))
                    }
                }
                RadSize::Extent(e) => {
                    let (cs, fs) = ((l.min(r), t.min(b)), (l.max(r), t.max(b)));
                    if *circle {
                        let r = match e {
                            Extent::ClosestSide => cs.0.min(cs.1),
                            Extent::FarthestSide => fs.0.max(fs.1),
                            Extent::ClosestCorner => min_c,
                            Extent::FarthestCorner => max_c,
                        };
                        (r, r)
                    } else {
                        match e {
                            Extent::ClosestSide => cs,
                            Extent::FarthestSide => fs,
                            Extent::ClosestCorner | Extent::FarthestCorner => {
                                let (side, far) = if *e == Extent::ClosestCorner { (cs, false) } else { (fs, true) };
                                // The ellipse of the side ratio that passes through the nearest / farthest corner.
                                let pick = |a: f32, b: f32| if far { a.max(b) } else { a.min(b) };
                                let (cdx, cdy) = (pick(l, r), pick(t, b));
                                let k = ((cdx / side.0.max(1.0e-3)).powi(2) + (cdy / side.1.max(1.0e-3)).powi(2)).sqrt();
                                (side.0 * k, side.1 * k)
                            }
                        }
                    }
                }
            };
            let (rx, ry) = (if rx.is_finite() { rx.max(1.0e-3) } else { 1.0e-3 }, if ry.is_finite() { ry.max(1.0e-3) } else { 1.0e-3 });
            (Geom::Radial { cx, cy, rx, ry }, rx)
        }
        GradKind::Conic { from, at } => (Geom::Conic { cx: at.0.resolve(w, 0.0), cy: at.1.resolve(h, 0.0), from: from / 360.0 }, 1.0),
    };
    // Color stop fix-up (css-images-3 3.5.3): first / last default to the ends, positions never go backwards, runs of
    // stops without a position are spread evenly.
    let mut cols: Vec<(Option<f32>, Rgba, Option<f32>)> = Vec::new(); // (position, color, hint position after it)
    for s in &g.stops {
        let p = s.pos.as_ref().map(|l| l.resolve(total));
        if s.hint {
            if let Some(last) = cols.last_mut() {
                last.2 = p;
            }
        } else {
            cols.push((p, premul(s.color.resolve(text)), None));
        }
    }
    if cols.is_empty() {
        return None;
    }
    if let [only] = cols.as_slice() {
        let only = (only.0.or(Some(0.0)), only.1, None);
        cols = vec![only, only];
        if let Some(l) = cols.last_mut() {
            l.0 = Some(only.0.unwrap_or(0.0).max(total));
        }
    }
    let n = cols.len();
    if let Some(f) = cols.first_mut()
        && f.0.is_none()
    {
        f.0 = Some(0.0);
    }
    if let Some(l) = cols.last_mut()
        && l.0.is_none()
    {
        l.0 = Some(total);
    }
    let mut max = f32::MIN;
    for c in &mut cols {
        if let Some(p) = c.0.as_mut() {
            *p = p.max(max);
            max = *p;
        }
    }
    let mut i = 0;
    while i < n {
        if cols.get(i).is_some_and(|c| c.0.is_none()) {
            let start = i;
            while cols.get(i).is_some_and(|c| c.0.is_none()) {
                i += 1;
            }
            let a = start.checked_sub(1).and_then(|k| cols.get(k)).and_then(|c| c.0).unwrap_or(0.0);
            let b = cols.get(i).and_then(|c| c.0).unwrap_or(a);
            let cnt = (i - start + 1) as f32;
            for (k, c) in cols.iter_mut().enumerate().take(i).skip(start) {
                c.0 = Some(a + (b - a) * (k - start + 1) as f32 / cnt);
            }
        } else {
            i += 1;
        }
    }
    let mut stops: Vec<Fixed> = Vec::with_capacity(n);
    for (k, c) in cols.iter().enumerate() {
        let pos = c.0.unwrap_or(0.0);
        let next = cols.get(k + 1).and_then(|x| x.0).unwrap_or(pos);
        let hint = match c.2 {
            Some(hp) if next > pos => ((hp.clamp(pos, next) - pos) / (next - pos)).clamp(0.0, 1.0),
            _ => 0.0,
        };
        // A hint at relative 0.5 is a plain linear transition.
        let hint = if (hint - 0.5).abs() < 1.0e-6 { 0.0 } else if c.2.is_some() && next > pos { hint.max(1.0e-7) } else { 0.0 };
        stops.push(Fixed { pos, color: c.1, hint });
    }
    let first = stops.first()?.pos;
    let last = stops.last()?.pos;
    // A repeating gradient whose first and last stop are at the same place has no period: it is the last color (css-images-3 3.5.2).
    let flat = g.repeating && last - first < 1.0e-4;
    let span = (last - first).max(1.0e-4);
    let entries = match g.kind {
        GradKind::Conic { .. } => 1024,
        _ => (span * 2.0).ceil() as usize,
    }
    .clamp(2, MAX_LUT);
    let mut lut = Vec::with_capacity(entries);
    for k in 0..entries {
        let s = if last - first < 1.0e-4 {
            if k * 2 < entries { first - 1.0 } else { first + 1.0 }
        } else {
            first + (k as f32 + 0.5) / entries as f32 * span
        };
        let c = color_at(&stops, s);
        let b = |v: f32| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
        lut.push([b(c[2]), b(c[1]), b(c[0]), b(c[3])]);
    }
    let bytes = |c: Rgba| {
        let b = |v: f32| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
        [b(c[2]), b(c[1]), b(c[0]), b(c[3])]
    };
    let (before, after) = (bytes(stops.first()?.color), bytes(stops.last()?.color));
    if flat {
        return Some(GradPaint { geom, repeating: false, first, span, lut: vec![after], before: after, after });
    }
    Some(GradPaint { geom, repeating: g.repeating, first, span, lut, before, after })
}

#[cfg(test)]
#[allow(clippy::indexing_slicing, clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::css::bg::{Pos1, Stop};
    use crate::css::color::ColorValue;
    use crate::css::values::Lp;

    fn stop(c: Color, pos: Option<f32>) -> Stop {
        Stop { color: ColorValue::Color(c), pos: pos.map(Lp::Pct), hint: false }
    }

    fn linear(dir: LinearDir, stops: Vec<Stop>) -> Gradient {
        Gradient { kind: GradKind::Linear(dir), repeating: false, stops }
    }

    #[test]
    fn a_vertical_gradient_goes_from_the_first_to_the_last_color() {
        let g = linear(LinearDir::To(0, 1), vec![stop(Color::rgb(255, 0, 0), None), stop(Color::rgb(0, 0, 255), None)]);
        let p = resolve(&g, 10.0, 100.0, Color::BLACK).unwrap();
        assert!(p.at(5.0, 0.5)[2] >= 253); // red at the top (BGRA: [b, g, r, a])
        let mid = p.at(5.0, 50.0);
        assert!((i32::from(mid[2]) - 128).abs() <= 2 && (i32::from(mid[0]) - 128).abs() <= 2, "{mid:?}");
        assert!(p.at(5.0, 99.5)[0] >= 253);
        assert_eq!(p.at(0.0, 50.0), p.at(9.9, 50.0));
    }

    #[test]
    fn hard_stops_and_repeating_gradients() {
        let (r, b) = (Color::rgb(255, 0, 0), Color::rgb(0, 0, 255));
        let g = linear(LinearDir::Angle(90.0), vec![stop(r, Some(0.0)), stop(r, Some(50.0)), stop(b, Some(50.0)), stop(b, Some(100.0))]);
        let p = resolve(&g, 100.0, 10.0, Color::BLACK).unwrap();
        assert_eq!((p.at(10.0, 5.0)[2], p.at(10.0, 5.0)[0]), (255, 0));
        assert_eq!((p.at(90.0, 5.0)[2], p.at(90.0, 5.0)[0]), (0, 255));
        let mut rep = linear(LinearDir::Angle(90.0), vec![stop(r, Some(0.0)), stop(b, Some(20.0))]);
        rep.repeating = true;
        let p = resolve(&rep, 100.0, 10.0, Color::BLACK).unwrap();
        // 20% of 100 px is one period of 20 px: the color at 5 px and at 25 px is the same.
        let (a, c) = (p.at(5.0, 5.0), p.at(25.0, 5.0));
        assert!((i32::from(a[2]) - i32::from(c[2])).abs() <= 2);
    }

    #[test]
    fn the_hint_power_is_close_to_powf() {
        for (t, e) in [(0.5f32, 2.0f32), (0.1, 0.3), (0.9, 5.0), (0.3, 1.0), (0.77, 0.1), (0.01, 0.5)] {
            assert!((pow_unit(t, e) - t.powf(e)).abs() < 2.0e-3, "{t}^{e}: {} vs {}", pow_unit(t, e), t.powf(e));
        }
    }

    #[test]
    fn a_color_hint_moves_the_midpoint() {
        let (w, k) = (Color::WHITE, Color::BLACK);
        let mut g = linear(LinearDir::Angle(90.0), vec![stop(w, Some(0.0)), Stop { color: ColorValue::CurrentColor, pos: Some(Lp::Pct(25.0)), hint: true }, stop(k, Some(100.0))]);
        let p = resolve(&g, 100.0, 10.0, Color::BLACK).unwrap();
        // The color is halfway (about 128) at the hint, 25 px.
        assert!((i32::from(p.at(25.0, 5.0)[2]) - 128).abs() <= 4, "{:?}", p.at(25.0, 5.0));
        g.stops.remove(1);
        let q = resolve(&g, 100.0, 10.0, Color::BLACK).unwrap();
        assert!((i32::from(q.at(25.0, 5.0)[2]) - 191).abs() <= 4);
    }

    #[test]
    fn radial_and_conic_gradients_follow_their_shape() {
        let (r, b) = (Color::rgb(255, 0, 0), Color::rgb(0, 0, 255));
        let g = Gradient {
            kind: GradKind::Radial { circle: true, size: RadSize::Extent(Extent::FarthestSide), at: (Pos1::CENTER, Pos1::CENTER) },
            repeating: false,
            stops: vec![stop(r, None), stop(b, None)],
        };
        let p = resolve(&g, 100.0, 100.0, Color::BLACK).unwrap();
        assert!(p.at(50.0, 50.0)[2] > 250);
        assert!(p.at(0.5, 50.0)[0] > 250 && p.at(99.5, 50.0)[0] > 250 && p.at(50.0, 0.5)[0] > 250);
        let conic = Gradient { kind: GradKind::Conic { from: 0.0, at: (Pos1::CENTER, Pos1::CENTER) }, repeating: false, stops: vec![stop(r, None), stop(b, None)] };
        let c = resolve(&conic, 100.0, 100.0, Color::BLACK).unwrap();
        // Red just right of the top (angle 0+), blue just left of it (angle 360-), half-way at the bottom.
        assert!(c.at(51.0, 5.0)[2] > 245 && c.at(49.0, 5.0)[0] > 245);
        assert!((i32::from(c.at(50.0, 95.0)[2]) - 128).abs() <= 6);
    }
}
