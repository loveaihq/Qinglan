//! The test platform (L1 task 8): every glyph is a 1em square, ascent 0.8em, descent 0.2em (like the
//! WPT "Ahem" font), so layout snapshots do not depend on the fonts installed on the machine.

use crate::fmath::Fclamp;
use std::rc::Rc;

use super::{FontId, FontMetrics, FontQuery, Glyph, GlyphMask, Platform};
use crate::textclass::{is_cjk_break, is_closing_punct, is_opening_punct, is_space_like};

pub struct TestPlatform;

impl TestPlatform {
    pub const ASCENT: f32 = 0.8;
    pub const DESCENT: f32 = 0.2;
    /// Ahem's OS/2 x-height (the "x" glyph is as tall as the ascent).
    pub const X_HEIGHT: f32 = 0.8;
    /// Every glyph advances 1em, "0" included.
    pub const ZERO_ADVANCE: f32 = 1.0;
}

/// Characters that take no room: ZWSP, ZWNJ, ZWJ, word joiner, BOM, variation selectors,
/// combining marks and C0/C1 controls.
fn zero_width(c: char) -> bool {
    matches!(c as u32, 0x0000..=0x001F | 0x007F..=0x009F | 0x0300..=0x036F | 0x200B..=0x200F | 0x202A..=0x202E | 0x2060 | 0x2066..=0x2069 | 0xFE00..=0xFE0F | 0xFEFF)
}

impl Platform for TestPlatform {
    fn decode_legacy(&self, _encoding: &str, _bytes: &[u8]) -> Option<String> {
        None
    }

    fn resolve_font(&self, _query: &FontQuery<'_>) -> Option<FontId> {
        // One font answers every family, weight and style.
        Some(FontId(0))
    }

    fn font_metrics(&self, _font: FontId, size_px: f32) -> FontMetrics {
        FontMetrics {
            ascent: Self::ASCENT * size_px,
            descent: Self::DESCENT * size_px,
            line_gap: 0.0,
            x_height: Some(Self::X_HEIGHT * size_px),
            zero_advance: Some(Self::ZERO_ADVANCE * size_px),
            underline_offset: 0.1 * size_px,
            underline_thickness: 0.05 * size_px,
            strikeout_offset: 0.3 * size_px,
            strikeout_thickness: 0.05 * size_px,
        }
    }

    fn shape(&self, text: &str, _query: &FontQuery<'_>, size_px: f32) -> Vec<Glyph> {
        text.char_indices()
            .map(|(i, c)| Glyph {
                font: FontId(0),
                glyph: c as u32,
                advance: if zero_width(c) { 0.0 } else { size_px },
                offset_x: 0.0,
                offset_y: 0.0,
                cluster_start: i as u32,
                cluster_end: (i + c.len_utf8()) as u32,
            })
            .collect()
    }

    /// Simple rules: a break is allowed after a run of spaces, after U+200B, and between two CJK
    /// characters or between a CJK and a non-CJK character; never before a closing punctuation mark
    /// (the Chinese list in `textclass`) and never after an opening one. Hyphens do not break.
    fn line_breaks(&self, text: &str, _lang: Option<&str>) -> Vec<usize> {
        let mut out = Vec::new();
        let mut prev: Option<char> = None;
        for (i, c) in text.char_indices() {
            if let Some(p) = prev {
                let allowed = if is_closing_punct(c) || is_opening_punct(p) {
                    false
                } else if is_space_like(p) {
                    !is_space_like(c)
                } else if p == '\u{200B}' {
                    true
                } else if is_space_like(c) || c == '\u{200B}' {
                    false
                } else {
                    is_cjk_break(p) || is_cjk_break(c)
                };
                if allowed {
                    out.push(i);
                }
            }
            prev = Some(c);
        }
        out
    }

    fn bidi_levels(&self, text: &str, rtl: bool) -> Vec<u8> {
        let chars: Vec<char> = text.chars().collect();
        uba_impl::uba(&chars, u8::from(rtl))
    }

