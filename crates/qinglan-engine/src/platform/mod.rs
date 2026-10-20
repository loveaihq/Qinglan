//! The platform interface: everything the engine needs from the operating system.
//!
//! Designed once for everything steps 1b and 1c need from fonts and text (L1 task 8). The engine
//! never calls Windows directly (CLAUDE.md "Technical decisions"): the Windows platform
//! (DirectWrite, `MultiByteToWideChar`) arrives in step 1b-3 and implements this same trait; the
//! engine's own tests use [`TestPlatform`].
//!
//! The trait is object-safe (`&dyn Platform` everywhere) and mentions no Windows types. Sizes are
//! CSS px as `f32`; the engine converts them to its 1/64 px layout units itself.
//! Images ([`image::ImageDecoder`]) and the network ([`crate::net`]) are separate, thread-safe interfaces added in 1c.

pub mod image;
pub mod test;

pub use image::{DecodedCache, DecodedImage, ImageDecoder, ImageError, ImageInfo, NoImages, is_truncated};
pub use test::TestPlatform;

use std::rc::Rc;

use crate::css::types::FontFamily;

/// A font face chosen by the platform (family, weight and style already resolved, synthetic
/// bold/italic included). Only meaningful for the platform that returned it.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct FontId(pub u32);

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum FontSlant {
    Upright,
    Italic,
    Oblique,
}

/// What the page asks of a font (css-fonts-4 "font matching"): the `font-family` list in order,
/// the weight (1..=1000), the slant and the language of the text (the `lang` attribute, so that a
/// Chinese, Japanese or Korean fallback font picks the right glyph shapes).
#[derive(Clone, Copy, Debug)]
pub struct FontQuery<'a> {
    pub families: &'a [FontFamily],
    pub weight: f32,
    pub slant: FontSlant,
    pub lang: Option<&'a str>,
}

/// Metrics of one font at one size, all in px. `ascent` and `descent` are positive distances above
/// and below the baseline. The `*_offset` fields are measured from the baseline to the top edge of
/// the line: down for the underline, up for the strike-through.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FontMetrics {
    pub ascent: f32,
    pub descent: f32,
    /// Extra leading between lines (`line-height: normal` = ascent + descent + line_gap).
    pub line_gap: f32,
    /// Height of a lowercase "x"; `None` when the font does not say (`ex` is then 0.5em).
    pub x_height: Option<f32>,
    /// Advance of "0"; `None` when the font has no such glyph (`ch` is then 0.5em).
    pub zero_advance: Option<f32>,
    pub underline_offset: f32,
    pub underline_thickness: f32,
    pub strikeout_offset: f32,
    pub strikeout_thickness: f32,
}

/// One shaped glyph. Glyphs come in logical (reading) order. `cluster_start..cluster_end` is the
/// byte range of the shaped text this glyph stands for; several glyphs may share a range (one
/// character drawn with several glyphs) and one glyph may cover several characters (a ligature).
/// The ranges of consecutive glyphs never go backwards. Font fallback happens during shaping, so
/// `font` can differ between glyphs of one run.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Glyph {
    pub font: FontId,
    /// Glyph index in `font`; 0 is the font's missing-glyph box (".notdef").
    pub glyph: u32,
    /// Horizontal advance, px.
    pub advance: f32,
    /// Offset of the drawn glyph from its pen position (marks, kerning), px, y down.
    pub offset_x: f32,
    pub offset_y: f32,
    pub cluster_start: u32,
    pub cluster_end: u32,
}

/// A rendered glyph: 8-bit coverage, row by row, top to bottom.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GlyphMask {
    /// Offset of the mask's left edge from the pen position, px (right is positive).
    pub left: i32,
    /// Offset of the mask's top edge from the baseline, px (down is positive, so usually negative).
    pub top: i32,
    pub width: u32,
    pub height: u32,
    pub alpha: Vec<u8>,
    /// A colour glyph (an emoji drawn from COLR layers, say): premultiplied BGRA pixels, 4 bytes each, row by row, the layers
    /// already composed. Then `alpha` is empty and the text colour plays no part. Empty for an ordinary coverage mask.
    pub bgra: Vec<u8>,
}

/// What the platform does with text in a web face that cannot be drawn yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WebFaceState {
    /// Not usable (not loaded, failed): text goes on to the next family of the list.
    Fallback,
    /// Loading, inside the block period of `font-display`: text in the face's `unicode-range` takes the room of the
    /// next family's font but is not drawn.
    Hidden,
}

/// Services the engine asks of the host.
pub trait Platform {
    /// Decode `bytes` from the legacy encoding `encoding`, a WHATWG encoding name such as
    /// "GBK", "Big5" or "Shift_JIS" (Encoding Standard 4.2). Bad bytes become U+FFFD.
    /// Returns `None` when the platform has no decoder for that encoding.
    fn decode_legacy(&self, encoding: &str, bytes: &[u8]) -> Option<String>;

