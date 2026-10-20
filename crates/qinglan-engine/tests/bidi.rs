//! Bidirectional text and colour glyphs (step 2d-2a) beyond the layout snapshots (`tests/pages/bidi.html`,
//! `rtl-block.html`): the order of the glyphs in a right-to-left fragment, the left-to-right fast path (no analysis at all),
//! the collapsed borders of an rtl table, and a colour glyph in the frame. All with the test platform: every glyph 10px wide.
#![allow(clippy::indexing_slicing, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::cell::Cell;
use std::rc::Rc;

use qinglan_engine::css::color::Color;
use qinglan_engine::css::{NoResources, StyleOptions, style_document_with};
use qinglan_engine::html::treebuilder::parse_document;
use qinglan_engine::layout::{FragKind, LayoutOptions, LayoutResult, LayoutUnit, layout_document};
use qinglan_engine::paint::images::NoBitmaps;
use qinglan_engine::paint::raster::{Frame, View, render_frame};
use qinglan_engine::paint::build_display_list;
use qinglan_engine::platform::{FontId, FontMetrics, FontQuery, Glyph, GlyphMask, Platform, TestPlatform};

/// The test platform that counts how often the layout asks for a bidi analysis.
struct Counting {
    inner: TestPlatform,
    analyses: Cell<u32>,
}

impl Platform for Counting {
    fn decode_legacy(&self, e: &str, b: &[u8]) -> Option<String> {
        self.inner.decode_legacy(e, b)
    }
    fn resolve_font(&self, q: &FontQuery<'_>) -> Option<FontId> {
        self.inner.resolve_font(q)
    }
    fn font_metrics(&self, f: FontId, s: f32) -> FontMetrics {
        self.inner.font_metrics(f, s)
    }
    fn shape(&self, t: &str, q: &FontQuery<'_>, s: f32) -> Vec<Glyph> {
        self.inner.shape(t, q, s)
    }
    fn line_breaks(&self, t: &str, l: Option<&str>) -> Vec<usize> {
        self.inner.line_breaks(t, l)
    }
    fn glyph_mask(&self, f: FontId, g: u32, s: f32, sub: u8) -> Option<Rc<GlyphMask>> {
        self.inner.glyph_mask(f, g, s, sub)
    }
    fn bidi_levels(&self, t: &str, rtl: bool) -> Vec<u8> {
        self.analyses.set(self.analyses.get() + 1);
        self.inner.bidi_levels(t, rtl)
    }
}

fn lay_doc(html: &str, platform: &dyn Platform) -> (qinglan_engine::html::dom::Document, LayoutResult) {
    let mut doc = parse_document(html.to_string());
    let sopts = StyleOptions { viewport_w: 400.0, viewport_h: 300.0, ..StyleOptions::default() };
    let styled = style_document_with(&mut doc, None, &mut NoResources, &sopts, Some(platform));
    let res = layout_document(&doc, &styled.tree, platform, &LayoutOptions { viewport_w: 400.0, viewport_h: 300.0, ..LayoutOptions::default() });
    (doc, res)
}

fn lay_with(html: &str, platform: &dyn Platform) -> LayoutResult {
    lay_doc(html, platform).1
}

fn lay(html: &str) -> LayoutResult {
    lay_with(html, &TestPlatform)
}

#[test]
fn a_right_to_left_fragment_draws_its_glyphs_in_visual_order() {
    // bidi-override rtl: the units are a b c (logical) and are shown c b a; the glyphs of the fragment are c, b, a left to right.
    let r = lay("<!doctype html><body style=\"margin:0;font:10px/10px Ahem\"><div style=\"width:100px;unicode-bidi:bidi-override;direction:rtl\">abc</div>");
    let f = r.fragments.iter().find(|f| matches!(f.kind, FragKind::Text { .. })).expect("a text fragment");
    let FragKind::Text { glyphs, start, end, .. } = f.kind else { panic!() };
    let drawn: String = r.glyphs[glyphs.0 as usize..glyphs.1 as usize].iter().map(|g| char::from_u32(g.glyph.into()).unwrap()).collect();
    assert_eq!(drawn, "cba");
    assert_eq!((start, end), (0, 3), "the fragment still says which text it shows");
    assert_eq!((f.x, f.w), (LayoutUnit::px(70), LayoutUnit::px(30)));
}

