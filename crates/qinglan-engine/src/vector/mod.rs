//! Vector graphics (L2 step 2c): paths, an exact-coverage scanline rasterizer, a stroker and coverage masks.
//!
//! * [`Path`] is a list of move/line/quadratic/cubic/close segments; [`Path::flatten`] turns it into polygons
//!   within a tolerance (in the units of the path, usually device pixels). A [`Transform`] maps a path to device space
//!   ([`Path::transformed`]); this is the hook that 2c-2 (CSS transforms) and 2e (SVG) use.
//! * [`fill_polys`] rasterizes polygons into 8-bit coverage rows with signed-area accumulation, as font rasterizers do
//!   (stb_truetype v2, font-rs): each edge deposits the exact area it covers in each pixel it crosses and a running sum
//!   along the row gives the coverage. Non-zero and even-odd rules; memory is bounded by working in bands of rows.
//! * [`stroke_polys`] turns polygons into the outline of their stroke (width, caps, joins, miter limit, dashes); fill the
//!   result with the non-zero rule.
//! * [`Mask`] is a bounded coverage mask (clip paths, border sides).
//!
//! Everything here treats its input as hostile: non-finite numbers are skipped, sizes are capped, nothing panics.

pub mod fill;
pub mod stroke;

pub use fill::{FillRule, Mask, fill_polys};
pub use stroke::{Cap, Join, Stroke, stroke_polys};

/// Flattening produces at most this many points per path; the rest of a hostile path is dropped.
pub const MAX_POINTS: usize = 1 << 20;
/// A curve is cut into at most this many lines.
const MAX_CURVE_LINES: usize = 128;
/// The control-point distance of a quarter circle drawn with a cubic Bézier.
pub const KAPPA: f32 = 0.552_284_8;

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Pt {
    pub x: f32,
    pub y: f32,
}

impl Pt {
    pub const fn new(x: f32, y: f32) -> Pt {
        Pt { x, y }
    }
    fn lerp(self, o: Pt, t: f32) -> Pt {
        Pt::new(self.x + (o.x - self.x) * t, self.y + (o.y - self.y) * t)
    }
    fn finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

/// An integer rectangle `x0..x1` x `y0..y1` (pixels).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IRect {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

impl IRect {
    pub fn intersect(&self, o: &IRect) -> IRect {
        IRect { x0: self.x0.max(o.x0), y0: self.y0.max(o.y0), x1: self.x1.min(o.x1), y1: self.y1.min(o.y1) }
    }
    pub fn is_empty(&self) -> bool {
        self.x1 <= self.x0 || self.y1 <= self.y0
    }
    pub fn width(&self) -> i32 {
        (self.x1 - self.x0).max(0)
    }
    pub fn height(&self) -> i32 {
        (self.y1 - self.y0).max(0)
    }
}

/// An affine transform: `x' = a x + c y + e`, `y' = b x + d y + f` (the CSS `matrix(a, b, c, d, e, f)`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
    pub e: f32,
    pub f: f32,
}

impl Transform {
    pub const IDENTITY: Transform = Transform { a: 1.0, b: 0.0, c: 0.0, d: 1.0, e: 0.0, f: 0.0 };

