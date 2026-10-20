//! DirectWrite: COM declarations written by hand from the SDK headers (`dwrite.h`, `dwrite_1.h`,
//! `dwrite_2.h`, Windows SDK 10.0.26100.0) and a small safe wrapper around them.
//!
//! Vtable slots are counted from the method order in those headers; `IUnknown` is slots 0 to 2.
//! Each vtable struct is declared only up to the last method we call, with `_padN` fields for the
//! slots in between. A wrong slot would call the wrong method, so the numbers in the comments are
//! the ones to check against the headers:
//!
//! * `IDWriteFactory` (dwrite.h): 3 GetSystemFontCollection, 21 CreateTextAnalyzer, 23
//!   CreateGlyphRunAnalysis. `IDWriteFactory1` adds 24, 25. `IDWriteFactory2` (dwrite_2.h): 26
//!   GetSystemFontFallback.
//! * `IDWriteFontCollection`: 4 GetFontFamily, 5 FindFamilyName.
//! * `IDWriteFontFamily` (derives IDWriteFontList 3 to 5): 6 GetFamilyNames, 7 GetFirstMatchingFont.
//! * `IDWriteFont`: 3 GetFontFamily, 10 GetSimulations, 13 CreateFontFace.
//! * `IDWriteFontFace`: 8 GetMetrics, 10 GetDesignGlyphMetrics, 11 GetGlyphIndices.
//! * `IDWriteLocalizedStrings`: 4 FindLocaleName, 7 GetStringLength, 8 GetString.
//! * `IDWriteTextAnalyzer`: 3 AnalyzeScript, 6 AnalyzeLineBreakpoints, 7 GetGlyphs, 8 GetGlyphPlacements.
//! * `IDWriteGlyphRunAnalysis`: 3 GetAlphaTextureBounds, 4 CreateAlphaTexture.
//! * `IDWriteFontFallback`: 3 MapCharacters.
//! * `IDWriteTextAnalysisSource` (we implement): 3 GetTextAtPosition, 4 GetTextBeforePosition, 5
//!   GetParagraphReadingDirection, 6 GetLocaleName, 7 GetNumberSubstitution.
//! * `IDWriteTextAnalysisSink` (we implement): 3 SetScriptAnalysis, 4 SetLineBreakpoints, 5
//!   SetBidiLevel, 6 SetNumberSubstitution.

use core::ffi::c_void;
use core::ptr::{NonNull, null, null_mut};

use super::{BOOL, Guid, HRESULT};

const S_OK: HRESULT = 0;
const E_NOINTERFACE: HRESULT = 0x8000_4002_u32 as i32;
const E_NOT_SUFFICIENT_BUFFER: HRESULT = 0x8007_007A_u32 as i32;

const IID_IUNKNOWN: Guid = Guid { d1: 0, d2: 0, d3: 0, d4: [0xC0, 0, 0, 0, 0, 0, 0, 0x46] };
const IID_IDWRITEFACTORY2: Guid = Guid { d1: 0x0439_fc60, d2: 0xca44, d3: 0x4994, d4: [0x8d, 0xee, 0x3a, 0x9a, 0xf7, 0xb7, 0x32, 0xec] };
const IID_IDWRITEFACTORY5: Guid = Guid { d1: 0x958d_b99a, d2: 0xbe2a, d3: 0x4f09, d4: [0xaf, 0x7d, 0x65, 0x18, 0x98, 0x03, 0xd1, 0xd3] };
const IID_IDWRITETEXTANALYSISSOURCE: Guid = Guid { d1: 0x688e_1a58, d2: 0x5094, d3: 0x47c8, d4: [0xad, 0xc8, 0xfb, 0xce, 0xa6, 0x0a, 0xe9, 0x2b] };
const IID_IDWRITETEXTANALYSISSINK: Guid = Guid { d1: 0x5810_cd44, d2: 0x0ca0, d3: 0x4701, d4: [0xb3, 0xfa, 0xbe, 0xc5, 0x18, 0x2a, 0xe4, 0xf6] };

#[link(name = "dwrite")]
unsafe extern "system" {
    fn DWriteCreateFactory(factory_type: u32, iid: *const Guid, factory: *mut *mut c_void) -> HRESULT;
}

type P = *mut c_void;

/// A COM object: a pointer to a vtable pointer.
#[repr(C)]
struct Raw<V> {
    vtbl: *const V,
}

#[repr(C)]
struct UnknownVtbl {
    query_interface: unsafe extern "system" fn(P, *const Guid, *mut P) -> HRESULT,
    add_ref: unsafe extern "system" fn(P) -> u32,
    release: unsafe extern "system" fn(P) -> u32,
}

type Out<V> = *mut *mut Raw<V>;

#[repr(C)]
struct FactoryVtbl {
    base: UnknownVtbl,
    get_system_font_collection: unsafe extern "system" fn(P, Out<CollectionVtbl>, BOOL) -> HRESULT, // 3
    _pad1a: [usize; 5],                                                                              // 4..=8
    create_font_face: unsafe extern "system" fn(P, u32, u32, *const P, u32, u32, Out<FaceVtbl>) -> HRESULT, // 9 CreateFontFace
    _pad1b: [usize; 3],                                                                              // 10..=12
    register_font_file_loader: unsafe extern "system" fn(P, P) -> HRESULT,                           // 13
    unregister_font_file_loader: unsafe extern "system" fn(P, P) -> HRESULT,                         // 14
    _pad1c: [usize; 6],                                                                              // 15..=20
    create_text_analyzer: unsafe extern "system" fn(P, Out<AnalyzerVtbl>) -> HRESULT,                // 21
    _pad2: usize,                                                                                    // 22
    create_glyph_run_analysis: unsafe extern "system" fn(P, *const GlyphRun, f32, *const c_void, u32, u32, f32, f32, Out<GlyphRunAnalysisVtbl>) -> HRESULT, // 23
    _pad3: [usize; 2],                                                                               // 24, 25
    get_system_font_fallback: unsafe extern "system" fn(P, Out<FallbackVtbl>) -> HRESULT,            // 26
}

