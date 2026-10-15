//! Image decoding as the engine asks for it (L1 task 15). The Windows implementation (WIC) lives in
//! `qinglan-win`; the engine only sees this trait and keeps the decoded-image cache itself.
//!
//! Decoders are `Send + Sync` and hold no state, so decoding can run on worker threads (unlike
//! [`super::Platform`], which owns font objects).

use std::fmt;
use std::sync::Arc;

use crate::lru::ByteLru;

/// Images with more pixels than this are refused without decoding (L1 task 15).
pub const MAX_IMAGE_PIXELS: u64 = 100_000_000;
/// The decoded-image cache budget (L1 task 15), counted in bytes of pixels.
pub const DECODED_CACHE_BUDGET: usize = 12 * 1024 * 1024;
/// A decode never produces more than this many pixels (16 Mpx = 64 MB of BGRA): a request for a
/// bigger display size is scaled down to fit.
pub const MAX_DECODED_PIXELS: u64 = 16_000_000;

/// Why an image is shown as broken (the page then shows its `alt` text).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImageError {
    /// No decoder for this format on this system (WebP without the codec, unknown data).
    Unsupported,
    /// SVG: not drawn in layer 1.
    Svg,
    /// The header claims more than [`MAX_IMAGE_PIXELS`].
    TooLarge { width: u32, height: u32 },
    /// Truncated or damaged data.
    Corrupt,
}

impl fmt::Display for ImageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImageError::Unsupported => write!(f, "unsupported image format"),
            ImageError::Svg => write!(f, "SVG images are not shown in layer 1"),
            ImageError::TooLarge { width, height } => write!(f, "image too large ({width}x{height} pixels)"),
            ImageError::Corrupt => write!(f, "damaged image"),
        }
    }
}

/// The pixel size of an image, from its header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImageInfo {
    pub width: u32,
    pub height: u32,
}

/// Pixels ready to draw: premultiplied BGRA, `width * 4` bytes per row, top row first.
#[derive(Debug, PartialEq, Eq)]
pub struct DecodedImage {
    pub width: u32,
    pub height: u32,
    /// The size of the image file, for bookkeeping.
    pub natural_width: u32,
    pub natural_height: u32,
    pub bgra: Vec<u8>,
}

pub trait ImageDecoder: Send + Sync {
    /// The size from the header only (no pixel decoding). Fails with [`ImageError::TooLarge`] for more
    /// than [`MAX_IMAGE_PIXELS`] and with [`ImageError::Corrupt`] / [`ImageError::Unsupported`] when the
    /// header is not a known image.
    fn probe(&self, bytes: &[u8]) -> Result<ImageInfo, ImageError>;

    /// Decode (the first frame of) the image straight to `width` x `height` device pixels, never
    /// building the full-size image first. The result may be smaller than asked when the request is
    /// over [`MAX_DECODED_PIXELS`] (the aspect ratio is kept).
    fn decode(&self, bytes: &[u8], width: u32, height: u32) -> Result<DecodedImage, ImageError>;
}

/// A decoder for hosts without image support: everything is `Unsupported`.
pub struct NoImages;

impl ImageDecoder for NoImages {
    fn probe(&self, _bytes: &[u8]) -> Result<ImageInfo, ImageError> {
        Err(ImageError::Unsupported)
    }
    fn decode(&self, _bytes: &[u8], _width: u32, _height: u32) -> Result<DecodedImage, ImageError> {
        Err(ImageError::Unsupported)
    }
}

/// Decoded images by (address, display width, display height), LRU by pixel bytes. Eviction by
/// viewport distance (images four screens away may go) is the window's job; it calls [`Self::remove`].
pub struct DecodedCache {
    lru: ByteLru<(String, u32, u32), Arc<DecodedImage>>,
}

impl DecodedCache {
    pub fn new() -> DecodedCache {
        DecodedCache::with_budget(DECODED_CACHE_BUDGET)
    }

    pub fn with_budget(bytes: usize) -> DecodedCache {
        DecodedCache { lru: ByteLru::new(bytes) }
    }

    pub fn get(&mut self, url: &str, width: u32, height: u32) -> Option<Arc<DecodedImage>> {
        self.lru.get(&(url.to_string(), width, height)).cloned()
    }

    pub fn insert(&mut self, url: &str, width: u32, height: u32, img: Arc<DecodedImage>) {
        let cost = img.bgra.len() + 64;
        self.lru.insert((url.to_string(), width, height), img, cost);
    }

    pub fn remove(&mut self, url: &str, width: u32, height: u32) {
        self.lru.remove(&(url.to_string(), width, height));
    }

