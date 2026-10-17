//! Property value parsing: declarations to longhand values (CSS 2.2 chapters 8-10, 16, css-text-3,
//! css-fonts-4, css-display-3, css-position-3, css-color-4 and css-values-4 for the value types).
//! Shorthands are expanded; declarations with `var()` are kept unparsed (see [`VarDecl`]).

use std::rc::Rc;

use super::color::{ColorValue, parse_color_cv};
use super::syntax::{BlockKind, Cv, split_commas, trim_ws};
use super::tokenizer::Token;
use super::types::*;
use super::values::{LenUnit, Lp, parse_integer, parse_length, parse_length_percentage, parse_number};

/// Parsing environment.
#[derive(Clone, Copy, Debug, Default)]
pub struct ParseEnv {
    /// Quirks mode: unitless numbers are lengths (px) for the properties of CSS Quirks 3.
    pub quirks: bool,
}

/// A parsed declaration: a longhand with its specified value.
pub type ParsedDecl = (PropId, Specified);

pub fn contains_var(cvs: &[Cv]) -> bool {
    cvs.iter().any(|c| match c {
        Cv::Func(n, items) => n.eq_ignore_ascii_case("var") || n.eq_ignore_ascii_case("env") || contains_var(items),
        Cv::Block(_, items) => contains_var(items),
        _ => false,
    })
}

fn css_wide_keyword(cvs: &[Cv]) -> Option<Specified> {
    let t = trim_ws(cvs);
    if t.len() != 1 {
        return None;
    }
    let id = t.first()?.ident()?.to_ascii_lowercase();
    Some(match id.as_str() {
        "inherit" => Specified::Inherit,
        "initial" => Specified::Initial,
        "unset" => Specified::Unset,
        "revert" => Specified::Revert,
        "revert-layer" => Specified::RevertLayer,
        _ => return None,
    })
}

/// The longhands a property name stands for (the property itself, or the shorthand's longhands).
pub fn longhands_of(name: &str) -> Option<Vec<PropId>> {
    use PropId::*;
    Some(match name {
        "margin" => vec![MarginTop, MarginRight, MarginBottom, MarginLeft],
        "padding" => vec![PaddingTop, PaddingRight, PaddingBottom, PaddingLeft],
        "border-width" => vec![BorderTopWidth, BorderRightWidth, BorderBottomWidth, BorderLeftWidth],
        "border-style" => vec![BorderTopStyle, BorderRightStyle, BorderBottomStyle, BorderLeftStyle],
        "border-color" => vec![BorderTopColor, BorderRightColor, BorderBottomColor, BorderLeftColor],
        "border" => vec![
            BorderTopWidth, BorderRightWidth, BorderBottomWidth, BorderLeftWidth, BorderTopStyle, BorderRightStyle,
            BorderBottomStyle, BorderLeftStyle, BorderTopColor, BorderRightColor, BorderBottomColor, BorderLeftColor,
        ],
        "border-top" => vec![BorderTopWidth, BorderTopStyle, BorderTopColor],
        "border-right" => vec![BorderRightWidth, BorderRightStyle, BorderRightColor],
        "border-bottom" => vec![BorderBottomWidth, BorderBottomStyle, BorderBottomColor],
        "border-left" => vec![BorderLeftWidth, BorderLeftStyle, BorderLeftColor],
        "border-radius" => vec![BorderTopLeftRadius, BorderTopRightRadius, BorderBottomRightRadius, BorderBottomLeftRadius],
        "overflow" => vec![OverflowX, OverflowY],
        "inset" => vec![Top, Right, Bottom, Left],
        "font" => vec![FontStyle, FontWeight, FontSize, LineHeight, FontFamily],
        "list-style" => vec![ListStyleType, ListStylePosition],
        "text-decoration" => vec![TextDecorationLine, TextDecorationStyle, TextDecorationColor, TextDecorationThickness],
        "background" => vec![BackgroundColor],
        "white-space" => vec![WhiteSpaceCollapse, TextWrapMode],
        "text-wrap" => vec![TextWrapMode],
        "flex" | "-webkit-flex" => vec![FlexGrow, FlexShrink, FlexBasis],
        "flex-flow" | "-webkit-flex-flow" => vec![FlexDirection, FlexWrap],
        "gap" | "grid-gap" => vec![RowGap, ColumnGap],
        "grid-template" => vec![GridTemplateRows, GridTemplateColumns, GridTemplateAreas],
        "grid" => vec![GridTemplateRows, GridTemplateColumns, GridTemplateAreas, GridAutoRows, GridAutoColumns, GridAutoFlow],
        "grid-area" => vec![GridRowStart, GridColumnStart, GridRowEnd, GridColumnEnd],
        "grid-row" => vec![GridRowStart, GridRowEnd],
        "grid-column" => vec![GridColumnStart, GridColumnEnd],
        "place-items" => vec![AlignItems, JustifyItems],
        "place-self" => vec![AlignSelf, JustifySelf],
        "place-content" => vec![AlignContent, JustifyContent],
        "all" => ALL_PROPS.iter().copied().filter(|p| *p != Direction).collect(),
        // Logical properties map to the physical ones (horizontal-tb, ltr only).
        "margin-block" => vec![MarginTop, MarginBottom],
        "margin-inline" => vec![MarginLeft, MarginRight],
        "padding-block" => vec![PaddingTop, PaddingBottom],
        "padding-inline" => vec![PaddingLeft, PaddingRight],
        "inset-block" => vec![Top, Bottom],
        "inset-inline" => vec![Left, Right],
        "border-block-width" => vec![BorderTopWidth, BorderBottomWidth],
        "border-inline-width" => vec![BorderLeftWidth, BorderRightWidth],
        "border-block-style" => vec![BorderTopStyle, BorderBottomStyle],
        "border-inline-style" => vec![BorderLeftStyle, BorderRightStyle],
        "border-block-color" => vec![BorderTopColor, BorderBottomColor],
        "border-inline-color" => vec![BorderLeftColor, BorderRightColor],
        "border-block" => vec![BorderTopWidth, BorderBottomWidth, BorderTopStyle, BorderBottomStyle, BorderTopColor, BorderBottomColor],
        "border-inline" => vec![BorderLeftWidth, BorderRightWidth, BorderLeftStyle, BorderRightStyle, BorderLeftColor, BorderRightColor],
        _ => vec![longhand_by_name(name)?],
    })
}

/// Physical longhand by name, including the logical aliases of single longhands.
pub fn longhand_by_name(name: &str) -> Option<PropId> {
    static MAP: std::sync::OnceLock<std::collections::HashMap<&'static str, PropId, crate::html::atoms::FxBuild>> = std::sync::OnceLock::new();
    let map = MAP.get_or_init(|| {
        let mut m: std::collections::HashMap<&'static str, PropId, crate::html::atoms::FxBuild> = Default::default();
        for p in ALL_PROPS {
            m.insert(p.name(), p);
        }
        for alias in ALIASES {
            if let Some(p) = ALL_PROPS.iter().copied().find(|p| p.name() == alias.1) {
                m.insert(alias.0, p);
            }
        }
        m
    });
    map.get(name).copied()
}

/// Logical-property aliases of single longhands (horizontal-tb, ltr).
const ALIASES: &[(&str, &str)] = &[
    ("word-wrap", "overflow-wrap"),
    ("grid-row-gap", "row-gap"),
    ("grid-column-gap", "column-gap"),
    ("-webkit-flex-direction", "flex-direction"),
    ("-webkit-flex-wrap", "flex-wrap"),
    ("-webkit-flex-grow", "flex-grow"),
    ("-webkit-flex-shrink", "flex-shrink"),
    ("-webkit-flex-basis", "flex-basis"),
    ("-webkit-order", "order"),
    ("-webkit-justify-content", "justify-content"),
    ("-webkit-align-items", "align-items"),
    ("-webkit-align-self", "align-self"),
    ("-webkit-align-content", "align-content"),
    ("margin-block-start", "margin-top"),
    ("margin-block-end", "margin-bottom"),
    ("margin-inline-start", "margin-left"),
    ("margin-inline-end", "margin-right"),
    ("padding-block-start", "padding-top"),
    ("padding-block-end", "padding-bottom"),
    ("padding-inline-start", "padding-left"),
    ("padding-inline-end", "padding-right"),
    ("inset-block-start", "top"),
    ("inset-block-end", "bottom"),
    ("inset-inline-start", "left"),
    ("inset-inline-end", "right"),
    ("inline-size", "width"),
    ("block-size", "height"),
    ("min-inline-size", "min-width"),
    ("min-block-size", "min-height"),
    ("max-inline-size", "max-width"),
    ("max-block-size", "max-height"),
    ("border-block-start-width", "border-top-width"),
    ("border-block-end-width", "border-bottom-width"),
    ("border-inline-start-width", "border-left-width"),
    ("border-inline-end-width", "border-right-width"),
    ("border-block-start-style", "border-top-style"),
    ("border-block-end-style", "border-bottom-style"),
    ("border-inline-start-style", "border-left-style"),
    ("border-inline-end-style", "border-right-style"),
    ("border-block-start-color", "border-top-color"),
    ("border-block-end-color", "border-bottom-color"),
    ("border-inline-start-color", "border-left-color"),
    ("border-inline-end-color", "border-right-color"),
    ("border-start-start-radius", "border-top-left-radius"),
    ("border-start-end-radius", "border-top-right-radius"),
    ("border-end-end-radius", "border-bottom-right-radius"),
    ("border-end-start-radius", "border-bottom-left-radius"),
];