/// `IDWriteFactory5` (dwrite_3.h): 44 CreateInMemoryFontFileLoader (IDWriteFactory3 adds 31 to 39, Factory4 40 to 43).
#[repr(C)]
struct Factory5Vtbl {
    base: UnknownVtbl,
    _pad1: [usize; 41],                                                                        // 3..=43
    create_in_memory_font_file_loader: unsafe extern "system" fn(P, Out<MemLoaderVtbl>) -> HRESULT, // 44
}

/// `IDWriteInMemoryFontFileLoader` (derives IDWriteFontFileLoader, whose only method is 3 CreateStreamFromKey):
/// 4 CreateInMemoryFontFileReference.
#[repr(C)]
struct MemLoaderVtbl {
    base: UnknownVtbl,
    _pad1: usize, // 3
    create_in_memory_font_file_reference: unsafe extern "system" fn(P, P, *const c_void, u32, P, Out<FontFileVtbl>) -> HRESULT, // 4
}

/// `IDWriteFontFile`: 3 GetReferenceKey, 4 GetLoader, 5 Analyze.
#[repr(C)]
struct FontFileVtbl {
    base: UnknownVtbl,
    _pad1: [usize; 2], // 3, 4
    analyze: unsafe extern "system" fn(P, *mut BOOL, *mut u32, *mut u32, *mut u32) -> HRESULT, // 5
}

#[repr(C)]
struct CollectionVtbl {
    base: UnknownVtbl,
    _pad1: usize,                                                                               // 3 GetFontFamilyCount
    get_font_family: unsafe extern "system" fn(P, u32, Out<FamilyVtbl>) -> HRESULT,             // 4
    find_family_name: unsafe extern "system" fn(P, *const u16, *mut u32, *mut BOOL) -> HRESULT, // 5
}

#[repr(C)]
struct FamilyVtbl {
    base: UnknownVtbl,
    _pad1: [usize; 3],                                                                              // 3 GetFontCollection, 4 GetFontCount, 5 GetFont
    get_family_names: unsafe extern "system" fn(P, Out<StringsVtbl>) -> HRESULT,                    // 6
    get_first_matching_font: unsafe extern "system" fn(P, u32, u32, u32, Out<FontVtbl>) -> HRESULT, // 7
}

#[repr(C)]
struct FontVtbl {
    base: UnknownVtbl,
    get_font_family: unsafe extern "system" fn(P, Out<FamilyVtbl>) -> HRESULT, // 3
    _pad1: [usize; 6],                                                         // 4..=9
    get_simulations: unsafe extern "system" fn(P) -> u32,                      // 10
    _pad2: [usize; 2],                                                         // 11 GetMetrics, 12 HasCharacter
    create_font_face: unsafe extern "system" fn(P, Out<FaceVtbl>) -> HRESULT,  // 13
}

#[repr(C)]
struct FaceVtbl {
    base: UnknownVtbl,
    _pad1: [usize; 5],                                                                                             // 3 GetType, 4 GetFiles, 5 GetIndex, 6 GetSimulations, 7 IsSymbolFont
    get_metrics: unsafe extern "system" fn(P, *mut FontMetricsRaw),                                                // 8
    _pad2: usize,                                                                                                  // 9 GetGlyphCount
    get_design_glyph_metrics: unsafe extern "system" fn(P, *const u16, u32, *mut GlyphMetricsRaw, BOOL) -> HRESULT, // 10
    get_glyph_indices: unsafe extern "system" fn(P, *const u32, u32, *mut u16) -> HRESULT,                         // 11
}

#[repr(C)]
struct StringsVtbl {
    base: UnknownVtbl,
    _pad1: usize,                                                                                // 3 GetCount
    find_locale_name: unsafe extern "system" fn(P, *const u16, *mut u32, *mut BOOL) -> HRESULT, // 4
    _pad2: [usize; 2],                                                                           // 5 GetLocaleNameLength, 6 GetLocaleName
    get_string_length: unsafe extern "system" fn(P, u32, *mut u32) -> HRESULT,                   // 7
    get_string: unsafe extern "system" fn(P, u32, *mut u16, u32) -> HRESULT,                     // 8
}

#[repr(C)]
struct AnalyzerVtbl {
    base: UnknownVtbl,
    analyze_script: unsafe extern "system" fn(P, P, u32, u32, P) -> HRESULT,           // 3
    _pad1: [usize; 2],                                                                 // 4 AnalyzeBidi, 5 AnalyzeNumberSubstitution
    analyze_line_breakpoints: unsafe extern "system" fn(P, P, u32, u32, P) -> HRESULT, // 6
    get_glyphs: unsafe extern "system" fn(
        P,
        *const u16, // textString
        u32,        // textLength
        P,          // fontFace
        BOOL,       // isSideways
        BOOL,       // isRightToLeft
        *const ScriptAnalysis,
        *const u16, // localeName
        P,          // numberSubstitution
        *const P,   // features
        *const u32, // featureRangeLengths
        u32,        // featureRanges
        u32,        // maxGlyphCount
        *mut u16,   // clusterMap
        *mut u16,   // textProps
        *mut u16,   // glyphIndices
        *mut u16,   // glyphProps
        *mut u32,   // actualGlyphCount
    ) -> HRESULT, // 7
    get_glyph_placements: unsafe extern "system" fn(
        P,
        *const u16, // textString
        *const u16, // clusterMap
        *mut u16,   // textProps
        u32,        // textLength
        *const u16, // glyphIndices
        *const u16, // glyphProps
        u32,        // glyphCount
        P,          // fontFace
        f32,        // fontEmSize
        BOOL,       // isSideways
        BOOL,       // isRightToLeft
        *const ScriptAnalysis,
        *const u16, // localeName
        *const P,   // features
        *const u32, // featureRangeLengths
        u32,        // featureRanges
        *mut f32,   // glyphAdvances
        *mut GlyphOffset,
    ) -> HRESULT, // 8
}

#[repr(C)]
struct GlyphRunAnalysisVtbl {
    base: UnknownVtbl,
    get_alpha_texture_bounds: unsafe extern "system" fn(P, u32, *mut Rect) -> HRESULT,             // 3
    create_alpha_texture: unsafe extern "system" fn(P, u32, *const Rect, *mut u8, u32) -> HRESULT, // 4
}