    fn glyph_mask(&self, _font: FontId, glyph: u32, size_px: f32, _subpixel: u8) -> Option<Rc<GlyphMask>> {
        let c = char::from_u32(glyph)?;
        if zero_width(c) || is_space_like(c) || c == '\u{00A0}' {
            return None;
        }
        let side = size_px.round().fclamp(1.0, 512.0) as u32;
        let ascent = (Self::ASCENT * size_px).round() as i32;
        // The private use characters U+E000..U+E0FF are colour glyphs (the display list keeps 16 bits of a glyph): the left half red, the right half blue, fully opaque.
        if (0xE000..=0xE0FF).contains(&glyph) {
            let mut bgra = vec![0u8; (side * side * 4) as usize];
            for (i, px) in bgra.as_chunks_mut::<4>().0.iter_mut().enumerate() {
                let x = i as u32 % side;
                px.copy_from_slice(if x < side / 2 { &[0, 0, 255, 255] } else { &[255, 0, 0, 255] });
            }
            return Some(Rc::new(GlyphMask { left: 0, top: -ascent, width: side, height: side, alpha: Vec::new(), bgra }));
        }
        Some(Rc::new(GlyphMask { left: 0, top: -ascent, width: side, height: side, alpha: vec![255; (side * side) as usize], bgra: Vec::new() }))
    }
}

// ---------------------------------------------------------------------------------------------
// A small implementation of the Unicode Bidirectional Algorithm (UAX #9) for the tests: the bidi classes come from a few
// ranges (Hebrew and Arabic letters, digits, the usual separators and terminators), brackets (N0) are not paired. It follows
// X1-X10, W1-W7, N1-N2 and I1-I2, so that explicit embeddings, isolates and overrides can be tested without real fonts.