    pub fn translate(x: f32, y: f32) -> Transform {
        Transform { e: x, f: y, ..Transform::IDENTITY }
    }
    pub fn scale(x: f32, y: f32) -> Transform {
        Transform { a: x, d: y, ..Transform::IDENTITY }
    }
    pub fn apply(&self, p: Pt) -> Pt {
        Pt::new(self.a * p.x + self.c * p.y + self.e, self.b * p.x + self.d * p.y + self.f)
    }
    /// The transform that applies `inner` first and then `self`.
    pub fn compose(&self, inner: &Transform) -> Transform {
        Transform {
            a: self.a * inner.a + self.c * inner.b,
            b: self.b * inner.a + self.d * inner.b,
            c: self.a * inner.c + self.c * inner.d,
            d: self.b * inner.c + self.d * inner.d,
            e: self.a * inner.e + self.c * inner.f + self.e,
            f: self.b * inner.e + self.d * inner.f + self.f,
        }
    }
    /// The inverse; every field is NaN for a transform that cannot be inverted (nothing maps back through it).
    pub fn inverse(&self) -> Transform {
        let det = self.a * self.d - self.b * self.c;
        if det.is_nan() || det.abs() <= 1.0e-12 {
            return Transform { a: f32::NAN, b: f32::NAN, c: f32::NAN, d: f32::NAN, e: f32::NAN, f: f32::NAN };
        }
        let (a, b, c, d) = (self.d / det, -self.b / det, -self.c / det, self.a / det);
        Transform { a, b, c, d, e: -(a * self.e + c * self.f), f: -(b * self.e + d * self.f) }
    }
    /// The square root of the area scale: how much lengths grow on average.
    pub fn scale_factor(&self) -> f32 {
        (self.a * self.d - self.b * self.c).abs().sqrt()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Seg {
    Move(Pt),
    Line(Pt),
    /// Control point, end point.
    Quad(Pt, Pt),
    /// Two control points, end point.
    Cubic(Pt, Pt, Pt),
    Close,
}

/// One flattened subpath.
#[derive(Clone, Debug, PartialEq)]
pub struct Poly {
    pub pts: Vec<Pt>,
    pub closed: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Path {
    pub segs: Vec<Seg>,
}

impl Path {
    pub fn new() -> Path {
        Path::default()
    }
    pub fn is_empty(&self) -> bool {
        self.segs.is_empty()
    }
    pub fn move_to(&mut self, x: f32, y: f32) {
        self.segs.push(Seg::Move(Pt::new(x, y)));
    }
    pub fn line_to(&mut self, x: f32, y: f32) {
        self.segs.push(Seg::Line(Pt::new(x, y)));
    }
    pub fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        self.segs.push(Seg::Quad(Pt::new(cx, cy), Pt::new(x, y)));
    }
    pub fn cubic_to(&mut self, c1x: f32, c1y: f32, c2x: f32, c2y: f32, x: f32, y: f32) {
        self.segs.push(Seg::Cubic(Pt::new(c1x, c1y), Pt::new(c2x, c2y), Pt::new(x, y)));
    }
    pub fn close(&mut self) {
        self.segs.push(Seg::Close);
    }

    pub fn rect(x: f32, y: f32, w: f32, h: f32) -> Path {
        let mut p = Path::new();
        p.move_to(x, y);
        p.line_to(x + w, y);
        p.line_to(x + w, y + h);
        p.line_to(x, y + h);
        p.close();
        p
    }

    /// A rectangle with elliptical corners `(rx, ry)` in the order top-left, top-right, bottom-right, bottom-left.
    /// The radii must fit (their sums must not exceed the sides); CSS scales them first (css-backgrounds-3 5.5).
    pub fn rounded_rect(x: f32, y: f32, w: f32, h: f32, radii: [(f32, f32); 4]) -> Path {
        let [tl, tr, br, bl] = radii;
        let k = KAPPA;
        let mut p = Path::new();
        p.move_to(x + tl.0, y);
        p.line_to(x + w - tr.0, y);
        p.cubic_to(x + w - tr.0 + k * tr.0, y, x + w, y + tr.1 - k * tr.1, x + w, y + tr.1);
        p.line_to(x + w, y + h - br.1);
        p.cubic_to(x + w, y + h - br.1 + k * br.1, x + w - br.0 + k * br.0, y + h, x + w - br.0, y + h);
        p.line_to(x + bl.0, y + h);
        p.cubic_to(x + bl.0 - k * bl.0, y + h, x, y + h - bl.1 + k * bl.1, x, y + h - bl.1);
        p.line_to(x, y + tl.1);
        p.cubic_to(x, y + tl.1 - k * tl.1, x + tl.0 - k * tl.0, y, x + tl.0, y);
        p.close();
        p
    }

    pub fn ellipse(cx: f32, cy: f32, rx: f32, ry: f32) -> Path {
        Path::rounded_rect(cx - rx, cy - ry, rx * 2.0, ry * 2.0, [(rx, ry); 4])
    }

    /// The path with every point mapped through `t`.
    pub fn transformed(&self, t: &Transform) -> Path {
        let m = |p: &Pt| t.apply(*p);
        Path {
            segs: self
                .segs
                .iter()
                .map(|s| match s {
                    Seg::Move(p) => Seg::Move(m(p)),
                    Seg::Line(p) => Seg::Line(m(p)),
                    Seg::Quad(c, p) => Seg::Quad(m(c), m(p)),
                    Seg::Cubic(a, b, p) => Seg::Cubic(m(a), m(b), m(p)),
                    Seg::Close => Seg::Close,
                })
                .collect(),
        }
    }

    /// Cut the curves into lines that stay within `tol` of the curve. Subpaths with a single point and no `Close` are dropped.
    pub fn flatten(&self, tol: f32) -> Vec<Poly> {
        let tol = if tol.is_finite() { tol.max(1.0e-3) } else { 0.1 };
        let mut out: Vec<Poly> = Vec::new();
        let mut cur: Vec<Pt> = Vec::new();
        let (mut start, mut at) = (Pt::default(), Pt::default());
        let mut total = 0usize;
        for seg in &self.segs {
            if total > MAX_POINTS {
                break;
            }
            match *seg {
                Seg::Move(p) => {
                    finish(&mut out, &mut cur, false);
                    if p.finite() {
                        cur.push(p);
                        start = p;
                        at = p;
                    }
                }
                Seg::Line(p) => {
                    if !p.finite() {
                        continue;
                    }
                    if cur.is_empty() {
                        cur.push(at);
                    }
                    cur.push(p);
                    at = p;
                    total += 1;
                }
                Seg::Quad(c, p) => {
                    if !(c.finite() && p.finite()) {
                        continue;
                    }
                    if cur.is_empty() {
                        cur.push(at);
                    }
                    let dd = Pt::new(at.x - 2.0 * c.x + p.x, at.y - 2.0 * c.y + p.y);
                    let n = curve_lines(0.25 * dd.x.hypot(dd.y), tol);
                    for i in 1..=n {
                        let t = i as f32 / n as f32;
                        cur.push(at.lerp(c, t).lerp(c.lerp(p, t), t));
                    }
                    at = p;
                    total += n;
                }
                Seg::Cubic(c1, c2, p) => {
                    if !(c1.finite() && c2.finite() && p.finite()) {
                        continue;
                    }
                    if cur.is_empty() {
                        cur.push(at);
                    }
                    let d1 = (at.x - 2.0 * c1.x + c2.x).hypot(at.y - 2.0 * c1.y + c2.y);
                    let d2 = (c1.x - 2.0 * c2.x + p.x).hypot(c1.y - 2.0 * c2.y + p.y);
                    let n = curve_lines(0.75 * d1.max(d2), tol);
                    for i in 1..=n {
                        let t = i as f32 / n as f32;
                        let (a, b, c) = (at.lerp(c1, t), c1.lerp(c2, t), c2.lerp(p, t));
                        cur.push(a.lerp(b, t).lerp(b.lerp(c, t), t));
                    }
                    at = p;
                    total += n;
                }
                Seg::Close => {
                    finish(&mut out, &mut cur, true);
                    at = start;
                }
            }
        }
        finish(&mut out, &mut cur, false);
        out
    }

    /// The coverage mask of the filled path inside `win` (None when it would be too big).
    pub fn fill_mask(&self, rule: FillRule, win: IRect, tol: f32) -> Option<Mask> {
        Mask::from_polys(&self.flatten(tol), rule, win)
    }
}

/// Number of lines for a curve whose error bound numerator is `err` (error <= err / n^2).
fn curve_lines(err: f32, tol: f32) -> usize {
    let n = (err / tol).sqrt().ceil();
    if n.is_finite() { (n as usize).clamp(1, MAX_CURVE_LINES) } else { 1 }
}

fn finish(out: &mut Vec<Poly>, cur: &mut Vec<Pt>, closed: bool) {
    let pts = std::mem::take(cur);
    if pts.len() >= 2 || (closed && !pts.is_empty()) {
        out.push(Poly { pts, closed });
    }
}

#[cfg(test)]
#[allow(clippy::indexing_slicing, clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    const WIN: IRect = IRect { x0: -100, y0: -100, x1: 400, y1: 400 };

    fn total(path: &Path, rule: FillRule) -> f32 {
        let m = path.fill_mask(rule, WIN, 0.01).unwrap();
        m.data.iter().map(|&v| f32::from(v)).sum::<f32>() / 255.0
    }

    #[test]
    fn a_rectangle_covers_the_exact_area_of_each_pixel() {
        // x 0.5..2.5, y 0.5..1.5: the corner pixel is a quarter covered, the edge pixels half, the middle one fully.
        let m = Path::rect(0.5, 0.5, 2.0, 1.0).fill_mask(FillRule::NonZero, WIN, 0.1).unwrap();
        assert_eq!(m.at(0, 0), 64);
        assert_eq!(m.at(1, 0), 128);
        assert_eq!(m.at(1, 1), 128);
        assert_eq!(m.at(2, 1), 64);
        assert_eq!(m.at(3, 0), 0);
        assert_eq!(m.at(0, 2), 0);
    }

    #[test]
    fn a_circle_has_the_area_of_a_circle() {
        let a = total(&Path::ellipse(50.0, 50.0, 20.0, 20.0), FillRule::NonZero);
        assert!((a - 1256.64).abs() < 1.5, "{a}");
        let e = total(&Path::ellipse(50.3, 50.7, 30.0, 10.0), FillRule::NonZero);
        assert!((e - 942.48).abs() < 1.5, "{e}");
    }

    #[test]
    fn even_odd_leaves_a_hole_where_non_zero_fills_it() {
        let mut p = Path::rect(0.0, 0.0, 10.0, 10.0);
        let inner = Path::rect(2.0, 2.0, 6.0, 6.0); // same direction as the outer one
        p.segs.extend(inner.segs);
        assert!((total(&p, FillRule::EvenOdd) - 64.0).abs() < 0.01);
        assert!((total(&p, FillRule::NonZero) - 100.0).abs() < 0.01);
    }

    #[test]
    fn a_curve_stays_within_the_tolerance() {
        let mut p = Path::new();
        p.move_to(0.0, 0.0);
        p.cubic_to(0.0, 100.0, 100.0, 100.0, 100.0, 0.0);
        let polys = p.flatten(0.05);
        // The curve's middle point (t = 0.5) is (50, 75).
        let near = polys[0].pts.iter().any(|q| (q.x - 50.0).abs() < 2.0 && (q.y - 75.0).abs() < 0.2);
        assert!(near);
        assert!(polys[0].pts.len() > 10);
    }

    #[test]
    fn transforms_compose_in_the_order_inner_first() {
        let t = Transform::translate(10.0, 0.0).compose(&Transform::scale(2.0, 2.0));
        assert_eq!(t.apply(Pt::new(1.0, 1.0)), Pt::new(12.0, 2.0));
        let m = Path::rect(0.0, 0.0, 1.0, 1.0).transformed(&t).fill_mask(FillRule::NonZero, WIN, 0.1).unwrap();
        assert_eq!((m.at(10, 0), m.at(11, 1), m.at(12, 0)), (255, 255, 0));
    }

    #[test]
    fn hostile_numbers_do_not_panic() {
        let mut p = Path::new();
        p.move_to(f32::NAN, 0.0);
        p.line_to(f32::INFINITY, 5.0);
        p.cubic_to(1.0e30, -1.0e30, 5.0, 5.0, 1.0e38, 1.0e38);
        p.line_to(3.0, 3.0);
        p.close();
        let _ = p.fill_mask(FillRule::NonZero, WIN, 0.0);
        let _ = p.flatten(f32::NAN);
    }
}
