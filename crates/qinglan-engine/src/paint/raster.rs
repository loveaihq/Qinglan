//! The software rasterizer: draws the part of a [`DisplayList`] that is in the viewport into a
//! premultiplied BGRA [`Frame`], in device pixels (CSS px times the device pixel ratio).
//!
//! Rectangle edges are snapped to whole device pixels (each edge rounded on its own, so boxes that touch in
//! CSS px touch in device pixels and borders keep their width). Glyphs are drawn at their rounded baseline
//! and at a horizontal position quantized to a quarter pixel (the platform's mask cache is keyed the same
//! way). Images are drawn 1:1 when the decoded bitmap has exactly the size of the destination rectangle (the
//! normal case: they are decoded at display size) and bilinearly scaled otherwise.

use std::sync::Arc;

use super::images::ImageSource;
use super::{DisplayList, Item, ItemKind, NO_CLIP, Rect, TextRun};
use crate::css::color::Color;
use crate::layout::units::LayoutUnit as LU;
use crate::platform::image::DecodedImage;
use crate::platform::{FontId, Platform};

/// A pixel buffer: premultiplied BGRA, `width * 4` bytes per row, top row first.
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl Frame {
    /// A transparent black frame. Sizes are the caller's to bound (`render --full` caps them).
    pub fn new(width: u32, height: u32) -> Frame {
        Frame { width, height, data: vec![0; width as usize * height as usize * 4] }
    }

    /// The pixel at (`x`, `y`) as `[b, g, r, a]`, `None` outside the frame.
    pub fn pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let i = (y as usize * self.width as usize + x as usize) * 4;
        self.data.get(i..i + 4).and_then(|p| p.try_into().ok())
    }
}

/// What a frame cost, for the benchmarks.
#[derive(Clone, Copy, Debug, Default)]
pub struct RasterStats {
    pub items: usize,
    pub glyphs: usize,
    pub images: usize,
}

/// An integer rectangle `x0..x1` x `y0..y1`.
#[derive(Clone, Copy, Debug)]
struct IRect {
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
}

impl IRect {
    fn intersect(&self, o: &IRect) -> IRect {
        IRect { x0: self.x0.max(o.x0), y0: self.y0.max(o.y0), x1: self.x1.min(o.x1), y1: self.y1.min(o.y1) }
    }
    fn is_empty(&self) -> bool {
        self.x1 <= self.x0 || self.y1 <= self.y0
    }
}

/// Layout units to device pixels, rounded.
fn dev(v: LU, dpr: f64) -> i32 {
    (f64::from(v.raw()) * dpr / 64.0).round().clamp(-1.0e9, 1.0e9) as i32
}

fn device_rect(r: &Rect, dpr: f64) -> IRect {
    IRect { x0: dev(r.x, dpr), y0: dev(r.y, dpr), x1: dev(r.right(), dpr), y1: dev(r.bottom(), dpr) }
}

/// The size in device pixels an item with page rectangle `r` is drawn at (what images are decoded to).
pub fn device_size(r: &Rect, dpr: f32) -> (u32, u32) {
    let d = device_rect(r, f64::from(dpr));
    ((d.x1 - d.x0).max(0) as u32, (d.y1 - d.y0).max(0) as u32)
}

fn div255(v: u32) -> u32 {
    (v + 128 + ((v + 128) >> 8)) >> 8
}

/// `c` as premultiplied BGRA.
fn premul(c: Color) -> [u8; 4] {
    let a = u32::from(c.a);
    [div255(u32::from(c.b) * a) as u8, div255(u32::from(c.g) * a) as u8, div255(u32::from(c.r) * a) as u8, c.a]
}

/// Source-over: premultiplied `src` onto premultiplied `dst`.
fn over(dst: &mut [u8], src: [u8; 4]) {
    if src[3] == 255 {
        if let Some(d) = dst.get_mut(..4) {
            d.copy_from_slice(&src);
        }
        return;
    }
    let inv = 255 - u32::from(src[3]);
    for (d, s) in dst.iter_mut().zip(src) {
        *d = (u32::from(s) + div255(u32::from(*d) * inv)).min(255) as u8;
    }
}

/// Which part of the page to draw: `scroll_y` in CSS px (rounded to a device pixel), at `dpr` device pixels per CSS px.
#[derive(Clone, Copy, Debug)]
pub struct View {
    pub scroll_y: f32,
    pub dpr: f32,
}

struct Ctx<'a> {
    list: &'a DisplayList,
    platform: &'a dyn Platform,
    images: &'a mut dyn ImageSource,
    dpr: f64,
    /// Device pixels the page is scrolled by.
    sy: i32,
    bounds: IRect,
    stats: RasterStats,
}

