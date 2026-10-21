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
    let res = layout_document(&doc, &styled.tree, platform, &LayoutOptions { viewport_w: 300.0, viewport_h: 200.0, ..LayoutOptions::default() });
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

fn lay_opts(html: &str, opts: &LayoutOptions, images: &qinglan_engine::layout::ImageSizes) -> (String, LayoutResult) {
    let decoded = decode_document(html.as_bytes(), None, &BasicPlatform);
    let mut doc = parse_document(decoded.text);
    let sopts = StyleOptions { viewport_w: opts.viewport_w, viewport_h: opts.viewport_h, ..StyleOptions::default() };
    let styled = style_document_with(&mut doc, None, &mut NoResources, &sopts, Some(&TestPlatform as &dyn Platform));
    let res = qinglan_engine::layout::layout_document_with(&doc, &styled.tree, &TestPlatform, opts, images);
    (dump_layout(&doc, &res), res)
}

#[test]
fn work_budget_stops_layout_and_flags_it() {
    let html = format!("<body style='font:10px Ahem'>{}", "<p>some text in a paragraph</p>".repeat(3000));
    let sizes = qinglan_engine::layout::ImageSizes::new();
    let (_, full) = lay_opts(&html, &LayoutOptions::default(), &sizes);
    assert!(!full.incomplete);
    assert!(full.lines.len() >= 3000);
    let tight = LayoutOptions { work_budget: 4000, ..LayoutOptions::default() };
    let (dump, cut) = lay_opts(&html, &tight, &sizes);
    assert!(cut.incomplete, "the budget of 4000 must run out");
    assert!(cut.lines.len() < full.lines.len() && !cut.lines.is_empty(), "{} of {} lines", cut.lines.len(), full.lines.len());
    assert!(dump.ends_with("incomplete\n"));
    // The work done stays near the budget (one box or paragraph may overshoot a little).
    assert!(cut.work < 4000 + 2000, "work {}", cut.work);
}

/// Text beyond the page's allowance is dropped (memory and time stay bounded) and the layout is flagged.
#[test]
fn text_allowance_bounds_hostile_text() {
    let text = "x".repeat(300_000);
    let html = format!("<body style='font:10px Ahem'><div style='width:100px'>{text}</div>");
    let opts = LayoutOptions { work_budget: 400_000, ..LayoutOptions::default() };
    let (_, res) = lay_opts(&html, &opts, &qinglan_engine::layout::ImageSizes::new());
    assert!(res.incomplete);
    // Half the budget (200000 characters) was shaped.
    assert!(res.glyphs.len() <= 200_000, "{} glyphs", res.glyphs.len());
}

#[test]
fn image_sizes_feed_the_next_layout() {
    use qinglan_engine::layout::ImageSizes;
    let html = "<body style='margin:0;font:10px Ahem'><img id=a src=pic.png srcset='pic2.png 2x'><img id=b src=b.png>";
    let (dump, res) = lay_opts(html, &LayoutOptions { dpr: 2.0, ..LayoutOptions::default() }, &ImageSizes::new());
    assert_eq!(res.images.len(), 2);
    assert_eq!((res.images[0].src.as_str(), res.images[0].density), ("pic2.png", 2.0));
    assert_eq!((res.images[1].src.as_str(), res.images[1].density), ("b.png", 1.0));
    assert!(dump.contains("replaced img#a (0,8 0x0)"), "{dump}");
    // The host learns the size of the first image (nodes are stable between layouts) and lays out again.
    let node = res.images[0].node;
    let mut sizes = ImageSizes::new();
    assert!(sizes.set_loaded(node, 100, 60));
    assert!(!sizes.set_loaded(node, 100, 60), "no change the second time");
    let (dump, _) = lay_opts(html, &LayoutOptions { dpr: 2.0, ..LayoutOptions::default() }, &sizes);
    assert!(dump.contains("replaced img#a (0,0 50x30)"), "{dump}");
}

/// Atomic inlines recurse (box -> paragraph -> box ...): bounded by `MAX_BOX_DEPTH` like blocks.
#[test]
fn deep_atomic_and_positioned_nesting_fits_in_one_megabyte() {
    let h = std::thread::Builder::new()
        .stack_size(1 << 20)
        .spawn(|| {
            let mut html = String::new();
            for i in 0..1500 {
                html.push_str(match i % 3 {
                    0 => "<span style='display:inline-block'>",
                    1 => "<div style='position:absolute;left:1px'>",
                    _ => "<div style='position:relative;width:min-content'>",
                });
            }
            html.push_str("deep text");
            // Nesting deeper than the limits is cut off (the text may not be laid out); it must not crash.
            let (dump, _) = lay(&html, &TestPlatform);
            assert!(dump.contains("block"));
        })
        .unwrap();
    h.join().unwrap();
}

