//! The scene renderer: draws a [`Scene`] into a premultiplied BGRA canvas (SVG 2 painting 13, pservers 13.2-13.3).
//!
//! Shapes go through [`crate::vector`]: the fill is the exact-coverage rasterizer on the path mapped to device space; the
//! stroke is built in the shape's own space (so non-uniform scales stretch the pen like they should) and mapped. Gradients
//! are found per pixel through the inverse of their matrix. Clips are 8-bit coverage planes the size of the canvas, opacity
//! groups are extra canvases (few, and counted). A pixel budget stops hostile scenes (a thousand full-canvas fills).

use crate::fmath::Fclamp;
use std::rc::Rc;

use super::build::path_bounds;
use super::{Family, Fill, Grad, Op, Paint, Par, Scene, Shape, TextOp, view_transform};
use crate::css::types::FontFamily;
use crate::css::color::Color;
use crate::platform::{FontQuery, Platform};
use crate::vector::{FillRule, IRect, Poly, Pt, Transform, fill_polys, stroke_polys};

/// Canvas pixels (16 Mpx = 64 MB of BGRA).
pub const MAX_AREA: u64 = 16_000_000;
/// Pixel visits per render, gradients counting three.
const MAX_WORK: u64 = 240_000_000;
/// Opacity groups open at once (each is a canvas).
const MAX_LAYERS: usize = 5;
const MAX_MASKS: usize = 6;
const LUT: usize = 512;
/// Flattening tolerance in device pixels.
const TOL: f32 = 0.05;

pub struct Canvas {
    pub w: u32,
    pub h: u32,
    pub data: Vec<u8>,
}

impl Canvas {
    fn new(w: u32, h: u32) -> Canvas {
        Canvas { w, h, data: vec![0; w as usize * h as usize * 4] }
    }
}

fn div255(v: u32) -> u32 {
    (v + 128 + ((v + 128) >> 8)) >> 8
}

fn premul(c: Color, alpha: u32) -> [u8; 4] {
    let a = div255(u32::from(c.a) * alpha);
    [div255(u32::from(c.b) * a) as u8, div255(u32::from(c.g) * a) as u8, div255(u32::from(c.r) * a) as u8, a as u8]
}

fn scaled(s: [u8; 4], k: u32) -> [u8; 4] {
    [div255(u32::from(s[0]) * k) as u8, div255(u32::from(s[1]) * k) as u8, div255(u32::from(s[2]) * k) as u8, div255(u32::from(s[3]) * k) as u8]
}

/// Source-over of a premultiplied pixel.
fn over(d: &mut [u8], s: [u8; 4]) {
    if s[3] == 255 {
        d.copy_from_slice(&s);
        return;
    }
    let inv = 255 - u32::from(s[3]);
    for (d, s) in d.iter_mut().zip(s) {
        *d = (u32::from(s) + div255(u32::from(*d) * inv)).min(255) as u8;
    }
}

struct Layer {
    canvas: Canvas,
    alpha: u8,
}

struct R<'a> {
    sc: &'a Scene,
    platform: Option<&'a dyn Platform>,
    w: u32,
    h: u32,
    root: Transform,
    layers: Vec<Layer>,
    skipped: u32,
    work: u64,
    masks: Vec<(u32, Rc<Vec<u8>>)>,
    luts: Vec<Option<Rc<Vec<[u8; 4]>>>>,
    /// The opacity (0..=255) the text drawn now gets from the groups it is in, when only the text is drawn ([`overlay_text`]).
    gain: u32,
}

/// The transform from the user space of the outermost svg to a `w` x `h` canvas: the viewBox mapping; without a viewBox a user
/// unit is `scale` pixels, or the scene's natural size is stretched to fit.
pub fn root_transform(sc: &Scene, w: f32, h: f32, scale: Option<f32>) -> Transform {
    let (nw, nh) = sc.intrinsic.object_size();
    sc.outer.compose(&match sc.intrinsic.vb {
        Some(vb) => view_transform(vb, sc.intrinsic.par.unwrap_or(Par::DEFAULT), w, h),
        None => match scale {
            Some(s) => Transform::scale(s, s),
            None => Transform::scale(w / nw.max(1.0e-3), h / nh.max(1.0e-3)),
        },
    })
}