#[repr(C)]
struct FallbackVtbl {
    base: UnknownVtbl,
    map_characters: unsafe extern "system" fn(P, P, u32, u32, P, *const u16, u32, u32, u32, *mut u32, Out<FontVtbl>, *mut f32) -> HRESULT, // 3
}

#[repr(C)]
struct FontMetricsRaw {
    design_units_per_em: u16,
    ascent: u16,
    descent: u16,
    line_gap: i16,
    cap_height: u16,
    x_height: u16,
    underline_position: i16,
    underline_thickness: u16,
    strikethrough_position: i16,
    strikethrough_thickness: u16,
}

#[repr(C)]
#[derive(Default)]
struct GlyphMetricsRaw {
    left_side_bearing: i32,
    advance_width: u32,
    right_side_bearing: i32,
    top_side_bearing: i32,
    advance_height: u32,
    bottom_side_bearing: i32,
    vertical_origin_y: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct GlyphOffset {
    advance_offset: f32,
    ascender_offset: f32,
}

#[repr(C)]
struct GlyphRun {
    font_face: P,
    font_em_size: f32,
    glyph_count: u32,
    glyph_indices: *const u16,
    glyph_advances: *const f32,
    glyph_offsets: *const GlyphOffset,
    is_sideways: BOOL,
    bidi_level: u32,
}

#[repr(C)]
#[derive(Default)]
struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

/// `DWRITE_SCRIPT_ANALYSIS`.
#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct ScriptAnalysis {
    pub script: u16,
    pub shapes: u32,
}

macro_rules! vcall {
    ($obj:expr, $m:ident $(, $a:expr)* $(,)?) => {{
        let o = &$obj;
        let f = (*(*o.p.as_ptr()).vtbl).$m;
        f(o.raw() $(, $a)*)
    }};
}

/// An owned COM interface pointer; releases on drop.
///
/// Invariant: `p` points to a live COM object whose first field is a pointer to a vtable that
/// starts with the three `IUnknown` methods and has at least the slots `V` declares.
struct ComPtr<V> {
    p: NonNull<Raw<V>>,
}

impl<V> ComPtr<V> {
    /// # Safety
    /// `p` is null or a valid interface pointer whose reference the caller hands over.
    unsafe fn from_raw(p: *mut Raw<V>) -> Option<Self> {
        NonNull::new(p).map(|p| ComPtr { p })
    }
    fn raw(&self) -> P {
        self.p.as_ptr().cast()
    }
    fn addr(&self) -> usize {
        self.p.as_ptr() as usize
    }
}

impl<V> Drop for ComPtr<V> {
    fn drop(&mut self) {
        // SAFETY: by the invariant every vtable starts with IUnknown's methods; we own one reference.
        unsafe {
            let vt = (*self.p.as_ptr()).vtbl as *const UnknownVtbl;
            ((*vt).release)(self.raw());
        }
    }
}

fn to_wide_nul(s: &str) -> Vec<u16> {
    let mut v: Vec<u16> = s.encode_utf16().collect();
    v.push(0);
    v
}

/// Font metrics in design units (`DWRITE_FONT_METRICS`).
#[derive(Clone, Copy, Debug, Default)]
pub struct DesignMetrics {
    pub units_per_em: f32,
    pub ascent: f32,
    pub descent: f32,
    pub line_gap: f32,
    pub x_height: f32,
    pub underline_position: f32,
    pub underline_thickness: f32,
    pub strikethrough_position: f32,
    pub strikethrough_thickness: f32,
}

/// One shaped run, straight from `GetGlyphs` and `GetGlyphPlacements`.
pub struct Shaped {
    pub glyphs: Vec<u16>,
    /// For every UTF-16 unit of the run, the index of the first glyph of its cluster.
    pub cluster_map: Vec<u16>,
    pub advances: Vec<f32>,
    /// (advanceOffset, ascenderOffset): right and up are positive.
    pub offsets: Vec<(f32, f32)>,
}

/// A glyph rendered with ClearType 3x1 coverage (three bytes per pixel: R, G, B).
pub struct RawMask {
    pub left: i32,
    pub top: i32,
    pub width: u32,
    pub height: u32,
    pub rgb: Vec<u8>,
}

pub struct Family {
    raw: ComPtr<FamilyVtbl>,
}

pub struct Font {
    raw: ComPtr<FontVtbl>,
}

pub struct Face {
    raw: ComPtr<FaceVtbl>,
}

pub struct Collection {
    raw: ComPtr<CollectionVtbl>,
}

fn hr_ok(hr: HRESULT) -> bool {
    hr >= 0
}

/// Read string `index` of an `IDWriteLocalizedStrings`.
fn strings_get(s: &ComPtr<StringsVtbl>, index: u32) -> Option<String> {
    let mut len = 0u32;
    // SAFETY: `s` is a live IDWriteLocalizedStrings; `len` is a valid out pointer.
    let hr = unsafe { vcall!(s, get_string_length, index, &mut len) };
    if !hr_ok(hr) || len > 4096 {
        return None;
    }
    let mut buf = vec![0u16; len as usize + 1];
    // SAFETY: `buf` has room for `len` units plus the terminator, as GetString requires.
    let hr = unsafe { vcall!(s, get_string, index, buf.as_mut_ptr(), len + 1) };
    if !hr_ok(hr) {
        return None;
    }
    buf.truncate(len as usize);
    Some(String::from_utf16_lossy(&buf))
}

impl Family {
    /// The family's name in en-US (or its first name).
    pub fn name(&self) -> Option<String> {
        let mut out: *mut Raw<StringsVtbl> = null_mut();
        // SAFETY: live family, valid out pointer.
        let hr = unsafe { vcall!(self.raw, get_family_names, &mut out) };
        if !hr_ok(hr) {
            return None;
        }
        // SAFETY: on success `out` holds a reference we own.
        let strings = unsafe { ComPtr::from_raw(out) }?;
        let loc = to_wide_nul("en-us");
        let (mut idx, mut exists) = (0u32, 0 as BOOL);
        // SAFETY: `loc` is NUL-terminated; out pointers are valid.
        let hr = unsafe { vcall!(strings, find_locale_name, loc.as_ptr(), &mut idx, &mut exists) };
        if !hr_ok(hr) || exists == 0 {
            idx = 0;
        }
        strings_get(&strings, idx)
    }

