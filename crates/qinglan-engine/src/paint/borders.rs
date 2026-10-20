#![allow(clippy::indexing_slicing)] // the indices are the sides 0..4 of arrays of four
//! Borders and outlines with a radius or a style other than `solid` (css-backgrounds-3 4, 5; css-ui-3 6).
//!
//! A border is a ring between the outer border edge and the inner (padding) edge, both rounded rectangles; the inner
//! radius of a corner is the outer one minus the border width, at least 0 (5.3). Each side owns the part of the ring in
//! its wedge: the quadrilateral from the two outer corners of the border box to the two inner corners (4.3 "corner
//! joins"), so differently colored sides meet on the diagonal. Everything is drawn from paths in device pixels with the
//! rasterizer of [`crate::vector`]; `dashed` and `dotted` are strokes along the middle of the border.

use super::raster::{Frame, device_rect, over, premul};
use super::{BorderDraw, Rect};
use crate::css::color::Color;
use crate::css::types::BorderLineStyle;
use crate::vector::{Cap, FillRule, IRect, Mask, Path, Poly, Pt, Stroke, fill_polys, stroke_polys};

/// Flattening tolerance, device pixels.
const TOL: f32 = 0.01;

/// A rounded rectangle in device pixels; radii are (horizontal, vertical) for top-left, top-right, bottom-right, bottom-left.
#[derive(Clone, Copy, Debug)]
pub(super) struct RR {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
    pub r: [(f32, f32); 4],
}

impl RR {
    /// Scale the radii down until the corners fit the sides (css-backgrounds-3 5.5).
    pub fn fit(mut self) -> RR {
        let (w, h) = (self.x1 - self.x0, self.y1 - self.y0);
        let [tl, tr, br, bl] = self.r;
        let mut f = 1.0f32;
        for (side, sum) in [(w, tl.0 + tr.0), (w, bl.0 + br.0), (h, tl.1 + bl.1), (h, tr.1 + br.1)] {
            if sum > side && sum > 0.0 {
                f = f.min(side.max(0.0) / sum);
            }
        }
        if f < 1.0 {
            for r in &mut self.r {
                *r = (r.0 * f, r.1 * f);
            }
        }
        self
    }

    /// The rectangle moved in by `d` (top, right, bottom, left); radii shrink by the same amounts.
    pub fn inset(&self, d: [f32; 4]) -> RR {
        let [t, r, b, l] = d;
        let [tl, tr, br, bl] = self.r;
        let s = |v: f32, d: f32| (v - d).max(0.0);
        RR {
            x0: self.x0 + l,
            y0: self.y0 + t,
            x1: (self.x1 - r).max(self.x0 + l),
            y1: (self.y1 - b).max(self.y0 + t),
            r: [(s(tl.0, l), s(tl.1, t)), (s(tr.0, r), s(tr.1, t)), (s(br.0, r), s(br.1, b)), (s(bl.0, l), s(bl.1, b))],
        }
        .fit()
    }

    pub fn polys(&self) -> Vec<Poly> {
        let sq = |r: (f32, f32)| if r.0 > 0.0 && r.1 > 0.0 { r } else { (0.0, 0.0) };
        Path::rounded_rect(self.x0, self.y0, self.x1 - self.x0, self.y1 - self.y0, self.r.map(sq)).flatten(TOL)
    }

    /// The corner points: top-left, top-right, bottom-right, bottom-left.
    fn corners(&self) -> [Pt; 4] {
        [Pt::new(self.x0, self.y0), Pt::new(self.x1, self.y0), Pt::new(self.x1, self.y1), Pt::new(self.x0, self.y1)]
    }

    fn has_radius(&self) -> bool {
        self.r.iter().any(|r| r.0 > 0.0 && r.1 > 0.0)
    }
}

