//! Bidirectional text (UAX #9, css-writing-modes-3 2.2 and 2.4): the CSS side. The platform resolves the levels
//! (`Platform::bidi_levels`); this module builds the string it analyses from the paragraph's items and reorders a line.
//!
//! `unicode-bidi` on an inline box is turned into the formatting characters CSS Writing Modes 3 2.2 names (embed: LRE/RLE
//! ... PDF, isolate: LRI/RLI ... PDI, bidi-override: LRO/RLO ... PDF, isolate-override both, plaintext: FSI ... PDI); atomic
//! inlines are U+FFFC (an object replacement character, neutral) and forced line breaks U+2028 (white space, so a `<br>`
//! does not start a new bidi paragraph; recorded in decisions.md).

use crate::css::types::UnicodeBidi;
use crate::platform::Platform;
use crate::textclass::strong_rtl;

const LRE: char = '\u{202A}';
const RLE: char = '\u{202B}';
const PDF: char = '\u{202C}';
const LRO: char = '\u{202D}';
const RLO: char = '\u{202E}';
const LRI: char = '\u{2066}';
const RLI: char = '\u{2067}';
const FSI: char = '\u{2068}';
const PDI: char = '\u{2069}';

/// UAX #9 allows 125 levels (BD2).
const MAX_DEPTH: u8 = 125;

/// The formatting characters that open and close a box with this `unicode-bidi` and `direction` (empty for `normal`).
pub(super) fn controls(ub: UnicodeBidi, rtl: bool) -> (&'static str, &'static str) {
    match (ub, rtl) {
        (UnicodeBidi::Normal, _) => ("", ""),
        (UnicodeBidi::Embed, false) => ("\u{202A}", "\u{202C}"),
        (UnicodeBidi::Embed, true) => ("\u{202B}", "\u{202C}"),
        (UnicodeBidi::Isolate, false) => ("\u{2066}", "\u{2069}"),
        (UnicodeBidi::Isolate, true) => ("\u{2067}", "\u{2069}"),
        (UnicodeBidi::BidiOverride, false) => ("\u{202D}", "\u{202C}"),
        (UnicodeBidi::BidiOverride, true) => ("\u{202E}", "\u{202C}"),
        (UnicodeBidi::IsolateOverride, false) => ("\u{2066}\u{202D}", "\u{202C}\u{2069}"),
        (UnicodeBidi::IsolateOverride, true) => ("\u{2067}\u{202E}", "\u{202C}\u{2069}"),
        (UnicodeBidi::Plaintext, _) => ("\u{2068}", "\u{2069}"),
    }
}

/// Whether the controls of a box can change the order of left-to-right text in a left-to-right paragraph (an embedded or
/// isolated left-to-right box cannot).
pub(super) fn reorders(ub: UnicodeBidi, rtl: bool) -> bool {
    match ub {
        UnicodeBidi::Normal => false,
        UnicodeBidi::Embed | UnicodeBidi::Isolate => rtl,
        _ => true,
    }
}