/// Draw `sc` onto a `w` x `h` canvas. A scene with a viewBox is mapped by it and `preserveAspectRatio`; without one a user unit
/// is `scale` pixels, or the canvas is the scene's natural size stretched to fit. `None` when the canvas is too big.
pub fn render(sc: &Scene, w: u32, h: u32, platform: Option<&dyn Platform>, scale: Option<f32>) -> Option<Canvas> {
    if w == 0 || h == 0 || u64::from(w) * u64::from(h) > MAX_AREA {
        return Some(Canvas::new(w.min(1), h.min(1)));
    }
    let root = root_transform(sc, w as f32, h as f32, scale);
    let mut r = R {
        sc,
        platform,
        w,
        h,
        root,
        layers: vec![Layer { canvas: Canvas::new(w, h), alpha: 255 }],
        skipped: 0,
        work: 0,
        masks: Vec::new(),
        luts: vec![None; sc.grads.len()],
        gain: 255,
    };
    if root.a.is_finite() && root.d.is_finite() && root.e.is_finite() && root.f.is_finite() && root.b.is_finite() && root.c.is_finite() {
        for op in &sc.ops {
            if r.work > MAX_WORK {
                break;
            }
            match op {
                Op::Shape(s) => r.shape(s),
                Op::Text(t) => r.text(t),
                Op::Begin(a) => r.begin(*a),
                Op::End => r.end(),
            }
        }
    }
    while r.layers.len() > 1 {
        r.end_layer();
    }
    r.layers.pop().map(|l| l.canvas)
}

/// Draw only the text of `sc` onto `canvas`, which holds the rest of the picture (drawn by [`render`] without a platform, at
/// this size and without a `scale`). Each text gets the opacity of the groups it is in; it lands on top of the shapes.
pub fn overlay_text(sc: &Scene, canvas: Canvas, platform: &dyn Platform) -> Canvas {
    let (w, h) = (canvas.w, canvas.h);
    let root = root_transform(sc, w as f32, h as f32, None);
    let mut r = R { sc, platform: Some(platform), w, h, root, layers: vec![Layer { canvas, alpha: 255 }], skipped: 0, work: 0, masks: Vec::new(), luts: Vec::new(), gain: 255 };
    if [root.a, root.b, root.c, root.d, root.e, root.f].iter().all(|v| v.is_finite()) {
        let mut groups: Vec<u32> = Vec::new();
        for op in &sc.ops {
            if r.work > MAX_WORK {
                break;
            }
            match op {
                Op::Begin(a) => groups.push(u32::from(*a)),
                Op::End => {
                    groups.pop();
                }
                Op::Text(t) => {
                    r.gain = groups.iter().fold(255, |g, a| div255(g * a));
                    r.text(t);
                }
                Op::Shape(_) => {}
            }
        }
    }
    r.layers.pop().map_or_else(|| Canvas::new(w.min(1), h.min(1)), |l| l.canvas)
}

