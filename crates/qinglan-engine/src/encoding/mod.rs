//! Character encodings: labels (Encoding Standard 4.2), BOM sniffing and the `<meta>` prescan
//! (HTML 13.2.3.2), and decoding of page bytes into text.
//!
//! UTF-8, UTF-16, windows-1252 and x-user-defined are decoded here; every other encoding goes
//! through [`Platform::decode_legacy`]. See docs/decisions.md for the GB18030 fallback.

mod labels;

use crate::platform::Platform;

/// An encoding, identified by its WHATWG name (Encoding Standard 4.2), e.g. "UTF-8" or "GBK".
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Encoding(&'static str);

impl Encoding {
    pub const UTF_8: Encoding = Encoding("UTF-8");
    pub const UTF_16LE: Encoding = Encoding("UTF-16LE");
    pub const UTF_16BE: Encoding = Encoding("UTF-16BE");
    pub const WINDOWS_1252: Encoding = Encoding("windows-1252");
    pub const GB18030: Encoding = Encoding("gb18030");
    pub const X_USER_DEFINED: Encoding = Encoding("x-user-defined");
    pub const REPLACEMENT: Encoding = Encoding("replacement");

    pub fn name(self) -> &'static str {
        self.0
    }

    /// "Get an encoding" (Encoding Standard 4.2): strip ASCII whitespace, match ASCII
    /// case-insensitively against the label table.
    pub fn for_label(label: &str) -> Option<Encoding> {
        let label = label.trim_matches(|c| matches!(c, '\t' | '\n' | '\x0c' | '\r' | ' '));
        if label.is_empty() || label.len() > 32 {
            return None;
        }
        let lower = label.to_ascii_lowercase();
        let i = labels::LABELS.binary_search_by(|(l, _)| (*l).cmp(lower.as_str())).ok()?;
        labels::LABELS.get(i).map(|&(_, name)| Encoding(name))
    }

    fn is_utf16(self) -> bool {
        self == Self::UTF_16LE || self == Self::UTF_16BE
    }
}

/// How the encoding of a document was decided (HTML 13.2.3.2 "confidence").
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EncodingSource {
    Bom,
    TransportLayer,
    Meta,
    /// Nothing declared; UTF-8 was the default.
    DefaultUtf8,
    /// Nothing declared and the start of the page is not valid UTF-8: GB18030 (decisions.md).
    GuessedGb18030,
}

/// A decoded page.
#[derive(Debug)]
pub struct Decoded {
    pub text: String,
    pub encoding: Encoding,
    pub source: EncodingSource,
    /// Set when the encoding is not available and windows-1252 was used instead.
    pub warning: Option<String>,
}

/// BOM sniffing (HTML 13.2.3.2, Encoding Standard "BOM sniff"): the encoding and the BOM length.
pub fn sniff_bom(bytes: &[u8]) -> Option<(Encoding, usize)> {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        Some((Encoding::UTF_8, 3))
    } else if bytes.starts_with(&[0xFE, 0xFF]) {
        Some((Encoding::UTF_16BE, 2))
    } else if bytes.starts_with(&[0xFF, 0xFE]) {
        Some((Encoding::UTF_16LE, 2))
    } else {
        None
    }
}

fn is_space(b: u8) -> bool {
    matches!(b, 0x09 | 0x0A | 0x0C | 0x0D | 0x20)
}

