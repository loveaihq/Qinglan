//! Colour emoji and right-to-left shaping on the Windows platform (step 2d-2a). They need the fonts of a normal Windows 10/11
//! install (Segoe UI, Segoe UI Emoji with COLR layers).
#![allow(clippy::indexing_slicing, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::rc::Rc;

use qinglan_engine::css::types::{FontFamily, GenericFamily};
use qinglan_engine::platform::{FontQuery, FontSlant, Platform};
use qinglan_win::WinPlatform;

fn plat() -> WinPlatform {
    WinPlatform::new().expect("DirectWrite")
}
fn sans() -> Vec<FontFamily> {
    vec![FontFamily::Generic(GenericFamily::SansSerif)]
}
fn q(f: &[FontFamily]) -> FontQuery<'_> {
    FontQuery { families: f, weight: 400.0, slant: FontSlant::Upright, lang: None }
}
fn named(n: &str) -> Vec<FontFamily> {
    vec![FontFamily::Named(Rc::from(n))]
}

#[test]
fn an_emoji_is_a_colour_glyph() {
    let p = plat();
    let f = sans();
    let g = p.shape("\u{1F600}", &q(&f), 32.0);
    assert_eq!(g.len(), 1, "{g:?}");
    assert_eq!(p.family_name(g[0].font).as_deref(), Some("Segoe UI Emoji"));
    let m = p.glyph_mask(g[0].font, g[0].glyph, 32.0, 0).expect("the emoji draws");
    assert!(m.alpha.is_empty() && m.bgra.len() == (m.width * m.height * 4) as usize, "a colour mask carries BGRA pixels");
    // Several different colours (the face is yellow, the eyes and mouth are dark): not one colour with varying coverage.
    let mut colours: Vec<[u8; 3]> = m.bgra.as_chunks::<4>().0.iter().filter(|px| px[3] == 255).map(|px| [px[0] / 32, px[1] / 32, px[2] / 32]).collect();
    colours.sort_unstable();
    colours.dedup();
    assert!(colours.len() >= 4, "{colours:?}");
    // The same glyph is kept in the cache and handed out again.
    assert!(Rc::ptr_eq(&m, &p.glyph_mask(g[0].font, g[0].glyph, 32.0, 0).unwrap()));
    // An ordinary letter still gets a coverage mask.
    let a = p.shape("A", &q(&f), 32.0);
    let am = p.glyph_mask(a[0].font, a[0].glyph, 32.0, 0).unwrap();
    assert!(am.bgra.is_empty() && !am.alpha.is_empty());
}

#[test]
fn emoji_sequences_are_one_glyph() {
    let p = plat();
    let f = sans();
    for (name, text) in [
        ("ZWJ family", "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}"),
        ("skin tone", "\u{1F44D}\u{1F3FD}"),
        ("VS16 heart", "\u{2764}\u{FE0F}"),
        ("keycap", "1\u{FE0F}\u{20E3}"),
    ] {
        let g = p.shape(text, &q(&f), 32.0);
        // One cluster: the whole sequence is one unit for the layout (the font composes some of them of several glyphs).
        assert!(!g.is_empty() && g.iter().all(|x| (x.cluster_start as usize, x.cluster_end as usize) == (0, text.len())), "{name}: {g:?}");
        assert!(g.iter().all(|x| p.family_name(x.font).as_deref() == Some("Segoe UI Emoji")), "{name}");
        assert!(g.iter().any(|x| p.glyph_mask(x.font, x.glyph, 32.0, 0).is_some_and(|m| !m.bgra.is_empty())), "{name} is drawn in colour");
    }
    // Text around an emoji is shaped on both sides: "a", the emoji, "b".
    let g = p.shape("a\u{1F600}b", &q(&f), 20.0);
    assert_eq!(g.len(), 3);
    assert_eq!(g[1].cluster_start..g[1].cluster_end, 1..5);
    assert_eq!(g[2].cluster_start, 5);
}

#[test]
fn right_to_left_text_is_shaped_with_joining_and_mirroring() {
    let p = plat();
    let f = named("Segoe UI");
    // Arabic letters join: the same four letters give other glyph forms shaped right to left than shaped left to right.
    let arabic = "\u{633}\u{644}\u{627}\u{645}";
    let (l, r) = (p.shape(arabic, &q(&f), 20.0), p.shape_rtl(arabic, &q(&f), 20.0));
    assert_eq!((l.len(), r.len()), (4, 4));
    assert_ne!(l.iter().map(|g| g.glyph).collect::<Vec<_>>(), r.iter().map(|g| g.glyph).collect::<Vec<_>>());
    // The clusters stay in logical order.
    assert!(r.windows(2).all(|w| w[0].cluster_end <= w[1].cluster_start));
    // A bracket is mirrored: "(" shaped right to left is the glyph of ")" shaped left to right.
    let (open_rtl, close_ltr) = (p.shape_rtl("(", &q(&f), 20.0), p.shape(")", &q(&f), 20.0));
    assert_eq!(open_rtl[0].glyph, close_ltr[0].glyph);
    // The caches keep the two directions apart.
    assert_ne!(p.shape("(", &q(&f), 20.0)[0].glyph, open_rtl[0].glyph);
}

#[test]
fn bidi_levels_follow_uax9() {
    let p = plat();
    // One level per character. Left-to-right paragraph: "ab " 0, an rtl isolate (RLI 0, Hebrew 1, PDI), " cd" 0.
    let text = "ab \u{2067}\u{5D0}\u{5D1}\u{5D2}\u{2069} cd";
    let l = p.bidi_levels(text, false);
    assert_eq!(l.len(), text.chars().count());
    assert_eq!(&l[..3], &[0, 0, 0]);
    assert_eq!(&l[4..7], &[1, 1, 1], "Hebrew inside the isolate");
    assert_eq!(&l[8..], &[0, 0, 0]);
    // Right-to-left paragraph: Latin letters are at level 2, the Hebrew at 1.
    let r = p.bidi_levels("ab \u{5D0}\u{5D1}", true);
    assert_eq!(r, vec![2, 2, 1, 1, 1]);
    // An override: Latin letters between RLO and PDF are reversed (level 1).
    let o = p.bidi_levels("\u{202E}xyz\u{202C}", false);
    assert_eq!(&o[1..4], &[1, 1, 1]);
}
