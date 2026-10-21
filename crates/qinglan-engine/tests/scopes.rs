//! Scroll containers, sticky boxes, the page's own sideways range and the form control looks (step 2e-2). Every number is
//! worked out by hand in the comment of its test.
#![allow(clippy::indexing_slicing, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use qinglan_engine::css::{NoResources, StyleOptions, style_document_with};
use qinglan_engine::encoding::decode_document;
use qinglan_engine::html::dom::Document;
use qinglan_engine::html::treebuilder::parse_document;
use qinglan_engine::interact::PageIndex;
use qinglan_engine::layout::{LayoutOptions, LayoutResult, layout_document};
use qinglan_engine::paint::images::NoBitmaps;
use qinglan_engine::paint::raster::{Frame, View, render_frame};
use qinglan_engine::paint::{DisplayList, Press, ScopeKind, build_display_list};
use qinglan_engine::platform::{BasicPlatform, Platform, TestPlatform};

const RED: [u8; 4] = [0, 0, 255, 255];
const GREEN: [u8; 4] = [0, 255, 0, 255];
const BLUE: [u8; 4] = [255, 0, 0, 255];
const WHITE: [u8; 4] = [255, 255, 255, 255];
const GREY: [u8; 4] = [0x88, 0x88, 0x88, 255];
const THUMB: [u8; 4] = [0x8a, 0x8a, 0x8a, 255];
const ACCENT: [u8; 4] = [0xff, 0x75, 0x00, 255];

struct Lay {
    doc: Document,
    res: LayoutResult,
    list: DisplayList,
}

fn lay(html: &str) -> Lay {
    let platform = TestPlatform;
    let decoded = decode_document(html.as_bytes(), None, &BasicPlatform);
    let mut doc = parse_document(decoded.text);
    let sopts = StyleOptions { viewport_w: 400.0, viewport_h: 300.0, ..StyleOptions::default() };
    let styled = style_document_with(&mut doc, None, &mut NoResources, &sopts, Some(&platform as &dyn Platform));
    let res = layout_document(&doc, &styled.tree, &platform, &LayoutOptions { viewport_w: 400.0, viewport_h: 300.0, ..LayoutOptions::default() });
    let list = build_display_list(&doc, &res, &platform);
    Lay { doc, res, list }
}

fn shot(l: &Lay, scroll_x: f32, scroll_y: f32) -> Frame {
    let mut frame = Frame::new(400, 300);
    render_frame(&l.list, &TestPlatform, &mut NoBitmaps, View { scroll_x, scroll_y, dpr: 1.0 }, &mut frame);
    frame
}

fn at(f: &Frame, x: u32, y: u32) -> [u8; 4] {
    f.pixel(x, y).unwrap_or([9, 9, 9, 9])
}

const BOX: &str = "<body style='margin:0'><div style='width:100px;height:100px;overflow:auto'>\
    <div style='height:100px;background:#f00'></div><div style='height:100px;background:#0f0'></div><div style='height:100px;background:#00f'></div></div>\
    <div style='height:50px;background:#888'></div></body>";

#[test]
fn a_scroll_container_clips_and_shows_the_part_at_its_offset() {
    let l = lay(BOX);
    // One scroll container: the 100 x 100 box; 300 px of content, so it scrolls 200 px. The page is the box plus the grey bar.
    assert_eq!(l.list.scopes.len(), 1);
    let s = &l.list.scopes[0];
    assert!(matches!(s.kind, ScopeKind::Scroll([1, 1])));
    assert_eq!(s.max(), (0.0, 200.0));
    assert_eq!(l.list.height.to_f32(), 150.0);
    let f = shot(&l, 0.0, 0.0);
    assert_eq!(at(&f, 10, 10), RED);
    assert_eq!(at(&f, 10, 99), RED);
    assert_eq!(at(&f, 10, 100), GREY);
    assert_eq!(at(&f, 150, 10), WHITE);
    // 100 px down: the second block is in view and nothing of the third shows below the box.
    l.list.set_scroll(s.node, 0.0, 100.0);
    let f = shot(&l, 0.0, 0.0);
    assert_eq!((at(&f, 10, 10), at(&f, 10, 99), at(&f, 10, 100)), (GREEN, GREEN, GREY));
    // Further than the content goes: stops at 200, the last block fills the box.
    l.list.set_scroll(s.node, 0.0, 999.0);
    assert_eq!(l.list.scopes[0].off.get(), (0.0, 200.0));
    assert_eq!(at(&shot(&l, 0.0, 0.0), 10, 10), BLUE);
}