    /// The cached decode, or decode now (and cache it).
    pub fn get_or_decode(&mut self, dec: &dyn ImageDecoder, url: &str, bytes: &[u8], width: u32, height: u32) -> Result<Arc<DecodedImage>, ImageError> {
        if let Some(hit) = self.get(url, width, height) {
            return Ok(hit);
        }
        let img = Arc::new(dec.decode(bytes, width, height)?);
        self.insert(url, width, height, img.clone());
        Ok(img)
    }

    /// Bytes counted against the budget.
    pub fn bytes(&self) -> usize {
        self.lru.bytes()
    }

    pub fn len(&self) -> usize {
        self.lru.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lru.is_empty()
    }
}

impl Default for DecodedCache {
    fn default() -> Self {
        DecodedCache::new()
    }
}

/// Whether `bytes` is a known image format that is cut off: the structure does not reach its end
/// marker (PNG `IEND`, JPEG `EOI`, GIF trailer) or the file is shorter than its header says (BMP,
/// WebP). Decoders are lenient and show half an image; layer 1 shows such files as broken (L1 task
/// 15), so the check runs before decoding. Unknown formats are not judged here.
pub fn is_truncated(bytes: &[u8]) -> bool {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        !png_complete(bytes)
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        // The end-of-image marker, allowing a little padding after it.
        let tail = bytes.get(bytes.len().saturating_sub(64)..).unwrap_or(&[]);
        !tail.windows(2).any(|w| w == [0xFF, 0xD9])
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        !gif_complete(bytes)
    } else if bytes.starts_with(b"BM") && bytes.len() >= 14 {
        let size = le32(bytes, 2).unwrap_or(0);
        size != 0 && (bytes.len() as u64) < u64::from(size)
    } else if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
        le32(bytes, 4).is_some_and(|size| (bytes.len() as u64) < u64::from(size) + 8)
    } else {
        false
    }
}

fn le32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

fn png_complete(b: &[u8]) -> bool {
    let mut i = 8usize;
    loop {
        let Some(len) = b.get(i..i + 4).and_then(|s| <[u8; 4]>::try_from(s).ok()).map(u32::from_be_bytes) else { return false };
        let Some(kind) = b.get(i + 4..i + 8) else { return false };
        // length + type + data + CRC
        let Some(end) = i.checked_add(12).and_then(|n| n.checked_add(len as usize)) else { return false };
        if end > b.len() {
            return false;
        }
        if kind == b"IEND" {
            return true;
        }
        i = end;
    }
}

fn gif_complete(b: &[u8]) -> bool {
    let Some(&flags) = b.get(10) else { return false };
    let mut i = 13usize;
    if flags & 0x80 != 0 {
        i += 3 << ((flags & 7) + 1);
    }
    // Skip data sub-blocks (a length byte, that many bytes, ... a zero length ends them).
    let skip_blocks = |mut i: usize| -> Option<usize> {
        loop {
            let n = usize::from(*b.get(i)?);
            i += 1;
            if n == 0 {
                return Some(i);
            }
            i += n;
            if i > b.len() {
                return None;
            }
        }
    };
    loop {
        match b.get(i) {
            Some(0x3B) => return true,
            Some(0x21) => match skip_blocks(i + 2) {
                Some(n) => i = n,
                None => return false,
            },
            Some(0x2C) => {
                let Some(&f) = b.get(i + 9) else { return false };
                i += 10;
                if f & 0x80 != 0 {
                    i += 3 << ((f & 7) + 1);
                }
                match skip_blocks(i + 1) {
                    Some(n) => i = n,
                    None => return false,
                }
            }
            _ => return false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::is_truncated;

    #[test]
    fn truncation_checks() {
        let png_end = b"\x89PNG\r\n\x1a\n\x00\x00\x00\x00IHDR\x00\x00\x00\x00\x00\x00\x00\x00IEND\xaeB`\x82";
        let mut png = png_end.to_vec();
        assert!(!is_truncated(&png));
        png.truncate(png.len() - 3);
        assert!(is_truncated(&png));
        assert!(is_truncated(b"\x89PNG\r\n\x1a\n\x00\x00"));
        assert!(!is_truncated(b"\xff\xd8\xff\xe0 data \xff\xd9"));
        assert!(is_truncated(b"\xff\xd8\xff\xe0 data data"));
        // GIF: header, 1x1 screen without a colour table, one image with LZW data, trailer.
        let gif = b"GIF89a\x01\x00\x01\x00\x00\x00\x00,\x00\x00\x00\x00\x01\x00\x01\x00\x00\x02\x02D\x01\x00;";
        assert!(!is_truncated(gif));
        assert!(is_truncated(&gif[..gif.len() - 1]));
        assert!(is_truncated(&gif[..20]));
        assert!(is_truncated(b"BM\x00\x10\x00\x00 short but 14+ bytes"));
        assert!(!is_truncated(b"BM\x00\x00\x00\x00 not judged: size 0 and long enough for a header"));
        assert!(!is_truncated(b"not an image"));
    }
}