/// Blend a row of coverage into the frame, inside `clip`.
fn blend_row(frame: &mut Frame, y: i32, x0: i32, cov: &[u8], color: [u8; 4], clip: &IRect) {
    if y < clip.y0 || y >= clip.y1 {
        return;
    }
    let (a, b) = (x0.max(clip.x0), (x0 + cov.len() as i32).min(clip.x1));
    if a >= b || y < 0 {
        return;
    }
    let stride = frame.width as usize * 4;
    let Some(row) = frame.data.get_mut(y as usize * stride..(y as usize + 1) * stride) else { return };
    let Some(px) = row.get_mut(a.max(0) as usize * 4..b.max(0) as usize * 4) else { return };
    let Some(cv) = cov.get((a - x0) as usize..(b - x0) as usize) else { return };
    for (p, &c) in px.as_chunks_mut::<4>().0.iter_mut().zip(cv) {
        match c {
            0 => {}
            255 => over(p, color),
            c => {
                let c = u32::from(c);
                let m = |v: u8| ((u32::from(v) * c + 127) / 255) as u8;
                over(p, [m(color[0]), m(color[1]), m(color[2]), m(color[3])]);
            }
        }
    }
}

fn fill(frame: &mut Frame, polys: &[Poly], rule: FillRule, color: Color, clip: &IRect) {
    let px = premul(color);
    fill_polys(polys, rule, *clip, &mut |y, x0, cov| blend_row(frame, y, x0, cov, px, clip));
}

fn fill_mask(frame: &mut Frame, m: &Mask, color: Color, clip: &IRect) {
    let px = premul(color);
    for y in m.rect.y0..m.rect.y1 {
        if let Some(row) = m.row(y) {
            blend_row(frame, y, m.rect.x0, row, px, clip);
        }
    }
}

/// Darker / lighter versions of a border color for `inset`, `outset`, `groove` and `ridge` (the way Chromium does it).
fn shade(c: Color, darken: bool) -> Color {
    let v = f32::from(c.r.max(c.g).max(c.b)) / 255.0;
    let scale = |m: f32| Color { r: (f32::from(c.r) * m).min(255.0) as u8, g: (f32::from(c.g) * m).min(255.0) as u8, b: (f32::from(c.b) * m).min(255.0) as u8, a: c.a };
    if darken {
        if (c.r, c.g, c.b) == (255, 255, 255) {
            return Color { r: 0xab, g: 0xab, b: 0xab, a: c.a };
        }
        if v <= 0.13 { c } else { scale(((v - 0.33) / v).max(0.0)) }
    } else if (c.r, c.g, c.b) == (0, 0, 0) {
        Color { r: 0x54, g: 0x54, b: 0x54, a: c.a }
    } else if v >= 0.92 {
        c
    } else {
        scale(((v + 0.33).min(1.0)) / v)
    }
}

/// The bands of one side: (color, from, to) with `from` and `to` fractions of the border width counted from the outer edge.
fn bands(style: BorderLineStyle, side: usize, c: Color, w: f32) -> Vec<(Color, f32, f32)> {
    let top_left = side == 0 || side == 3;
    // Chromium: a side is darkened when it is a top / left side of an inset border or a bottom / right side of an outset one.
    let inset_like = |outer_half: bool| shade(c, top_left == outer_half);
    match style {
        BorderLineStyle::Double if w >= 3.0 => vec![(c, 0.0, 1.0 / 3.0), (c, 2.0 / 3.0, 1.0)],
        BorderLineStyle::Inset => vec![(inset_like(true), 0.0, 1.0)],
        BorderLineStyle::Outset => vec![(inset_like(false), 0.0, 1.0)],
        BorderLineStyle::Groove if w >= 2.0 => vec![(inset_like(true), 0.0, 0.5), (inset_like(false), 0.5, 1.0)],
        BorderLineStyle::Ridge if w >= 2.0 => vec![(inset_like(false), 0.0, 0.5), (inset_like(true), 0.5, 1.0)],
        BorderLineStyle::Groove => vec![(inset_like(true), 0.0, 1.0)],
        BorderLineStyle::Ridge => vec![(inset_like(false), 0.0, 1.0)],
        _ => vec![(c, 0.0, 1.0)],
    }
}

