//! The software rasterizer: draws the part of a [`DisplayList`] that is in the viewport into a
//! premultiplied BGRA [`Frame`], in device pixels (CSS px times the device pixel ratio).
//!
//! Rectangle edges are snapped to whole device pixels (each edge rounded on its own, so boxes that touch in
//! CSS px touch in device pixels and borders keep their width). Glyphs are drawn at their rounded baseline
//! and at a horizontal position quantized to a quarter pixel (the platform's mask cache is keyed the same
//! way). Images are drawn 1:1 when the decoded bitmap has exactly the size of the destination rectangle (the
//! normal case: they are decoded at display size) and bilinearly scaled otherwise.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use super::borders::RR;
use super::images::ImageSource;
use super::{BgDraw, DisplayList, Item, ItemKind, NO_CLIP, RRect, Rect, TextRun};
use crate::vector::{FillRule, IRect, Mask};
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
    /// Make this frame `width` x `height` pixels, reusing the allocation when it is big enough. The contents are
    /// not meaningful afterwards (`render_frame` and `render_band` overwrite every pixel).
    pub fn set_size(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.data.resize(width as usize * height as usize * 4, 0);
    }

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

/// Layout units to device pixels, rounded.
fn dev(v: LU, dpr: f64) -> i32 {
    (f64::from(v.raw()) * dpr / 64.0).round().clamp(-1.0e9, 1.0e9) as i32
}

