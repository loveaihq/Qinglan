//! CSS values: lengths, percentages and `calc()`/`min()`/`max()`/`clamp()` (css-values-4).

use std::rc::Rc;

use super::syntax::{BlockKind, Cv, split_commas, trim_ws};
use super::tokenizer::Token;

/// calc() nesting limit (L1 task sheet).
pub const MAX_CALC_DEPTH: usize = 32;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LenUnit {
    Px,
    Cm,
    Mm,
    Q,
    In,
    Pc,
    Pt,
    Em,
    Rem,
    Ex,
    Ch,
    Vw,
    Vh,
    Vmin,
    Vmax,
}

impl LenUnit {
    pub fn from_name(s: &str) -> Option<LenUnit> {
        // dvh/svh/lvh and friends count as vh (L1 task sheet).
        let u = match s.to_ascii_lowercase().as_str() {
            "px" => LenUnit::Px,
            "cm" => LenUnit::Cm,
            "mm" => LenUnit::Mm,
            "q" => LenUnit::Q,
            "in" => LenUnit::In,
            "pc" => LenUnit::Pc,
            "pt" => LenUnit::Pt,
            "em" => LenUnit::Em,
            "rem" => LenUnit::Rem,
            "ex" => LenUnit::Ex,
            "ch" => LenUnit::Ch,
            "vw" | "dvw" | "svw" | "lvw" => LenUnit::Vw,
            "vh" | "dvh" | "svh" | "lvh" => LenUnit::Vh,
            "vmin" | "dvmin" | "svmin" | "lvmin" => LenUnit::Vmin,
            "vmax" | "dvmax" | "svmax" | "lvmax" => LenUnit::Vmax,
            _ => return None,
        };
        Some(u)
    }

    pub fn name(self) -> &'static str {
        match self {
            LenUnit::Px => "px",
            LenUnit::Cm => "cm",
            LenUnit::Mm => "mm",
            LenUnit::Q => "Q",
            LenUnit::In => "in",
            LenUnit::Pc => "pc",
            LenUnit::Pt => "pt",
            LenUnit::Em => "em",
            LenUnit::Rem => "rem",
            LenUnit::Ex => "ex",
            LenUnit::Ch => "ch",
            LenUnit::Vw => "vw",
            LenUnit::Vh => "vh",
            LenUnit::Vmin => "vmin",
            LenUnit::Vmax => "vmax",
        }
    }
}

/// Everything needed to turn a specified length into pixels.
#[derive(Clone, Copy, Debug)]
pub struct LenCtx {
    /// Font size used for `em` (the element's own, or the parent's when computing `font-size`).
    pub font_size: f32,
    pub root_font_size: f32,
    pub viewport_w: f32,
    pub viewport_h: f32,
}

/// Largest absolute length kept (keeps arithmetic finite; layout clamps further).
pub const MAX_PX: f32 = 1.0e9;

fn clamp_px(v: f32) -> f32 {
    if v.is_nan() { 0.0 } else { v.clamp(-MAX_PX, MAX_PX) }
}

impl LenUnit {
    pub fn to_px(self, v: f32, ctx: &LenCtx) -> f32 {
        let px = match self {
            LenUnit::Px => v,
            LenUnit::Cm => v * 96.0 / 2.54,
            LenUnit::Mm => v * 96.0 / 25.4,
            LenUnit::Q => v * 96.0 / 101.6,
            LenUnit::In => v * 96.0,
            LenUnit::Pc => v * 16.0,
            LenUnit::Pt => v * 96.0 / 72.0,
            LenUnit::Em => v * ctx.font_size,
            LenUnit::Rem => v * ctx.root_font_size,
            // No font metrics in the engine core: ex and ch are half an em (decisions.md).
            LenUnit::Ex | LenUnit::Ch => v * ctx.font_size * 0.5,
            LenUnit::Vw => v * ctx.viewport_w / 100.0,
            LenUnit::Vh => v * ctx.viewport_h / 100.0,
            LenUnit::Vmin => v * ctx.viewport_w.min(ctx.viewport_h) / 100.0,
            LenUnit::Vmax => v * ctx.viewport_w.max(ctx.viewport_h) / 100.0,
        };
        clamp_px(px)
    }
}

