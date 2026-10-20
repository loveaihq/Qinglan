//! The Windows implementation of the engine's `Platform` trait (L1 task 8, "Windows 平台").
//!
//! Fonts come from the DirectWrite system collection; characters a font lacks are looked up
//! through `IDWriteFontFallback::MapCharacters` with the element's language; text is shaped with
//! `IDWriteTextAnalyzer`; masks come from `IDWriteGlyphRunAnalysis`. See docs/decisions.md
//! ("1b-3") for the choices made where the task sheet leaves room.

use std::cell::{Cell, RefCell};
use std::time::Instant;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use qinglan_engine::css::types::{FontFamily, GenericFamily};
use qinglan_engine::fonts::face::{FaceDesc, rank_faces};
use qinglan_engine::platform::{FontId, FontMetrics, FontQuery, FontSlant, Glyph, GlyphMask, Platform, WebFaceState};

use crate::fx::{FxHasher, FxMap};
use crate::lru::Lru;
use crate::sys;
use crate::sys::dwrite::{DesignMetrics, Face, Factory, Font, ScriptAnalysis, Shaped, WebFontLoader, break_between};

/// Glyph mask cache budget (L1 task 8: LRU, at most 8 MB).
const GLYPH_CACHE_BYTES: usize = 3 << 19;
/// Shaping cache budget. Counted by text bytes plus 28 bytes per glyph plus a fixed overhead.
const SHAPE_CACHE_BYTES: usize = 1 << 20;
/// Most fonts (faces) the platform keeps; more are refused and the text falls back to the primary font.
const MAX_FONTS: usize = 512;
/// Most distinct font queries kept (family list, weight, slant, language).
const MAX_QUERIES: usize = 8192;
/// A single `GetGlyphs` call gets at most this many UTF-16 units (its cluster map holds 16-bit values).
const MAX_RUN_UNITS: usize = 8192;

/// Font ids travel as 16 bits through layout and the display list (`PlacedGlyph::font`): system fonts are below
/// `WEB_BASE`, web fonts from `WEB_BASE` to `HIDDEN`, and `HIDDEN` is set in the id of a font whose text is laid out but not drawn (a web font still loading in its block period).
const HIDDEN: u32 = 0x8000;
/// Ids of web fonts start here; they live in their own table so that they can be dropped with the page.
const WEB_BASE: u32 = 0x4000;

/// Size at which context-free characters are shaped (advances are scaled linearly afterwards).
const PROBE: f32 = 100.0;
/// Most UTF-16 units of words shaped by one bulk call.
const BATCH_UNITS: usize = 6000;
/// Separates the words of a bulk call: a private-use character, which fonts map to the missing-glyph box, so
/// that no kerning or ligature rule of the font reaches across it.
const WORD_SEP: u16 = 0xE000;
/// Most characters shaped by one bulk call: each takes up to 2 UTF-16 units plus a separator.
const BATCH_CHARS: usize = 2048;
/// Size of one cached glyph, for the shaping cache's byte count.
const GLYPH_COST: usize = std::mem::size_of::<Glyph>();

/// Counters for tests and the `--time` report.
#[derive(Clone, Copy, Debug, Default)]
pub struct CacheStats {
    pub shape_hits: u64,
    pub shape_misses: u64,
    pub shape_entries: usize,
    pub shape_bytes: usize,
    pub glyph_hits: u64,
    pub glyph_misses: u64,
    pub glyph_entries: usize,
    pub glyph_bytes: usize,
    /// Calls to `MapCharacters` (system font fallback).
    pub fallback_calls: u64,
    pub fonts: usize,
    /// Time spent on cache misses, in microseconds: choosing fonts, `AnalyzeScript`, shaping.
    pub us_choose: u64,
    pub us_analyze: u64,
    pub us_shape: u64,
    /// Calls to `AnalyzeScript` and to `GetGlyphs` + `GetGlyphPlacements`.
    pub analyze_calls: u64,
    pub shape_calls: u64,
    /// Bulk calls for context-free characters (`prefetch`), the characters they covered, their time;
    /// and the time of the one-character calls that remain.
    pub batch_calls: u64,
    pub batch_chars: u64,
    pub us_batch: u64,
    pub us_single: u64,
    /// Words (text between spaces, and the spaces) found in / added to the word cache.
    pub word_hits: u64,
    pub word_misses: u64,
    /// Words put into the word cache by `prefetch`.
    pub word_prefetched: u64,
    /// Shaping calls over 300 us and their time.
    pub slow_calls: u64,
    pub us_slow: u64,
    /// Time in `MapCharacters`, in creating font faces, and in `AnalyzeLineBreakpoints`; calls to the last.
    pub us_map: u64,
    pub us_face: u64,
    pub us_breaks: u64,
    pub break_calls: u64,
    /// Time in `Platform::shape` for texts that were not in the cache (everything above except line breaks, plus the glue).
    pub us_miss: u64,
}