/// The wedge of `side` between the corners of `a` (outside) and `b` (inside).
fn wedge(a: &RR, b: &RR, side: usize) -> Vec<Poly> {
    let (ca, cb) = (a.corners(), b.corners());
    let at = |c: &[Pt; 4], i: usize| c.get(i % 4).copied().unwrap_or_default();
    let pts = vec![at(&ca, side), at(&ca, side + 1), at(&cb, side + 1), at(&cb, side)];
    vec![Poly { pts, closed: true }]
}

/// Paint a border or an outline. `sy` is the scroll offset in device pixels, `clip` the drawing area in frame pixels.
pub(super) fn draw_border(frame: &mut Frame, b: &BorderDraw, dpr: f64, sy: i32, clip: &IRect) {
    let shift = |mut r: IRect| {
        r.y0 -= sy;
        r.y1 -= sy;
        r
    };
    let o = shift(device_rect(&b.rect, dpr));
    let inner_lu = Rect::new(b.rect.x + b.w[3], b.rect.y + b.w[0], (b.rect.w - b.w[1] - b.w[3]).max0(), (b.rect.h - b.w[0] - b.w[2]).max0());
    let i = shift(device_rect(&inner_lu, dpr));
    let wd = [(i.y0 - o.y0).max(0), (o.x1 - i.x1).max(0), (o.y1 - i.y1).max(0), (i.x0 - o.x0).max(0)];
    let wf = wd.map(|v| v as f32);
    let k = dpr as f32;
    let outer = RR { x0: o.x0 as f32, y0: o.y0 as f32, x1: o.x1 as f32, y1: o.y1 as f32, r: b.radii.map(|(a, c)| (a * k, c * k)) }.fit();
    let inner = outer.inset(wf);
    let active: Vec<usize> = (0..4).filter(|&s| wd.get(s).is_some_and(|w| *w > 0) && b.color.get(s).is_some_and(|c| c.a > 0) && !matches!(b.style.get(s), None | Some(BorderLineStyle::None | BorderLineStyle::Hidden))).collect();
    let (Some(&first), true) = (active.first(), !clip.is_empty()) else { return };
    let (st0, c0, w0) = (b.style[first], b.color[first], wd[first]);
    let same = |f: &dyn Fn(usize) -> bool| active.iter().all(|&s| f(s));
    if st0 == BorderLineStyle::Solid && same(&|s| b.style[s] == st0 && b.color[s] == c0) {
        let mut ring = outer.polys();
        ring.extend(inner.polys());
        return fill(frame, &ring, FillRule::EvenOdd, c0, clip);
    }
    let dashed = |s: BorderLineStyle| matches!(s, BorderLineStyle::Dashed | BorderLineStyle::Dotted);
    if dashed(st0) && active.len() == 4 && same(&|s| b.style[s] == st0 && b.color[s] == c0 && wd[s] == w0) {
        // One stroke along the middle of the whole border.
        let half = w0 as f32 / 2.0;
        let mid = outer.inset([half; 4]);
        let polys = mid.polys();
        let len: f32 = polys.iter().map(|p| perimeter(&p.pts)).sum();
        let (dash, cap) = pattern(len, w0 as f32, st0 == BorderLineStyle::Dotted, true);
        let mut s = Stroke::new(w0 as f32);
        s.dashes = dash;
        s.cap = cap;
        return fill(frame, &stroke_polys(&polys, &s, TOL), FillRule::NonZero, c0, clip);
    }
    let mut done = [false; 4];
    for &side in &active {
        if done[side] {
            continue;
        }
        let (style, color, w) = (b.style[side], b.color[side], wd[side]);
        if dashed(style) {
            let m = outer.inset(wf.map(|v| v * 0.5));
            let c = m.corners();
            let (mut p, mut q) = (c.get(side).copied().unwrap_or_default(), c.get((side + 1) % 4).copied().unwrap_or_default());
            let mut len = (q.x - p.x).hypot(q.y - p.y);
            let dotted = style == BorderLineStyle::Dotted;
            if dotted && len > w as f32 {
                // The first and the last dot lie inside the border box: the line starts and ends half a dot in.
                let (ux, uy) = ((q.x - p.x) / len, (q.y - p.y) / len);
                let r = w as f32 / 2.0;
                (p, q) = (Pt::new(p.x + ux * r, p.y + uy * r), Pt::new(q.x - ux * r, q.y - uy * r));
                len -= w as f32;
            }
            let (dash, cap) = pattern(len, w as f32, dotted, false);
            let mut s = Stroke::new(w as f32);
            s.dashes = dash;
            s.cap = cap;
            let line = [Poly { pts: vec![p, q], closed: false }];
            fill(frame, &stroke_polys(&line, &s, TOL), FillRule::NonZero, color, clip);
            continue;
        }
        // Solid sides of one color are drawn together: coverage adds up along the diagonal between them, so there is no seam.
        let group: Vec<usize> = if style == BorderLineStyle::Solid {
            active.iter().copied().filter(|&j| !done[j] && b.style[j] == style && b.color[j] == color).collect()
        } else {
            vec![side]
        };
        for &j in &group {
            done[j] = true;
        }
        for (col, from, to) in bands(style, side, color, w as f32) {
            if col.a == 0 {
                continue;
            }
            let ra = outer.inset(wf.map(|v| v * from));
            let rb = outer.inset(wf.map(|v| v * to));
            if !outer.has_radius() {
                let polys: Vec<Poly> = group.iter().flat_map(|&j| wedge(&ra, &rb, j)).collect();
                fill(frame, &polys, FillRule::NonZero, col, clip);
                continue;
            }
            // Ring between the two boundaries, cut to the wedges of the sides.
            let full: Vec<Poly> = group.iter().flat_map(|&j| wedge(&outer, &inner, j)).collect();
            let mut ring = ra.polys();
            ring.extend(rb.polys());
            let Some(area) = crate::vector::fill::bounds(&full, *clip) else { continue };
            let (Some(mut m), Some(wm)) = (Mask::from_polys(&ring, FillRule::EvenOdd, area), Mask::from_polys(&full, FillRule::NonZero, area)) else { continue };
            m.multiply(&wm);
            fill_mask(frame, &m, col, clip);
        }
    }
}