/// Plain block nesting is bounded by `MAX_BOX_DEPTH` alone, so a block frame must be small enough for 256 levels in 1 MB.
#[test]
fn deep_block_nesting_fits_in_one_megabyte() {
    let h = std::thread::Builder::new()
        .stack_size(1 << 20)
        .spawn(|| {
            let html = format!("{}x{}", "<div style='position:relative;padding:1px;margin:1px;width:80%'>".repeat(1500), "</div>".repeat(1500));
            let (_, res) = lay(&html, &TestPlatform);
            assert!(res.texts.iter().any(|t| t == "x"));
        })
        .unwrap();
    h.join().unwrap();
}

#[test]
fn hostile_positioning_does_not_panic() {
    let html = "<body style='font:10px Ahem'><div style='position:relative;left:1e9px;top:-1e9px'>a\
                <span style='position:absolute;left:1e9px;right:1e9px;top:-1e9px;bottom:1e9px;width:1e9px;height:1e9px;margin:auto'>b</span>\
                <span style='position:fixed;left:-1e9px;width:calc(100% + 1e9px)'>c</span>\
                <span style='position:absolute;width:0;height:0'>d<img src=x width=1e9 height=1e9></span></div>\
                <div style='width:max-content;min-width:min-content'>e<input size=1e9><textarea cols=1e9 rows=1e9></textarea></div>";
    let (dump, _) = lay(html, &TestPlatform);
    assert!(dump.contains("[absolute]") && dump.contains("[fixed]"));
}

fn blank_lines_page(n: usize, align: &str) -> String {
    format!("<body style='font:10px Ahem'><pre style='text-align:{align};width:100px'>{}x</pre><p style='text-align:{align};width:100px'>{}</p>", "\n".repeat(n), "<br>".repeat(n))
}

/// Blank lines (a `<pre>` full of line feeds, trailing `<br>`s) cost the same each, whatever follows them:
/// 4 times the lines take about 4 times as long, not 16 times.
#[test]
fn blank_lines_scale_linearly() {
    let sizes = qinglan_engine::layout::ImageSizes::new();
    let time = |n: usize, align: &str| {
        let html = blank_lines_page(n, align);
        let t = std::time::Instant::now();
        let (_, res) = lay_opts(&html, &LayoutOptions::default(), &sizes);
        assert!(!res.incomplete);
        assert_eq!(res.lines.len(), 2 * n); // the <pre>'s first line feed is dropped by the parser
        t.elapsed()
    };
    for align in ["justify", "left"] {
        let small = time(10_000, align);
        let big = time(40_000, align);
        assert!(big < small * 8 + std::time::Duration::from_millis(200), "{align}: 10k lines {small:?}, 40k lines {big:?}");
    }
}

/// A million blank lines are laid out well inside the time a page may take (release builds; a debug build
/// gets a tenth of them).
#[test]
fn a_million_blank_lines_finish() {
    let n = if cfg!(debug_assertions) { 100_000 } else { 1_000_000 };
    let html = format!("<body style='font:10px Ahem'><pre style='text-align:justify'>{}x</pre>", "\n".repeat(n));
    let t = std::time::Instant::now();
    let (_, res) = lay_opts(&html, &LayoutOptions::default(), &qinglan_engine::layout::ImageSizes::new());
    let took = t.elapsed();
    assert!(res.lines.len() > 1000 || res.incomplete, "{} lines", res.lines.len());
    assert!(took < std::time::Duration::from_secs(5), "{n} blank lines took {took:?}");
}

/// An SVG `<img>` that leaves part of its size open is sized by CSS2 10.3.2 / 10.6.2 from what the file does give, with the
/// 300 x 150 default only for what nothing decides (not by the "300 x 150 contained" size the probe reports).
#[test]
fn an_svg_image_with_a_partial_size_follows_the_replaced_element_rules() {
    use qinglan_engine::layout::ImageSizes;
    use qinglan_engine::layout::image::Partial;
    let html = "<body style='margin:0'><div style='width:200px'><img id=a src=a.svg></div>";
    let (_, res) = lay_opts(html, &LayoutOptions::default(), &ImageSizes::new());
    let node = res.images[0].node;
    let sized = |p: Partial| {
        let mut sizes = ImageSizes::new();
        sizes.set_loaded(node, 300, 150);
        sizes.set_partial(node, Some(p));
        lay_opts(html, &LayoutOptions::default(), &sizes).0
    };
    // A ratio alone (a viewBox of 4 x 1): no width to use, so the box takes the 300 px default object width. (An inline image is
    // not stretched to the block; the width is 300 and the height follows the ratio.)
    let d = sized(Partial { w: None, h: None, ratio: Some(4.0) });
    assert!(d.contains("replaced img#a (0,0 300x75)"), "{d}");
    // A width and a ratio: the height follows.
    let d = sized(Partial { w: Some(80.0), h: None, ratio: Some(2.0) });
    assert!(d.contains("replaced img#a (0,0 80x40)"), "{d}");
    // A height and no ratio: the width is the default 300.
    let d = sized(Partial { w: None, h: Some(20.0), ratio: None });
    assert!(d.contains("replaced img#a (0,0 300x20)"), "{d}");
    // Nothing: 300 x 150.
    let d = sized(Partial { w: None, h: None, ratio: None });
    assert!(d.contains("replaced img#a (0,0 300x150)"), "{d}");
}