/// "Prescan a byte stream to determine its encoding" (HTML 13.2.3.2), over the first 1024 bytes.
pub fn prescan(bytes: &[u8]) -> Option<Encoding> {
    let bytes = bytes.get(..1024.min(bytes.len())).unwrap_or(bytes);
    let at = |i: usize| bytes.get(i).copied();
    // Prescan for UTF-16 XML declarations.
    if bytes.starts_with(&[0x3C, 0x00, 0x3F, 0x00, 0x78, 0x00]) {
        return Some(Encoding::UTF_16LE);
    }
    if bytes.starts_with(&[0x00, 0x3C, 0x00, 0x3F, 0x00, 0x78]) {
        return Some(Encoding::UTF_16BE);
    }
    let mut pos = 0usize;
    while pos < bytes.len() {
        let rest = bytes.get(pos..).unwrap_or(&[]);
        if rest.starts_with(b"<!--") {
            // Advance to the first '>' preceded by "--" that comes after the '<'.
            let mut i = pos + 2; // the "--" of "<!--" may be shared with the closing "-->"
            loop {
                match at(i) {
                    None => return get_xml_encoding(bytes),
                    Some(b'>') if i >= pos + 4 && at(i - 1) == Some(b'-') && at(i - 2) == Some(b'-') => break,
                    _ => i += 1,
                }
            }
            pos = i;
        } else if rest.len() > 5 && rest.get(..5).is_some_and(|h| h.eq_ignore_ascii_case(b"<meta")) && matches!(rest.get(5), Some(&b) if is_space(b) || b == b'/')
        {
            pos += 5;
            let mut seen: Vec<Vec<u8>> = Vec::new();
            let mut got_pragma = false;
            let mut need_pragma: Option<bool> = None;
            let mut charset: Option<Option<Encoding>> = None; // None = null, Some(None) = failure
            loop {
                let Some((name, value, next)) = get_attribute(bytes, pos) else {
                    // Ran out of bytes inside the tag.
                    return get_xml_encoding(bytes);
                };
                pos = next;
                let Some(name) = name else { break };
                if seen.contains(&name) {
                    continue;
                }
                seen.push(name.clone());
                match name.as_slice() {
                    b"http-equiv" => {
                        if value == b"content-type" {
                            got_pragma = true;
                        }
                    }
                    b"content" => {
                        if let Some(enc) = extract_encoding_from_meta(&value)
                            && charset.is_none()
                        {
                            charset = Some(Some(enc));
                            need_pragma = Some(true);
                        }
                    }
                    b"charset" => {
                        charset = Some(std::str::from_utf8(&value).ok().and_then(Encoding::for_label));
                        need_pragma = Some(false);
                    }
                    _ => {}
                }
            }
            // Processing.
            let ok = match need_pragma {
                None => false,
                Some(true) => got_pragma,
                Some(false) => true,
            };
            if ok && let Some(Some(mut enc)) = charset {
                if enc.is_utf16() {
                    enc = Encoding::UTF_8;
                }
                if enc == Encoding::X_USER_DEFINED {
                    enc = Encoding::WINDOWS_1252;
                }
                return Some(enc);
            }
            // "Next byte".
        } else if rest.first() == Some(&b'<')
            && matches!(rest.get(1), Some(b) if b.is_ascii_alphabetic())
            || (rest.starts_with(b"</") && matches!(rest.get(2), Some(b) if b.is_ascii_alphabetic()))
        {
            pos += 1;
            if at(pos) == Some(b'/') {
                pos += 1;
            }
            while let Some(b) = at(pos) {
                if is_space(b) || b == b'>' {
                    break;
                }
                pos += 1;
            }
            loop {
                match get_attribute(bytes, pos) {
                    Some((Some(_), _, next)) => pos = next,
                    Some((None, _, next)) => {
                        pos = next;
                        break;
                    }
                    None => return get_xml_encoding(bytes),
                }
            }
            continue;
        } else if rest.starts_with(b"<!") || rest.starts_with(b"</") || rest.starts_with(b"<?") {
            let mut i = pos + 2;
            loop {
                match at(i) {
                    None => return get_xml_encoding(bytes),
                    Some(b'>') => break,
                    _ => i += 1,
                }
            }
            pos = i;
        }
        pos += 1;
    }
    get_xml_encoding(bytes)
}

/// "Get an attribute" (HTML 13.2.3.2). Returns (name or None if there is none, value, new position),
/// or `None` if the input ran out.
#[allow(clippy::type_complexity)]
fn get_attribute(bytes: &[u8], mut pos: usize) -> Option<(Option<Vec<u8>>, Vec<u8>, usize)> {
    let at = |i: usize| bytes.get(i).copied();
    loop {
        match at(pos) {
            None => return None,
            Some(b) if is_space(b) || b == b'/' => pos += 1,
            Some(b'>') => return Some((None, Vec::new(), pos)),
            Some(_) => break,
        }
    }
    let mut name = Vec::new();
    let mut value = Vec::new();
    // Attribute name.
    loop {
        let b = at(pos)?;
        if b == b'=' && !name.is_empty() {
            pos += 1;
            break;
        } else if is_space(b) {
            // Spaces.
            while at(pos).is_some_and(is_space) {
                pos += 1;
            }
            at(pos)?;
            if at(pos) != Some(b'=') {
                return Some((Some(name), value, pos));
            }
            pos += 1;
            break;
        } else if b == b'/' || b == b'>' {
            return Some((Some(name), value, pos));
        }
        name.push(b.to_ascii_lowercase());
        pos += 1;
    }
    // Value.
    while at(pos).is_some_and(is_space) {
        pos += 1;
    }
    let b = at(pos)?;
    if b == b'"' || b == b'\'' {
        loop {
            pos += 1;
            let c = at(pos)?;
            if c == b {
                pos += 1;
                return Some((Some(name), value, pos));
            }
            value.push(c.to_ascii_lowercase());
        }
    } else if b == b'>' {
        return Some((Some(name), value, pos));
    }
    loop {
        let c = at(pos)?;
        if is_space(c) || c == b'>' {
            return Some((Some(name), value, pos));
        }
        value.push(c.to_ascii_lowercase());
        pos += 1;
    }
}

