//! Grid layout beyond the hand-computed pages: parsing of the properties, the caps on tracks and repetitions, and the cost of
//! nested containers (a nest of grids must not take exponential work; the work meter is what the page budget counts).
#![allow(clippy::indexing_slicing, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use qinglan_engine::css::types::{Bread, GRID_MAX_TRACKS, GridLine};
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

const HEAD: &str = "<!doctype html><style>body{margin:0;font:10px/10px Ahem} .g{display:grid}</style>";

/// A grid in a grid ... with a short sibling at each level; the columns are `1fr auto`.
fn nest_cols(depth: usize) -> String {
    let mut s = String::from(HEAD);
    for i in 0..depth {
        s += &format!("<div class=g style=\"grid-template-columns:1fr auto\"><div>t{i}</div><div class=g style=\"grid-template-columns:1fr auto\"><div>u{i}</div>");
    }
    s += "<div>core text</div>";
    s += &"</div></div>".repeat(depth);
    s
}

/// Rows `auto 1fr` with a minimum height: every level needs the height of the next one (a throw-away layout) and the
/// minimum height makes it size the rows twice.
fn nest_rows(depth: usize) -> String {
    let mut s = String::from(HEAD);
    for i in 0..depth {
        s += &format!("<div class=g style=\"grid-template-rows:auto 1fr;min-height:{}px\"><div>head{i}</div><div class=g style=\"grid-template-rows:auto 1fr\">", 40 + i);
    }
    s += "core";
    s += &"</div></div>".repeat(depth);
    s
}

/// The nested grid spans three auto columns and its content depends on their widths.
fn nest_span(depth: usize) -> String {
    let mut s = String::from(HEAD);
    for i in 0..depth {
        s += &format!("<div class=g style=\"grid-template-columns:repeat(3,auto)\"><div style=\"grid-column:1/4\">a{i} b{i} c{i}</div><div class=g style=\"grid-column:2/4;grid-template-columns:auto 1fr\"><div>x</div>");
    }
    s += "core";
    s += &"</div></div>".repeat(depth);
    s
}

#[test]
fn deep_nesting_is_not_exponential() {
    for (name, make) in [("cols", nest_cols as fn(usize) -> String), ("rows", nest_rows), ("span", nest_span)] {
        let w5 = lay(&make(5)).work;
        let w20 = lay(&make(20)).work;
        eprintln!("grid nest {name}: work at depth 5 = {w5}, at depth 20 = {w20}");
        // Linear growth would be 4x; allow a lot of slack but nothing like 2^15.
        assert!(w20 < w5 * 40, "{name}: depth 5 {w5}, depth 20 {w20}");
        assert!(w20 < 200_000, "{name}: depth 20 needs {w20} units");
        let r = lay(&make(20));
        assert!(!r.incomplete, "{name}: ran out of budget");
    }
}

#[test]
fn grid_properties_are_parsed() {
    let r = lay(&format!(
        "{HEAD}<div class=g style='grid-template-columns:[a] 1fr repeat(2,minmax(10px,2fr)) [b c];grid-template-rows:fit-content(20px) auto;\
         grid-template-areas:\"x y\" \"x z\";grid-auto-flow:column dense;grid-auto-rows:30px 40px;justify-items:end;place-content:center start'>\
         <div style='grid-area:2/1/span 2/auto'></div></div>"
    ));
    let c = r.tree.boxes.iter().find(|b| b.style.grid.cols.is_some()).expect("the container");
    let g = &c.style.grid;
    let cols = g.cols.as_ref().unwrap();
    assert_eq!(cols.tracks.len(), 3);
    assert_eq!(cols.tracks[0].max, Bread::Fr(1.0));
    assert_eq!(cols.names[0].iter().map(|s| &**s).collect::<Vec<_>>(), ["a"]);
    assert_eq!(cols.names[3].iter().map(|s| &**s).collect::<Vec<_>>(), ["b", "c"]);
    assert!(g.rows.as_ref().unwrap().tracks[0].fit);
    let areas = g.areas.as_ref().unwrap();
    assert_eq!(areas.list.len(), 3);
    assert_eq!((&*areas.list[0].0, areas.list[0].1, areas.list[0].3), ("x", 1, 3));
    assert!(g.flow_col && g.dense);
    assert_eq!(g.auto_rows.len(), 2);
    let it = r.tree.boxes.iter().find(|b| b.style.grid.line[0] != GridLine::Auto).expect("the item");
    assert_eq!(it.style.grid.line[0], GridLine::Num(2));
    assert_eq!(it.style.grid.line[2], GridLine::Span(2));
    assert_eq!(it.style.grid.line[3], GridLine::Auto);
}