    /// `GetFirstMatchingFont`: weight 1..=999, style 0 normal / 1 oblique / 2 italic, stretch normal.
    pub fn first_matching_font(&self, weight: u32, style: u32) -> Option<Font> {
        let mut out: *mut Raw<FontVtbl> = null_mut();
        // SAFETY: live family, valid out pointer.
        let hr = unsafe { vcall!(self.raw, get_first_matching_font, weight, 5, style, &mut out) };
        if !hr_ok(hr) {
            return None;
        }
        // SAFETY: on success `out` holds a reference we own.
        unsafe { ComPtr::from_raw(out) }.map(|raw| Font { raw })
    }
}

impl Collection {
    /// Index of the family called `name` (any of its localized names), if the system has it.
    pub fn find_family(&self, name: &str) -> Option<u32> {
        let w = to_wide_nul(name);
        let (mut idx, mut exists) = (0u32, 0 as BOOL);
        // SAFETY: `w` is NUL-terminated; out pointers are valid; the collection is live.
        let hr = unsafe { vcall!(self.raw, find_family_name, w.as_ptr(), &mut idx, &mut exists) };
        (hr_ok(hr) && exists != 0).then_some(idx)
    }

    pub fn family(&self, index: u32) -> Option<Family> {
        let mut out: *mut Raw<FamilyVtbl> = null_mut();
        // SAFETY: live collection, valid out pointer.
        let hr = unsafe { vcall!(self.raw, get_font_family, index, &mut out) };
        if !hr_ok(hr) {
            return None;
        }
        // SAFETY: on success `out` holds a reference we own.
        unsafe { ComPtr::from_raw(out) }.map(|raw| Family { raw })
    }
}

impl Font {
    /// `DWRITE_FONT_SIMULATIONS` bits: 1 bold, 2 oblique.
    pub fn simulations(&self) -> u32 {
        // SAFETY: live font.
        unsafe { vcall!(self.raw, get_simulations) }
    }

    pub fn family_name(&self) -> Option<String> {
        let mut out: *mut Raw<FamilyVtbl> = null_mut();
        // SAFETY: live font, valid out pointer.
        let hr = unsafe { vcall!(self.raw, get_font_family, &mut out) };
        if !hr_ok(hr) {
            return None;
        }
        // SAFETY: on success `out` holds a reference we own.
        let raw = unsafe { ComPtr::from_raw(out) }?;
        Family { raw }.name()
    }
}

impl Face {
    /// Address of the COM object: equal for two handles to the same face.
    pub fn addr(&self) -> usize {
        self.raw.addr()
    }

    pub fn metrics(&self) -> DesignMetrics {
        let mut m = FontMetricsRaw {
            design_units_per_em: 0,
            ascent: 0,
            descent: 0,
            line_gap: 0,
            cap_height: 0,
            x_height: 0,
            underline_position: 0,
            underline_thickness: 0,
            strikethrough_position: 0,
            strikethrough_thickness: 0,
        };
        // SAFETY: live face; `m` is a valid DWRITE_FONT_METRICS out buffer.
        unsafe { vcall!(self.raw, get_metrics, &mut m) };
        DesignMetrics {
            units_per_em: f32::from(m.design_units_per_em),
            ascent: f32::from(m.ascent),
            descent: f32::from(m.descent),
            line_gap: f32::from(m.line_gap),
            x_height: f32::from(m.x_height),
            underline_position: f32::from(m.underline_position),
            underline_thickness: f32::from(m.underline_thickness),
            strikethrough_position: f32::from(m.strikethrough_position),
            strikethrough_thickness: f32::from(m.strikethrough_thickness),
        }
    }

    /// Glyph index of each code point (0 = the font has none).
    pub fn glyph_indices(&self, cps: &[u32]) -> Option<Vec<u16>> {
        let mut out = vec![0u16; cps.len()];
        // SAFETY: live face; `cps` and `out` both hold `cps.len()` elements.
        let hr = unsafe { vcall!(self.raw, get_glyph_indices, cps.as_ptr(), cps.len() as u32, out.as_mut_ptr()) };
        hr_ok(hr).then_some(out)
    }

    /// Advance width of `glyph` in design units.
    pub fn design_advance(&self, glyph: u16) -> Option<u32> {
        let mut m = GlyphMetricsRaw::default();
        // SAFETY: live face; one glyph index in, one metrics struct out.
        let hr = unsafe { vcall!(self.raw, get_design_glyph_metrics, &glyph, 1, &mut m, 0) };
        hr_ok(hr).then_some(m.advance_width)
    }
}

// ---------------------------------------------------------------------------------------------
// Our own COM objects: IDWriteTextAnalysisSource and the sink. They live on the caller's stack for
// the duration of one synchronous DirectWrite call, so reference counting is a no-op.
// ---------------------------------------------------------------------------------------------

#[repr(C)]
struct SourceVtbl {
    base: UnknownVtbl,
    get_text_at_position: unsafe extern "system" fn(P, u32, *mut *const u16, *mut u32) -> HRESULT,
    get_text_before_position: unsafe extern "system" fn(P, u32, *mut *const u16, *mut u32) -> HRESULT,
    get_paragraph_reading_direction: unsafe extern "system" fn(P) -> u32,
    get_locale_name: unsafe extern "system" fn(P, u32, *mut u32, *mut *const u16) -> HRESULT,
    get_number_substitution: unsafe extern "system" fn(P, u32, *mut u32, *mut P) -> HRESULT,
}

#[repr(C)]
struct Source {
    vtbl: *const SourceVtbl,
    text: *const u16,
    len: u32,
    /// NUL-terminated.
    locale: *const u16,
}

unsafe extern "system" fn static_add_ref(_this: P) -> u32 {
    1
}
unsafe extern "system" fn static_release(_this: P) -> u32 {
    1
}

unsafe extern "system" fn source_qi(this: P, iid: *const Guid, out: *mut P) -> HRESULT {
    // SAFETY: DirectWrite passes valid, non-null pointers (checked anyway).
    unsafe {
        if iid.is_null() || out.is_null() {
            return E_NOINTERFACE;
        }
        if *iid == IID_IUNKNOWN || *iid == IID_IDWRITETEXTANALYSISSOURCE {
            *out = this;
            S_OK
        } else {
            *out = null_mut();
            E_NOINTERFACE
        }
    }
}