/// "Extracting a character encoding from a meta element" (HTML 4.2.5.4 / 2.3.2 helper).
fn extract_encoding_from_meta(s: &[u8]) -> Option<Encoding> {
    let mut position = 0usize;
    loop {
        let rest = s.get(position..)?;
        let found = rest.windows(7).position(|w| w.eq_ignore_ascii_case(b"charset"))?;
        let mut i = position + found + 7;
        while s.get(i).is_some_and(|&b| is_space(b)) {
            i += 1;
        }
        if s.get(i) != Some(&b'=') {
            position = i; // "move position to point just before that next character"
            if i >= s.len() {
                return None;
            }
            continue;
        }
        i += 1;
        while s.get(i).is_some_and(|&b| is_space(b)) {
            i += 1;
        }
        let first = *s.get(i)?;
        let label: &[u8] = if first == b'"' || first == b'\'' {
            let body = s.get(i + 1..)?;
            let end = body.iter().position(|&b| b == first)?;
            body.get(..end)?
        } else {
            let body = s.get(i..)?;
            let end = body.iter().position(|&b| is_space(b) || b == b';').unwrap_or(body.len());
            body.get(..end)?
        };
        return std::str::from_utf8(label).ok().and_then(Encoding::for_label);
    }
}

/// "Get an XML encoding" (HTML 13.2.3.2).
fn get_xml_encoding(bytes: &[u8]) -> Option<Encoding> {
    if !bytes.starts_with(b"<?xml") {
        return None;
    }
    let end = bytes.iter().position(|&b| b == b'>')?;
    let decl = bytes.get(..end)?;
    let at = decl.windows(8).position(|w| w == b"encoding")? + 8;
    let mut i = at;
    while decl.get(i).is_some_and(|&b| b <= 0x20) {
        i += 1;
    }
    if decl.get(i) != Some(&b'=') {
        return None;
    }
    i += 1;
    while decl.get(i).is_some_and(|&b| b <= 0x20) {
        i += 1;
    }
    let quote = *decl.get(i)?;
    if quote != b'"' && quote != b'\'' {
        return None;
    }
    i += 1;
    let body = decl.get(i..)?;
    let len = body.iter().position(|&b| b == quote)?;
    let label = body.get(..len)?;
    if label.iter().any(|&b| b <= 0x20) {
        return None;
    }
    let enc = Encoding::for_label(std::str::from_utf8(label).ok()?)?;
    Some(if enc.is_utf16() { Encoding::UTF_8 } else { enc })
}

/// Decode a page (HTML 13.2.3.2 "determining the character encoding").
/// `transport_charset` is the `charset` parameter of the Content-Type header, if any.
pub fn decode_document(bytes: &[u8], transport_charset: Option<&str>, platform: &dyn Platform) -> Decoded {
    let (encoding, source, skip) = if let Some((enc, n)) = sniff_bom(bytes) {
        (enc, EncodingSource::Bom, n)
    } else if let Some(enc) = transport_charset.and_then(Encoding::for_label) {
        (enc, EncodingSource::TransportLayer, 0)
    } else if let Some(enc) = prescan(bytes) {
        (enc, EncodingSource::Meta, 0)
    } else if looks_like_utf8(bytes) {
        (Encoding::UTF_8, EncodingSource::DefaultUtf8, 0)
    } else {
        (Encoding::GB18030, EncodingSource::GuessedGb18030, 0)
    };
    let body = bytes.get(skip..).unwrap_or(&[]);
    let (text, warning) = decode_with(encoding, body, platform);
    Decoded { text, encoding, source, warning }
}

