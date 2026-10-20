//! The stroker: turns polylines into the polygons of their stroke (css-backgrounds-3 border dashes and dots use it;
//! SVG will, too). The outline is a union of simple polygons, one rectangle per segment, one wedge per join and one
//! cap at each open end, all wound the same way, so filling the result with the non-zero rule is the stroke.

use super::{Poly, Pt};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Cap {
    #[default]
    Butt,
    Round,
    Square,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Join {
    #[default]
    Miter,
    Round,
    Bevel,
}

#[derive(Clone, Debug)]
pub struct Stroke {
    pub width: f32,
    pub cap: Cap,
    pub join: Join,
    /// Longest miter / line width before a miter join becomes a bevel (SVG `stroke-miterlimit`, default 4).
    pub miter_limit: f32,
    /// Alternating dash and gap lengths; empty: solid. An odd count is repeated (SVG / canvas rule).
    pub dashes: Vec<f32>,
    pub dash_offset: f32,
}

impl Stroke {
    pub fn new(width: f32) -> Stroke {
        Stroke { width, cap: Cap::Butt, join: Join::Miter, miter_limit: 4.0, dashes: Vec::new(), dash_offset: 0.0 }
    }
}

/// At most this many polygons come out of one call (hostile paths with tiny dashes).
const MAX_OUT: usize = 1 << 17;
/// At most this many dashes per call.
const MAX_DASHES: usize = 1 << 15;

type V = (f32, f32);

fn unit(a: Pt, b: Pt) -> V {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let l = dx.hypot(dy);
    if l > 0.0 { (dx / l, dy / l) } else { (1.0, 0.0) }
}

fn at(p: Pt, v: V, k: f32) -> Pt {
    Pt::new(p.x + v.0 * k, p.y + v.1 * k)
}

/// Add a polygon, wound the same way as every other one (negative area in y-down pixels).
fn push(out: &mut Vec<Poly>, mut pts: Vec<Pt>) {
    if pts.len() < 3 || out.len() >= MAX_OUT {
        return;
    }
    let mut area = 0.0f32;
    for (i, a) in pts.iter().enumerate() {
        if let Some(b) = pts.get((i + 1) % pts.len()) {
            area += a.x * b.y - b.x * a.y;
        }
    }
    if area > 0.0 {
        pts.reverse();
    }
    out.push(Poly { pts, closed: true });
}

/// Points of the arc around `c` with radius `r` from angle `a0` by `sweep`, endpoints included.
fn arc(c: Pt, r: f32, a0: f32, sweep: f32, tol: f32) -> Vec<Pt> {
    // The chord angle whose sagitta is `tol`: r (1 - cos(a / 2)) ~ r a^2 / 8.
    let step = 2.0 * (2.0 * tol.min(r) / r).sqrt();
    let n = if step > 1.0e-3 { (sweep.abs() / step).ceil() as usize } else { 64 }.clamp(1, 256);
    (0..=n)
        .map(|i| {
            let a = a0 + sweep * i as f32 / n as f32;
            let (s, co) = crate::fmath::sin_cos(f64::from(a));
            Pt::new(c.x + r * co as f32, c.y + r * s as f32)
        })
        .collect()
}

/// The outline of the stroke of `polys`. Fill it with the non-zero rule.
pub fn stroke_polys(polys: &[Poly], s: &Stroke, tol: f32) -> Vec<Poly> {
    let mut out = Vec::new();
    let hw = s.width * 0.5;
    if hw <= 0.0 || !hw.is_finite() {
        return out;
    }
    let tol = if tol.is_finite() { tol.max(1.0e-3) } else { 0.1 };
    let dashes: Vec<f32> = if s.dashes.iter().all(|d| d.is_finite() && *d >= 0.0) && s.dashes.iter().sum::<f32>() > 0.0 {
        let mut d = s.dashes.clone();
        if d.len() % 2 == 1 {
            d.extend_from_slice(&s.dashes);
        }
        d
    } else {
        Vec::new()
    };
    let mut dash_budget = MAX_DASHES;
    for p in polys {
        let mut pts: Vec<Pt> = Vec::with_capacity(p.pts.len());
        for q in &p.pts {
            if q.x.is_finite() && q.y.is_finite() && pts.last().is_none_or(|l| (l.x - q.x).hypot(l.y - q.y) > 1.0e-5) {
                pts.push(*q);
            }
        }
        let mut closed = p.closed;
        if closed && pts.len() > 1 && pts.first().zip(pts.last()).is_some_and(|(a, b)| (a.x - b.x).hypot(a.y - b.y) <= 1.0e-5) {
            pts.pop();
        }
        if pts.len() < 2 {
            closed = false;
        }
        if dashes.is_empty() {
            one(&pts, closed, hw, s, tol, &mut out);
        } else {
            for d in split_dashes(&pts, closed, &dashes, s.dash_offset, &mut dash_budget) {
                one(&d, false, hw, s, tol, &mut out);
            }
        }
        if out.len() >= MAX_OUT {
            break;
        }
    }
    out
}

/// Stroke one polyline.
fn one(pts: &[Pt], closed: bool, hw: f32, s: &Stroke, tol: f32, out: &mut Vec<Poly>) {
    let n = pts.len();
    let Some(first) = pts.first().copied() else { return };
    if n == 1 {
        // A zero-length subpath: a dot for round caps, a square for square caps.
        match s.cap {
            Cap::Round => push(out, arc(first, hw, 0.0, std::f32::consts::TAU, tol)),
            Cap::Square => push(out, vec![Pt::new(first.x - hw, first.y - hw), Pt::new(first.x + hw, first.y - hw), Pt::new(first.x + hw, first.y + hw), Pt::new(first.x - hw, first.y + hw)]),
            Cap::Butt => {}
        }
        return;
    }
    let segs = if closed { n } else { n - 1 };
    let dir: Vec<V> = (0..segs).filter_map(|i| Some(unit(*pts.get(i)?, *pts.get((i + 1) % n)?))).collect();
    for (i, d) in dir.iter().enumerate() {
        let (Some(a), Some(b)) = (pts.get(i), pts.get((i + 1) % n)) else { continue };
        let nrm = (-d.1, d.0);
        push(out, vec![at(*a, nrm, hw), at(*b, nrm, hw), at(*b, nrm, -hw), at(*a, nrm, -hw)]);
    }
    let joins = if closed { 0..n } else { 1..n - 1 };
    for i in joins {
        let (Some(d0), Some(d1), Some(v)) = (dir.get((i + segs - 1) % segs), dir.get(i % segs), pts.get(i)) else { continue };
        join(*v, *d0, *d1, hw, s, tol, out);
    }
    if !closed {
        if let (Some(d), Some(p)) = (dir.first(), pts.first()) {
            cap(*p, (-d.0, -d.1), hw, s.cap, tol, out);
        }
        if let (Some(d), Some(p)) = (dir.last(), pts.last()) {
            cap(*p, *d, hw, s.cap, tol, out);
        }
    }
}

/// A cap at `p`; `u` is the unit vector pointing out of the line.
fn cap(p: Pt, u: V, hw: f32, kind: Cap, tol: f32, out: &mut Vec<Poly>) {
    let nrm = (-u.1, u.0);
    match kind {
        Cap::Butt => {}
        Cap::Square => {
            let (a, b) = (at(p, nrm, hw), at(p, nrm, -hw));
            push(out, vec![a, at(a, u, hw), at(b, u, hw), b]);
        }
        Cap::Round => push(out, arc(p, hw, crate::fmath::atan2(nrm.1, nrm.0), -std::f32::consts::PI, tol)),
    }
}

/// The wedge that fills the outer side of the corner at `v` between directions `d0` and `d1`.
fn join(v: Pt, d0: V, d1: V, hw: f32, s: &Stroke, tol: f32, out: &mut Vec<Poly>) {
    let cross = d0.0 * d1.1 - d0.1 * d1.0;
    let dot = d0.0 * d1.0 + d0.1 * d1.1;
    if cross.abs() < 1.0e-6 && dot > 0.0 {
        return;
    }
    if dot < -0.999_999 {
        if s.join == Join::Round {
            cap(v, d0, hw, Cap::Round, tol, out);
        }
        return;
    }
    // The outer side is the one the path turns away from.
    let side = if cross > 0.0 { -1.0 } else { 1.0 };
    let (n0, n1) = ((-d0.1, d0.0), (-d1.1, d1.0));
    let a = at(v, n0, side * hw);
    let b = at(v, n1, side * hw);
    match s.join {
        Join::Round => {
            let a0 = crate::fmath::atan2(a.y - v.y, a.x - v.x);
            let a1 = crate::fmath::atan2(b.y - v.y, b.x - v.x);
            let mut sweep = a1 - a0;
            while sweep > std::f32::consts::PI {
                sweep -= std::f32::consts::TAU;
            }
            while sweep < -std::f32::consts::PI {
                sweep += std::f32::consts::TAU;
            }
            let mut pts = vec![v];
            pts.extend(arc(v, hw, a0, sweep, tol));
            push(out, pts);
        }
        Join::Miter if (2.0 / (1.0 + dot)).sqrt() <= s.miter_limit => {
            let k = side * hw / (1.0 + dot);
            let tip = Pt::new(v.x + (n0.0 + n1.0) * k, v.y + (n0.1 + n1.1) * k);
            push(out, vec![v, a, tip, b]);
        }
        _ => push(out, vec![v, a, b]),
    }
}

/// Cut a polyline into dashes (open polylines).
fn split_dashes(pts: &[Pt], closed: bool, pattern: &[f32], offset: f32, budget: &mut usize) -> Vec<Vec<Pt>> {
    let total: f32 = pattern.iter().sum();
    let mut idx = 0usize;
    let mut left = pattern.first().copied().unwrap_or(0.0);
    let mut phase = if offset.is_finite() { offset - (offset / total).floor() * total } else { 0.0 };
    // Skip into the pattern by the offset.
    while phase > 0.0 {
        if phase >= left {
            phase -= left;
            idx = (idx + 1) % pattern.len();
            left = pattern.get(idx).copied().unwrap_or(0.0);
        } else {
            left -= phase;
            phase = 0.0;
        }
    }
    let mut out: Vec<Vec<Pt>> = Vec::new();
    let mut cur: Vec<Pt> = Vec::new();
    let on = |i: usize| i.is_multiple_of(2);
    if on(idx)
        && let Some(p) = pts.first()
    {
        cur.push(*p);
    }
    let n = pts.len();
    let segs = if closed { n } else { n.saturating_sub(1) };
    for i in 0..segs {
        let (Some(&a), Some(&b)) = (pts.get(i), pts.get((i + 1) % n)) else { continue };
        let len = (b.x - a.x).hypot(b.y - a.y);
        let mut used = 0.0f32;
        while len - used > left {
            used += left;
            let t = if len > 0.0 { used / len } else { 0.0 };
            let p = Pt::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t);
            if on(idx) {
                cur.push(p);
                out.push(std::mem::take(&mut cur));
                if out.len() >= *budget {
                    *budget = 0;
                    return out;
                }
            } else {
                cur.clear();
                cur.push(p);
            }
            idx = (idx + 1) % pattern.len();
            left = pattern.get(idx).copied().unwrap_or(0.0);
        }
        left -= len - used;
        if on(idx) {
            cur.push(b);
        }
    }
    if on(idx) && cur.len() >= 2 {
        out.push(cur);
    }
    *budget = budget.saturating_sub(out.len());
    out
}

