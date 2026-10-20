//! Web fonts: the decoders for the font containers a page can ship (WOFF 1.0 with its zlib streams, WOFF2 with its
//! Brotli stream and table transforms), and the `@font-face` data the style and platform code share (step 2d-1).
//!
//! Every input here comes from a web page, so it is hostile: nothing indexes a slice directly or panics, every length
//! a file declares is checked against the limits below *before* anything is allocated for it, and every loop either
//! consumes input or produces output that is counted against a limit.

pub mod bits;
pub mod brotli;
mod brotli_tables;
pub mod face;
pub mod huff;
pub mod inflate;
pub mod sfnt;
pub mod woff;
pub mod woff2;

use std::fmt;

/// A decoded font may be this big (docs/decisions.md, limits table). Bigger fonts are not used.
pub const MAX_FONT_BYTES: usize = 8 << 20;
/// All the web fonts of one page together, decoded.
pub const MAX_PAGE_FONT_BYTES: usize = 16 << 20;
/// A font file with more tables than this is refused (a real font has 10 to 30).
pub const MAX_TABLES: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontError {
    /// The data ends before the structure does.
    Truncated,
    /// The data is not what the format allows.
    Corrupt(&'static str),
    /// The decoded font would be over a limit.
    TooLarge,
    /// A valid feature that this decoder does not do (font collections in WOFF2).
    Unsupported(&'static str),
}

impl fmt::Display for FontError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FontError::Truncated => f.write_str("truncated font"),
            FontError::Corrupt(w) => write!(f, "corrupt font ({w})"),
            FontError::TooLarge => f.write_str("font too large"),
            FontError::Unsupported(w) => write!(f, "unsupported font ({w})"),
        }
    }
}

/// `s[i]` that is an error instead of a panic.
#[inline]
pub(crate) fn at<T: Copy>(s: &[T], i: usize) -> Result<T, FontError> {
    s.get(i).copied().ok_or(FontError::Corrupt("index out of range"))
}

/// A reader of big-endian font data.
pub(crate) struct Rd<'a> {
    pub d: &'a [u8],
    pub p: usize,
}

impl<'a> Rd<'a> {
    pub fn new(d: &'a [u8]) -> Rd<'a> {
        Rd { d, p: 0 }
    }

    #[inline(never)]
    pub fn take(&mut self, n: usize) -> Result<&'a [u8], FontError> {
        let end = self.p.checked_add(n).ok_or(FontError::Truncated)?;
        let s = self.d.get(self.p..end).ok_or(FontError::Truncated)?;
        self.p = end;
        Ok(s)
    }

    pub fn u8(&mut self) -> Result<u8, FontError> {
        Ok(self.take(1)?.first().copied().unwrap_or(0))
    }

    pub fn u16(&mut self) -> Result<u16, FontError> {
        let b = self.take(2)?;
        Ok(u16::from_be_bytes([at(b, 0)?, at(b, 1)?]))
    }

    pub fn u32(&mut self) -> Result<u32, FontError> {
        let b = self.take(4)?;
        Ok(u32::from_be_bytes([at(b, 0)?, at(b, 1)?, at(b, 2)?, at(b, 3)?]))
    }

    pub fn tag(&mut self) -> Result<[u8; 4], FontError> {
        let b = self.take(4)?;
        Ok([at(b, 0)?, at(b, 1)?, at(b, 2)?, at(b, 3)?])
    }
}

/// Big-endian `u16` at `off` in `d`.
pub(crate) fn be16(d: &[u8], off: usize) -> Option<u16> {
    let b = d.get(off..off.checked_add(2)?)?;
    Some(u16::from_be_bytes([*b.first()?, *b.get(1)?]))
}

/// A web font file as the platform wants it: an sfnt (TrueType or OpenType, or a collection) that DirectWrite can open.
/// WOFF and WOFF2 are unpacked; a bare sfnt is passed on after a look at its header. The result is at most
/// [`MAX_FONT_BYTES`].
pub fn decode_font(data: &[u8]) -> Result<Vec<u8>, FontError> {
    match data.get(0..4) {
        Some(b"wOFF") => woff::decode(data),
        Some(b"wOF2") => woff2::decode(data),
        Some([0, 1, 0, 0]) | Some(b"true") | Some(b"OTTO") | Some(b"ttcf") => {
            if data.len() > MAX_FONT_BYTES {
                return Err(FontError::TooLarge);
            }
            // The table directory of a single font must fit in the file; DirectWrite checks the rest.
            if data.get(0..4) != Some(b"ttcf") {
                let n = usize::from(be16(data, 4).ok_or(FontError::Truncated)?);
                if n == 0 || n > MAX_TABLES || 12 + 16 * n > data.len() {
                    return Err(FontError::Corrupt("table directory"));
                }
            }
            Ok(data.to_vec())
        }
        Some(_) => Err(FontError::Corrupt("not a font file")),
        None => Err(FontError::Truncated),
    }
}
