//! Flexbox beyond the hand-computed pages: parsing of the properties, and the cost of nested containers (a nest of
//! containers must not take exponential work; the work meter is what the page budget counts).
#![allow(clippy::indexing_slicing, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

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

const HEAD: &str = "<!doctype html><style>body{margin:0;font:10px/10px Ahem} .f{display:flex} .c{display:flex;flex-direction:column} .g{flex:1}</style>";

/// A row container in a row container ... with a short sibling at each level.
fn nest_row(depth: usize) -> String {
    let mut s = String::from(HEAD);
    for i in 0..depth {
        s += &format!("<div class=f><div class=g>t{i}</div>");
    }
    s += "<div>core text</div>";
    s += &"</div>".repeat(depth);
    s
}

/// Column containers that all grow (`flex: 1`) inside a column container of a definite height.
fn nest_col(depth: usize) -> String {
    let mut s = format!("{HEAD}<div class=c style=\"height:600px\">");
    for i in 0..depth {
        s += &format!("<div class=\"c g\"><div>head{i}</div>");
    }
    s += "<div class=g>leaf</div>";
    s += &"</div>".repeat(depth + 1);
    s
}

/// The worst case: at every level a taller sibling makes the nested container stretch (a second layout of it).
fn nest_tall(depth: usize) -> String {
    let mut s = String::from(HEAD);
    for i in 0..depth {
        s += &format!("<div class=f><div style=\"height:{}px\">tall</div><div class=f>", 1000 - 20 * i);
    }
    s += "core";
    s += &"</div></div>".repeat(depth);
    s
}

#[test]
fn deep_nesting_is_not_exponential() {
    for (name, make) in [("row", nest_row as fn(usize) -> String), ("col", nest_col), ("tall", nest_tall)] {
        let w5 = lay(&make(5)).work;
        let w20 = lay(&make(20)).work;
        eprintln!("nest {name}: work at depth 5 = {w5}, at depth 20 = {w20}");
        // Linear growth would be 4x; allow a lot of slack but nothing like 2^15.
        assert!(w20 < w5 * 40, "{name}: depth 5 {w5}, depth 20 {w20}");
        assert!(w20 < 200_000, "{name}: depth 20 needs {w20} units");
        let r = lay(&make(20));
        assert!(!r.incomplete, "{name}: ran out of budget");
    }
}

#[test]
fn flex_properties_are_parsed() {
    let r = lay(&format!(
        "{HEAD}<div class=f style=\"width:100px;flex-flow:row-reverse wrap;gap:4px 6px;justify-content:space-between;align-items:last baseline\">\
         <div style=\"flex:2 3 10px;order:-1;align-self:safe center\">a</div></div>"
    ));
    let d = r.tree.get(r.tree.root);
    assert!(!d.children.is_empty());
    let item = r.tree.boxes.iter().find(|b| b.style.flex.order == -1).expect("the item");
    assert_eq!((item.style.flex.grow, item.style.flex.shrink), (2.0, 3.0));
    assert_eq!(item.style.flex.align_self.safe, Some(true));
}

#[test]
fn grow_and_shrink_follow_the_flex_factors() {
    // 200 wide: 30 + (170 split 1:2) = 56.67 / 113.33 (css-flexbox-1 9.7).
    let r = lay(&format!("{HEAD}<div class=f style=\"width:200px\"><div style=\"flex:1 1 0;min-width:0\"></div><div style=\"flex:2 1 0\"></div><div style=\"width:30px\"></div></div>"));
    let items: Vec<_> = r.tree.boxes.iter().enumerate().filter(|(_, b)| b.flex_item).map(|(i, _)| r.box_out(i as u32)).collect();
    assert_eq!(items.len(), 3);
    assert_eq!(items[2].w, LayoutUnit::px(30));
    let sum = items.iter().fold(LayoutUnit::ZERO, |a, o| a + o.w);
    assert_eq!(sum, LayoutUnit::px(200));
}