/// Parse one (non-custom) declaration. `None` means the declaration is invalid and must be dropped.
pub fn parse_declaration(name: &str, value: &[Cv], env: &ParseEnv, var_id: &mut u32) -> Option<Vec<ParsedDecl>> {
    if name.starts_with("--") {
        return None;
    }
    let lname: std::borrow::Cow<'_, str> =
        if name.bytes().any(|b| b.is_ascii_uppercase()) { std::borrow::Cow::Owned(name.to_ascii_lowercase()) } else { std::borrow::Cow::Borrowed(name) };
    let lname = &*lname;
    if let Some(kw) = css_wide_keyword(value) {
        let longs = longhands_of(lname)?;
        return Some(longs.into_iter().map(|p| (p, kw.clone())).collect());
    }
    if contains_var(value) {
        let longs = longhands_of(lname)?;
        if trim_ws(value).is_empty() {
            return None;
        }
        *var_id += 1;
        let vd = Rc::new(VarDecl { prop: lname.to_string(), tokens: trim_ws(value).to_vec(), id: *var_id });
        return Some(longs.into_iter().map(|p| (p, Specified::Var(vd.clone()))).collect());
    }
    let values = parse_value(lname, trim_ws(value), env)?;
    Some(values.into_iter().map(|(p, v)| (p, Specified::Value(v))).collect())
}

/// Parse a (non-var) value of the property `name` into longhand values.
fn parse_value(name: &str, cvs: &[Cv], env: &ParseEnv) -> Option<Vec<(PropId, Value)>> {
    use PropId::*;
    if cvs.is_empty() {
        return None;
    }
    let items: Vec<&Cv> = cvs.iter().filter(|c| !c.is_ws()).collect();
    // Space separated component values of 1 to 4 (box shorthands).
    let four = |parse: &dyn Fn(&Cv) -> Option<Value>, props: [PropId; 4]| -> Option<Vec<(PropId, Value)>> {
        if items.is_empty() || items.len() > 4 {
            return None;
        }
        let vals: Vec<Value> = items.iter().map(|c| parse(c)).collect::<Option<Vec<_>>>()?;
        let v = |i: usize| vals.get(i).cloned();
        let (t, r, b, l) = match vals.len() {
            1 => (v(0)?, v(0)?, v(0)?, v(0)?),
            2 => (v(0)?, v(1)?, v(0)?, v(1)?),
            3 => (v(0)?, v(1)?, v(2)?, v(1)?),
            _ => (v(0)?, v(1)?, v(2)?, v(3)?),
        };
        Some(vec![(props[0], t), (props[1], r), (props[2], b), (props[3], l)])
    };
    // Two values for block / inline pairs.
    let two = |parse: &dyn Fn(&Cv) -> Option<Value>, props: [PropId; 2]| -> Option<Vec<(PropId, Value)>> {
        if items.is_empty() || items.len() > 2 {
            return None;
        }
        let vals: Vec<Value> = items.iter().map(|c| parse(c)).collect::<Option<Vec<_>>>()?;
        let a = vals.first()?.clone();
        let b = vals.get(1).cloned().unwrap_or_else(|| a.clone());
        Some(vec![(props[0], a), (props[1], b)])
    };
    let q = env.quirks;
    match name {
        "margin" => four(&|c| p_margin(c, q), [MarginTop, MarginRight, MarginBottom, MarginLeft]),
        "margin-block" => two(&|c| p_margin(c, q), [MarginTop, MarginBottom]),
        "margin-inline" => two(&|c| p_margin(c, q), [MarginLeft, MarginRight]),
        "padding" => four(&|c| p_padding(c, q), [PaddingTop, PaddingRight, PaddingBottom, PaddingLeft]),
        "padding-block" => two(&|c| p_padding(c, q), [PaddingTop, PaddingBottom]),
        "padding-inline" => two(&|c| p_padding(c, q), [PaddingLeft, PaddingRight]),
        "inset" => four(&|c| p_offset(c, q), [Top, Right, Bottom, Left]),
        "inset-block" => two(&|c| p_offset(c, q), [Top, Bottom]),
        "inset-inline" => two(&|c| p_offset(c, q), [Left, Right]),
        "border-width" => four(&|c| p_border_width(c, q), [BorderTopWidth, BorderRightWidth, BorderBottomWidth, BorderLeftWidth]),
        "border-block-width" => two(&|c| p_border_width(c, q), [BorderTopWidth, BorderBottomWidth]),
        "border-inline-width" => two(&|c| p_border_width(c, q), [BorderLeftWidth, BorderRightWidth]),
        "border-style" => four(&p_border_style, [BorderTopStyle, BorderRightStyle, BorderBottomStyle, BorderLeftStyle]),
        "border-block-style" => two(&p_border_style, [BorderTopStyle, BorderBottomStyle]),
        "border-inline-style" => two(&p_border_style, [BorderLeftStyle, BorderRightStyle]),
        "border-color" => four(&p_color, [BorderTopColor, BorderRightColor, BorderBottomColor, BorderLeftColor]),
        "border-block-color" => two(&p_color, [BorderTopColor, BorderBottomColor]),
        "border-inline-color" => two(&p_color, [BorderLeftColor, BorderRightColor]),
        "border" => {
            let (w, s, c) = parse_border_side(&items, q)?;
            let mut out = Vec::new();
            for p in [BorderTopWidth, BorderRightWidth, BorderBottomWidth, BorderLeftWidth] {
                out.push((p, w.clone()));
            }
            for p in [BorderTopStyle, BorderRightStyle, BorderBottomStyle, BorderLeftStyle] {
                out.push((p, s.clone()));
            }
            for p in [BorderTopColor, BorderRightColor, BorderBottomColor, BorderLeftColor] {
                out.push((p, c.clone()));
            }
            Some(out)
        }
        "border-block" | "border-inline" => {
            let (w, s, c) = parse_border_side(&items, q)?;
            let (a, b) = if name == "border-block" { (0, 2) } else { (3, 1) };
            let sides = [
                [BorderTopWidth, BorderTopStyle, BorderTopColor],
                [BorderRightWidth, BorderRightStyle, BorderRightColor],
                [BorderBottomWidth, BorderBottomStyle, BorderBottomColor],
                [BorderLeftWidth, BorderLeftStyle, BorderLeftColor],
            ];
            let mut out = Vec::new();
            for i in [a, b] {
                let sd = sides.get(i)?;
                out.push((sd[0], w.clone()));
                out.push((sd[1], s.clone()));
                out.push((sd[2], c.clone()));
            }
            Some(out)
        }
        "border-top" | "border-right" | "border-bottom" | "border-left" | "border-block-start" | "border-block-end"
        | "border-inline-start" | "border-inline-end" => {
            let (w, s, c) = parse_border_side(&items, q)?;
            let sd = match name {
                "border-top" | "border-block-start" => [BorderTopWidth, BorderTopStyle, BorderTopColor],
                "border-right" | "border-inline-end" => [BorderRightWidth, BorderRightStyle, BorderRightColor],
                "border-bottom" | "border-block-end" => [BorderBottomWidth, BorderBottomStyle, BorderBottomColor],
                _ => [BorderLeftWidth, BorderLeftStyle, BorderLeftColor],
            };
            Some(vec![(sd[0], w), (sd[1], s), (sd[2], c)])
        }
        "border-radius" => {
            // <length-percentage>{1,4} [ / <length-percentage>{1,4} ]?
            let slash = items.iter().position(|c| c.is_delim('/'));
            let (first, second) = match slash {
                Some(i) => (items.get(..i)?, Some(items.get(i + 1..)?)),
                None => (items.as_slice(), None),
            };
            let ok = |part: &[&Cv]| !part.is_empty() && part.len() <= 4 && part.iter().all(|c| radius_ok(c, q));
            if !ok(first) || second.is_some_and(|s| !ok(s)) {
                return None;
            }
            Some([BorderTopLeftRadius, BorderTopRightRadius, BorderBottomRightRadius, BorderBottomLeftRadius]
                .into_iter()
                .map(|p| (p, Value::Ignored))
                .collect())
        }
        "overflow" => {
            let vals: Vec<Overflow> = items.iter().map(|c| p_overflow(c)).collect::<Option<Vec<_>>>()?;
            match vals.as_slice() {
                [a] => Some(vec![(OverflowX, Value::Overflow(*a)), (OverflowY, Value::Overflow(*a))]),
                [a, b] => Some(vec![(OverflowX, Value::Overflow(*a)), (OverflowY, Value::Overflow(*b))]),
                _ => None,
            }
        }
        "flex" | "-webkit-flex" => parse_flex(&items, q),
        "flex-flow" | "-webkit-flex-flow" => {
            let mut dir = None;
            let mut wrap = None;
            if items.is_empty() || items.len() > 2 {
                return None;
            }
            for c in &items {
                let taken = match (kw(c, super::types::FlexDirection::from_keyword), kw(c, super::types::FlexWrap::from_keyword)) {
                    (Some(d), _) => dir.replace(d).is_some(),
                    (None, Some(w)) => wrap.replace(w).is_some(),
                    (None, None) => true,
                };
                if taken {
                    return None;
                }
            }
            Some(vec![
                (FlexDirection, Value::FlexDirection(dir.unwrap_or_default())),
                (FlexWrap, Value::FlexWrap(wrap.unwrap_or_default())),
            ])
        }
        "gap" | "grid-gap" => {
            if items.is_empty() || items.len() > 2 {
                return None;
            }
            let a = p_gap(items.first()?, q)?;
            let b = match items.get(1) {
                Some(c) => p_gap(c, q)?,
                None => a.clone(),
            };
            Some(vec![(RowGap, a), (ColumnGap, b)])
        }
        "grid-template" => super::parse_grid::parse_template(&items),
        "grid" => super::parse_grid::parse_grid(&items),
        "grid-area" | "grid-row" | "grid-column" => super::parse_grid::parse_lines(name, &items),
        "place-items" | "place-self" | "place-content" => {
            let (a, j) = match name {
                "place-items" => (AlignItems, JustifyItems),
                "place-self" => (AlignSelf, JustifySelf),
                _ => (AlignContent, JustifyContent),
            };
            let lead = |i: usize| items.get(i).and_then(|c| c.ident()).is_some_and(|s| matches!(s.to_ascii_lowercase().as_str(), "safe" | "unsafe" | "first" | "last"));
            let k = if lead(0) { 2 } else { 1 };
            let (x, y) = (items.get(..k)?, items.get(k..)?);
            let y = if y.is_empty() { x } else { y };
            Some(vec![(a, parse_align(a, x)?), (j, parse_align(j, y)?)])
        }
        "font" => parse_font_shorthand(cvs, q),
        "list-style" => parse_list_style(&items),
        "text-decoration" => parse_text_decoration(&items, q),
        "background" => parse_background(cvs),
        "white-space" => parse_white_space(&items),
        "text-wrap" => {
            let [c] = items.as_slice() else { return None };
            let id = c.ident()?.to_ascii_lowercase();
            let mode = match id.as_str() {
                "nowrap" => super::types::TextWrapMode::Nowrap,
                "wrap" | "balance" | "pretty" | "stable" => super::types::TextWrapMode::Wrap,
                _ => return None,
            };
            Some(vec![(TextWrapMode, Value::TextWrapMode(mode))])
        }
        _ => {
            let prop = longhand_by_name(name)?;
            let v = parse_longhand(prop, cvs, &items, q)?;
            Some(vec![(prop, v)])
        }
    }
}

