//! Tests of the font decoders (step 2d-1): inflate, Brotli, WOFF and WOFF2.
//!
//! Vectors: the Brotli test data of google/brotli (MIT, `fixtures/brotli/LICENSE`), fonts from the Web Platform
//! Tests (`fixtures/fonts/NOTICE-fonts.txt`) and the expected output of `tools/make_font_vectors.py`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use std::path::PathBuf;
use std::time::{Duration, Instant};

use qinglan_engine::fonts::{FontError, MAX_FONT_BYTES, brotli, decode_font, inflate};

fn fixture(rel: &str) -> Vec<u8> {
    let p: PathBuf = [env!("CARGO_MANIFEST_DIR"), "tests", "fixtures", rel].iter().collect();
    std::fs::read(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

#[test]
fn brotli_official_vectors() {
    let pairs = [
        "10x10y", "64x", "backward65536", "empty", "monkey", "quickfox", "quickfox_repeated", "ukkonooa", "x", "xyzzy", "zeros", "zerosukkanooa", "asyoulik.txt", "random_org_10k.bin",
    ];
    for name in pairs {
        let plain = fixture(&format!("brotli/{name}"));
        let comp = fixture(&format!("brotli/{name}.compressed"));
        let got = brotli::decompress(&comp, 1 << 20).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(got == plain, "{name}: output differs ({} vs {} bytes)", got.len(), plain.len());
    }
    for i in 0..19 {
        let got = brotli::decompress(&fixture(&format!("brotli/empty.compressed.{i:02}")), 16).unwrap_or_else(|e| panic!("empty.{i}: {e}"));
        assert!(got.is_empty(), "empty.compressed.{i:02}");
    }
    for i in 0..4 {
        let got = brotli::decompress(&fixture(&format!("brotli/x.compressed.{i:02}")), 16).unwrap_or_else(|e| panic!("x.{i}: {e}"));
        assert_eq!(got, fixture("brotli/x"), "x.compressed.{i:02}");
    }
}

#[test]
fn brotli_static_dictionary_is_intact() {
    let d = brotli::dictionary().unwrap();
    assert_eq!(d.len(), 122_784);
    assert_eq!(&d[..16], b"timedownlifeleft");
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for &b in d {
        h = (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3);
    }
    assert_eq!(h, 0xd8a8_43a9_cc93_2619);
}

#[test]
fn brotli_limits_truncation_and_bit_flips() {
    let start = Instant::now();
    let big = fixture("brotli/asyoulik.txt.compressed");
    assert_eq!(brotli::decompress(&big, 1000), Err(FontError::TooLarge));
    assert_eq!(brotli::decompress(&fixture("brotli/zeros.compressed"), 1000), Err(FontError::TooLarge));
    // Every proper prefix of a stream is an error, never a panic.
    let monkey = fixture("brotli/monkey.compressed");
    for n in 0..monkey.len() {
        assert!(brotli::decompress(&monkey[..n], 1 << 20).is_err(), "prefix {n}");
    }
    // Every single-bit change decodes or fails, within a small output cap.
    for i in 0..monkey.len() * 8 {
        let mut c = monkey.clone();
        c[i / 8] ^= 1 << (i % 8);
        let _ = brotli::decompress(&c, 1 << 16);
    }
    let fox = fixture("brotli/quickfox_repeated.compressed");
    for i in 0..fox.len() * 8 {
        let mut c = fox.clone();
        c[i / 8] ^= 1 << (i % 8);
        let _ = brotli::decompress(&c, 1 << 20);
    }
    assert!(start.elapsed() < Duration::from_secs(10), "{:?}", start.elapsed());
}

#[test]
fn inflate_and_zlib() {
    assert_eq!(inflate::zlib(&[0x78, 0x9c, 0x03, 0x00, 0x00, 0x00, 0x00, 0x01], 10).unwrap(), b"");
    assert_eq!(inflate::zlib(&[0x78, 0x9c, 0x4b, 0x04, 0x00, 0x00, 0x62, 0x00, 0x62], 10).unwrap(), b"a");
    // A wrong checksum, a bad header and cut-off streams.
    assert!(inflate::zlib(&[0x78, 0x9c, 0x4b, 0x04, 0x00, 0x00, 0x62, 0x00, 0x63], 10).is_err());
    assert!(inflate::zlib(&[0x79, 0x9c, 0x4b, 0x04, 0x00, 0x00, 0x62, 0x00, 0x62], 10).is_err());
    assert!(inflate::zlib(&[0x78, 0x9c, 0x4b, 0x04], 10).is_err());
    // A stored block, and a limit that stops it.
    let stored = [1, 5, 0, 0xfa, 0xff, b'h', b'e', b'l', b'l', b'o'];
    let (out, used) = inflate::inflate(&stored, 100).unwrap();
    assert_eq!((out.as_slice(), used), (&b"hello"[..], 10));
    assert_eq!(inflate::inflate(&stored, 4).unwrap_err(), FontError::TooLarge);
    assert!(inflate::inflate(&[1, 5, 0, 0xfb, 0xff, b'h'], 100).is_err());
}

/// The tables of an sfnt: (tag, data).
fn tables(f: &[u8]) -> Vec<([u8; 4], Vec<u8>)> {
    let n = usize::from(u16::from_be_bytes([f[4], f[5]]));
    (0..n)
        .map(|i| {
            let e = &f[12 + 16 * i..28 + 16 * i];
            let off = u32::from_be_bytes([e[8], e[9], e[10], e[11]]) as usize;
            let len = u32::from_be_bytes([e[12], e[13], e[14], e[15]]) as usize;
            ([e[0], e[1], e[2], e[3]], f[off..off + len].to_vec())
        })
        .collect()
}

fn table<'a>(f: &'a [([u8; 4], Vec<u8>)], tag: &[u8; 4]) -> &'a [u8] {
    &f.iter().find(|(k, _)| k == tag).unwrap().1
}