/// A calc() expression tree.
#[derive(Clone, Debug, PartialEq)]
pub enum Calc {
    Num(f32),
    Len(f32, LenUnit),
    Pct(f32),
    Sum(Vec<Calc>),
    Mul(Box<Calc>, Box<Calc>),
    Div(Box<Calc>, Box<Calc>),
    Min(Vec<Calc>),
    Max(Vec<Calc>),
    Clamp(Box<Calc>, Box<Calc>, Box<Calc>),
}

impl Calc {
    /// Replace every length by pixels.
    pub fn compute(&self, ctx: &LenCtx) -> Calc {
        let c = |x: &Calc| Box::new(x.compute(ctx));
        match self {
            Calc::Num(v) => Calc::Num(*v),
            Calc::Len(v, u) => Calc::Len(u.to_px(*v, ctx), LenUnit::Px),
            Calc::Pct(p) => Calc::Pct(*p),
            Calc::Sum(t) => Calc::Sum(t.iter().map(|x| x.compute(ctx)).collect()),
            Calc::Mul(a, b) => Calc::Mul(c(a), c(b)),
            Calc::Div(a, b) => Calc::Div(c(a), c(b)),
            Calc::Min(t) => Calc::Min(t.iter().map(|x| x.compute(ctx)).collect()),
            Calc::Max(t) => Calc::Max(t.iter().map(|x| x.compute(ctx)).collect()),
            Calc::Clamp(a, b, d) => Calc::Clamp(c(a), c(b), c(d)),
        }
    }

    /// True if a percentage occurs anywhere.
    pub fn has_percent(&self) -> bool {
        match self {
            Calc::Pct(_) => true,
            Calc::Num(_) | Calc::Len(..) => false,
            Calc::Sum(t) | Calc::Min(t) | Calc::Max(t) => t.iter().any(Calc::has_percent),
            Calc::Mul(a, b) | Calc::Div(a, b) => a.has_percent() || b.has_percent(),
            Calc::Clamp(a, b, c) => a.has_percent() || b.has_percent() || c.has_percent(),
        }
    }

    /// Evaluate a computed expression (lengths in px) with `percent_base` for percentages.
    pub fn eval(&self, percent_base: f32) -> f32 {
        let v = match self {
            Calc::Num(v) | Calc::Len(v, _) => *v,
            Calc::Pct(p) => p * percent_base / 100.0,
            Calc::Sum(t) => t.iter().map(|x| x.eval(percent_base)).sum(),
            Calc::Mul(a, b) => a.eval(percent_base) * b.eval(percent_base),
            Calc::Div(a, b) => {
                let d = b.eval(percent_base);
                if d == 0.0 { f32::MAX.copysign(a.eval(percent_base)) } else { a.eval(percent_base) / d }
            }
            Calc::Min(t) => t.iter().map(|x| x.eval(percent_base)).fold(f32::INFINITY, f32::min),
            Calc::Max(t) => t.iter().map(|x| x.eval(percent_base)).fold(f32::NEG_INFINITY, f32::max),
            Calc::Clamp(lo, v, hi) => {
                let (lo, v, hi) = (lo.eval(percent_base), v.eval(percent_base), hi.eval(percent_base));
                // clamp(MIN, VAL, MAX) = max(MIN, min(VAL, MAX))
                lo.max(v.min(hi))
            }
        };
        clamp_px(v)
    }

    pub fn to_css(&self) -> String {
        match self {
            Calc::Num(v) => fmt_num(*v),
            Calc::Len(v, u) => format!("{}{}", fmt_num(*v), u.name()),
            Calc::Pct(p) => format!("{}%", fmt_num(*p)),
            Calc::Sum(t) => {
                let parts: Vec<String> = t.iter().map(Calc::to_css).collect();
                format!("({})", parts.join(" + "))
            }
            Calc::Mul(a, b) => format!("({} * {})", a.to_css(), b.to_css()),
            Calc::Div(a, b) => format!("({} / {})", a.to_css(), b.to_css()),
            Calc::Min(t) => format!("min({})", t.iter().map(Calc::to_css).collect::<Vec<_>>().join(", ")),
            Calc::Max(t) => format!("max({})", t.iter().map(Calc::to_css).collect::<Vec<_>>().join(", ")),
            Calc::Clamp(a, b, c) => format!("clamp({}, {}, {})", a.to_css(), b.to_css(), c.to_css()),
        }
    }
}