/// Draw the viewport into `frame`: the canvas, the fixed items below the flow, the items of the list that touch
/// the band `scroll_y ..` (found through the index), then the other fixed items. The frame is `dpr` times the size
/// of the viewport in CSS px.
pub fn render_frame(list: &DisplayList, platform: &dyn Platform, images: &mut dyn ImageSource, view: View, frame: &mut Frame) -> RasterStats {
    let dpr = f64::from(view.dpr).clamp(0.25, 8.0);
    let sy = (f64::from(view.scroll_y.max(0.0)) * dpr).round().min(1.0e9) as i32;
    let (w, h) = (frame.width as i32, frame.height as i32);
    let bounds = IRect { x0: 0, y0: 0, x1: w, y1: h };
    let canvas = list.canvas.unwrap_or(Color::WHITE);
    // A translucent canvas colour lands on white, the colour of an unpainted page.
    let base = premul(Color::WHITE);
    let c = premul(canvas);
    let mut px = base;
    over(&mut px, c);
    for p in frame.data.as_chunks_mut::<4>().0 {
        *p = px;
    }
    let mut cx = Ctx { list, platform, images, dpr, sy: 0, bounds, stats: RasterStats::default() };
    for it in list.fixed.iter().take(list.fixed_below) {
        cx.item(frame, it);
    }
    cx.sy = sy;
    let top = LU::from_f32(view.scroll_y.max(0.0));
    let bottom = LU::from_f32(view.scroll_y.max(0.0) + (h as f32 / dpr as f32) + 1.0);
    let mut idx = Vec::new();
    list.query(top, bottom, &mut idx);
    for &i in &idx {
        if let Some(it) = list.items.get(i as usize) {
            cx.item(frame, it);
        }
    }
    cx.sy = 0;
    for it in list.fixed.iter().skip(list.fixed_below) {
        cx.item(frame, it);
    }
    cx.stats
}

impl Ctx<'_> {
    fn item(&mut self, frame: &mut Frame, it: &Item) {
        let mut r = device_rect(&it.bounds, self.dpr);
        r.y0 -= self.sy;
        r.y1 -= self.sy;
        let mut clip = self.bounds;
        if it.clip != NO_CLIP {
            let Some(c) = self.list.clip_rect(it.clip) else { return };
            let mut c = device_rect(&c, self.dpr);
            c.y0 -= self.sy;
            c.y1 -= self.sy;
            clip = clip.intersect(&c);
        }
        if clip.is_empty() {
            return;
        }
        match it.kind {
            ItemKind::Rect(color) => {
                let d = r.intersect(&clip);
                if !d.is_empty() {
                    fill(frame, &d, premul(color));
                    self.stats.items += 1;
                }
            }
            ItemKind::Text(run) => {
                if r.intersect(&clip).is_empty() {
                    return;
                }
                self.stats.items += 1;
                self.text(frame, &run, &clip);
            }
            ItemKind::Image { node } => {
                let d = r.intersect(&clip);
                if d.is_empty() {
                    return;
                }
                let (dw, dh) = ((r.x1 - r.x0).max(0) as u32, (r.y1 - r.y0).max(0) as u32);
                if let Some(img) = self.images.bitmap(node, dw, dh) {
                    self.stats.images += 1;
                    draw_image(frame, &img, &r, &d);
                }
            }
        }
    }

    fn text(&mut self, frame: &mut Frame, run: &TextRun, clip: &IRect) {
        let Some(glyphs) = self.list.glyphs.get(run.glyphs.0 as usize..run.glyphs.1 as usize) else { return };
        let dpr = self.dpr as f32;
        let size = run.size * dpr;
        let color = premul(run.color);
        let mut pen = run.x.to_f32() * dpr;
        let base = dev(run.baseline, self.dpr) - self.sy;
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
            let Some(mask) = self.platform.glyph_mask(FontId(g.font), g.glyph, size, sub) else { continue };
            let x = ix as i32 + mask.left;
            let y = base + (g.dy * dpr).round() as i32 + mask.top;
            let area = IRect { x0: x, y0: y, x1: x + mask.width as i32, y1: y + mask.height as i32 }.intersect(clip);
            if area.is_empty() || mask.width == 0 {
                continue;
            }
            self.stats.glyphs += 1;
            blend_mask(frame, &mask.alpha, mask.width as usize, (x, y), &area, color);
        }
    }
}

