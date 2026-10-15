//! The Windows implementation of the engine's `Platform` trait (L1 task 8, "Windows 平台").
//!
//! Fonts come from the DirectWrite system collection; characters a font lacks are looked up
//! through `IDWriteFontFallback::MapCharacters` with the element's language; text is shaped with
//! `IDWriteTextAnalyzer`; masks come from `IDWriteGlyphRunAnalysis`. See docs/decisions.md
//! ("1b-3") for the choices made where the task sheet leaves room.

use std::cell::RefCell;
use std::time::Instant;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use qinglan_engine::css::types::{FontFamily, GenericFamily};
use qinglan_engine::platform::{FontId, FontMetrics, FontQuery, FontSlant, Glyph, GlyphMask, Platform};

use crate::fx::{FxHasher, FxMap};
use crate::lru::Lru;
use crate::sys;
use crate::sys::dwrite::{DesignMetrics, Face, Factory, Font, Shaped, break_between};

/// Glyph mask cache budget (L1 task 8: LRU, at most 8 MB).
const GLYPH_CACHE_BYTES: usize = 8 << 20;
/// Shaping cache budget. Counted by text bytes plus 28 bytes per glyph plus a fixed overhead.
const SHAPE_CACHE_BYTES: usize = 8 << 20;
/// Most fonts (faces) the platform keeps; more are refused and the text falls back to the primary font.
const MAX_FONTS: usize = 512;
/// Most distinct font queries kept (family list, weight, slant, language).
const MAX_QUERIES: usize = 8192;
/// A single `GetGlyphs` call gets at most this many UTF-16 units (its cluster map holds 16-bit values).
const MAX_RUN_UNITS: usize = 8192;
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
}

struct FontEntry {
    face: Face,
    font: Font,
    m: DesignMetrics,
    /// Advance of "0" in design units.
    zero_advance: Option<f32>,
    /// Code point to glyph index (0 = the font has none).
    cmap: RefCell<FxMap<u32, u16>>,
}

