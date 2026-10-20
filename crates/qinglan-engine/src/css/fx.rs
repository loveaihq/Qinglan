//! Effect properties (L2 step 2c-2): `opacity` (css-color-4 12.1), `box-shadow` (css-backgrounds-3 6.1), `text-shadow`
//! (css-text-decor-3 4.1), `transform` and `transform-origin` (css-transforms-1 6, 7, 8). 3D functions keep their 2D part
//! (the upper left 2x2 and the translation of the 4x4 matrix).

use std::rc::Rc;

use super::color::{ColorValue, parse_color_cv};
use super::syntax::{Cv, split_commas, trim_ws};
use super::tokenizer::Token;
use super::types::Value;
use super::values::{LenCtx, Lp, parse_length, parse_length_percentage, parse_number};
use crate::fmath::sin_cos;
use crate::vector::Transform;

/// A shadow list longer than this is cut; a transform list too.
const MAX_LIST: usize = 16;

/// One `<shadow>`: offsets, blur radius, spread distance (all computed lengths), color and `inset` (`box-shadow` only).
#[derive(Clone, PartialEq)]
pub struct Shadow {
    pub dx: Lp,
    pub dy: Lp,
    pub blur: Lp,
    pub spread: Lp,
    pub color: ColorValue,
    pub inset: bool,
}

// (Only the names: derived `Debug` for these is a few KB of executable nobody reads.)
impl std::fmt::Debug for Shadow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Shadow")
    }
}

impl std::fmt::Debug for TFn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("TFn")
    }
}

impl std::fmt::Debug for Fx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Fx")
    }
}

impl Shadow {
    pub fn compute(&self, ctx: &LenCtx) -> Shadow {
        Shadow { dx: self.dx.compute(ctx), dy: self.dy.compute(ctx), blur: self.blur.compute(ctx), spread: self.spread.compute(ctx), ..self.clone() }
    }
    /// (dx, dy, blur, spread) in px.
    pub fn px(&self) -> [f32; 4] {
        [self.dx.resolve(0.0), self.dy.resolve(0.0), self.blur.resolve(0.0).max(0.0), self.spread.resolve(0.0)]
    }
}

/// One transform function; angles in radians.
#[derive(Clone, PartialEq)]
pub enum TFn {
    Matrix([f32; 6]),
    Translate(Lp, Lp),
    Scale(f32, f32),
    Rotate(f32),
    Skew(f32, f32),
}

impl TFn {
    pub fn compute(&self, ctx: &LenCtx) -> TFn {
        match self {
            TFn::Translate(x, y) => TFn::Translate(x.compute(ctx), y.compute(ctx)),
            o => o.clone(),
        }
    }
}

/// The effect properties of an element; `MiscStyle::fx` is `None` while all are initial.
#[derive(Clone, PartialEq)]
pub struct Fx {
    /// `opacity` as 0..=255.
    pub opacity: u8,
    pub shadows: Vec<Shadow>,
    pub xform: Vec<TFn>,
    pub origin: (Lp, Lp),
}

impl Fx {
    pub fn initial() -> Fx {
        Fx { opacity: 255, shadows: Vec::new(), xform: Vec::new(), origin: (Lp::Pct(50.0), Lp::Pct(50.0)) }
    }

    /// The transform of a box of `w` x `h` px about its `transform-origin`, in the box's own coordinates
    /// (css-transforms-1 7: the transform list applied about the origin; the reference box is the border box).
    pub fn matrix(&self, w: f32, h: f32) -> Transform {
        let mut m = Transform::IDENTITY;
        for f in &self.xform {
            let t = match f {
                TFn::Matrix([a, b, c, d, e, f]) => Transform { a: *a, b: *b, c: *c, d: *d, e: *e, f: *f },
                TFn::Translate(x, y) => Transform::translate(x.resolve(w), y.resolve(h)),
                TFn::Scale(x, y) => Transform::scale(*x, *y),
                TFn::Rotate(a) => {
                    let (s, c) = sin_cos(f64::from(*a));
                    Transform { a: c as f32, b: s as f32, c: -s as f32, d: c as f32, e: 0.0, f: 0.0 }
                }
                TFn::Skew(ax, ay) => {
                    let tan = |a: f32| {
                        let (s, c) = sin_cos(f64::from(a));
                        (s / c).clamp(-1.0e6, 1.0e6) as f32
                    };
                    Transform { a: 1.0, b: tan(*ay), c: tan(*ax), d: 1.0, e: 0.0, f: 0.0 }
                }
            };
            m = m.compose(&t);
        }
        let (ox, oy) = (self.origin.0.resolve(w), self.origin.1.resolve(h));
        Transform::translate(ox, oy).compose(&m).compose(&Transform::translate(-ox, -oy))
    }
}