/// `<'row-gap'>`: `normal | <length-percentage [0,inf]>` (css-align-3 8).
fn p_gap(c: &Cv, q: bool) -> Option<Value> {
    if c.is_ident("normal") {
        return Some(Value::Gap(None));
    }
    Some(Value::Gap(Some(p_lp(c, q, true)?)))
}

/// `flex-basis` (css-flexbox-1 7.3): `content | <'width'>`.
fn p_flex_basis(c: &Cv, q: bool) -> Option<FlexBasis> {
    if c.is_ident("content") {
        return Some(FlexBasis::Content);
    }
    match p_size(c, q, false)? {
        Value::Dim(Dim::Auto) => Some(FlexBasis::Auto),
        Value::Dim(d) => Some(FlexBasis::Size(d)),
        _ => None,
    }
}

fn flex_number(c: &Cv) -> Option<f32> {
    match c {
        Cv::Tok(Token::Number { value, .. }) if *value >= 0.0 => Some(*value as f32),
        _ => None,
    }
}

/// The `flex` shorthand (css-flexbox-1 7.1): `none | [ <'flex-grow'> <'flex-shrink'>? || <'flex-basis'> ]`. An omitted
/// `flex-grow` or `flex-shrink` is 1, an omitted `flex-basis` is `0%`; a unitless zero not preceded by two flex factors is
/// a flex factor.
fn parse_flex(items: &[&Cv], q: bool) -> Option<Vec<(PropId, Value)>> {
    let make = |g: f32, s: f32, b: FlexBasis| {
        vec![(PropId::FlexGrow, Value::FlexFactor(g)), (PropId::FlexShrink, Value::FlexFactor(s)), (PropId::FlexBasis, Value::FlexBasis(b))]
    };
    let first = items.first()?;
    if items.len() == 1 {
        match first.ident().map(str::to_ascii_lowercase).as_deref() {
            Some("none") => return Some(make(0.0, 0.0, FlexBasis::Auto)),
            Some("auto") => return Some(make(1.0, 1.0, FlexBasis::Auto)),
            _ => {}
        }
    }
    let mut i = 0;
    let mut basis = None;
    let mut nums: Vec<f32> = Vec::new();
    if flex_number(first).is_none() {
        basis = Some(p_flex_basis(first, q)?);
        i = 1;
    }
    while i < items.len() && nums.len() < 2 {
        let Some(n) = flex_number(items.get(i)?) else { break };
        nums.push(n);
        i += 1;
    }
    if basis.is_none() && i < items.len() {
        basis = Some(p_flex_basis(items.get(i)?, q)?);
        i += 1;
    }
    if i != items.len() || (nums.is_empty() && basis.is_none()) {
        return None;
    }
    let grow = nums.first().copied().unwrap_or(1.0);
    let shrink = nums.get(1).copied().unwrap_or(1.0);
    Some(make(grow, shrink, basis.unwrap_or(FlexBasis::Size(Dim::Lp(Lp::Pct(0.0))))))
}

/// A value of `justify-content`, `align-content`, `align-items` or `align-self` (css-align-3 6, 7, 8).
fn parse_align(prop: PropId, items: &[&Cv]) -> Option<Value> {
    let ids: Vec<String> = items.iter().map(|c| c.ident().map(str::to_ascii_lowercase)).collect::<Option<_>>()?;
    let (safe, rest) = match ids.first().map(String::as_str) {
        Some("safe") => (Some(true), ids.get(1..)?),
        Some("unsafe") => (Some(false), ids.get(1..)?),
        _ => (None, ids.as_slice()),
    };
    let kind = match rest {
        [k] => AlignKind::from_keyword(k).filter(|k| *k != AlignKind::LastBaseline)?,
        [f, b] if b == "baseline" && f == "first" => AlignKind::Baseline,
        [f, b] if b == "baseline" && f == "last" => AlignKind::LastBaseline,
        _ => return None,
    };
    use AlignKind::*;
    let content = matches!(prop, PropId::JustifyContent | PropId::AlignContent);
    let ok = match kind {
        Auto => matches!(prop, PropId::AlignSelf | PropId::JustifySelf),
        SpaceBetween | SpaceAround | SpaceEvenly => content,
        Left | Right => matches!(prop, PropId::JustifyContent | PropId::JustifyItems | PropId::JustifySelf),
        SelfStart | SelfEnd => !content,
        _ => true,
    };
    ok.then_some(Value::Align(AlignValue { kind, safe }))
}

