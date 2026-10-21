//! Image decoding through WIC (L1 task 15): the Windows implementation of the engine's
//! [`ImageDecoder`]. Stateless and thread-safe: each call sets up COM on its thread and creates its own
//! WIC objects, so decoding runs on worker threads.

use qinglan_engine::net::content::is_svg_bytes;
use qinglan_engine::platform::image::{DecodedImage, ImageDecoder, ImageError, ImageInfo, MAX_DECODED_PIXELS, MAX_IMAGE_PIXELS, is_truncated};

use crate::sys::wic::{self, DecodeFail};

pub struct WicDecoder;

/// WIC's "no decoder for this" results: unknown format, component not found, unsupported pixel format
/// or operation. Everything else is a damaged file.
fn error_for(hr: i32) -> ImageError {
    match hr as u32 {
        0x8898_2F07 | 0x8898_2F50 | 0x8898_2F80 | 0x8898_2F81 => ImageError::Unsupported,
        _ => ImageError::Corrupt,
    }
}

/// Shrink a requested size so it has at most [`MAX_DECODED_PIXELS`] pixels, keeping the aspect ratio.
fn clamp_target(width: u32, height: u32) -> (u32, u32) {
    let (w, h) = (width.max(1), height.max(1));
    let area = u64::from(w) * u64::from(h);
    if area <= MAX_DECODED_PIXELS {
        return (w, h);
    }
    let k = (MAX_DECODED_PIXELS as f64 / area as f64).sqrt();
    (((f64::from(w) * k) as u32).max(1), ((f64::from(h) * k) as u32).max(1))
}

/// The size from the first bytes of a PNG, JPEG, WebP, GIF or BMP, read directly. A WIC probe takes 20 to 400 ms (it asks
/// every codec on the system, WebP's is an extension package), and the UI thread probes each image that arrives while the
/// page scrolls (step 2-0). `None` for anything else, which WIC then looks at. A file that is damaged further in, or a WebP
/// the system has no codec for, shows up when it is decoded (the image is then drawn as nothing, not as its alt text).
fn header_size(b: &[u8]) -> Option<(u32, u32)> {
    let be32 = |at: usize| Some(u32::from_be_bytes(b.get(at..at + 4)?.try_into().ok()?));
    let le32 = |at: usize| Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?));
    let be16 = |at: usize| Some(u16::from_be_bytes([*b.get(at)?, *b.get(at + 1)?]));
    if b.starts_with(b"\x89PNG\r\n\x1a\n") && b.get(12..16) == Some(b"IHDR") {
        return Some((be32(16)?, be32(20)?));
    }
    if b.starts_with(b"GIF8") {
        // The logical screen (the first frame is nearly always as big).
        let le16 = |at: usize| Some(u32::from(u16::from_le_bytes([*b.get(at)?, *b.get(at + 1)?])));
        return Some((le16(6)?, le16(8)?));
    }
    if b.starts_with(b"RIFF") && b.get(8..12) == Some(b"WEBP") {
        let le24 = |at: usize| Some(u32::from_le_bytes([*b.get(at)?, *b.get(at + 1)?, *b.get(at + 2)?, 0]));
        return match b.get(12..16)? {
            // Lossy: frame tag, start code 9D 01 2A, then two 14-bit sizes.
            b"VP8 " if b.get(23..26) == Some(&[0x9D, 0x01, 0x2A]) => Some((le32(26)? & 0x3FFF, (le32(26)? >> 16) & 0x3FFF)),
            // Lossless: signature 2F, then 14 bits of width - 1 and 14 bits of height - 1.
            b"VP8L" if b.get(20) == Some(&0x2F) => {
                let bits = le32(21)?;
                Some(((bits & 0x3FFF) + 1, ((bits >> 14) & 0x3FFF) + 1))
            }
            // Extended: the canvas, 24 bits each, minus one.
            b"VP8X" => Some((le24(24)? + 1, le24(27)? + 1)),
            _ => None,
        };
    }
    if b.starts_with(b"BM") {
        // BITMAPCOREHEADER (12 bytes: 16-bit sizes) or BITMAPINFOHEADER and later (signed 32-bit, negative height = top-down).
        return match le32(14)? {
            12 => Some((u32::from(u16::from_le_bytes([*b.get(18)?, *b.get(19)?])), u32::from(u16::from_le_bytes([*b.get(20)?, *b.get(21)?])))),
            40.. => Some(((le32(18)? as i32).unsigned_abs(), (le32(22)? as i32).unsigned_abs())),
            _ => None,
        };
    }
    if b.starts_with(&[0xFF, 0xD8, 0xFF]) {
        // Walk the segments up to the frame header (SOF0..SOF15 but DHT, JPG and DAC).
        let mut i = 2;
        loop {
            if *b.get(i)? != 0xFF {
                return None;
            }
            let code = loop {
                i += 1;
                let c = *b.get(i)?;
                if c != 0xFF {
                    break c;
                }
            };
            i += 1;
            match code {
                0x01 | 0xD0..=0xD7 => {}
                0xD8..=0xDA => return None,
                0xC0..=0xCF if !matches!(code, 0xC4 | 0xC8 | 0xCC) => return Some((u32::from(be16(i + 5)?), u32::from(be16(i + 3)?))),
                _ => {
                    let len = usize::from(be16(i)?);
                    if len < 2 {
                        return None;
                    }
                    i += len;
                }
            }
        }
    }
    None
}

