#![allow(clippy::indexing_slicing)] // pixel buffers indexed inside bounds checked just above
//! Shadows, text-shadow blur and groups (`opacity`, `transform`) for the rasterizer (L2 step 2c-2).
//!
//! * **Blur** (css-backgrounds-3 6.1.2): a Gaussian of standard deviation `blur / 2` is approximated by three box blurs per
//!   axis (the box sizes of filter-effects-1 `feGaussianBlur`), each a running sum, so the cost does not depend on the radius.
//! * **Box shadows**: the blurred coverage of the shadow shape is made once for a (shape size, radii, sigma) and kept
//!   ([`MASKS`]); a frame only blends it, outside the border box for an outer shadow and inside the padding box for an
//!   inset one (6.1.1). A shadow without blur and with square corners is a rectangle and needs no mask.
//! * **Groups**: the items are drawn upright into an offscreen layer at the scale of the transform (so scaled text is
//!   re-rasterized, not stretched), then the layer is composited through the rest of the transform (a whole-pixel offset,
//!   or bilinear sampling for rotation and skew) with the group's opacity (css-color-4 12.1, css-compositing-1 3).

use std::cell::RefCell;
use std::hash::Hasher;
use std::rc::Rc;

use super::borders::RR;
use super::raster::{Ctx, Frame, corner_boxes, device_rect, div255, over, premul};
use super::{Group, ShadowDraw, TextRun};
use crate::html::atoms::{FxHashMap, FxHasher};
use crate::platform::GlyphMask;
use crate::vector::{FillRule, IRect, Mask};

/// A blurred mask bigger than this many pixels is not kept from frame to frame.
const MAX_KEPT_PX: usize = 1 << 20;
/// All kept masks together.
const MAX_KEPT_BYTES: usize = 6 << 20;
/// A shadow whose mask would be bigger than this many pixels is not drawn.
const MAX_MASK_PX: i64 = 1 << 22;
/// Groups nested deeper than this are not drawn.
const MAX_DEPTH: u8 = 6;

type MaskCache = (FxHashMap<u64, Rc<Vec<u8>>>, usize);

thread_local! {
    /// Blurred shadow coverage by (shape, radii, sigma): scrolling moves a shadow by whole pixels, so it is the same mask.
    static MASKS: RefCell<MaskCache> = RefCell::new((FxHashMap::default(), 0));
}

// ---- blur ----------------------------------------------------------------------------------------------

/// The (left, right) extents of the three box blurs for a Gaussian of standard deviation `sigma` (filter-effects-1
/// `feGaussianBlur`: `d = floor(s * 3 * sqrt(2 pi) / 4 + 0.5)`; odd `d`: three boxes of `d`; even: two of `d`, one a pixel
/// to the left and one to the right, and one of `d + 1`).
fn passes(sigma: f32) -> [(usize, usize); 3] {
    let d = (sigma * 1.880_15 + 0.5).floor().clamp(1.0, 4096.0) as usize;
    if d % 2 == 1 { [(d / 2, d / 2); 3] } else { [(d / 2, d / 2 - 1), (d / 2 - 1, d / 2), (d / 2, d / 2)] }
}

/// How far the three passes spread a pixel.
fn reach(sigma: f32) -> i32 {
    if sigma < 0.3 { 0 } else { passes(sigma).iter().map(|&(l, r)| l.max(r) as i32).sum::<i32>() + 1 }
}

/// `dst[i]` = the mean of `src[i - l ..= i + r]` (zero outside).
fn box_line(src: &[u8], dst: &mut [u8], l: usize, r: usize) {
    let n = src.len();
    let size = (l + r + 1) as u32;
    let mut sum: u32 = src.iter().take(r + 1).map(|&v| u32::from(v)).sum();
    for i in 0..n {
        dst[i] = ((sum + size / 2) / size) as u8;
        if i + r + 1 < n {
            sum += u32::from(src[i + r + 1]);
        }
        if i >= l {
            sum -= u32::from(src[i - l]);
        }
    }
}