fn radius_ok(c: &Cv, quirks: bool) -> bool {
    p_lp(c, quirks, true).is_some()
}

// ---------------------------------------------------------------------------------------------
// Single-value parsers

fn kw<T>(c: &Cv, f: impl Fn(&str) -> Option<T>) -> Option<T> {
    f(c.ident()?)
}

/// `<length-percentage>`, non-negative if `nonneg`; unitless numbers as px in quirks mode.
pub(super) fn p_lp(c: &Cv, quirks: bool, nonneg: bool) -> Option<Lp> {
    let lp = match parse_length_percentage(c, true) {
        Some(l) => l,
        None if quirks => match c {
            Cv::Tok(Token::Number { value, .. }) => Lp::px(*value as f32),
            _ => return None,
        },
        None => return None,
    };
    if nonneg && lp.is_negative_length() {
        return None;
    }
    Some(lp)
}

fn p_margin(c: &Cv, q: bool) -> Option<Value> {
    if c.is_ident("auto") {
        return Some(Value::Dim(Dim::Auto));
    }
    Some(Value::Dim(Dim::Lp(p_lp(c, q, false)?)))
}

fn p_padding(c: &Cv, q: bool) -> Option<Value> {
    Some(Value::Dim(Dim::Lp(p_lp(c, q, true)?)))
}

fn p_offset(c: &Cv, q: bool) -> Option<Value> {
    if c.is_ident("auto") {
        return Some(Value::Dim(Dim::Auto));
    }
    Some(Value::Dim(Dim::Lp(p_lp(c, q, false)?)))
}

fn p_size(c: &Cv, q: bool, allow_none: bool) -> Option<Value> {
    if let Cv::Func(n, _) = c
        && n.eq_ignore_ascii_case("fit-content")
    {
        return Some(Value::Dim(Dim::FitContent));
    }
    if let Some(id) = c.ident() {
        let d = match id.to_ascii_lowercase().as_str() {
            "auto" if !allow_none => Dim::Auto,
            "none" if allow_none => Dim::None,
            "min-content" | "-webkit-min-content" | "-moz-min-content" => Dim::MinContent,
            "max-content" | "-webkit-max-content" | "-moz-max-content" => Dim::MaxContent,
            "fit-content" | "-webkit-fit-content" | "-moz-fit-content" => Dim::FitContent,
            // `stretch` / fill-available: treat as `auto` (blocks fill their container anyway).
            "stretch" | "-webkit-fill-available" | "-moz-available" if !allow_none => Dim::Auto,
            _ => return None,
        };
        return Some(Value::Dim(d));
    }
    Some(Value::Dim(Dim::Lp(p_lp(c, q, true)?)))
}

fn p_border_width(c: &Cv, q: bool) -> Option<Value> {
    if let Some(id) = c.ident() {
        let k = match id.to_ascii_lowercase().as_str() {
            "thin" => BorderWidthKw::Thin,
            "medium" => BorderWidthKw::Medium,
            "thick" => BorderWidthKw::Thick,
            _ => return None,
        };
        return Some(Value::BorderWidth(BorderWidthSpec::Kw(k)));
    }
    // Border widths are lengths, not percentages.
    let l = match parse_length(c, true) {
        Some(l) => l,
        None if q => match c {
            Cv::Tok(Token::Number { value, .. }) => Lp::px(*value as f32),
            _ => return None,
        },
        None => return None,
    };
    if l.is_negative_length() {
        return None;
    }
    Some(Value::BorderWidth(BorderWidthSpec::Lp(l)))
}

fn p_border_style(c: &Cv) -> Option<Value> {
    kw(c, BorderLineStyle::from_keyword).map(Value::BorderStyle)
}

fn p_color(c: &Cv) -> Option<Value> {
    parse_color_cv(c).map(Value::Color)
}

fn p_overflow(c: &Cv) -> Option<Overflow> {
    let id = c.ident()?.to_ascii_lowercase();
    if id == "overlay" {
        return Some(Overflow::Auto);
    }
    Overflow::from_keyword(&id)
}

/// The width/style/color of `border`, `border-top`, ... in any order, each at most once.
fn parse_border_side(items: &[&Cv], q: bool) -> Option<(Value, Value, Value)> {
    let mut w = None;
    let mut s = None;
    let mut c = None;
    if items.is_empty() {
        return None;
    }
    for it in items {
        if w.is_none()
            && let Some(v) = p_border_width(it, q)
        {
            w = Some(v);
        } else if s.is_none()
            && let Some(v) = p_border_style(it)
        {
            s = Some(v);
        } else if c.is_none()
            && let Some(v) = p_color(it)
        {
            c = Some(v);
        } else {
            return None;
        }
    }
    Some((
        w.unwrap_or(Value::BorderWidth(BorderWidthSpec::Kw(BorderWidthKw::Medium))),
        s.unwrap_or(Value::BorderStyle(BorderLineStyle::None)),
        c.unwrap_or(Value::Color(ColorValue::CurrentColor)),
    ))
}

fn parse_display(items: &[&Cv]) -> Option<Value> {
    let words: Vec<String> = items.iter().map(|c| c.ident().map(str::to_ascii_lowercase)).collect::<Option<Vec<_>>>()?;
    let d = match words.as_slice() {
        [w] => match w.as_str() {
            "-webkit-box" | "-webkit-flex" | "-ms-flexbox" | "-moz-box" => Display::Flex,
            "-webkit-inline-box" | "-webkit-inline-flex" | "-ms-inline-flexbox" | "-moz-inline-box" => Display::InlineFlex,
            "-ms-grid" => Display::Grid,
            "-ms-inline-grid" => Display::InlineGrid,
            "run-in" => Display::Inline,
            other => Display::from_keyword(other)?,
        },
        _ => {
            // <display-outside> || <display-inside>, plus `list-item`.
            let mut outside = None;
            let mut inside = None;
            let mut list_item = false;
            for w in &words {
                match w.as_str() {
                    "block" | "inline" | "run-in" if outside.is_none() => outside = Some(w.as_str()),
                    "flow" | "flow-root" | "table" | "flex" | "grid" | "ruby" if inside.is_none() => inside = Some(w.as_str()),
                    "list-item" if !list_item => list_item = true,
                    _ => return None,
                }
            }
            let out_inline = outside == Some("inline") || outside == Some("run-in");
            if list_item {
                if !matches!(inside, None | Some("flow")) {
                    return None;
                }
                Display::ListItem
            } else {
                match (out_inline, inside.unwrap_or("flow")) {
                    (false, "flow") => Display::Block,
                    (true, "flow") => Display::Inline,
                    (false, "flow-root") => Display::FlowRoot,
                    (true, "flow-root") => Display::InlineBlock,
                    (false, "table") => Display::Table,
                    (true, "table") => Display::InlineTable,
                    (false, "flex") => Display::Flex,
                    (true, "flex") => Display::InlineFlex,
                    (false, "grid") => Display::Grid,
                    (true, "grid") => Display::InlineGrid,
                    (_, "ruby") => Display::Ruby,
                    _ => return None,
                }
            }
        }
    };
    Some(Value::Display(d))
}

fn parse_font_family(cvs: &[Cv]) -> Option<Value> {
    let mut list = Vec::new();
    for part in split_commas(cvs) {
        let p = trim_ws(part);
        if p.is_empty() {
            return None;
        }
        if let [Cv::Tok(Token::Str(s))] = p {
            list.push(FontFamily::Named(Rc::from(s.as_str())));
            continue;
        }
        // A sequence of identifiers, joined by single spaces.
        let mut words = Vec::new();
        for c in p {
            if c.is_ws() {
                continue;
            }
            words.push(c.ident()?.to_string());
        }
        if words.is_empty() {
            return None;
        }
        if words.len() == 1 {
            let w = words.first()?;
            let l = w.to_ascii_lowercase();
            if matches!(l.as_str(), "inherit" | "initial" | "unset" | "revert" | "revert-layer" | "default") {
                return None;
            }
            if let Some(g) = GenericFamily::from_keyword(&l) {
                list.push(FontFamily::Generic(g));
                continue;
            }
        }
        list.push(FontFamily::Named(Rc::from(words.join(" ").as_str())));
    }
    if list.is_empty() { None } else { Some(Value::FontFamily(Rc::new(list))) }
}

