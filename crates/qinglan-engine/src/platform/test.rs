//! The test platform (L1 task 8): every glyph is a 1em square, ascent 0.8em, descent 0.2em (like the
//! WPT "Ahem" font), so layout snapshots do not depend on the fonts installed on the machine.

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
    matches!(c as u32, 0x0000..=0x001F | 0x007F..=0x009F | 0x0300..=0x036F | 0x200B..=0x200F | 0x2060 | 0xFE00..=0xFE0F | 0xFEFF)
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

    fn glyph_mask(&self, _font: FontId, glyph: u32, size_px: f32, _subpixel: u8) -> Option<GlyphMask> {
        let c = char::from_u32(glyph)?;
        if zero_width(c) || is_space_like(c) || c == '\u{00A0}' {
            return None;
        }
        let side = size_px.round().clamp(1.0, 512.0) as u32;
        let ascent = (Self::ASCENT * size_px).round() as i32;
        Some(GlyphMask { left: 0, top: -ascent, width: side, height: side, alpha: vec![255; (side * side) as usize] })
    }
}