fn loca_offsets(f: &[([u8; 4], Vec<u8>)]) -> Vec<usize> {
    let (head, loca) = (table(f, b"head"), table(f, b"loca"));
    if head[50] == 0 {
        loca.chunks(2).map(|c| 2 * usize::from(u16::from_be_bytes([c[0], c[1]]))).collect()
    } else {
        loca.chunks(4).map(|c| u32::from_be_bytes([c[0], c[1], c[2], c[3]]) as usize).collect()
    }
}

#[test]
fn woff_files_unpack_to_the_expected_font() {
    for name in ["Revalia.woff", "noto-sans-v8-latin-regular.woff", "pass.woff"] {
        let got = decode_font(&fixture(&format!("fonts/{name}"))).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(got == fixture(&format!("fonts/{name}.expected.ttf")), "{name}");
    }
}

#[test]
fn woff2_files_unpack_to_the_expected_font() {
    for name in ["NotoSansNko-regular-webfont.woff2", "NotoSansMongolian-regular.woff2"] {
        let got = tables(&decode_font(&fixture(&format!("fonts/{name}"))).unwrap_or_else(|e| panic!("{name}: {e}")));
        let want = tables(&fixture(&format!("fonts/{name}.expected.ttf")));
        assert_eq!(got.iter().map(|t| t.0).collect::<Vec<_>>(), want.iter().map(|t| t.0).collect::<Vec<_>>(), "{name}: tables");
        for ((tag, g), (_, w)) in got.iter().zip(&want) {
            match tag {
                // The glyphs are written in the encoding of the reconstruction, which may differ in the flags and
                // the coordinate widths; the headers (contours, bounding box) must agree, and the outlines are
                // compared by drawing in qinglan-win's tests.
                b"glyf" | b"loca" => {}
                b"head" => assert!(g[..8] == w[..8] && g[12..] == w[12..], "{name}: head"),
                _ => assert!(g == w, "{name}: table {}", String::from_utf8_lossy(tag)),
            }
        }
        let (gl, wl) = (loca_offsets(&got), loca_offsets(&want));
        assert_eq!(gl.len(), wl.len(), "{name}: loca");
        let (gg, wg) = (table(&got, b"glyf"), table(&want, b"glyf"));
        let mut nonempty = 0;
        for i in 0..gl.len() - 1 {
            let (a, b) = (&gg[gl[i]..gl[i + 1]], &wg[wl[i]..wl[i + 1]]);
            assert_eq!(a.is_empty(), b.is_empty(), "{name}: glyph {i} emptiness");
            if !b.is_empty() {
                nonempty += 1;
                assert_eq!(a[..10], b[..10], "{name}: glyph {i} header");
            }
        }
        assert!(nonempty > 10, "{name}: only {nonempty} glyphs");
    }
}