fn parse_font_size(c: &Cv, q: bool) -> Option<Value> {
    if let Some(id) = c.ident() {
        let s = match id.to_ascii_lowercase().as_str() {
            "xx-small" => FontSizeSpec::Absolute(0),
            "x-small" => FontSizeSpec::Absolute(1),
            "small" => FontSizeSpec::Absolute(2),
            "medium" => FontSizeSpec::Absolute(3),
            "large" => FontSizeSpec::Absolute(4),
            "x-large" => FontSizeSpec::Absolute(5),
            "xx-large" => FontSizeSpec::Absolute(6),
            "xxx-large" => FontSizeSpec::Absolute(7),
            "larger" => FontSizeSpec::Larger,
            "smaller" => FontSizeSpec::Smaller,
            _ => return None,
        };
        return Some(Value::FontSize(s));
    }
    Some(Value::FontSize(FontSizeSpec::Lp(p_lp(c, q, true)?)))
}

fn parse_font_weight(c: &Cv) -> Option<Value> {
    if let Some(id) = c.ident() {
        let w = match id.to_ascii_lowercase().as_str() {
            "normal" => FontWeightSpec::Number(400.0),
            "bold" => FontWeightSpec::Number(700.0),
            "bolder" => FontWeightSpec::Bolder,
            "lighter" => FontWeightSpec::Lighter,
            _ => return None,
        };
        return Some(Value::FontWeight(w));
    }
    let n = parse_number(c)?;
    if !(1.0..=1000.0).contains(&n) {
        return None;
    }
    Some(Value::FontWeight(FontWeightSpec::Number(n)))
}

fn parse_line_height(c: &Cv, q: bool) -> Option<Value> {
    if c.is_ident("normal") {
        return Some(Value::LineHeight(LineHeightSpec::Normal));
    }
    if let Some(n) = parse_number(c) {
        return (n >= 0.0).then_some(Value::LineHeight(LineHeightSpec::Number(n)));
    }
    Some(Value::LineHeight(LineHeightSpec::Lp(p_lp(c, q, true)?)))
}

fn parse_vertical_align(c: &Cv, q: bool) -> Option<Value> {
    if let Some(id) = c.ident() {
        let v = match id.to_ascii_lowercase().as_str() {
            "baseline" => VerticalAlign::Baseline,
            "sub" => VerticalAlign::Sub,
            "super" => VerticalAlign::Super,
            "text-top" => VerticalAlign::TextTop,
            "text-bottom" => VerticalAlign::TextBottom,
            "middle" => VerticalAlign::Middle,
            "top" => VerticalAlign::Top,
            "bottom" => VerticalAlign::Bottom,
            _ => return None,
        };
        return Some(Value::VerticalAlign(v));
    }
    Some(Value::VerticalAlign(VerticalAlign::Lp(p_lp(c, q, false)?)))
}

fn parse_cursor(cvs: &[Cv]) -> Option<Value> {
    // [ <url> [ <x> <y> ]? , ]* <keyword>
    let parts = split_commas(cvs);
    let last = trim_ws(parts.last()?);
    let [Cv::Tok(Token::Ident(k))] = last else { return None };
    let kind = CursorKind::from_keyword(k)?;
    for p in parts.iter().take(parts.len() - 1) {
        let p = trim_ws(p);
        let first = p.first()?;
        let is_url = matches!(first, Cv::Tok(Token::Url(_))) || matches!(first, Cv::Func(n, _) if n.eq_ignore_ascii_case("url") || n.eq_ignore_ascii_case("image-set"));
        if !is_url {
            return None;
        }
    }
    Some(Value::Cursor(kind))
}

fn parse_list_style_type(c: &Cv) -> Option<Value> {
    match c {
        Cv::Tok(Token::Ident(k)) => ListStyleType::from_keyword(k).map(Value::ListStyleType),
        Cv::Tok(Token::Str(s)) => Some(Value::ListStyleType(ListStyleType::Str(Rc::from(s.as_str())))),
        Cv::Func(n, _) if n.eq_ignore_ascii_case("symbols") => Some(Value::ListStyleType(ListStyleType::Other(Rc::from("symbols")))),
        _ => None,
    }
}

fn parse_text_decoration_line(items: &[&Cv]) -> Option<Value> {
    if let [c] = items
        && c.is_ident("none")
    {
        return Some(Value::TextDecorationLine(TextDecorationLine::NONE));
    }
    let mut flags = 0u8;
    for c in items {
        let f = match c.ident()?.to_ascii_lowercase().as_str() {
            "underline" => TextDecorationLine::UNDERLINE,
            "overline" => TextDecorationLine::OVERLINE,
            "line-through" => TextDecorationLine::LINE_THROUGH,
            "blink" => TextDecorationLine::BLINK,
            _ => return None,
        };
        if flags & f != 0 {
            return None;
        }
        flags |= f;
    }
    if flags == 0 { None } else { Some(Value::TextDecorationLine(TextDecorationLine(flags))) }
}

fn parse_text_decoration_thickness(c: &Cv, q: bool) -> Option<Value> {
    if let Some(id) = c.ident() {
        return match id.to_ascii_lowercase().as_str() {
            "auto" | "from-font" => Some(Value::TextDecorationThickness(None)),
            _ => None,
        };
    }
    Some(Value::TextDecorationThickness(Some(p_lp(c, q, true)?)))
}

fn parse_text_decoration(items: &[&Cv], q: bool) -> Option<Vec<(PropId, Value)>> {
    let mut line: Vec<&Cv> = Vec::new();
    let mut style = None;
    let mut color = None;
    let mut thick = None;
    let mut none = false;
    for c in items {
        if let Some(id) = c.ident() {
            let l = id.to_ascii_lowercase();
            if l == "none" {
                none = true;
                continue;
            }
            if matches!(l.as_str(), "underline" | "overline" | "line-through" | "blink") {
                line.push(c);
                continue;
            }
            if let Some(s) = TextDecorationStyle::from_keyword(&l) {
                if style.is_some() {
                    return None;
                }
                style = Some(s);
                continue;
            }
            if matches!(l.as_str(), "auto" | "from-font") {
                if thick.is_some() {
                    return None;
                }
                thick = Some(Value::TextDecorationThickness(None));
                continue;
            }
        }
        if color.is_none()
            && let Some(v) = parse_color_cv(c)
        {
            color = Some(v);
        } else if thick.is_none()
            && let Some(v) = parse_text_decoration_thickness(c, q)
        {
            thick = Some(v);
        } else {
            return None;
        }
    }
    let line_v = if line.is_empty() {
        if !none && color.is_none() && style.is_none() && thick.is_none() {
            return None;
        }
        Value::TextDecorationLine(TextDecorationLine::NONE)
    } else if none {
        return None;
    } else {
        parse_text_decoration_line(&line)?
    };
    Some(vec![
        (PropId::TextDecorationLine, line_v),
        (PropId::TextDecorationStyle, Value::TextDecorationStyle(style.unwrap_or(TextDecorationStyle::Solid))),
        (PropId::TextDecorationColor, Value::Color(color.unwrap_or(ColorValue::CurrentColor))),
        (PropId::TextDecorationThickness, thick.unwrap_or(Value::TextDecorationThickness(None))),
    ])
}

fn parse_list_style(items: &[&Cv]) -> Option<Vec<(PropId, Value)>> {
    let mut ty = None;
    let mut pos = None;
    let mut none_count = 0;
    let mut image_seen = false;
    for c in items {
        if c.is_ident("none") {
            none_count += 1;
            continue;
        }
        if let Some(p) = kw(c, ListStylePosition::from_keyword) {
            if pos.is_some() {
                return None;
            }
            pos = Some(p);
            continue;
        }
        if matches!(c, Cv::Tok(Token::Url(_))) || matches!(c, Cv::Func(n, _) if n.eq_ignore_ascii_case("url") || n.contains("gradient")) {
            if image_seen {
                return None;
            }
            image_seen = true;
            continue;
        }
        if ty.is_none()
            && let Some(Value::ListStyleType(t)) = parse_list_style_type(c)
        {
            ty = Some(t);
        } else {
            return None;
        }
    }
    // `none` stands for the type and/or the image.
    let ty = match (ty, none_count, image_seen) {
        (Some(t), n, _) if n <= 1 && !(n == 1 && image_seen) => t,
        (Some(_), _, _) => return None,
        (None, 0, _) => ListStyleType::Disc,
        (None, 1, true) => ListStyleType::Disc,
        (None, 1, false) => ListStyleType::None,
        (None, 2, false) => ListStyleType::None,
        _ => return None,
    };
    Some(vec![
        (PropId::ListStyleType, Value::ListStyleType(ty)),
        (PropId::ListStylePosition, Value::ListStylePosition(pos.unwrap_or(ListStylePosition::Outside))),
    ])
}