/// Is the start of the page (first 4 KB) valid UTF-8? A character cut by the 4 KB limit is not an error.
fn looks_like_utf8(bytes: &[u8]) -> bool {
    const LIMIT: usize = 4096;
    let head = bytes.get(..LIMIT.min(bytes.len())).unwrap_or(bytes);
    match std::str::from_utf8(head) {
        Ok(_) => true,
        Err(e) => e.error_len().is_none() && bytes.len() > LIMIT,
    }
}

/// Decode `bytes` (no BOM) with `encoding`. Bad bytes become U+FFFD.
/// The second value is a warning when the platform lacks the encoding and windows-1252 was used.
pub fn decode_with(encoding: Encoding, bytes: &[u8], platform: &dyn Platform) -> (String, Option<String>) {
    match encoding.name() {
        "UTF-8" => (String::from_utf8_lossy(bytes).into_owned(), None),
        "UTF-16LE" => (decode_utf16(bytes, false), None),
        "UTF-16BE" => (decode_utf16(bytes, true), None),
        "windows-1252" => (decode_windows_1252(bytes), None),
        "x-user-defined" => (bytes.iter().map(|&b| if b < 0x80 { b as char } else { char::from_u32(0xF780 + b as u32 - 0x80).unwrap_or('\u{FFFD}') }).collect(), None),
        "replacement" => ("\u{FFFD}".to_string(), None),
        name => match platform.decode_legacy(name, bytes) {
            Some(text) => (text, None),
            None => (
                decode_windows_1252(bytes),
                Some(format!("encoding {name} is not available on this platform; decoded as windows-1252")),
            ),
        },
    }
}

/// UTF-16 decoder (Encoding Standard 14.3): unpaired surrogates and a trailing odd byte become U+FFFD.
pub fn decode_utf16(bytes: &[u8], big_endian: bool) -> String {
    let (pairs, rest) = bytes.as_chunks::<2>();
    let units = pairs.iter().map(|&p| if big_endian { u16::from_be_bytes(p) } else { u16::from_le_bytes(p) });
    let mut out: String = char::decode_utf16(units).map(|r| r.unwrap_or('\u{FFFD}')).collect();
    if !rest.is_empty() {
        out.push('\u{FFFD}');
    }
    out
}

/// windows-1252 (Encoding Standard index single-byte "windows-1252").
fn decode_windows_1252(bytes: &[u8]) -> String {
    const HIGH: [u16; 32] = [
        0x20AC, 0x0081, 0x201A, 0x0192, 0x201E, 0x2026, 0x2020, 0x2021, 0x02C6, 0x2030, 0x0160, 0x2039, 0x0152, 0x008D, 0x017D,
        0x008F, 0x0090, 0x2018, 0x2019, 0x201C, 0x201D, 0x2022, 0x2013, 0x2014, 0x02DC, 0x2122, 0x0161, 0x203A, 0x0153, 0x009D,
        0x017E, 0x0178,
    ];
    bytes
        .iter()
        .map(|&b| match b {
            0x80..=0x9F => HIGH.get((b - 0x80) as usize).and_then(|&u| char::from_u32(u as u32)).unwrap_or('\u{FFFD}'),
            _ => b as char,
        })
        .collect()
}