impl R<'_> {
    fn begin(&mut self, alpha: u8) {
        if self.layers.len() >= MAX_LAYERS || self.skipped > 0 {
            self.skipped += 1;
            return;
        }
        self.layers.push(Layer { canvas: Canvas::new(self.w, self.h), alpha });
        self.work += u64::from(self.w) * u64::from(self.h) / 8;
    }

    fn end(&mut self) {
        if self.skipped > 0 {
            self.skipped -= 1;
        } else {
            self.end_layer();
        }
    }

    /// Composite the top layer onto the one below with its alpha.
    fn end_layer(&mut self) {
        let Some(top) = self.layers.pop() else { return };
        let Some(below) = self.layers.last_mut() else {
            self.layers.push(top);
            return;
        };
        let k = u32::from(top.alpha);
        for (d, s) in below.canvas.data.as_chunks_mut::<4>().0.iter_mut().zip(top.canvas.data.as_chunks::<4>().0) {
            if s[3] != 0 {
                over(d, if k == 255 { *s } else { scaled(*s, k) });
            }
        }
        self.work += u64::from(self.w) * u64::from(self.h) / 8;
    }

    fn clip_mask(&mut self, idx: u32) -> Option<Rc<Vec<u8>>> {
        if let Some((_, m)) = self.masks.iter().find(|(i, _)| *i == idx) {
            return Some(m.clone());
        }
        let sc = self.sc;
        let clip = sc.clips.get(idx as usize - 1)?;
        let (w, h) = (self.w as usize, self.h as usize);
        let win = IRect { x0: 0, y0: 0, x1: self.w as i32, y1: self.h as i32 };
        let mut cov = vec![0u8; w * h];
        let mut work = 0u64;
        for (path, ctm, rule) in &clip.parts {
            let m = self.root.compose(ctm);
            let polys = path.transformed(&m).flatten(TOL);
            fill_polys(&polys, *rule, win, &mut |y, x0, c| {
                work += c.len() as u64;
                let at = y as usize * w + x0 as usize;
                if let Some(dst) = cov.get_mut(at..at + c.len()) {
                    for (d, s) in dst.iter_mut().zip(c) {
                        *d = (*d).max(*s);
                    }
                }
            });
        }
        if clip.next != 0
            && let Some(outer) = self.clip_mask(clip.next)
        {
            for (d, o) in cov.iter_mut().zip(outer.iter()) {
                *d = div255(u32::from(*d) * u32::from(*o)) as u8;
            }
        }
        self.work += work + (w * h) as u64 / 4;
        let m = Rc::new(cov);
        if self.masks.len() >= MAX_MASKS {
            self.masks.remove(0);
        }
        self.masks.push((idx, m.clone()));
        Some(m)
    }

    fn lut(&mut self, i: usize) -> Option<Rc<Vec<[u8; 4]>>> {
        if let Some(Some(l)) = self.luts.get(i) {
            return Some(l.clone());
        }
        let g = self.sc.grads.get(i)?;
        let l = Rc::new(build_lut(g));
        *self.luts.get_mut(i)? = Some(l.clone());
        Some(l)
    }

    fn shape(&mut self, s: &Shape) {
        let m = self.root.compose(&s.ctm);
        if ![m.a, m.b, m.c, m.d, m.e, m.f].iter().all(|v| v.is_finite()) {
            return;
        }
        let clip = if s.clip != 0 { self.clip_mask(s.clip) } else { None };
        if s.clip != 0 && clip.is_none() {
            return;
        }
        let bbox = || path_bounds(&s.path, &Transform::IDENTITY);
        if let Some(f) = &s.fill {
            let polys = s.path.transformed(&m).flatten(TOL);
            self.paint(&polys, s.rule, f, &m, &bbox, clip.as_deref());
        }
        if let Some((f, st)) = &s.stroke {
            let k = m.scale_factor();
            if !(k.is_finite() && k > 1.0e-9) {
                return;
            }
            let local = s.path.flatten(TOL / k);
            let outline = stroke_polys(&local, st, TOL / k);
            let dev: Vec<Poly> = outline.iter().map(|p| Poly { pts: p.pts.iter().map(|q| m.apply(*q)).collect(), closed: true }).collect();
            self.paint(&dev, FillRule::NonZero, f, &m, &bbox, clip.as_deref());
        }
    }

    fn paint(&mut self, polys: &[Poly], rule: FillRule, f: &Fill, m: &Transform, bbox: &dyn Fn() -> Option<[f32; 4]>, clip: Option<&Vec<u8>>) {
        let win = IRect { x0: 0, y0: 0, x1: self.w as i32, y1: self.h as i32 };
        let w = self.w as usize;
        let grad = match f.paint {
            Paint::Grad(i) => {
                let Some(g) = self.sc.grads.get(i as usize) else { return };
                let Some(lut) = self.lut(i as usize) else { return };
                let mut gm = *m;
                if g.bbox {
                    let Some(b) = bbox().filter(|b| b[2] > 0.0 && b[3] > 0.0) else { return };
                    gm = gm.compose(&Transform { a: b[2], b: 0.0, c: 0.0, d: b[3], e: b[0], f: b[1] });
                }
                let gm = gm.compose(&g.xf);
                let inv = gm.inverse();
                if !inv.a.is_finite() {
                    return;
                }
                Some(GradEval::new(g, inv, lut))
            }
            Paint::Color(_) => None,
        };
        let solid = match f.paint {
            Paint::Color(c) => premul(c, u32::from(f.alpha)),
            Paint::Grad(_) => [0; 4],
        };
        let galpha = u32::from(f.alpha);
        let mut work = 0u64;
        let unit = if grad.is_some() { 3 } else { 1 };
        let Some(layer) = self.layers.last_mut() else { return };
        let data = &mut layer.canvas.data;
        fill_polys(polys, rule, win, &mut |y, x0, cov| {
            work += cov.len() as u64 * unit;
            let base = (y as usize * w + x0 as usize) * 4;
            let Some(row) = data.get_mut(base..base + cov.len() * 4) else { return };
            for (i, (px, &cv)) in row.as_chunks_mut::<4>().0.iter_mut().zip(cov).enumerate() {
                let mut k = u32::from(cv);
                if let Some(c) = clip {
                    k = div255(k * u32::from(c.get(y as usize * w + x0 as usize + i).copied().unwrap_or(0)));
                }
                if k == 0 {
                    continue;
                }
                let src = match &grad {
                    Some(g) => scaled(g.at((x0 as usize + i) as f32, y as f32), galpha),
                    None => solid,
                };
                over(px, if k == 255 { src } else { scaled(src, k) });
            }
        });
        self.work += work;
    }

    fn text(&mut self, t: &TextOp) {
        let Some(pl) = self.platform else { return };
        let m = self.root.compose(&t.ctm);
        let k = m.scale_factor();
        if !(k.is_finite() && k > 1.0e-6) {
            return;
        }
        let clip = if t.clip != 0 { self.clip_mask(t.clip) } else { None };
        // (font, glyph, x from the chunk start, y from the baseline, run) in device pixels.
        let mut glyphs = Vec::new();
        let (mut pen, mut dy) = (0.0f32, 0.0f32);
        for (ri, run) in t.runs.iter().enumerate() {
            pen += run.dx * k;
            dy += run.dy * k;
            let families: Vec<FontFamily> = run.families.iter().map(Family::font).collect();
            let q = FontQuery { families: &families, weight: run.weight, slant: run.slant, lang: None };
            for g in pl.shape(&run.text, &q, run.size * k) {
                glyphs.push((g.font, g.glyph, pen + g.offset_x, dy + g.offset_y, ri));
                pen += g.advance;
            }
        }
        let shift = match t.anchor {
            -1 => 0.0,
            0 => -pen / 2.0,
            _ => -pen,
        };
        // dominant-baseline: y is not the baseline but the middle of the em box (central) or of the x-height (middle).
        let drop = match (t.base, glyphs.first()) {
            (1.., Some(&(font, _, _, _, ri))) => {
                let size = t.runs.get(ri).map_or(0.0, |r| r.size) * k;
                let fm = pl.font_metrics(font, size);
                if t.base == 1 { (fm.ascent - fm.descent) / 2.0 } else { fm.x_height.unwrap_or(size / 2.0) / 2.0 }
            }
            _ => 0.0,
        };
        let o = m.apply(Pt::new(t.x, t.y));
        let (w, h) = (self.w as i32, self.h as i32);
        let wu = self.w as usize;
        let Some(layer) = self.layers.last_mut() else { return };
        for (font, glyph, gx, gy, ri) in glyphs {
            let Some(run) = t.runs.get(ri) else { continue };
            let px = o.x + shift + gx;
            let (ix, by) = (px.floor(), (o.y + drop + gy).round());
            let sub = (((px - ix) * 4.0) as u8).min(3);
            let Some(mask) = pl.glyph_mask(font, glyph, run.size * k, sub) else { continue };
            let alpha = div255(u32::from(run.fill.alpha) * self.gain);
            let color = match run.fill.paint {
                Paint::Color(c) => premul(c, alpha),
                Paint::Grad(_) => continue,
            };
            let colored = !mask.bgra.is_empty();
            for row in 0..mask.height as i32 {
                let y = by as i32 + mask.top + row;
                if y < 0 || y >= h {
                    continue;
                }
                for col in 0..mask.width as i32 {
                    let x = ix as i32 + mask.left + col;
                    if x < 0 || x >= w {
                        continue;
                    }
                    let mi = (row as usize) * mask.width as usize + col as usize;
                    let mut cov = if colored { 255 } else { u32::from(mask.alpha.get(mi).copied().unwrap_or(0)) };
                    if let Some(c) = &clip {
                        cov = div255(cov * u32::from(c.get(y as usize * wu + x as usize).copied().unwrap_or(0)));
                    }
                    if cov == 0 {
                        continue;
                    }
                    let src = if colored {
                        let b = mi * 4;
                        let Some(&[p0, p1, p2, p3]) = mask.bgra.get(b..b + 4) else { continue };
                        scaled([p0, p1, p2, p3], div255(cov * alpha))
                    } else {
                        scaled(color, cov)
                    };
                    let at = (y as usize * wu + x as usize) * 4;
                    if let Some(d) = layer.canvas.data.get_mut(at..at + 4) {
                        over(d, src);
                    }
                }
            }
            self.work += u64::from(mask.width) * u64::from(mask.height);
        }
    }
}

