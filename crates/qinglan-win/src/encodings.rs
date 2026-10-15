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

/// Bad bytes turn into U+FFFD, everything else decodes as usual. The input is cut into chunks at bytes
/// below 0x30, which are never part of a multi-byte character in any of these encodings (trail
/// bytes start at 0x30, and the 4-byte GB18030 form uses digits); a chunk that decodes cleanly goes
/// through `MultiByteToWideChar` in one call. A chunk with a bad byte is halved again and again, and
/// only the small piece around the bad byte is decoded one character at a time. (Per-character calls
/// for the whole page cost about seven times as much as one bulk call.)
fn decode_slow(t: &Entry, bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len());
    let mut failures = 0u32;
    let mut start = 0usize;
    while start < bytes.len() {
        let end = safe_cut(bytes, start, CHUNK);
        decode_range(t, bytes.get(start..end).unwrap_or(&[]), &mut out, &mut failures, 0);
        start = end;
    }
    out
}

/// Chunk size of the bulk decoding, and the piece size below which a bad chunk is decoded per character.
const CHUNK: usize = 16 * 1024;
const LEAF: usize = 512;
/// After this many failed bulk attempts the input is plainly broken: stop trying and decode per character.
const MAX_FAILURES: u32 = 256;

/// The first position at least `size` bytes after `start` that is a byte below 0x30 (or the end).
fn safe_cut(bytes: &[u8], start: usize, size: usize) -> usize {
    let from = start.saturating_add(size);
    match bytes.get(from..) {
        Some(rest) => rest.iter().position(|&b| b < 0x30).map_or(bytes.len(), |p| from + p),
        None => bytes.len(),
    }
}

fn decode_range(t: &Entry, chunk: &[u8], out: &mut String, failures: &mut u32, depth: u32) {
    if chunk.is_empty() {
        return;
    }
    if *failures < MAX_FAILURES {
        match sys::multi_byte_to_wide(t.code_page, sys::MB_ERR_INVALID_CHARS, chunk) {
            Ok(w) => {
                out.push_str(&String::from_utf16_lossy(&w));
                return;
            }
            Err(_) => *failures += 1,
        }
        if chunk.len() > LEAF && depth < 16 {
            let mid = safe_cut(chunk, 0, chunk.len() / 2);
            if mid > 0 && mid < chunk.len() {
                let (a, b) = chunk.split_at(mid);
                decode_range(t, a, out, failures, depth + 1);
                decode_range(t, b, out, failures, depth + 1);
                return;
            }
        }
    }
    decode_chars(t, chunk, out);
}

/// One character at a time, so that exactly the bad bytes turn into U+FFFD. WHATWG decoders drop
/// the lead byte of an invalid pair but give a following ASCII byte back; the same is done here.
fn decode_chars(t: &Entry, bytes: &[u8], out: &mut String) {
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
}

#[cfg(test)]
#[allow(clippy::indexing_slicing, clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    /// Reference: the whole input one character at a time.
    fn per_char(name: &str, bytes: &[u8]) -> String {
        let t = find(name).unwrap();
        let mut out = String::new();
        decode_chars(t, bytes, &mut out);
        out
    }

    fn gbk_text(n: usize) -> Vec<u8> {
        // "中文" and an ASCII tail with a space and a newline, repeated.
        b"\xD6\xD0\xCE\xC4 ab12\n".iter().copied().cycle().take(n).collect()
    }

    #[test]
    fn chunked_decoding_matches_the_per_character_reference() {
        let mut v = gbk_text(200_000);
        // Bad bytes of every kind: a lone 0xFF, a lead byte before ASCII (below and above 0x30), a bad pair, at the very start and end.
        for (i, b) in [(0usize, 0xFFu8), (17, 0xFF), (16_383, 0x81), (16_384, 0xFE), (50_001, 0xD6), (99_999, 0x80), (199_999, 0xFF)] {
            v[i] = b;
        }
        v.extend_from_slice(&[0x81]);
        for name in ["GBK", "gb18030", "Big5", "Shift_JIS", "EUC-KR", "EUC-JP"] {
            assert_eq!(decode_slow(find(name).unwrap(), &v), per_char(name, &v), "{name}");
        }
        // Mostly bad input falls back to per-character decoding and still agrees.
        let pat = gbk_text(10);
        let bad: Vec<u8> = (0..300_000u32).map(|i| if i % 7 == 0 { 0xFF } else { pat[(i % 10) as usize] }).collect();
        assert_eq!(decode_slow(find("GBK").unwrap(), &bad), per_char("GBK", &bad));
    }

    #[test]
    fn one_bad_byte_in_a_big_page_costs_about_a_bulk_decode() {
        let mut v = gbk_text(6_000_000);
        let clean = {
            let t0 = std::time::Instant::now();
            assert!(decode_legacy("GBK", &v).unwrap().chars().all(|c| c != '\u{FFFD}'));
            t0.elapsed()
        };
        v[3_000_004] = 0xFF;
        let t0 = std::time::Instant::now();
        let s = decode_legacy("GBK", &v).unwrap();
        let bad = t0.elapsed();
        assert_eq!(s.chars().filter(|&c| c == '\u{FFFD}').count(), 1);
        // Per-character decoding of the page took about seven times as long as the bulk decode.
        assert!(bad < clean * 4 + std::time::Duration::from_millis(100), "clean {clean:?}, with one bad byte {bad:?}");
    }
}