/// Decode a style sheet (css-syntax-3 3.2 "determine the fallback encoding"): a BOM, then the transport
/// charset, then an `@charset` rule at the very start, then `fallback` (the encoding of the linking page).
pub fn decode_stylesheet(bytes: &[u8], transport_charset: Option<&str>, fallback: Encoding, platform: &dyn Platform) -> String {
    if let Some((enc, n)) = sniff_bom(bytes) {
        return decode_with(enc, bytes.get(n..).unwrap_or(&[]), platform).0;
    }
    let mut enc = transport_charset.and_then(Encoding::for_label);
    if enc.is_none() && bytes.starts_with(b"@charset \"") {
        let rest = bytes.get(10..).unwrap_or(&[]);
        if let Some(end) = rest.iter().take(64).position(|&b| b == b'"')
            && rest.get(end + 1) == Some(&b';')
            && let Some(label) = rest.get(..end).and_then(|l| std::str::from_utf8(l).ok())
            && let Some(e) = Encoding::for_label(label)
        {
            // A UTF-16 @charset is read as UTF-8 (the rule itself would be unreadable otherwise).
            enc = Some(if e.is_utf16() { Encoding::UTF_8 } else { e });
        }
    }
    decode_with(enc.unwrap_or(fallback), bytes, platform).0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::BasicPlatform;

    #[test]
    fn labels() {
        assert_eq!(Encoding::for_label(" GB2312 "), Some(Encoding("GBK")));
        assert_eq!(Encoding::for_label("latin1"), Some(Encoding::WINDOWS_1252));
        assert_eq!(Encoding::for_label("UTF8"), Some(Encoding::UTF_8));
        assert_eq!(Encoding::for_label("nonsense"), None);
    }

    #[test]
    fn prescan_cases() {
        assert_eq!(prescan(b"<html><head><meta charset=gbk>"), Some(Encoding("GBK")));
        assert_eq!(prescan(b"<META HTTP-EQUIV=\"Content-Type\" CONTENT=\"text/html; charset=Big5\">"), Some(Encoding("Big5")));
        // content without http-equiv is ignored
        assert_eq!(prescan(b"<meta content=\"text/html; charset=big5\">"), None);
        // a comment hides the meta
        assert_eq!(prescan(b"<!-- <meta charset=gbk> --><meta charset=utf-8>"), Some(Encoding::UTF_8));
        // UTF-16 in meta means UTF-8
        assert_eq!(prescan(b"<meta charset=utf-16le>"), Some(Encoding::UTF_8));
        assert_eq!(prescan(b"<?xml version=\"1.0\" encoding=\"gb2312\"?><html>"), Some(Encoding("GBK")));
        // beyond 1024 bytes: not seen
        let mut late = vec![b' '; 1100];
        late.extend_from_slice(b"<meta charset=gbk>");
        assert_eq!(prescan(&late), None);
    }

    #[test]
    fn order_of_sources() {
        let p = BasicPlatform;
        // BOM beats the header.
        let d = decode_document(b"\xEF\xBB\xBFhi", Some("gbk"), &p);
        assert_eq!((d.text.as_str(), d.source), ("hi", EncodingSource::Bom));
        // Header beats meta.
        let d = decode_document(b"<meta charset=windows-1252>\xE9", Some("utf-8"), &p);
        assert_eq!(d.source, EncodingSource::TransportLayer);
        assert!(d.text.ends_with('\u{FFFD}'));
        // Meta beats the default.
        let d = decode_document(b"<meta charset=windows-1252>\xE9", None, &p);
        assert_eq!(d.source, EncodingSource::Meta);
        assert!(d.text.ends_with('\u{e9}'));
        // Nothing declared, invalid UTF-8: GB18030 (not available in BasicPlatform: warning).
        let d = decode_document(b"<p>\xC4\xE3\xBA\xC3", None, &p);
        assert_eq!(d.source, EncodingSource::GuessedGb18030);
        assert!(d.warning.is_some());
        // Nothing declared, valid UTF-8.
        let d = decode_document("<p>你好".as_bytes(), None, &p);
        assert_eq!((d.text.as_str(), d.source), ("<p>你好", EncodingSource::DefaultUtf8));
    }

    #[test]
    fn stylesheets() {
        let p = BasicPlatform;
        let utf8 = Encoding::UTF_8;
        assert_eq!(decode_stylesheet(b"\xEF\xBB\xBFa{}", Some("windows-1252"), utf8, &p), "a{}");
        // @charset beats the fallback; the transport charset beats @charset.
        assert_eq!(decode_stylesheet(b"@charset \"windows-1252\"; a{x:\xE9}", None, utf8, &p), "@charset \"windows-1252\"; a{x:\u{e9}}");
        assert_eq!(decode_stylesheet(b"@charset \"windows-1252\"; a{x:\xE9}", Some("utf-8"), utf8, &p), "@charset \"windows-1252\"; a{x:\u{FFFD}}");
        assert_eq!(decode_stylesheet(b"a{x:\xE9}", None, Encoding::WINDOWS_1252, &p), "a{x:\u{e9}}");
    }

    #[test]
    fn utf16() {
        let d = decode_document(b"\xFF\xFE<\x00p\x00>\x00", None, &BasicPlatform);
        assert_eq!(d.text, "<p>");
        assert_eq!(decode_utf16(b"\xD8\x00\x00A\x00", true), "\u{FFFD}A\u{FFFD}");
    }
}