#[test]
fn the_bar_is_a_thumb_over_the_right_edge_that_moves_with_the_offset() {
    let l = lay(BOX);
    // Thumb: 100 * 100 / 300 = 33.3 px long at the top, 6 px wide at x = 100 - 12 + 3 = 91 .. 97.
    assert_eq!(at(&shot(&l, 0.0, 0.0), 94, 10), THUMB);
    assert_eq!(at(&shot(&l, 0.0, 0.0), 94, 60), RED);
    // At the end it sits at the bottom: 100 - 33.3 = 66.7 .. 100.
    l.list.set_scroll(l.list.scopes[0].node, 0.0, 200.0);
    let f = shot(&l, 0.0, 0.0);
    assert_eq!((at(&f, 94, 90), at(&f, 94, 10)), (THUMB, BLUE));
    // Dragging the thumb 30 px (it was taken at 10 px from its start): 30 / (100 - 33.3) * 200 = 90 px.
    l.list.set_scroll(l.list.scopes[0].node, 0.0, 0.0);
    let Some(Press::Thumb { scope, axis, grab }) = l.list.bar_press((94.0, 10.0)) else { panic!("no thumb") };
    assert_eq!((scope, axis), (0, 1));
    l.list.bar_drag(scope, axis, grab, (94.0, 40.0));
    assert!((l.list.scopes[0].off.get().1 - 90.0).abs() < 1.0, "{:?}", l.list.scopes[0].off.get());
    // A press on the track below the thumb pages 87.5 px.
    l.list.set_scroll(l.list.scopes[0].node, 0.0, 0.0);
    assert!(matches!(l.list.bar_press((94.0, 80.0)), Some(Press::Track)));
    assert_eq!(l.list.scopes[0].off.get().1, 87.5);
}

#[test]
fn hidden_is_clipped_but_not_a_scroll_container() {
    let l = lay("<body style='margin:0'><div style='width:100px;height:100px;overflow:hidden'><div style='height:300px;background:#f00'></div></div></body>");
    assert!(l.list.scopes.is_empty());
    assert_eq!(l.list.wheel((10.0, 10.0), 0.0, 50.0), (0.0, 50.0));
    assert_eq!(at(&shot(&l, 0.0, 0.0), 10, 150), WHITE);
}

#[test]
fn the_wheel_goes_to_the_innermost_container_that_can_move_then_to_the_one_around_it() {
    let l = lay("<body style='margin:0'><div style='width:200px;height:100px;overflow:auto'>\
        <div style='width:200px;height:100px;overflow:auto'><div style='height:300px;background:#f00'></div></div>\
        <div style='height:200px;background:#00f'></div></div></body>");
    assert_eq!(l.list.scopes.len(), 2);
    // The inner one (index 1) has 200 px to go, the outer one 200 px (100 + 200 of content in 100).
    assert_eq!(l.list.wheel((10.0, 10.0), 0.0, 250.0), (0.0, 0.0));
    assert_eq!(l.list.scopes[1].off.get(), (0.0, 200.0));
    assert_eq!(l.list.scopes[0].off.get(), (0.0, 50.0));
    // The inner one is at its end: the outer one takes the next 150 and 350 are left for the page.
    assert_eq!(l.list.wheel((10.0, 10.0), 0.0, 500.0), (0.0, 350.0));
    assert_eq!(l.list.scopes[0].off.get(), (0.0, 200.0));
    // Outside the boxes nothing moves.
    assert_eq!(l.list.wheel((300.0, 10.0), 0.0, 10.0), (0.0, 10.0));
}

#[test]
fn offsets_are_carried_to_the_next_layout_by_node() {
    let l = lay(BOX);
    l.list.set_scroll(l.list.scopes[0].node, 0.0, 120.0);
    let offs = l.list.offsets();
    let again = lay(BOX);
    again.list.restore(&offs);
    assert_eq!(again.list.scopes[0].off.get(), (0.0, 120.0));
}