impl ImageDecoder for WicDecoder {
    fn probe(&self, bytes: &[u8]) -> Result<ImageInfo, ImageError> {
        if bytes.is_empty() {
            return Err(ImageError::Corrupt);
        }
        if is_svg_bytes(bytes) {
            return qinglan_engine::svg::probe(bytes).map(|(width, height)| ImageInfo { width, height });
        }
        if is_truncated(bytes) {
            return Err(ImageError::Corrupt);
        }
        let (width, height) = match header_size(bytes) {
            Some(size) => size,
            None => wic::probe(bytes).map_err(error_for)?,
        };
        if width == 0 || height == 0 {
            return Err(ImageError::Corrupt);
        }
        if u64::from(width) * u64::from(height) > MAX_IMAGE_PIXELS {
            return Err(ImageError::TooLarge { width, height });
        }
        Ok(ImageInfo { width, height })
    }

    fn decode(&self, bytes: &[u8], width: u32, height: u32) -> Result<DecodedImage, ImageError> {
        if bytes.is_empty() {
            return Err(ImageError::Corrupt);
        }
        if is_svg_bytes(bytes) {
            return qinglan_engine::svg::rasterize(bytes, width, height);
        }
        if is_truncated(bytes) {
            return Err(ImageError::Corrupt);
        }
        let (tw, th) = clamp_target(width, height);
        match wic::decode_scaled(bytes, tw, th, MAX_IMAGE_PIXELS) {
            Ok(p) => Ok(DecodedImage { width: p.width, height: p.height, natural_width: p.natural_width, natural_height: p.natural_height, bgra: p.bgra }),
            Err(DecodeFail::TooLarge { width, height }) => Err(ImageError::TooLarge { width, height }),
            Err(DecodeFail::Hr(hr)) => Err(error_for(hr)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{clamp_target, header_size};

    #[test]
    fn sizes_are_read_from_png_jpeg_and_bmp_headers() {
        let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
        png.extend_from_slice(&[0, 0, 1, 0, 0, 0, 0, 7, 8, 6, 0, 0, 0]);
        assert_eq!(header_size(&png), Some((256, 7)));
        // JPEG: SOI, APP0 (length 4), a segment after fill bytes, SOF2 (progressive) 9 wide and 7 high.
        let jpeg = [0xFF, 0xD8, 0xFF, 0xE0, 0, 4, 1, 2, 0xFF, 0xFF, 0xDB, 0, 3, 0, 0xFF, 0xC2, 0, 11, 8, 0, 7, 0, 9, 1, 1, 0x11, 0];
        assert_eq!(header_size(&jpeg), Some((9, 7)));
        // The scan comes before any frame header: not judged here.
        assert_eq!(header_size(&[0xFF, 0xD8, 0xFF, 0xDA, 0, 2]), None);
        // BMP, BITMAPINFOHEADER, 20 wide and 12 high, then the same stored top-down.
        let mut bmp = b"BM".to_vec();
        bmp.extend_from_slice(&[0; 12]);
        bmp.extend_from_slice(&40u32.to_le_bytes());
        bmp.extend_from_slice(&20i32.to_le_bytes());
        bmp.extend_from_slice(&12i32.to_le_bytes());
        assert_eq!(header_size(&bmp), Some((20, 12)));
        let n = bmp.len();
        bmp[n - 4..].copy_from_slice(&(-12i32).to_le_bytes());
        assert_eq!(header_size(&bmp), Some((20, 12)));
        // GIF: the logical screen. WebP: lossy, lossless and extended.
        assert_eq!(header_size(b"GIF89a\x03\0\x05\0\0\0\0"), Some((3, 5)));
        let riff = |kind: &[u8; 4], body: &[u8]| [b"RIFF\0\0\0\0WEBP".as_slice(), kind, &[0; 4], body].concat();
        // Lossy: 4 bytes of frame tag/start code lead, here 3 + 9D 01 2A, then 13 wide and 11 high (14 bits each).
        assert_eq!(header_size(&riff(b"VP8 ", &[0x10, 0, 0, 0x9D, 0x01, 0x2A, 13, 0x40, 11, 0x80])), Some((13, 11)));
        // Lossless: width - 1 = 99 and height - 1 = 49 in 14 bits each after the 2F signature.
        let bits: u32 = 99 | (49 << 14);
        let mut vp8l = vec![0x2F];
        vp8l.extend_from_slice(&bits.to_le_bytes());
        assert_eq!(header_size(&riff(b"VP8L", &vp8l)), Some((100, 50)));
        // Extended: flags (4 bytes), then 24-bit canvas sizes minus one.
        assert_eq!(header_size(&riff(b"VP8X", &[0, 0, 0, 0, 0x2B, 0x01, 0, 0x13, 0, 0])), Some((300, 20)));
        // Too short, other formats, nothing: left to WIC.
        for bytes in [&png[..20], &jpeg[..9], &bmp[..16], b"GIF89a\x01".as_slice(), b"RIFF\0\0\0\0WEBPVP8 ".as_slice(), b"".as_slice()] {
            assert_eq!(header_size(bytes), None);
        }
    }

    #[test]
    fn oversized_targets_are_scaled_down_keeping_the_ratio() {
        assert_eq!(clamp_target(100, 50), (100, 50));
        assert_eq!(clamp_target(0, 0), (1, 1));
        let (w, h) = clamp_target(40_000, 20_000);
        assert!(u64::from(w) * u64::from(h) <= 16_000_000);
        assert!((f64::from(w) / f64::from(h) - 2.0).abs() < 0.01);
    }
}
