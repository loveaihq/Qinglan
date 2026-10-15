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

impl ImageDecoder for WicDecoder {
    fn probe(&self, bytes: &[u8]) -> Result<ImageInfo, ImageError> {
        if bytes.is_empty() {
            return Err(ImageError::Corrupt);
        }
        if is_svg_bytes(bytes) {
            return Err(ImageError::Svg);
        }
        if is_truncated(bytes) {
            return Err(ImageError::Corrupt);
        }
        let (width, height) = wic::probe(bytes).map_err(error_for)?;
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
            return Err(ImageError::Svg);
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
    use super::clamp_target;

    #[test]
    fn oversized_targets_are_scaled_down_keeping_the_ratio() {
        assert_eq!(clamp_target(100, 50), (100, 50));
        assert_eq!(clamp_target(0, 0), (1, 1));
        let (w, h) = clamp_target(40_000, 20_000);
        assert!(u64::from(w) * u64::from(h) <= 16_000_000);
        assert!((f64::from(w) / f64::from(h) - 2.0).abs() < 0.01);
    }
}
