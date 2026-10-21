//! The coverage rasterizer: signed-area accumulation per scanline (stb_truetype v2 / font-rs).
//!
//! Every edge adds, to each pixel it passes through, the signed area to the right of the edge inside that pixel
//! (`dy * (1 - x_mid)` in the pixel and `dy * x_mid` in the next one). The running sum along a row is then the winding
//! coverage of each pixel: for the non-zero rule its absolute value clamped to 1, for even-odd the triangle wave of it.
//! Edges that overlap inside one pixel add their areas, so a pixel where two same-direction shapes partly overlap
//! is covered a little too much (the usual limit of this method); pixels fully inside a shape are exact.

use crate::fmath::Fclamp;
use super::{IRect, Poly, Pt};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FillRule {
    NonZero,
    EvenOdd,
}

/// The accumulation buffer holds at most this many floats (4 MB); taller areas are done in bands of rows.
const MAX_ACC: usize = 1 << 20;
/// Masks are limited to this many pixels (a 2048 x 2048 area).
pub const MAX_MASK_PIXELS: i64 = 1 << 22;
/// Coordinates are clamped to this before they are used (keeps the arithmetic finite).
const LIMIT: f32 = 1.0e8;

/// The pixels the polygons can touch, inside `win`.
pub fn bounds(polys: &[Poly], win: IRect) -> Option<IRect> {
    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for q in polys.iter().flat_map(|p| p.pts.iter()) {
        if q.x.is_finite() && q.y.is_finite() {
            x0 = x0.min(q.x);
            y0 = y0.min(q.y);
            x1 = x1.max(q.x);
            y1 = y1.max(q.y);
        }
    }
    if x0 > x1 {
        return None;
    }
    let r = IRect { x0: x0.max(-LIMIT).floor() as i32, y0: y0.max(-LIMIT).floor() as i32, x1: x1.min(LIMIT).ceil() as i32, y1: y1.min(LIMIT).ceil() as i32 };
    let r = r.intersect(&win);
    (!r.is_empty()).then_some(r)
}

/// Rasterize `polys` (every subpath is closed implicitly) inside `win`. `row(y, x0, cov)` is called, top to bottom, for
/// each row that has coverage; `cov[i]` is the coverage (0..=255) of the pixel at `x0 + i, y`.
pub fn fill_polys(polys: &[Poly], rule: FillRule, win: IRect, row: &mut dyn FnMut(i32, i32, &[u8])) {
    let Some(area) = bounds(polys, win) else { return };
    let w = area.width() as usize;
    let stride = w + 2;
    let band = (MAX_ACC / stride).max(1);
    let mut acc = vec![0.0f32; stride * band.min(area.height() as usize)];
    let mut cov = vec![0u8; w];
    let mut by = area.y0;
    while by < area.y1 {
        let rows = band.min((area.y1 - by) as usize);
        let org = Pt::new(area.x0 as f32, by as f32);
        for p in polys {
            let n = p.pts.len();
            for (i, a) in p.pts.iter().enumerate() {
                let Some(b) = p.pts.get((i + 1) % n) else { continue };
                if n > 1 {
                    edge(&mut acc, stride, w as f32, rows, clampp(*a, org), clampp(*b, org));
                }
            }
        }
        for (r, line) in acc.chunks_exact_mut(stride).take(rows).enumerate() {
            let mut sum = 0.0f32;
            let mut any = false;
            for (c, a) in cov.iter_mut().zip(line.iter()) {
                sum += *a;
                let v = match rule {
                    FillRule::NonZero => sum.abs().min(1.0),
                    FillRule::EvenOdd => {
                        let a = sum.abs();
                        let t = a - (a * 0.5).floor() * 2.0;
                        if t > 1.0 { 2.0 - t } else { t }
                    }
                };
                *c = (v * 255.0 + 0.5) as u8;
                any |= *c != 0;
            }
            line.fill(0.0);
            if any {
                row(by + r as i32, area.x0, &cov);
            }
        }
        by += rows as i32;
    }
}

/// `p` relative to `org`, finite and bounded.
fn clampp(p: Pt, org: Pt) -> Pt {
    let f = |v: f32| if v.is_finite() { v.fclamp(-LIMIT, LIMIT) } else { 0.0 };
    Pt::new(f(p.x) - org.x, f(p.y) - org.y)
}

fn add(a: &mut [f32], i: usize, v: f32) {
    if let Some(c) = a.get_mut(i) {
        *c += v;
    }
}