fn blur_mask(data: &mut [u8], w: usize, h: usize, sigma: f32) {
    if sigma < 0.3 || w == 0 || h == 0 || data.len() < w * h {
        return;
    }
    let ps = passes(sigma);
    let mut tmp = vec![0u8; w.max(h)];
    for row in data.chunks_exact_mut(w) {
        for &(l, r) in &ps {
            box_line(row, &mut tmp[..w], l, r);
            row.copy_from_slice(&tmp[..w]);
        }
    }
    let mut col = vec![0u8; h];
    for x in 0..w {
        for (y, c) in col.iter_mut().enumerate() {
            *c = data[y * w + x];
        }
        for &(l, r) in &ps {
            box_line(&col, &mut tmp[..h], l, r);
            col.copy_from_slice(&tmp[..h]);
        }
        for (y, c) in col.iter().enumerate() {
            data[y * w + x] = *c;
        }
    }
}

/// css-backgrounds-3 6.1.1: the radius of a shadow shape that is `spread` (device px; negative: smaller) bigger than the box:
/// the radius plus the spread, the spread first multiplied by `1 + (r - 1)^3` when the radius `r` is smaller than it.
fn spread_radius(radius: f32, spread: f32) -> f32 {
    if radius <= 0.0 {
        return 0.0;
    }
    let a = spread.abs();
    let f = if radius < a { 1.0 + (radius / a - 1.0).powi(3) } else { 1.0 };
    (radius + spread * f).max(0.0)
}

fn with_cache(key: u64, build: impl FnOnce() -> Option<Vec<u8>>) -> Option<Rc<Vec<u8>>> {
    if let Some(m) = MASKS.with(|c| c.borrow().0.get(&key).cloned()) {
        return Some(m);
    }
    let m = Rc::new(build()?);
    if m.len() <= MAX_KEPT_PX {
        MASKS.with(|c| {
            let mut c = c.borrow_mut();
            if c.1 + m.len() > MAX_KEPT_BYTES {
                c.0.clear();
                c.1 = 0;
            }
            c.1 += m.len();
            c.0.insert(key, Rc::clone(&m));
        });
    }
    Some(m)
}

/// The coverage of the polygons of `rr` placed in a `w` x `h` buffer, `fill` before and `invert`ed if asked.
fn shape_buffer(rr: &RR, w: i32, h: i32, invert: bool) -> Option<Vec<u8>> {
    let m = Mask::from_polys(&rr.polys(), FillRule::NonZero, IRect { x0: 0, y0: 0, x1: w, y1: h })?;
    let mut data = vec![if invert { 255 } else { 0 }; w as usize * h as usize];
    let mw = m.rect.width() as usize;
    for (r, line) in m.data.chunks_exact(mw.max(1)).enumerate() {
        let at = (m.rect.y0 as usize + r) * w as usize + m.rect.x0 as usize;
        if let Some(dst) = data.get_mut(at..at + mw) {
            for (d, &s) in dst.iter_mut().zip(line) {
                *d = if invert { 255 - s } else { s };
            }
        }
    }
    Some(data)
}

fn blend(px: &mut [u8], color: [u8; 4], a: u32) {
    match a {
        0 => {}
        255 => over(px, color),
        a => over(px, [div255(u32::from(color[0]) * a) as u8, div255(u32::from(color[1]) * a) as u8, div255(u32::from(color[2]) * a) as u8, div255(u32::from(color[3]) * a) as u8]),
    }
}