#[test]
fn invalid_grid_values_are_dropped() {
    let r = lay(&format!(
        "{HEAD}<div class=g id=x style='grid-template-areas:\"a b\" \"c\";grid-template-columns:repeat(auto-fill,1fr);grid-column:0;grid-auto-flow:row row'></div>"
    ));
    let c = r.tree.boxes.iter().find(|b| b.style.boxs.display == qinglan_engine::css::types::Display::Grid).expect("the container");
    assert!(c.style.grid.areas.is_none(), "an area that is not a rectangle");
    assert!(c.style.grid.cols.is_none(), "a flexible track in an auto repeat");
    assert_eq!(c.style.grid.line[1], GridLine::Auto);
    assert!(!c.style.grid.flow_col);
}

#[test]
fn tracks_and_repetitions_are_capped() {
    // 100 000 repetitions are cut at the cap; the page still lays out.
    let r = lay(&format!("{HEAD}<div class=g style='width:5000px;grid-template-columns:repeat(100000,1px)'><div></div></div>"));
    let c = r.tree.boxes.iter().find(|b| b.style.grid.cols.is_some()).expect("the container");
    assert_eq!(c.style.grid.cols.as_ref().unwrap().tracks.len(), GRID_MAX_TRACKS);
    assert!(!r.incomplete);
    // An auto-fill that would need 5000 tracks gets as many as the cap allows.
    let r = lay(&format!("{HEAD}<div class=g style='width:5000px;grid-template-columns:repeat(auto-fill,1px)'><div></div></div>"));
    assert!(!r.incomplete);
    // 3000 items in one column: the rows stop at the cap and nothing blows up.
    let mut s = format!("{HEAD}<div class=g style='grid-auto-rows:1px'>");
    s += &"<div></div>".repeat(3000);
    s += "</div>";
    let r = lay(&s);
    assert!(!r.incomplete);
    assert!(r.work < 400_000, "{} units", r.work);
    let h = r.box_out(r.tree.boxes.iter().position(|b| b.style.boxs.display == qinglan_engine::css::types::Display::Grid).unwrap() as u32).h;
    assert_eq!(h, LayoutUnit::px(GRID_MAX_TRACKS as i32));
}

#[test]
fn spanning_items_share_what_the_tracks_lack() {
    // Two auto columns; the spanning item needs 70 (min-content "aaaaaaa" = 70), the first track already has 20: the
    // second gets the rest (11.5.1: equal shares up to the limits, then the remainder).
    let r = lay(&format!(
        "{HEAD}<div class=g style='width:200px;grid-template-columns:auto auto'><div id=a style='height:10px;width:20px'></div><div id=b style='grid-column:1/3;height:10px'>aaaaaaa</div></div>"
    ));
    let ids: Vec<_> = r.tree.boxes.iter().enumerate().filter(|(_, b)| b.flex_item).map(|(i, _)| i as u32).collect();
    assert_eq!(ids.len(), 2);
    let (a, b) = (r.box_out(ids[0]), r.box_out(ids[1]));
    assert_eq!(b.w, LayoutUnit::px(200), "the spanning item fills both tracks, which stretch to the container (11.8)");
    assert!(a.w >= LayoutUnit::px(20));
}
