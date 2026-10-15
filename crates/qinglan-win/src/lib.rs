//! Windows platform layer for qinglan: DirectWrite fonts, text shaping, line breaking and glyph
//! masks, and legacy text encodings through `MultiByteToWideChar` (step 1b-3); the WinHTTP transport
//! and WIC image decoding (step 1c-1). The window arrives in step 1c-2.
//!
//! `unsafe` is only allowed under `sys/`; everything else in this crate is safe Rust on top of the
//! safe wrappers `sys` exports.
#![deny(unsafe_code)]

mod encodings;
mod fx;
mod images;
mod lru;
mod net;
mod platform;
#[allow(unsafe_code)]
mod sys;

pub use encodings::{code_page_for, decode_legacy};
pub use images::WicDecoder;
pub use net::{WinHttpConfig, WinHttpTransport, accept_language, map_error};
pub use platform::{CacheStats, WinPlatform};

/// The user's display languages in order of preference (`zh-CN`, `en-US` ...).
pub fn ui_languages() -> Vec<String> {
    sys::user_ui_languages()
}

/// Write `bgra` (premultiplied BGRA, `width * 4` bytes per row, top row first) as a PNG file at `path`,
/// through WIC. Transparent pixels come out white.
pub fn write_png(path: &std::path::Path, width: u32, height: u32, bgra: &[u8]) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    sys::wic::write_png(&wide, width, height, bgra).map_err(|hr| format!("cannot write {}: error 0x{:08X}", path.display(), hr as u32))
}

/// The fonts the prewarm opens (L1 task 17): the common ones of the Windows platform's font choice, regular and
/// bold, each with a sample that makes DirectWrite read its shaping tables.
const LATIN_SAMPLE: &str = "The quick brown fox jumps over the lazy dog 0123456789 (fi) .,;:";
const HAN_SAMPLE: &str = "\u{4e2d}\u{6587}\u{7ef4}\u{57fa}\u{767e}\u{79d1}\u{ff0c}\u{4eba}\u{6c11}\u{5171}\u{548c}\u{56fd}\u{3002} Wikipedia 2026";

static WARM: std::sync::OnceLock<Option<sys::dwrite::Warm>> = std::sync::OnceLock::new();

/// Warm up DirectWrite (L1 task 17, "startup prewarm"): create the factory, the system font collection and the
/// fallback, and open the common fonts (Segoe UI, Microsoft YaHei, SimSun, Consolas, Times New Roman, regular and
/// bold) with their shaping tables read. Call it from a background thread right after the window is up; it does not
/// touch a [`WinPlatform`], and a platform created before, during or after it just finds DirectWrite's shared
/// caches filled. Returns the number of font faces opened (0 when DirectWrite is missing or this is not the first
/// call). Content that arrives before it is done is laid out at once; nothing waits for it.
pub fn prewarm() -> usize {
    let mut opened = 0;
    WARM.get_or_init(|| {
        let mut fonts = Vec::new();
        for (family, sample, locale) in [
            ("Segoe UI", LATIN_SAMPLE, "en-US"),
            ("Microsoft YaHei", HAN_SAMPLE, "zh-Hans"),
            ("SimSun", HAN_SAMPLE, "zh-Hans"),
            ("Consolas", LATIN_SAMPLE, "en-US"),
            ("Times New Roman", LATIN_SAMPLE, "en-US"),
        ] {
            for weight in [400, 700] {
                fonts.push(sys::dwrite::WarmFont { family, weight, sample, locale });
            }
        }
        let (warm, n) = sys::dwrite::warm(&fonts)?;
        opened = n;
        Some(warm)
    });
    opened
}