#[test]
fn a_page_without_right_to_left_text_is_not_analysed() {
    let p = Counting { inner: TestPlatform, analyses: Cell::new(0) };
    // Latin, CJK, a left-to-right isolate and embedding: nothing can change order.
    lay_with("<!doctype html><body style=\"font:10px/10px Ahem\"><p>Hello 中文 world <span dir=ltr>abc</span> <span style=\"unicode-bidi:embed\">def</span></p><p style=\"direction:ltr\">more text</p>", &p);
    assert_eq!(p.analyses.get(), 0);
    // Hebrew in a paragraph asks once per paragraph that has it; an rtl block asks even without Hebrew in it.
    lay_with("<!doctype html><body style=\"font:10px/10px Ahem\"><p>abc אבג</p><p dir=rtl>abc</p><p>no</p>", &p);
    assert_eq!(p.analyses.get(), 2);
    // A bdo makes left-to-right text reorder, and so does a rtl isolate.
    lay_with("<!doctype html><body style=\"font:10px/10px Ahem\"><p>x <bdo dir=rtl>abc</bdo></p><p>y <span dir=rtl>abc</span></p>", &p);
    assert_eq!(p.analyses.get(), 4);
}

#[test]
fn collapsed_borders_of_an_rtl_table_are_mirrored() {
    // Two cells in rtl: the first (red border, 2px) is on the right, the second (blue, 4px) on the left; the wider border wins
    // between them. The outer edges: blue on the left of the table, red on the right.
    let (doc, r) = lay_doc("<!doctype html><body style=\"margin:0;font:10px/10px Ahem\"><style>td{padding:0}</style><table dir=rtl style=\"border-collapse:collapse\"><tr>\
        <td style=\"border:2px solid red\">a</td><td style=\"border:4px solid blue\">b</td></tr></table>", &TestPlatform);
    let t = r.tables.values().next().expect("paint data of the collapsed table");
    let (red, blue) = (Color::rgb(255, 0, 0), Color::rgb(0, 0, 255));
    let min_x = |c: Color| t.segs.iter().filter(|s| s.4 == c).map(|s| s.0).min().expect("a segment");
    let max_x = |c: Color| t.segs.iter().filter(|s| s.4 == c).map(|s| s.0).max().expect("a segment");
    assert!(min_x(blue) < min_x(red), "{:?}", t.segs);
    assert!(max_x(red) > max_x(blue) || min_x(red) > LayoutUnit::px(10), "{:?}", t.segs);
    // The cells themselves: "a" in the right cell.
    let dump = qinglan_engine::layout::dump::dump_layout(&doc, &r);
    let tds: Vec<i32> = dump.lines().filter_map(|l| l.trim_start().strip_prefix("block td (")).filter_map(|l| l.split(',').next()?.parse().ok()).collect();
    assert_eq!(tds.len(), 2, "{dump}");
    assert!(tds[0] > tds[1], "the first cell is on the right: {dump}");
}

#[test]
fn a_colour_glyph_is_drawn_as_it_is() {
    // The test platform draws U+E000 as a colour glyph: left half red, right half blue, opaque, 10px square here.
    let platform = TestPlatform;
    let (doc, r) = lay_doc("<!doctype html><body style=\"margin:0;font:10px/10px Ahem;color:green\"><p style=\"margin:0\">\u{E000}x</p>", &platform);
    let list = build_display_list(&doc, &r, &platform);
    let mut frame = Frame::new(40, 20);
    render_frame(&list, &platform, &mut NoBitmaps, View { scroll_y: 0.0, dpr: 1.0 }, &mut frame);
    // Frame pixels are BGRA. The text colour (green) plays no part in the emoticon, the "x" next to it is green.
    assert_eq!(frame.pixel(2, 4), Some([0, 0, 255, 255]));
    assert_eq!(frame.pixel(7, 4), Some([255, 0, 0, 255]));
    assert_eq!(frame.pixel(15, 4), Some([0, 128, 0, 255]));
}