/// The premultiplied colors along a gradient: stops are interpolated in non-premultiplied sRGB (pservers 13.2.4).
fn build_lut(g: &Grad) -> Vec<[u8; 4]> {
    let mut out = Vec::with_capacity(LUT);
    for i in 0..LUT {
        let t = i as f32 / (LUT - 1) as f32;
        let hi = g.stops.partition_point(|s| s.0 <= t);
        let c = match (hi.checked_sub(1).and_then(|j| g.stops.get(j)), g.stops.get(hi)) {
            (Some(a), Some(b)) => {
                let u = if b.0 > a.0 { (t - a.0) / (b.0 - a.0) } else { 1.0 };
                let mix = |p: u8, q: u8| (f32::from(p) + (f32::from(q) - f32::from(p)) * u + 0.5) as u8;
                Color { r: mix(a.1.r, b.1.r), g: mix(a.1.g, b.1.g), b: mix(a.1.b, b.1.b), a: mix(a.1.a, b.1.a) }
            }
            (Some(a), None) => a.1,
            (None, Some(b)) => b.1,
            (None, None) => Color { r: 0, g: 0, b: 0, a: 0 },
        };
        out.push(premul(c, 255));
    }
    out
}

struct GradEval {
    inv: Transform,
    radial: bool,
    p: [f32; 6],
    spread: u8,
    lut: Rc<Vec<[u8; 4]>>,
}