fn parse_white_space(items: &[&Cv]) -> Option<Vec<(PropId, Value)>> {
    use WhiteSpaceCollapse as C;
    if let [c] = items {
        let (col, wrap) = match c.ident()?.to_ascii_lowercase().as_str() {
            "normal" => (C::Collapse, TextWrapMode::Wrap),
            "pre" => (C::Preserve, TextWrapMode::Nowrap),
            "nowrap" => (C::Collapse, TextWrapMode::Nowrap),
            "pre-wrap" => (C::Preserve, TextWrapMode::Wrap),
            "pre-line" => (C::PreserveBreaks, TextWrapMode::Wrap),
            "break-spaces" => (C::BreakSpaces, TextWrapMode::Wrap),
            "collapse" => (C::Collapse, TextWrapMode::Wrap),
            "preserve" => (C::Preserve, TextWrapMode::Wrap),
            "preserve-breaks" => (C::PreserveBreaks, TextWrapMode::Wrap),
            "preserve-spaces" => (C::PreserveSpaces, TextWrapMode::Wrap),
            "wrap" => (C::Collapse, TextWrapMode::Wrap),
            _ => return None,
        };
        return Some(vec![(PropId::WhiteSpaceCollapse, Value::WhiteSpaceCollapse(col)), (PropId::TextWrapMode, Value::TextWrapMode(wrap))]);
    }
    // <'white-space-collapse'> || <'text-wrap-mode'>
    let mut col = None;
    let mut wrap = None;
    for c in items {
        let id = c.ident()?;
        if col.is_none()
            && let Some(v) = C::from_keyword(id)
        {
            col = Some(v);
        } else if wrap.is_none()
            && let Some(v) = TextWrapMode::from_keyword(id)
        {
            wrap = Some(v);
        } else {
            return None;
        }
    }
    Some(vec![
        (PropId::WhiteSpaceCollapse, Value::WhiteSpaceCollapse(col.unwrap_or(C::Collapse))),
        (PropId::TextWrapMode, Value::TextWrapMode(wrap.unwrap_or(TextWrapMode::Wrap))),
    ])
}

/// `background`: only the color is kept. The other components are validated loosely.
fn parse_background(cvs: &[Cv]) -> Option<Vec<(PropId, Value)>> {
    let layers = split_commas(cvs);
    let mut color = Value::Color(ColorValue::Color(super::color::Color::TRANSPARENT));
    let n = layers.len();
    for (i, layer) in layers.iter().enumerate() {
        let items: Vec<&Cv> = trim_ws(layer).iter().filter(|c| !c.is_ws()).collect();
        if items.is_empty() {
            return None;
        }
        let mut layer_color = None;
        for c in items {
            if let Some(v) = parse_color_cv(c) {
                if layer_color.is_some() {
                    return None;
                }
                layer_color = Some(v);
                continue;
            }
            let ok = match c {
                Cv::Tok(Token::Url(_)) | Cv::Tok(Token::Str(_)) => true,
                Cv::Func(..) => true,
                Cv::Tok(Token::Ident(id)) => matches!(
                    id.to_ascii_lowercase().as_str(),
                    "none" | "scroll" | "fixed" | "local" | "repeat" | "repeat-x" | "repeat-y" | "no-repeat" | "space" | "round"
                        | "border-box" | "padding-box" | "content-box" | "text" | "top" | "bottom" | "left" | "right" | "center"
                        | "auto" | "cover" | "contain"
                ),
                Cv::Tok(Token::Delim('/')) => true,
                Cv::Tok(Token::Number { .. } | Token::Percentage { .. } | Token::Dimension { .. }) => true,
                _ => false,
            };
            if !ok {
                return None;
            }
        }
        if i + 1 == n {
            if let Some(c) = layer_color {
                color = Value::Color(c);
            }
        } else if layer_color.is_some() {
            return None; // only the final layer may have a color
        }
    }
    Some(vec![(PropId::BackgroundColor, color)])
}

fn parse_font_shorthand(cvs: &[Cv], q: bool) -> Option<Vec<(PropId, Value)>> {
    let items: Vec<&Cv> = cvs.iter().filter(|c| !c.is_ws()).collect();
    // System fonts.
    if let [c] = items.as_slice()
        && let Some(id) = c.ident()
        && matches!(id.to_ascii_lowercase().as_str(), "caption" | "icon" | "menu" | "message-box" | "small-caption" | "status-bar")
    {
        return Some(vec![
            (PropId::FontStyle, Value::FontStyle(FontStyleKind::Normal)),
            (PropId::FontWeight, Value::FontWeight(FontWeightSpec::Number(400.0))),
            (PropId::FontSize, Value::FontSize(FontSizeSpec::Lp(Lp::px(13.333_334)))),
            (PropId::LineHeight, Value::LineHeight(LineHeightSpec::Normal)),
            (PropId::FontFamily, Value::FontFamily(Rc::new(vec![FontFamily::Generic(GenericFamily::SystemUi)]))),
        ]);
    }
    let mut style = None;
    let mut weight = None;
    let mut i = 0usize;
    // [ <font-style> || <font-variant-css2> || <font-weight> || <font-stretch-css3> ]?
    let mut normals = 0;
    while let Some(c) = items.get(i) {
        let Some(id) = c.ident() else {
            if weight.is_none()
                && let Some(w) = parse_font_weight(c)
            {
                weight = Some(w);
                i += 1;
                continue;
            }
            break;
        };
        let l = id.to_ascii_lowercase();
        if l == "normal" {
            normals += 1;
            i += 1;
        } else if let Some(s) = FontStyleKind::from_keyword(&l).filter(|s| *s != FontStyleKind::Normal) {
            if style.is_some() {
                return None;
            }
            style = Some(s);
            i += 1;
            // `oblique <angle>`
            if s == FontStyleKind::Oblique
                && let Some(Cv::Tok(Token::Dimension { unit, .. })) = items.get(i)
                && matches!(unit.to_ascii_lowercase().as_str(), "deg" | "rad" | "grad" | "turn")
            {
                i += 1;
            }
        } else if matches!(l.as_str(), "bold" | "bolder" | "lighter") {
            if weight.is_some() {
                return None;
            }
            weight = parse_font_weight(c);
            i += 1;
        } else if l == "small-caps" || matches!(l.as_str(), "ultra-condensed" | "extra-condensed" | "condensed" | "semi-condensed" | "semi-expanded" | "expanded" | "extra-expanded" | "ultra-expanded") {
            i += 1;
        } else {
            break;
        }
        if normals > 4 {
            return None;
        }
    }
    let size = parse_font_size(items.get(i)?, q)?;
    i += 1;
    let mut line_height = Value::LineHeight(LineHeightSpec::Normal);
    if items.get(i).is_some_and(|c| c.is_delim('/')) {
        line_height = parse_line_height(items.get(i + 1)?, q)?;
        i += 2;
    }
    // The family list is everything that remains (including commas): re-slice from the original values.
    let rest_start = {
        // Find the position of items[i] in cvs by pointer identity.
        let first = items.get(i)?;
        cvs.iter().position(|c| std::ptr::eq(c, *first))?
    };
    let family = parse_font_family(cvs.get(rest_start..)?)?;
    Some(vec![
        (PropId::FontStyle, Value::FontStyle(style.unwrap_or(FontStyleKind::Normal))),
        (PropId::FontWeight, weight.unwrap_or(Value::FontWeight(FontWeightSpec::Number(400.0)))),
        (PropId::FontSize, size),
        (PropId::LineHeight, line_height),
        (PropId::FontFamily, family),
    ])
}