pub fn fmt_num(v: f32) -> String {
    if v == v.trunc() && v.abs() < 1.0e9 {
        format!("{}", v as i64)
    } else {
        let s = format!("{v:.4}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

/// A length or a percentage (or a calc() of them). Computed values only contain px lengths.
#[derive(Clone, Debug, PartialEq)]
pub enum Lp {
    Len(f32, LenUnit),
    Pct(f32),
    Calc(Rc<Calc>),
}

impl Lp {
    pub const ZERO: Lp = Lp::Len(0.0, LenUnit::Px);

    pub fn px(v: f32) -> Lp {
        Lp::Len(v, LenUnit::Px)
    }

    /// Computed value: lengths in px; a calc() without percentages collapses to a length.
    pub fn compute(&self, ctx: &LenCtx) -> Lp {
        match self {
            Lp::Len(v, u) => Lp::Len(u.to_px(*v, ctx), LenUnit::Px),
            Lp::Pct(p) => Lp::Pct(*p),
            Lp::Calc(c) => {
                let cc = c.compute(ctx);
                if cc.has_percent() { Lp::Calc(Rc::new(cc)) } else { Lp::px(cc.eval(0.0)) }
            }
        }
    }

    /// The px value if this is a plain (computed) length.
    pub fn as_px(&self) -> Option<f32> {
        match self {
            Lp::Len(v, LenUnit::Px) => Some(*v),
            _ => None,
        }
    }

    /// Resolve a computed value against a percentage base.
    pub fn resolve(&self, percent_base: f32) -> f32 {
        match self {
            Lp::Len(v, _) => *v,
            Lp::Pct(p) => p * percent_base / 100.0,
            Lp::Calc(c) => c.eval(percent_base),
        }
    }

    pub fn is_negative_length(&self) -> bool {
        matches!(self, Lp::Len(v, _) if *v < 0.0) || matches!(self, Lp::Pct(p) if *p < 0.0)
    }

    pub fn to_css(&self) -> String {
        match self {
            Lp::Len(v, u) => format!("{}{}", fmt_num(*v), u.name()),
            Lp::Pct(p) => format!("{}%", fmt_num(*p)),
            Lp::Calc(c) => format!("calc{}", c.to_css()),
        }
    }
}

// -----------------------------------------------------------------------------------------------
// calc() parsing

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ty {
    Num,
    Len,
    Pct,
    LenPct,
}

impl Ty {
    fn is_dim(self) -> bool {
        self != Ty::Num
    }
    fn combine(self, other: Ty) -> Option<Ty> {
        match (self, other) {
            (Ty::Num, Ty::Num) => Some(Ty::Num),
            (Ty::Num, _) | (_, Ty::Num) => None,
            (a, b) if a == b => Some(a),
            _ => Some(Ty::LenPct),
        }
    }
}

struct Cursor<'a> {
    items: &'a [Cv],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn skip_ws(&mut self) {
        while self.items.get(self.pos).is_some_and(Cv::is_ws) {
            self.pos += 1;
        }
    }
    fn peek(&self) -> Option<&'a Cv> {
        self.items.get(self.pos)
    }
}

/// Parse `calc(...)`, `min(...)`, `max(...)`, `clamp(...)` given the function name and arguments.
pub fn parse_math_function(name: &str, items: &[Cv]) -> Option<(Calc, Ty)> {
    parse_function(name, items, 1)
}

fn parse_function(name: &str, items: &[Cv], depth: usize) -> Option<(Calc, Ty)> {
    if depth > MAX_CALC_DEPTH {
        return None;
    }
    match name.to_ascii_lowercase().as_str() {
        "calc" | "-webkit-calc" | "-moz-calc" => parse_sum(items, depth),
        "min" | "max" => {
            let args = split_commas(items);
            let mut nodes = Vec::new();
            let mut ty: Option<Ty> = None;
            for a in args {
                let (n, t) = parse_sum(a, depth)?;
                ty = Some(match ty {
                    None => t,
                    Some(prev) => prev.combine(t)?,
                });
                nodes.push(n);
            }
            let node = if name.eq_ignore_ascii_case("min") { Calc::Min(nodes) } else { Calc::Max(nodes) };
            Some((node, ty?))
        }
        "clamp" => {
            let args = split_commas(items);
            if args.len() != 3 {
                return None;
            }
            let mut parts = Vec::new();
            let mut ty: Option<Ty> = None;
            for a in args {
                let (n, t) = parse_sum(a, depth)?;
                ty = Some(match ty {
                    None => t,
                    Some(prev) => prev.combine(t)?,
                });
                parts.push(n);
            }
            let mut it = parts.into_iter();
            Some((Calc::Clamp(Box::new(it.next()?), Box::new(it.next()?), Box::new(it.next()?)), ty?))
        }
        _ => None,
    }
}