/// What one item of the paragraph is in the analysed string.
pub(super) enum Part<'a> {
    Text(&'a str),
    Ctl(&'a str),
    /// An atomic inline.
    Object,
    /// A forced line break.
    Break,
    Nothing,
}

/// The result for one paragraph.
pub(super) struct Para {
    /// The resolved level of every character of the analysed string.
    pub levels: Vec<u8>,
    /// For each item, the index of its first character in `levels`.
    pub start: Vec<usize>,
    /// For each item, the embedding level the item sits in (what edges and placeholders take).
    pub item_level: Vec<u8>,
    /// The paragraph level: 0 or 1.
    pub base: u8,
}

/// The first strong direction of `s`, ignoring what sits inside isolates (UAX #9 P2).
pub(super) fn first_strong(s: &str) -> Option<bool> {
    let mut depth = 0u32;
    for c in s.chars() {
        match c {
            LRI | RLI | FSI => depth += 1,
            PDI => depth = depth.saturating_sub(1),
            _ if depth == 0 => {
                if let Some(r) = strong_rtl(c) {
                    return Some(r);
                }
            }
            _ => {}
        }
    }
    None
}

/// Resolve the levels of the paragraph whose items are `parts`. `rtl` is the block's `direction`; with `plaintext` the first
/// strong character decides instead. `None`: the platform does no analysis (or answered with the wrong count).
pub(super) fn analyze(platform: &dyn Platform, parts: &[Part<'_>], rtl: bool, plaintext: bool) -> Option<Para> {
    let mut s = String::new();
    let mut start = Vec::with_capacity(parts.len());
    let mut n = 0usize;
    for p in parts {
        start.push(n);
        let t = match p {
            Part::Text(t) | Part::Ctl(t) => t,
            Part::Object => "\u{FFFC}",
            Part::Break => "\u{2028}",
            Part::Nothing => "",
        };
        s.push_str(t);
        n += t.chars().count();
    }
    let base_rtl = if plaintext { first_strong(&s).unwrap_or(rtl) } else { rtl };
    let levels = platform.bidi_levels(&s, base_rtl);
    if levels.len() != n {
        return None;
    }
    // The embedding level at each item: the stack of the formatting characters seen so far.
    let base = u8::from(base_rtl);
    let mut stack = vec![base];
    let mut item_level = Vec::with_capacity(parts.len());
    for (i, p) in parts.iter().enumerate() {
        // The formatting characters of the boxes, and those typed in the text, open and close embeddings the same way.
        if let Part::Ctl(t) | Part::Text(t) = p {
            for c in t.chars().filter(|&c| c >= '\u{202A}') {
                let top = stack.last().copied().unwrap_or(base);
                match c {
                    LRE | LRO | LRI => stack.push(next_level(top, false)),
                    RLE | RLO | RLI => stack.push(next_level(top, true)),
                    FSI => {
                        let rtl_inside = parts.get(i + 1..).and_then(|rest| rest.iter().find_map(|p| if let Part::Text(t) = p { first_strong(t) } else { None })).unwrap_or(false);
                        stack.push(next_level(top, rtl_inside));
                    }
                    PDF | PDI if stack.len() > 1 => {
                        stack.pop();
                    }
                    _ => {}
                }
            }
        }
        item_level.push(stack.last().copied().unwrap_or(base));
    }
    Some(Para { levels, start, item_level, base })
}

/// The least level above `level` with the direction asked for (X2-X5: next odd for right-to-left, next even otherwise).
fn next_level(level: u8, rtl: bool) -> u8 {
    let l = if rtl { (level + 1) | 1 } else { (level + 2) & !1 };
    l.min(MAX_DEPTH)
}

/// UAX #9 L2: the visual order of a line whose characters (here: units) have the levels `levels`. `order[i]` is the index
/// in `levels` of the item shown at place `i`, counted from the left. Left unchanged (the identity) when every level is 0.
pub(super) fn reorder(levels: &[u8], order: &mut Vec<usize>) {
    order.clear();
    order.extend(0..levels.len());
    let Some(&highest) = levels.iter().max() else { return };
    let Some(lowest_odd) = levels.iter().copied().filter(|l| l % 2 == 1).min() else { return };
    let mut level = highest;
    while level >= lowest_odd && level > 0 {
        let mut i = 0usize;
        while i < order.len() {
            if order.get(i).and_then(|&k| levels.get(k)).is_some_and(|&l| l >= level) {
                let mut j = i;
                while order.get(j).and_then(|&k| levels.get(k)).is_some_and(|&l| l >= level) {
                    j += 1;
                }
                if let Some(run) = order.get_mut(i..j) {
                    run.reverse();
                }
                i = j;
            } else {
                i += 1;
            }
        }
        level -= 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reorder_follows_l2() {
        let mut o = Vec::new();
        // "ab CD ef" with CD right to left at level 1.
        reorder(&[0, 0, 0, 1, 1, 0, 0, 0], &mut o);
        assert_eq!(o, vec![0, 1, 2, 4, 3, 5, 6, 7]);
        // A right-to-left paragraph with a left-to-right word inside: levels 1 1 1 2 2 1.
        reorder(&[1, 1, 1, 2, 2, 1], &mut o);
        assert_eq!(o, vec![5, 3, 4, 2, 1, 0]);
        reorder(&[0, 0], &mut o);
        assert_eq!(o, vec![0, 1]);
    }

    #[test]
    fn controls_open_and_close() {
        assert_eq!(controls(UnicodeBidi::IsolateOverride, true), ("\u{2067}\u{202E}", "\u{202C}\u{2069}"));
        assert_eq!(controls(UnicodeBidi::Normal, true), ("", ""));
        assert!(!reorders(UnicodeBidi::Isolate, false));
        assert!(reorders(UnicodeBidi::BidiOverride, false));
    }

    #[test]
    fn levels_climb_and_stop() {
        assert_eq!(next_level(0, true), 1);
        assert_eq!(next_level(1, false), 2);
        assert_eq!(next_level(1, true), 3);
        assert_eq!(next_level(124, true), 125);
        assert_eq!(next_level(125, true), 125);
    }
}