pub(super) fn device_rect(r: &Rect, dpr: f64) -> IRect {
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
pub(super) fn premul(c: Color) -> [u8; 4] {
    let a = u32::from(c.a);
    [div255(u32::from(c.b) * a) as u8, div255(u32::from(c.g) * a) as u8, div255(u32::from(c.r) * a) as u8, c.a]
}

/// Source-over: premultiplied `src` onto premultiplied `dst`.
pub(super) fn over(dst: &mut [u8], src: [u8; 4]) {
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
    /// Coverage of the corner boxes of rounded clips, found when first needed in this frame: (clip, corner, mask, where the
    /// mask's origin is on the frame).
    corner_cache: Vec<(u32, usize, Rc<Mask>, i32, i32)>,
}

/// A corner box up to this many pixels on a side has its coverage kept from frame to frame.
const KEPT_CORNER: i32 = 64;
const KEPT_CORNERS: usize = 512;
/// Corner (0..4), the two radii (bits), box width and height.
type CornerMasks = HashMap<(u8, u32, u32, i32, i32), Rc<Mask>>;

thread_local! {
    /// The coverage of one rounded corner over its corner box, from the top left of the box. It depends on the corner's radii
    /// and box only, not on where the shape is: scrolling moves it by whole pixels, so a page's few distinct radii are
    /// rasterized once instead of once per shape per frame. Key: corner, the two radii (bits), box width and height.
    static CORNERS: RefCell<CornerMasks> = RefCell::new(HashMap::new());
}

/// Draw the viewport into `frame`: the canvas, the fixed items below the flow, the items of the list that touch
/// the band `scroll_y ..` (found through the index), then the other fixed items. The frame is `dpr` times the size
/// of the viewport in CSS px.
pub fn render_frame(list: &DisplayList, platform: &dyn Platform, images: &mut dyn ImageSource, view: View, frame: &mut Frame) -> RasterStats {
    render_band(list, platform, images, view, 0, frame)
}

/// Draw rows `y0 ..` (device pixels from the top of the viewport) of the viewport into `frame`, whose height is the
/// number of rows wanted: a window can draw and copy the viewport a band at a time and so needs a buffer of a band,
/// not of the whole viewport. The pixels are the same as the same rows of [`render_frame`].
pub fn render_band(list: &DisplayList, platform: &dyn Platform, images: &mut dyn ImageSource, view: View, y0: u32, frame: &mut Frame) -> RasterStats {
    let dpr = f64::from(view.dpr).clamp(0.25, 8.0);
    let band = y0.min(1 << 30) as i32;
    let sy = (f64::from(view.scroll_y.max(0.0)) * dpr).round().min(1.0e9) as i32 + band;
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
    let mut cx = Ctx { list, platform, images, dpr, sy: band, bounds, stats: RasterStats::default(), corner_cache: Vec::new() };
    for it in list.fixed.iter().take(list.fixed_below) {
        cx.item(frame, it);
    }
    cx.sy = sy;
    let top_css = view.scroll_y.max(0.0) + (f64::from(band) / dpr) as f32;
    let top = LU::from_f32(top_css);
    let bottom = LU::from_f32(top_css + (h as f32 / dpr as f32) + 1.0);
    let mut idx = Vec::new();
    list.query(top, bottom, &mut idx);
    for &i in &idx {
        if let Some(it) = list.items.get(i as usize) {
            cx.item(frame, it);
        }
    }
    cx.sy = band;
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
        let area = r.intersect(&clip);
        if clip.is_empty() || area.is_empty() {
            return;
        }
        if it.clip == NO_CLIP || self.list.rounds.get(it.clip as usize).is_none_or(Option::is_none) {
            self.draw(frame, it, &r, &clip);
            return;
        }
        // A rounded clip differs from its rectangle only in the corner boxes. Draw the item through the rectangle, then
        // in each corner box that it reaches, take the pixels back towards what was there before by the coverage of the
        // shape: `before + (after - before) * coverage` is the same as drawing the item with its alpha times coverage.
        let mut saved: Vec<(IRect, Vec<u8>, u32, usize, RR)> = Vec::new();
        for (cidx, shape) in self.list.round_shapes(it.clip) {
            let rr = self.device_rr(&shape);
            for (k, cb) in corner_boxes(&rr).into_iter().enumerate() {
                let reg = cb.intersect(&area);
                if !reg.is_empty() {
                    saved.push((reg, copy_region(frame, &reg), cidx, k, rr));
                }
            }
        }
        self.draw(frame, it, &r, &clip);
        for (reg, before, cidx, k, rr) in saved {
            let Some(mi) = self.corner_mask(cidx, k, &rr) else { continue };
            let Some((mask, ox, oy)) = self.corner_cache.get(mi).map(|c| (Rc::clone(&c.2), c.3, c.4)) else { continue };
            let w = (reg.x1 - reg.x0) as usize;
            let stride = frame.width as usize * 4;
            for (ry, y) in (reg.y0..reg.y1).enumerate() {
                let Some(row) = frame.data.get_mut(y as usize * stride..(y as usize + 1) * stride) else { continue };
                for (rx, x) in (reg.x0..reg.x1).enumerate() {
                    let cov = i32::from(mask.at(x - ox, y - oy));
                    if cov == 255 {
                        continue;
                    }
                    for c in 0..4 {
                        let Some(b) = before.get((ry * w + rx) * 4 + c).copied() else { continue };
                        if let Some(d) = row.get_mut(x as usize * 4 + c) {
                            let (b, a) = (i32::from(b), i32::from(*d));
                            *d = (b + ((a - b) * cov + if a >= b { 127 } else { -127 }) / 255).clamp(0, 255) as u8;
                        }
                    }
                }
            }
        }
    }

    fn draw(&mut self, frame: &mut Frame, it: &Item, r: &IRect, clip: &IRect) {
        match it.kind {
            ItemKind::Rect(color) => {
                let d = r.intersect(clip);
                if !d.is_empty() {
                    fill(frame, &d, premul(color));
                    self.stats.items += 1;
                }
            }
            ItemKind::Text(run) => {
                if r.intersect(clip).is_empty() {
                    return;
                }
                self.stats.items += 1;
                self.text(frame, &run, clip);
            }
            ItemKind::Image { node } => {
                let d = r.intersect(clip);
                if d.is_empty() {
                    return;
                }
                let (dw, dh) = ((r.x1 - r.x0).max(0) as u32, (r.y1 - r.y0).max(0) as u32);
                if let Some(img) = self.images.bitmap(node, dw, dh) {
                    self.stats.images += 1;
                    draw_image(frame, &img, r, &d);
                }
            }
            ItemKind::Bg(i) => {
                let d = r.intersect(clip);
                if let Some(b) = self.list.bgs.get(i as usize).copied()
                    && !d.is_empty()
                {
                    self.stats.items += 1;
                    self.bg(frame, &b, &d);
                }
            }
            ItemKind::Border(i) => {
                if let Some(b) = self.list.borders.get(i as usize) {
                    self.stats.items += 1;
                    super::borders::draw_border(frame, b, self.dpr, self.sy, clip);
                }
            }
        }
    }

    /// A rounded shape of the clip chain in device pixels.
    fn device_rr(&self, s: &RRect) -> RR {
        let d = device_rect(&s.rect, self.dpr);
        let k = self.dpr as f32;
        RR { x0: d.x0 as f32, y0: (d.y0 - self.sy) as f32, x1: d.x1 as f32, y1: (d.y1 - self.sy) as f32, r: s.radii.map(|(a, b)| (a * k, b * k)) }.fit()
    }

    /// The coverage of corner `k` of the shape of clip `cidx` over its corner box (made once per frame).
    fn corner_mask(&mut self, cidx: u32, k: usize, rr: &RR) -> Option<usize> {
        if let Some(i) = self.corner_cache.iter().position(|c| c.0 == cidx && c.1 == k) {
            return Some(i);
        }
        let cb = *corner_boxes(rr).get(k)?;
        let (bw, bh) = (cb.x1 - cb.x0, cb.y1 - cb.y0);
        let kept = bw <= KEPT_CORNER && bh <= KEPT_CORNER;
        let entry = if kept {
            let radius = rr.r.get(k).copied().unwrap_or((0.0, 0.0));
            let key = (k as u8, radius.0.to_bits(), radius.1.to_bits(), bw, bh);
            let found = CORNERS.with(|c| c.borrow().get(&key).cloned());
            let mask = match found {
                Some(m) => m,
                None => {
                    // The box itself as a rectangle with only this corner rounded.
                    let mut r = [(0.0, 0.0); 4];
                    if let Some(dst) = r.get_mut(k) {
                        *dst = radius;
                    }
                    let local = RR { x0: 0.0, y0: 0.0, x1: bw as f32, y1: bh as f32, r };
                    let m = Rc::new(Mask::from_polys(&local.polys(), FillRule::NonZero, IRect { x0: 0, y0: 0, x1: bw, y1: bh })?);
                    CORNERS.with(|c| {
                        let mut c = c.borrow_mut();
                        if c.len() >= KEPT_CORNERS {
                            c.clear();
                        }
                        c.insert(key, Rc::clone(&m));
                    });
                    m
                }
            };
            (cidx, k, mask, cb.x0, cb.y0)
        } else {
            let win = cb.intersect(&self.bounds);
            (cidx, k, Rc::new(Mask::from_polys(&rr.polys(), FillRule::NonZero, win)?), 0, 0)
        };
        self.corner_cache.push(entry);
        Some(self.corner_cache.len() - 1)
    }

    /// One background layer: tiles of `b.tile` CSS px from `b.origin`, `b.step` apart, clipped to `area`.
    fn bg(&mut self, frame: &mut Frame, b: &BgDraw, area: &IRect) {
        let dpr = self.dpr;
        let tile = Rect::new(LU::ZERO, LU::ZERO, LU::from_f32(b.tile.0), LU::from_f32(b.tile.1));
        let (tw, th) = device_size(&tile, dpr as f32);
        let (tw, th) = (tw as i32, th as i32);
        if tw == 0 || th == 0 {
            return;
        }
        let list = self.list;
        let grad = list.grads.get(b.grad as usize);
        let bmp = if grad.is_none() { self.images.bitmap(b.node, tw as u32, th as u32) } else { None };
        if grad.is_none() && bmp.is_none() {
            return;
        }
        self.stats.images += usize::from(bmp.is_some());
        let ox = (f64::from(b.origin.0) * dpr).round().clamp(-1.0e9, 1.0e9) as i32;
        let oy = (f64::from(b.origin.1) * dpr).round().clamp(-1.0e9, 1.0e9) as i32 - self.sy;
        let step = |s: f32| (s > 0.0).then(|| f64::from(s) * dpr);
        let (stx, sty) = (step(b.step.0), step(b.step.1));
        // Start of tile `k` along an axis with origin `o` and step `s`.
        let start_of = |o: i32, s: Option<f64>, k: i64| o + s.map_or(0, |s| (k as f64 * s).round() as i32);
        let stride = frame.width as usize * 4;
        let (cw, ch) = (b.tile.0 / tw as f32, b.tile.1 / th as f32);
        for y in area.y0..area.y1 {
            // The tile row y is in, and the row inside the tile.
            let ky = sty.map_or(0, |s| ((f64::from(y - oy)) / s).floor() as i64);
            let Some(ly) = [ky, ky + 1, ky - 1].into_iter().filter(|k| sty.is_some() || *k == 0).find_map(|k| {
                let l = y - start_of(oy, sty, k);
                (0..th).contains(&l).then_some(l)
            }) else {
                continue;
            };
            let Some(row) = frame.data.get_mut(y as usize * stride..(y as usize + 1) * stride) else { continue };
            let mut kx = stx.map_or(0, |s| ((f64::from(area.x0 - ox)) / s).floor() as i64 - 1);
            for _ in 0..(1 << 14) {
                let start = start_of(ox, stx, kx);
                if start >= area.x1 {
                    break;
                }
                let (a, e) = (start.max(area.x0), (start + tw).min(area.x1));
                if let Some(px) = (a < e).then(|| row.get_mut(a as usize * 4..e as usize * 4)).flatten() {
                    let v = (ly as f32 + 0.5) * ch;
                    for (i, p) in px.as_chunks_mut::<4>().0.iter_mut().enumerate() {
                        let lx = a - start + i as i32;
                        let src = match (&grad, &bmp) {
                            (Some(g), _) => g.at((lx as f32 + 0.5) * cw, v),
                            (None, Some(img)) => {
                                let (sw, sh) = (img.width as usize, img.height as usize);
                                let (sx, sy) = (lx as usize * sw / tw as usize, ly as usize * sh / th as usize);
                                match img.bgra.get((sy * sw + sx) * 4..(sy * sw + sx) * 4 + 4).and_then(|q| <[u8; 4]>::try_from(q).ok()) {
                                    Some(q) => q,
                                    None => continue,
                                }
                            }
                            _ => continue,
                        };
                        over(p, src);
                    }
                }
                if stx.is_none() {
                    break;
                }
                kx += 1;
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
            let Some(mask) = self.platform.glyph_mask(FontId(u32::from(g.font)), u32::from(g.glyph), size, sub) else { continue };
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

/// The four boxes the corners of `rr` can differ from a rectangle in (top-left, top-right, bottom-right, bottom-left);
/// empty for a square corner.
fn corner_boxes(rr: &RR) -> [IRect; 4] {
    let [tl, tr, br, bl] = rr.r;
    let b = |x0: f32, y0: f32, x1: f32, y1: f32, r: (f32, f32)| {
        if r.0 > 0.0 && r.1 > 0.0 { IRect { x0: x0.floor() as i32, y0: y0.floor() as i32, x1: x1.ceil() as i32, y1: y1.ceil() as i32 } } else { IRect { x0: 0, y0: 0, x1: 0, y1: 0 } }
    };
    [
        b(rr.x0, rr.y0, rr.x0 + tl.0, rr.y0 + tl.1, tl),
        b(rr.x1 - tr.0, rr.y0, rr.x1, rr.y0 + tr.1, tr),
        b(rr.x1 - br.0, rr.y1 - br.1, rr.x1, rr.y1, br),
        b(rr.x0, rr.y1 - bl.1, rr.x0 + bl.0, rr.y1, bl),
    ]
}

/// The pixels of `reg` (inside the frame), row by row.
fn copy_region(frame: &Frame, reg: &IRect) -> Vec<u8> {
    let stride = frame.width as usize * 4;
    let mut out = Vec::with_capacity((reg.x1 - reg.x0).max(0) as usize * (reg.y1 - reg.y0).max(0) as usize * 4);
    for y in reg.y0..reg.y1 {
        match frame.data.get(y.max(0) as usize * stride + reg.x0.max(0) as usize * 4..y.max(0) as usize * stride + reg.x1.max(0) as usize * 4) {
            Some(row) => out.extend_from_slice(row),
            None => out.resize(out.len() + (reg.x1 - reg.x0).max(0) as usize * 4, 0),
        }
    }
    out
}
