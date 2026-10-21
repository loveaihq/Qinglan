//! SVG's small syntaxes: numbers (SVG 2 4.? basic data types), lengths (7.? units), path data (9.3-9.5 with the error
//! handling of 9.5.4 "render up to the first error"), transform lists (css-transforms-1 "SVG transform attribute") and
//! number lists. Every parser works on hostile text and returns what it could read; none panics.

use crate::fmath::Fclamp;
use crate::fmath::sin_cos;
use crate::vector::{Path, Pt, Transform};

pub struct Cur<'a> {
    b: &'a [u8],
    s: &'a str,
    pub i: usize,
}

impl<'a> Cur<'a> {
    pub fn new(s: &'a str) -> Cur<'a> {
        Cur { b: s.as_bytes(), s, i: 0 }
    }
    pub fn peek(&self) -> u8 {
        self.b.get(self.i).copied().unwrap_or(0)
    }
    pub fn end(&self) -> bool {
        self.i >= self.b.len()
    }
    #[inline(never)]
    pub fn ws(&mut self) {
        while matches!(self.peek(), b' ' | b'\t' | b'\n' | b'\r' | 0x0c) {
            self.i += 1;
        }
    }
    /// White space and at most one comma.
    #[inline(never)]
    pub fn sep(&mut self) {
        self.ws();
        if self.peek() == b',' {
            self.i += 1;
            self.ws();
        }
    }
    pub fn rest(&self) -> &'a str {
        self.s.get(self.i..).unwrap_or("")
    }
    fn digits(&mut self) -> usize {
        let s = self.i;
        while self.peek().is_ascii_digit() {
            self.i += 1;
        }
        self.i - s
    }
    /// `[+-]? (digits [. digits*] | . digits) ([eE] [+-]? digits)?`; an `e` that does not start an exponent (`1em`) is left.
    #[inline(never)]
    pub fn number(&mut self) -> Option<f32> {
        let start = self.i;
        if matches!(self.peek(), b'+' | b'-') {
            self.i += 1;
        }
        let int = self.digits();
        let mut frac = 0;
        if self.peek() == b'.' {
            self.i += 1;
            frac = self.digits();
        }
        if int + frac == 0 {
            self.i = start;
            return None;
        }
        if matches!(self.peek(), b'e' | b'E') {
            let save = self.i;
            self.i += 1;
            if matches!(self.peek(), b'+' | b'-') {
                self.i += 1;
            }
            if self.digits() == 0 {
                self.i = save;
            }
        }
        let v = self.s.get(start..self.i)?.parse::<f32>().ok().filter(|v| v.is_finite());
        if v.is_none() {
            self.i = start;
        }
        v
    }
    /// An arc flag: a single `0` or `1`.
    fn flag(&mut self) -> Option<bool> {
        let v = match self.peek() {
            b'0' => false,
            b'1' => true,
            _ => return None,
        };
        self.i += 1;
        Some(v)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Unit {
    Px,
    Pct,
    Em,
    Ex,
}

/// A length: the number and its unit (absolute units are converted to px, CSS px = 1/96 in).
#[inline(never)]
pub fn length(s: &str) -> Option<(f32, Unit)> {
    let mut c = Cur::new(s.trim());
    let v = c.number()?;
    let u = c.rest();
    let r = match u {
        "" | "px" => (v, Unit::Px),
        "%" => (v, Unit::Pct),
        "em" => (v, Unit::Em),
        "ex" => (v, Unit::Ex),
        "pt" => (v * 4.0 / 3.0, Unit::Px),
        "pc" => (v * 16.0, Unit::Px),
        "mm" => (v * 96.0 / 25.4, Unit::Px),
        "cm" => (v * 96.0 / 2.54, Unit::Px),
        "in" => (v * 96.0, Unit::Px),
        "q" | "Q" => (v * 96.0 / 101.6, Unit::Px),
        _ => return None,
    };
    Some(r)
}

/// A number list separated by white space and commas; `None` when anything else is in it.
#[inline(never)]
pub fn numbers(s: &str, max: usize) -> Option<Vec<f32>> {
    let mut c = Cur::new(s);
    let mut out = Vec::new();
    c.ws();
    while !c.end() {
        if out.len() >= max {
            break;
        }
        out.push(c.number()?);
        c.sep();
    }
    Some(out)
}

/// Path data. Returns the path up to the first error (an incomplete command is dropped) and whether all of `d` was read
/// (false after an error or when `max_segs` stopped it).
pub fn path(d: &str, max_segs: usize) -> (Path, bool) {
    let mut p = Path::new();
    let mut c = Cur::new(d);
    let (mut cur, mut start) = (Pt::default(), Pt::default());
    // The previous command's last control point and its kind (1 cubic, 2 quadratic), for S and T.
    let (mut ctrl, mut kind) = (Pt::default(), 0u8);
    let mut cmd = 0u8;
    let mut first = true;
    c.ws();
    loop {
        c.ws();
        if c.end() {
            return (p, true);
        }
        if p.segs.len() >= max_segs {
            return (p, false);
        }
        let b = c.peek();
        if b.is_ascii_alphabetic() {
            cmd = b;
            c.i += 1;
            if !matches!(cmd.to_ascii_lowercase(), b'm' | b'l' | b'h' | b'v' | b'c' | b's' | b'q' | b't' | b'a' | b'z') || (first && !matches!(cmd, b'M' | b'm')) {
                return (p, false);
            }
        } else if first || matches!(cmd, b'z' | b'Z') {
            return (p, false);
        }
        first = false;
        let rel = cmd.is_ascii_lowercase();
        let base = if rel { cur } else { Pt::default() };
        // Read the arguments of one command; any failure ends the path before this command.
        let ok = (|| -> Option<()> {
            let num = |c: &mut Cur| -> Option<f32> {
                c.ws();
                let v = c.number();
                c.sep();
                v
            };
            match cmd.to_ascii_lowercase() {
                b'm' => {
                    let (x, y) = (num(&mut c)?, num(&mut c)?);
                    cur = Pt::new(base.x + x, base.y + y);
                    start = cur;
                    p.move_to(cur.x, cur.y);
                    cmd = if rel { b'l' } else { b'L' };
                    kind = 0;
                }
                b'l' => {
                    let (x, y) = (num(&mut c)?, num(&mut c)?);
                    cur = Pt::new(base.x + x, base.y + y);
                    p.line_to(cur.x, cur.y);
                    kind = 0;
                }
                b'h' => {
                    cur = Pt::new(base.x + num(&mut c)?, cur.y);
                    p.line_to(cur.x, cur.y);
                    kind = 0;
                }
                b'v' => {
                    cur = Pt::new(cur.x, base.y + num(&mut c)?);
                    p.line_to(cur.x, cur.y);
                    kind = 0;
                }
                b'c' | b's' => {
                    let c1 = if cmd.eq_ignore_ascii_case(&b'c') {
                        let (x, y) = (num(&mut c)?, num(&mut c)?);
                        Pt::new(base.x + x, base.y + y)
                    } else if kind == 1 {
                        Pt::new(2.0 * cur.x - ctrl.x, 2.0 * cur.y - ctrl.y)
                    } else {
                        cur
                    };
                    let (x2, y2, x, y) = (num(&mut c)?, num(&mut c)?, num(&mut c)?, num(&mut c)?);
                    let (c2, to) = (Pt::new(base.x + x2, base.y + y2), Pt::new(base.x + x, base.y + y));
                    p.cubic_to(c1.x, c1.y, c2.x, c2.y, to.x, to.y);
                    (ctrl, cur, kind) = (c2, to, 1);
                }
                b'q' | b't' => {
                    let c1 = if cmd.eq_ignore_ascii_case(&b'q') {
                        let (x, y) = (num(&mut c)?, num(&mut c)?);
                        Pt::new(base.x + x, base.y + y)
                    } else if kind == 2 {
                        Pt::new(2.0 * cur.x - ctrl.x, 2.0 * cur.y - ctrl.y)
                    } else {
                        cur
                    };
                    let (x, y) = (num(&mut c)?, num(&mut c)?);
                    let to = Pt::new(base.x + x, base.y + y);
                    p.quad_to(c1.x, c1.y, to.x, to.y);
                    (ctrl, cur, kind) = (c1, to, 2);
                }
                b'a' => {
                    let (rx, ry, rot) = (num(&mut c)?, num(&mut c)?, num(&mut c)?);
                    c.ws();
                    let large = c.flag()?;
                    c.sep();
                    let sweep = c.flag()?;
                    c.sep();
                    let (x, y) = (num(&mut c)?, num(&mut c)?);
                    let to = Pt::new(base.x + x, base.y + y);
                    arc(&mut p, cur, (rx, ry, rot), (large, sweep), to);
                    cur = to;
                    kind = 0;
                }
                _ => {
                    // z: back to the start of the subpath.
                    p.close();
                    cur = start;
                    kind = 0;
                }
            }
            Some(())
        })();
        if ok.is_none() {
            return (p, false);
        }
    }
}

/// An elliptical arc from `from` to `to` as cubic Béziers, quarter turns at most (SVG 2 appendix B.2.4 endpoint to center
/// conversion; radii that are too small are scaled up, a zero radius is a line, equal ends draw nothing).
fn arc(p: &mut Path, from: Pt, (rx, ry, rot): (f32, f32, f32), (large, sweep): (bool, bool), to: Pt) {
    if from == to {
        return;
    }
    let (mut rx, mut ry) = (f64::from(rx.abs()), f64::from(ry.abs()));
    if rx == 0.0 || ry == 0.0 {
        p.line_to(to.x, to.y);
        return;
    }
    let (sp, cp) = sin_cos(f64::from(rot).to_radians());
    let (dx, dy) = (f64::from(from.x - to.x) / 2.0, f64::from(from.y - to.y) / 2.0);
    let (x1, y1) = (cp * dx + sp * dy, -sp * dx + cp * dy);
    let lambda = x1 * x1 / (rx * rx) + y1 * y1 / (ry * ry);
    if lambda > 1.0 {
        let s = lambda.sqrt();
        rx *= s;
        ry *= s;
    }
    let (rx2, ry2) = (rx * rx, ry * ry);
    let den = rx2 * y1 * y1 + ry2 * x1 * x1;
    if den == 0.0 || !den.is_finite() {
        p.line_to(to.x, to.y);
        return;
    }
    let coef = ((rx2 * ry2 - den) / den).max(0.0).sqrt() * if large == sweep { -1.0 } else { 1.0 };
    let (cxp, cyp) = (coef * rx * y1 / ry, -coef * ry * x1 / rx);
    let cx = cp * cxp - sp * cyp + f64::from(from.x + to.x) / 2.0;
    let cy = sp * cxp + cp * cyp + f64::from(from.y + to.y) / 2.0;
    let ang = |ux: f64, uy: f64| crate::fmath::atan2(uy as f32, ux as f32) as f64;
    let t1 = ang((x1 - cxp) / rx, (y1 - cyp) / ry);
    let t2 = ang((-x1 - cxp) / rx, (-y1 - cyp) / ry);
    let mut dt = t2 - t1;
    let tau = std::f64::consts::TAU;
    if !sweep && dt > 0.0 {
        dt -= tau;
    } else if sweep && dt < 0.0 {
        dt += tau;
    }
    let n = (dt.abs() / (std::f64::consts::FRAC_PI_2) - 1.0e-9).ceil().fclamp(1.0, 4.0) as usize;
    let step = dt / n as f64;
    let k = 4.0 / 3.0 * { let (s, c) = sin_cos(step / 4.0); s / c };
    let pt = |t: f64, d: f64| {
        // The point of the ellipse at angle `t` (d = 0) or its tangent scaled by `k` (d = 1).
        let (s, c) = sin_cos(t);
        let (ex, ey) = if d == 0.0 { (rx * c, ry * s) } else { (-rx * s, ry * c) };
        (cp * ex - sp * ey, sp * ex + cp * ey)
    };
    for i in 0..n {
        let (a, b) = (t1 + step * i as f64, t1 + step * (i + 1) as f64);
        let (p0, d0) = (pt(a, 0.0), pt(a, 1.0));
        let (p1, d1) = (pt(b, 0.0), pt(b, 1.0));
        let end = if i + 1 == n { (f64::from(to.x), f64::from(to.y)) } else { (cx + p1.0, cy + p1.1) };
        p.cubic_to(
            (cx + p0.0 + k * d0.0) as f32,
            (cy + p0.1 + k * d0.1) as f32,
            (cx + p1.0 - k * d1.0) as f32,
            (cy + p1.1 - k * d1.1) as f32,
            end.0 as f32,
            end.1 as f32,
        );
    }
}

/// A transform list (`translate(10 20) rotate(45)`); `None` for a syntax error (the attribute is then ignored).
pub fn transform(s: &str) -> Option<Transform> {
    let mut c = Cur::new(s);
    let mut m = Transform::IDENTITY;
    c.ws();
    let mut count = 0;
    while !c.end() {
        let start = c.i;
        while c.peek().is_ascii_alphabetic() {
            c.i += 1;
        }
        let name = s.get(start..c.i)?;
        c.ws();
        if c.peek() != b'(' {
            return None;
        }
        c.i += 1;
        let mut args = Vec::new();
        c.ws();
        while c.peek() != b')' {
            args.push(c.number()?);
            c.sep();
            if args.len() > 6 {
                return None;
            }
        }
        c.i += 1;
        let a = |i: usize| args.get(i).copied();
        let t = match (name, args.len()) {
            ("matrix", 6) => Transform { a: a(0)?, b: a(1)?, c: a(2)?, d: a(3)?, e: a(4)?, f: a(5)? },
            ("translate", 1 | 2) => Transform::translate(a(0)?, a(1).unwrap_or(0.0)),
            ("scale", 1 | 2) => Transform::scale(a(0)?, a(1).unwrap_or(a(0)?)),
            ("rotate", 1 | 3) => {
                let (s, co) = sin_cos(f64::from(a(0)?).to_radians());
                let r = Transform { a: co as f32, b: s as f32, c: -s as f32, d: co as f32, e: 0.0, f: 0.0 };
                match (a(1), a(2)) {
                    (Some(x), Some(y)) => Transform::translate(x, y).compose(&r).compose(&Transform::translate(-x, -y)),
                    _ => r,
                }
            }
            ("skewX", 1) => {
                let (s, co) = sin_cos(f64::from(a(0)?).to_radians());
                Transform { c: (s / co) as f32, ..Transform::IDENTITY }
            }
            ("skewY", 1) => {
                let (s, co) = sin_cos(f64::from(a(0)?).to_radians());
                Transform { b: (s / co) as f32, ..Transform::IDENTITY }
            }
            _ => return None,
        };
        m = m.compose(&t);
        c.sep();
        count += 1;
        if count > 64 {
            return None;
        }
    }
    Some(m)
}

#[cfg(test)]
#[allow(clippy::indexing_slicing, clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::vector::Seg;

    #[test]
    fn numbers_and_lengths() {
        let mut c = Cur::new("1.5e2,-.5.25 1em");
        assert_eq!(c.number(), Some(150.0));
        c.sep();
        assert_eq!(c.number(), Some(-0.5));
        assert_eq!(c.number(), Some(0.25));
        c.ws();
        assert_eq!(c.number(), Some(1.0));
        assert_eq!(c.rest(), "em");
        assert_eq!(length("12pt"), Some((16.0, Unit::Px)));
        assert_eq!(length("50%"), Some((50.0, Unit::Pct)));
        assert_eq!(length("2em"), Some((2.0, Unit::Em)));
        assert_eq!(length("1e"), None);
        assert_eq!(numbers("1, 2 3", 10), Some(vec![1.0, 2.0, 3.0]));
        assert_eq!(numbers("1 x", 10), None);
    }

    #[test]
    fn path_commands_implicit_repeats_and_errors() {
        let (p, ok) = path("M10 20 30 40 h5 v-5 z", 100);
        assert!(ok);
        assert_eq!(p.segs.len(), 5);
        assert_eq!(p.segs[1], Seg::Line(Pt::new(30.0, 40.0)));
        assert_eq!(p.segs[2], Seg::Line(Pt::new(35.0, 40.0)));
        // Rendered up to the error: the broken `L 5` is dropped, `M` is kept.
        let (p, ok) = path("M1 2 L 3 4 L 5", 100);
        assert!(!ok);
        assert_eq!(p.segs.len(), 2);
        // The first command must be a moveto.
        assert!(path("L 1 2", 100).0.is_empty());
        // Numbers after z are an error; a command after z starts at the subpath start.
        let (p, ok) = path("M0 0 L10 0 z 5 5", 100);
        assert!(!ok && p.segs.len() == 3);
        let (p, _) = path("M0 0 L10 0 z l 1 1", 100);
        assert_eq!(p.segs.last(), Some(&Seg::Line(Pt::new(1.0, 1.0))));
        // Flags can be glued to the numbers after them.
        let (p, ok) = path("M0 0a5 5 0 1110 0", 100);
        assert!(ok && p.segs.len() > 1);
        // The cap stops a long path.
        let (p, ok) = path(&"M0 0L1 1".repeat(100), 10);
        assert!(!ok && p.segs.len() == 10);
    }

    #[test]
    fn smooth_curves_reflect_the_previous_control_point() {
        let (p, _) = path("M0 0 C 0 10 10 10 10 0 S 20 -10 20 0", 100);
        assert_eq!(p.segs[2], Seg::Cubic(Pt::new(10.0, -10.0), Pt::new(20.0, -10.0), Pt::new(20.0, 0.0)));
        let (p, _) = path("M0 0 Q 5 10 10 0 T 20 0", 100);
        assert_eq!(p.segs[2], Seg::Quad(Pt::new(15.0, -10.0), Pt::new(20.0, 0.0)));
    }

    #[test]
    fn a_half_circle_arc_passes_through_its_top() {
        let (p, ok) = path("M0 0 A 10 10 0 0 1 20 0", 100);
        assert!(ok);
        let polys = p.flatten(0.01);
        let top = polys[0].pts.iter().map(|q| q.y).fold(f32::MAX, f32::min);
        assert!((top + 10.0).abs() < 0.05, "{top}");
        let last = polys[0].pts.last().unwrap();
        assert_eq!((last.x, last.y), (20.0, 0.0));
        // Radii too small are scaled up.
        assert!(path("M0 0 A 1 1 0 0 1 20 0", 100).0.segs.len() > 1);
    }

    #[test]
    fn transform_lists() {
        let t = transform("translate(10, 20) scale(2)").unwrap();
        assert_eq!(t.apply(Pt::new(1.0, 1.0)), Pt::new(12.0, 22.0));
        let r = transform("rotate(90 10 10)").unwrap();
        let q = r.apply(Pt::new(20.0, 10.0));
        assert!((q.x - 10.0).abs() < 1.0e-4 && (q.y - 20.0).abs() < 1.0e-4);
        assert!(transform("translate(1").is_none());
        assert!(transform("foo(1)").is_none());
        assert_eq!(transform("").unwrap(), Transform::IDENTITY);
    }
}
