//! List markers: the text of a `::marker` for the counter styles of layer 1 (css-counter-styles-3
//! sections 6.1 to 6.4, css-lists-3 3). Symbol markers are text too (the Windows platform draws the
//! bullet glyph; the test platform draws a 1em square).

use crate::css::types::ListStyleType;

fn decimal(n: i32) -> String {
    n.to_string()
}

/// Bijective base-`digits.len()` numbering ("alphabetic" system, 6.1.2): 1 -> a, 26 -> z, 27 -> aa.
fn alphabetic(n: i32, digits: &[char]) -> Option<String> {
    if n < 1 || digits.is_empty() {
        return None;
    }
    let base = digits.len() as i64;
    let mut v = n as i64;
    let mut out = Vec::new();
    while v > 0 {
        v -= 1;
        out.push(digits.get((v % base) as usize).copied()?);
        v /= base;
    }
    out.reverse();
    Some(out.into_iter().collect())
}

/// "additive" system for roman numerals, range 1..=3999 (6.1.4 / 6.4).
fn roman(n: i32, upper: bool) -> Option<String> {
    if !(1..=3999).contains(&n) {
        return None;
    }
    const TABLE: [(i32, &str); 13] =
        [(1000, "m"), (900, "cm"), (500, "d"), (400, "cd"), (100, "c"), (90, "xc"), (50, "l"), (40, "xl"), (10, "x"), (9, "ix"), (5, "v"), (4, "iv"), (1, "i")];
    let mut v = n;
    let mut s = String::new();
    for (val, sym) in TABLE {
        while v >= val {
            s.push_str(sym);
            v -= val;
        }
    }
    Some(if upper { s.to_uppercase() } else { s })
}

fn cjk_decimal(n: i32) -> String {
    const DIGITS: [char; 10] = ['〇', '一', '二', '三', '四', '五', '六', '七', '八', '九'];
    n.to_string()
        .chars()
        .map(|c| c.to_digit(10).and_then(|d| DIGITS.get(d as usize).copied()).unwrap_or(c))
        .collect()
}

/// The marker text for list item number `n`: the counter text plus its suffix (css-counter-styles-3
/// 6.1 "suffix"), or `None` for `list-style-type: none`.
pub(crate) fn marker_text(ty: &ListStyleType, n: i32) -> Option<String> {
    const GREEK: [char; 24] = ['α', 'β', 'γ', 'δ', 'ε', 'ζ', 'η', 'θ', 'ι', 'κ', 'λ', 'μ', 'ν', 'ξ', 'ο', 'π', 'ρ', 'σ', 'τ', 'υ', 'φ', 'χ', 'ψ', 'ω'];
    let lower: Vec<char> = ('a'..='z').collect();
    let upper: Vec<char> = ('A'..='Z').collect();
    let dot = |s: String| Some(format!("{s}. "));
    match ty {
        ListStyleType::None => None,
        ListStyleType::Disc => Some("\u{2022} ".to_string()),
        ListStyleType::Circle => Some("\u{25E6} ".to_string()),
        ListStyleType::Square => Some("\u{25AA} ".to_string()),
        ListStyleType::Decimal => dot(decimal(n)),
        ListStyleType::DecimalLeadingZero => {
            let body = format!("{:02}", n.unsigned_abs());
            dot(if n < 0 { format!("-{body}") } else { body })
        }
        ListStyleType::LowerAlpha => dot(alphabetic(n, &lower).unwrap_or_else(|| decimal(n))),
        ListStyleType::UpperAlpha => dot(alphabetic(n, &upper).unwrap_or_else(|| decimal(n))),
        ListStyleType::LowerRoman => dot(roman(n, false).unwrap_or_else(|| decimal(n))),
        ListStyleType::UpperRoman => dot(roman(n, true).unwrap_or_else(|| decimal(n))),
        ListStyleType::LowerGreek => dot(alphabetic(n, &GREEK).unwrap_or_else(|| decimal(n))),
        ListStyleType::CjkDecimal => Some(format!("{}\u{3001}", cjk_decimal(n))),
        // `<details>` is always open in layer 1, so "closed" shows the open triangle too.
        ListStyleType::Other(name) if name.starts_with("disclosure-") => Some("\u{25BE} ".to_string()),
        // Counter styles layer 1 does not know fall back to decimal (6.1 "fallback").
        ListStyleType::Other(_) => dot(decimal(n)),
        ListStyleType::Str(s) => Some(s.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counter_styles() {
        assert_eq!(marker_text(&ListStyleType::Decimal, 7).as_deref(), Some("7. "));
        assert_eq!(marker_text(&ListStyleType::LowerAlpha, 28).as_deref(), Some("ab. "));
        assert_eq!(marker_text(&ListStyleType::UpperRoman, 1994).as_deref(), Some("MCMXCIV. "));
        assert_eq!(marker_text(&ListStyleType::LowerRoman, 4000).as_deref(), Some("4000. "));
        assert_eq!(marker_text(&ListStyleType::CjkDecimal, 10).as_deref(), Some("一〇\u{3001}"));
        assert_eq!(marker_text(&ListStyleType::DecimalLeadingZero, 3).as_deref(), Some("03. "));
        assert_eq!(marker_text(&ListStyleType::None, 1), None);
    }
}
