//! WIC image decoding (step 1c-1): header probe, decode straight to the display size, premultiplied
//! alpha, first GIF frame, hostile headers (100k x 100k), truncated and garbage files.
#![allow(clippy::indexing_slicing, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use qinglan_engine::platform::image::{DecodedCache, ImageDecoder, ImageError, ImageInfo};
use qinglan_win::WicDecoder;

const PNG: &[u8] = include_bytes!("fixtures/rb16x8.png");
const JPG: &[u8] = include_bytes!("fixtures/rb16x8.jpg");
const GIF: &[u8] = include_bytes!("fixtures/rb16x8.gif");
const BMP: &[u8] = include_bytes!("fixtures/rb16x8.bmp");
const BIG: &[u8] = include_bytes!("fixtures/rb400x200.png");
const ALPHA: &[u8] = include_bytes!("fixtures/alpha4x4.png");

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc ^= u32::from(b);
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
        }
    }
    !crc
}

/// A PNG that is only signature, IHDR (claiming `w` x `h`) and IEND.
fn png_header(w: u32, h: u32) -> Vec<u8> {
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut chunk = |kind: &[u8; 4], data: &[u8]| {
        out.extend((data.len() as u32).to_be_bytes());
        let mut body = kind.to_vec();
        body.extend(data);
        out.extend(&body);
        out.extend(crc32(&body).to_be_bytes());
    };
    let mut ihdr = Vec::new();
    ihdr.extend(w.to_be_bytes());
    ihdr.extend(h.to_be_bytes());
    ihdr.extend([8, 2, 0, 0, 0]);
    chunk(b"IHDR", &ihdr);
    // One IDAT holding an empty zlib stream: a structurally complete PNG with no pixels.
    chunk(b"IDAT", &[0x78, 0x9c, 0x03, 0x00, 0x00, 0x00, 0x00, 0x01]);
    chunk(b"IEND", &[]);
    out
}

/// BGRA of the pixel at (x, y) of a decoded image.
fn px(img: &qinglan_engine::platform::image::DecodedImage, x: u32, y: u32) -> [u8; 4] {
    let i = ((y * img.width + x) * 4) as usize;
    [img.bgra[i], img.bgra[i + 1], img.bgra[i + 2], img.bgra[i + 3]]
}

fn near(a: [u8; 4], b: [u8; 4], tol: i32) -> bool {
    a.iter().zip(b).all(|(x, y)| (i32::from(*x) - i32::from(y)).abs() <= tol)
}

const RED: [u8; 4] = [0, 0, 255, 255]; // B G R A
const BLUE: [u8; 4] = [255, 0, 0, 255];

#[test]
fn probe_reads_sizes_of_every_format_from_the_header() {
    let d = WicDecoder;
    for (name, bytes) in [("png", PNG), ("jpg", JPG), ("gif", GIF), ("bmp", BMP)] {
        assert_eq!(d.probe(bytes), Ok(ImageInfo { width: 16, height: 8 }), "{name}");
    }
    assert_eq!(d.probe(BIG), Ok(ImageInfo { width: 400, height: 200 }));
}

#[test]
fn decode_full_size_gives_premultiplied_bgra() {
    let d = WicDecoder;
    for (name, bytes, tol) in [("png", PNG, 0), ("bmp", BMP, 0), ("gif", GIF, 8), ("jpg", JPG, 60)] {
        let img = d.decode(bytes, 16, 8).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!((img.width, img.height, img.natural_width, img.natural_height), (16, 8, 16, 8), "{name}");
        assert_eq!(img.bgra.len(), 16 * 8 * 4);
        assert!(near(px(&img, 2, 3), RED, tol), "{name} left {:?}", px(&img, 2, 3));
        assert!(near(px(&img, 13, 3), BLUE, tol), "{name} right {:?}", px(&img, 13, 3));
    }
    // 50% red: premultiplied, the colour channels are halved along with the alpha.
    let img = d.decode(ALPHA, 4, 4).unwrap();
    let p = px(&img, 1, 1);
    assert!(near(p, [0, 0, 128, 128], 2), "{p:?}");
}

#[test]
fn decode_goes_straight_to_the_display_size() {
    let d = WicDecoder;
    let img = d.decode(BIG, 40, 20).unwrap();
    assert_eq!((img.width, img.height, img.natural_width, img.natural_height), (40, 20, 400, 200));
    assert_eq!(img.bgra.len(), 40 * 20 * 4);
    assert!(near(px(&img, 5, 10), RED, 4) && near(px(&img, 34, 10), BLUE, 4));
    // Upscaling and odd sizes work too; a zero size is treated as 1.
    let up = d.decode(PNG, 32, 16).unwrap();
    assert_eq!((up.width, up.height), (32, 16));
    assert_eq!(d.decode(PNG, 0, 0).map(|i| (i.width, i.height)), Ok((1, 1)));
}

#[test]
fn huge_header_is_refused_without_decoding() {
    let d = WicDecoder;
    let hostile = png_header(100_000, 100_000);
    let e = d.probe(&hostile).unwrap_err();
    assert_eq!(e, ImageError::TooLarge { width: 100_000, height: 100_000 }, "{e:?}");
    assert!(matches!(d.decode(&hostile, 100, 100), Err(ImageError::TooLarge { .. })));
    // Just under and just over the limit (10 000 x 10 000 = 100 Mpx exactly is allowed).
    assert!(d.probe(&png_header(10_000, 10_000)).is_ok());
    assert!(matches!(d.probe(&png_header(10_001, 10_000)), Err(ImageError::TooLarge { .. })));
    // A 65535 x 65535 header does not overflow anything.
    assert!(d.probe(&png_header(u32::MAX >> 1, u32::MAX >> 1)).is_err());
}