    /// The primary font for a query: the first family of the list that exists and has a space, or the platform's
    /// default font when none does. `None` only when the platform has no fonts at all.
    fn resolve_font(&self, query: &FontQuery<'_>) -> Option<FontId>;

    /// Metrics of `font` at `size_px`.
    fn font_metrics(&self, font: FontId, size_px: f32) -> FontMetrics;

    /// Shape `text` (one run of one style) with the font(s) the query selects, falling back to
    /// other fonts for characters the primary font lacks. No letter- or word-spacing is applied.
    fn shape(&self, text: &str, query: &FontQuery<'_>, size_px: f32) -> Vec<Glyph>;

    /// A hint before layout: `chars` are the distinct non-ASCII characters of a page that will be
    /// shaped with `query`, and `words` its distinct ASCII words (maximal runs of printable ASCII
    /// without space), both in order of first appearance. A platform may shape them in bulk now, which
    /// is cheaper than one call per character or word later. It must not change what `shape` returns;
    /// the default does nothing.
    fn prefetch(&self, _chars: &str, _words: &[&str], _query: &FontQuery<'_>) {}

    /// The `@font-face` faces of the page being shown, in order of declaration (replacing the earlier ones, whose fonts
    /// are freed; an empty list is how a page is left). All start as [`WebFaceState::Fallback`]. A family with faces
    /// shadows an installed font of the same name. The default does nothing (no web fonts).
    fn set_web_faces(&self, _faces: &[crate::fonts::face::FaceDesc]) {}

    fn set_web_face_state(&self, _index: usize, _state: WebFaceState) {}

    /// Use the decoded font file `sfnt` (TrueType or OpenType) for face `index`. False when the platform cannot use
    /// it: the face stays as it was.
    fn load_web_face(&self, _index: usize, _sfnt: &[u8]) -> bool {
        false
    }

    /// Use the installed font `name` for face `index` (`src: local(...)`). False when there is none.
    fn use_local_face(&self, _index: usize, _name: &str) -> bool {
        false
    }

    /// UAX #9: the resolved embedding level of every character of `text`, one bidi paragraph in which the explicit formatting
    /// characters (U+202A..U+202E, U+2066..U+2069) are present as characters. `rtl` is the paragraph level (right to left or
    /// not). Empty when the platform does no bidi analysis: then everything is left to right.
    fn bidi_levels(&self, _text: &str, _rtl: bool) -> Vec<u8> {
        Vec::new()
    }

    /// `shape` for upright text in vertical writing (css-writing-modes-3 5.1): the vertical forms of the glyphs (OpenType `vert`)
    /// are chosen. The default shapes it as horizontal text.
    fn shape_upright(&self, text: &str, query: &FontQuery<'_>, size_px: f32) -> Vec<Glyph> {
        self.shape(text, query, size_px)
    }

    /// `shape` for text that runs right to left (an odd bidi level): joining and mark placement follow that direction and
    /// mirrored characters get their mirror image. The glyphs come in logical order of the clusters, as for `shape`, but the
    /// glyphs inside one cluster are in left-to-right drawing order. The default shapes as left to right.
    fn shape_rtl(&self, text: &str, query: &FontQuery<'_>, size_px: f32) -> Vec<Glyph> {
        self.shape(text, query, size_px)
    }

    /// Soft line-break opportunities of a whole paragraph (UAX #14): ascending byte offsets
    /// `o` (0 < o < `text.len()`) such that a line may end before `text[o..]`. No CSS is applied
    /// here: the engine adds `word-break`, `overflow-wrap` and `white-space` on top.
    fn line_breaks(&self, text: &str, lang: Option<&str>) -> Vec<usize>;

    /// Render one glyph as a coverage mask. `subpixel` (0..=3) is the horizontal position of the
    /// pen in quarter pixels. `None` for glyphs that draw nothing (spaces).
    /// The mask is shared with the platform's cache, so drawing a glyph twice does not copy it.
    fn glyph_mask(&self, font: FontId, glyph: u32, size_px: f32, subpixel: u8) -> Option<Rc<GlyphMask>>;
}

/// A platform with no legacy-encoding support and no fonts: only what the engine decodes itself
/// (UTF-8, UTF-16, windows-1252, x-user-defined). Used by the 1a tests and tools.
pub struct BasicPlatform;

impl Platform for BasicPlatform {
    fn decode_legacy(&self, _encoding: &str, _bytes: &[u8]) -> Option<String> {
        None
    }
    fn resolve_font(&self, _query: &FontQuery<'_>) -> Option<FontId> {
        None
    }
    fn font_metrics(&self, _font: FontId, _size_px: f32) -> FontMetrics {
        FontMetrics::default()
    }
    fn shape(&self, _text: &str, _query: &FontQuery<'_>, _size_px: f32) -> Vec<Glyph> {
        Vec::new()
    }
    fn line_breaks(&self, _text: &str, _lang: Option<&str>) -> Vec<usize> {
        Vec::new()
    }
    fn glyph_mask(&self, _font: FontId, _glyph: u32, _size_px: f32, _subpixel: u8) -> Option<Rc<GlyphMask>> {
        None
    }
}
