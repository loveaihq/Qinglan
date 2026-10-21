//! Vertical pages scroll along their lines (step 2d-2c): the range is horizontal, the start is the right edge for `vertical-rl`,
//! the leftmost column can be reached, hit testing and scroll anchoring count the scroll offset, a horizontal page is as before.
#![allow(clippy::indexing_slicing, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use qinglan_engine::css::{NoResources, StyleOptions, style_document_with};
use qinglan_engine::encoding::decode_document;
use qinglan_engine::html::dom::Document;
use qinglan_engine::html::treebuilder::parse_document;
use qinglan_engine::interact::{PageIndex, link_target};
use qinglan_engine::layout::{LayoutOptions, LayoutResult, layout_document};
use qinglan_engine::paint::images::NoBitmaps;
use qinglan_engine::paint::raster::{Frame, View, render_frame};
use qinglan_engine::paint::{DisplayList, ScrollAxis, build_display_list};
use qinglan_engine::platform::{BasicPlatform, Platform, TestPlatform};

const RED: [u8; 4] = [0, 0, 255, 255];
const GREEN: [u8; 4] = [0, 255, 0, 255];
const BLUE: [u8; 4] = [255, 0, 0, 255];

const VW: f32 = 400.0;
const VH: f32 = 100.0;
/// 40 columns of 50 x 100 px: the first red, the last green (with a link in it), the others blue.
const COLUMNS: usize = 40;

struct Lay {
    doc: Document,
    res: LayoutResult,
    list: DisplayList,
    index: PageIndex,
}

fn lay(html: &str) -> Lay {
    let platform = TestPlatform;
    let decoded = decode_document(html.as_bytes(), None, &BasicPlatform);
    let mut doc = parse_document(decoded.text);
    let sopts = StyleOptions { viewport_w: VW, viewport_h: VH, ..StyleOptions::default() };
    let styled = style_document_with(&mut doc, None, &mut NoResources, &sopts, Some(&platform as &dyn Platform));
    let res = layout_document(&doc, &styled.tree, &platform, &LayoutOptions { viewport_w: VW, viewport_h: VH, ..LayoutOptions::default() });
    let list = build_display_list(&doc, &res, &platform);
    let index = PageIndex::new(&res);
    Lay { doc, res, list, index }
}

fn columns(mode: &str) -> Lay {
    let mut html = format!("<body style='margin:0;writing-mode:{mode};font:10px/10px sans-serif'>");
    for i in 0..COLUMNS {
        let (bg, text) = match i {
            0 => ("#f00", "a".to_string()),
            i if i == COLUMNS - 1 => ("#0f0", "<a href=end>z</a>".to_string()),
            _ => ("#00f", "b".to_string()),
        };
        html.push_str(&format!("<div style='width:50px;height:100px;background:{bg}'>{text}</div>"));
    }
    lay(&html)
}

/// The frame of the viewport scrolled `p` px along the page's axis.
fn frame_at(l: &Lay, p: f32) -> Frame {
    let (scroll_x, scroll_y) = l.list.axis.offset(p);
    let mut frame = Frame::new(VW as u32, VH as u32);
    render_frame(&l.list, &TestPlatform, &mut NoBitmaps, View { scroll_x, scroll_y, dpr: 1.0 }, &mut frame);
    frame
}

fn at(f: &Frame, x: u32, y: u32) -> [u8; 4] {
    f.pixel(x, y).unwrap_or([9, 9, 9, 9])
}

