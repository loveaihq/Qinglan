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

/// The character is, or may be, bidirectional-relevant (UAX #9): a right-to-left script or presentation form, or an explicit
/// formatting character. A paragraph without one (and a left-to-right base direction) needs no bidi analysis at all.
pub fn is_bidi_relevant(c: char) -> bool {
    matches!(c as u32,
        0x0590..=0x08FF | 0x200E | 0x200F | 0x202A..=0x202E | 0x2066..=0x2069 | 0xFB1D..=0xFDFF | 0xFE70..=0xFEFF | 0x10800..=0x10FFF | 0x1E800..=0x1EFFF)
}

/// Is the character set upright in vertical text with `text-orientation: mixed` (UAX #50 Vertical_Orientation U and Tu, for the
/// common scripts: CJK, kana, Hangul, Bopomofo, CJK and full-width punctuation and symbols, emoji)? The rest is turned sideways.
pub fn is_vertical_upright(c: char) -> bool {
    matches!(c as u32,
        0x1100..=0x11FF | 0x2460..=0x24FF | 0x25A0..=0x27BF | 0x2E80..=0x33FF | 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xA000..=0xA4CF
        | 0xA960..=0xA97F | 0xAC00..=0xD7FF | 0xF900..=0xFAFF | 0xFE10..=0xFE1F | 0xFE30..=0xFE6F | 0xFF01..=0xFF60 | 0xFFE0..=0xFFE6
        | 0x1B000..=0x1B16F | 0x1F000..=0x1FAFF | 0x20000..=0x3FFFD)
}

/// The strong direction of a character for "first strong" decisions (`dir=auto`, `unicode-bidi: plaintext`; UAX #9 P2):
/// `Some(true)` for R and AL letters, `Some(false)` for L, `None` for everything else (digits, marks, neutrals).
pub fn strong_rtl(c: char) -> Option<bool> {
    let u = c as u32;
    match u {
        0x200F => Some(true),
        0x200E => Some(false),
        0x05BE | 0x05C0 | 0x05C3 | 0x05C6 | 0x05D0..=0x05FF | 0x0608 | 0x060B | 0x060D | 0x061B..=0x064A | 0x066D..=0x066F | 0x0671..=0x06D5
        | 0x06E5 | 0x06E6 | 0x06EE | 0x06EF | 0x06FA..=0x0710 | 0x0712..=0x072F | 0x074D..=0x07A5 | 0x07B1 | 0x07C0..=0x07EA | 0x07F4..=0x07FA
        | 0x0800..=0x0815 | 0x081A | 0x0824 | 0x0828 | 0x0830..=0x083E | 0x0840..=0x0858 | 0x085E | 0x0860..=0x08B4 | 0x08B6..=0x08C7
        | 0xFB1D | 0xFB1F..=0xFB28 | 0xFB2A..=0xFB4F | 0xFB50..=0xFD3D | 0xFD50..=0xFDFC | 0xFE70..=0xFEFC | 0x10800..=0x10FFF | 0x1E800..=0x1EFFF => Some(true),
        _ if u < 0x80 => c.is_ascii_alphabetic().then_some(false),
        _ => c.is_alphabetic().then_some(false),
    }
}