impl FontEntry {
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
        let e = self.inner.borrow().fonts.get(font.0 as usize).cloned()?;
        e.font.family_name()
    }

    /// `DWRITE_FONT_SIMULATIONS` of `font`: 1 = synthetic bold, 2 = synthetic oblique.
    pub fn simulations(&self, font: FontId) -> u32 {
        self.inner.borrow().fonts.get(font.0 as usize).map_or(0, |e| e.font.simulations())
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
        self.fonts.push(Rc::new(FontEntry { face, font, m, zero_advance, cmap: RefCell::new(FxMap::default()) }));
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
        for fam in q.families.iter().take(16) {
            let fi = match fam {
                FontFamily::Named(n) => self.find_family(dw, n),
                FontFamily::Generic(g) => generic_names(*g).iter().find_map(|n| self.find_family(dw, n)),
            };
            if let Some(f) = fi.and_then(|fi| self.font_from_family(dw, fi, weight, style))
                && !chain.contains(&f) {
                    chain.push(f);
                }
        }
        if chain.is_empty()
            && let Some(f) = self.default_font(dw, weight, style) {
                chain.push(f);
            }
        let locale_name = q.lang.and_then(normalize_lang).or_else(|| normalize_lang(&self.default_locale)).unwrap_or_else(|| "en-US".to_string());
        let base_name = chain.first().and_then(|&f| self.fonts.get(f as usize)).and_then(|e| e.font.family_name()).unwrap_or_else(|| "Segoe UI".to_string());
        let info = QueryInfo {
            families: q.families.to_vec(),
            weight_bits: q.weight.to_bits(),
            slant: q.slant,
            lang: q.lang.map(str::to_string),
            chain,
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
        self.fonts.get(font as usize).is_some_and(|e| e.glyph_of(cp) != 0)
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

    fn resolve_char(&mut self, dw: &Factory, qi: u32, q: &QueryInfo, cp: u32) -> Option<u32> {
        if let Some(&f) = q.chain.iter().find(|&&f| self.covers(f, cp)) {
            return Some(f);
        }
        let sticky = self.sticky.get(qi as usize).cloned().unwrap_or_default();
        if let Some(&f) = sticky.iter().find(|&&f| self.covers(f, cp)) {
            return Some(f);
        }
        let c = char::from_u32(cp)?;
        let mut buf = [0u16; 2];
        let units = c.encode_utf16(&mut buf);
        self.stats.fallback_calls += 1;
        let (_, font) = dw.map_characters(units, &q.locale, &q.base_family, q.weight, q.style)?;
        let f = self.add_font(dw, font?)?;
        if let Some(s) = self.sticky.get_mut(qi as usize) {
            s.retain(|&x| x != f);
            s.insert(0, f);
            s.truncate(4);
        }
        Some(f)
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
            let f = self.choose_font(dw, qi, &q, c, prev);
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
                self.shape_segment(dw, &q, f, seg, a, &map, size, &mut out);
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
            let Some(e) = self.fonts.get(g.font.0 as usize) else { continue };
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
        let Some(entry) = self.fonts.get(fid as usize).cloned() else { return };
        let mut i = 0usize;
        let mut pending = 0usize; // start of the stretch not yet shaped
        while i < seg.len() {
            let (cp, len) = decode_unit(seg, i);
            // A mark or variation selector after the character must be shaped together with it.
            let next_attached = seg.get(i + len..).filter(|r| !r.is_empty()).is_some_and(|rest| char::from_u32(decode_unit(rest, 0).0).is_some_and(is_attached));
            let simple = is_context_free(cp) && !next_attached;
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
        const PROBE: f32 = 100.0;
        let key = (fid, q.loc, cp);
        if let Some(&r) = self.simple.get(&key) {
            return Some(r);
        }
        let t = Instant::now();
        let sa = dw.analyze_script(unit, &q.locale).first().map(|r| r.2)?;
        let sh = dw.shape(unit, &entry.face, PROBE, sa, &q.locale)?;
        self.stats.us_shape += t.elapsed().as_micros() as u64;
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

    /// Script runs of `seg[from..to]`, each shaped with the font of `entry`.
    #[allow(clippy::too_many_arguments)]
    fn shape_complex(&mut self, dw: &Factory, q: &QueryInfo, fid: u32, entry: &FontEntry, seg: &[u16], from: usize, to: usize, base: usize, map: &[u32], size: f32, out: &mut Vec<Glyph>) {
        let Some(part) = seg.get(from..to) else { return };
        let (base, seg) = (base + from, part);
        let ascii = seg.iter().all(|&u| (0x20..0x7F).contains(&u));
        let t = Instant::now();
        let runs = if ascii { vec![(0u32, seg.len() as u32, dw.latin_script())] } else { dw.analyze_script(seg, &q.locale) };
        self.stats.us_analyze += t.elapsed().as_micros() as u64;
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
                let t = Instant::now();
                let shaped = seg.get(s..e).and_then(|sub| dw.shape(sub, &entry.face, size, sa, &q.locale));
                self.stats.us_shape += t.elapsed().as_micros() as u64;
                match shaped {
                    Some(sh) => emit_run(out, fid, &sh, base + s, e - s, map),
                    None => notdef_run(out, fid, base + s, base + e, map, size),
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
        | 0x3099..=0x309A | 0xA960..=0xA97C | 0xD7B0..=0xD7FF | 0xFE00..=0xFE0F | 0xFE20..=0xFE2F | 0xE0100..=0xE01EF)
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
fn emit_run(out: &mut Vec<Glyph>, fid: u32, sh: &Shaped, start: usize, len: usize, map: &[u32]) {
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
                advance: sh.advances.get(g).copied().unwrap_or(0.0),
                offset_x: ox,
                offset_y: -oy,
                cluster_start: cs,
                cluster_end: ce,
            });
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

impl Platform for WinPlatform {
    fn decode_legacy(&self, encoding: &str, bytes: &[u8]) -> Option<String> {
        crate::encodings::decode_legacy(encoding, bytes)
    }

    fn resolve_font(&self, query: &FontQuery<'_>) -> Option<FontId> {
        let mut inn = self.inner.borrow_mut();
        let qi = inn.query_index(&self.dw, query);
        inn.queries.get(qi as usize)?.chain.first().map(|&f| FontId(f))
    }

    fn font_metrics(&self, font: FontId, size_px: f32) -> FontMetrics {
        let inn = self.inner.borrow();
        let Some(e) = inn.fonts.get(font.0 as usize) else { return FontMetrics::default() };
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
        if text.is_empty() {
            return Vec::new();
        }
        let size = if size_px.is_finite() { size_px.clamp(0.01, 4096.0) } else { 16.0 };
        let size_bits = size.to_bits();
        let mut inn = self.inner.borrow_mut();
        let qi = inn.query_index(&self.dw, query);
        let key = shape_hash(qi, size_bits, text);
        let hit = match inn.shape_cache.get(&key) {
            Some(e) if e.qi == qi && e.size_bits == size_bits && *e.text == *text => Some(Rc::clone(&e.glyphs)),
            _ => None,
        };
        if let Some(g) = hit {
            inn.stats.shape_hits += 1;
            return g.to_vec();
        }
        inn.stats.shape_misses += 1;
        let glyphs = inn.shape_uncached(&self.dw, text, qi, size);
        let cost = text.len() + glyphs.len() * GLYPH_COST + 96;
        inn.shape_cache.insert(key, ShapeEntry { qi, size_bits, text: text.into(), glyphs: glyphs.as_slice().into() }, cost);
        glyphs
    }

    fn line_breaks(&self, text: &str, lang: Option<&str>) -> Vec<usize> {
        if text.len() < 2 {
            return Vec::new();
        }
        let locale = {
            let inn = self.inner.borrow();
            wide_nul(&lang.and_then(normalize_lang).or_else(|| normalize_lang(&inn.default_locale)).unwrap_or_else(|| "en-US".to_string()))
        };
        let mut u16s: Vec<u16> = Vec::with_capacity(text.len());
        let mut map: Vec<u32> = Vec::with_capacity(text.len());
        for (bi, c) in text.char_indices() {
            let mut buf = [0u16; 2];
            for &u in c.encode_utf16(&mut buf).iter() {
                u16s.push(u);
                map.push(bi as u32);
            }
        }
        let bits = self.dw.analyze_line_breaks(&u16s, &locale);
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

    fn glyph_mask(&self, font: FontId, glyph: u32, size_px: f32, subpixel: u8) -> Option<GlyphMask> {
        let sub = subpixel.min(3);
        let size = if size_px.is_finite() { size_px.clamp(0.01, 1024.0) } else { return None };
        let g = u16::try_from(glyph).ok()?;
        let key = (font.0, glyph, size.to_bits(), sub);
        let mut inn = self.inner.borrow_mut();
        if let Some(v) = inn.glyph_cache.get(&key) {
            let r = v.as_ref().map(|m| (**m).clone());
            inn.stats.glyph_hits += 1;
            return r;
        }
        inn.stats.glyph_misses += 1;
        let entry = inn.fonts.get(font.0 as usize)?.clone();
        let mask = self.dw.rasterize(&entry.face, g, size, f32::from(sub) / 4.0).map(|raw| {
            // ClearType 3x1 gives one coverage value per colour stripe; the mean is plain grayscale.
            let alpha: Vec<u8> = raw.rgb.as_chunks::<3>().0.iter().map(|p| (p.iter().map(|&b| u32::from(b)).sum::<u32>() / 3) as u8).collect();
            GlyphMask { left: raw.left, top: raw.top, width: raw.width, height: raw.height, alpha }
        });
        match mask {
            Some(m) => {
                let cost = m.alpha.len() + 64;
                let out = m.clone();
                inn.glyph_cache.insert(key, Some(Rc::new(m)), cost);
                Some(out)
            }
            None => {
                inn.glyph_cache.insert(key, None, 64);
                None
            }
        }
    }
}