/// Rows `d.y0..d.y1` of `frame` with the pixels `d.x0..d.x1` of each, as byte slices.
fn rows_mut<'a>(frame: &'a mut Frame, d: &IRect) -> impl Iterator<Item = (i32, &'a mut [u8])> {
    let stride = frame.width as usize * 4;
    let (x0, x1) = (d.x0.max(0) as usize * 4, d.x1.max(0) as usize * 4);
    let (y0, y1) = (d.y0.max(0), d.y1.max(0));
    frame
        .data
        .chunks_exact_mut(stride.max(4))
        .enumerate()
        .skip(y0 as usize)
        .take((y1 - y0) as usize)
        .filter_map(move |(y, row)| row.get_mut(x0..x1).map(|r| (y as i32, r)))
}

fn fill(frame: &mut Frame, d: &IRect, px: [u8; 4]) {
    for (_, row) in rows_mut(frame, d) {
        if px[3] == 255 {
            for p in row.as_chunks_mut::<4>().0 {
                *p = px;
            }
        } else {
            for p in row.as_chunks_mut::<4>().0 {
                over(p, px);
            }
        }
    }
}

/// Blend a coverage mask (`mw` pixels per row, its top left corner at `origin`) in `color` (premultiplied) into `area`.
fn blend_mask(frame: &mut Frame, alpha: &[u8], mw: usize, origin: (i32, i32), area: &IRect, color: [u8; 4]) {
    let col0 = (area.x0 - origin.0).max(0) as usize;
    for (y, row) in rows_mut(frame, area) {
        let my = (y - origin.1).max(0) as usize;
        let Some(mrow) = alpha.get(my * mw..(my + 1) * mw) else { continue };
        let Some(cov) = mrow.get(col0..) else { continue };
        for (p, &c) in row.as_chunks_mut::<4>().0.iter_mut().zip(cov) {
            match c {
                0 => {}
                255 => over(p, color),
                c => {
                    let c = u32::from(c);
                    let s = [div255(u32::from(color[0]) * c) as u8, div255(u32::from(color[1]) * c) as u8, div255(u32::from(color[2]) * c) as u8, div255(u32::from(color[3]) * c) as u8];
                    over(p, s);
                }
            }
        }
    }
}

/// Draw `img` stretched over `dst` (the whole rectangle, possibly partly outside the clip), only inside `area`.
fn draw_image(frame: &mut Frame, img: &Arc<DecodedImage>, dst: &IRect, area: &IRect) {
    let (sw, sh) = (img.width as usize, img.height as usize);
    let (dw, dh) = ((dst.x1 - dst.x0).max(1) as usize, (dst.y1 - dst.y0).max(1) as usize);
    if sw == 0 || sh == 0 || img.bgra.len() < sw * sh * 4 {
        return;
    }
    let src_row = |y: usize| img.bgra.get(y * sw * 4..(y + 1) * sw * 4);
    if (sw, sh) == (dw, dh) {
        for (y, row) in rows_mut(frame, area) {
            let Some(s) = src_row((y - dst.y0) as usize) else { continue };
            let Some(s) = s.get((area.x0 - dst.x0) as usize * 4..) else { continue };
            for (p, q) in row.as_chunks_mut::<4>().0.iter_mut().zip(s.as_chunks::<4>().0) {
                over(p, *q);
            }
        }
        return;
    }
    // Bilinear: per destination column the two source columns and the weight of the second (0..=256).
    let kx = sw as f32 / dw as f32;
    let ky = sh as f32 / dh as f32;
    let cols: Vec<(usize, usize, u32)> = (area.x0..area.x1)
        .map(|x| {
            let fx = ((x - dst.x0) as f32 + 0.5) * kx - 0.5;
            let fx = fx.clamp(0.0, (sw - 1) as f32);
            let x0 = fx.floor() as usize;
            (x0, (x0 + 1).min(sw - 1), ((fx - x0 as f32) * 256.0) as u32)
        })
        .collect();
    for (y, row) in rows_mut(frame, area) {
        let fy = (((y - dst.y0) as f32 + 0.5) * ky - 0.5).clamp(0.0, (sh - 1) as f32);
        let y0 = fy.floor() as usize;
        let wy = ((fy - y0 as f32) * 256.0) as u32;
        let (Some(r0), Some(r1)) = (src_row(y0), src_row((y0 + 1).min(sh - 1))) else { continue };
        for (p, &(xa, xb, wx)) in row.as_chunks_mut::<4>().0.iter_mut().zip(&cols) {
            let mut out = [0u8; 4];
            for (k, o) in out.iter_mut().enumerate() {
                let at = |r: &[u8], x: usize| r.get(x * 4 + k).copied().map_or(0, u32::from);
                let top = at(r0, xa) * (256 - wx) + at(r0, xb) * wx;
                let bot = at(r1, xa) * (256 - wx) + at(r1, xb) * wx;
                *o = ((top * (256 - wy) + bot * wy) >> 16).min(255) as u8;
            }
            over(p, out);
        }
    }
}
