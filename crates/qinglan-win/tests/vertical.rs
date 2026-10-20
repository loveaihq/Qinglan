//! Vertical forms of the glyphs for upright text (step 2d-2b): the `vert` feature makes the brackets and the ideographic comma
//! come out as other glyphs. Needs a Japanese font of a normal Windows 10/11 install (Yu Mincho or MS Mincho).
#![allow(clippy::indexing_slicing, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::rc::Rc;

use qinglan_engine::css::types::FontFamily;
use qinglan_engine::platform::{FontQuery, FontSlant, Platform};
use qinglan_win::WinPlatform;

#[test]
fn upright_text_gets_the_vertical_forms() {
    let p = WinPlatform::new().expect("DirectWrite");
    for name in ["Yu Mincho", "MS Mincho", "Yu Gothic", "Meiryo"] {
        let fam = vec![FontFamily::Named(Rc::from(name))];
        let q = FontQuery { families: &fam, weight: 400.0, slant: FontSlant::Upright, lang: Some("ja") };
        let h = p.shape("\u{3001}\u{300C}", &q, 24.0);
        let v = p.shape_upright("\u{3001}\u{300C}", &q, 24.0);
        eprintln!("{name}: horizontal {:?} vertical {:?}", h.iter().map(|g| (g.font.0, g.glyph)).collect::<Vec<_>>(), v.iter().map(|g| (g.font.0, g.glyph)).collect::<Vec<_>>());
        if p.family_name(h[0].font).as_deref() == Some(name) {
            assert_ne!(h.iter().map(|g| g.glyph).collect::<Vec<_>>(), v.iter().map(|g| g.glyph).collect::<Vec<_>>(), "{name} has no vertical forms for the comma and the corner bracket");
            return;
        }
    }
    eprintln!("no Japanese font installed: nothing checked");
}