#[test]
fn a_link_in_a_scrolled_container_is_hit_where_it_is_drawn() {
    let l = lay("<body style='margin:0;font:10px/10px sans-serif'><div style='width:100px;height:100px;overflow:auto'><div style='height:150px'></div><a href=x>ab</a></div></body>");
    let index = PageIndex::new(&l.res);
    let hit = |y: f32| index.link_at_scoped(&l.doc, &l.res, Some(&l.list), (5.0, y), (5.0, y));
    // The link is 150 px down in the box (150 .. 160), below its edge: not hit where it would be on the page.
    assert!(hit(155.0).is_none());
    // The box scrolls 160 - 100 = 60: the link is drawn at 90 .. 100.
    l.list.set_scroll(l.list.scopes[0].node, 0.0, 100.0);
    assert!(hit(95.0).is_some());
    assert!(hit(50.0).is_none());
}

#[test]
fn sticky_sticks_to_the_viewport_and_stays_in_its_containing_block() {
    // The grey bar is 100 high; the wrapper is 400 high from y = 100; the sticky box is 50 high at its top, 10 px from the top.
    let l = lay("<body style='margin:0;height:1000px'><div style='height:100px;background:#888'></div>\
        <div style='height:400px'><div style='position:sticky;top:10px;height:50px;background:#f00'></div></div></body>");
    assert_eq!(l.list.scopes.len(), 1);
    assert_eq!(at(&shot(&l, 0.0, 0.0), 10, 110), RED);
    // Scrolled 150: the view starts at 160, the box is shifted 60 and drawn at 100 + 60 - 150 = 10 .. 60.
    let f = shot(&l, 0.0, 150.0);
    assert_eq!((at(&f, 10, 5), at(&f, 10, 20), at(&f, 10, 59), at(&f, 10, 61)), (WHITE, RED, RED, WHITE));
    // Scrolled 450: the shift would be 360 but the box must stay in its block (ends at 500): 350 -> 450 .. 500 -> 0 .. 50.
    let f = shot(&l, 0.0, 450.0);
    assert_eq!((at(&f, 10, 20), at(&f, 10, 49), at(&f, 10, 51)), (RED, RED, WHITE));
    // Scrolled 500: gone with its block.
    assert_eq!(at(&shot(&l, 0.0, 500.0), 10, 20), WHITE);
}

#[test]
fn sticky_in_a_scroll_container_sticks_to_it_and_a_hidden_one_never_sticks() {
    let l = lay("<body style='margin:0'><div style='width:100px;height:100px;overflow:auto'><div style='height:30px;background:#888'></div>\
        <div style='position:sticky;top:0;height:20px;background:#f00'></div><div style='height:300px;background:#00f'></div></div></body>");
    assert_eq!(l.list.scopes.len(), 2);
    assert_eq!(at(&shot(&l, 0.0, 0.0), 10, 35), RED);
    // Scrolled 100: the box (30 .. 50) is shifted 70 to 100 .. 120 in the content: 0 .. 20 on the screen.
    l.list.set_scroll(l.list.scopes[0].node, 0.0, 100.0);
    let f = shot(&l, 0.0, 0.0);
    assert_eq!((at(&f, 10, 5), at(&f, 10, 25)), (RED, BLUE));
    // In an `overflow: hidden` box (which never scrolls) the box is where it is laid out; the wheel passes through.
    let h = lay("<body style='margin:0;height:1000px'><div style='width:100px;height:100px;overflow:hidden'><div style='position:sticky;top:0;height:20px;background:#f00;margin-top:50px'></div></div></body>");
    assert_eq!(h.list.wheel((10.0, 10.0), 0.0, 30.0), (0.0, 30.0));
    assert_eq!((at(&shot(&h, 0.0, 0.0), 10, 55), at(&shot(&h, 0.0, 40.0), 10, 15)), (RED, RED));
}

