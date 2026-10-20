//! Web fonts through DirectWrite (step 2d-1): a font decoded from WOFF2 draws exactly like the same font as plain
//! TrueType, a font DirectWrite does not accept is not used, `unicode-range` limits what a face draws, and a face that is
//! hidden (still loading in its block period) keeps its room but draws nothing.
#![allow(clippy::indexing_slicing, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::rc::Rc;

use qinglan_engine::css::types::FontFamily;
use qinglan_engine::fonts::decode_font;
use qinglan_engine::fonts::face::{FaceDesc, FaceStyle};
use qinglan_engine::platform::{FontQuery, FontSlant, Platform, WebFaceState};
use qinglan_win::WinPlatform;

fn font(name: &str) -> Vec<u8> {
    std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../qinglan-engine/tests/fixtures/fonts/").to_string() + name).unwrap()
}

fn desc(family: &str, ranges: Vec<(u32, u32)>) -> FaceDesc {
    FaceDesc { family: family.to_string(), weight: (400.0, 400.0), stretch: (100.0, 100.0), style: FaceStyle::Normal, ranges }
}

fn query(family: &'static [FontFamily]) -> FontQuery<'static> {
    FontQuery { families: family, weight: 400.0, slant: FontSlant::Upright, lang: None }
}

fn nko() -> &'static [FontFamily] {
    thread_local!(static F: &'static [FontFamily] = Box::leak(vec![FontFamily::Named(Rc::from("nko"))].into_boxed_slice()));
    F.with(|f| *f)
}

const NKO_TEXT: &str = "\u{7C0}\u{7C1}\u{7C2}\u{7C3}\u{7C4}\u{7C5}\u{7CA}\u{7CB}\u{7CC}\u{7CD}\u{7D0}\u{7D1}\u{7D2}\u{7D3}\u{7D4}\u{7D5}\u{7D6}\u{7D7}\u{7E0}";

#[test]
fn woff2_draws_like_the_truetype_it_came_from() {
    let woff2 = decode_font(&font("NotoSansNko-regular-webfont.woff2")).unwrap();
    let ttf = font("NotoSansNko-regular-webfont.woff2.expected.ttf");
    let (a, b) = (WinPlatform::new().unwrap(), WinPlatform::new().unwrap());
    for (p, bytes) in [(&a, &woff2), (&b, &ttf)] {
        p.set_web_faces(&[desc("nko", vec![])]);
        assert!(p.load_web_face(0, bytes), "DirectWrite refused the font");
    }
    let (ga, gb) = (a.shape(NKO_TEXT, &query(nko()), 40.0), b.shape(NKO_TEXT, &query(nko()), 40.0));
    assert_eq!(ga.len(), gb.len());
    let mut inked = 0;
    for (x, y) in ga.iter().zip(&gb) {
        assert_eq!((x.glyph, x.advance), (y.glyph, y.advance));
        assert_ne!(x.glyph, 0, "a character of the font came out as the missing glyph");
        let (ma, mb) = (a.glyph_mask(x.font, x.glyph, 40.0, 0), b.glyph_mask(y.font, y.glyph, 40.0, 0));
        assert_eq!(ma.as_deref(), mb.as_deref(), "glyph {}", x.glyph);
        inked += usize::from(ma.is_some());
    }
    assert!(inked >= 15, "{inked} glyphs drawn");
}

#[test]
fn a_font_directwrite_rejects_is_not_used() {
    let p = WinPlatform::new().unwrap();
    p.set_web_faces(&[desc("nko", vec![])]);
    assert!(!p.load_web_face(0, b"this is not a font at all"));
    assert!(!p.load_web_face(0, &font("pass.woff")[..100]));
    // The page goes on in the next family; nothing is hidden.
    let g = p.shape("abc", &query(nko()), 20.0);
    assert!(!g.is_empty() && g.iter().all(|g| g.font.0 < 0x4000));
}

#[test]
fn unicode_range_and_the_block_period() {
    let p = WinPlatform::new().unwrap();
    let ttf = font("NotoSansNko-regular-webfont.woff2.expected.ttf");
    // The face is only for U+07C0..07CF; U+07D0.. goes to the next font of the list (here the system's).
    p.set_web_faces(&[desc("nko", vec![(0x7C0, 0x7CF)])]);
    assert!(p.load_web_face(0, &ttf));
    let g = p.shape("\u{7C1}\u{7D1}", &query(nko()), 30.0);
    assert_eq!(g.len(), 2);
    assert!(g[0].font.0 >= 0x4000 && g[0].font.0 < 0x8000, "the web font draws U+07C1");
    assert!(g[1].font.0 < 0x4000, "U+07D1 is outside the range");
    // A face still loading inside its block period: same room as the fallback font, nothing drawn.
    p.set_web_faces(&[desc("nko", vec![])]);
    let plain = p.shape("Hello", &query(nko()), 30.0);
    p.set_web_face_state(0, WebFaceState::Hidden);
    let hidden = p.shape("Hello", &query(nko()), 30.0);
    assert_eq!(plain.len(), hidden.len());
    for (x, y) in plain.iter().zip(&hidden) {
        assert_eq!((x.glyph, x.advance), (y.glyph, y.advance));
        assert_eq!(y.font.0, x.font.0 | 0x8000);
        assert!(p.glyph_mask(y.font, y.glyph, 30.0, 0).is_none());
        assert!(p.glyph_mask(x.font, x.glyph, 30.0, 0).is_some());
        assert!(p.font_metrics(y.font, 30.0).ascent > 0.0);
    }
    // After the block period it is shown in the fallback font.
    p.set_web_face_state(0, WebFaceState::Fallback);
    assert_eq!(p.shape("Hello", &query(nko()), 30.0), plain);
    // Leaving the page frees the fonts.
    p.set_web_faces(&[]);
}