#[allow(clippy::indexing_slicing, clippy::style, clippy::perf, clippy::complexity, clippy::restriction)]
mod uba_impl {
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Bc {
    L,
    R,
    AL,
    EN,
    ES,
    ET,
    AN,
    CS,
    NSM,
    BN,
    B,
    S,
    WS,
    ON,
    LRE,
    LRO,
    RLE,
    RLO,
    PDF,
    LRI,
    RLI,
    FSI,
    PDI,
}

fn class_of(c: char) -> Bc {
    use Bc::*;
    let u = c as u32;
    match u {
        0x202A => LRE,
        0x202D => LRO,
        0x202B => RLE,
        0x202E => RLO,
        0x202C => PDF,
        0x2066 => LRI,
        0x2067 => RLI,
        0x2068 => FSI,
        0x2069 => PDI,
        0x200E => L,
        0x200F => R,
        0x061C => AL,
        0x0A | 0x0D | 0x1C..=0x1E | 0x85 | 0x2029 => B,
        0x09 | 0x0B | 0x1F => S,
        0x0C | 0x20 | 0x1680 | 0x2000..=0x200A | 0x2028 | 0x205F | 0x3000 => WS,
        0x00..=0x08 | 0x0E..=0x1B | 0x7F..=0x84 | 0x86..=0x9F | 0xAD | 0x200B..=0x200D | 0x2060..=0x2064 | 0xFEFF => BN,
        0x30..=0x39 | 0x06F0..=0x06F9 => EN,
        0x0660..=0x0669 | 0x066B | 0x066C | 0x0600..=0x0605 | 0x06DD => AN,
        0x2B | 0x2D | 0x2212 => ES,
        0x23..=0x25 | 0xA2..=0xA5 | 0xB0 | 0xB1 | 0x066A | 0x2030..=0x2034 | 0x20A0..=0x20CF => ET,
        0x2C | 0x2E | 0x2F | 0x3A | 0xA0 | 0x060C | 0x202F => CS,
        0x0300..=0x036F | 0x0483..=0x0489 | 0x0591..=0x05BD | 0x05BF | 0x05C1 | 0x05C2 | 0x05C4 | 0x05C5 | 0x05C7 | 0x0610..=0x061A | 0x064B..=0x065F | 0x0670
        | 0x06D6..=0x06DC | 0x06DF..=0x06E4 | 0x06E7 | 0x06E8 | 0x06EA..=0x06ED => NSM,
        _ => match crate::textclass::strong_rtl(c) {
            Some(true) => {
                if matches!(u, 0x0600..=0x07BF | 0x0860..=0x08FF | 0xFB50..=0xFDFF | 0xFE70..=0xFEFF) {
                    AL
                } else {
                    R
                }
            }
            Some(false) => L,
            None => ON,
        },
    }
}

fn is_ni(t: Bc) -> bool {
    matches!(t, Bc::B | Bc::S | Bc::WS | Bc::ON | Bc::FSI | Bc::LRI | Bc::RLI | Bc::PDI)
}

fn is_removed(t: Bc) -> bool {
    matches!(t, Bc::LRE | Bc::RLE | Bc::LRO | Bc::RLO | Bc::PDF | Bc::BN)
}

fn next_lvl(l: u8, rtl: bool) -> u8 {
    if rtl { (l + 1) | 1 } else { (l + 2) & !1 }
}

fn dir_class(level: u8) -> Bc {
    if level % 2 == 1 { Bc::R } else { Bc::L }
}

/// The index of the PDI that matches each isolate initiator (BD9), if there is one.
fn match_isolates(cls: &[Bc]) -> Vec<Option<usize>> {
    let mut out = vec![None; cls.len()];
    let mut open: Vec<usize> = Vec::new();
    for (i, &t) in cls.iter().enumerate() {
        match t {
            Bc::LRI | Bc::RLI | Bc::FSI => open.push(i),
            Bc::PDI => {
                if let Some(o) = open.pop() {
                    out[o] = Some(i);
                }
            }
            _ => {}
        }
    }
    out
}

pub(super) fn uba(text: &[char], base: u8) -> Vec<u8> {
    use Bc::*;
    let n = text.len();
    let cls: Vec<Bc> = text.iter().map(|&c| class_of(c)).collect();
    let mut types = cls.clone();
    let mut levels = vec![base; n];
    let matching = match_isolates(&cls);
    // X1-X8.
    struct St {
        level: u8,
        ovr: Option<Bc>,
        iso: bool,
    }
    let mut stack = vec![St { level: base, ovr: None, iso: false }];
    let (mut over_iso, mut over_emb, mut valid_iso) = (0u32, 0u32, 0u32);
    for i in 0..n {
        let (top_level, top_ovr) = stack.last().map_or((base, None), |s| (s.level, s.ovr));
        match cls[i] {
            RLE | LRE | RLO | LRO => {
                levels[i] = top_level;
                let rtl = matches!(cls[i], RLE | RLO);
                let new = next_lvl(top_level, rtl);
                if new <= 125 && over_iso == 0 && over_emb == 0 {
                    let ovr = match cls[i] {
                        RLO => Some(R),
                        LRO => Some(L),
                        _ => None,
                    };
                    stack.push(St { level: new, ovr, iso: false });
                } else if over_iso == 0 {
                    over_emb += 1;
                }
            }
            RLI | LRI | FSI => {
                levels[i] = top_level;
                if let Some(o) = top_ovr {
                    types[i] = o;
                }
                let rtl = match cls[i] {
                    RLI => true,
                    LRI => false,
                    _ => {
                        let end = matching[i].unwrap_or(n);
                        text[i + 1..end].iter().find_map(|&c| crate::textclass::strong_rtl(c)).unwrap_or(false)
                    }
                };
                let new = next_lvl(top_level, rtl);
                if new <= 125 && over_iso == 0 && over_emb == 0 {
                    valid_iso += 1;
                    stack.push(St { level: new, ovr: None, iso: true });
                } else {
                    over_iso += 1;
                }
            }
            PDI => {
                if over_iso > 0 {
                    over_iso -= 1;
                } else if valid_iso > 0 {
                    over_emb = 0;
                    while stack.last().is_some_and(|s| !s.iso) {
                        stack.pop();
                    }
                    stack.pop();
                    valid_iso -= 1;
                }
                let (l, o) = stack.last().map_or((base, None), |s| (s.level, s.ovr));
                levels[i] = l;
                if let Some(o) = o {
                    types[i] = o;
                }
            }
            PDF => {
                levels[i] = top_level;
                if over_iso == 0 {
                    if over_emb > 0 {
                        over_emb -= 1;
                    } else if stack.last().is_some_and(|s| !s.iso) && stack.len() >= 2 {
                        stack.pop();
                    }
                }
            }
            B => levels[i] = base,
            BN => levels[i] = top_level,
            _ => {
                levels[i] = top_level;
                if let Some(o) = top_ovr {
                    types[i] = o;
                }
            }
        }
    }
    // X9: the characters left out; X10: level runs and isolating run sequences over the rest.
    let kept: Vec<usize> = (0..n).filter(|&i| !is_removed(cls[i])).collect();
    let mut runs: Vec<Vec<usize>> = Vec::new();
    for &i in &kept {
        match runs.last_mut() {
            Some(r) if r.last().is_some_and(|&j| levels[j] == levels[i]) => r.push(i),
            _ => runs.push(vec![i]),
        }
    }
    let run_of = |ch: usize| runs.iter().position(|r| r.first() == Some(&ch));
    let mut seqs: Vec<Vec<usize>> = Vec::new();
    for r in &runs {
        let first = r[0];
        let continues = cls[first] == PDI && matching.iter().any(|m| *m == Some(first));
        if continues {
            continue;
        }
        let mut seq = r.clone();
        loop {
            let last = *seq.last().unwrap_or(&first);
            let Some(Some(pdi)) = matching.get(last).copied() else { break };
            let Some(ri) = run_of(pdi) else { break };
            seq.extend(runs[ri].iter().copied());
        }
        seqs.push(seq);
    }
    for seq in &seqs {
        let first = seq[0];
        let last = *seq.last().unwrap_or(&first);
        let level = levels[first];
        let prev_level = kept.iter().rev().find(|&&k| k < first).map_or(base, |&k| levels[k]);
        let sos = dir_class(level.max(prev_level));
        let next_level = if matches!(cls[last], LRI | RLI | FSI) && matching[last].is_none() {
            base
        } else {
            kept.iter().find(|&&k| k > last).map_or(base, |&k| levels[k])
        };
        let eos = dir_class(level.max(next_level));
        let mut t: Vec<Bc> = seq.iter().map(|&i| types[i]).collect();
        let m = t.len();
        // W1.
        for k in 0..m {
            if t[k] == NSM {
                t[k] = if k == 0 {
                    sos
                } else if matches!(t[k - 1], LRI | RLI | FSI | PDI) {
                    ON
                } else {
                    t[k - 1]
                };
            }
        }
        // W2, W3.
        let mut strong = sos;
        for k in 0..m {
            match t[k] {
                L | R | AL => strong = t[k],
                EN if strong == AL => t[k] = AN,
                _ => {}
            }
        }
        for x in t.iter_mut() {
            if *x == AL {
                *x = R;
            }
        }
        // W4.
        for k in 1..m.saturating_sub(1) {
            if t[k] == ES && t[k - 1] == EN && t[k + 1] == EN {
                t[k] = EN;
            } else if t[k] == CS && t[k - 1] == t[k + 1] && matches!(t[k - 1], EN | AN) {
                t[k] = t[k - 1];
            }
        }
        // W5.
        let mut k = 0;
        while k < m {
            if t[k] == ET {
                let mut e = k;
                while e < m && t[e] == ET {
                    e += 1;
                }
                if (k > 0 && t[k - 1] == EN) || (e < m && t[e] == EN) {
                    for x in &mut t[k..e] {
                        *x = EN;
                    }
                }
                k = e;
            } else {
                k += 1;
            }
        }
        // W6, W7.
        for x in t.iter_mut() {
            if matches!(*x, ES | ET | CS) {
                *x = ON;
            }
        }
        let mut strong = sos;
        for x in t.iter_mut() {
            match *x {
                L | R => strong = *x,
                EN if strong == L => *x = L,
                _ => {}
            }
        }
        // N1, N2.
        let mut k = 0;
        while k < m {
            if is_ni(t[k]) {
                let mut e = k;
                while e < m && is_ni(t[e]) {
                    e += 1;
                }
                let as_dir = |c: Bc| if matches!(c, EN | AN) { R } else { c };
                let before = if k == 0 { sos } else { as_dir(t[k - 1]) };
                let after = if e == m { eos } else { as_dir(t[e]) };
                let d = if before == after { before } else { dir_class(level) };
                for x in &mut t[k..e] {
                    *x = d;
                }
                k = e;
            } else {
                k += 1;
            }
        }
        // I1, I2.
        for (k, &i) in seq.iter().enumerate() {
            levels[i] = match (level % 2 == 0, t[k]) {
                (true, R) => level + 1,
                (true, AN | EN) => level + 2,
                (false, L | EN | AN) => level + 1,
                _ => level,
            };
        }
    }
    // The characters X9 left out take the level of the one before them (the paragraph level at the start).
    let mut prev = base;
    for i in 0..n {
        if is_removed(cls[i]) {
            levels[i] = prev;
        } else {
            prev = levels[i];
        }
    }
    levels
}
}