/// Deposit the area of the edge `p0 -> p1` (relative to the band) into `acc`.
fn edge(acc: &mut [f32], stride: usize, wf: f32, rows: usize, p0: Pt, p1: Pt) {
    if p0.y == p1.y {
        return;
    }
    let (dir, p0, p1) = if p0.y < p1.y { (1.0f32, p0, p1) } else { (-1.0, p1, p0) };
    let rowsf = rows as f32;
    if p1.y <= 0.0 || p0.y >= rowsf {
        return;
    }
    let dxdy = (p1.x - p0.x) / (p1.y - p0.y);
    let mut x = p0.x;
    if p0.y < 0.0 {
        x -= p0.y * dxdy;
    }
    let ystart = p0.y.max(0.0);
    let yend = p1.y.min(rowsf);
    let mut y = ystart as usize;
    while (y as f32) < yend {
        let top = (y as f32).max(ystart);
        let bot = ((y + 1) as f32).min(yend);
        let dy = bot - top;
        let xnext = x + dxdy * dy;
        let d = dy * dir;
        let (lo, hi) = if x < xnext { (x, xnext) } else { (xnext, x) };
        let (lo, hi) = (lo.fclamp(0.0, wf), hi.fclamp(0.0, wf));
        let base = y * stride;
        let lo_floor = lo.floor();
        let lo_i = lo_floor as usize;
        let hi_ceil = hi.ceil();
        let hi_i = hi_ceil as usize;
        if hi_i <= lo_i + 1 {
            let xmf = 0.5 * (lo + hi) - lo_floor;
            add(acc, base + lo_i, d - d * xmf);
            add(acc, base + lo_i + 1, d * xmf);
        } else {
            let s = 1.0 / (hi - lo);
            let lo_f = lo - lo_floor;
            let a0 = 0.5 * s * (1.0 - lo_f) * (1.0 - lo_f);
            let hi_f = hi - hi_ceil + 1.0;
            let am = 0.5 * s * hi_f * hi_f;
            add(acc, base + lo_i, d * a0);
            if hi_i == lo_i + 2 {
                add(acc, base + lo_i + 1, d * (1.0 - a0 - am));
            } else {
                let a1 = s * (1.5 - lo_f);
                add(acc, base + lo_i + 1, d * (a1 - a0));
                for xi in lo_i + 2..hi_i - 1 {
                    add(acc, base + xi, d * s);
                }
                let a2 = a1 + (hi_i - lo_i - 3) as f32 * s;
                add(acc, base + hi_i - 1, d * (1.0 - a2 - am));
            }
            add(acc, base + hi_i, d * am);
        }
        x = xnext;
        y += 1;
    }
}

/// A coverage mask: `data` holds `rect.width() * rect.height()` bytes, row by row; outside `rect` the coverage is 0.
#[derive(Clone, Debug)]
pub struct Mask {
    pub rect: IRect,
    pub data: Vec<u8>,
}

impl Mask {
    /// A mask of the polygons inside `win`; `None` when the area to cover is bigger than [`MAX_MASK_PIXELS`].
    pub fn from_polys(polys: &[Poly], rule: FillRule, win: IRect) -> Option<Mask> {
        let Some(area) = bounds(polys, win) else {
            return Some(Mask { rect: IRect { x0: 0, y0: 0, x1: 0, y1: 0 }, data: Vec::new() });
        };
        if i64::from(area.width()) * i64::from(area.height()) > MAX_MASK_PIXELS {
            return None;
        }
        let w = area.width() as usize;
        let mut data = vec![0u8; w * area.height() as usize];
        fill_polys(polys, rule, area, &mut |y, x0, cov| {
            let at = (y - area.y0) as usize * w + (x0 - area.x0) as usize;
            if let Some(dst) = data.get_mut(at..at + cov.len()) {
                dst.copy_from_slice(cov);
            }
        });
        Some(Mask { rect: area, data })
    }

    /// The coverage of pixel (`x`, `y`).
    pub fn at(&self, x: i32, y: i32) -> u8 {
        if x < self.rect.x0 || x >= self.rect.x1 || y < self.rect.y0 || y >= self.rect.y1 {
            return 0;
        }
        let i = (y - self.rect.y0) as usize * self.rect.width() as usize + (x - self.rect.x0) as usize;
        self.data.get(i).copied().unwrap_or(0)
    }

    /// The coverage of row `y` over `rect.x0..rect.x1`.
    pub fn row(&self, y: i32) -> Option<&[u8]> {
        if y < self.rect.y0 || y >= self.rect.y1 {
            return None;
        }
        let w = self.rect.width() as usize;
        let i = (y - self.rect.y0) as usize * w;
        self.data.get(i..i + w)
    }

    /// Multiply by `other` (intersect two clips): the result is 0 outside `other`.
    pub fn multiply(&mut self, other: &Mask) {
        let w = self.rect.width() as usize;
        if w == 0 {
            return;
        }
        for (r, line) in self.data.chunks_exact_mut(w).enumerate() {
            let y = self.rect.y0 + r as i32;
            for (c, v) in line.iter_mut().enumerate() {
                let o = u32::from(other.at(self.rect.x0 + c as i32, y));
                *v = ((u32::from(*v) * o + 127) / 255) as u8;
            }
        }
    }
}
