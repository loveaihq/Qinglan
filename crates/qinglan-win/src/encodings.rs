//! Legacy text encodings through `MultiByteToWideChar` (L1 task 4, step 1b-3).
//!
//! The engine hands over a WHATWG encoding name ("GBK", "Big5", "Shift_JIS", ...); the table here
//! maps it to a Windows code page. Bad bytes become U+FFFD: the Windows function would otherwise put
//! a default character (U+30FB or '?') in their place, so decoding first tries the whole input with
//! `MB_ERR_INVALID_CHARS` and, only when that fails, redoes it one character at a time.

use crate::sys;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    /// One byte per character.
    Sbcs,
    /// One or two bytes per character.
    Dbcs,
    /// One, two or four bytes per character (GB18030).
    Gb18030,
    /// Escape-sequence encodings (ISO-2022-JP): Windows accepts no flags for these.
    Stateful,
}

struct Entry {
    /// WHATWG name, lower case.
    name: &'static str,
    code_page: u32,
    kind: Kind,
}

const fn e(name: &'static str, code_page: u32, kind: Kind) -> Entry {
    Entry { name, code_page, kind }
}

/// WHATWG encoding name (Encoding Standard 4.2) to Windows code page. `gb2312`, `gbk` and `gb18030`
/// all use 54936 (task sheet); `euc-jp` uses 20932 because `MultiByteToWideChar` rejects 51932
/// (checked: ERROR_INVALID_PARAMETER); Windows has no code page for ISO-8859-10, -14 and -16, which
/// are left out so the engine falls back to windows-1252 with a warning.
static TABLE: &[Entry] = &[
    e("ibm866", 866, Kind::Sbcs),
    e("iso-8859-2", 28592, Kind::Sbcs),
    e("iso-8859-3", 28593, Kind::Sbcs),
    e("iso-8859-4", 28594, Kind::Sbcs),
    e("iso-8859-5", 28595, Kind::Sbcs),
    e("iso-8859-6", 28596, Kind::Sbcs),
    e("iso-8859-7", 28597, Kind::Sbcs),
    e("iso-8859-8", 28598, Kind::Sbcs),
    e("iso-8859-8-i", 38598, Kind::Sbcs),
    e("iso-8859-13", 28603, Kind::Sbcs),
    e("iso-8859-15", 28605, Kind::Sbcs),
    e("koi8-r", 20866, Kind::Sbcs),
    e("koi8-u", 21866, Kind::Sbcs),
    e("macintosh", 10000, Kind::Sbcs),
    e("windows-874", 874, Kind::Sbcs),
    e("windows-1250", 1250, Kind::Sbcs),
    e("windows-1251", 1251, Kind::Sbcs),
    e("windows-1252", 1252, Kind::Sbcs),
    e("windows-1253", 1253, Kind::Sbcs),
    e("windows-1254", 1254, Kind::Sbcs),
    e("windows-1255", 1255, Kind::Sbcs),
    e("windows-1256", 1256, Kind::Sbcs),
    e("windows-1257", 1257, Kind::Sbcs),
    e("windows-1258", 1258, Kind::Sbcs),
    e("x-mac-cyrillic", 10007, Kind::Sbcs),
    e("gbk", 54936, Kind::Gb18030),
    e("gb2312", 54936, Kind::Gb18030),
    e("gb18030", 54936, Kind::Gb18030),
    e("big5", 950, Kind::Dbcs),
    e("euc-jp", 20932, Kind::Dbcs),
    e("iso-2022-jp", 50220, Kind::Stateful),
    e("shift_jis", 932, Kind::Dbcs),
    e("euc-kr", 949, Kind::Dbcs),
];

fn find(encoding: &str) -> Option<&'static Entry> {
    TABLE.iter().find(|t| t.name.eq_ignore_ascii_case(encoding.trim()))
}

/// The Windows code page for a WHATWG encoding name, if Windows has one.
pub fn code_page_for(encoding: &str) -> Option<u32> {
    find(encoding).map(|t| t.code_page)
}

/// Decode `bytes` from the legacy encoding `encoding`; `None` when Windows has no code page for it.
/// Bad bytes become U+FFFD.
pub fn decode_legacy(encoding: &str, bytes: &[u8]) -> Option<String> {
    let t = find(encoding)?;
    if bytes.is_empty() {
        return Some(String::new());
    }
    if t.kind == Kind::Stateful {
        return Some(decode_unflagged(t, bytes));
    }
    match sys::multi_byte_to_wide(t.code_page, sys::MB_ERR_INVALID_CHARS, bytes) {
        Ok(w) => Some(String::from_utf16_lossy(&w)),
        Err(sys::ERROR_INVALID_FLAGS) => Some(decode_unflagged(t, bytes)),
        Err(_) => Some(decode_slow(t, bytes)),
    }
}

/// Whole input, no flags (the code page does not support `MB_ERR_INVALID_CHARS`).
fn decode_unflagged(t: &Entry, bytes: &[u8]) -> String {
    match sys::multi_byte_to_wide(t.code_page, 0, bytes) {
        Ok(w) => String::from_utf16_lossy(&w),
        Err(_) => "\u{FFFD}".repeat(bytes.len().min(1 << 20)),
    }
}

/// One character at a time, so that exactly the bad bytes turn into U+FFFD. WHATWG decoders drop
/// the lead byte of an invalid pair but give a following ASCII byte back; the same is done here.
fn decode_slow(t: &Entry, bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len());
    let mut i = 0usize;
    while let Some(&b) = bytes.get(i) {
        if b < 0x80 {
            out.push(char::from(b));
            i += 1;
            continue;
        }
        let next = bytes.get(i + 1).copied();
        let candidates: &[usize] = match t.kind {
            Kind::Dbcs => &[2, 1],
            Kind::Gb18030 if next.is_some_and(|n| (0x30..=0x39).contains(&n)) => &[4, 2, 1],
            Kind::Gb18030 => &[2, 1],
            Kind::Sbcs | Kind::Stateful => &[1],
        };
        let mut used = 0usize;
        for &l in candidates {
            let Some(chunk) = bytes.get(i..i + l) else { continue };
            if let Ok(w) = sys::multi_byte_to_wide(t.code_page, sys::MB_ERR_INVALID_CHARS, chunk) {
                let s = String::from_utf16_lossy(&w);
                if s.chars().count() == 1 {
                    out.push_str(&s);
                    used = l;
                    break;
                }
            }
        }
        if used == 0 {
            out.push('\u{FFFD}');
            used = if sys::is_dbcs_lead_byte(t.code_page, b) && next.is_some_and(|n| n >= 0x80) { 2 } else { 1 };
        }
        i += used;
    }
    out
}