unsafe extern "system" fn source_text_at(this: P, pos: u32, text: *mut *const u16, len: *mut u32) -> HRESULT {
    // SAFETY: `this` is the `Source` we passed to DirectWrite, alive for the call; out pointers are valid.
    unsafe {
        let s = &*(this as *const Source);
        if pos < s.len {
            *text = s.text.add(pos as usize);
            *len = s.len - pos;
        } else {
            *text = null();
            *len = 0;
        }
    }
    S_OK
}

unsafe extern "system" fn source_text_before(this: P, pos: u32, text: *mut *const u16, len: *mut u32) -> HRESULT {
    // SAFETY: as for `source_text_at`.
    unsafe {
        let s = &*(this as *const Source);
        if pos > 0 && pos <= s.len {
            *text = s.text;
            *len = pos;
        } else {
            *text = null();
            *len = 0;
        }
    }
    S_OK
}

unsafe extern "system" fn source_direction(_this: P) -> u32 {
    0 // DWRITE_READING_DIRECTION_LEFT_TO_RIGHT
}

unsafe extern "system" fn source_locale(this: P, pos: u32, len: *mut u32, locale: *mut *const u16) -> HRESULT {
    // SAFETY: as for `source_text_at`.
    unsafe {
        let s = &*(this as *const Source);
        *locale = s.locale;
        *len = s.len.saturating_sub(pos);
    }
    S_OK
}

unsafe extern "system" fn source_number_subst(this: P, pos: u32, len: *mut u32, subst: *mut P) -> HRESULT {
    // SAFETY: as for `source_text_at`.
    unsafe {
        let s = &*(this as *const Source);
        *subst = null_mut();
        *len = s.len.saturating_sub(pos);
    }
    S_OK
}

static SOURCE_VTBL: SourceVtbl = SourceVtbl {
    base: UnknownVtbl { query_interface: source_qi, add_ref: static_add_ref, release: static_release },
    get_text_at_position: source_text_at,
    get_text_before_position: source_text_before,
    get_paragraph_reading_direction: source_direction,
    get_locale_name: source_locale,
    get_number_substitution: source_number_subst,
};

impl Source {
    /// `locale` must be NUL-terminated and outlive the value.
    fn new(text: &[u16], locale: &[u16]) -> Source {
        Source { vtbl: &SOURCE_VTBL, text: text.as_ptr(), len: text.len() as u32, locale: locale.as_ptr() }
    }
}

#[repr(C)]
struct SinkVtbl {
    base: UnknownVtbl,
    set_script_analysis: unsafe extern "system" fn(P, u32, u32, *const ScriptAnalysis) -> HRESULT,
    set_line_breakpoints: unsafe extern "system" fn(P, u32, u32, *const u8) -> HRESULT,
    set_bidi_level: unsafe extern "system" fn(P, u32, u32, u8, u8) -> HRESULT,
    set_number_substitution: unsafe extern "system" fn(P, u32, u32, P) -> HRESULT,
}

/// Collects what `AnalyzeScript` and `AnalyzeLineBreakpoints` report.
#[repr(C)]
struct Sink {
    vtbl: *const SinkVtbl,
    runs: Vec<(u32, u32, ScriptAnalysis)>,
    /// One `DWRITE_LINE_BREAKPOINT` byte per UTF-16 unit.
    breaks: Vec<u8>,
}

unsafe extern "system" fn sink_qi(this: P, iid: *const Guid, out: *mut P) -> HRESULT {
    // SAFETY: as for `source_qi`.
    unsafe {
        if iid.is_null() || out.is_null() {
            return E_NOINTERFACE;
        }
        if *iid == IID_IUNKNOWN || *iid == IID_IDWRITETEXTANALYSISSINK {
            *out = this;
            S_OK
        } else {
            *out = null_mut();
            E_NOINTERFACE
        }
    }
}

unsafe extern "system" fn sink_script(this: P, pos: u32, len: u32, a: *const ScriptAnalysis) -> HRESULT {
    // SAFETY: `this` is the `Sink` we passed in and nothing else uses it during the call; `a` is valid.
    unsafe {
        let s = &mut *(this as *mut Sink);
        if s.runs.len() < 1 << 20 {
            s.runs.push((pos, len, *a));
        }
    }
    S_OK
}

unsafe extern "system" fn sink_breaks(this: P, pos: u32, len: u32, bp: *const u8) -> HRESULT {
    // SAFETY: as above; `bp` points to `len` DWRITE_LINE_BREAKPOINT bytes.
    unsafe {
        let s = &mut *(this as *mut Sink);
        for i in 0..len as usize {
            if let Some(dst) = s.breaks.get_mut(pos as usize + i) {
                *dst = *bp.add(i);
            }
        }
    }
    S_OK
}

unsafe extern "system" fn sink_bidi(_this: P, _pos: u32, _len: u32, _explicit: u8, _resolved: u8) -> HRESULT {
    S_OK
}

unsafe extern "system" fn sink_number_subst(_this: P, _pos: u32, _len: u32, _subst: P) -> HRESULT {
    S_OK
}

static SINK_VTBL: SinkVtbl = SinkVtbl {
    base: UnknownVtbl { query_interface: sink_qi, add_ref: static_add_ref, release: static_release },
    set_script_analysis: sink_script,
    set_line_breakpoints: sink_breaks,
    set_bidi_level: sink_bidi,
    set_number_substitution: sink_number_subst,
};

impl Sink {
    fn new(text_len: usize, want_breaks: bool) -> Sink {
        Sink { vtbl: &SINK_VTBL, runs: Vec::new(), breaks: if want_breaks { vec![0; text_len] } else { Vec::new() } }
    }
}

/// Whether a line may end between two UTF-16 units, from the `DWRITE_LINE_BREAKPOINT` bytes of the
/// unit before and the unit after: break conditions are 0 neutral, 1 can break, 2 may not break,
/// 3 must break; "after" is bits 2..4 of the previous unit, "before" bits 0..2 of the next.
pub fn break_between(prev: u8, next: u8) -> bool {
    let after = (prev >> 2) & 3;
    let before = next & 3;
    if after == 2 || before == 2 {
        return false;
    }
    after == 1 || after == 3 || before == 1 || before == 3
}