#[test]
fn damaged_fonts_fail_cleanly_and_fast() {
    let start = Instant::now();
    let mut slowest = Duration::ZERO;
    for name in ["NotoSansNko-regular-webfont.woff2", "pass.woff", "Revalia.woff"] {
        let data = fixture(&format!("fonts/{name}"));
        // Proper prefixes (about 150 per file).
        for n in (0..data.len()).step_by(data.len() / 150 + 1) {
            let t = Instant::now();
            assert!(decode_font(&data[..n]).is_err(), "{name}: prefix {n} decoded");
            slowest = slowest.max(t.elapsed());
        }
        // Changed bytes: any outcome but a panic.
        for i in (0..data.len()).step_by(data.len() / 400 + 1) {
            for x in [0xFFu8, 0x01, 0x80] {
                let mut c = data.clone();
                c[i] ^= x;
                let t = Instant::now();
                if let Ok(f) = decode_font(&c) {
                    assert!(f.len() <= MAX_FONT_BYTES);
                }
                slowest = slowest.max(t.elapsed());
            }
        }
    }
    // Each clean failure takes well under a second, even in a debug build.
    assert!(slowest < Duration::from_secs(1), "slowest {slowest:?}");
    assert!(start.elapsed() < Duration::from_secs(30), "{:?}", start.elapsed());
}

#[test]
fn declared_sizes_are_checked_before_anything_is_allocated() {
    // A WOFF2 header that claims a 1 GB font, and one that claims a 1 GB table.
    let mut h = Vec::new();
    h.extend_from_slice(b"wOF2");
    h.extend_from_slice(&[0, 1, 0, 0]);
    h.extend_from_slice(&48u32.to_be_bytes());
    h.extend_from_slice(&1u16.to_be_bytes());
    h.extend_from_slice(&[0, 0]);
    h.extend_from_slice(&(1u32 << 30).to_be_bytes());
    h.extend_from_slice(&[0; 28]);
    assert_eq!(decode_font(&h), Err(FontError::TooLarge));
    let mut t = h.clone();
    t[16..20].copy_from_slice(&1000u32.to_be_bytes());
    // flags 1 (head), origLength 1 GB as UIntBase128 (0x84 0x80 0x80 0x80 0x00).
    t.extend_from_slice(&[1, 0x84, 0x80, 0x80, 0x80, 0x00]);
    let len = t.len() as u32;
    t[8..12].copy_from_slice(&len.to_be_bytes());
    assert_eq!(decode_font(&t), Err(FontError::TooLarge));
    // A WOFF claiming the same.
    let mut w = vec![0u8; 44 + 20];
    w[..4].copy_from_slice(b"wOFF");
    w[4..8].copy_from_slice(&[0, 1, 0, 0]);
    w[8..12].copy_from_slice(&64u32.to_be_bytes());
    w[12..14].copy_from_slice(&1u16.to_be_bytes());
    w[16..20].copy_from_slice(&(1u32 << 30).to_be_bytes());
    assert_eq!(decode_font(&w), Err(FontError::TooLarge));
    w[16..20].copy_from_slice(&1000u32.to_be_bytes());
    w[44 + 4..44 + 8].copy_from_slice(&64u32.to_be_bytes());
    w[44 + 12..44 + 16].copy_from_slice(&(1u32 << 30).to_be_bytes());
    assert_eq!(decode_font(&w), Err(FontError::TooLarge));
    // Not a font at all.
    assert!(decode_font(b"<html>").is_err());
    assert!(decode_font(b"").is_err());
}