fn parse_sum(items: &[Cv], depth: usize) -> Option<(Calc, Ty)> {
    let mut cur = Cursor { items: trim_ws(items), pos: 0 };
    let (first, mut ty) = parse_product(&mut cur, depth)?;
    let mut terms = vec![first];
    loop {
        // A sum operator needs white space on both sides.
        let had_ws = cur.peek().is_some_and(Cv::is_ws);
        cur.skip_ws();
        let Some(op) = cur.peek() else { break };
        let negative = if op.is_delim('+') {
            false
        } else if op.is_delim('-') {
            true
        } else {
            return None;
        };
        if !had_ws {
            return None;
        }
        cur.pos += 1;
        if !cur.peek().is_some_and(Cv::is_ws) {
            return None;
        }
        cur.skip_ws();
        let (t, tty) = parse_product(&mut cur, depth)?;
        ty = ty.combine(tty)?;
        terms.push(if negative { Calc::Mul(Box::new(Calc::Num(-1.0)), Box::new(t)) } else { t });
    }
    if cur.pos < cur.items.len() {
        return None;
    }
    if terms.len() == 1 { Some((terms.pop()?, ty)) } else { Some((Calc::Sum(terms), ty)) }
}

fn parse_product(cur: &mut Cursor<'_>, depth: usize) -> Option<(Calc, Ty)> {
    let (mut node, mut ty) = parse_value(cur, depth)?;
    loop {
        let save = cur.pos;
        cur.skip_ws();
        let Some(op) = cur.peek() else {
            cur.pos = save;
            break;
        };
        if op.is_delim('*') {
            cur.pos += 1;
            cur.skip_ws();
            let (rhs, rty) = parse_value(cur, depth)?;
            ty = match (ty, rty) {
                (Ty::Num, t) | (t, Ty::Num) => t,
                _ => return None,
            };
            node = Calc::Mul(Box::new(node), Box::new(rhs));
        } else if op.is_delim('/') {
            cur.pos += 1;
            cur.skip_ws();
            let (rhs, rty) = parse_value(cur, depth)?;
            if rty != Ty::Num {
                return None;
            }
            node = Calc::Div(Box::new(node), Box::new(rhs));
        } else {
            cur.pos = save;
            break;
        }
    }
    Some((node, ty))
}

fn parse_value(cur: &mut Cursor<'_>, depth: usize) -> Option<(Calc, Ty)> {
    let cv = cur.peek()?;
    cur.pos += 1;
    match cv {
        Cv::Tok(Token::Number { value, .. }) => Some((Calc::Num(*value as f32), Ty::Num)),
        Cv::Tok(Token::Percentage { value }) => Some((Calc::Pct(*value as f32), Ty::Pct)),
        Cv::Tok(Token::Dimension { value, unit, .. }) => {
            let u = LenUnit::from_name(unit)?;
            Some((Calc::Len(*value as f32, u), Ty::Len))
        }
        Cv::Block(BlockKind::Paren, inner) => {
            if depth + 1 > MAX_CALC_DEPTH {
                return None;
            }
            parse_sum(inner, depth + 1)
        }
        Cv::Func(name, inner) => parse_function(name, inner, depth + 1),
        _ => None,
    }
}

/// Evaluate a unit-free calc() (numbers and percentages): (value, is_percentage).
pub fn eval_calc_number_or_percent(items: &[Cv]) -> Option<(f64, bool)> {
    let (node, ty) = parse_sum(items, 1)?;
    match ty {
        Ty::Num => Some((node.eval(0.0) as f64, false)),
        Ty::Pct => Some((node.eval(100.0) as f64, true)),
        _ => None,
    }
}

// -----------------------------------------------------------------------------------------------
// Single-value parsers

/// A `<number>` (including `calc()` of numbers).
pub fn parse_number(cv: &Cv) -> Option<f32> {
    match cv {
        Cv::Tok(Token::Number { value, .. }) => Some(*value as f32),
        Cv::Func(name, items) => {
            let (n, ty) = parse_math_function(name, items)?;
            (ty == Ty::Num).then(|| n.eval(0.0))
        }
        _ => None,
    }
}