#[test]
fn vertical_rl_starts_at_the_right_edge_and_the_leftmost_column_can_be_reached() {
    let l = columns("vertical-rl");
    assert_eq!(l.list.axis, ScrollAxis::Left);
    // 40 columns of 50 px end at the right edge of the viewport: 2000 px of page, 400 shown, 1600 to scroll.
    let max = l.list.max_progress(VW, VH);
    assert!((max - 1600.0).abs() < 1.0, "range {max}");
    let start = frame_at(&l, 0.0);
    assert_eq!(at(&start, 390, 50), RED, "the first column is at the right edge");
    assert_ne!(at(&start, 10, 50), GREEN, "the last column is out of view at the start");
    let end = frame_at(&l, max);
    assert_eq!(at(&end, 10, 50), GREEN, "the last column is at the left edge after scrolling to the end");
    assert_eq!(at(&end, 390, 50), BLUE);
    // Half way: still a blue column in the middle, no red or green.
    let mid = frame_at(&l, 800.0);
    assert_eq!(at(&mid, 200, 50), BLUE);
}

#[test]
fn vertical_lr_starts_at_the_left_edge_and_the_last_column_is_on_the_right_at_the_end() {
    let l = columns("vertical-lr");
    assert_eq!(l.list.axis, ScrollAxis::Right);
    let max = l.list.max_progress(VW, VH);
    assert!((max - 1600.0).abs() < 1.0, "range {max}");
    assert_eq!(at(&frame_at(&l, 0.0), 10, 50), RED);
    assert_eq!(at(&frame_at(&l, max), 390, 50), GREEN);
}

#[test]
fn horizontal_pages_are_as_before() {
    let l = lay("<body style='margin:0'><div style='height:900px;background:#f00'></div>");
    assert_eq!(l.list.axis, ScrollAxis::Down);
    assert_eq!(l.list.max_progress(VW, VH), 900.0 - VH);
    assert_eq!(ScrollAxis::Down.offset(30.0), (0.0, 30.0));
    // A page wider than the viewport is not scrolled sideways.
    let wide = lay("<body style='margin:0'><div style='width:3000px;height:10px;background:#f00'></div>");
    assert_eq!(wide.list.max_progress(VW, VH), 0.0);
}

#[test]
fn a_link_in_the_leftmost_column_is_hit_after_scrolling() {
    let l = columns("vertical-rl");
    let max = l.list.max_progress(VW, VH);
    let (ox, oy) = l.list.axis.offset(max);
    let hit = |p: (f32, f32), off: (f32, f32)| {
        l.index.link_at_in(&l.doc, &l.res, (p.0 + off.0, p.1 + off.1), p).and_then(|n| link_target(&l.doc, n)).map(|t| t.0)
    };
    let probe = |off: (f32, f32)| (0..50).find_map(|x| hit((x as f32, 5.0), off));
    assert_eq!(probe((ox, oy)).as_deref(), Some("end"), "the link is under the pointer at the end");
    assert_eq!(probe((0.0, 0.0)), None, "and not at the start");
}

#[test]
fn anchoring_along_the_axis_keeps_the_first_visible_column() {
    let l = columns("vertical-rl");
    // The 3rd column (index 2) spans 100..150 along the axis, its first (only) line 100..110.
    let (node, offset) = l.index.anchor_on(&l.res, ScrollAxis::Left, VW, 100.0).expect("anchor");
    let start = l.index.node_start(&l.res, node, ScrollAxis::Left, VW).unwrap();
    assert!((start - 100.0).abs() < 1.0, "start {start}");
    assert!(offset.abs() < 1.0, "offset {offset}");
    // Scrolled 5 px into that line: the same node, 5 px before the edge.
    let (n2, o2) = l.index.anchor_on(&l.res, ScrollAxis::Left, VW, 105.0).expect("anchor");
    assert_eq!(n2, node);
    assert!((o2 + 5.0).abs() < 1.0, "offset {o2}");
    // Going the other way (vertical-lr) measures from the left edge.
    let r = columns("vertical-lr");
    let (node, _) = r.index.anchor_on(&r.res, ScrollAxis::Right, VW, 100.0).expect("anchor");
    let start = r.index.node_start(&r.res, node, ScrollAxis::Right, VW).unwrap();
    assert!((start - 100.0).abs() < 1.0, "start {start}");
}