#[test]
fn damaged_files_are_broken_not_crashes() {
    let d = WicDecoder;
    assert_eq!(d.probe(b""), Err(ImageError::Corrupt));
    assert!(d.probe(b"not an image at all").is_err());
    assert!(d.decode(b"not an image at all", 10, 10).is_err());
    assert_eq!(d.probe(b"<svg xmlns='http://www.w3.org/2000/svg' width='10' height='10'/>"), Err(ImageError::Svg));
    assert_eq!(d.decode(b"<?xml version='1.0'?><svg/>", 10, 10), Err(ImageError::Svg));
    // A WebP header without (valid) data: refused or broken, whether or not the system has the codec.
    let fake_webp = b"RIFF\x20\x00\x00\x00WEBPVP8 \x14\x00\x00\x00garbagegarbagegarbage";
    assert!(d.probe(fake_webp).is_err());
    // Truncated PNG: the header is fine, the pixels are not.
    let cut = &BIG[..BIG.len() / 2];
    assert_eq!(d.probe(cut), Err(ImageError::Corrupt), "a cut-off file is broken from the start");
    assert!(d.decode(cut, 40, 20).is_err(), "half a PNG cannot be decoded");
    let cut = &PNG[..40];
    assert!(d.decode(cut, 16, 8).is_err());
    // Truncated GIF and BMP.
    assert!(d.decode(&GIF[..GIF.len() / 2], 16, 8).is_err());
    assert!(d.decode(&BMP[..BMP.len() / 2], 16, 8).is_err());
    // Truncated JPEG: either refused or decoded as far as it goes; never a crash.
    let _ = d.decode(&JPG[..JPG.len() / 2], 16, 8);
    // Flipped bytes everywhere in the headers: no panic.
    for i in 0..PNG.len() {
        let mut m = PNG.to_vec();
        m[i] ^= 0xFF;
        let _ = d.decode(&m, 16, 8);
        let _ = d.probe(&m);
    }
}

#[test]
fn decoding_runs_on_many_threads_and_the_cache_counts() {
    let d = Arc::new(WicDecoder);
    let handles: Vec<_> = (0..8)
        .map(|_| {
            let d = d.clone();
            std::thread::spawn(move || (0..20).all(|_| d.decode(BIG, 100, 50).is_ok_and(|i| i.width == 100)))
        })
        .collect();
    for h in handles {
        assert!(h.join().unwrap());
    }
    let mut cache = DecodedCache::with_budget(100 * 50 * 4 * 2 + 200);
    let a = cache.get_or_decode(&*d, "u1", BIG, 100, 50).unwrap();
    let b = cache.get_or_decode(&*d, "u1", BIG, 100, 50).unwrap();
    assert!(Arc::ptr_eq(&a, &b), "second request is a cache hit");
    cache.get_or_decode(&*d, "u2", BIG, 100, 50).unwrap();
    cache.get_or_decode(&*d, "u3", BIG, 100, 50).unwrap();
    assert!(cache.bytes() <= 100 * 50 * 4 * 2 + 200 && cache.len() == 2, "{} bytes, {} entries", cache.bytes(), cache.len());
    assert!(cache.get("u1", 100, 50).is_none(), "least recently used went first");
}

/// `write_png` through WIC: a file that reads back (by our own decoder) with the pixels we wrote.
#[test]
fn png_writer_round_trips_pixels() {
    let (w, h) = (70u32, 150u32); // more rows than one write chunk
    let mut bgra = Vec::new();
    for y in 0..h {
        for x in 0..w {
            bgra.extend_from_slice(&[(x * 3) as u8, (y * 2) as u8, 200, 255]);
        }
    }
    let path = std::env::temp_dir().join(format!("qinglan-png-test-{}.png", std::process::id()));
    qinglan_win::write_png(&path, w, h, &bgra).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    std::fs::remove_file(&path).unwrap();
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    let dec = WicDecoder;
    assert_eq!(dec.probe(&bytes), Ok(ImageInfo { width: w, height: h }));
    let img = dec.decode(&bytes, w, h).unwrap();
    for (x, y) in [(0u32, 0u32), (69, 0), (0, 149), (35, 70), (69, 149)] {
        let i = ((y * w + x) * 4) as usize;
        assert_eq!(&img.bgra[i..i + 4], &[(x * 3) as u8, (y * 2) as u8, 200, 255], "pixel ({x},{y})");
    }
    // Translucent pixels are composed over white; bad sizes and paths are errors, not panics.
    let path = std::env::temp_dir().join(format!("qinglan-png-test2-{}.png", std::process::id()));
    qinglan_win::write_png(&path, 1, 1, &[0, 0, 128, 128]).unwrap();
    let one = dec.decode(&std::fs::read(&path).unwrap(), 1, 1).unwrap();
    std::fs::remove_file(&path).unwrap();
    assert_eq!(&one.bgra[..4], &[127, 127, 255, 255]);
    assert!(qinglan_win::write_png(&path, 0, 5, &[]).is_err());
    assert!(qinglan_win::write_png(&path, 4, 4, &[0; 8]).is_err());
    assert!(qinglan_win::write_png(std::path::Path::new("Z:/no/such/dir/x.png"), 1, 1, &[0; 4]).is_err());
}