impl GradEval {
    fn new(g: &Grad, inv: Transform, lut: Rc<Vec<[u8; 4]>>) -> GradEval {
        let mut p = g.p;
        if g.radial {
            // SVG 1.1: a focal point outside the circle moves onto it.
            let (dx, dy) = (p[3] - p[0], p[4] - p[1]);
            let d = dx.hypot(dy);
            if d > p[2] * 0.999 && d > 0.0 {
                let k = p[2] * 0.999 / d;
                p[3] = p[0] + dx * k;
                p[4] = p[1] + dy * k;
            }
        }
        GradEval { inv, radial: g.radial, p, spread: g.spread, lut }
    }

    fn at(&self, x: f32, y: f32) -> [u8; 4] {
        let q = self.inv.apply(Pt::new(x + 0.5, y + 0.5));
        let t = if self.radial {
            let [cx, cy, r, fx, fy, fr] = self.p;
            let (cdx, cdy, dr) = (cx - fx, cy - fy, r - fr);
            let (pdx, pdy) = (q.x - fx, q.y - fy);
            let a = cdx * cdx + cdy * cdy - dr * dr;
            let b = pdx * cdx + pdy * cdy + fr * dr;
            let c = pdx * pdx + pdy * pdy - fr * fr;
            if a.abs() < 1.0e-9 {
                if b.abs() < 1.0e-12 {
                    // The circles coincide: the last stop fills everything (SVG 2 pservers 13.3.? "fr = r").
                    return self.lut.last().copied().unwrap_or([0; 4]);
                }
                c / (2.0 * b)
            } else {
                let disc = b * b - a * c;
                if disc < 0.0 {
                    return [0; 4];
                }
                let s = disc.sqrt();
                let (t1, t2) = ((b + s) / a, (b - s) / a);
                let (hi, lo) = if t1 > t2 { (t1, t2) } else { (t2, t1) };
                if fr + hi * dr >= 0.0 { hi } else { lo }
            }
        } else {
            let [x1, y1, x2, y2, ..] = self.p;
            let (dx, dy) = (x2 - x1, y2 - y1);
            let len2 = dx * dx + dy * dy;
            if len2 == 0.0 {
                return self.lut.last().copied().unwrap_or([0; 4]);
            }
            ((q.x - x1) * dx + (q.y - y1) * dy) / len2
        };
        if !t.is_finite() {
            return [0; 4];
        }
        let t = match self.spread {
            1 => {
                let u = t - (t * 0.5).floor() * 2.0;
                if u > 1.0 { 2.0 - u } else { u }
            }
            2 => t - t.floor(),
            _ => t.fclamp(0.0, 1.0),
        };
        let i = (t * (LUT - 1) as f32 + 0.5) as usize;
        self.lut.get(i.min(LUT - 1)).copied().unwrap_or([0; 4])
    }
}
