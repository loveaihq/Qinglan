//! Tables beyond the hand-computed pages: the collapsed borders that get painted, and the limits (nesting, spans, columns).
#![allow(clippy::indexing_slicing, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use qinglan_engine::css::color::Color;
use qinglan_engine::css::{NoResources, StyleOptions, style_document_with};
use qinglan_engine::html::treebuilder::parse_document;
use qinglan_engine::layout::{LayoutOptions, LayoutResult, LayoutUnit, layout_document};
use qinglan_engine::platform::{Platform, TestPlatform};

fn lay(html: &str) -> LayoutResult {
    let platform = TestPlatform;
    let mut doc = parse_document(html.to_string());
    let sopts = StyleOptions { viewport_w: 800.0, viewport_h: 600.0, ..StyleOptions::default() };
    let styled = style_document_with(&mut doc, None, &mut NoResources, &sopts, Some(&platform as &dyn Platform));
    layout_document(&doc, &styled.tree, &platform, &LayoutOptions { viewport_w: 800.0, viewport_h: 600.0, ..LayoutOptions::default() })
}

#[test]
fn collapsed_edges_are_painted_where_the_wider_border_wins() {
    let r = lay("<!doctype html><body style=\"margin:0;font:10px/10px Ahem\"><style>td{padding:0}</style><table style=\"border-collapse:collapse\"><tr>\
        <td style=\"border:2px solid red\">a</td><td style=\"border:4px solid blue\">b</td></tr></table>");
    let t = r.tables.values().next().expect("the collapsed table has paint data");
    let px = LayoutUnit::px;
    let blue = Color::rgb(0, 0, 255);
    // The edge between the cells is blue and 4px wide, centred on x = 1 + 13 = 14: from 12 to 16.
    assert!(t.segs.iter().any(|s| s.4 == blue && s.0 == px(12) && s.2 == px(4)), "{:?}", t.segs);
    // No red vertical edge is left in the middle.
    let red = Color::rgb(255, 0, 0);
    assert!(!t.segs.iter().any(|s| s.4 == red && s.2 == px(2) && s.0 > px(10) && s.0 < px(16)), "{:?}", t.segs);
}

#[test]
fn spans_columns_and_nesting_are_bounded() {
    // colspan / rowspan far beyond the caps, 3000 cells in a row, 150 tables inside one another.
    let mut html = String::from("<body style=\"font:10px/10px Ahem\"><table><tr><td colspan=100000 rowspan=100000>x</td><td>y</td></tr></table><table><tr>");
    for _ in 0..3000 {
        html.push_str("<td>c</td>");
    }
    html.push_str("</tr></table>");
    for _ in 0..150 {
        html.push_str("<table><tr><td>");
    }
    html.push('z');
    for _ in 0..150 {
        html.push_str("</td></tr></table>");
    }
    let r = lay(&html);
    assert!(r.box_out(r.tree.root).h > LayoutUnit::ZERO);
}

#[test]
fn table_parts_inside_an_inline_box_make_an_inline_table_that_does_not_split_it() {
    let r = lay("<!doctype html><body style=\"margin:0;font:10px/10px Ahem\"><span>a<span style=\"display:table-cell\">b</span><span style=\"display:table-cell\">c</span>d</span>");
    // One line: a, the table (b c), d side by side, 10 high.
    let body = r.tree.get(r.tree.root).children[0];
    let o = r.box_out(body);
    assert_eq!(o.h.to_f32(), 10.0);
}