/// An `<integer>` (calc() results are rounded).
pub fn parse_integer(cv: &Cv) -> Option<i32> {
    match cv {
        Cv::Tok(Token::Number { value, is_int: true, .. }) => Some((*value).clamp(i32::MIN as f64, i32::MAX as f64) as i32),
        Cv::Func(name, items) => {
            let (n, ty) = parse_math_function(name, items)?;
            (ty == Ty::Num).then(|| n.eval(0.0).round().clamp(i32::MIN as f32, i32::MAX as f32) as i32)
        }
        _ => None,
    }
}

/// A `<length>` (no percentage). Unitless zero is allowed only when `zero_ok`.
pub fn parse_length(cv: &Cv, zero_ok: bool) -> Option<Lp> {
    match cv {
        Cv::Tok(Token::Dimension { value, unit, .. }) => Some(Lp::Len(*value as f32, LenUnit::from_name(unit)?)),
        Cv::Tok(Token::Number { value, .. }) if zero_ok && *value == 0.0 => Some(Lp::ZERO),
        Cv::Func(name, items) => {
            let (n, ty) = parse_math_function(name, items)?;
            (ty == Ty::Len).then(|| Lp::Calc(Rc::new(n)))
        }
        _ => None,
    }
}

/// A `<length-percentage>`.
pub fn parse_length_percentage(cv: &Cv, zero_ok: bool) -> Option<Lp> {
    match cv {
        Cv::Tok(Token::Percentage { value }) => Some(Lp::Pct(*value as f32)),
        Cv::Func(name, items) => {
            let (n, ty) = parse_math_function(name, items)?;
            match ty {
                Ty::Len | Ty::Pct | Ty::LenPct => Some(Lp::Calc(Rc::new(n))),
                Ty::Num => None,
            }
        }
        _ => parse_length(cv, zero_ok),
    }
}

impl Ty {
    pub fn is_length_percentage(self) -> bool {
        self.is_dim()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::css::syntax::parse_cvs;

    fn lp(s: &str) -> Option<Lp> {
        let cvs = parse_cvs(s);
        parse_length_percentage(trim_ws(&cvs).first()?, true)
    }

    fn ctx() -> LenCtx {
        LenCtx { font_size: 16.0, root_font_size: 16.0, viewport_w: 1280.0, viewport_h: 800.0 }
    }

    fn px(s: &str, base: f32) -> Option<f32> {
        lp(s).map(|v| v.compute(&ctx()).resolve(base))
    }

    #[test]
    fn calc_basics() {
        assert_eq!(px("calc(1px + 2px)", 0.0), Some(3.0));
        assert_eq!(px("calc(100% - 20px)", 200.0), Some(180.0));
        assert_eq!(px("calc(2 * (3px + 1px))", 0.0), Some(8.0));
        assert_eq!(px("calc(10px / 4)", 0.0), Some(2.5));
        assert_eq!(px("min(10px, 5px, 20px)", 0.0), Some(5.0));
        assert_eq!(px("max(1em, 10px)", 0.0), Some(16.0));
        assert_eq!(px("clamp(10px, 50%, 30px)", 100.0), Some(30.0));
        assert_eq!(px("calc(1em + 1rem + 1vw)", 0.0), Some(16.0 + 16.0 + 12.8));
    }

    #[test]
    fn calc_errors() {
        assert_eq!(lp("calc(1px+2px)"), None); // sum needs spaces
        assert_eq!(lp("calc(1px * 2px)"), None);
        assert_eq!(lp("calc(1px + 2)"), None);
        assert_eq!(lp("calc(1px / 2px)"), None);
        assert_eq!(lp("calc()"), None);
    }

    #[test]
    fn calc_depth_limit() {
        let deep = |n: usize| format!("{}1px{}", "calc(".repeat(n), ")".repeat(n));
        assert!(lp(&deep(20)).is_some());
        assert!(lp(&deep(40)).is_none());
    }

    #[test]
    fn units() {
        assert_eq!(px("1in", 0.0), Some(96.0));
        assert_eq!(px("72pt", 0.0), Some(96.0));
        assert_eq!(px("10%", 50.0), Some(5.0));
        assert_eq!(px("0", 0.0), Some(0.0));
        assert_eq!(px("1dvh", 0.0), Some(8.0));
    }
}