#[cfg(test)]
#[allow(clippy::indexing_slicing, clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::super::{FillRule, IRect, Mask, Path};
    use super::*;

    const WIN: IRect = IRect { x0: -50, y0: -50, x1: 200, y1: 200 };

    fn mask(path: &Path, s: &Stroke) -> Mask {
        let outline = stroke_polys(&path.flatten(0.05), s, 0.05);
        Mask::from_polys(&outline, FillRule::NonZero, WIN).unwrap()
    }

    fn line(x0: f32, y0: f32, x1: f32, y1: f32) -> Path {
        let mut p = Path::new();
        p.move_to(x0, y0);
        p.line_to(x1, y1);
        p
    }

    fn sum(m: &Mask) -> f32 {
        m.data.iter().map(|&v| f32::from(v)).sum::<f32>() / 255.0
    }

    #[test]
    fn a_butt_line_covers_width_times_length() {
        let m = mask(&line(1.0, 5.0, 9.0, 5.0), &Stroke::new(2.0));
        assert_eq!((m.at(4, 4), m.at(4, 5), m.at(4, 3), m.at(4, 6), m.at(0, 5), m.at(9, 5)), (255, 255, 0, 0, 0, 0));
        assert!((sum(&m) - 16.0).abs() < 0.05);
    }

    #[test]
    fn caps_extend_the_line_by_half_the_width() {
        let mut s = Stroke::new(4.0);
        s.cap = Cap::Square;
        let sq = mask(&line(10.0, 10.0, 20.0, 10.0), &s);
        assert!((sum(&sq) - 56.0).abs() < 0.1, "{}", sum(&sq));
        s.cap = Cap::Round;
        let rd = mask(&line(10.0, 10.0, 20.0, 10.0), &s);
        // 10 x 4 plus a full disc of radius 2.
        assert!((sum(&rd) - (40.0 + 12.566)).abs() < 0.6, "{}", sum(&rd));
    }

    #[test]
    fn miter_round_and_bevel_joins_differ_at_the_corner() {
        let mut p = Path::new();
        p.move_to(10.0, 50.0);
        p.line_to(50.0, 50.0);
        p.line_to(50.0, 10.0);
        let mut s = Stroke::new(10.0);
        let miter = sum(&mask(&p, &s));
        s.join = Join::Bevel;
        let bevel = sum(&mask(&p, &s));
        s.join = Join::Round;
        let round = sum(&mask(&p, &s));
        // The miter fills the 5 x 5 outer corner square; the bevel only its diagonal half (12.5); the round join is a
        // quarter disc of radius 5 (19.63).
        assert!((miter - bevel - 12.5).abs() < 0.3, "{miter} {bevel}");
        assert!((round - bevel - (std::f32::consts::PI * 25.0 / 4.0 - 12.5)).abs() < 0.5, "{round} {bevel}");
        // A miter limit below the corner's 1.414 turns it into a bevel.
        s.join = Join::Miter;
        s.miter_limit = 1.2;
        assert!((sum(&mask(&p, &s)) - bevel).abs() < 0.05);
    }

    #[test]
    fn dashes_alternate_along_the_line() {
        let mut s = Stroke::new(2.0);
        s.dashes = vec![4.0, 2.0];
        let m = mask(&line(0.0, 5.0, 24.0, 5.0), &s);
        let on: Vec<bool> = (0..24).map(|x| m.at(x, 5) > 200).collect();
        let want: Vec<bool> = (0..24).map(|x| x % 6 < 4).collect();
        assert_eq!(on, want);
        s.dash_offset = 4.0; // starts in the gap
        let m = mask(&line(0.0, 5.0, 12.0, 5.0), &s);
        assert_eq!((m.at(0, 5), m.at(1, 5), m.at(2, 5), m.at(5, 5), m.at(6, 5)), (0, 0, 255, 255, 0));
    }

    #[test]
    fn hostile_strokes_do_not_panic_and_stay_bounded() {
        let mut p = Path::new();
        p.move_to(0.0, 0.0);
        p.line_to(1.0e7, 1.0e7);
        p.line_to(f32::NAN, 3.0);
        let mut s = Stroke::new(1.0e30);
        s.dashes = vec![1.0e-6, 1.0e-6];
        let out = stroke_polys(&p.flatten(0.1), &s, 0.1);
        assert!(out.len() <= 1 << 17);
        let mut s = Stroke::new(3.0);
        s.dashes = vec![0.001, 0.001];
        let out = stroke_polys(&Path::rect(0.0, 0.0, 1000.0, 1000.0).flatten(0.1), &s, 0.1);
        assert!(out.len() <= 1 << 17);
    }
}
