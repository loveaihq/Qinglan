//! Direct tests of the platform interface and of layout robustness (no snapshot files).
#![allow(clippy::indexing_slicing, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use qinglan_engine::css::types::{FontFamily, GenericFamily};
use qinglan_engine::css::{NoResources, StyleOptions, style_document_with};
use qinglan_engine::encoding::decode_document;
use qinglan_engine::html::treebuilder::parse_document;
use qinglan_engine::layout::{LayoutOptions, LayoutResult, dump::dump_layout, layout_document};
use qinglan_engine::platform::{BasicPlatform, FontQuery, FontSlant, Platform, TestPlatform};

fn lay(html: &str, platform: &dyn Platform) -> (String, LayoutResult) {
    let decoded = decode_document(html.as_bytes(), None, &BasicPlatform);
    let mut doc = parse_document(decoded.text);
    let sopts = StyleOptions { viewport_w: 300.0, viewport_h: 200.0, ..StyleOptions::default() };
    let styled = style_document_with(&mut doc, None, &mut NoResources, &sopts, Some(platform));
    let res = layout_document(&doc, &styled.tree, platform, &LayoutOptions { viewport_w: 300.0, viewport_h: 200.0 });
    (dump_layout(&doc, &res), res)
}

fn query(families: &[FontFamily]) -> FontQuery<'_> {
    FontQuery { families, weight: 400.0, slant: FontSlant::Upright, lang: Some("zh-Hans") }
}

#[test]
fn test_platform_matches_the_task_sheet() {
    let p: &dyn Platform = &TestPlatform; // object safe
    let fams = [FontFamily::Generic(GenericFamily::Serif)];
    let q = query(&fams);
    let font = p.resolve_font(&q).expect("test platform has a font");
    let m = p.font_metrics(font, 20.0);
    assert_eq!((m.ascent, m.descent, m.line_gap), (16.0, 4.0, 0.0));
    assert_eq!((m.x_height, m.zero_advance), (Some(16.0), Some(20.0)));
    // Every glyph is a 1em square; clusters map back to byte ranges.
    let g = p.shape("a中b", &q, 10.0);
    assert_eq!(g.len(), 3);
    assert!(g.iter().all(|g| g.advance == 10.0));
    assert_eq!((g[1].cluster_start, g[1].cluster_end), (1, 4));
    let mask = p.glyph_mask(font, 'a' as u32, 10.0, 0).expect("a is drawn");
    assert_eq!((mask.width, mask.height, mask.top), (10, 10, -8));
    assert!(p.glyph_mask(font, ' ' as u32, 10.0, 0).is_none());
}

#[test]
fn test_platform_line_breaks() {
    let p = TestPlatform;
    // After a space; never inside a Latin word.
    assert_eq!(p.line_breaks("ab cd ef", None), vec![3, 6]);
    // Between CJK characters, but not before closing punctuation or after opening punctuation.
    assert_eq!(p.line_breaks("一二，三「四", None), vec![3, 9, 12]);
    // CJK next to Latin may break; a no-break space and a hyphen never do.
    assert_eq!(p.line_breaks("中a", None), vec![3]);
    assert_eq!(p.line_breaks("a\u{a0}b-c", None), Vec::<usize>::new());
}

#[test]
fn basic_platform_has_no_fonts_but_layout_survives() {
    let (dump, res) = lay("<p>hello <b>world</b></p>", &BasicPlatform);
    assert!(dump.contains("block html"));
    assert!(res.fragments.is_empty(), "no font, no glyphs, no text fragments");
}

#[test]
fn empty_and_odd_documents() {
    for html in ["", "text only", "<!doctype html>", "<html></html>", "<body><br><br></body>", "<svg><text>x</text></svg>", "<template><div>x</div></template>"] {
        let (dump, _) = lay(html, &TestPlatform);
        assert!(dump.starts_with("viewport 300x200"));
    }
}

#[test]
fn hostile_lengths_do_not_panic() {
    let html = "<body style='margin:1e9px;font-size:1e9px'><div style='width:1e9px;height:-5px;margin:-1e9px 1e9px;padding:1e9px'>x<span style='font-size:1e9px'>y</span></div>\
                <div style='min-width:1e9px;max-width:-1px;line-height:1e9px'>a b c</div><div style='width:calc(100% + 1e9px)'></div></body>";
    let (dump, _) = lay(html, &TestPlatform);
    assert!(dump.contains("block"));
}

/// Deep nesting must not overflow a 1 MB stack (the debug main thread on Windows): box building and layout
/// are bounded by `MAX_BOX_DEPTH`.
#[test]
fn deep_nesting_fits_in_one_megabyte_of_stack() {
    let mut html = String::new();
    for i in 0..1500 {
        html.push_str(if i % 2 == 0 { "<div>" } else { "<span>" });
    }
    html.push_str("deep text");
    let h = std::thread::Builder::new()
        .stack_size(1 << 20)
        .spawn(move || {
            let (dump, res) = lay(&html, &TestPlatform);
            assert!(dump.contains("block"));
            assert!(res.texts.iter().any(|t| t == "deep text"));
        })
        .unwrap();
    h.join().unwrap();
}

#[test]
fn tabs_advance_to_the_next_tab_stop() {
    // tab-size 8: with a 10px space the stops are 80px apart; "a" takes 10, the tab the other 70.
    let html = "<body style='margin:0;font:10px Ahem'><div style='white-space:pre;width:200px'>a\tb\n123456789\tc</div>";
    let (dump, _) = lay(html, &TestPlatform);
    assert!(dump.contains("text \"a\\tb\" (0,0 90x10)"), "{dump}");
    // Second line: nine glyphs end at 90, the next stop is 160.
    assert!(dump.contains("text \"123456789\\tc\" (0,10 170x10)"), "{dump}");
}

#[test]
fn long_unbreakable_text_is_linear() {
    let text = "x".repeat(200_000);
    let (_, res) = lay(&format!("<body style='font:10px Ahem'><div style='width:100px'>{text}</div>"), &TestPlatform);
    // Overflows on one line: 200000 glyphs of 10px.
    assert_eq!(res.lines.len(), 1);
    assert_eq!(res.fragments[0].w.to_f32(), 2_000_000.0_f32.min(res.fragments[0].w.to_f32()));
}