fn perimeter(pts: &[Pt]) -> f32 {
    let n = pts.len();
    pts.iter().enumerate().map(|(i, a)| pts.get((i + 1) % n).map_or(0.0, |b| (b.x - a.x).hypot(b.y - a.y))).sum()
}

/// Dash lengths and the cap for a line of `len` and width `w` (Chromium's ratios, gaps adjusted so the pattern fits).
fn pattern(len: f32, w: f32, dotted: bool, closed: bool) -> (Vec<f32>, Cap) {
    let (dash, gap, cap) = if dotted {
        (0.01, 2.0 * w - 0.01, Cap::Round)
    } else if w >= 3.0 {
        (2.0 * w, w, Cap::Butt)
    } else {
        (3.0 * w, 2.0 * w, Cap::Butt)
    };
    let period = dash + gap;
    if len <= 0.0 || period <= 0.0 {
        return (Vec::new(), cap);
    }
    if closed {
        let n = (len / period).round().max(1.0);
        (vec![dash, (len / n - dash).max(0.0)], cap)
    } else {
        let n = ((len + gap) / period).floor().max(1.0);
        if n <= 1.0 { (Vec::new(), Cap::Butt) } else { (vec![dash, ((len - n * dash) / (n - 1.0)).max(0.0)], cap) }
    }
}