impl CacheStats {
    /// One line per group for `qinglan-cli layout --time`.
    pub fn report(&self) -> String {
        let ms = |us: u64| us as f64 / 1000.0;
        let mut o = String::new();
        o.push_str(&format!("text: {} texts shaped ({} cached), {:.1} ms in total
", self.shape_misses, self.shape_hits, ms(self.us_miss)));
        o.push_str(&format!("  GetGlyphs {} calls {:.1} ms (bulk CJK {} calls for {} chars {:.1} ms; single CJK {:.1} ms)
", self.shape_calls, ms(self.us_shape), self.batch_calls, self.batch_chars, ms(self.us_batch), ms(self.us_single)));
        o.push_str(&format!("  AnalyzeScript {} calls {:.1} ms; choosing fonts {:.1} ms (MapCharacters {} calls {:.1} ms); font faces {} {:.1} ms
", self.analyze_calls, ms(self.us_analyze), ms(self.us_choose), self.fallback_calls, ms(self.us_map), self.fonts, ms(self.us_face)));
        o.push_str(&format!("  words {} cached, {} shaped on demand, {} shaped in bulk\n", self.word_hits, self.word_misses, self.word_prefetched));
        o.push_str(&format!("  shaping calls over 300 us: {} taking {:.1} ms
", self.slow_calls, ms(self.us_slow)));
        o.push_str(&format!("breaks: {} calls {:.1} ms", self.break_calls, ms(self.us_breaks)));
        o
    }
}

struct FontEntry {
    face: Face,
    /// The system font this face was made from (none for a web font).
    font: Option<Font>,
    m: DesignMetrics,
    /// Advance of "0" in design units.
    zero_advance: Option<f32>,
    /// Code point to glyph index (0 = the font has none).
    cmap: RefCell<FxMap<u32, u16>>,
    /// The `unicode-range` of a web font; it draws nothing outside it.
    ranges: Option<Rc<[(u32, u32)]>>,
    /// Colour glyphs (COLR layers): -1 once one was drawn, else the number of glyphs tried that had none (after 64 the font is
    /// taken for a plain one and not asked again).
    color_state: Cell<i32>,
}

#[derive(Clone, Copy)]
enum WState {
    Fallback,
    Hidden,
    /// Loaded: the id of its font (in `Inner::web_fonts`).
    Ready(u32),
    /// `src: local()`: the system font family with this collection index.
    Local(u32),
}

struct WebFace {
    desc: FaceDesc,
    state: WState,
}

impl FontEntry {
    fn in_ranges(&self, cp: u32) -> bool {
        self.ranges.as_ref().is_none_or(|r| r.iter().any(|&(a, b)| (a..=b).contains(&cp)))
    }

    fn glyph_of(&self, cp: u32) -> u16 {
        if let Some(&g) = self.cmap.borrow().get(&cp) {
            return g;
        }
        let g = self.face.glyph_indices(&[cp]).and_then(|v| v.first().copied()).unwrap_or(0);
        self.cmap.borrow_mut().insert(cp, g);
        g
    }
}

struct QueryInfo {
    families: Vec<FontFamily>,
    weight_bits: u32,
    slant: FontSlant,
    lang: Option<String>,
    /// The fonts of the `font-family` list that exist, in order (or the default font).
    chain: Vec<u32>,
    /// The whole family list in order when some web face of it is hidden (`HIDDEN | face index` marks it); else empty.
    items: Vec<u32>,
    /// The primary font (css-fonts-4 5.1 "first available font"): the first of the chain that has a space.
    primary: u32,
    /// Normalized locale, NUL-terminated UTF-16, and a hash of it.
    locale: Vec<u16>,
    loc: u32,
    /// Family name of the primary font, NUL-terminated UTF-16 (the base for `MapCharacters`).
    base_family: Vec<u16>,
    weight: u32,
    style: u32,
}

impl QueryInfo {
    fn matches(&self, q: &FontQuery<'_>) -> bool {
        self.families.as_slice() == q.families && self.weight_bits == q.weight.to_bits() && self.slant == q.slant && self.lang.as_deref() == q.lang
    }
}

struct ShapeEntry {
    qi: u32,
    size_bits: u32,
    text: Box<str>,
    glyphs: Rc<[Glyph]>,
}

struct Inner {
    fonts: Vec<Rc<FontEntry>>,
    by_face: FxMap<usize, u32>,
    /// Lower-case family name to collection index.
    families: FxMap<String, Option<u32>>,
    /// (collection index, weight, style) to font.
    fam_fonts: FxMap<(u32, u32, u8), Option<u32>>,
    queries: Vec<Rc<QueryInfo>>,
    query_map: FxMap<u64, u32>,
    /// Per query: fallback fonts already chosen by the system, tried before asking it again.
    sticky: Vec<Vec<u32>>,
    /// (query, code point) to the font that draws it.
    choice: FxMap<u64, u32>,
    shape_cache: Lru<u64, ShapeEntry>,
    glyph_cache: Lru<(u32, u32, u32, u8), Option<Rc<GlyphMask>>>,
    default_locale: String,
    stats: CacheStats,
    /// (font, locale hash, code point) to the glyph and advance of one context-free character.
    simple: FxMap<(u32, u32, u32), (u16, f32)>,
    /// Shaped words (see `shape_words`).
    words: FxMap<u64, WordEntry>,
    /// The `@font-face` faces of the page, in order of declaration, and the faces of each family (lower-case name).
    web: Vec<WebFace>,
    web_by_family: FxMap<String, Vec<usize>>,
    web_fonts: FxMap<u32, Rc<FontEntry>>,
    next_web_id: u32,
    web_loader: Option<WebFontLoader>,
    /// Shaping right to left now (set around `shape_uncached`).
    rtl: bool,
    /// The emoji font (Segoe UI Emoji): not looked for yet / not installed / its id.
    emoji: Option<Option<u32>>,
}

/// A word (or one space) shaped at `PROBE` size; advances and offsets are scaled to the real size when used.
struct WordEntry {
    fid: u32,
    loc: u32,
    script: u16,
    shapes: u32,
    units: Box<[u16]>,
    shaped: Rc<Shaped>,
}

/// DirectWrite-backed platform. Not `Send`: use it from the thread that created it.
pub struct WinPlatform {
    dw: Factory,
    inner: RefCell<Inner>,
}

impl WinPlatform {
    /// `None` when DirectWrite (Windows 8.1 or later) is not available.
    pub fn new() -> Option<WinPlatform> {
        let dw = Factory::new()?;
        let inner = Inner {
            fonts: Vec::new(),
            by_face: FxMap::default(),
            families: FxMap::default(),
            fam_fonts: FxMap::default(),
            queries: Vec::new(),
            query_map: FxMap::default(),
            sticky: Vec::new(),
            choice: FxMap::default(),
            shape_cache: Lru::new(SHAPE_CACHE_BYTES),
            glyph_cache: Lru::new(GLYPH_CACHE_BYTES),
            default_locale: sys::user_locale_name(),
            stats: CacheStats::default(),
            simple: FxMap::default(),
            words: FxMap::default(),
            web: Vec::new(),
            web_by_family: FxMap::default(),
            web_fonts: FxMap::default(),
            next_web_id: 0,
            web_loader: None,
            rtl: false,
            emoji: None,
        };
        Some(WinPlatform { dw, inner: RefCell::new(inner) })
    }

    /// Cache and fallback counters.
    pub fn cache_stats(&self) -> CacheStats {
        let inn = self.inner.borrow();
        CacheStats {
            shape_entries: inn.shape_cache.len(),
            shape_bytes: inn.shape_cache.bytes(),
            glyph_entries: inn.glyph_cache.len(),
            glyph_bytes: inn.glyph_cache.bytes(),
            fonts: inn.fonts.len(),
            ..inn.stats
        }
    }

    /// Name (en-US) of the family of `font`, for tests and diagnostics.
    pub fn family_name(&self, font: FontId) -> Option<String> {
        let e = self.inner.borrow().entry(font.0).cloned()?;
        e.font.as_ref()?.family_name()
    }

    /// `DWRITE_FONT_SIMULATIONS` of `font`: 1 = synthetic bold, 2 = synthetic oblique.
    pub fn simulations(&self, font: FontId) -> u32 {
        self.inner.borrow().entry(font.0).map_or(0, |e| e.font.as_ref().map_or(0, |f| f.simulations()))
    }

    /// Empty the glyph mask cache (the memory breakdown of the benches).
    pub fn clear_glyph_cache(&self) {
        self.inner.borrow_mut().glyph_cache = Lru::new(GLYPH_CACHE_BYTES);
    }

    /// Empty the word and single-character shaping tables and the font choices (the memory breakdown of the
    /// benches).
    pub fn clear_word_caches(&self) {
        let mut inn = self.inner.borrow_mut();
        inn.words = FxMap::default();
        inn.simple = FxMap::default();
        inn.choice = FxMap::default();
    }

    /// Empty the shaping cache (tests).
    pub fn clear_shape_cache(&self) {
        self.inner.borrow_mut().shape_cache = Lru::new(SHAPE_CACHE_BYTES);
    }
}

fn wide_nul(s: &str) -> Vec<u16> {
    let mut v: Vec<u16> = s.encode_utf16().collect();
    v.push(0);
    v
}

/// BCP 47 language tag from a `lang` attribute to the name DirectWrite's fallback knows:
/// Simplified and Traditional Chinese are told apart (`zh-Hans` / `zh-Hant`, by script or region),
/// Japanese and Korean stay `ja` / `ko`, anything else keeps its language and region.
fn normalize_lang(lang: &str) -> Option<String> {
    let l: String = lang.trim().chars().take(35).map(|c| if c == '_' { '-' } else { c.to_ascii_lowercase() }).collect();
    if l.is_empty() || !l.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return None;
    }
    let parts: Vec<&str> = l.split('-').collect();
    let primary = *parts.first()?;
    if !(2..=3).contains(&primary.len()) || !primary.chars().all(|c| c.is_ascii_alphabetic()) {
        return None;
    }
    let rest = || parts.iter().skip(1);
    match primary {
        "zh" => {
            let hant = rest().any(|p| matches!(*p, "hant" | "tw" | "hk" | "mo"));
            let hans = rest().any(|p| matches!(*p, "hans" | "cn" | "sg"));
            Some(if hant && !hans { "zh-Hant" } else { "zh-Hans" }.to_string())
        }
        "ja" | "ko" => Some(primary.to_string()),
        _ => match parts.get(1) {
            Some(r) if r.len() == 2 && r.chars().all(|c| c.is_ascii_alphabetic()) => Some(format!("{primary}-{}", r.to_ascii_uppercase())),
            _ => Some(primary.to_string()),
        },
    }
}

/// Generic family to the Windows fonts that stand for it (L1 task 8), first existing one wins.
fn generic_names(g: GenericFamily) -> &'static [&'static str] {
    match g {
        GenericFamily::Serif | GenericFamily::UiSerif => &["Times New Roman"],
        GenericFamily::SansSerif | GenericFamily::SystemUi | GenericFamily::UiSansSerif | GenericFamily::UiRounded => &["Segoe UI"],
        GenericFamily::Monospace | GenericFamily::UiMonospace => &["Consolas"],
        GenericFamily::Cursive => &["Comic Sans MS", "Segoe Script"],
        GenericFamily::Fantasy => &["Impact"],
        GenericFamily::Emoji => &["Segoe UI Emoji"],
        GenericFamily::Math => &["Cambria Math"],
        GenericFamily::Fangsong => &["FangSong", "SimSun"],
    }
}

/// English names of common Chinese font names, tried when `FindFamilyName` does not know the
/// Chinese one (it depends on the localized names the machine's fonts carry).
fn alias(name: &str) -> Option<&'static str> {
    Some(match name {
        "微软雅黑" => "Microsoft YaHei",
        "微软正黑体" => "Microsoft JhengHei",
        "宋体" => "SimSun",
        "新宋体" => "NSimSun",
        "黑体" => "SimHei",
        "楷体" | "楷体_GB2312" => "KaiTi",
        "仿宋" | "仿宋_GB2312" => "FangSong",
        "隶书" => "LiSu",
        "幼圆" => "YouYuan",
        "细明体" => "MingLiU",
        "新细明体" => "PMingLiU",
        "标楷体" => "DFKai-SB",
        "微軟正黑體" => "Microsoft JhengHei",
        "メイリオ" => "Meiryo",
        "ＭＳ ゴシック" => "MS Gothic",
        "ＭＳ Ｐゴシック" => "MS PGothic",
        "맑은 고딕" => "Malgun Gothic",
        _ => return None,
    })
}

fn query_hash(q: &FontQuery<'_>) -> u64 {
    let mut h = FxHasher::default();
    for f in q.families {
        match f {
            FontFamily::Named(n) => {
                h.write_u8(1);
                h.write(n.as_bytes());
            }
            FontFamily::Generic(g) => {
                h.write_u8(2);
                g.hash(&mut h);
            }
        }
    }
    q.weight.to_bits().hash(&mut h);
    q.slant.hash(&mut h);
    q.lang.hash(&mut h);
    h.finish()
}

fn locale_hash(name: &str) -> u32 {
    let mut h = FxHasher::default();
    h.write(name.as_bytes());
    h.finish() as u32
}

/// Characters whose shaping does not depend on their neighbours: the ideographs, kana, Hangul
/// syllables and the CJK and full-width punctuation. They have no default kerning, ligatures or
/// contextual forms, so each can be shaped once and reused (the combining marks inside these blocks
/// are left out).
fn is_context_free(cp: u32) -> bool {
    matches!(cp,
        0x3000..=0x3029 | 0x3030..=0x3098 | 0x309B..=0x30FF
        | 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xAC00..=0xD7A3 | 0xF900..=0xFAFF
        | 0xFF01..=0xFF60 | 0xFFE0..=0xFFE6 | 0x20000..=0x2FA1F)
}

fn word_key(fid: u32, loc: u32, sa: ScriptAnalysis, word: &[u16]) -> u64 {
    let mut h = FxHasher::default();
    h.write_u32(fid);
    h.write_u32(loc);
    h.write_u32(u32::from(sa.script));
    h.write_u32(sa.shapes);
    for &u in word {
        h.write_u16(u);
    }
    h.finish()
}

fn shape_hash(qi: u32, size_bits: u32, text: &str) -> u64 {
    let mut h = FxHasher::default();
    h.write_u32(qi);
    h.write_u32(size_bits);
    h.write(text.as_bytes());
    h.finish()
}

/// Characters that never pick a font of their own: they follow the character before them (controls,
/// zero-width and bidi format characters, combining marks, variation selectors).
fn is_attached(c: char) -> bool {
    let u = c as u32;
    u < 0x20
        || (0x7F..0xA0).contains(&u)
        || u == 0xAD
        || (0x300..0x370).contains(&u)
        || (0x1AB0..0x1B00).contains(&u)
        || (0x1DC0..0x1E00).contains(&u)
        || (0x200B..=0x200F).contains(&u)
        || (0x202A..=0x202E).contains(&u)
        || (0x2060..=0x206F).contains(&u)
        || (0x20D0..0x2100).contains(&u)
        || (0xFE00..=0xFE0F).contains(&u)
        || (0xFE20..0xFE30).contains(&u)
        || u == 0xFEFF
        || (0xE0100..=0xE01EF).contains(&u)
}

impl Inner {
    /// A font by id: a system font, or a web font (the `HIDDEN` bit is ignored).
    fn entry(&self, id: u32) -> Option<&Rc<FontEntry>> {
        let id = id & !HIDDEN;
        if id >= WEB_BASE { self.web_fonts.get(&id) } else { self.fonts.get(id as usize) }
    }

    /// The web faces changed: nothing computed from the font lists is valid any more.
    fn invalidate_text(&mut self) {
        self.queries.clear();
        self.query_map.clear();
        self.sticky.clear();
        self.choice.clear();
        self.shape_cache = Lru::new(SHAPE_CACHE_BYTES);
    }

    /// Time of one `GetGlyphs` + `GetGlyphPlacements` call; calls over 300 us are counted apart
    /// (the first call for a font and script pays for loading its tables).
    fn add_shape_time(&mut self, us: u64) {
        self.stats.us_shape += us;
        if us > 300 {
            self.stats.slow_calls += 1;
            self.stats.us_slow += us;
        }
    }

    fn find_family(&mut self, dw: &Factory, name: &str) -> Option<u32> {
        let key = name.to_lowercase();
        if let Some(&r) = self.families.get(&key) {
            return r;
        }
        let r = dw.collection().find_family(name).or_else(|| alias(name).and_then(|a| dw.collection().find_family(a)));
        if self.families.len() > 4096 {
            self.families.clear();
        }
        self.families.insert(key, r);
        r
    }

    fn add_font(&mut self, dw: &Factory, font: Font) -> Option<u32> {
        let t = Instant::now();
        let r = self.add_font_inner(dw, font);
        self.stats.us_face += t.elapsed().as_micros() as u64;
        r
    }

    fn add_font_inner(&mut self, dw: &Factory, font: Font) -> Option<u32> {
        let face = dw.create_face(&font)?;
        let addr = face.addr();
        if let Some(&id) = self.by_face.get(&addr) {
            return Some(id);
        }
        if self.fonts.len() >= MAX_FONTS {
            return None;
        }
        let m = face.metrics();
        let zero_advance = face.glyph_indices(&[u32::from(b'0')]).and_then(|v| v.first().copied()).filter(|&g| g != 0).and_then(|g| face.design_advance(g)).map(|a| a as f32);
        let id = self.fonts.len() as u32;
        self.fonts.push(Rc::new(FontEntry { face, font: Some(font), m, zero_advance, cmap: RefCell::new(FxMap::default()), ranges: None, color_state: Cell::new(0) }));
        self.by_face.insert(addr, id);
        Some(id)
    }

    fn font_from_family(&mut self, dw: &Factory, family: u32, weight: u32, style: u32) -> Option<u32> {
        let key = (family, weight, style as u8);
        if let Some(&r) = self.fam_fonts.get(&key) {
            return r;
        }
        let r = dw.collection().family(family).and_then(|f| f.first_matching_font(weight, style)).and_then(|font| self.add_font(dw, font));
        self.fam_fonts.insert(key, r);
        r
    }

    fn default_font(&mut self, dw: &Factory, weight: u32, style: u32) -> Option<u32> {
        for name in ["Segoe UI", "Arial", "Tahoma", "Microsoft YaHei", "SimSun"] {
            if let Some(fi) = self.find_family(dw, name)
                && let Some(f) = self.font_from_family(dw, fi, weight, style) {
                    return Some(f);
                }
        }
        self.font_from_family(dw, 0, weight, style)
    }

    fn query_index(&mut self, dw: &Factory, q: &FontQuery<'_>) -> u32 {
        let key = query_hash(q);
        if let Some(&i) = self.query_map.get(&key)
            && self.queries.get(i as usize).is_some_and(|qi| qi.matches(q)) {
                return i;
            }
        if self.queries.len() >= MAX_QUERIES {
            self.queries.clear();
            self.query_map.clear();
            self.sticky.clear();
            self.choice.clear();
            self.shape_cache = Lru::new(SHAPE_CACHE_BYTES);
        }
        let w = if q.weight.is_finite() { q.weight } else { 400.0 };
        let weight = w.round().clamp(1.0, 999.0) as u32;
        let style = match q.slant {
            FontSlant::Upright => 0,
            FontSlant::Oblique => 1,
            FontSlant::Italic => 2,
        };
        let mut chain: Vec<u32> = Vec::new();
        let mut items: Vec<u32> = Vec::new();
        let mut any_hidden = false;
        for fam in q.families.iter().take(16) {
            // A family with @font-face faces is those faces (best match first); an installed font of the same
            // name is not used (css-fonts-4 4.1).
            if let FontFamily::Named(n) = fam
                && let Some(ids) = self.web_by_family.get(&n.to_lowercase()).cloned()
            {
                let ranked: Vec<usize> = {
                    let descs: Vec<&FaceDesc> = ids.iter().filter_map(|&i| self.web.get(i).map(|w| &w.desc)).collect();
                    rank_faces(&descs, q.weight, q.slant).into_iter().filter_map(|r| ids.get(r).copied()).collect()
                };
                for wi in ranked {
                    let state = self.web.get(wi).map(|w| w.state);
                    let font = match state {
                        Some(WState::Ready(id)) => Some(id),
                        Some(WState::Local(fi)) => self.font_from_family(dw, fi, weight, style),
                        Some(WState::Hidden) => {
                            items.push(HIDDEN | wi as u32);
                            any_hidden = true;
                            None
                        }
                        _ => None,
                    };
                    if let Some(f) = font
                        && !chain.contains(&f)
                    {
                        chain.push(f);
                        items.push(f);
                    }
                }
                continue;
            }
            let fi = match fam {
                FontFamily::Named(n) => self.find_family(dw, n),
                FontFamily::Generic(g) => generic_names(*g).iter().find_map(|n| self.find_family(dw, n)),
            };
            if let Some(f) = fi.and_then(|fi| self.font_from_family(dw, fi, weight, style))
                && !chain.contains(&f) {
                    chain.push(f);
                    items.push(f);
                }
        }
        if chain.is_empty()
            && let Some(f) = self.default_font(dw, weight, style) {
                chain.push(f);
                items.push(f);
            }
        if !any_hidden {
            items.clear();
        }
        let primary = chain.iter().copied().find(|&f| self.covers(f, 0x20)).or_else(|| chain.first().copied()).unwrap_or(0);
        let locale_name = q.lang.and_then(normalize_lang).or_else(|| normalize_lang(&self.default_locale)).unwrap_or_else(|| "en-US".to_string());
        let base_name = chain.first().and_then(|&f| self.entry(f)).and_then(|e| e.font.as_ref()).and_then(|f| f.family_name()).unwrap_or_else(|| "Segoe UI".to_string());
        let info = QueryInfo {
            families: q.families.to_vec(),
            weight_bits: q.weight.to_bits(),
            slant: q.slant,
            lang: q.lang.map(str::to_string),
            chain,
            items,
            primary,
            loc: locale_hash(&locale_name),
            locale: wide_nul(&locale_name),
            base_family: wide_nul(&base_name),
            weight,
            style,
        };
        let i = self.queries.len() as u32;
        self.queries.push(Rc::new(info));
        self.sticky.push(Vec::new());
        self.query_map.insert(key, i);
        i
    }

    fn covers(&self, font: u32, cp: u32) -> bool {
        self.entry(font).is_some_and(|e| e.in_ranges(cp) && e.glyph_of(cp) != 0)
    }

    /// The font that draws `c` for query `qi`: the first of the family list that has it, then the
    /// fallback fonts the system already picked for this query, then the system fallback itself.
    fn choose_font(&mut self, dw: &Factory, qi: u32, q: &QueryInfo, c: char, prev: Option<u32>) -> u32 {
        let primary = q.chain.first().copied().unwrap_or(0);
        if is_attached(c) {
            return prev.unwrap_or(primary);
        }
        let cp = c as u32;
        let key = (u64::from(qi) << 32) | u64::from(cp);
        if let Some(&f) = self.choice.get(&key) {
            return f;
        }
        let f = self.resolve_char(dw, qi, q, cp).unwrap_or(primary);
        if self.choice.len() > 400_000 {
            self.choice.clear();
        }
        self.choice.insert(key, f);
        f
    }

    /// The font id of Segoe UI Emoji, if installed.
    fn emoji_font(&mut self, dw: &Factory) -> Option<u32> {
        if self.emoji.is_none() {
            let f = self.find_family(dw, "Segoe UI Emoji").and_then(|fam| self.font_from_family(dw, fam, 400, 0));
            self.emoji = Some(f);
        }
        self.emoji.flatten()
    }

    fn resolve_char(&mut self, dw: &Factory, qi: u32, q: &QueryInfo, cp: u32) -> Option<u32> {
        // A hidden web face that has the character makes it invisible; the font that gives it room is the next one.
        let mut hidden = 0u32;
        if q.items.is_empty() {
            if let Some(&f) = q.chain.iter().find(|&&f| self.covers(f, cp)) {
                return Some(f);
            }
        } else {
            for &it in &q.items {
                if it & HIDDEN != 0 {
                    if self.web.get((it & !HIDDEN) as usize).is_some_and(|w| w.desc.covers(cp)) {
                        hidden = HIDDEN;
                    }
                } else if self.covers(it, cp) {
                    return Some(it | hidden);
                }
            }
        }
        let sticky = self.sticky.get(qi as usize).cloned().unwrap_or_default();
        if let Some(&f) = sticky.iter().find(|&&f| self.covers(f, cp)) {
            return Some(f | hidden);
        }
        let c = char::from_u32(cp)?;
        let mut buf = [0u16; 2];
        let units = c.encode_utf16(&mut buf);
        self.stats.fallback_calls += 1;
        let t = Instant::now();
        let mapped = dw.map_characters(units, &q.locale, &q.base_family, q.weight, q.style);
        self.stats.us_map += t.elapsed().as_micros() as u64;
        let (_, font) = mapped?;
        let f = self.add_font(dw, font?)?;
        if let Some(s) = self.sticky.get_mut(qi as usize) {
            s.retain(|&x| x != f);
            s.insert(0, f);
            s.truncate(4);
        }
        Some(f | hidden)
    }

    fn shape_uncached(&mut self, dw: &Factory, text: &str, qi: u32, size: f32) -> Vec<Glyph> {
        let Some(q) = self.queries.get(qi as usize).cloned() else { return Vec::new() };
        if q.chain.is_empty() {
            return Vec::new();
        }
        let mut u16s: Vec<u16> = Vec::with_capacity(text.len());
        // UTF-16 unit to the byte offset of its character (both units of a pair share it).
        let mut map: Vec<u32> = Vec::with_capacity(text.len() + 1);
        let mut unit_font: Vec<u32> = Vec::with_capacity(text.len());
        let mut prev = None;
        let t_choose = Instant::now();
        for (bi, c) in text.char_indices() {
            // A character followed by VARIATION SELECTOR-16 asks for its emoji look: the emoji font draws it, and the
            // sequence (keycap, ZWJ sequence) is shaped with it (UTS #51 1.4.6).
            let emoji = if !is_attached(c) && text.get(bi + c.len_utf8()..).is_some_and(|r| r.starts_with('\u{FE0F}')) {
                self.emoji_font(dw).filter(|&e| self.covers(e, c as u32))
            } else {
                None
            };
            let f = match emoji {
                Some(e) => e,
                None => self.choose_font(dw, qi, &q, c, prev),
            };
            prev = Some(f);
            let mut buf = [0u16; 2];
            for &u in c.encode_utf16(&mut buf).iter() {
                u16s.push(u);
                map.push(bi as u32);
                unit_font.push(f);
            }
        }
        map.push(text.len() as u32);
        self.stats.us_choose += t_choose.elapsed().as_micros() as u64;
        let n = u16s.len();
        let mut out = Vec::with_capacity(n);
        let mut a = 0usize;
        while let Some(&f) = unit_font.get(a) {
            let mut b = a + 1;
            while unit_font.get(b) == Some(&f) {
                b += 1;
            }
            if let Some(seg) = u16s.get(a..b) {
                let before = out.len();
                self.shape_segment(dw, &q, f & !HIDDEN, seg, a, &map, size, &mut out);
                if f & HIDDEN != 0 {
                    for g in out.iter_mut().skip(before) {
                        g.font = FontId(g.font.0 | HIDDEN);
                    }
                }
            }
            a = b;
        }
        if out.iter().any(|g| g.glyph == 0) {
            self.fix_controls(text, &mut out, size);
        }
        out
    }

    /// DirectWrite gives control characters (tab, newline) the font's missing-glyph box. They are
    /// not drawn: use the space glyph, zero wide (a tab keeps the width of a space; the engine
    /// replaces it by its own tab stop anyway).
    fn fix_controls(&self, text: &str, out: &mut [Glyph], size: f32) {
        for g in out.iter_mut().filter(|g| g.glyph == 0) {
            let Some(s) = text.get(g.cluster_start as usize..g.cluster_end as usize) else { continue };
            if s.is_empty() || !s.chars().all(is_attached) {
                continue;
            }
            let Some(e) = self.entry(g.font.0) else { continue };
            let sp = e.glyph_of(0x20);
            if sp == 0 {
                continue;
            }
            g.glyph = u32::from(sp);
            g.advance = if s == "	" { e.face.design_advance(sp).map_or(0.0, |a| a as f32 * size / e.m.units_per_em.max(1.0)) } else { 0.0 };
        }
    }

    /// Shape a stretch of text drawn by one font. Context-free CJK characters come from a
    /// per-character cache; everything between them is split into script runs and shaped whole.
    #[allow(clippy::too_many_arguments)]
    fn shape_segment(&mut self, dw: &Factory, q: &QueryInfo, fid: u32, seg: &[u16], base: usize, map: &[u32], size: f32, out: &mut Vec<Glyph>) {
        let Some(entry) = self.entry(fid).cloned() else { return };
        let mut i = 0usize;
        let mut pending = 0usize; // start of the stretch not yet shaped
        while i < seg.len() {
            let (cp, len) = decode_unit(seg, i);
            // A mark or variation selector after the character must be shaped together with it.
            let next_attached = seg.get(i + len..).filter(|r| !r.is_empty()).is_some_and(|rest| char::from_u32(decode_unit(rest, 0).0).is_some_and(is_attached));
            let simple = !self.rtl && is_context_free(cp) && !next_attached;
            let hit = if simple { seg.get(i..i + len).and_then(|unit| self.context_free_glyph(dw, q, fid, &entry, cp, unit)) } else { None };
            if let Some((glyph, adv_em)) = hit {
                if pending < i {
                    self.shape_complex(dw, q, fid, &entry, seg, pending, i, base, map, size, out);
                }
                let cs = map.get(base + i).copied().unwrap_or(0);
                let ce = map.get(base + i + len).copied().unwrap_or(cs);
                out.push(Glyph { font: FontId(fid), glyph: u32::from(glyph), advance: adv_em * size, offset_x: 0.0, offset_y: 0.0, cluster_start: cs, cluster_end: ce });
                pending = i + len;
            }
            i += len;
        }
        if pending < seg.len() {
            self.shape_complex(dw, q, fid, &entry, seg, pending, seg.len(), base, map, size, out);
        }
    }

    /// Glyph and advance (in em) of one context-free character, shaped once by DirectWrite at a
    /// fixed size. `None` when DirectWrite does not give exactly one glyph (then it is shaped with
    /// its neighbours instead).
    fn context_free_glyph(&mut self, dw: &Factory, q: &QueryInfo, fid: u32, entry: &FontEntry, cp: u32, unit: &[u16]) -> Option<(u16, f32)> {
        let key = (fid, q.loc, cp);
        if let Some(&r) = self.simple.get(&key) {
            return Some(r);
        }
        let t = Instant::now();
        let sa = dw.analyze_script(unit, &q.locale).first().map(|r| r.2);
        self.stats.us_analyze += t.elapsed().as_micros() as u64;
        self.stats.analyze_calls += 1;
        let t = Instant::now();
        let sh = sa.and_then(|sa| dw.shape(unit, &entry.face, PROBE, sa, &q.locale));
        let us = t.elapsed().as_micros() as u64;
        self.add_shape_time(us);
        self.stats.us_single += us;
        self.stats.shape_calls += 1;
        let sh = sh?;
        let r = match (sh.glyphs.as_slice(), sh.advances.as_slice()) {
            ([g], [a]) => (*g, *a / PROBE),
            _ => return None,
        };
        if self.simple.len() > 300_000 {
            self.simple.clear();
        }
        self.simple.insert(key, r);
        Some(r)
    }

    /// Shape the context-free characters of `chars` that are not cached yet in bulk: the uncached
    /// ones are grouped by (font, script) and each group goes through `GetGlyphs` as one string, every
    /// character followed by a space so that no pair of them can interact. The result is used only if
    /// each character came out as exactly one glyph (cluster map 1:1); otherwise nothing is cached and
    /// the characters are shaped one at a time later, as before. Every glyph and advance is the same as
    /// shaping the character alone (checked against the whole CJK range by `bulk_shaping_matches_single`).
    fn prefetch_chars(&mut self, dw: &Factory, qi: u32, chars: &str) {
        let Some(q) = self.queries.get(qi as usize).cloned() else { return };
        if q.chain.is_empty() {
            return;
        }
        type Key = (u32, u16, u32);
        let mut index: FxMap<Key, usize> = FxMap::default();
        let mut groups: Vec<(Key, Vec<char>)> = Vec::new();
        for c in chars.chars() {
            let cp = c as u32;
            if !is_context_free(cp) {
                continue;
            }
            let fid = self.choose_font(dw, qi, &q, c, None);
            if self.simple.contains_key(&(fid, q.loc, cp)) {
                continue;
            }
            let mut buf = [0u16; 2];
            let t = Instant::now();
            let sa = dw.analyze_script(c.encode_utf16(&mut buf), &q.locale).first().map(|r| r.2);
            self.stats.us_analyze += t.elapsed().as_micros() as u64;
            self.stats.analyze_calls += 1;
            let Some(sa) = sa else { continue };
            let key = (fid, sa.script, sa.shapes);
            let gi = *index.entry(key).or_insert_with(|| {
                groups.push((key, Vec::new()));
                groups.len() - 1
            });
            if let Some((_, v)) = groups.get_mut(gi) {
                v.push(c);
            }
        }
        for ((fid, script, shapes), cs) in groups {
            let Some(entry) = self.entry(fid).cloned() else { continue };
            for part in cs.chunks(BATCH_CHARS) {
                self.shape_batch(dw, &q, fid, &entry, ScriptAnalysis { script, shapes }, part);
            }
        }
    }

    fn shape_batch(&mut self, dw: &Factory, q: &QueryInfo, fid: u32, entry: &FontEntry, sa: ScriptAnalysis, chars: &[char]) {
        let mut units: Vec<u16> = Vec::with_capacity(chars.len() * 3);
        let mut pos: Vec<(usize, usize)> = Vec::with_capacity(chars.len());
        for &c in chars {
            let mut buf = [0u16; 2];
            let u = c.encode_utf16(&mut buf);
            pos.push((units.len(), u.len()));
            units.extend_from_slice(u);
            units.push(0x20);
        }
        let t = Instant::now();
        let sh = dw.shape(&units, &entry.face, PROBE, sa, &q.locale);
        let us = t.elapsed().as_micros() as u64;
        self.add_shape_time(us);
        self.stats.us_batch += us;
        self.stats.shape_calls += 1;
        self.stats.batch_calls += 1;
        let Some(sh) = sh else { return };
        // Glyph 2i is character i, glyph 2i + 1 its space; the cluster map says the same.
        if sh.glyphs.len() != 2 * chars.len() {
            return;
        }
        for (i, &(u, l)) in pos.iter().enumerate() {
            let want = (2 * i) as u16;
            let ok = sh.cluster_map.get(u) == Some(&want) && (l == 1 || sh.cluster_map.get(u + 1) == Some(&want)) && sh.cluster_map.get(u + l) == Some(&(want + 1));
            if !ok {
                return;
            }
        }
        if self.simple.len() > 300_000 {
            self.simple.clear();
        }
        for (i, &c) in chars.iter().enumerate() {
            if let (Some(&g), Some(&a)) = (sh.glyphs.get(2 * i), sh.advances.get(2 * i)) {
                self.simple.insert((fid, q.loc, c as u32), (g, a / PROBE));
                self.stats.batch_chars += 1;
            }
        }
    }

    /// Shape the ASCII words of `words` that are not in the word cache yet, many to a `GetGlyphs` call:
    /// `w1 SEP w2 SEP ...` with `SEP` = `WORD_SEP`. The glyphs of each word are cut out of the result and
    /// cached like a word shaped alone. A word is only taken when it came out exactly as it would alone:
    /// the glyph run is contiguous and one separator glyph follows, and its last glyph has its
    /// design advance (the one thing a rule between the word and the separator could change).
    /// Everything else is left to be shaped when it is needed.
    fn prefetch_words(&mut self, dw: &Factory, qi: u32, words: &[&str]) {
        let Some(q) = self.queries.get(qi as usize).cloned() else { return };
        let Some(&fid) = q.chain.first() else { return };
        let Some(entry) = self.entry(fid).cloned() else { return };
        // All printable ASCII must come from the primary font, or the words would be split by fallback.
        if !(0x21u32..0x7F).all(|cp| self.covers(fid, cp)) {
            return;
        }
        let sa = dw.latin_script();
        let em = entry.m.units_per_em.max(1.0);
        let mut todo: Vec<(u64, Vec<u16>)> = Vec::new();
        for w in words {
            let units: Vec<u16> = w.encode_utf16().collect();
            let key = word_key(fid, q.loc, sa, &units);
            if self.words.contains_key(&key) || todo.last().is_some_and(|(k, _)| *k == key) {
                continue;
            }
            todo.push((key, units));
        }
        let mut i = 0usize;
        while i < todo.len() {
            let mut j = i;
            let mut n = 0usize;
            while let Some((_, u)) = todo.get(j) {
                if n + u.len() + 1 > BATCH_UNITS && j > i {
                    break;
                }
                n += u.len() + 1;
                j += 1;
            }
            let Some(part) = todo.get(i..j) else { return };
            i = j;
            let mut units: Vec<u16> = Vec::with_capacity(n);
            let mut pos: Vec<usize> = Vec::with_capacity(part.len());
            for (_, u) in part {
                pos.push(units.len());
                units.extend_from_slice(u);
                units.push(WORD_SEP);
            }
            let t = Instant::now();
            let sh = dw.shape(&units, &entry.face, PROBE, sa, &q.locale);
            let us = t.elapsed().as_micros() as u64;
            self.add_shape_time(us);
            self.stats.us_batch += us;
            self.stats.shape_calls += 1;
            self.stats.batch_calls += 1;
            let Some(sh) = sh else { continue };
            for (k, (key, u)) in part.iter().enumerate() {
                let Some(&start) = pos.get(k) else { break };
                let end = start + u.len();
                let (Some(&g0), Some(&g1)) = (sh.cluster_map.get(start), sh.cluster_map.get(end)) else { continue };
                let (g0, g1) = (usize::from(g0), usize::from(g1));
                // The separator is one glyph: the next word starts right after it (or the run ends).
                let next = pos.get(k + 1).map_or(Some(sh.glyphs.len()), |&p| sh.cluster_map.get(p).map(|&g| usize::from(g)));
                if g0 >= g1 || next != Some(g1 + 1) || sh.cluster_map.get(start..end).is_none_or(|m| m.iter().any(|&g| !(g0..g1).contains(&usize::from(g)))) {
                    continue;
                }
                let (Some(glyphs), Some(advances), Some(offsets), Some(map)) = (sh.glyphs.get(g0..g1), sh.advances.get(g0..g1), sh.offsets.get(g0..g1), sh.cluster_map.get(start..end)) else { continue };
                let (Some(&last_g), Some(&last_adv)) = (glyphs.last(), advances.last()) else { continue };
                let design = entry.face.design_advance(last_g).map_or(f32::NAN, |a| a as f32 * PROBE / em);
                let close = (last_adv - design).abs() < 0.01; // false for NaN too
                if !close {
                    continue;
                }
                let shaped = Shaped { glyphs: glyphs.to_vec(), cluster_map: map.iter().map(|&g| g - g0 as u16).collect(), advances: advances.to_vec(), offsets: offsets.to_vec() };
                if self.words.len() > 16_000 {
                    self.words.clear();
                }
                self.words.insert(*key, WordEntry { fid, loc: q.loc, script: sa.script, shapes: sa.shapes, units: u.as_slice().into(), shaped: Rc::new(shaped) });
                self.stats.word_prefetched += 1;
            }
        }
    }

    /// Shape one script run word by word, the way Blink's word cache does: the text is cut at every
    /// U+0020, each word and each space is shaped alone (at `PROBE` size, scaled afterwards) and
    /// remembered by (font, locale, script, text). Kerning between a word and the space next to it is
    /// lost, as in Chrome. Runs the platform cannot shape become missing-glyph boxes.
    #[allow(clippy::too_many_arguments)]
    fn shape_words(&mut self, dw: &Factory, q: &QueryInfo, fid: u32, entry: &FontEntry, sub: &[u16], sa: ScriptAnalysis, size: f32, base: usize, map: &[u32], out: &mut Vec<Glyph>) {
        if self.rtl {
            // Right to left: the run goes through as a whole (words are not cached; joining and kerning need the context).
            let t = Instant::now();
            let sh = dw.shape_dir(sub, &entry.face, size, sa, &q.locale, true);
            self.add_shape_time(t.elapsed().as_micros() as u64);
            self.stats.shape_calls += 1;
            match sh {
                Some(sh) => emit_run_rtl(out, fid, &sh, base, sub.len(), map, 1.0),
                None => notdef_run(out, fid, base, base + sub.len(), map, size),
            }
            return;
        }
        let mut i = 0usize;
        while i < sub.len() {
            let j = if sub.get(i) == Some(&0x20) { i + 1 } else { sub.get(i..).and_then(|r| r.iter().position(|&u| u == 0x20)).map_or(sub.len(), |p| i + p) };
            let Some(word) = sub.get(i..j) else { return };
            let key = word_key(fid, q.loc, sa, word);
            let hit = self.words.get(&key).filter(|w| w.fid == fid && w.loc == q.loc && w.script == sa.script && w.shapes == sa.shapes && *w.units == *word).map(|w| Rc::clone(&w.shaped));
            let shaped = match hit {
                Some(sh) => {
                    self.stats.word_hits += 1;
                    Some(sh)
                }
                None => {
                    self.stats.word_misses += 1;
                    let t = Instant::now();
                    let sh = dw.shape(word, &entry.face, PROBE, sa, &q.locale).map(Rc::new);
                    self.add_shape_time(t.elapsed().as_micros() as u64);
                    self.stats.shape_calls += 1;
                    if let Some(sh) = &sh {
                        if self.words.len() > 16_000 {
                            self.words.clear();
                        }
                        self.words.insert(key, WordEntry { fid, loc: q.loc, script: sa.script, shapes: sa.shapes, units: word.into(), shaped: Rc::clone(sh) });
                    }
                    sh
                }
            };
            match shaped {
                Some(sh) => emit_run(out, fid, &sh, base + i, j - i, map, size / PROBE),
                None => notdef_run(out, fid, base + i, base + j, map, size),
            }
            i = j;
        }
    }

    /// Script runs of `seg[from..to]`, each shaped with the font of `entry`.
    #[allow(clippy::too_many_arguments)]
    fn shape_complex(&mut self, dw: &Factory, q: &QueryInfo, fid: u32, entry: &FontEntry, seg: &[u16], from: usize, to: usize, base: usize, map: &[u32], size: f32, out: &mut Vec<Glyph>) {
        let Some(part) = seg.get(from..to) else { return };
        let (base, seg) = (base + from, part);
        let ascii = seg.iter().all(|&u| (0x20..0x7F).contains(&u));
        let t = Instant::now();
        let runs = if ascii { vec![(0u32, seg.len() as u32, dw.latin_script())] } else { dw.analyze_script(seg, &q.locale) };
        self.stats.us_analyze += t.elapsed().as_micros() as u64;
        self.stats.analyze_calls += u64::from(!ascii);
        let mut cursor = 0usize;
        for (ps, pl, sa) in runs {
            let (ps, pl) = (ps as usize, pl as usize);
            if ps > cursor {
                notdef_run(out, fid, base + cursor, base + ps, map, size);
            }
            let end = (ps + pl).min(seg.len());
            let mut s = ps.max(cursor);
            while s < end {
                let e = if s + MAX_RUN_UNITS >= end { end } else { run_cut(seg, s, s + MAX_RUN_UNITS) };
                if let Some(sub) = seg.get(s..e) {
                    self.shape_words(dw, q, fid, entry, sub, sa, size, base + s, map, out);
                }
                s = e;
            }
            cursor = end;
        }
        if cursor < seg.len() {
            notdef_run(out, fid, base + cursor, base + seg.len(), map, size);
        }
    }
}

/// How far back from the limit `run_cut` looks for a good place to cut a long run.
const CUT_SEARCH: usize = 512;

/// Whether a character joins the character before it (combining marks, variation selectors, joiners,
/// Hangul medial and final jamo, kana voicing marks): cutting before it would split a cluster.
fn joins_previous(u: u32) -> bool {
    matches!(u,
        0x0300..=0x036F | 0x0483..=0x0489 | 0x0591..=0x05C7 | 0x0610..=0x061A | 0x064B..=0x065F | 0x0670
        | 0x06D6..=0x06ED | 0x0900..=0x0903 | 0x093A..=0x094F | 0x0E31 | 0x0E34..=0x0E3A | 0x0E47..=0x0E4E
        | 0x1160..=0x11FF | 0x1AB0..=0x1AFF | 0x1DC0..=0x1DFF | 0x200B..=0x200F | 0x20D0..=0x20FF
        | 0x3099..=0x309A | 0xA960..=0xA97C | 0xD7B0..=0xD7FF | 0xFE00..=0xFE0F | 0xFE20..=0xFE2F | 0x1F3FB..=0x1F3FF | 0xE0020..=0xE007F | 0xE0100..=0xE01EF)
}

/// Where to cut `seg[s..limit]` (the run is longer than one `GetGlyphs` call takes): after a space
/// if there is one near the limit, otherwise at the last position that neither splits a surrogate pair
/// nor separates a character from the marks, joiner or variation selector that follow it (or from the
/// joiner before). Cutting inside a cluster would shape the two halves wrongly. If nothing fits in
/// `CUT_SEARCH` units, the cut stays at the limit.
fn run_cut(seg: &[u16], s: usize, limit: usize) -> usize {
    let lo = (limit.saturating_sub(CUT_SEARCH)).max(s + 1);
    let ok = |p: usize| {
        let (cp, _) = decode_unit(seg, p);
        let prev = seg.get(p.wrapping_sub(1)).copied();
        !(0xDC00..0xE000).contains(&seg.get(p).copied().unwrap_or(0)) && !joins_previous(cp) && prev != Some(0x200D)
    };
    let mut p = limit;
    while p >= lo {
        if seg.get(p - 1) == Some(&0x20) && ok(p) {
            return p;
        }
        p -= 1;
    }
    p = limit;
    while p >= lo {
        if ok(p) {
            return p;
        }
        p -= 1;
    }
    limit
}

/// The code point starting at unit `i` of `units` and how many units it takes.
fn decode_unit(units: &[u16], i: usize) -> (u32, usize) {
    match (units.get(i).copied(), units.get(i + 1).copied()) {
        (Some(h @ 0xD800..=0xDBFF), Some(l @ 0xDC00..=0xDFFF)) => (0x10000 + ((u32::from(h) - 0xD800) << 10) + (u32::from(l) - 0xDC00), 2),
        (Some(u), _) => (u32::from(u), 1),
        (None, _) => (0, 1),
    }
}

/// Glyphs of one shaped run. `start`/`len` locate the run in the paragraph's UTF-16 units; `map`
/// turns unit positions into byte offsets. A cluster is a stretch of text whose cluster-map values
/// are equal; it owns the glyphs from its value up to the next cluster's value.
/// `scale` multiplies advances and offsets (the run was shaped at another size).
fn emit_run(out: &mut Vec<Glyph>, fid: u32, sh: &Shaped, start: usize, len: usize, map: &[u32], scale: f32) {
    let gcount = sh.glyphs.len();
    let mut i = 0usize;
    while i < len {
        let Some(&g0) = sh.cluster_map.get(i) else { break };
        let mut j = i + 1;
        while j < len && sh.cluster_map.get(j) == Some(&g0) {
            j += 1;
        }
        let g1 = if j < len { sh.cluster_map.get(j).map_or(gcount, |&v| usize::from(v)) } else { gcount };
        let cs = map.get(start + i).copied().unwrap_or(0);
        let ce = map.get(start + j).copied().unwrap_or(cs);
        for g in usize::from(g0)..g1.min(gcount) {
            let (ox, oy) = sh.offsets.get(g).copied().unwrap_or((0.0, 0.0));
            out.push(Glyph {
                font: FontId(fid),
                glyph: sh.glyphs.get(g).map_or(0, |&v| u32::from(v)),
                advance: sh.advances.get(g).copied().unwrap_or(0.0) * scale,
                offset_x: ox * scale,
                offset_y: -oy * scale,
                cluster_start: cs,
                cluster_end: ce,
            });
        }
        i = j;
    }
}

/// `emit_run` for a right-to-left run. DirectWrite hands the glyphs of such a run over in logical order like those of any
/// run, to be drawn from the right edge leftwards; the glyphs of one cluster are put into left-to-right drawing order here,
/// so that a base glyph and its marks (which have no advance) land where DirectWrite would put them.
fn emit_run_rtl(out: &mut Vec<Glyph>, fid: u32, sh: &Shaped, start: usize, len: usize, map: &[u32], scale: f32) {
    let from = out.len();
    emit_run(out, fid, sh, start, len, map, scale);
    let mut i = from;
    while let Some(g0) = out.get(i).copied() {
        let mut j = i + 1;
        while out.get(j).is_some_and(|g| g.cluster_start == g0.cluster_start && g.cluster_end == g0.cluster_end) {
            j += 1;
        }
        if let Some(run) = out.get_mut(i..j) {
            run.reverse();
        }
        i = j;
    }
}

/// One missing-glyph (index 0), 1 em wide box per character of the units `from..to`.
fn notdef_run(out: &mut Vec<Glyph>, fid: u32, from: usize, to: usize, map: &[u32], size: f32) {
    let mut i = from;
    while i < to {
        let cs = map.get(i).copied().unwrap_or(0);
        let mut j = i + 1;
        while j < to && map.get(j) == Some(&cs) {
            j += 1;
        }
        let ce = map.get(j).copied().unwrap_or(cs);
        out.push(Glyph { font: FontId(fid), glyph: 0, advance: size, offset_x: 0.0, offset_y: 0.0, cluster_start: cs, cluster_end: ce });
        i = j;
    }
}

impl WinPlatform {
    /// `Platform::shape` and `shape_rtl`: `rtl` shapes right to left (the cache keeps the two apart).
    fn shape_dir(&self, text: &str, query: &FontQuery<'_>, size_px: f32, rtl: bool) -> Vec<Glyph> {
        if text.is_empty() {
            return Vec::new();
        }
        let size = if size_px.is_finite() { size_px.clamp(0.01, 4096.0) } else { 16.0 };
        let size_bits = size.to_bits();
        let mut inn = self.inner.borrow_mut();
        let qi = inn.query_index(&self.dw, query);
        let qk = qi | (u32::from(rtl) << 31);
        let key = shape_hash(qk, size_bits, text);
        let hit = match inn.shape_cache.get(&key) {
            Some(e) if e.qi == qk && e.size_bits == size_bits && *e.text == *text => Some(Rc::clone(&e.glyphs)),
            _ => None,
        };
        if let Some(g) = hit {
            inn.stats.shape_hits += 1;
            return g.to_vec();
        }
        inn.stats.shape_misses += 1;
        let t_miss = Instant::now();
        inn.rtl = rtl;
        let glyphs = inn.shape_uncached(&self.dw, text, qi, size);
        inn.rtl = false;
        inn.stats.us_miss += t_miss.elapsed().as_micros() as u64;
        let cost = text.len() + glyphs.len() * GLYPH_COST + 96;
        inn.shape_cache.insert(key, ShapeEntry { qi: qk, size_bits, text: text.into(), glyphs: glyphs.as_slice().into() }, cost);
        glyphs
    }
}

impl Platform for WinPlatform {
    fn decode_legacy(&self, encoding: &str, bytes: &[u8]) -> Option<String> {
        crate::encodings::decode_legacy(encoding, bytes)
    }

    fn resolve_font(&self, query: &FontQuery<'_>) -> Option<FontId> {
        let mut inn = self.inner.borrow_mut();
        let qi = inn.query_index(&self.dw, query);
        // It gives `ex`, `ch` and the strut; a web font whose `unicode-range` leaves out U+0020 is passed over.
        let q = inn.queries.get(qi as usize)?;
        (!q.chain.is_empty()).then_some(FontId(q.primary))
    }

    fn font_metrics(&self, font: FontId, size_px: f32) -> FontMetrics {
        let inn = self.inner.borrow();
        let Some(e) = inn.entry(font.0) else { return FontMetrics::default() };
        let m = &e.m;
        let s = if size_px.is_finite() { size_px / m.units_per_em.max(1.0) } else { 0.0 };
        FontMetrics {
            ascent: m.ascent * s,
            descent: m.descent * s,
            line_gap: m.line_gap.max(0.0) * s,
            x_height: (m.x_height > 0.0).then_some(m.x_height * s),
            zero_advance: e.zero_advance.map(|z| z * s),
            // DirectWrite measures the top edge of the line up from the baseline; the trait wants
            // the underline's distance down and the strike-through's distance up.
            underline_offset: -m.underline_position * s,
            underline_thickness: m.underline_thickness * s,
            strikeout_offset: m.strikethrough_position * s,
            strikeout_thickness: m.strikethrough_thickness * s,
        }
    }

    fn shape(&self, text: &str, query: &FontQuery<'_>, size_px: f32) -> Vec<Glyph> {
        self.shape_dir(text, query, size_px, false)
    }

    fn shape_rtl(&self, text: &str, query: &FontQuery<'_>, size_px: f32) -> Vec<Glyph> {
        self.shape_dir(text, query, size_px, true)
    }

    fn bidi_levels(&self, text: &str, rtl: bool) -> Vec<u8> {
        // Exact sizes: a page of 4 MB of text must not cost a word per byte here.
        let mut units: Vec<u16> = Vec::with_capacity(text.chars().map(char::len_utf16).sum());
        units.extend(text.encode_utf16());
        let levels = self.dw.analyze_bidi(&units, rtl);
        drop(units);
        // One level per character: the level of its first UTF-16 unit.
        let mut out = Vec::with_capacity(text.chars().count());
        let mut i = 0usize;
        for c in text.chars() {
            let Some(&l) = levels.get(i) else { return Vec::new() };
            out.push(l);
            i += c.len_utf16();
        }
        if i != levels.len() {
            return Vec::new();
        }
        out
    }

    fn prefetch(&self, chars: &str, words: &[&str], query: &FontQuery<'_>) {
        let mut inn = self.inner.borrow_mut();
        let qi = inn.query_index(&self.dw, query);
        inn.prefetch_chars(&self.dw, qi, chars);
        inn.prefetch_words(&self.dw, qi, words);
    }

    fn set_web_faces(&self, faces: &[FaceDesc]) {
        let mut inn = self.inner.borrow_mut();
        if inn.web.is_empty() && faces.is_empty() {
            return;
        }
        // The faces of the page before go first (they use the bytes the loader holds), then the loader.
        inn.web_fonts.clear();
        inn.web_loader = None;
        if inn.next_web_id > 0 {
            // Font ids (16 bits wide in the display list) are used again from 0: nothing cached under the old ones may stay.
            inn.next_web_id = 0;
            inn.glyph_cache = Lru::new(GLYPH_CACHE_BYTES);
            inn.simple = FxMap::default();
            inn.words = FxMap::default();
        }
        inn.web = faces.iter().map(|d| WebFace { desc: d.clone(), state: WState::Fallback }).collect();
        inn.web_by_family = FxMap::default();
        for (i, d) in faces.iter().enumerate() {
            inn.web_by_family.entry(d.family.clone()).or_default().push(i);
        }
        inn.invalidate_text();
    }

    fn set_web_face_state(&self, index: usize, state: WebFaceState) {
        let mut inn = self.inner.borrow_mut();
        if let Some(w) = inn.web.get_mut(index)
            && matches!(w.state, WState::Fallback | WState::Hidden)
        {
            w.state = match state {
                WebFaceState::Fallback => WState::Fallback,
                WebFaceState::Hidden => WState::Hidden,
            };
            inn.invalidate_text();
        }
    }

    fn load_web_face(&self, index: usize, sfnt: &[u8]) -> bool {
        let mut inn = self.inner.borrow_mut();
        if index >= inn.web.len() || inn.next_web_id >= 0x3000 {
            return false;
        }
        if inn.web_loader.is_none() {
            inn.web_loader = WebFontLoader::new(&self.dw);
        }
        let Some(face) = inn.web_loader.as_mut().and_then(|l| l.create_face(sfnt.to_vec())) else { return false };
        let m = face.metrics();
        let zero_advance = face.glyph_indices(&[u32::from(b'0')]).and_then(|v| v.first().copied()).filter(|&g| g != 0).and_then(|g| face.design_advance(g)).map(|a| a as f32);
        let ranges: Option<Rc<[(u32, u32)]>> = inn.web.get(index).filter(|w| !w.desc.ranges.is_empty()).map(|w| w.desc.ranges.as_slice().into());
        let id = WEB_BASE + inn.next_web_id;
        inn.next_web_id += 1;
        inn.web_fonts.insert(id, Rc::new(FontEntry { face, font: None, m, zero_advance, cmap: RefCell::new(FxMap::default()), ranges, color_state: Cell::new(0) }));
        if let Some(w) = inn.web.get_mut(index) {
            w.state = WState::Ready(id);
        }
        inn.invalidate_text();
        true
    }

    fn use_local_face(&self, index: usize, name: &str) -> bool {
        let mut inn = self.inner.borrow_mut();
        let Some(fi) = inn.find_family(&self.dw, name) else { return false };
        match inn.web.get_mut(index) {
            Some(w) => w.state = WState::Local(fi),
            None => return false,
        }
        inn.invalidate_text();
        true
    }

    fn line_breaks(&self, text: &str, lang: Option<&str>) -> Vec<usize> {
        if text.len() < 2 {
            return Vec::new();
        }
        let locale = {
            let inn = self.inner.borrow();
            wide_nul(&lang.and_then(normalize_lang).or_else(|| normalize_lang(&inn.default_locale)).unwrap_or_else(|| "en-US".to_string()))
        };
        // One unit per character (two for the rare astral one): the byte length overshoots by up to 3x for CJK text.
        let n_chars = text.chars().count();
        let mut u16s: Vec<u16> = Vec::with_capacity(n_chars);
        let mut map: Vec<u32> = Vec::with_capacity(n_chars);
        for (bi, c) in text.char_indices() {
            let mut buf = [0u16; 2];
            for &u in c.encode_utf16(&mut buf).iter() {
                u16s.push(u);
                map.push(bi as u32);
            }
        }
        let t_br = Instant::now();
        let bits = self.dw.analyze_line_breaks(&u16s, &locale);
        {
            let mut inn = self.inner.borrow_mut();
            inn.stats.us_breaks += t_br.elapsed().as_micros() as u64;
            inn.stats.break_calls += 1;
        }
        if bits.len() != u16s.len() {
            return Vec::new();
        }
        let mut out = Vec::new();
        for i in 1..u16s.len() {
            // Not inside a surrogate pair.
            if map.get(i) == map.get(i - 1) {
                continue;
            }
            if let (Some(&p), Some(&n), Some(&off)) = (bits.get(i - 1), bits.get(i), map.get(i))
                && break_between(p, n) {
                    out.push(off as usize);
                }
        }
        out
    }

    fn glyph_mask(&self, font: FontId, glyph: u32, size_px: f32, subpixel: u8) -> Option<Rc<GlyphMask>> {
        if font.0 & HIDDEN != 0 {
            return None;
        }
        let sub = subpixel.min(3);
        let size = if size_px.is_finite() { size_px.clamp(0.01, 1024.0) } else { return None };
        let g = u16::try_from(glyph).ok()?;
        let key = (font.0, glyph, size.to_bits(), sub);
        let mut inn = self.inner.borrow_mut();
        if let Some(v) = inn.glyph_cache.get(&key) {
            let r = v.clone();
            inn.stats.glyph_hits += 1;
            return r;
        }
        inn.stats.glyph_misses += 1;
        let entry = inn.entry(font.0)?.clone();
        // A colour glyph (emoji) is asked for first, unless the font has shown it has none.
        let st = entry.color_state.get();
        let colored = if st < 64 {
            let c = self.dw.rasterize_color(&entry.face, g, size, f32::from(sub) / 4.0);
            entry.color_state.set(if c.is_some() { -1 } else if st >= 0 { st + 1 } else { st });
            c.map(|c| GlyphMask { left: c.left, top: c.top, width: c.width, height: c.height, alpha: Vec::new(), bgra: c.bgra })
        } else {
            None
        };
        let mask = colored.or_else(|| {
            self.dw.rasterize(&entry.face, g, size, f32::from(sub) / 4.0).map(|raw| {
                // ClearType 3x1 gives one coverage value per colour stripe; the mean is plain grayscale.
                let alpha: Vec<u8> = raw.rgb.as_chunks::<3>().0.iter().map(|p| (p.iter().map(|&b| u32::from(b)).sum::<u32>() / 3) as u8).collect();
                GlyphMask { left: raw.left, top: raw.top, width: raw.width, height: raw.height, alpha, bgra: Vec::new() }
            })
        });
        match mask {
            Some(m) => {
                let cost = m.alpha.len() + m.bgra.len() + 64;
                let out = Rc::new(m);
                inn.glyph_cache.insert(key, Some(out.clone()), cost);
                Some(out)
            }
            None => {
                inn.glyph_cache.insert(key, None, 64);
                None
            }
        }
    }
}
