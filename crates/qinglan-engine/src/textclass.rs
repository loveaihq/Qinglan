//! Character classes shared by the test platform's line breaking and the engine's white-space
//! processing. Hand-written ranges for the scripts layer 1 cares about (Chinese, Japanese,
//! Korean); the Windows platform uses DirectWrite's UAX #14 instead for the breaking itself.

/// Spaces after which a line may break (a subset of UAX #14 class SP/BA): space, tab, OGHAM space,
/// EN QUAD..HAIR SPACE except the no-break FIGURE SPACE, MEDIUM MATHEMATICAL SPACE.
pub fn is_space_like(c: char) -> bool {
    matches!(c as u32, 0x20 | 0x09 | 0x1680 | 0x2000..=0x2006 | 0x2008..=0x200A | 0x205F)
}

pub fn is_hangul(c: char) -> bool {
    matches!(c as u32, 0x1100..=0x11FF | 0x3130..=0x318F | 0xA960..=0xA97F | 0xAC00..=0xD7FF | 0xFFA0..=0xFFDC)
}

/// Han, kana, bopomofo, Hangul, CJK symbols and punctuation, fullwidth forms: the characters
/// between which a line may break without a space.
pub fn is_cjk_break(c: char) -> bool {
    matches!(
        c as u32,
        0x1100..=0x11FF
            | 0x2E80..=0x2FFF
            | 0x3000..=0x33FF
            | 0x3400..=0x4DBF
            | 0x4E00..=0x9FFF
            | 0xA000..=0xA4CF
            | 0xA960..=0xA97F
            | 0xAC00..=0xD7FF
            | 0xF900..=0xFAFF
            | 0xFE30..=0xFE4F
            | 0xFF00..=0xFF60
            | 0xFFE0..=0xFFE6
            | 0x1B000..=0x1B16F
            | 0x20000..=0x3FFFD
    )
}

/// Punctuation that must not start a line: the Chinese "后置标点" and its Japanese/fullwidth kin, plus
/// the ASCII ones (UAX #14 classes CL, EX, IS), so that mixed Chinese/English text breaks sensibly.
pub fn is_closing_punct(c: char) -> bool {
    matches!(
        c,
        ',' | '.' | ';' | ':' | '!' | '?' | ')' | ']' | '}'
            | '，' | '。'
            | '、'
            | '；'
            | '：'
            | '？'
            | '！'
            | '）'
            | '」'
            | '』'
            | '】'
            | '〕'
            | '〉'
            | '》'
            | '〗'
            | '〙'
            | '〞'
            | '…'
            | '‥'
            | '・'
            | '．'
            | '］'
            | '｝'
            | '｠'
            | '｡'
            | '､'
            | '｣'
    )
}

/// Punctuation that must not end a line.
pub fn is_opening_punct(c: char) -> bool {
    matches!(c, '(' | '[' | '{' | '（' | '「' | '『' | '【' | '〔' | '〈' | '《' | '〖' | '〘' | '〝' | '［' | '｛' | '｟' | '｢')
}

/// East Asian Width Wide, Fullwidth or Halfwidth, but not Hangul (css-text-4 "segment break
/// transformation rules": a line feed between two such characters is removed instead of becoming a space).
pub fn is_east_asian_wide_non_hangul(c: char) -> bool {
    if is_hangul(c) {
        return false;
    }
    matches!(
        c as u32,
        0x2E80..=0x303E
            | 0x3041..=0x33FF
            | 0x3400..=0x4DBF
            | 0x4E00..=0x9FFF
            | 0xA000..=0xA4CF
            | 0xF900..=0xFAFF
            | 0xFE30..=0xFE6F
            | 0xFF01..=0xFF60
            | 0xFF61..=0xFF9F
            | 0xFFE0..=0xFFE6
            | 0x1B000..=0x1B2FF
            | 0x20000..=0x3FFFD
    )
}
