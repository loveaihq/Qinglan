//! Link hit testing, `#fragment` targets, element positions, scroll anchoring and titles, with the test platform
//! (every glyph a 1em square, so the positions are exact).
#![allow(clippy::indexing_slicing, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use qinglan_engine::css::{NoResources, StyleOptions, style_document_with};
use qinglan_engine::html::dom::Document;
use qinglan_engine::html::treebuilder::parse_document;
use qinglan_engine::interact::{PageIndex, document_title, fragment_target, link_target};
use qinglan_engine::layout::image::ImageSizes;
use qinglan_engine::layout::{LayoutOptions, LayoutResult, layout_document_with};
use qinglan_engine::platform::{Platform, TestPlatform};

struct Lay {
    doc: Document,
    res: LayoutResult,
    index: PageIndex,
}

fn lay(html: &str, sizes: &ImageSizes) -> Lay {
    let platform = TestPlatform;
    let mut doc = parse_document(html.to_string());
    let sopts = StyleOptions { viewport_w: 400.0, viewport_h: 300.0, ..StyleOptions::default() };
    let styled = style_document_with(&mut doc, None, &mut NoResources, &sopts, Some(&platform as &dyn Platform));
    let res = layout_document_with(&doc, &styled.tree, &platform, &LayoutOptions { viewport_w: 400.0, viewport_h: 300.0, ..LayoutOptions::default() }, sizes);
    let index = PageIndex::new(&res);
    Lay { doc, res, index }
}

const PAGE: &str = "<body style='margin:0'><p style='margin:0'>go <a href=' /x ' id=l target=_BLANK>link</a></p>\
    <div style='height:1000px'></div><div id=t>target</div><a name=n>named</a><p id='\u{4e2d}'>zh</p>";

#[test]
fn link_under_the_pointer() {
    let l = lay(PAGE, &ImageSizes::new());
    // 16px glyphs: "go " is 48 px wide, "link" spans x 48..112 on the first line (y 0..16 or so).
    let hit = l.index.link_at(&l.doc, &l.res, 60.0, 5.0, 5.0).expect("link");
    assert_eq!(link_target(&l.doc, hit), Some(("/x".to_string(), true)));
    assert!(l.index.link_at(&l.doc, &l.res, 10.0, 5.0, 5.0).is_none(), "the text before the link");
    assert!(l.index.link_at(&l.doc, &l.res, 200.0, 5.0, 5.0).is_none(), "right of the link");
    assert!(l.index.link_at(&l.doc, &l.res, 60.0, 500.0, 500.0).is_none(), "below it");
}

#[test]
fn fragment_targets() {
    let l = lay(PAGE, &ImageSizes::new());
    let t = fragment_target(&l.doc, "t").expect("id");
    assert!(l.index.node_y(&l.res, t).unwrap() >= 1000.0);
    let n = fragment_target(&l.doc, "n").expect("a name");
    assert!(l.index.node_y(&l.res, n).unwrap() >= 1000.0);
    assert!(fragment_target(&l.doc, "%E4%B8%AD").is_some(), "percent-decoded id");
    assert!(fragment_target(&l.doc, "").is_none());
    assert!(fragment_target(&l.doc, "nothing").is_none());
}

#[test]
fn id_wins_over_name() {
    let l = lay("<a name=x>first</a><p id=x>second</p>", &ImageSizes::new());
    let t = fragment_target(&l.doc, "x").unwrap();
    assert_eq!(l.doc.local_name(t), "p");
}

#[test]
fn titles() {
    let l = lay("<title>  A \n b\t</title><p>x", &ImageSizes::new());
    assert_eq!(document_title(&l.doc), "A b");
    let l = lay("<p>no title", &ImageSizes::new());
    assert_eq!(document_title(&l.doc), "");
    let long = format!("<title>{}</title>", "x".repeat(1000));
    assert_eq!(document_title(&lay(&long, &ImageSizes::new()).doc).chars().count(), 300);
}

#[test]
fn anchoring_keeps_the_text_in_place_when_an_image_above_it_arrives() {
    let html = "<body style='margin:0'><p style='margin:0'>intro</p><img src=a.png><div style='height:200px'></div><p id=p style='margin:0'>one two three four five six seven eight nine ten</p><div style='height:2000px'></div>";
    let before = lay(html, &ImageSizes::new());
    let img = before.res.images[0].node;
    let scroll = 100.0;
    let (node, offset) = before.index.anchor(&before.res, scroll).expect("anchor");
    let y_before = before.index.node_y(&before.res, node).unwrap();
    assert!((y_before - scroll - offset).abs() < 0.01);

    let mut sizes = ImageSizes::new();
    sizes.set_loaded(img, 100, 300);
    let after = lay(html, &sizes);
    // The same document parsed twice gives the same node numbers.
    let y_after = after.index.node_y(&after.res, node).unwrap();
    assert!(y_after >= y_before + 250.0, "the image pushed the text down ({y_before} -> {y_after})");
    let new_scroll = y_after - offset;
    assert!((new_scroll - (scroll + (y_after - y_before))).abs() < 0.01);
}

#[test]
fn hidden_elements_have_no_position() {
    let l = lay("<div id=h style='display:none'>x</div><p>y</p>", &ImageSizes::new());
    let h = fragment_target(&l.doc, "h").unwrap();
    assert!(l.index.node_y(&l.res, h).is_none());
}