impl Ctx<'_> {
    /// A box shadow (css-backgrounds-3 6.1).
    #[inline(never)]
    pub(super) fn shadow(&mut self, frame: &mut Frame, i: u32, sh: &ShadowDraw, clip: &IRect) {
        let k = self.dpr as f32;
        let mut b = device_rect(&sh.rect, self.dpr);
        b.x0 -= self.sx;
        b.x1 -= self.sx;
        b.y0 -= self.sy;
        b.y1 -= self.sy;
        let sigma = sh.blur * k / 2.0;
        let pad = reach(sigma);
        let (dx, dy, sp) = ((sh.dx * k).round() as i32, (sh.dy * k).round() as i32, (sh.spread * k).round() as i32);
        let clip_rr = RR { x0: b.x0 as f32, y0: b.y0 as f32, x1: b.x1 as f32, y1: b.y1 as f32, r: sh.radii.map(|(a, c)| (a * k, c * k)) }.fit();
        // The shadow shape: the box moved by the offsets and grown by the spread (inset: the hole, shrunk by it).
        let g = if sh.inset { -sp } else { sp };
        let s0 = (b.x0 + dx - g, b.y0 + dy - g);
        let s1 = ((b.x1 + dx + g).max(s0.0), (b.y1 + dy + g).max(s0.1));
        let sr = clip_rr.r.map(|(a, c)| if a > 0.0 && c > 0.0 { (spread_radius(a, g as f32), spread_radius(c, g as f32)) } else { (0.0, 0.0) });
        let square = sr.iter().all(|r| r.0 <= 0.0 || r.1 <= 0.0);
        let (win, fast) = if sh.inset { (IRect { x0: b.x0 - pad, y0: b.y0 - pad, x1: b.x1 + pad, y1: b.y1 + pad }, false) } else { (IRect { x0: s0.0 - pad, y0: s0.1 - pad, x1: s1.0 + pad, y1: s1.1 + pad }, sigma < 0.3 && square) };
        let (w, h) = (win.x1 - win.x0, win.y1 - win.y0);
        let outer_area = if sh.inset { win.intersect(clip).intersect(&b) } else { win.intersect(clip) };
        if outer_area.is_empty() || w <= 0 || h <= 0 || (!fast && i64::from(w) * i64::from(h) > MAX_MASK_PX) {
            return;
        }
        let mask = if fast {
            None
        } else {
            let mut hs = FxHasher::default();
            hs.write_u8(u8::from(sh.inset));
            for v in [w, h, s0.0 - win.x0, s0.1 - win.y0, s1.0 - win.x0, s1.1 - win.y0] {
                hs.write_i32(v);
            }
            for (a, c) in sr {
                hs.write_u32(a.to_bits());
                hs.write_u32(c.to_bits());
            }
            hs.write_u32(sigma.to_bits());
            let shape = RR { x0: (s0.0 - win.x0) as f32, y0: (s0.1 - win.y0) as f32, x1: (s1.0 - win.x0) as f32, y1: (s1.1 - win.y0) as f32, r: sr };
            let Some(m) = with_cache(hs.finish(), || {
                let mut data = shape_buffer(&shape.fit(), w, h, sh.inset)?;
                blur_mask(&mut data, w as usize, h as usize, sigma);
                Some(data)
            }) else {
                return;
            };
            Some(m)
        };
        let corners = self.clip_corners(&clip_rr, 0x8000_0000 | i);
        let color = premul(sh.color);
        let (xa, xb) = (outer_area.x0.max(0), outer_area.x1.min(frame.width as i32));
        if xa >= xb {
            return;
        }
        let mut prow = vec![0u8; (xb - xa) as usize];
        let stride = frame.width as usize * 4;
        self.stats.items += 1;
        for y in outer_area.y0.max(0)..outer_area.y1.min(frame.height as i32) {
            // How much of the pixel is where the shadow may be drawn: outside the box for an outer shadow, inside for an inset one.
            if y >= b.y0 && y < b.y1 {
                cover_row(&b, &corners, y, xa, &mut prow);
            } else if sh.inset {
                continue;
            } else {
                prow.fill(0);
            }
            let Some(row) = frame.data.get_mut(y as usize * stride + xa as usize * 4..y as usize * stride + xb as usize * 4) else { continue };
            let mrow = mask.as_ref().and_then(|d| {
                let at = (y - win.y0) as usize * w as usize + (xa - win.x0) as usize;
                d.get(at..at + prow.len())
            });
            let in_rect = y >= s0.1 && y < s1.1;
            for (j, (px, &p)) in row.as_chunks_mut::<4>().0.iter_mut().zip(&prow).enumerate() {
                let p = if sh.inset { u32::from(p) } else { 255 - u32::from(p) };
                if p == 0 {
                    continue;
                }
                let m = match mrow {
                    Some(r) => u32::from(r.get(j).copied().unwrap_or(0)),
                    None => u32::from(in_rect && (xa + j as i32) >= s0.0 && (xa + j as i32) < s1.0) * 255,
                };
                if m != 0 {
                    blend(px, color, if m == 255 { p } else if p == 255 { m } else { div255(m * p) });
                }
            }
        }
    }

    /// The masks of the corners of the shape `rr` (made once per frame): (corner box, mask, where the mask's origin is).
    fn clip_corners(&mut self, rr: &RR, cidx: u32) -> Vec<(IRect, Rc<Mask>, i32, i32)> {
        let mut out = Vec::new();
        for (k, cb) in corner_boxes(rr).into_iter().enumerate() {
            if cb.is_empty() {
                continue;
            }
            if let Some(mi) = self.corner_mask(cidx, k, rr)
                && let Some(c) = self.corner_cache.get(mi)
            {
                out.push((cb, Rc::clone(&c.2), c.3, c.4));
            }
        }
        out
    }

    /// Call `f(x, y, mask)` for the glyph masks of a text run that may reach into `clip` (device pixels).
    pub(super) fn glyphs_of(&self, run: &TextRun, clip: &IRect, f: &mut dyn FnMut(i32, i32, &GlyphMask)) {
        let Some(glyphs) = self.list.glyphs.get(run.glyphs.0 as usize..run.glyphs.1 as usize) else { return };
        let dpr = self.dpr as f32;
        let size = run.size * dpr;
        let mut pen = run.x.to_f32() * dpr - self.sx as f32;
        let base = super::raster::dev(run.baseline, self.dpr) - self.sy;
        // Glyphs this far outside the clip cannot reach into it.
        let (lo, hi) = (clip.x0 as f32 - 2.0 * size - 4.0, clip.x1 as f32 + 2.0 * size + 4.0);
        for g in glyphs {
            let gx = pen + g.dx * dpr;
            pen += g.advance * dpr;
            if !(gx > lo && gx < hi) {
                continue;
            }
            let ix = gx.floor();
            let sub = (((gx - ix) * 4.0) as u8).min(3);
            let Some(mask) = self.platform.glyph_mask(crate::platform::FontId(u32::from(g.font)), u32::from(g.glyph), size, sub) else { continue };
            f(ix as i32 + mask.left, base + (g.dy * dpr).round() as i32 + mask.top, &mask);
        }
    }

    /// A text run with a blurred shadow: the glyph coverage is blurred as a whole (css-text-decor-3 4.1).
    #[inline(never)]
    pub(super) fn text_blur(&mut self, frame: &mut Frame, run: &TextRun, clip: &IRect) {
        let sigma = run.blur * self.dpr as f32 / 2.0;
        let pad = reach(sigma);
        let wide = IRect { x0: clip.x0 - pad, y0: clip.y0 - pad, x1: clip.x1 + pad, y1: clip.y1 + pad };
        let mut u = IRect { x0: i32::MAX, y0: i32::MAX, x1: i32::MIN, y1: i32::MIN };
        self.glyphs_of(run, &wide, &mut |x, y, m| {
            if m.width > 0 {
                u = IRect { x0: u.x0.min(x), y0: u.y0.min(y), x1: u.x1.max(x + m.width as i32), y1: u.y1.max(y + m.height as i32) };
            }
        });
        let u = IRect { x0: u.x0 - pad, y0: u.y0 - pad, x1: u.x1 + pad, y1: u.y1 + pad }.intersect(&wide);
        let (w, h) = (u.x1 - u.x0, u.y1 - u.y0);
        if u.is_empty() || i64::from(w) * i64::from(h) > MAX_MASK_PX / 2 {
            return;
        }
        let mut buf = vec![0u8; w as usize * h as usize];
        self.glyphs_of(run, &wide, &mut |x, y, m| {
            for (r, line) in m.alpha.chunks_exact((m.width as usize).max(1)).enumerate() {
                let py = y + r as i32 - u.y0;
                if py < 0 || py >= h {
                    continue;
                }
                for (c, &a) in line.iter().enumerate() {
                    let px = x + c as i32 - u.x0;
                    if px >= 0 && px < w {
                        let d = &mut buf[py as usize * w as usize + px as usize];
                        *d = d.saturating_add(a);
                    }
                }
            }
        });
        blur_mask(&mut buf, w as usize, h as usize, sigma);
        let color = premul(run.color);
        let area = u.intersect(clip);
        self.stats.items += 1;
        for y in area.y0..area.y1 {
            let stride = frame.width as usize * 4;
            let Some(row) = frame.data.get_mut(y as usize * stride..(y as usize + 1) * stride) else { continue };
            for x in area.x0..area.x1 {
                let a = u32::from(buf[(y - u.y0) as usize * w as usize + (x - u.x0) as usize]);
                if let Some(px) = row.get_mut(x as usize * 4..x as usize * 4 + 4) {
                    blend(px, color, a);
                }
            }
        }
    }

    /// A group (see [`Group`]): `r` is its transformed extent in device pixels, `clip` the area it may draw in.
    #[inline(never)]
    pub(super) fn group(&mut self, frame: &mut Frame, g: &Group, r: &IRect, clip: &IRect) {
        if self.depth >= MAX_DEPTH {
            return;
        }
        let xf = &g.xf;
        let (a, b, c, d) = (f64::from(xf.a), f64::from(xf.b), f64::from(xf.c), f64::from(xf.d));
        let k = a.hypot(b).max(c.hypot(d)).clamp(0.125, 8.0);
        // What is left of the linear part once the scale is in the layer.
        let near = |v: f64, t: f64| (v - t).abs() < 1.0e-3;
        // A turn by a multiple of 90 degrees is exact, not the rounding of a cosine.
        let snap = |v: f64| if near(v, 0.0) { 0.0 } else if near(v.abs(), 1.0) { v.signum() } else { v };
        let (ra, rb, rc, rd) = (snap(a / k), snap(b / k), snap(c / k), snap(d / k));
        let same = near(ra, 1.0) && near(rd, 1.0) && near(rb, 0.0) && near(rc, 0.0);
        let perm = [ra, rb, rc, rd].iter().all(|&v| near(v, 0.0) || near(v.abs(), 1.0));
        let (mut tx, mut ty) = (self.dpr * f64::from(xf.e) - f64::from(self.sx), self.dpr * f64::from(xf.f) - f64::from(self.sy));
        if same || perm {
            (tx, ty) = (tx.round(), ty.round());
        }
        let dpr = self.dpr * k;
        let alpha = u32::from(g.opacity);
        if same {
            let off = (tx as i32, ty as i32);
            let area = r.intersect(clip);
            if area.is_empty() {
                return;
            }
            if alpha == 255 {
                // Nothing to composite: draw straight into the frame.
                let mut child = self.child(dpr, -off.0, -off.1, area);
                for it in &g.items {
                    child.item(frame, it);
                }
                let st = child.stats;
                self.merge(st);
                return;
            }
            let (w, h) = (area.x1 - area.x0, area.y1 - area.y0);
            let mut layer = Frame::new(w as u32, h as u32);
            let mut child = self.child(dpr, area.x0 - off.0, area.y0 - off.1, IRect { x0: 0, y0: 0, x1: w, y1: h });
            for it in &g.items {
                child.item(&mut layer, it);
            }
            let st = child.stats;
                self.merge(st);
            for y in 0..h {
                let Some(src) = layer.data.get(y as usize * w as usize * 4..(y as usize + 1) * w as usize * 4) else { continue };
                let stride = frame.width as usize * 4;
                let Some(row) = frame.data.get_mut((area.y0 + y) as usize * stride..(area.y0 + y + 1) as usize * stride) else { continue };
                for (x, s) in src.as_chunks::<4>().0.iter().enumerate() {
                    if s[3] != 0
                        && let Some(px) = row.get_mut((area.x0 as usize + x) * 4..(area.x0 as usize + x) * 4 + 4)
                    {
                        over(px, s.map(|v| div255(u32::from(v) * alpha) as u8));
                    }
                }
            }
            return;
        }
        // Rotation, skew, flips: the layer holds the part of the upright drawing that lands in the destination.
        let det = ra * rd - rb * rc;
        if det.abs() < 1.0e-6 {
            return;
        }
        let dest = IRect { x0: r.x0 - 1, y0: r.y0 - 1, x1: r.x1 + 1, y1: r.y1 + 1 }.intersect(clip);
        if dest.is_empty() {
            return;
        }
        // Frame pixel (x, y) shows the layer-space point R^-1 * ((x, y) - t).
        let inv = |x: f64, y: f64| ((rd * (x - tx) - rc * (y - ty)) / det, (-rb * (x - tx) + ra * (y - ty)) / det);
        let corners = [(dest.x0, dest.y0), (dest.x1, dest.y0), (dest.x0, dest.y1), (dest.x1, dest.y1)].map(|(x, y)| inv(f64::from(x), f64::from(y)));
        let fold = |f: fn(f64, f64) -> f64, g: fn(&(f64, f64)) -> f64, init: f64| corners.iter().map(g).fold(init, f);
        let (qx0, qx1) = (fold(f64::min, |p| p.0, f64::MAX), fold(f64::max, |p| p.0, f64::MIN));
        let (qy0, qy1) = (fold(f64::min, |p| p.1, f64::MAX), fold(f64::max, |p| p.1, f64::MIN));
        // The drawing itself, in layer space (scaled by dpr * k), is the other limit.
        let own = device_rect(&g.local, dpr);
        let lay = IRect { x0: qx0.floor().max(f64::from(own.x0) - 1.0) as i32, y0: qy0.floor().max(f64::from(own.y0) - 1.0) as i32, x1: qx1.ceil().min(f64::from(own.x1) + 1.0) as i32, y1: qy1.ceil().min(f64::from(own.y1) + 1.0) as i32 };
        let (w, h) = (lay.x1 - lay.x0, lay.y1 - lay.y0);
        let cap = i64::from(frame.width).max(512) * i64::from(frame.height).max(512) * 2;
        if lay.is_empty() || i64::from(w) * i64::from(h) > cap {
            return;
        }
        let mut layer = Frame::new(w as u32, h as u32);
        let mut child = self.child(dpr, lay.x0, lay.y0, IRect { x0: 0, y0: 0, x1: w, y1: h });
        for it in &g.items {
            child.item(&mut layer, it);
        }
        let st = child.stats;
                self.merge(st);
        let px_at = |x: i32, y: i32| -> [u32; 4] {
            if x < 0 || y < 0 || x >= w || y >= h {
                return [0; 4];
            }
            let i = (y as usize * w as usize + x as usize) * 4;
            match layer.data.get(i..i + 4) {
                Some(p) => [u32::from(p[0]), u32::from(p[1]), u32::from(p[2]), u32::from(p[3])],
                None => [0; 4],
            }
        };
        for y in dest.y0..dest.y1 {
            let stride = frame.width as usize * 4;
            let Some(row) = frame.data.get_mut(y.max(0) as usize * stride..(y.max(0) as usize + 1) * stride) else { continue };
            for x in dest.x0..dest.x1 {
                let (qx, qy) = inv(f64::from(x) + 0.5, f64::from(y) + 0.5);
                // Sample positions in 1/256 pixel: the rounding of the matrix must not leak into the weights.
                let q = |v: f64| (v * 256.0).round() / 256.0;
                let (fx, fy) = (q(qx - f64::from(lay.x0) - 0.5), q(qy - f64::from(lay.y0) - 0.5));
                let (ix, iy) = (fx.floor(), fy.floor());
                let (wx, wy) = (((fx - ix) * 256.0) as u32, ((fy - iy) * 256.0) as u32);
                let (ix, iy) = (ix as i32, iy as i32);
                let (p00, p10, p01, p11) = (px_at(ix, iy), px_at(ix + 1, iy), px_at(ix, iy + 1), px_at(ix + 1, iy + 1));
                let mut out = [0u8; 4];
                for (c, o) in out.iter_mut().enumerate() {
                    let top = p00[c] * (256 - wx) + p10[c] * wx;
                    let bot = p01[c] * (256 - wx) + p11[c] * wx;
                    *o = div255((((top * (256 - wy) + bot * wy) >> 16).min(255)) * alpha) as u8;
                }
                if out[3] != 0
                    && x >= 0
                    && let Some(px) = row.get_mut(x as usize * 4..x as usize * 4 + 4)
                {
                    over(px, out);
                }
            }
        }
    }

    /// A context for the items of a group: `dpr` device pixels per CSS px, the page at `(-sx, -sy)`, drawing inside `bounds`.
    fn child(&mut self, dpr: f64, sx: i32, sy: i32, bounds: IRect) -> Ctx<'_> {
        Ctx { list: self.list, platform: self.platform, images: &mut *self.images, dpr, sx, sy, bounds, stats: super::raster::RasterStats::default(), corner_cache: Vec::new(), depth: self.depth + 1, base_dpr: self.base_dpr }
    }

    fn merge(&mut self, st: super::raster::RasterStats) {
        self.stats.items += st.items;
        self.stats.glyphs += st.glyphs;
        self.stats.images += st.images;
    }
}

/// The coverage of the (possibly rounded) shape `b` (a rectangle with the corners of `corners`) for the pixels `x0 ..` of row `y`.
fn cover_row(b: &IRect, corners: &[(IRect, Rc<Mask>, i32, i32)], y: i32, x0: i32, out: &mut [u8]) {
    out.fill(0);
    if y < b.y0 || y >= b.y1 {
        return;
    }
    let x1 = x0 + out.len() as i32;
    let (lo, hi) = ((b.x0.max(x0) - x0).max(0) as usize, (b.x1.min(x1) - x0).max(0) as usize);
    if lo < hi {
        out[lo..hi].fill(255);
    }
    for (cb, m, ox, oy) in corners {
        if y >= cb.y0 && y < cb.y1 {
            for x in cb.x0.max(x0)..cb.x1.min(x1) {
                out[(x - x0) as usize] = m.at(x - ox, y - oy);
            }
        }
    }
}