#[test]
fn a_page_wider_than_the_viewport_has_a_sideways_range_and_a_bar() {
    let l = lay("<body style='margin:0'><div style='width:600px;height:20px;background:#f00'></div></body>");
    assert_eq!(l.list.x_max.to_f32(), 600.0);
    let (lane, ts, tl, max) = l.list.page_bar(0.0).expect("a bar");
    // 400 of 600 shown: a thumb two thirds of the lane, at the left; the range is 200.
    assert_eq!((lane, ts, max), (400.0, 0.0, 200.0));
    assert!((tl - 400.0 * 400.0 / 600.0).abs() < 0.01);
    let f = shot(&l, 200.0, 0.0);
    assert_eq!(at(&f, 399, 10), RED);
    // The bar is over the bottom edge of the viewport: at x = 133 .. 400, y = 300 - 12 + 3 = 291 .. 297.
    assert_eq!(at(&f, 300, 294), THUMB);
    // `overflow-x: hidden` on the body goes to the viewport: no range, no bar.
    let h = lay("<body style='margin:0;overflow-x:hidden'><div style='width:600px;height:20px;background:#f00'></div></body>");
    assert!(h.list.page_bar(0.0).is_none());
    // Text that ends exactly at the edge does not make a range.
    let t = lay("<body style='margin:0;font:10px/10px sans-serif;text-align:right'>abcdefghij</body>");
    assert!(t.list.page_bar(0.0).is_none());
}

#[test]
fn form_controls_are_drawn_in_the_windows_11_style() {
    let boxes = lay("<body style='margin:0'><input type=checkbox checked style='display:block'><input type=checkbox style='display:block;margin-top:20px'>\
        <input type=radio checked style='display:block;margin-top:20px'><input type=checkbox checked style='display:block;margin-top:20px;appearance:none'></body>");
    // The first box is 13 x 13 at (4, 3): the blue border, the white tick, the blue fill.
    let f = shot(&boxes, 0.0, 0.0);
    assert_eq!((at(&f, 4, 9), at(&f, 7, 9), at(&f, 14, 9)), (ACCENT, WHITE, ACCENT));
    // The second at y = 16 + max(3, 20) = 36 .. 49 (the margins collapse): grey border, white inside.
    assert_eq!((at(&f, 4, 42), at(&f, 10, 42)), ([0x76, 0x76, 0x76, 255], WHITE));
    // The radio button at y = 49 + 20 = 69 .. 82: round, blue dot in the middle, a white ring around it, nothing in the corner.
    assert_eq!((at(&f, 10, 75), at(&f, 4, 69)), (ACCENT, WHITE));
    assert!(at(&f, 10, 70).iter().all(|c| *c >= 0xf0), "{:?}", at(&f, 10, 70));
    // `appearance: none` paints nothing (the box at 102 .. 115).
    assert_eq!(at(&f, 10, 108), WHITE);
    let g = lay("<body style='margin:0'><progress value=0.5 max=1 style='display:block'></progress><select style='display:block;width:100px;height:20px;margin-top:8px'><option>a</option></select></body>");
    let f = shot(&g, 0.0, 0.0);
    // Half of 160: blue to x = 80, the grey track after it.
    assert_eq!((at(&f, 40, 8), at(&f, 120, 8)), (ACCENT, [0xe6, 0xe6, 0xe6, 255]));
    // The select at y = 16 + 8 = 24: its arrow's lowest point is a run at x = 85 + 3 = 88, y = 24 + 8 + 3 = 35.
    assert_eq!(at(&f, 88, 35), [0x40, 0x40, 0x40, 255]);
}

#[test]
fn a_sticky_sidebar_that_scrolls_is_both() {
    // The box is 100 high at y 100, sticky at the top and 50 wide with 300 high content: two scopes, the sticky one outside.
    let l = lay("<body style='margin:0;height:1000px'><div style='height:100px;background:#888'></div><div style='height:400px'>        <div style='position:sticky;top:0;width:50px;height:100px;overflow:auto;background:#f00'><div style='height:300px;width:30px;background:#00f'></div></div></div></body>");
    assert_eq!(l.list.scopes.len(), 2);
    assert!(matches!(l.list.scopes[0].kind, ScopeKind::Sticky(_)) && matches!(l.list.scopes[1].kind, ScopeKind::Scroll(_)));
    assert_eq!((at(&shot(&l, 0.0, 0.0), 10, 150), at(&shot(&l, 0.0, 0.0), 40, 150)), (BLUE, RED));
    // Page scrolled 150: stuck at the top (shifted 50), 0 .. 100 on the screen; its own content scrolled 100 shows the middle block.
    let f = shot(&l, 0.0, 150.0);
    assert_eq!((at(&f, 10, 50), at(&f, 40, 50), at(&f, 10, 120)), (BLUE, RED, WHITE));
}