// ---- parsing -----------------------------------------------------------------------------------------

/// `opacity`: `<number> | <percentage>`, clamped to 0..=1 (css-color-4 12.1).
pub fn parse_opacity(c: &Cv) -> Option<Value> {
    let v = match c {
        Cv::Tok(Token::Percentage { value }) => *value as f32 / 100.0,
        _ => parse_number(c)?,
    };
    v.is_finite().then(|| Value::Opacity(v.clamp(0.0, 1.0)))
}

/// `box-shadow` / `text-shadow`: `none` or a comma-separated list of `<color>? && <length>{2,4} && inset?` (text: no spread, no `inset`).
#[inline(never)]
pub fn parse_shadows(cvs: &[Cv], text: bool) -> Option<Value> {
    let t = trim_ws(cvs);
    if matches!(t, [c] if c.is_ident("none")) {
        return Some(Value::Shadows(None));
    }
    let mut out = Vec::new();
    for part in split_commas(t) {
        let (mut lens, mut color, mut inset) = (Vec::new(), None, false);
        for c in part.iter().filter(|c| !c.is_ws()) {
            if c.is_ident("inset") && !text && !inset {
                inset = true;
            } else if let Some(l) = parse_length(c, true) {
                lens.push(l);
            } else if color.is_none() && let Some(col) = parse_color_cv(c) {
                color = Some(col);
            } else {
                return None;
            }
        }
        let mut it = lens.into_iter();
        let (dx, dy) = (it.next()?, it.next()?);
        let blur = it.next().unwrap_or(Lp::ZERO);
        let spread = it.next().unwrap_or(Lp::ZERO);
        if it.next().is_some() || blur.is_negative_length() || (text && spread != Lp::ZERO) {
            return None;
        }
        if out.len() < MAX_LIST {
            out.push(Shadow { dx, dy, blur, spread, color: color.unwrap_or(ColorValue::CurrentColor), inset });
        }
    }
    (!out.is_empty()).then(|| Value::Shadows(Some(Rc::new(out))))
}

fn angle(c: &Cv) -> Option<f32> {
    match c {
        Cv::Tok(Token::Dimension { value, unit, .. }) => {
            let v = *value as f32;
            let deg = match unit.to_ascii_lowercase().as_str() {
                "deg" => v,
                "grad" => v * 0.9,
                "rad" => v.to_degrees(),
                "turn" => v * 360.0,
                _ => return None,
            };
            Some(deg.to_radians())
        }
        Cv::Tok(Token::Number { value, .. }) if *value == 0.0 => Some(0.0),
        _ => None,
    }
}

/// The 2D part of `rotate3d(x, y, z, a)` (css-transforms-2 rotate3d matrix): a zero axis does not rotate.
fn rotate3d(x: f32, y: f32, z: f32, a: f32) -> TFn {
    let len = (x * x + y * y + z * z).sqrt();
    if !(len > 0.0 && len.is_finite()) {
        return TFn::Scale(1.0, 1.0);
    }
    let (x, y, z) = (x / len, y / len, z / len);
    let (s, c) = sin_cos(f64::from(a));
    let (s, c) = (s as f32, c as f32);
    let t = 1.0 - c;
    TFn::Matrix([c + x * x * t, x * y * t + z * s, x * y * t - z * s, c + y * y * t, 0.0, 0.0])
}