/// A longhand value. `cvs` is the trimmed value, `items` its non-white-space components.
fn parse_longhand(prop: PropId, cvs: &[Cv], items: &[&Cv], q: bool) -> Option<Value> {
    use PropId::*;
    let single = |items: &[&Cv]| -> Option<Cv> { if items.len() == 1 { items.first().map(|c| (*c).clone()) } else { None } };
    let one = single(items);
    let one_ref = one.as_ref();
    match prop {
        Display => parse_display(items),
        Position => kw(one_ref?, super::types::Position::from_keyword).map(Value::Position),
        Float => {
            let id = one_ref?.ident()?.to_ascii_lowercase();
            // Logical values map to the physical ones in ltr.
            let f = match id.as_str() {
                "inline-start" => super::types::Float::Left,
                "inline-end" => super::types::Float::Right,
                other => super::types::Float::from_keyword(other)?,
            };
            Some(Value::Float(f))
        }
        Clear => {
            let id = one_ref?.ident()?.to_ascii_lowercase();
            let c = match id.as_str() {
                "inline-start" => super::types::Clear::Left,
                "inline-end" => super::types::Clear::Right,
                other => super::types::Clear::from_keyword(other)?,
            };
            Some(Value::Clear(c))
        }
        BoxSizing => kw(one_ref?, super::types::BoxSizing::from_keyword).map(Value::BoxSizing),
        ZIndex => {
            let c = one_ref?;
            if c.is_ident("auto") {
                return Some(Value::ZIndex(None));
            }
            parse_integer(c).map(|i| Value::ZIndex(Some(i)))
        }
        OverflowX | OverflowY => p_overflow(one_ref?).map(Value::Overflow),
        Top | Right | Bottom | Left => p_offset(one_ref?, q),
        Width | Height | MinWidth | MinHeight => p_size(one_ref?, q, false),
        MaxWidth | MaxHeight => p_size(one_ref?, q, true),
        MarginTop | MarginRight | MarginBottom | MarginLeft => p_margin(one_ref?, q),
        PaddingTop | PaddingRight | PaddingBottom | PaddingLeft => p_padding(one_ref?, q),
        BorderTopWidth | BorderRightWidth | BorderBottomWidth | BorderLeftWidth => p_border_width(one_ref?, q),
        BorderTopStyle | BorderRightStyle | BorderBottomStyle | BorderLeftStyle => p_border_style(one_ref?),
        BorderTopColor | BorderRightColor | BorderBottomColor | BorderLeftColor | Color | TextDecorationColor | BackgroundColor => {
            p_color(one_ref?)
        }
        FontFamily => parse_font_family(cvs),
        FontSize => parse_font_size(one_ref?, q),
        FontWeight => parse_font_weight(one_ref?),
        FontStyle => {
            // normal | italic | oblique <angle>?
            let first = items.first()?;
            let k = super::types::FontStyleKind::from_keyword(first.ident()?)?;
            match items.len() {
                1 => Some(Value::FontStyle(k)),
                2 if k == super::types::FontStyleKind::Oblique && matches!(items.get(1), Some(Cv::Tok(Token::Dimension { .. }))) => {
                    Some(Value::FontStyle(k))
                }
                _ => None,
            }
        }
        LineHeight => parse_line_height(one_ref?, q),
        LetterSpacing | WordSpacing => {
            let c = one_ref?;
            if c.is_ident("normal") {
                return Some(Value::Spacing(None));
            }
            let l = match parse_length(c, true) {
                Some(l) => l,
                None if q => match c {
                    Cv::Tok(Token::Number { value, .. }) => Lp::px(*value as f32),
                    _ => return None,
                },
                None => return None,
            };
            Some(Value::Spacing(Some(l)))
        }
        TextAlign => {
            let id = one_ref?.ident()?.to_ascii_lowercase();
            let a = match id.as_str() {
                "-webkit-center" | "-moz-center" => super::types::TextAlign::Center,
                "-webkit-left" | "-moz-left" => super::types::TextAlign::Left,
                "-webkit-right" | "-moz-right" => super::types::TextAlign::Right,
                other => super::types::TextAlign::from_keyword(other)?,
            };
            Some(Value::TextAlign(a))
        }
        TextIndent => {
            let lp = p_lp(items.first()?, q, false)?;
            for extra in items.iter().skip(1) {
                if !(extra.is_ident("hanging") || extra.is_ident("each-line")) {
                    return None;
                }
            }
            Some(Value::TextIndent(lp))
        }
        TabSize => {
            let it = one_ref?;
            match it {
                Cv::Tok(Token::Number { value, .. }) if *value >= 0.0 => Some(Value::TabSize(super::types::TabSize::Spaces(*value as f32))),
                _ => {
                    let l = parse_length(it, true)?;
                    (!l.is_negative_length()).then_some(Value::TabSize(super::types::TabSize::Len(l)))
                }
            }
        }
        TextTransform => kw(one_ref?, super::types::TextTransform::from_keyword).map(Value::TextTransform),
        WhiteSpaceCollapse => kw(one_ref?, super::types::WhiteSpaceCollapse::from_keyword).map(Value::WhiteSpaceCollapse),
        TextWrapMode => kw(one_ref?, super::types::TextWrapMode::from_keyword).map(Value::TextWrapMode),
        WordBreak => kw(one_ref?, super::types::WordBreak::from_keyword).map(Value::WordBreak),
        OverflowWrap => kw(one_ref?, super::types::OverflowWrap::from_keyword).map(Value::OverflowWrap),
        Direction => kw(one_ref?, super::types::Direction::from_keyword).map(Value::Direction),
        Visibility => kw(one_ref?, super::types::Visibility::from_keyword).map(Value::Visibility),
        Cursor => parse_cursor(cvs),
        ListStyleType => parse_list_style_type(one_ref?),
        ListStylePosition => kw(one_ref?, super::types::ListStylePosition::from_keyword).map(Value::ListStylePosition),
        VerticalAlign => parse_vertical_align(one_ref?, q),
        TextDecorationLine => parse_text_decoration_line(items),
        TextDecorationStyle => kw(one_ref?, super::types::TextDecorationStyle::from_keyword).map(Value::TextDecorationStyle),
        TextDecorationThickness => parse_text_decoration_thickness(one_ref?, q),
        Content => parse_content(items),
        Quotes => parse_quotes(items),
        CounterReset | CounterSet => parse_counters(items, 0),
        BorderCollapse => kw(one_ref?, super::types::BorderCollapse::from_keyword).map(Value::BorderCollapse),
        EmptyCells => kw(one_ref?, super::types::EmptyCells::from_keyword).map(Value::EmptyCells),
        CaptionSide => kw(one_ref?, super::types::CaptionSide::from_keyword).map(Value::CaptionSide),
        TableLayout => kw(one_ref?, super::types::TableLayout::from_keyword).map(Value::TableLayout),
        FlexDirection => kw(one_ref?, super::types::FlexDirection::from_keyword).map(Value::FlexDirection),
        FlexWrap => kw(one_ref?, super::types::FlexWrap::from_keyword).map(Value::FlexWrap),
        Order => parse_integer(one_ref?).map(Value::Order),
        FlexGrow | FlexShrink => flex_number(one_ref?).map(Value::FlexFactor),
        FlexBasis => p_flex_basis(one_ref?, q).map(Value::FlexBasis),
        JustifyContent | AlignItems | AlignSelf | AlignContent | JustifyItems | JustifySelf => parse_align(prop, items),
        GridTemplateColumns | GridTemplateRows => super::parse_grid::parse_tracks(items),
        GridTemplateAreas => super::parse_grid::parse_areas(items),
        GridAutoColumns | GridAutoRows => super::parse_grid::parse_auto_tracks(items),
        GridAutoFlow => super::parse_grid::parse_flow(items),
        GridRowStart | GridColumnStart | GridRowEnd | GridColumnEnd => super::parse_grid::grid_line(items).map(Value::GridLine),
        RowGap | ColumnGap => p_gap(one_ref?, q),
        BorderSpacing => {
            if items.len() > 2 {
                return None;
            }
            let a = p_lp(items.first()?, q, true)?;
            let b = match items.get(1) {
                Some(c) => p_lp(c, q, true)?,
                None => a.clone(),
            };
            Some(Value::TableSpacing(a, b))
        }
        CounterIncrement => parse_counters(items, 1),
        BorderTopLeftRadius | BorderTopRightRadius | BorderBottomRightRadius | BorderBottomLeftRadius => {
            if (1..=2).contains(&items.len()) && items.iter().all(|c| radius_ok(c, q)) {
                Some(Value::Ignored)
            } else {
                None
            }
        }
    }
}

/// A string of a `content` or `quotes` value, cut to a sane length.
fn gen_str(s: &str) -> Rc<str> {
    const MAX_GEN_STR: usize = 4096;
    match s.char_indices().nth(MAX_GEN_STR) {
        Some((i, _)) => Rc::from(s.get(..i).unwrap_or(s)),
        None => Rc::from(s),
    }
}

/// A counter name: any identifier but `none` and the CSS-wide keywords (css-lists-3 4.1, css-values-4 2.1).
fn counter_name(c: &Cv) -> Option<Rc<str>> {
    let id = c.ident()?;
    let l = id.to_ascii_lowercase();
    (!matches!(l.as_str(), "none" | "inherit" | "initial" | "unset" | "default" | "revert" | "revert-layer")).then(|| gen_str(id))
}

