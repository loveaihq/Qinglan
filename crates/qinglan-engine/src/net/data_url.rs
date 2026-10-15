//! `data:` URLs (Fetch "data: URL processor", RFC 2397), decoded by hand: percent-decoding and base64
//! are our own (L1 task 13), so a page cannot reach any other code through them.

use crate::url::{Url, percent_decode};

#[derive(Debug, PartialEq, Eq)]
pub struct DataUrl {
    /// The media type as written (with parameters), or `text/plain;charset=US-ASCII` when empty.
    pub mime: String,
    pub body: Vec<u8>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum DataUrlError {
    /// No comma, or a bad base64 body.
    Malformed,
    TooLarge,
}

/// Decode the `data:` URL `url`; the body may not be longer than `max_bytes`.
pub fn decode(url: &Url, max_bytes: usize) -> Result<DataUrl, DataUrlError> {
    let full = url.serialize(true);
    let rest = full.strip_prefix("data:").ok_or(DataUrlError::Malformed)?;
    let (header, payload) = rest.split_once(',').ok_or(DataUrlError::Malformed)?;
    let mut mime = header.trim_matches(|c: char| c.is_ascii_whitespace()).to_string();
    // A trailing `;base64` (any case, spaces allowed around it) switches the payload to base64.
    let mut is_base64 = false;
    if mime.len() >= 6 {
        let (head, tail) = mime.split_at(mime.len() - 6);
        if tail.eq_ignore_ascii_case("base64") {
            let head_trim = head.trim_end_matches(|c: char| c.is_ascii_whitespace());
            if let Some(h) = head_trim.strip_suffix(';') {
                is_base64 = true;
                mime = h.trim_end_matches(|c: char| c.is_ascii_whitespace()).to_string();
            }
        }
    }
    if mime.is_empty() || mime.starts_with(';') {
        mime = format!("text/plain{mime};charset=US-ASCII");
    }
    // Check the size before allocating: percent-decoding never grows the data, and base64 shrinks it
    // to three quarters.
    let bound = if is_base64 { payload.len() / 4 * 3 + 3 } else { payload.len() };
    if bound > max_bytes && !is_base64 {
        return Err(DataUrlError::TooLarge);
    }
    if is_base64 && payload.len() / 4 * 3 > max_bytes.saturating_add(3) {
        return Err(DataUrlError::TooLarge);
    }
    let raw = percent_decode(payload.as_bytes());
    let body = if is_base64 { forgiving_base64(&raw).ok_or(DataUrlError::Malformed)? } else { raw };
    if body.len() > max_bytes {
        return Err(DataUrlError::TooLarge);
    }
    Ok(DataUrl { mime, body })
}

fn sextet(b: u8) -> Option<u32> {
    match b {
        b'A'..=b'Z' => Some(u32::from(b - b'A')),
        b'a'..=b'z' => Some(u32::from(b - b'a') + 26),
        b'0'..=b'9' => Some(u32::from(b - b'0') + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

/// Infra Standard "forgiving-base64 decode": ASCII white space is dropped, one or two trailing `=`
/// are allowed when the length is a multiple of four, a length of 1 mod 4 or any other character fails.
pub fn forgiving_base64(input: &[u8]) -> Option<Vec<u8>> {
    let mut data: Vec<u8> = input.iter().copied().filter(|b| !b.is_ascii_whitespace()).collect();
    if data.len().is_multiple_of(4) {
        for _ in 0..2 {
            if data.last() == Some(&b'=') {
                data.pop();
            }
        }
    }
    if data.len() % 4 == 1 {
        return None;
    }
    let mut out = Vec::with_capacity(data.len() / 4 * 3 + 2);
    for chunk in data.chunks(4) {
        let mut acc = 0u32;
        for &b in chunk {
            acc = (acc << 6) | sextet(b)?;
        }
        acc <<= 6 * (4 - chunk.len()) as u32;
        let bytes = acc.to_be_bytes();
        // chunk.len() 4 -> 3 bytes, 3 -> 2 bytes, 2 -> 1 byte
        out.extend(bytes.get(1..chunk.len()).unwrap_or(&[]));
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> Result<DataUrl, DataUrlError> {
        decode(&Url::parse(s, None).expect("valid url"), 1 << 20)
    }

    #[test]
    fn plain_percent_and_base64() {
        assert_eq!(d("data:text/plain,hello%20world").map(|x| x.body), Ok(b"hello world".to_vec()));
        let x = d("data:text/css;charset=utf-8;base64,Ym9keXtjb2xvcjpyZWR9").expect("ok");
        assert_eq!(x.body, b"body{color:red}");
        assert_eq!(x.mime, "text/css;charset=utf-8");
        assert_eq!(d("data:;base64,QQ").map(|x| x.body), Ok(b"A".to_vec())); // no padding needed
        assert_eq!(d("data:text/plain;BASE64,QUI=").map(|x| x.body), Ok(b"AB".to_vec()));
        assert_eq!(d("data:,a?b=c").map(|x| x.body), Ok(b"a?b=c".to_vec()), "the query is part of the data");
        assert_eq!(d("data:,x").map(|x| x.mime), Ok("text/plain;charset=US-ASCII".to_string()));
    }

    #[test]
    fn malformed_and_too_large() {
        assert_eq!(d("data:text/plain"), Err(DataUrlError::Malformed));
        assert_eq!(d("data:;base64,A"), Err(DataUrlError::Malformed));
        assert_eq!(d("data:;base64,@@@@"), Err(DataUrlError::Malformed));
        let big = format!("data:;base64,{}", "AAAA".repeat(1000));
        assert_eq!(decode(&Url::parse(&big, None).expect("url"), 100), Err(DataUrlError::TooLarge));
        let big = format!("data:,{}", "a".repeat(1000));
        assert_eq!(decode(&Url::parse(&big, None).expect("url"), 100), Err(DataUrlError::TooLarge));
    }

    #[test]
    fn base64_whitespace_and_padding() {
        assert_eq!(forgiving_base64(b"QU\nI=").as_deref(), Some(&b"AB"[..]));
        assert_eq!(forgiving_base64(b"QUJD").as_deref(), Some(&b"ABC"[..]));
        assert_eq!(forgiving_base64(b"QUJ").as_deref(), Some(&b"AB"[..]));
        assert_eq!(forgiving_base64(b"Q==="), None);
        assert_eq!(forgiving_base64(b""), Some(Vec::new()));
    }
}