/// What [`warm`] keeps alive: the factory (with its system collection, fallback and analyzer) and the faces
/// it opened, so that DirectWrite's shared caches stay filled for the platform created later.
pub struct Warm {
    _factory: Factory,
    _faces: Vec<Face>,
}

// SAFETY: DirectWrite objects are free-threaded (the documentation of IDWriteFactory, IDWriteFontFace,
// IDWriteFontCollection, IDWriteTextAnalyzer and IDWriteFontFallback says they may be used from any thread).
// `Warm` is never used after it is built; it only holds the references so that the objects stay alive.
unsafe impl Send for Warm {}
// SAFETY: as for `Send`: shared references to a `Warm` give no access to its contents at all.
unsafe impl Sync for Warm {}

/// One font to open in [`warm`]: family name, weight, and the sample text that is shaped (and a glyph of it
/// drawn) so that the shaping tables are read.
pub struct WarmFont<'a> {
    pub family: &'a str,
    pub weight: u32,
    pub sample: &'a str,
    pub locale: &'a str,
}

/// Create the factory, the system collection, the fallback and the text analyzer, then open every font
/// of `fonts` that is installed, shape its sample text and draw one of its glyphs. Returns the objects (keep
/// them) and how many faces were opened. `None` when DirectWrite is not available.
pub fn warm(fonts: &[WarmFont<'_>]) -> Option<(Warm, usize)> {
    let f = Factory::new()?;
    let mut faces = Vec::new();
    for wf in fonts {
        let Some(fi) = f.collection().find_family(wf.family) else { continue };
        let Some(font) = f.collection().family(fi).and_then(|fam| fam.first_matching_font(wf.weight, 0)) else { continue };
        let Some(face) = f.create_face(&font) else { continue };
        let text: Vec<u16> = wf.sample.encode_utf16().collect();
        let locale = to_wide_nul(wf.locale);
        if let Some(&(start, len, sa)) = f.analyze_script(&text, &locale).first()
            && let Some(run) = text.get(start as usize..(start + len) as usize)
        {
            let _ = f.shape(run, &face, 16.0, sa, &locale);
        }
        if let Some(c) = wf.sample.chars().find(|c| !c.is_whitespace())
            && let Some(g) = face.glyph_indices(&[c as u32]).and_then(|v| v.first().copied())
        {
            let _ = f.rasterize(&face, g, 16.0, 0.0);
        }
        faces.push(face);
    }
    // The system fallback's first use loads its mapping tables (a Han character with the Chinese locale).
    let han: Vec<u16> = "\u{4e2d}\u{6587}".encode_utf16().collect();
    let _ = f.map_characters(&han, &to_wide_nul("zh-Hans"), &to_wide_nul("Segoe UI"), 400, 0);
    let n = faces.len();
    Some((Warm { _factory: f, _faces: faces }, n))
}

/// `QueryInterface` for another interface of the same object.
fn query<V, W>(p: &ComPtr<V>, iid: &Guid) -> Option<ComPtr<W>> {
    let mut out: P = null_mut();
    // SAFETY: by `ComPtr`'s invariant the vtable starts with IUnknown's methods; `iid` and `out` are valid.
    let hr = unsafe {
        let vt = (*p.p.as_ptr()).vtbl as *const UnknownVtbl;
        ((*vt).query_interface)(p.raw(), iid, &mut out)
    };
    if !hr_ok(hr) {
        return None;
    }
    // SAFETY: on success `out` is a new reference to the interface asked for.
    unsafe { ComPtr::from_raw(out.cast()) }
}

/// Fonts that came with a page, made from memory with `IDWriteInMemoryFontFileLoader` (Windows 10 1809 and later).
/// The loader is registered with the factory for as long as this value lives; the font bytes are kept here too, so
/// they outlive every face made from them (the loader may use them in place). Drop the faces first, then this.
pub struct WebFontLoader {
    factory: ComPtr<FactoryVtbl>,
    loader: ComPtr<MemLoaderVtbl>,
    data: Vec<Vec<u8>>,
}

impl WebFontLoader {
    /// `None` when the system has no `IDWriteFactory5`.
    pub fn new(dw: &Factory) -> Option<WebFontLoader> {
        let f5: ComPtr<Factory5Vtbl> = query(&dw.factory, &IID_IDWRITEFACTORY5)?;
        let mut out: *mut Raw<MemLoaderVtbl> = null_mut();
        // SAFETY: live IDWriteFactory5, valid out pointer.
        let hr = unsafe { vcall!(f5, create_in_memory_font_file_loader, &mut out) };
        if !hr_ok(hr) {
            return None;
        }
        // SAFETY: owned reference on success.
        let loader = unsafe { ComPtr::from_raw(out) }?;
        let factory: ComPtr<FactoryVtbl> = query(&dw.factory, &IID_IDWRITEFACTORY2)?;
        // SAFETY: both are live; the in-memory loader derives from IDWriteFontFileLoader, which is what is registered.
        let hr = unsafe { vcall!(factory, register_font_file_loader, loader.raw()) };
        if !hr_ok(hr) {
            return None;
        }
        Some(WebFontLoader { factory, loader, data: Vec::new() })
    }

    /// A font face for the font file `bytes` (an sfnt), or `None` when DirectWrite does not accept it.
    pub fn create_face(&mut self, bytes: Vec<u8>) -> Option<Face> {
        let len = u32::try_from(bytes.len()).ok()?;
        self.data.push(bytes);
        let face = self.make_face(len);
        if face.is_none() {
            self.data.pop();
        }
        face
    }

    fn make_face(&self, len: u32) -> Option<Face> {
        let data = self.data.last()?;
        let mut file: *mut Raw<FontFileVtbl> = null_mut();
        // SAFETY: the loader and factory are live; `data` stays where it is as long as `self` lives (the buffer of
        // a `Vec` does not move when the outer vector grows); a null owner means the loader keeps no reference.
        let hr = unsafe { vcall!(self.loader, create_in_memory_font_file_reference, self.factory.raw(), data.as_ptr().cast(), len, null_mut(), &mut file) };
        if !hr_ok(hr) {
            return None;
        }
        // SAFETY: owned reference on success.
        let file = unsafe { ComPtr::from_raw(file) }?;
        let (mut supported, mut file_type, mut face_type, mut faces) = (0 as BOOL, 0u32, 0u32, 0u32);
        // SAFETY: live font file, valid out pointers.
        let hr = unsafe { vcall!(file, analyze, &mut supported, &mut file_type, &mut face_type, &mut faces) };
        if !hr_ok(hr) || supported == 0 || faces == 0 {
            return None;
        }
        let files = [file.raw()];
        let mut out: *mut Raw<FaceVtbl> = null_mut();
        // SAFETY: one live font file; face 0, no simulations.
        let hr = unsafe { vcall!(self.factory, create_font_face, face_type, 1, files.as_ptr(), 0, 0, &mut out) };
        if !hr_ok(hr) {
            return None;
        }
        // SAFETY: owned reference on success.
        unsafe { ComPtr::from_raw(out) }.map(|raw| Face { raw })
    }
}

impl Drop for WebFontLoader {
    fn drop(&mut self) {
        // SAFETY: both pointers are live; this undoes the registration made in `new`.
        unsafe {
            vcall!(self.factory, unregister_font_file_loader, self.loader.raw());
        }
    }
}

/// The DirectWrite objects the platform needs.
pub struct Factory {
    factory: ComPtr<FactoryVtbl>,
    collection: Collection,
    fallback: ComPtr<FallbackVtbl>,
    analyzer: ComPtr<AnalyzerVtbl>,
    latin: ScriptAnalysis,
}

impl Factory {
    pub fn new() -> Option<Factory> {
        let mut out: P = null_mut();
        // SAFETY: valid IID and out pointers; type 0 is DWRITE_FACTORY_TYPE_SHARED.
        let hr = unsafe { DWriteCreateFactory(0, &IID_IDWRITEFACTORY2, &mut out) };
        if !hr_ok(hr) {
            return None;
        }
        // SAFETY: on success `out` holds an IDWriteFactory2 reference we own.
        let factory: ComPtr<FactoryVtbl> = unsafe { ComPtr::from_raw(out.cast()) }?;

        let mut coll: *mut Raw<CollectionVtbl> = null_mut();
        // SAFETY: live factory, valid out pointer.
        let hr = unsafe { vcall!(factory, get_system_font_collection, &mut coll, 0) };
        if !hr_ok(hr) {
            return None;
        }
        // SAFETY: owned reference on success.
        let collection = Collection { raw: unsafe { ComPtr::from_raw(coll) }? };

        let mut fb: *mut Raw<FallbackVtbl> = null_mut();
        // SAFETY: live IDWriteFactory2, valid out pointer.
        let hr = unsafe { vcall!(factory, get_system_font_fallback, &mut fb) };
        if !hr_ok(hr) {
            return None;
        }
        // SAFETY: owned reference on success.
        let fallback = unsafe { ComPtr::from_raw(fb) }?;

        let mut an: *mut Raw<AnalyzerVtbl> = null_mut();
        // SAFETY: live factory, valid out pointer.
        let hr = unsafe { vcall!(factory, create_text_analyzer, &mut an) };
        if !hr_ok(hr) {
            return None;
        }
        // SAFETY: owned reference on success.
        let analyzer = unsafe { ComPtr::from_raw(an) }?;

        let mut f = Factory { factory, collection, fallback, analyzer, latin: ScriptAnalysis::default() };
        let locale = to_wide_nul("en-US");
        let a: Vec<u16> = vec![u16::from(b'a')];
        f.latin = f.analyze_script(&a, &locale).first().map(|r| r.2)?;
        Some(f)
    }

    /// The script analysis of plain Latin text (used for printable ASCII without asking).
    pub fn latin_script(&self) -> ScriptAnalysis {
        self.latin
    }

    pub fn collection(&self) -> &Collection {
        &self.collection
    }

    pub fn create_face(&self, font: &Font) -> Option<Face> {
        let mut out: *mut Raw<FaceVtbl> = null_mut();
        // SAFETY: live font, valid out pointer.
        let hr = unsafe { vcall!(font.raw, create_font_face, &mut out) };
        if !hr_ok(hr) {
            return None;
        }
        // SAFETY: owned reference on success.
        unsafe { ComPtr::from_raw(out) }.map(|raw| Face { raw })
    }

    /// `IDWriteFontFallback::MapCharacters` on `text` (UTF-16) with `locale` (NUL-terminated) and
    /// the base family `base_family` (NUL-terminated). Returns the mapped length and the font,
    /// which is `None` when no installed font has the first character.
    pub fn map_characters(&self, text: &[u16], locale: &[u16], base_family: &[u16], weight: u32, style: u32) -> Option<(u32, Option<Font>)> {
        if text.is_empty() || base_family.last() != Some(&0) || locale.last() != Some(&0) {
            return None;
        }
        let mut src = Source::new(text, locale);
        let (mut mapped, mut scale) = (0u32, 1.0f32);
        let mut out: *mut Raw<FontVtbl> = null_mut();
        // SAFETY: `src` is a valid IDWriteTextAnalysisSource for the duration of the call; the
        // string arguments are NUL-terminated; a null base collection means the system collection.
        let hr = unsafe {
            vcall!(self.fallback, map_characters, (&raw mut src).cast(), 0, text.len() as u32, null_mut(), base_family.as_ptr(), weight, style, 5, &mut mapped, &mut out, &mut scale)
        };
        if !hr_ok(hr) {
            return None;
        }
        // SAFETY: `out` is null or an owned reference.
        let font = unsafe { ComPtr::from_raw(out) }.map(|raw| Font { raw });
        Some((mapped, font))
    }

    /// Script runs of `text` as `(start, length, analysis)` in UTF-16 units.
    pub fn analyze_script(&self, text: &[u16], locale: &[u16]) -> Vec<(u32, u32, ScriptAnalysis)> {
        if text.is_empty() || locale.last() != Some(&0) {
            return Vec::new();
        }
        let mut src = Source::new(text, locale);
        let mut sink = Sink::new(0, false);
        // SAFETY: `src` and `sink` are COM objects whose layouts start with a vtable pointer that
        // matches IDWriteTextAnalysisSource / Sink; both outlive the synchronous call.
        let hr = unsafe { vcall!(self.analyzer, analyze_script, (&raw mut src).cast(), 0, text.len() as u32, (&raw mut sink).cast()) };
        if hr_ok(hr) { sink.runs } else { Vec::new() }
    }

    /// One `DWRITE_LINE_BREAKPOINT` byte per UTF-16 unit of `text` (empty on failure).
    pub fn analyze_line_breaks(&self, text: &[u16], locale: &[u16]) -> Vec<u8> {
        if text.is_empty() || locale.last() != Some(&0) {
            return Vec::new();
        }
        let mut src = Source::new(text, locale);
        let mut sink = Sink::new(text.len(), true);
        // SAFETY: as for `analyze_script`.
        let hr = unsafe { vcall!(self.analyzer, analyze_line_breakpoints, (&raw mut src).cast(), 0, text.len() as u32, (&raw mut sink).cast()) };
        if hr_ok(hr) { sink.breaks } else { Vec::new() }
    }

    /// Shape one run of one script with one font. `locale` is NUL-terminated.
    pub fn shape(&self, text: &[u16], face: &Face, size: f32, sa: ScriptAnalysis, locale: &[u16]) -> Option<Shaped> {
        let n = text.len();
        if n == 0 || n > (1 << 20) || locale.last() != Some(&0) {
            return None;
        }
        let mut max = n * 3 / 2 + 16;
        loop {
            let mut cluster = vec![0u16; n];
            let mut tprops = vec![0u16; n];
            let mut gidx = vec![0u16; max];
            let mut gprops = vec![0u16; max];
            let mut actual = 0u32;
            // SAFETY: live analyzer and face; every buffer has the element count the method expects
            // (`n` for the per-character ones, `max` for the per-glyph ones); no features are passed.
            let hr = unsafe {
                vcall!(
                    self.analyzer,
                    get_glyphs,
                    text.as_ptr(),
                    n as u32,
                    face.raw.raw(),
                    0,
                    0,
                    &sa,
                    locale.as_ptr(),
                    null_mut(),
                    null(),
                    null(),
                    0,
                    max as u32,
                    cluster.as_mut_ptr(),
                    tprops.as_mut_ptr(),
                    gidx.as_mut_ptr(),
                    gprops.as_mut_ptr(),
                    &mut actual
                )
            };
            if hr == E_NOT_SUFFICIENT_BUFFER {
                max *= 2;
                if max > 16 * n + 1024 {
                    return None;
                }
                continue;
            }
            if !hr_ok(hr) {
                return None;
            }
            let g = (actual as usize).min(max);
            gidx.truncate(g);
            gprops.truncate(g);
            let mut advances = vec![0f32; g];
            let mut offs = vec![GlyphOffset::default(); g];
            // SAFETY: as above; `advances` and `offs` hold `g` elements, `gidx`/`gprops` hold `g`.
            let hr = unsafe {
                vcall!(
                    self.analyzer,
                    get_glyph_placements,
                    text.as_ptr(),
                    cluster.as_ptr(),
                    tprops.as_mut_ptr(),
                    n as u32,
                    gidx.as_ptr(),
                    gprops.as_ptr(),
                    g as u32,
                    face.raw.raw(),
                    size,
                    0,
                    0,
                    &sa,
                    locale.as_ptr(),
                    null(),
                    null(),
                    0,
                    advances.as_mut_ptr(),
                    offs.as_mut_ptr()
                )
            };
            if !hr_ok(hr) {
                return None;
            }
            let offsets = offs.iter().map(|o| (o.advance_offset, o.ascender_offset)).collect();
            return Some(Shaped { glyphs: gidx, cluster_map: cluster, advances, offsets });
        }
    }

    /// Render one glyph with natural-symmetric anti-aliasing, the pen at `origin_x` (fractional
    /// pixels) on the baseline. `None` when the glyph draws nothing.
    pub fn rasterize(&self, face: &Face, glyph: u16, size: f32, origin_x: f32) -> Option<RawMask> {
        if !(size > 0.0 && size <= 1024.0) {
            return None;
        }
        let run = GlyphRun { font_face: face.raw.raw(), font_em_size: size, glyph_count: 1, glyph_indices: &glyph, glyph_advances: null(), glyph_offsets: null(), is_sideways: 0, bidi_level: 0 };
        let mut out: *mut Raw<GlyphRunAnalysisVtbl> = null_mut();
        // SAFETY: `run` and the glyph index it points to are valid for the call; null advances and
        // offsets are allowed; null transform is the identity. Rendering mode 5 is NATURAL_SYMMETRIC,
        // measuring mode 0 is NATURAL.
        let hr = unsafe { vcall!(self.factory, create_glyph_run_analysis, &run, 1.0, null(), 5, 0, origin_x, 0.0, &mut out) };
        if !hr_ok(hr) {
            return None;
        }
        // SAFETY: owned reference on success.
        let analysis = unsafe { ComPtr::from_raw(out) }?;
        let mut rect = Rect::default();
        // SAFETY: live analysis; texture type 1 is DWRITE_TEXTURE_CLEARTYPE_3x1.
        let hr = unsafe { vcall!(analysis, get_alpha_texture_bounds, 1, &mut rect) };
        if !hr_ok(hr) {
            return None;
        }
        let (w, h) = (i64::from(rect.right) - i64::from(rect.left), i64::from(rect.bottom) - i64::from(rect.top));
        if w <= 0 || h <= 0 || w > 4096 || h > 4096 {
            return None;
        }
        let mut rgb = vec![0u8; (w * h * 3) as usize];
        // SAFETY: `rgb` holds exactly 3 bytes for each pixel of `rect`, the size passed.
        let hr = unsafe { vcall!(analysis, create_alpha_texture, 1, &rect, rgb.as_mut_ptr(), rgb.len() as u32) };
        if !hr_ok(hr) {
            return None;
        }
        Some(RawMask { left: rect.left, top: rect.top, width: w as u32, height: h as u32, rgb })
    }
}