/// `transform`: `none` or a list of transform functions.
#[inline(never)]
pub fn parse_transform(cvs: &[Cv]) -> Option<Value> {
    let t = trim_ws(cvs);
    if matches!(t, [c] if c.is_ident("none")) {
        return Some(Value::Transform(None));
    }
    let mut list = Vec::new();
    for c in t.iter().filter(|c| !c.is_ws()) {
        let Cv::Func(name, args) = c else { return None };
        let a: Vec<&Cv> = args.iter().filter(|x| !x.is_ws() && !x.is_comma()).collect();
        let lp = |i: usize| a.get(i).and_then(|c| parse_length_percentage(c, true));
        let num = |i: usize| a.get(i).and_then(|c| parse_number(c));
        // `scale()` takes numbers and percentages (css-transforms-2 scale()).
        let sc = |i: usize| a.get(i).and_then(|c| match c {
            Cv::Tok(Token::Percentage { value }) => Some(*value as f32 / 100.0),
            c => parse_number(c),
        });
        let ang = |i: usize| a.get(i).and_then(|c| angle(c));
        let one = TFn::Scale(1.0, 1.0);
        let f = match (name.to_ascii_lowercase().as_str(), a.len()) {
            ("matrix", 6) => TFn::Matrix([num(0)?, num(1)?, num(2)?, num(3)?, num(4)?, num(5)?]),
            ("matrix3d", 16) => {
                let m: Vec<f32> = (0..16).map(num).collect::<Option<_>>()?;
                let g = |i: usize| m.get(i).copied().unwrap_or(0.0);
                TFn::Matrix([g(0), g(1), g(4), g(5), g(12), g(13)])
            }
            ("translate", 1 | 2) => TFn::Translate(lp(0)?, if a.len() == 2 { lp(1)? } else { Lp::ZERO }),
            ("translatex", 1) => TFn::Translate(lp(0)?, Lp::ZERO),
            ("translatey", 1) => TFn::Translate(Lp::ZERO, lp(0)?),
            ("translatez" | "perspective", 1) => one,
            ("translate3d", 3) => TFn::Translate(lp(0)?, lp(1)?),
            ("scale", 1 | 2) => {
                let x = sc(0)?;
                TFn::Scale(x, if a.len() == 2 { sc(1)? } else { x })
            }
            ("scalex", 1) => TFn::Scale(sc(0)?, 1.0),
            ("scaley", 1) => TFn::Scale(1.0, sc(0)?),
            // A zero scale along z makes the 3D matrix singular: nothing is displayed.
            ("scalez", 1) => {
                if sc(0)? == 0.0 { TFn::Scale(0.0, 0.0) } else { one }
            }
            ("scale3d", 3) => {
                let (x, y, z) = (sc(0)?, sc(1)?, sc(2)?);
                if z == 0.0 { TFn::Scale(0.0, 0.0) } else { TFn::Scale(x, y) }
            }
            ("rotate" | "rotatez", 1) => TFn::Rotate(ang(0)?),
            ("rotatex", 1) => rotate3d(1.0, 0.0, 0.0, ang(0)?),
            ("rotatey", 1) => rotate3d(0.0, 1.0, 0.0, ang(0)?),
            ("rotate3d", 4) => rotate3d(num(0)?, num(1)?, num(2)?, ang(3)?),
            ("skew", 1 | 2) => TFn::Skew(ang(0)?, if a.len() == 2 { ang(1)? } else { 0.0 }),
            ("skewx", 1) => TFn::Skew(ang(0)?, 0.0),
            ("skewy", 1) => TFn::Skew(0.0, ang(0)?),
            _ => return None,
        };
        // `matrix()` and friends must be finite.
        if let TFn::Matrix(m) = &f
            && !m.iter().all(|v| v.is_finite())
        {
            return None;
        }
        if list.len() < MAX_LIST * 2 {
            list.push(f);
        }
    }
    (!list.is_empty()).then(|| Value::Transform(Some(Rc::new(list))))
}

/// `transform-origin`: one to three of `left | center | right | top | bottom | <length-percentage>`; the third is a length that
/// has no 2D meaning (css-transforms-1 8).
pub fn parse_origin(items: &[&Cv]) -> Option<Value> {
    if items.is_empty() || items.len() > 3 {
        return None;
    }
    // (axis: 0 horizontal keyword, 1 vertical keyword, 2 center, 3 length), value
    let one = |c: &Cv| -> Option<(u8, Lp)> {
        match c.ident().map(str::to_ascii_lowercase).as_deref() {
            Some("left") => Some((0, Lp::Pct(0.0))),
            Some("right") => Some((0, Lp::Pct(100.0))),
            Some("top") => Some((1, Lp::Pct(0.0))),
            Some("bottom") => Some((1, Lp::Pct(100.0))),
            Some("center") => Some((2, Lp::Pct(50.0))),
            Some(_) => None,
            None => parse_length_percentage(c, true).map(|l| (3, l)),
        }
    };
    if let Some(z) = items.get(2) {
        parse_length(z, true)?;
    }
    let mid = Lp::Pct(50.0);
    let (x, y) = match (items.first().and_then(|c| one(c)), items.get(1).and_then(|c| one(c))) {
        (Some((a, l)), None) if items.len() == 1 => {
            if a == 1 {
                (mid, l)
            } else {
                (l, mid)
            }
        }
        (Some((a, l)), Some((b, m))) => {
            if a == 1 || b == 0 {
                (m, l)
            } else {
                (l, m)
            }
        }
        _ => return None,
    };
    Some(Value::Origin(x, y))
}
