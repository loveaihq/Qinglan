//! Media queries (mediaqueries-4) and the `@supports` condition (css-conditional-3).

use super::syntax::{BlockKind, Cv};
use super::tokenizer::Token;
use super::values::{LenUnit, parse_length};

/// The environment media queries are evaluated against.
#[derive(Clone, Copy, Debug)]
pub struct MediaEnv {
    pub width: f32,
    pub height: f32,
    /// Device pixels per CSS pixel.
    pub resolution: f32,
    /// `prefers-color-scheme: dark`?
    pub dark: bool,
    pub reduced_motion: bool,
    /// `print` media instead of `screen`.
    pub print: bool,
}

impl MediaEnv {
    pub fn screen(width: f32, height: f32) -> MediaEnv {
        MediaEnv { width, height, resolution: 1.0, dark: false, reduced_motion: false, print: false }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tri {
    True,
    False,
    Unknown,
}

impl Tri {
    fn not(self) -> Tri {
        match self {
            Tri::True => Tri::False,
            Tri::False => Tri::True,
            Tri::Unknown => Tri::Unknown,
        }
    }
    fn from_bool(b: bool) -> Tri {
        if b { Tri::True } else { Tri::False }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Cmp {
    Eq,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Clone, PartialEq, Debug)]
enum FVal {
    /// px
    Len(f32),
    Num(f32),
    Ident(String),
    Ratio(f32),
    /// dppx
    Res(f32),
}

#[derive(Clone, PartialEq, Debug)]
pub struct Feature {
    name: String,
    /// All must hold; empty means "boolean context".
    constraints: Vec<(Cmp, FVal)>,
}

#[derive(Clone, PartialEq, Debug)]
pub enum Cond {
    Not(Box<Cond>),
    And(Vec<Cond>),
    Or(Vec<Cond>),
    Feature(Feature),
    /// General enclosed / unknown: evaluates to unknown (false).
    Unknown,
}

#[derive(Clone, PartialEq, Debug)]
pub struct MediaQuery {
    negated: bool,
    /// `None`: no media type given (condition only), i.e. `all`.
    media_type: Option<String>,
    cond: Option<Cond>,
    /// Malformed queries are `not all`.
    valid: bool,
}

/// A parsed media query list. An empty list matches everything.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct MediaQueryList {
    queries: Vec<MediaQuery>,
}

impl MediaQueryList {
    pub fn all() -> MediaQueryList {
        MediaQueryList { queries: Vec::new() }
    }

    pub fn is_all(&self) -> bool {
        self.queries.is_empty()
    }

    pub fn matches(&self, env: &MediaEnv) -> bool {
        if self.queries.is_empty() {
            return true;
        }
        self.queries.iter().any(|q| q.matches(env))
    }

    /// Parse a comma separated media query list.
    pub fn parse(cvs: &[Cv]) -> MediaQueryList {
        let trimmed = super::syntax::trim_ws(cvs);
        if trimmed.is_empty() {
            return MediaQueryList::all();
        }
        let queries = super::syntax::split_commas(trimmed).into_iter().map(parse_query).collect();
        MediaQueryList { queries }
    }
}

impl MediaQuery {
    fn matches(&self, env: &MediaEnv) -> bool {
        if !self.valid {
            return false;
        }
        let type_ok = match self.media_type.as_deref() {
            None | Some("all") => true,
            Some("screen") => !env.print,
            Some("print") => env.print,
            Some(_) => false,
        };
        let cond = match &self.cond {
            None => Tri::True,
            Some(c) => eval_cond(c, env),
        };
        let both = if type_ok && cond == Tri::True {
            Tri::True
        } else if !type_ok || cond == Tri::False {
            Tri::False
        } else {
            Tri::Unknown
        };
        let r = if self.negated { both.not() } else { both };
        r == Tri::True
    }
}

fn invalid() -> MediaQuery {
    MediaQuery { negated: false, media_type: None, cond: None, valid: false }
}

fn parse_query(cvs: &[Cv]) -> MediaQuery {
    // Conditions nested deeper than the cap are invalid; this also bounds the recursion below.
    if super::syntax::nests_deeper_than(cvs, super::syntax::MAX_CONDITION_NESTING) {
        return invalid();
    }
    let items: Vec<&Cv> = cvs.iter().filter(|c| !c.is_ws()).collect();
    if items.is_empty() {
        return invalid();
    }
    let first_is_cond_start = |i: usize| matches!(items.get(i), Some(Cv::Block(BlockKind::Paren, _)) | Some(Cv::Func(..)));
    // Condition-only form: `(...)`, or `not (...)`.
    if first_is_cond_start(0) || (items.first().is_some_and(|c| c.is_ident("not")) && first_is_cond_start(1)) {
        let owned: Vec<Cv> = items.iter().map(|c| (*c).clone()).collect();
        return match parse_condition(&owned, true) {
            Some(c) => MediaQuery { negated: false, media_type: None, cond: Some(c), valid: true },
            None => invalid(),
        };
    }
    let mut i = 0usize;
    let mut negated = false;
    if let Some(c) = items.first() {
        if c.is_ident("not") {
            negated = true;
            i = 1;
        } else if c.is_ident("only") {
            i = 1;
        }
    }
    let Some(Cv::Tok(Token::Ident(t))) = items.get(i) else { return invalid() };
    let tl = t.to_ascii_lowercase();
    if matches!(tl.as_str(), "not" | "only" | "and" | "or" | "layer") {
        return invalid();
    }
    i += 1;
    let mut cond = None;
    if i < items.len() {
        if !items.get(i).is_some_and(|c| c.is_ident("and")) {
            return invalid();
        }
        let rest: Vec<Cv> = items.iter().skip(i + 1).map(|c| (*c).clone()).collect();
        match parse_condition(&rest, false) {
            Some(c) => cond = Some(c),
            None => return invalid(),
        }
    }
    MediaQuery { negated, media_type: Some(tl), cond, valid: true }
}

/// `<media-condition>` (or `<media-condition-without-or>` when `allow_or` is false).
fn parse_condition(items: &[Cv], allow_or: bool) -> Option<Cond> {
    let items: Vec<&Cv> = items.iter().filter(|c| !c.is_ws()).collect();
    if items.first().is_some_and(|c| c.is_ident("not")) {
        let rest = items.get(1..)?;
        let [one] = rest else { return None };
        return Some(Cond::Not(Box::new(parse_in_parens(one)?)));
    }
    let mut parts = Vec::new();
    let mut op: Option<&str> = None;
    let mut i = 0usize;
    loop {
        parts.push(parse_in_parens(items.get(i)?)?);
        i += 1;
        match items.get(i) {
            None => break,
            Some(c) if c.is_ident("and") => {
                if op == Some("or") {
                    return None;
                }
                op = Some("and");
            }
            Some(c) if c.is_ident("or") && allow_or => {
                if op == Some("and") {
                    return None;
                }
                op = Some("or");
            }
            _ => return None,
        }
        i += 1;
    }
    Some(match (parts.len(), op) {
        (1, _) => parts.pop()?,
        (_, Some("or")) => Cond::Or(parts),
        _ => Cond::And(parts),
    })
}

fn parse_in_parens(cv: &Cv) -> Option<Cond> {
    match cv {
        Cv::Block(BlockKind::Paren, inner) => {
            let non_ws: Vec<&Cv> = inner.iter().filter(|c| !c.is_ws()).collect();
            // Nested condition?
            if non_ws.first().is_some_and(|c| c.is_ident("not") || matches!(c, Cv::Block(BlockKind::Paren, _)) || matches!(c, Cv::Func(..)))
                && !is_range_with_leading_value(&non_ws)
                && let Some(c) = parse_condition(inner, true)
            {
                return Some(c);
            }
            Some(parse_feature(&non_ws).map_or(Cond::Unknown, Cond::Feature))
        }
        Cv::Func(..) => Some(Cond::Unknown),
        _ => None,
    }
}

fn is_range_with_leading_value(items: &[&Cv]) -> bool {
    matches!(items.first(), Some(Cv::Tok(Token::Number { .. } | Token::Dimension { .. })))
}

fn parse_fval(c: &Cv) -> Option<FVal> {
    match c {
        Cv::Tok(Token::Number { value, .. }) => Some(FVal::Num(*value as f32)),
        Cv::Tok(Token::Ident(i)) => Some(FVal::Ident(i.to_ascii_lowercase())),
        Cv::Tok(Token::Dimension { value, unit, .. }) => {
            let u = unit.to_ascii_lowercase();
            match u.as_str() {
                "dpi" => Some(FVal::Res(*value as f32 / 96.0)),
                "dpcm" => Some(FVal::Res(*value as f32 * 2.54 / 96.0)),
                "dppx" | "x" => Some(FVal::Res(*value as f32)),
                _ => {
                    let lp = parse_length(c, true)?;
                    // em/rem in media queries use the initial font size (16px).
                    let px = match lp {
                        super::values::Lp::Len(v, unit) => match unit {
                            LenUnit::Em | LenUnit::Rem | LenUnit::Ex | LenUnit::Ch => v * 16.0,
                            other => other.to_px(
                                v,
                                &super::values::LenCtx { font_size: 16.0, root_font_size: 16.0, viewport_w: 0.0, viewport_h: 0.0, ex: 8.0, ch: 8.0 },
                            ),
                        },
                        _ => return None,
                    };
                    Some(FVal::Len(px))
                }
            }
        }
        _ => None,
    }
}

fn parse_cmp(items: &[&Cv], i: usize) -> Option<(Cmp, usize)> {
    let c = items.get(i)?;
    let eq_follows = items.get(i + 1).is_some_and(|n| n.is_delim('='));
    if c.is_delim('<') {
        Some(if eq_follows { (Cmp::Le, 2) } else { (Cmp::Lt, 1) })
    } else if c.is_delim('>') {
        Some(if eq_follows { (Cmp::Ge, 2) } else { (Cmp::Gt, 1) })
    } else if c.is_delim('=') {
        Some((Cmp::Eq, 1))
    } else {
        None
    }
}

fn flip(c: Cmp) -> Cmp {
    match c {
        Cmp::Lt => Cmp::Gt,
        Cmp::Le => Cmp::Ge,
        Cmp::Gt => Cmp::Lt,
        Cmp::Ge => Cmp::Le,
        Cmp::Eq => Cmp::Eq,
    }
}

fn parse_feature(items: &[&Cv]) -> Option<Feature> {
    // Boolean: (name)
    if let [Cv::Tok(Token::Ident(n))] = items {
        return Some(Feature { name: n.to_ascii_lowercase(), constraints: Vec::new() });
    }
    // Plain: (name: value)
    if let [Cv::Tok(Token::Ident(n)), Cv::Tok(Token::Colon), rest @ ..] = items {
        let name = n.to_ascii_lowercase();
        // A ratio value `a / b` or a single value.
        let val = match rest {
            [v] => parse_fval(v)?,
            [Cv::Tok(Token::Number { value: a, .. }), d, Cv::Tok(Token::Number { value: b, .. })] if d.is_delim('/') => {
                if *b == 0.0 { FVal::Ratio(f32::MAX) } else { FVal::Ratio((*a / *b) as f32) }
            }
            _ => return None,
        };
        if let Some(base) = name.strip_prefix("min-") {
            return Some(Feature { name: base.to_string(), constraints: vec![(Cmp::Ge, val)] });
        }
        if let Some(base) = name.strip_prefix("max-") {
            return Some(Feature { name: base.to_string(), constraints: vec![(Cmp::Le, val)] });
        }
        return Some(Feature { name, constraints: vec![(Cmp::Eq, val)] });
    }
    // Range forms: `name op value`, `value op name`, `value op name op value`.
    let mut i = 0usize;
    let mut name: Option<String> = None;
    let mut cons: Vec<(Cmp, FVal)> = Vec::new();
    // First operand.
    let first = items.get(i)?;
    let mut left_val: Option<FVal> = None;
    if let Cv::Tok(Token::Ident(n)) = first {
        name = Some(n.to_ascii_lowercase());
    } else {
        left_val = Some(parse_fval(first)?);
    }
    i += 1;
    let (op1, n1) = parse_cmp(items, i)?;
    i += n1;
    let second = items.get(i)?;
    i += 1;
    match (&name, left_val) {
        (Some(_), None) => {
            cons.push((op1, parse_fval(second)?));
        }
        (None, Some(lv)) => {
            let Cv::Tok(Token::Ident(n)) = second else { return None };
            name = Some(n.to_ascii_lowercase());
            // `value op name` means name flip(op) value.
            cons.push((flip(op1), lv));
            if i < items.len() {
                let (op2, n2) = parse_cmp(items, i)?;
                i += n2;
                let third = items.get(i)?;
                i += 1;
                // Both operators must point the same way.
                let same_dir = matches!((op1, op2), (Cmp::Lt | Cmp::Le, Cmp::Lt | Cmp::Le) | (Cmp::Gt | Cmp::Ge, Cmp::Gt | Cmp::Ge));
                if !same_dir {
                    return None;
                }
                cons.push((op2, parse_fval(third)?));
            }
        }
        _ => return None,
    }
    if i != items.len() {
        return None;
    }
    Some(Feature { name: name?, constraints: cons })
}

fn eval_cond(c: &Cond, env: &MediaEnv) -> Tri {
    match c {
        Cond::Unknown => Tri::Unknown,
        Cond::Not(inner) => eval_cond(inner, env).not(),
        Cond::And(parts) => {
            let mut r = Tri::True;
            for p in parts {
                match eval_cond(p, env) {
                    Tri::False => return Tri::False,
                    Tri::Unknown => r = Tri::Unknown,
                    Tri::True => {}
                }
            }
            r
        }
        Cond::Or(parts) => {
            let mut r = Tri::False;
            for p in parts {
                match eval_cond(p, env) {
                    Tri::True => return Tri::True,
                    Tri::Unknown => r = Tri::Unknown,
                    Tri::False => {}
                }
            }
            r
        }
        Cond::Feature(f) => eval_feature(f, env),
    }
}

fn compare(a: f32, op: Cmp, b: f32) -> bool {
    match op {
        Cmp::Eq => (a - b).abs() < 1.0e-4,
        Cmp::Lt => a < b,
        Cmp::Le => a <= b,
        Cmp::Gt => a > b,
        Cmp::Ge => a >= b,
    }
}

fn eval_feature(f: &Feature, env: &MediaEnv) -> Tri {
    let actual: FVal = match f.name.as_str() {
        "width" | "device-width" => FVal::Len(env.width),
        "height" | "device-height" => FVal::Len(env.height),
        "aspect-ratio" | "device-aspect-ratio" => FVal::Ratio(if env.height > 0.0 { env.width / env.height } else { 1.0 }),
        "orientation" => FVal::Ident(if env.width >= env.height { "landscape" } else { "portrait" }.into()),
        "resolution" => FVal::Res(env.resolution),
        "prefers-color-scheme" => FVal::Ident(if env.dark { "dark" } else { "light" }.into()),
        "prefers-reduced-motion" => FVal::Ident(if env.reduced_motion { "reduce" } else { "no-preference" }.into()),
        "hover" | "any-hover" => FVal::Ident("hover".into()),
        "pointer" | "any-pointer" => FVal::Ident("fine".into()),
        "color" => FVal::Num(8.0),
        "color-index" | "monochrome" => FVal::Num(0.0),
        "scripting" => FVal::Ident("none".into()),
        "update" => FVal::Ident("fast".into()),
        "display-mode" => FVal::Ident("browser".into()),
        "prefers-contrast" => FVal::Ident("no-preference".into()),
        "forced-colors" => FVal::Ident("none".into()),
        "inverted-colors" => FVal::Ident("none".into()),
        "prefers-reduced-transparency" => FVal::Ident("no-preference".into()),
        "prefers-reduced-data" => FVal::Ident("no-preference".into()),
        "overflow-block" => FVal::Ident("scroll".into()),
        "overflow-inline" => FVal::Ident("scroll".into()),
        "grid" => FVal::Num(0.0),
        _ => return Tri::Unknown,
    };
    if f.constraints.is_empty() {
        // Boolean context: true unless the value is zero / none.
        return Tri::from_bool(match &actual {
            FVal::Len(v) | FVal::Num(v) | FVal::Ratio(v) | FVal::Res(v) => *v != 0.0,
            FVal::Ident(s) => s != "none",
        });
    }
    for (op, want) in &f.constraints {
        let ok = match (&actual, want) {
            (FVal::Len(a), FVal::Len(b)) => compare(*a, *op, *b),
            (FVal::Num(a), FVal::Num(b)) => compare(*a, *op, *b),
            (FVal::Ratio(a), FVal::Ratio(b)) => compare(*a, *op, *b),
            (FVal::Res(a), FVal::Res(b)) => compare(*a, *op, *b),
            // A bare number 0 compares as a zero length.
            (FVal::Len(a), FVal::Num(b)) if *b == 0.0 => compare(*a, *op, 0.0),
            (FVal::Ident(a), FVal::Ident(b)) => *op == Cmp::Eq && a == b,
            // `(color: 8)` style integer features, `(resolution: 2)` etc.
            _ => return Tri::False,
        };
        if !ok {
            return Tri::False;
        }
    }
    Tri::True
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::css::syntax::parse_cvs;

    fn m(q: &str, w: f32) -> bool {
        MediaQueryList::parse(&parse_cvs(q)).matches(&MediaEnv::screen(w, 800.0))
    }

    #[test]
    fn condition_nesting_cap() {
        let nest = |n: usize| format!("{}min-width: 1px{}", "(".repeat(n), ")".repeat(n));
        assert!(m(&nest(32), 100.0));
        assert!(!m(&nest(33), 100.0));
    }

    #[test]
    fn widths() {
        for (q, a, b, c) in [
            ("(min-width: 600px)", false, true, true),
            ("(max-width: 700px)", true, false, false),
            ("screen and (min-width: 600px) and (max-width: 800px)", false, true, false),
            ("(width >= 600px)", false, true, true),
            ("(400px <= width <= 800px)", false, true, false),
            ("not (min-width: 600px)", true, false, false),
            ("(min-width: 1000px), (max-width: 400px)", true, false, true),
            ("print", false, false, false),
            ("only screen and (orientation: landscape)", false, false, true),
            ("(hover: hover) and (pointer: fine)", true, true, true),
            ("(prefers-color-scheme: dark)", false, false, false),
            ("(prefers-reduced-motion: reduce)", false, false, false),
            ("(min-width: 600px) or (hover: none)", false, true, true),
            ("bogus-type", false, false, false),
            ("(unknown-feature: 1)", false, false, false),
            ("not all", false, false, false),
            ("", true, true, true),
        ] {
            assert_eq!([m(q, 360.0), m(q, 768.0), m(q, 1280.0)], [a, b, c], "{q}");
        }
    }
}