/// A Brotli stream of `blocks` meta-blocks of 958 * 1095 zero bytes (see `hostile.rs`, which builds the bombs).
fn brotli_zeros(blocks: usize) -> Vec<u8> {
    let (mut bytes, mut n) = (Vec::<u8>::new(), 0usize);
    let mut put = |value: u64, count: usize| {
        for i in 0..count {
            if n.is_multiple_of(8) {
                bytes.push(0);
            }
            if i < 64 && (value >> i) & 1 == 1 {
                let last = bytes.len() - 1;
                bytes[last] |= 1 << (n % 8);
            }
            n += 1;
        }
    };
    put(0, 1);
    let mlen: u64 = 958 * 1095;
    for _ in 0..blocks {
        put(0, 1);
        put(2, 2);
        put(mlen - 1, 24);
        put(0, 1);
        put(0, 3);
        put(0, 2);
        put(0, 4);
        put(0, 2);
        put(0, 2);
        for (alphabet_bits, symbol) in [(8, 0u64), (10, 398), (6, 16)] {
            put(1, 2);
            put(0, 2);
            put(symbol, alphabet_bits);
        }
        put(0, 11 * 958);
    }
    put(1, 1);
    put(1, 1);
    bytes
}

#[test]
fn brotli_one_symbol_codes_and_bombs() {
    let start = Instant::now();
    let two = brotli::decompress(&brotli_zeros(2), 1 << 24).unwrap();
    assert_eq!(two.len(), 2 * 958 * 1095);
    assert!(two.iter().all(|&b| b == 0));
    // 600 meta-blocks would be 630 MB: the cap stops it at once, however big the cap is allowed to be.
    let bomb = brotli_zeros(600);
    assert!(bomb.len() < 800_000, "{}", bomb.len());
    assert_eq!(brotli::decompress(&bomb, 8_000_000), Err(FontError::TooLarge));
    assert_eq!(brotli::decompress(&bomb, 1_000_000), Err(FontError::TooLarge));
    assert!(start.elapsed() < Duration::from_secs(5), "{:?}", start.elapsed());
}

/// A Brotli stream of `blocks` meta-blocks of one byte each that ask for the most setup work an 82-byte meta-block can: 256
/// literal block types, a 16,384-entry literal context map (one run of zeros) with the move-to-front transform undone.
fn brotli_context_maps(blocks: usize) -> Vec<u8> {
    let (mut bytes, mut n) = (Vec::<u8>::new(), 0usize);
    let mut put = |value: u64, count: usize| {
        for i in 0..count {
            if n.is_multiple_of(8) {
                bytes.push(0);
            }
            if i < 64 && (value >> i) & 1 == 1 {
                let last = bytes.len() - 1;
                bytes[last] |= 1 << (n % 8);
            }
            n += 1;
        }
    };
    put(0, 1);
    for _ in 0..blocks {
        for (v, c) in [(0, 1), (0, 2), (0, 16), (0, 1), (1, 1), (7, 3), (127, 7), (1, 2), (0, 2), (0, 9), (1, 2), (0, 2), (0, 5), (3, 2)] {
            put(v, c);
        }
        for (v, c) in [(0, 1), (0, 1), (0, 2), (0, 4)] {
            put(v, c);
        }
        for _ in 0..256 {
            put(0, 2);
        }
        for (v, c) in [(1, 1), (0, 3), (1, 1), (13, 4), (1, 2), (0, 2), (14, 4), (0, 14), (1, 1), (0, 1)] {
            put(v, c);
        }
        for _ in 0..2 {
            for (v, c) in [(1, 2), (0, 2), (65, 8)] {
                put(v, c);
            }
        }
        for (v, c) in [(1, 2), (0, 2), (8, 10), (1, 2), (0, 2), (0, 6)] {
            put(v, c);
        }
    }
    put(1, 1);
    put(1, 1);
    bytes
}

#[test]
fn brotli_setup_work_is_limited_by_the_input() {
    // 20 meta-blocks are a stream like any other: 20 bytes out.
    assert_eq!(brotli::decompress(&brotli_context_maps(20), MAX_FONT_BYTES).unwrap(), vec![b'A'; 20]);
    // 12,000 of them (984 KB) used to cost 8.5 s; the work budget (a megaunit and 16 per input byte) stops the stream
    // after a few hundred, and what it does until then is linear.
    let stream = brotli_context_maps(12_000);
    assert!(stream.len() < 1_000_000);
    let started = Instant::now();
    assert!(matches!(brotli::decompress(&stream, MAX_FONT_BYTES), Err(FontError::TooLarge)));
    assert!(started.elapsed() < Duration::from_secs(2), "took {:?}", started.elapsed());
}
