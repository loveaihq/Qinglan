#![allow(clippy::indexing_slicing)] // pixel buffers indexed inside bounds checked just above
//! Shadows, text-shadow blur and groups (`opacity`, `transform`) for the rasterizer (L2 step 2c-2).
//!
//! * **Blur** (css-backgrounds-3 6.1.2): a Gaussian of standard deviation `blur / 2` is approximated by three box blurs per
//!   axis (the box sizes of filter-effects-1 `feGaussianBlur`), each a running sum, so the cost does not depend on the radius.
//! * **Box shadows**: the blurred coverage of the shadow shape is made once and kept ([`MASKS`]); a frame only blends it,
//!   outside the border box for an outer shadow and inside the padding box for an inset one (6.1.1). A shadow without blur
//!   and with square corners is a rectangle and needs no mask. The mask is a **9-patch** ([`Patch`]): far from the edges
//!   and corners the blurred shape does not change along an axis, so the columns (rows) in between are cut out, the small
//!   shape is blurred, and a pixel of the real window reads the small mask at a mapped position. The mask depends on the
//!   corners and the blur and not on the size of the box, so a card of any size costs the same mask, and 300 boxes of one
//!   shape share it.
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
/// Groups nested deeper than this are not given a layer: they are drawn straight into the frame (without their opacity;
/// a transform other than scale and shift is dropped), so that content never goes missing.
const MAX_DEPTH: u8 = 6;
/// Groups nested deeper than this are not drawn at all (the drawing recurses once per level).
const MAX_NESTED: u8 = 64;

type MaskCache = (FxHashMap<u64, Rc<Vec<u8>>>, usize);

/// The linear part of a group's transform, as `Ctx::group` splits it, for [`Ctx::rotated`].
#[derive(Clone, Copy)]
struct Lin {
    ra: f64,
    rb: f64,
    rc: f64,
    rd: f64,
    det: f64,
    tx: f64,
    ty: f64,
    dpr: f64,
    alpha: u32,
}

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

/// `sum / size` rounded, for the sums of a blur (at most 255 * 4097) without a division: `(sum + size / 2) * m >> 40` with
/// `m = 2^40 / size + 1` is exact there (the error is under `sum / 2^40 < 2^-19`, and a quotient is never within `1 / size`
/// of the next integer without being it).
struct Mean {
    m: u64,
    half: u32,
}

impl Mean {
    fn new(size: u32) -> Mean {
        Mean { m: (1u64 << 40) / u64::from(size.max(1)) + 1, half: size / 2 }
    }

    #[inline(always)]
    fn of(&self, sum: u32) -> u8 {
        ((u64::from(sum + self.half) * self.m) >> 40) as u8
    }
}

/// `dst[i]` = the mean of `src[i - l ..= i + r]` (zero outside).
fn box_line(src: &[u8], dst: &mut [u8], l: usize, r: usize) {
    let n = src.len();
    let mean = Mean::new((l + r + 1) as u32);
    let mut sum: u32 = src.iter().take(r + 1).map(|&v| u32::from(v)).sum();
    for i in 0..n {
        dst[i] = mean.of(sum);
        if i + r + 1 < n {
            sum += u32::from(src[i + r + 1]);
        }
        if i >= l {
            sum -= u32::from(src[i - l]);
        }
    }
}

