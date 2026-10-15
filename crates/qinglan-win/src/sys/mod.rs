//! Hand-written Windows declarations (no `windows` / `windows-sys` crates). This is the only place
//! in the workspace where `unsafe` is allowed; every block says why it is sound.
//!
//! The rest of the crate sees only the safe wrappers exported from here.
#![allow(non_snake_case, non_camel_case_types, clippy::upper_case_acronyms)]

pub mod alloc;
pub mod dwrite;
pub mod wic;
pub mod winhttp;
pub mod window;

pub type HRESULT = i32;
pub type BOOL = i32;

/// A COM interface id (`GUID`).
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Guid {
    pub d1: u32,
    pub d2: u16,
    pub d3: u16,
    pub d4: [u8; 8],
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn MultiByteToWideChar(code_page: u32, flags: u32, src: *const u8, src_len: i32, dst: *mut u16, dst_len: i32) -> i32;
    fn GetUserDefaultLocaleName(buf: *mut u16, cch: i32) -> i32;
    fn GetUserPreferredUILanguages(flags: u32, num: *mut u32, buf: *mut u16, len: *mut u32) -> BOOL;
    fn GetLastError() -> u32;
    fn IsDBCSLeadByteEx(code_page: u32, b: u8) -> BOOL;
}

/// `MB_ERR_INVALID_CHARS`: fail instead of substituting a default character.
pub const MB_ERR_INVALID_CHARS: u32 = 0x8;
/// `ERROR_INVALID_FLAGS`.
pub const ERROR_INVALID_FLAGS: u32 = 1004;

/// `MultiByteToWideChar` for the whole of `bytes`. `Err` carries `GetLastError()` (for example
/// `ERROR_NO_UNICODE_TRANSLATION` = 1113 when `MB_ERR_INVALID_CHARS` is set and a byte is bad).
pub fn multi_byte_to_wide(code_page: u32, flags: u32, bytes: &[u8]) -> Result<Vec<u16>, u32> {
    if bytes.is_empty() {
        return Ok(Vec::new());
    }
    let Ok(n) = i32::try_from(bytes.len()) else { return Err(0) };
    // Every input byte yields at most one UTF-16 unit (a 4-byte GB18030 sequence yields two), so
    // `bytes.len()` units are always enough.
    let mut out = vec![0u16; bytes.len()];
    // SAFETY: `bytes` is valid for `n` bytes and `out` for `n` units; both live across the call.
    let r = unsafe { MultiByteToWideChar(code_page, flags, bytes.as_ptr(), n, out.as_mut_ptr(), n) };
    if r <= 0 {
        // SAFETY: GetLastError has no preconditions.
        return Err(unsafe { GetLastError() });
    }
    out.truncate(r as usize);
    Ok(out)
}

/// Whether `b` is a lead byte of a double-byte character in `code_page`.
pub fn is_dbcs_lead_byte(code_page: u32, b: u8) -> bool {
    // SAFETY: plain value arguments.
    unsafe { IsDBCSLeadByteEx(code_page, b) != 0 }
}

/// `MUI_LANGUAGE_NAME`: language names ("zh-CN") instead of identifiers.
const MUI_LANGUAGE_NAME: u32 = 0x8;

/// The user's preferred display languages in order ("zh-CN", "en-US" ...); the user locale when Windows
/// does not say.
pub fn user_ui_languages() -> Vec<String> {
    let (mut num, mut len) = (0u32, 0u32);
    // SAFETY: a null buffer asks only for the size; `num` and `len` are valid out values.
    let ok = unsafe { GetUserPreferredUILanguages(MUI_LANGUAGE_NAME, &mut num, core::ptr::null_mut(), &mut len) };
    if ok != 0 && len > 0 && len <= 4096 {
        let mut buf = vec![0u16; len as usize];
        // SAFETY: `buf` holds `len` units, the size Windows just asked for; `num` and `len` are valid.
        let ok = unsafe { GetUserPreferredUILanguages(MUI_LANGUAGE_NAME, &mut num, buf.as_mut_ptr(), &mut len) };
        if ok != 0 {
            // A double-NUL-terminated list of NUL-terminated names.
            let langs: Vec<String> = buf.split(|&u| u == 0).filter(|s| !s.is_empty()).map(String::from_utf16_lossy).collect();
            if !langs.is_empty() {
                return langs;
            }
        }
    }
    vec![user_locale_name()]
}

/// The user's default locale name ("zh-CN", "en-US" ...), or "en-US" when Windows does not say.
pub fn user_locale_name() -> String {
    let mut buf = [0u16; 86];
    // SAFETY: `buf` is valid for 86 units, the documented maximum (LOCALE_NAME_MAX_LENGTH).
    let n = unsafe { GetUserDefaultLocaleName(buf.as_mut_ptr(), buf.len() as i32) };
    if n <= 1 {
        return "en-US".to_string();
    }
    let units = buf.get(..(n as usize - 1)).unwrap_or(&[]);
    String::from_utf16_lossy(units)
}