/// The arguments of a function split at the commas, white space dropped.
fn comma_groups(args: &[Cv]) -> Vec<Vec<&Cv>> {
    let mut groups: Vec<Vec<&Cv>> = vec![Vec::new()];
    for a in args {
        match a {
            Cv::Tok(Token::Comma) => groups.push(Vec::new()),
            c if c.is_ws() => {}
            c => {
                if let Some(g) = groups.last_mut() {
                    g.push(c);
                }
            }
        }
    }
    groups
}

/// `counter-style` of `counter()` / `counters()`: absent is `decimal`.
fn counter_style(g: Option<&Vec<&Cv>>) -> Option<ListStyleType> {
    match g.map(Vec::as_slice) {
        None => Some(ListStyleType::Decimal),
        // `none` is not a <counter-style-name> (css-counter-styles-3 2): the whole `content` is invalid.
        Some([c]) if c.ident().is_some_and(|i| i.eq_ignore_ascii_case("none")) => None,
        Some([c]) => ListStyleType::from_keyword(c.ident()?),
        Some(_) => None,
    }
}

/// `content` (CSS2 12.2). What follows a `/` is alternative text (css-content-3 2), not shown.
fn parse_content(items: &[&Cv]) -> Option<Value> {
    use super::types::{Content, ContentItem};
    if let [c] = items
        && let Some(id) = c.ident()
    {
        if id.eq_ignore_ascii_case("normal") {
            return Some(Value::Content(Content::Normal));
        }
        if id.eq_ignore_ascii_case("none") {
            return Some(Value::Content(Content::None));
        }
    }
    let mut out: Vec<ContentItem> = Vec::new();
    for c in items {
        let item = match c {
            Cv::Tok(Token::Str(s)) => ContentItem::Str(gen_str(s)),
            Cv::Tok(Token::Url(u)) => ContentItem::Url(gen_str(u)),
            Cv::Tok(Token::Delim('/')) if !out.is_empty() => break,
            Cv::Tok(Token::Ident(k)) => match k.to_ascii_lowercase().as_str() {
                "open-quote" => ContentItem::OpenQuote,
                "close-quote" => ContentItem::CloseQuote,
                "no-open-quote" => ContentItem::NoOpenQuote,
                "no-close-quote" => ContentItem::NoCloseQuote,
                _ => return None,
            },
            Cv::Func(n, args) => {
                let g = comma_groups(args);
                match n.to_ascii_lowercase().as_str() {
                    "url" => match g.first().map(Vec::as_slice) {
                        Some([Cv::Tok(Token::Str(u))]) => ContentItem::Url(gen_str(u)),
                        _ => return None,
                    },
                    "attr" => match g.first().map(Vec::as_slice) {
                        Some([n]) => ContentItem::Attr(gen_str(n.ident()?)),
                        _ => return None,
                    },
                    "counter" if (1..=2).contains(&g.len()) => {
                        let name = match g.first().map(Vec::as_slice) {
                            Some([n]) => counter_name(n)?,
                            _ => return None,
                        };
                        ContentItem::Counter(name, counter_style(g.get(1))?)
                    }
                    "counters" if (2..=3).contains(&g.len()) => {
                        let name = match g.first().map(Vec::as_slice) {
                            Some([n]) => counter_name(n)?,
                            _ => return None,
                        };
                        let sep = match g.get(1).map(Vec::as_slice) {
                            Some([Cv::Tok(Token::Str(s))]) => gen_str(s),
                            _ => return None,
                        };
                        ContentItem::Counters(name, sep, counter_style(g.get(2))?)
                    }
                    _ => return None,
                }
            }
            _ => return None,
        };
        if out.len() < super::types::MAX_GEN_ITEMS {
            out.push(item);
        }
    }
    (!out.is_empty()).then(|| Value::Content(Content::Items(Rc::new(out))))
}

/// `quotes`: `auto`, `none` or pairs of strings (CSS2 12.3.1).
fn parse_quotes(items: &[&Cv]) -> Option<Value> {
    use super::types::Quotes;
    if let [c] = items
        && let Some(id) = c.ident()
    {
        if id.eq_ignore_ascii_case("auto") {
            return Some(Value::Quotes(Quotes::Auto));
        }
        if id.eq_ignore_ascii_case("none") {
            return Some(Value::Quotes(Quotes::None));
        }
        return None;
    }
    if items.is_empty() || !items.len().is_multiple_of(2) {
        return None;
    }
    let strs: Vec<Rc<str>> = items
        .iter()
        .map(|c| match c {
            Cv::Tok(Token::Str(s)) => Some(gen_str(s)),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()?;
    let pairs: Vec<(Rc<str>, Rc<str>)> = strs.chunks(2).filter_map(|p| Some((p.first()?.clone(), p.get(1)?.clone()))).take(super::types::MAX_GEN_ITEMS).collect();
    Some(Value::Quotes(Quotes::Pairs(Rc::new(pairs))))
}

/// `counter-reset`, `counter-increment`, `counter-set`: `none` or names each with an optional integer
/// (`default` when there is none).
fn parse_counters(items: &[&Cv], default: i32) -> Option<Value> {
    if let [c] = items
        && c.ident().is_some_and(|i| i.eq_ignore_ascii_case("none"))
    {
        return Some(Value::Counters(Rc::new(Vec::new())));
    }
    let mut out: Vec<(Rc<str>, i32)> = Vec::new();
    let mut i = 0;
    while let Some(c) = items.get(i) {
        let name = counter_name(c)?;
        i += 1;
        let mut value = default;
        if let Some(Cv::Tok(Token::Number { value: v, is_int, .. })) = items.get(i) {
            if !*is_int {
                return None;
            }
            value = (*v).clamp(i32::MIN as f64, i32::MAX as f64) as i32;
            i += 1;
        }
        if out.len() < super::types::MAX_GEN_ITEMS {
            out.push((name, value));
        }
    }
    (!out.is_empty()).then(|| Value::Counters(Rc::new(out)))
}

#[allow(dead_code)]
fn _keep(_: BlockKind, _: LenUnit) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::css::syntax::parse_cvs;

    fn parse(name: &str, v: &str) -> Option<Vec<ParsedDecl>> {
        let mut id = 0;
        parse_declaration(name, &parse_cvs(v), &ParseEnv::default(), &mut id)
    }

    fn known(name: &str, v: &str) -> Vec<(PropId, Specified)> {
        parse(name, v).unwrap_or_default()
    }

    #[test]
    fn shorthands() {
        assert_eq!(known("margin", "1px 2px").len(), 4);
        assert_eq!(known("margin", "1px 2px 3px 4px 5px").len(), 0);
        assert_eq!(known("border", "1px solid red").len(), 12);
        assert_eq!(known("border-top", "solid").len(), 3);
        assert_eq!(known("font", "italic bold 12px/1.5 Arial, sans-serif").len(), 5);
        assert_eq!(known("font", "12px").len(), 0); // family is required
        assert_eq!(known("background", "url(a.png) no-repeat #fff").len(), 1);
        assert_eq!(known("list-style", "none").len(), 2);
        assert_eq!(known("text-decoration", "underline dotted red").len(), 4);
        assert_eq!(known("overflow", "hidden auto").len(), 2);
    }

    #[test]
    fn invalid_values_drop() {
        assert!(parse("width", "-5px").is_none());
        assert!(parse("display", "bogus").is_none());
        assert!(parse("color", "notacolor").is_none());
        assert!(parse("unknown-prop", "1").is_none());
        assert!(parse("padding", "1px auto").is_none());
    }

    #[test]
    fn css_wide_and_var() {
        let d = known("margin", "inherit");
        assert!(d.iter().all(|(_, s)| *s == Specified::Inherit) && d.len() == 4);
        let d = known("color", "var(--x, red)");
        assert!(matches!(d.first(), Some((PropId::Color, Specified::Var(_)))));
        let d = known("margin", "var(--m)");
        assert_eq!(d.len(), 4);
    }

    #[test]
    fn quirks_lengths() {
        let mut id = 0;
        let q = ParseEnv { quirks: true };
        assert!(parse_declaration("width", &parse_cvs("100"), &q, &mut id).is_some());
        assert!(parse_declaration("width", &parse_cvs("100"), &ParseEnv::default(), &mut id).is_none());
    }
}