/// `box_line` down every column at once: a running sum for each column, moved one row at a time, so that memory is read in
/// order (going down one column at a time touches a new cache line for every pixel).
fn box_cols(src: &[u8], dst: &mut [u8], w: usize, h: usize, l: usize, r: usize, sums: &mut [u32]) {
    let mean = Mean::new((l + r + 1) as u32);
    sums.fill(0);
    for y in 0..=r.min(h.saturating_sub(1)) {
        for (s, &v) in sums.iter_mut().zip(&src[y * w..(y + 1) * w]) {
            *s += u32::from(v);
        }
    }
    for y in 0..h {
        for (o, &s) in dst[y * w..(y + 1) * w].iter_mut().zip(sums.iter()) {
            *o = mean.of(s);
        }
        if y + r + 1 < h {
            for (s, &v) in sums.iter_mut().zip(&src[(y + r + 1) * w..(y + r + 2) * w]) {
                *s += u32::from(v);
            }
        }
        if y >= l {
            for (s, &v) in sums.iter_mut().zip(&src[(y - l) * w..(y - l + 1) * w]) {
                *s -= u32::from(v);
            }
        }
    }
}

fn blur_mask(data: &mut [u8], w: usize, h: usize, sigma: f32) {
    if sigma < 0.3 || w == 0 || h == 0 || data.len() < w * h {
        return;
    }
    let data = &mut data[..w * h];
    let ps = passes(sigma);
    let mut tmp = vec![0u8; w.max(h)];
    for row in data.chunks_exact_mut(w) {
        for &(l, r) in &ps {
            box_line(row, &mut tmp[..w], l, r);
            row.copy_from_slice(&tmp[..w]);
        }
    }
    // The columns: three passes between the mask and a second buffer, the last one ending in the second buffer.
    let mut second = vec![0u8; w * h];
    let mut sums = vec![0u32; w];
    let mut in_data = true;
    for &(l, r) in &ps {
        if in_data {
            box_cols(data, &mut second, w, h, l, r, &mut sums);
        } else {
            box_cols(&second, data, w, h, l, r, &mut sums);
        }
        in_data = !in_data;
    }
    if !in_data {
        data.copy_from_slice(&second);
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

/// The geometry of a box-shadow mask, in window pixels: the shape's edges `e` = (left, top, right, bottom), its corner radii
/// (top-left, top-right, bottom-right, bottom-left; already fitted), the blur, and how far the blur reaches.
struct ShadowGeometry {
    inset: bool,
    w: i32,
    h: i32,
    e: [i32; 4],
    r: [(f32, f32); 4],
    sigma: f32,
    pad: i32,
}

/// The blurred shadow mask of a window, with the stretch cut out (see the module comment): `data` is `w` wide; where `cx`
/// is `Some((m0, m1))` the window columns `m0..m1` are all the column `m0`, and the columns from `m1` on are `m1 - m0 - 2 * pad`
/// to the left of where they would be; the rows likewise with `cy`.
struct Patch {
    data: Rc<Vec<u8>>,
    w: i32,
    cx: Option<(i32, i32)>,
    cy: Option<(i32, i32)>,
    pad: i32,
}

/// Where a window row or column `v` is in the cut-down mask.
fn mapped(cut: Option<(i32, i32)>, pad: i32, v: i32) -> i32 {
    match cut {
        Some((m0, m1)) if v >= m1 => v - (m1 - m0) + 2 * pad,
        Some((m0, _)) if v >= m0 => m0,
        _ => v,
    }
}

/// The columns (rows) `m0..m1` of a window of `n` whose blurred values are all the same, when the shape's edges are `e0` and
/// `e1` and its corners reach `r0` and `r1` into it: the shape does not change along the axis between the corners
/// (`e0 + r0 .. e1 - r1`, inside the window), and a pixel sees `pad` to each side. `None` when that saves little.
fn axis_cut(e0: i32, e1: i32, r0: i32, r1: i32, n: i32, pad: i32) -> Option<(i32, i32)> {
    let a = e0.saturating_add(r0).max(0);
    let b = e1.saturating_sub(r1).min(n);
    let (m0, m1) = (a.saturating_add(pad), b.saturating_sub(pad));
    (m1.saturating_sub(m0) > 2 * pad + 8).then_some((m0, m1))
}

/// The mask of the shape: cut, blurred, and kept in [`MASKS`]. `None` when the (cut) window is too big to draw.
fn shadow_patch(g: &ShadowGeometry) -> Option<Patch> {
    // At least one pixel: the repeated column is the first of the cut, so it has to be one that is kept.
    let pad = g.pad.max(1);
    let [ex0, ey0, ex1, ey1] = g.e;
    let [tl, tr, br, bl] = g.r;
    let reach = |v: f32| v.ceil() as i32;
    let cx = axis_cut(ex0, ex1, reach(tl.0.max(bl.0)), reach(tr.0.max(br.0)), g.w, pad);
    let cy = axis_cut(ey0, ey1, reach(tl.1.max(tr.1)), reach(bl.1.max(br.1)), g.h, pad);
    let (cut_x, cut_y) = (cx.map_or(0, |(m0, m1)| m1 - m0 - 2 * pad), cy.map_or(0, |(m0, m1)| m1 - m0 - 2 * pad));
    let (pw, ph) = (g.w - cut_x, g.h - cut_y);
    if pw <= 0 || ph <= 0 || i64::from(pw) * i64::from(ph) > MAX_MASK_PX {
        return None;
    }
    let (px1, py1) = (ex1 - cut_x, ey1 - cut_y);
    let mut hs = FxHasher::default();
    hs.write_u8(u8::from(g.inset));
    for v in [pw, ph, ex0, ey0, px1, py1] {
        hs.write_i32(v);
    }
    for (a, c) in g.r {
        hs.write_u32(a.to_bits());
        hs.write_u32(c.to_bits());
    }
    hs.write_u32(g.sigma.to_bits());
    let shape = RR { x0: ex0 as f32, y0: ey0 as f32, x1: px1 as f32, y1: py1 as f32, r: g.r };
    let data = with_cache(hs.finish(), || {
        let mut data = shape_buffer(&shape, pw, ph, g.inset)?;
        blur_mask(&mut data, pw as usize, ph as usize, g.sigma);
        Some(data)
    })?;
    Some(Patch { data, w: pw, cx, cy, pad })
}

impl Patch {
    /// The mask row that window row `y` reads.
    fn row(&self, y: i32) -> &[u8] {
        let sy = mapped(self.cy, self.pad, y).max(0) as usize;
        let w = self.w.max(0) as usize;
        self.data.get(sy * w..(sy + 1) * w).unwrap_or(&[])
    }

    /// The mask of the window pixels `x0 ..` of window row `y`, `out.len()` of them.
    fn fill(&self, y: i32, x0: i32, out: &mut [u8]) {
        let row = self.row(y);
        let x1 = x0 + out.len() as i32;
        let mut x = x0;
        while x < x1 {
            // The stretch of window columns that are consecutive columns of the mask, or the one column that is repeated.
            let (end, flat) = match self.cx {
                Some((m0, _)) if x < m0 => (x1.min(m0), false),
                Some((_, m1)) if x < m1 => (x1.min(m1), true),
                _ => (x1, false),
            };
            let at = mapped(self.cx, self.pad, x).max(0) as usize;
            if let Some(dst) = out.get_mut((x - x0) as usize..(end - x0) as usize) {
                if flat {
                    dst.fill(row.get(at).copied().unwrap_or(0));
                } else if let Some(src) = row.get(at..at + dst.len()) {
                    dst.copy_from_slice(src);
                } else {
                    dst.fill(0);
                }
            }
            x = end;
        }
    }

    /// The mask value of window pixel (x, y).
    fn at(&self, x: i32, y: i32) -> u8 {
        self.row(y).get(mapped(self.cx, self.pad, x).max(0) as usize).copied().unwrap_or(0)
    }
}

/// `spans` without the pixels `from .. to`.
fn cut_spans(spans: &mut Vec<(i32, i32)>, scratch: &mut Vec<(i32, i32)>, from: i32, to: i32) {
    if from >= to {
        return;
    }
    scratch.clear();
    for &(a, b) in spans.iter() {
        if to <= a || from >= b {
            scratch.push((a, b));
            continue;
        }
        if a < from {
            scratch.push((a, from));
        }
        if to < b {
            scratch.push((to, b));
        }
    }
    std::mem::swap(spans, scratch);
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
        if outer_area.is_empty() || w <= 0 || h <= 0 {
            return;
        }
        // The shadow shape in window pixels, and its mask: a 9-patch, so that its size is that of the corners and the blur,
        // not of the box (a window over `MAX_MASK_PX` that has no stretch to cut out is not drawn).
        let shape = RR { x0: (s0.0 - win.x0) as f32, y0: (s0.1 - win.y0) as f32, x1: (s1.0 - win.x0) as f32, y1: (s1.1 - win.y0) as f32, r: sr }.fit();
        let patch = if fast {
            None
        } else {
            let geo = ShadowGeometry { inset: sh.inset, w, h, e: [s0.0 - win.x0, s0.1 - win.y0, s1.0 - win.x0, s1.1 - win.y0], r: shape.r, sigma, pad };
            match shadow_patch(&geo) {
                Some(p) => Some(p),
                None => return,
            }
        };
        let corners = self.clip_corners(&clip_rr, 0x8000_0000 | i);
        let color = premul(sh.color);
        let (xa, xb) = (outer_area.x0.max(0), outer_area.x1.min(frame.width as i32));
        if xa >= xb {
            return;
        }
        // The middle of the border box is covered by the box itself for an outer shadow: nothing of the shadow shows there.
        // The corners of the box reach no further in than this.
        let [ctl, ctr, cbr, cbl] = clip_rr.r;
        let reach_in = |v: f32| v.ceil() as i32 + 1;
        let covered = (b.x0 + reach_in(ctl.0.max(cbl.0)), b.x1 - reach_in(ctr.0.max(cbr.0)));
        let mut prow = vec![0u8; (xb - xa) as usize];
        let mut mrow = vec![0u8; (xb - xa) as usize];
        let mut spans: Vec<(i32, i32)> = Vec::new();
        let mut scratch: Vec<(i32, i32)> = Vec::new();
        let stride = frame.width as usize * 4;
        self.stats.items += 1;
        for y in outer_area.y0.max(0)..outer_area.y1.min(frame.height as i32) {
            let in_box = y >= b.y0 && y < b.y1;
            // An inset shadow is only drawn inside the box.
            if sh.inset && !in_box {
                continue;
            }
            // The pixels that can show something: not under the box (outer), not where the mask is all one value and
            // that value is nothing (the middle of the window, for an inset shadow).
            spans.clear();
            spans.push((xa, xb));
            if !sh.inset && in_box {
                cut_spans(&mut spans, &mut scratch, covered.0, covered.1);
            }
            if let Some(p) = &patch
                && let Some((m0, m1)) = p.cx
                && p.at(m0, y - win.y0) == 0
            {
                cut_spans(&mut spans, &mut scratch, win.x0 + m0, win.x0 + m1);
            }
            for &(sa, sb) in &spans {
                let len = (sb - sa) as usize;
                let (Some(row), Some(prow), Some(mrow)) =
                    (frame.data.get_mut(y as usize * stride + sa as usize * 4..y as usize * stride + sb as usize * 4), prow.get_mut(..len), mrow.get_mut(..len))
                else {
                    continue;
                };
                // How much of the pixel is where the shadow may be drawn: outside the box for an outer shadow, inside for an inset one.
                if in_box {
                    cover_row(&b, &corners, y, sa, prow);
                } else {
                    prow.fill(0);
                }
                if let Some(p) = &patch {
                    p.fill(y - win.y0, sa - win.x0, mrow);
                }
                let in_rect = y >= s0.1 && y < s1.1;
                for (j, (px, &p)) in row.as_chunks_mut::<4>().0.iter_mut().zip(prow.iter()).enumerate() {
                    let p = if sh.inset { u32::from(p) } else { 255 - u32::from(p) };
                    if p == 0 {
                        continue;
                    }
                    let m = if patch.is_some() { u32::from(mrow.get(j).copied().unwrap_or(0)) } else { u32::from(in_rect && (sa + j as i32) >= s0.0 && (sa + j as i32) < s1.0) * 255 };
                    if m != 0 {
                        blend(px, color, if m == 255 { p } else if p == 255 { m } else { div255(m * p) });
                    }
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
        if run.rot {
            // Sideways text: a glyph is turned a quarter clockwise about its pen position, which moves down the line.
            let px = run.x.to_f32() * dpr - self.sx as f32;
            let mut pen = run.baseline.to_f32() * dpr - self.sy as f32;
            let (lo, hi) = (clip.y0 as f32 - 2.0 * size - 4.0, clip.y1 as f32 + 2.0 * size + 4.0);
            for g in glyphs {
                let gy = pen + g.dx * dpr;
                pen += g.advance * dpr;
                if !(gy > lo && gy < hi) {
                    continue;
                }
                let Some(mask) = self.platform.glyph_mask(crate::platform::FontId(u32::from(g.font)), u32::from(g.glyph), size, 0) else { continue };
                let r = turn_mask(&mask);
                f((px - g.dy * dpr).round() as i32 + r.left, gy.round() as i32 + r.top, &r);
            }
            return;
        }
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

    /// The blurred coverage of the glyphs of a whole text run, kept from frame to frame: scrolling moves a run by whole
    /// pixels, so the same glyphs at the same fraction of a pixel blur to the same mask. The mask is returned with the
    /// device position of its corner in the frame; `None`: the run is too big to keep (or has no glyphs to show).
    fn run_mask(&self, run: &TextRun, sigma: f32, pad: i32) -> Option<(Rc<Vec<u8>>, i32, i32)> {
        let glyphs = self.list.glyphs.get(run.glyphs.0 as usize..run.glyphs.1 as usize)?;
        let pen = run.x.to_f32() * self.dpr as f32 - self.sx as f32;
        let (ox, oy) = (pen.floor() as i32, super::raster::dev(run.baseline, self.dpr) - self.sy);
        let mut hs = FxHasher::default();
        hs.write_u8(1);
        hs.write_u32(run.size.to_bits());
        hs.write_u64(self.dpr.to_bits());
        hs.write_u32(sigma.to_bits());
        hs.write_u32(((pen - pen.floor()) * 1024.0) as u32);
        for g in glyphs {
            for v in [u32::from(g.font), u32::from(g.glyph), g.advance.to_bits(), g.dx.to_bits(), g.dy.to_bits()] {
                hs.write_u32(v);
            }
        }
        let m = with_cache(hs.finish(), || {
            let all = IRect { x0: i32::MIN / 4, y0: i32::MIN / 4, x1: i32::MAX / 4, y1: i32::MAX / 4 };
            let mut u = IRect { x0: i32::MAX, y0: i32::MAX, x1: i32::MIN, y1: i32::MIN };
            self.glyphs_of(run, &all, &mut |x, y, m| {
                if m.width > 0 {
                    u = IRect { x0: u.x0.min(x), y0: u.y0.min(y), x1: u.x1.max(x + m.width as i32), y1: u.y1.max(y + m.height as i32) };
                }
            });
            let u = IRect { x0: u.x0.saturating_sub(pad), y0: u.y0.saturating_sub(pad), x1: u.x1.saturating_add(pad), y1: u.y1.saturating_add(pad) };
            let (w, h) = (i64::from(u.x1) - i64::from(u.x0), i64::from(u.y1) - i64::from(u.y0));
            if u.is_empty() || w * h > MAX_KEPT_PX as i64 {
                return None;
            }
            let (w, h) = (w as i32, h as i32);
            let mut buf = vec![0u8; w as usize * h as usize];
            self.glyphs_of(run, &all, &mut |x, y, m| accumulate(&mut buf, w, h, (x - u.x0, y - u.y0), m));
            blur_mask(&mut buf, w as usize, h as usize, sigma);
            // The corner and the size in front of the pixels.
            let mut out = Vec::with_capacity(16 + buf.len());
            for v in [u.x0 - ox, u.y0 - oy, w, h] {
                out.extend_from_slice(&v.to_le_bytes());
            }
            out.extend_from_slice(&buf);
            Some(out)
        })?;
        let field = |i: usize| m.get(i * 4..i * 4 + 4).and_then(|b| b.try_into().ok()).map_or(0, i32::from_le_bytes);
        let (x, y) = (ox + field(0), oy + field(1));
        Some((m, x, y))
    }

    /// Blend `color` over the frame through the coverage `buf` (`u` is its rectangle), inside `clip`.
    fn blend_mask(&mut self, frame: &mut Frame, buf: &[u8], u: &IRect, color: crate::css::color::Color, clip: &IRect) {
        let w = (u.x1 - u.x0) as usize;
        let color = premul(color);
        let area = u.intersect(clip);
        self.stats.items += 1;
        let stride = frame.width as usize * 4;
        for y in area.y0.max(0)..area.y1.min(frame.height as i32) {
            let Some(row) = frame.data.get_mut(y as usize * stride..(y as usize + 1) * stride) else { continue };
            for x in area.x0.max(0)..area.x1.min(frame.width as i32) {
                let a = u32::from(buf.get((y - u.y0) as usize * w + (x - u.x0) as usize).copied().unwrap_or(0));
                if let Some(px) = row.get_mut(x as usize * 4..x as usize * 4 + 4) {
                    blend(px, color, a);
                }
            }
        }
    }

    /// A text run with a blurred shadow: the glyph coverage is blurred as a whole (css-text-decor-3 4.1).
    #[inline(never)]
    pub(super) fn text_blur(&mut self, frame: &mut Frame, run: &TextRun, clip: &IRect) {
        let sigma = run.blur * self.dpr as f32 / 2.0;
        let pad = reach(sigma);
        if let Some((m, x, y)) = self.run_mask(run, sigma, pad) {
            let field = |i: usize| m.get(i * 4..i * 4 + 4).and_then(|b| b.try_into().ok()).map_or(0, i32::from_le_bytes);
            let u = IRect { x0: x, y0: y, x1: x + field(2), y1: y + field(3) };
            self.blend_mask(frame, m.get(16..).unwrap_or(&[]), &u, run.color, clip);
            return;
        }
        // Too big to keep: only what reaches into the clip is blurred, every frame.
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
        self.glyphs_of(run, &wide, &mut |x, y, m| accumulate(&mut buf, w, h, (x - u.x0, y - u.y0), m));
        blur_mask(&mut buf, w as usize, h as usize, sigma);
        self.blend_mask(frame, &buf, &u, run.color, clip);
    }

    /// A group (see [`Group`]): `r` is its transformed extent in device pixels, `clip` the area it may draw in.
    #[inline(never)]
    pub(super) fn group(&mut self, frame: &mut Frame, g: &Group, r: &IRect, clip: &IRect) {
        if self.depth >= MAX_NESTED {
            return;
        }
        let flat = self.depth >= MAX_DEPTH;
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
        let alpha = if flat { 255 } else { u32::from(g.opacity) };
        if flat && !same {
            // No layer is left to turn or skew the drawing in: the content is drawn where it was laid out.
            let mut child = self.child(self.dpr, self.sx, self.sy, *clip);
            for it in &g.items {
                child.item(frame, it);
            }
            let st = child.stats;
            self.merge(st);
            return;
        }
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
        // When that part is more than a layer may hold (a big element turned a little), the destination is done in
        // bands of rows, each with a layer of its own: the same pixels, drawn once per band.
        let lin = Lin { ra, rb, rc, rd, det, tx, ty, dpr, alpha };
        let (mut y, mut step) = (dest.y0, dest.y1 - dest.y0);
        while y < dest.y1 {
            let band = IRect { y0: y, y1: (y + step).min(dest.y1), ..dest };
            if self.rotated(frame, g, &lin, &band) || step <= 1 {
                y = band.y1;
            } else {
                step = (step + 1) / 2;
            }
        }
    }

    /// The rows `dest` of the frame shown by the group `g` through the turn, skew or flip `l`. `false`: the layer it
    /// needs is over the limit and nothing was drawn.
    fn rotated(&mut self, frame: &mut Frame, g: &Group, l: &Lin, dest: &IRect) -> bool {
        let Lin { ra, rb, rc, rd, det, tx, ty, dpr, alpha } = *l;
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
        if lay.is_empty() {
            return true;
        }
        let cap = i64::from(frame.width).max(512) * i64::from(frame.height).max(512) * 2;
        if i64::from(w) * i64::from(h) > cap {
            return false;
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
        true
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

/// Add the coverage of a glyph, whose corner is at `at` in the `w` x `h` buffer, to it.
fn accumulate(buf: &mut [u8], w: i32, h: i32, at: (i32, i32), m: &GlyphMask) {
    for (r, line) in m.alpha.chunks_exact((m.width as usize).max(1)).enumerate() {
        let py = at.1 + r as i32;
        if py < 0 || py >= h {
            continue;
        }
        for (c, &a) in line.iter().enumerate() {
            let px = at.0 + c as i32;
            if px >= 0 && px < w {
                let d = &mut buf[py as usize * w as usize + px as usize];
                *d = d.saturating_add(a);
            }
        }
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

/// A glyph mask turned a quarter clockwise about the pen position: the point `(u, v)` (right of the pen, below the baseline)
/// goes to `(-v, u)`.
fn turn_mask(m: &GlyphMask) -> GlyphMask {
    let (w, h) = (m.width as usize, m.height as usize);
    let turn = |src: &[u8], bpp: usize| -> Vec<u8> {
        let mut out = vec![0u8; src.len()];
        for my in 0..h {
            for mx in 0..w {
                let (cx, ry) = (h - 1 - my, mx);
                if let (Some(s), Some(d)) = (src.get((my * w + mx) * bpp..(my * w + mx + 1) * bpp), out.get_mut((ry * h + cx) * bpp..(ry * h + cx + 1) * bpp)) {
                    d.copy_from_slice(s);
                }
            }
        }
        out
    };
    GlyphMask {
        left: -(m.top + m.height as i32),
        top: m.left,
        width: m.height,
        height: m.width,
        alpha: if m.alpha.len() == w * h { turn(&m.alpha, 1) } else { Vec::new() },
        bgra: if m.bgra.len() == w * h * 4 { turn(&m.bgra, 4) } else { Vec::new() },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The mask the long way: the whole window, shaped and blurred.
    fn full(g: &ShadowGeometry) -> Vec<u8> {
        let [ex0, ey0, ex1, ey1] = g.e;
        let shape = RR { x0: ex0 as f32, y0: ey0 as f32, x1: ex1 as f32, y1: ey1 as f32, r: g.r };
        let mut data = shape_buffer(&shape, g.w, g.h, g.inset).unwrap();
        blur_mask(&mut data, g.w as usize, g.h as usize, g.sigma);
        data
    }

    fn geometry(inset: bool, sigma: f32, shape: (i32, i32, i32, i32), r: [(f32, f32); 4], window: (i32, i32)) -> ShadowGeometry {
        ShadowGeometry { inset, w: window.0, h: window.1, e: [shape.0, shape.1, shape.2, shape.3], r, sigma, pad: reach(sigma) }
    }

    /// Reading the 9-patch gives the mask of the whole window: exactly for square corners, within one level for round ones
    /// (the corner arcs are flattened from different coordinates).
    #[test]
    fn a_patch_reads_as_the_whole_mask() {
        let round = [(20.0, 20.0), (30.0, 10.0), (5.0, 5.0), (12.0, 25.0)];
        let square = [(0.0, 0.0); 4];
        let mut cases = Vec::new();
        for (sigma, r) in [(0.0, square), (0.0, round), (2.0, square), (2.0, round), (12.0, square), (12.0, round), (30.0, round)] {
            let pad = reach(sigma);
            // An outer shadow of a 700 x 500 box.
            cases.push(geometry(false, sigma, (pad, pad, pad + 700, pad + 500), r, (700 + 2 * pad, 500 + 2 * pad)));
            // Inset: the hole is the box moved by (7, -5), (-300, 4) (an edge outside the window), or the window is all hole.
            let (w, h) = (640 + 2 * pad, 420 + 2 * pad);
            cases.push(geometry(true, sigma, (pad + 7, pad - 5, pad + 7 + 640, pad - 5 + 420), r, (w, h)));
            cases.push(geometry(true, sigma, (pad - 300, pad + 4, pad - 300 + 640, pad + 4 + 420), r, (w, h)));
            // A narrow one: only the rows are cut.
            cases.push(geometry(false, sigma, (pad, pad, pad + 40, pad + 600), r, (40 + 2 * pad, 600 + 2 * pad)));
        }
        let mut cut = 0;
        for g in &cases {
            let want = full(g);
            let patch = shadow_patch(g).unwrap();
            cut += usize::from(patch.cx.is_some() || patch.cy.is_some());
            let exact = g.r.iter().all(|r| r.0 == 0.0 && r.1 == 0.0);
            let mut worst = 0;
            for y in 0..g.h {
                let mut row = vec![0u8; g.w as usize];
                patch.fill(y, 0, &mut row);
                for (x, &got) in row.iter().enumerate() {
                    let w = want[y as usize * g.w as usize + x];
                    worst = worst.max(got.abs_diff(w));
                    assert_eq!(patch.at(x as i32, y), got);
                }
            }
            assert!(worst <= u8::from(!exact), "sigma {} inset {} edges {:?} radii {:?}: worst difference {worst}", g.sigma, g.inset, g.e, g.r);
        }
        assert!(cut >= cases.len() / 2, "only {cut} of {} cases were cut", cases.len());
    }

    /// The multiply-and-shift mean is the division.
    #[test]
    fn the_mean_is_the_rounded_quotient() {
        for size in (1..=4097u32).step_by(7).chain([4096, 4097]) {
            let mean = Mean::new(size);
            for sum in (0..=255 * size).step_by(97).chain([255 * size, 255 * size - 1]) {
                assert_eq!(u32::from(mean.of(sum)), (sum + size / 2) / size, "size {size} sum {sum}");
            }
        }
    }

    /// A box blur by columns is the one by rows of the turned mask.
    #[test]
    fn the_column_pass_is_the_row_pass() {
        let (w, h) = (37usize, 23usize);
        let src: Vec<u8> = (0..w * h).map(|i| ((i * 7919) % 251) as u8).collect();
        for (l, r) in [(0, 0), (2, 1), (3, 3), (30, 29)] {
            let mut cols = vec![0u8; w * h];
            box_cols(&src, &mut cols, w, h, l, r, &mut vec![0u32; w]);
            for x in 0..w {
                let col: Vec<u8> = (0..h).map(|y| src[y * w + x]).collect();
                let mut want = vec![0u8; h];
                box_line(&col, &mut want, l, r);
                for y in 0..h {
                    assert_eq!(cols[y * w + x], want[y], "l {l} r {r} at ({x}, {y})");
                }
            }
        }
    }
}
