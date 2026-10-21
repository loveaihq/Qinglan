//! Paint tests with the test platform (every glyph a 1em square that is fully covered, ascent 0.8em):
//! known colours at known pixels for rectangles, borders, clipping, images, text, decorations, fixed
//! items after scrolling and paint order; lazy loading and eviction of images; first-screen-first
//! gives the same first screen.
#![allow(clippy::indexing_slicing, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashMap;
use std::sync::Arc;

use qinglan_engine::css::{NoResources, StyleOptions, style_document_with};
use qinglan_engine::encoding::decode_document;
use qinglan_engine::html::dom::NodeId;
use qinglan_engine::html::treebuilder::parse_document;
use qinglan_engine::layout::{LayoutOptions, LayoutResult, layout_document};
use qinglan_engine::net::Response;
use qinglan_engine::paint::decode::{DecodePool, Finished};
use qinglan_engine::paint::images::{Band, ImageSource, ImageStore, NoBitmaps};
use qinglan_engine::paint::raster::{Frame, View, render_band, render_frame};
use qinglan_engine::paint::{DisplayList, RasterStats, build_display_list, far_images, wanted_images};
use qinglan_engine::platform::image::{DecodedCache, DecodedImage, ImageDecoder, ImageError, ImageInfo};
use qinglan_engine::platform::{BasicPlatform, Platform, TestPlatform};
use qinglan_engine::url::Url;

const RED: [u8; 4] = [0, 0, 255, 255];
const GREEN: [u8; 4] = [0, 255, 0, 255];
const BLUE: [u8; 4] = [255, 0, 0, 255];
const YELLOW: [u8; 4] = [0, 255, 255, 255];
const WHITE: [u8; 4] = [255, 255, 255, 255];

struct Page {
    res: LayoutResult,
    list: DisplayList,
}

fn lay(html: &str, vw: u32, vh: u32, dpr: f32, stop_y: Option<f32>) -> Page {
    let (vw, vh) = (vw as f32, vh as f32);
    let platform = TestPlatform;
    let decoded = decode_document(html.as_bytes(), None, &BasicPlatform);
    let mut doc = parse_document(decoded.text);
    let sopts = StyleOptions { viewport_w: vw, viewport_h: vh, ..StyleOptions::default() };
    let styled = style_document_with(&mut doc, None, &mut NoResources, &sopts, Some(&platform as &dyn Platform));
    let res = layout_document(&doc, &styled.tree, &platform, &LayoutOptions { viewport_w: vw, viewport_h: vh, dpr, stop_y, ..LayoutOptions::default() });
    let list = build_display_list(&doc, &res, &platform);
    Page { res, list }
}

fn draw(page: &Page, vw: u32, vh: u32, dpr: f32, scroll: f32, images: &mut dyn ImageSource) -> (Frame, RasterStats) {
    let (vw, vh) = (vw as f32, vh as f32);
    let mut frame = Frame::new((vw * dpr).round() as u32, (vh * dpr).round() as u32);
    let stats = render_frame(&page.list, &TestPlatform, images, View { scroll_x: 0.0, scroll_y: scroll, dpr }, &mut frame);
    (frame, stats)
}

fn paint(html: &str, vw: u32, vh: u32) -> Frame {
    let page = lay(html, vw, vh, 1.0, None);
    draw(&page, vw, vh, 1.0, 0.0, &mut NoBitmaps).0
}

fn at(f: &Frame, x: u32, y: u32) -> [u8; 4] {
    f.pixel(x, y).unwrap_or([9, 9, 9, 9])
}

#[track_caller]
fn expect(f: &Frame, pts: &[(u32, u32, [u8; 4])]) {
    for &(x, y, c) in pts {
        assert_eq!(at(f, x, y), c, "pixel ({x},{y})");
    }
}

#[test]
fn rectangles_and_the_white_canvas() {
    let f = paint("<body style='margin:0'><div style='width:50px;height:30px;background:#f00;margin:10px 0 0 20px'></div></body>", 100, 60);
    expect(&f, &[(30, 20, RED), (20, 10, RED), (69, 39, RED), (19, 20, WHITE), (70, 20, WHITE), (30, 9, WHITE), (30, 40, WHITE), (0, 0, WHITE)]);
}

#[test]
fn borders_are_solid_rectangles_over_the_background() {
    let html = "<body style='margin:0'><div style='margin:10px;width:30px;height:20px;border:4px solid #00f;border-bottom-color:#0f0;background:#ff0'></div></body>";
    let f = paint(html, 80, 60);
    // Border box x 10..48, y 10..38; top/bottom borders run the full width, the sides sit between them.
    // Where two sides of different colors meet (bottom left) the border is cut along the diagonal from the outer to the inner
    // corner: (10,36) is above it, (11,37) below. Sides of one color (top, left, right) have no seam.
    expect(&f, &[(10, 10, BLUE), (47, 10, BLUE), (11, 20, BLUE), (44, 20, BLUE), (10, 36, BLUE), (11, 37, GREEN), (30, 35, GREEN), (20, 20, YELLOW), (20, 9, WHITE), (48, 20, WHITE)]);
}

#[test]
fn dotted_borders_are_round_dots_and_none_is_not_drawn() {
    // Outer 16 x 16; the middle line of the 3 px border is a 13 x 13 square of length 52, nine dots 52/9 apart from its
    // top left corner (1.5, 1.5): a dot of radius 1.5 covers pixel (1,1) and (7,1) completely; (4,1) lies in the gap.
    let html = "<body style='margin:0'><div style='width:10px;height:10px;border:3px dotted #00f'></div><div style='width:10px;height:10px;border:3px none #00f;margin-top:5px'></div></body>";
    let f = paint(html, 40, 60);
    expect(&f, &[(1, 1, BLUE), (7, 1, BLUE), (4, 1, WHITE), (7, 7, WHITE), (0, 16, WHITE), (0, 21, WHITE)]);
}

#[test]
fn overflow_hidden_clips_to_the_padding_box() {
    let html = "<body style='margin:0'><div style='width:20px;height:20px;overflow:hidden'><div style='width:100px;height:100px;background:#f00'></div></div></body>";
    let f = paint(html, 60, 60);
    expect(&f, &[(10, 10, RED), (19, 19, RED), (20, 10, WHITE), (10, 20, WHITE), (50, 50, WHITE)]);
    // With a border the content is clipped to the padding box: the border stays blue.
    let html = "<body style='margin:0'><div style='width:20px;height:20px;border:5px solid #00f;overflow:hidden'><div style='width:100px;height:100px;background:#f00'></div></div></body>";
    let f = paint(html, 60, 60);
    expect(&f, &[(2, 2, BLUE), (5, 5, RED), (24, 24, RED), (25, 10, BLUE), (27, 10, BLUE), (10, 27, BLUE), (30, 10, WHITE)]);
}

#[test]
fn nested_clips_intersect() {
    let html = "<body style='margin:0'><div style='width:30px;height:30px;overflow:hidden'><div style='margin-left:20px;width:30px;height:30px;overflow:hidden'><div style='width:100px;height:100px;background:#f00'></div></div></div></body>";
    let f = paint(html, 60, 60);
    // The inner box covers x 20..50 but the outer one cuts it at x = 30.
    expect(&f, &[(25, 10, RED), (29, 10, RED), (30, 10, WHITE), (10, 10, WHITE)]);
}

#[test]
fn text_is_blended_in_the_text_colour_at_the_baseline() {
    let html = "<body style='margin:0;font:10px/10px Ahem;color:#00f'><div style='width:100px'>ab</div></body>";
    let f = paint(html, 60, 30);
    // Two 10x10 squares: the line is 10 high, the baseline at 8 and the mask reaches 8 up and 2 down.
    expect(&f, &[(0, 0, BLUE), (9, 9, BLUE), (10, 0, BLUE), (19, 9, BLUE), (20, 5, WHITE), (5, 10, WHITE)]);
}

#[test]
fn translucent_text_mixes_with_the_background() {
    let html = "<body style='margin:0;font:10px/10px Ahem;background:#fff;color:rgba(0,0,255,0.5)'><div>a</div></body>";
    let f = paint(html, 30, 20);
    let p = at(&f, 5, 5);
    assert_eq!(p[3], 255);
    assert!((125..=130).contains(&p[2]) && (125..=130).contains(&p[1]) && p[0] == 255, "{p:?}");
}

#[test]
fn decoration_lines_are_drawn_over_the_text() {
    let html = "<body style='margin:0;font:10px/10px Ahem;color:#00f'><div><span style='text-decoration:underline;text-decoration-color:#f00'>ab</span></div></body>";
    let page = lay(html, 60, 30, 1.0, None);
    let (f, _) = draw(&page, 60, 30, 1.0, 0.0, &mut NoBitmaps);
    // Underline: a red bar 1px below the baseline (y 9), 20 wide.
    expect(&f, &[(5, 9, RED), (19, 9, RED), (20, 9, WHITE), (5, 8, BLUE)]);
}

#[test]
fn device_pixel_ratio_scales_everything() {
    let html = "<body style='margin:0;font:10px/10px Ahem;color:#00f'><div style='width:100px'>a</div><div style='margin-left:5px;width:10px;height:5px;background:#f00'></div></body>";
    let page = lay(html, 40, 30, 2.0, None);
    let (f, _) = draw(&page, 40, 30, 2.0, 0.0, &mut NoBitmaps);
    // The glyph is a 20x20 square at dpr 2; the red box starts at css (5,10) = device (10,20), 20x10 device px.
    expect(&f, &[(0, 0, BLUE), (19, 19, BLUE), (20, 5, WHITE), (10, 20, RED), (29, 29, RED), (30, 20, WHITE), (9, 20, WHITE), (10, 30, WHITE)]);
    // At 1.5 the 10px glyph is 15 device px, the box 15x7.5 device px starting at css y 10 = 15.
    let page = lay(html, 40, 30, 1.5, None);
    let (f, _) = draw(&page, 40, 30, 1.5, 0.0, &mut NoBitmaps);
    // Edges are rounded one by one: x 7.5..22.5 becomes 8..23.
    expect(&f, &[(14, 5, BLUE), (15, 5, WHITE), (7, 15, WHITE), (8, 15, RED), (22, 15, RED), (23, 15, WHITE)]);
}

/// An image source that returns one fixed bitmap and remembers the sizes it was asked for.
struct Fixed {
    img: Arc<DecodedImage>,
    asked: Vec<(u32, u32)>,
}

impl ImageSource for Fixed {
    fn bitmap(&mut self, _node: NodeId, w: u32, h: u32) -> Option<Arc<DecodedImage>> {
        self.asked.push((w, h));
        Some(self.img.clone())
    }
}

fn bitmap(w: u32, h: u32, f: impl Fn(u32, u32) -> [u8; 4]) -> Arc<DecodedImage> {
    let mut bgra = Vec::new();
    for y in 0..h {
        for x in 0..w {
            bgra.extend_from_slice(&f(x, y));
        }
    }
    Arc::new(DecodedImage { width: w, height: h, natural_width: w, natural_height: h, bgra })
}

#[test]
fn images_are_copied_one_to_one_and_scaled_bilinearly() {
    let html = "<body style='margin:0'><div style='height:5px'></div><img src='a.png' width='20' height='10' style='display:block;margin-left:10px'></body>";
    let page = lay(html, 50, 30, 1.0, None);
    // Exactly the size of the destination: a straight copy.
    let mut src = Fixed { img: bitmap(20, 10, |x, _| if x < 10 { RED } else { BLUE }), asked: Vec::new() };
    let (f, stats) = draw(&page, 50, 30, 1.0, 0.0, &mut src);
    assert_eq!(src.asked, vec![(20, 10)]);
    assert_eq!(stats.images, 1);
    expect(&f, &[(10, 5, RED), (19, 14, RED), (20, 5, BLUE), (29, 14, BLUE), (9, 5, WHITE), (30, 5, WHITE), (15, 4, WHITE), (15, 15, WHITE)]);
    // A 2x1 bitmap stretched over 20x10: pure colours at the ends, a mix in the middle.
    let mut src = Fixed { img: bitmap(2, 1, |x, _| if x == 0 { RED } else { BLUE }), asked: Vec::new() };
    let (f, _) = draw(&page, 50, 30, 1.0, 0.0, &mut src);
    expect(&f, &[(10, 5, RED), (11, 9, RED), (29, 5, BLUE), (28, 14, BLUE)]);
    let mid = at(&f, 20, 8);
    assert!(mid[2] > 60 && mid[0] > 60 && mid[1] == 0, "{mid:?}");
    // At dpr 2 the image is asked for at device size and fills 40x20 device pixels.
    let page = lay(html, 50, 30, 2.0, None);
    let mut src = Fixed { img: bitmap(40, 20, |_, _| GREEN), asked: Vec::new() };
    let (f, _) = draw(&page, 50, 30, 2.0, 0.0, &mut src);
    assert_eq!(src.asked, vec![(40, 20)]);
    expect(&f, &[(20, 10, GREEN), (59, 29, GREEN), (60, 10, WHITE), (20, 9, WHITE)]);
}

#[test]
fn transparent_image_pixels_show_what_is_behind() {
    let html = "<body style='margin:0;background:#fff'><img src='a.png' width='4' height='4' style='display:block'></body>";
    let page = lay(html, 10, 10, 1.0, None);
    // Premultiplied half-transparent red over white.
    let mut src = Fixed { img: bitmap(4, 4, |_, _| [0, 0, 128, 128]), asked: Vec::new() };
    let (f, _) = draw(&page, 10, 10, 1.0, 0.0, &mut src);
    let p = at(&f, 1, 1);
    assert_eq!(p[3], 255);
    assert!(p[2] == 255 && (126..=128).contains(&p[1]), "{p:?}");
}

#[test]
fn fixed_items_stay_while_the_page_scrolls() {
    let html = "<body style='margin:0'><div style='height:600px'></div><div style='height:20px;background:#f00'></div><div style='height:1400px'></div>\
                <div style='position:fixed;top:5px;left:5px;width:10px;height:10px;background:#0f0'></div></body>";
    let page = lay(html, 100, 100, 1.0, None);
    let (f0, _) = draw(&page, 100, 100, 1.0, 0.0, &mut NoBitmaps);
    expect(&f0, &[(8, 8, GREEN), (50, 50, WHITE)]);
    let (f1, _) = draw(&page, 100, 100, 1.0, 550.0, &mut NoBitmaps);
    // The red bar is at page y 600..620: frame y 50..70. The green box has not moved.
    expect(&f1, &[(8, 8, GREEN), (50, 50, RED), (50, 69, RED), (50, 70, WHITE), (50, 49, WHITE)]);
    let (f2, _) = draw(&page, 100, 100, 1.0, 1800.0, &mut NoBitmaps);
    expect(&f2, &[(8, 8, GREEN), (50, 50, WHITE)]);
    // The same at dpr 2.
    let page = lay(html, 100, 100, 2.0, None);
    let (f3, _) = draw(&page, 100, 100, 2.0, 550.0, &mut NoBitmaps);
    expect(&f3, &[(16, 16, GREEN), (100, 100, RED), (100, 139, RED), (100, 140, WHITE), (100, 99, WHITE)]);
}

#[test]
fn positioned_boxes_are_painted_in_z_order() {
    let html = "<body style='margin:0'>\
        <div style='position:absolute;left:0;top:0;width:30px;height:30px;background:#00f;z-index:-1'></div>\
        <div style='width:20px;height:20px;background:#f00'></div>\
        <div style='position:absolute;left:10px;top:10px;width:30px;height:30px;background:#0f0'></div></body>";
    let f = paint(html, 60, 60);
    // Flow (red) over the negative z-index box (blue); the positioned green box over both.
    expect(&f, &[(25, 5, BLUE), (5, 5, RED), (15, 15, GREEN), (35, 35, GREEN), (45, 45, WHITE), (5, 25, BLUE)]);
}

#[test]
fn fixed_boxes_with_negative_z_index_go_below_the_flow() {
    let html = "<body style='margin:0'><div style='position:fixed;left:0;top:0;width:30px;height:30px;background:#00f;z-index:-1'></div>\
        <div style='width:20px;height:20px;background:#f00'></div><div style='height:2000px'></div></body>";
    let page = lay(html, 60, 60, 1.0, None);
    for scroll in [0.0, 10.0] {
        let (f, _) = draw(&page, 60, 60, 1.0, scroll, &mut NoBitmaps);
        expect(&f, &[(25, 25, BLUE)]);
    }
    let (f, _) = draw(&page, 60, 60, 1.0, 0.0, &mut NoBitmaps);
    expect(&f, &[(5, 5, RED)]);
}

#[test]
fn an_absolute_box_is_not_clipped_by_a_static_overflow_ancestor_but_by_its_containing_block() {
    let html = "<body style='margin:0'>\
        <div style='position:relative;width:60px;height:60px'><div style='width:10px;height:10px;overflow:hidden'>\
        <div style='position:absolute;left:0;top:0;width:40px;height:40px;background:#f00'></div></div></div>\
        <div style='position:relative;margin-top:10px;width:20px;height:20px;overflow:hidden'>\
        <div style='position:absolute;left:0;top:0;width:40px;height:40px;background:#0f0'></div></div></body>";
    let f = paint(html, 80, 120);
    // First: the clipping parent is static, the containing block is the 60x60 box: red fills 40x40.
    // Second: the containing block clips (20x20).
    expect(&f, &[(30, 30, RED), (39, 39, RED), (45, 45, WHITE), (5, 75, GREEN), (25, 75, WHITE)]);
}

#[test]
fn the_canvas_takes_the_body_background_everywhere() {
    let f = paint("<body style='margin:20px;background:#0f0'><div style='height:5px'></div></body>", 50, 50);
    expect(&f, &[(0, 0, GREEN), (49, 49, GREEN), (25, 25, GREEN)]);
    let f = paint("<html style='background:#00f'><body style='margin:20px;background:#0f0'><div style='height:5px;width:4px;background:#f00'></div></body></html>", 50, 50);
    // The root's background is the canvas; the body paints its own (20,20 to 30,25).
    expect(&f, &[(0, 0, BLUE), (21, 22, RED), (28, 22, GREEN), (40, 40, BLUE), (28, 26, BLUE)]);
}

#[test]
fn hidden_boxes_paint_nothing_but_visible_children_do() {
    let html = "<body style='margin:0'><div style='width:30px;height:30px;background:#f00;visibility:hidden'><div style='visibility:visible;width:10px;height:10px;background:#0f0'></div></div></body>";
    let f = paint(html, 40, 40);
    expect(&f, &[(5, 5, GREEN), (20, 20, WHITE), (20, 5, WHITE)]);
}

#[test]
fn list_markers_and_form_controls_are_painted() {
    let html = "<body style='margin:0;font:10px/10px Ahem;color:#00f'><ul style='margin:0;padding-left:30px'><li>x</li></ul>\
        <input type='checkbox' checked style='display:block;margin:5px 0 0 0'></body>";
    let f = paint(html, 80, 60);
    // The marker "• " is drawn left of the item (a square in the test font, then a space): the bullet is x 10..20.
    expect(&f, &[(15, 5, BLUE), (25, 5, WHITE), (35, 5, BLUE)]);
    // The check box: 13x13, dark inside a white square with a grey frame.
    let c = at(&f, 6, 18);
    assert_eq!(c, [0x33, 0x33, 0x33, 255], "{c:?}");
    expect(&f, &[(0, 15, [0x76, 0x76, 0x76, 255])]);
}

#[test]
fn only_items_in_the_viewport_are_touched() {
    let mut html = String::from("<body style='margin:0;font:10px/10px Ahem;color:#00f'>");
    for i in 0..400 {
        html.push_str(&format!("<div style='height:50px;background:{}'>row {i}</div>", if i % 2 == 0 { "#ffe" } else { "#eef" }));
    }
    html.push_str("</body>");
    let page = lay(&html, 200, 100, 1.0, None);
    let total = page.list.items.len();
    assert!(total >= 800, "{total} items");
    let (f, stats) = draw(&page, 200, 100, 1.0, 10_000.0, &mut NoBitmaps);
    // Rows 200 and 201 (y 10000 and 10050) fill the frame; the index hands the rasterizer a handful of items.
    assert!(stats.items <= 12, "{} of {total} items drawn", stats.items);
    expect(&f, &[(150, 5, [0xee, 0xff, 0xff, 255]), (150, 55, [0xff, 0xee, 0xee, 255])]);
}

#[test]
fn hostile_pages_do_not_panic_the_painter() {
    let html = "<body style='margin:1e9px'><div style='position:absolute;left:-1e9px;top:1e9px;width:1e9px;height:1e9px;background:#f00;border:1e9px solid #00f'>x</div>\
        <div style='position:fixed;top:-1e9px;width:1e9px;height:1e9px;overflow:hidden'><div style='margin:1e9px;height:1e9px;background:#0f0'>y</div></div></body>";
    let page = lay(html, 80, 80, 1.0, None);
    for scroll in [0.0, 1.0e9, -5.0] {
        let _ = draw(&page, 80, 80, 1.0, scroll, &mut NoBitmaps);
    }
    let _ = draw(&page, 80, 80, 8.0, 12345.0, &mut NoBitmaps);
}

// ---- first-screen-first ------------------------------------------------------------------------------------

#[test]
fn a_partial_layout_paints_the_same_first_screen() {
    let mut html = String::from("<body style='margin:8px;font:10px/12px Ahem;color:#123'><div style='position:fixed;top:2px;right:2px;width:9px;height:9px;background:#0a0'></div>        <div style='position:absolute;left:3px;top:20px;width:30px;height:20px;background:#c00'></div>");
    for i in 0..300 {
        html.push_str(&format!("<h2 style='margin:7px 0;border-bottom:1px solid #999'>Title {i}</h2><p style='margin:0 0 9px;background:#eef'>text of paragraph {i} that wraps over several lines of the test font so that it is long enough</p>"));
    }
    // A positioned box whose place in the flow is below the stop line is only there in the full layout.
    html.push_str("<div style='position:absolute;left:100px;top:500px;width:30px;height:20px;background:#00c'></div></body>");
    let full = lay(&html, 300, 200, 1.0, None);
    let part = lay(&html, 300, 200, 1.0, Some(400.0));
    assert!(!full.res.partial && part.res.partial);
    assert!(part.res.lines.len() < full.res.lines.len() / 3, "{} of {} lines", part.res.lines.len(), full.res.lines.len());
    assert!(part.list.height < full.list.height);
    let (late_full, _) = draw(&full, 300, 200, 1.0, 450.0, &mut NoBitmaps);
    let (late_part, _) = draw(&part, 300, 200, 1.0, 450.0, &mut NoBitmaps);
    expect(&late_full, &[(110, 60, [0xcc, 0, 0, 255])]);
    expect(&late_part, &[(110, 60, WHITE)]);
    for dpr in [1.0, 1.5] {
        let (a, _) = draw(&full, 300, 200, dpr, 0.0, &mut NoBitmaps);
        let (b, _) = draw(&part, 300, 200, dpr, 0.0, &mut NoBitmaps);
        let first = a.data.as_chunks::<4>().0.iter().zip(b.data.as_chunks::<4>().0).position(|(p, q)| p != q);
        assert!(first.is_none(), "the first screen differs at dpr {dpr}, first at pixel {:?}", first.map(|i| (i % a.width as usize, i / a.width as usize)));
    }
    // Scrolled to where the partial layout still has content (stop line 400, viewport 200).
    let (a, _) = draw(&full, 300, 200, 1.0, 150.0, &mut NoBitmaps);
    let (b, _) = draw(&part, 300, 200, 1.0, 150.0, &mut NoBitmaps);
    assert!(a.data == b.data, "scroll 150 differs");
}

// ---- lazy loading and eviction -------------------------------------------------------------------------------

fn image_page(n: usize, lazy: &[usize]) -> String {
    let mut s = String::from("<body style='margin:0'>");
    for i in 0..n {
        let l = if lazy.contains(&i) { " loading='lazy'" } else { "" };
        s.push_str(&format!("<img src='i{i}.png' width='100' height='100' style='display:block'{l}>"));
    }
    s.push_str("</body>");
    s
}

fn wanted_indices(page: &Page, band: Band, all: bool) -> Vec<usize> {
    wanted_images(&page.res, &page.list, band, all).iter().map(|r| r.src.trim_start_matches('i').trim_end_matches(".png").parse().unwrap()).collect()
}

#[test]
fn lazy_images_wait_and_far_images_wait_two_screens() {
    let page = lay(&image_page(20, &[3, 5, 8]), 100, 200, 1.0, None);
    assert_eq!(page.res.images.len(), 20);
    // Images are 100 px high and stacked from y = 0; the viewport is 200 high, so a screen is 200 px.
    // Eager: within two screens (top <= 600). Lazy: within one (top <= 400): 3 yes, 5 and 8 not yet.
    assert_eq!(wanted_indices(&page, Band::new(0.0, 200.0), false), vec![0, 1, 2, 3, 4, 6]);
    // At scroll 1000: eager images with bottom >= 600 and top <= 1600; lazy 3 and 5 are far, 8 (y 800..900) is near.
    assert_eq!(wanted_indices(&page, Band::new(1000.0, 200.0), false), vec![6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
    // render --full loads everything.
    assert_eq!(wanted_indices(&page, Band::new(0.0, 200.0), true).len(), 20);
}

#[test]
fn nothing_below_a_first_screen_stop_is_wanted_until_the_full_layout() {
    let page = lay(&image_page(40, &[]), 100, 200, 1.0, Some(400.0));
    assert!(page.res.partial);
    // Only the images that were laid out count (the one at y = 400 is where layout stopped): the ones past the stop line have no place yet.
    let got = wanted_indices(&page, Band::new(0.0, 200.0), false);
    assert_eq!(got, vec![0, 1, 2, 3]);
    let full = lay(&image_page(40, &[]), 100, 200, 1.0, None);
    assert_eq!(wanted_indices(&full, Band::new(0.0, 200.0), false), vec![0, 1, 2, 3, 4, 5, 6]);
}

struct FakeDecoder {
    decodes: std::sync::atomic::AtomicUsize,
}

impl ImageDecoder for FakeDecoder {
    fn probe(&self, _bytes: &[u8]) -> Result<ImageInfo, ImageError> {
        Ok(ImageInfo { width: 10, height: 10 })
    }
    fn decode(&self, _bytes: &[u8], width: u32, height: u32) -> Result<DecodedImage, ImageError> {
        self.decodes.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(DecodedImage { width, height, natural_width: 10, natural_height: 10, bgra: RED.repeat((width * height) as usize) })
    }
}

#[test]
fn decoded_bitmaps_far_away_are_dropped_and_come_back_from_the_cached_bytes() {
    let page = lay(&image_page(40, &[]), 100, 200, 1.0, None);
    let mut store = ImageStore::new();
    let mut bytes: HashMap<String, Arc<Response>> = HashMap::new();
    for (i, r) in page.res.images.iter().enumerate() {
        let url = format!("http://x.test/i{i}.png");
        store.set_url(r.node, &url);
        bytes.insert(url.clone(), Arc::new(Response { url: Url::parse(&url, None).unwrap(), status: 200, headers: vec![], body: b"png".to_vec() }));
    }
    let dec = FakeDecoder { decodes: Default::default() };
    let mut lookup = |u: &str| bytes.get(u).cloned();
    // Scroll 0, 200 high: decoded up to one screen below (images 0..=4 touch y < 400).
    let band0 = Band::new(0.0, 200.0);
    let rep = store.prepare(&page.list, band0, 1.0, &dec, &mut lookup);
    assert_eq!(rep.decoded, 5, "{rep:?}");
    assert!(rep.missing.is_empty());
    assert_eq!(store.len(), 5);
    assert_eq!(store.bytes(), 5 * (100 * 100 * 4 + 64));
    // Preparing again decodes nothing.
    assert_eq!(store.prepare(&page.list, band0, 1.0, &dec, &mut lookup).decoded, 0);
    // Nothing is more than four screens (800 px) away yet; at scroll 1500 the images above y 700 are.
    let band1 = Band::new(1500.0, 200.0);
    let far = far_images(&page.list, band1);
    assert_eq!(far.len(), 20, "images 0..=5 (ending above y 700) and 26..=39 (starting below y 2500)");
    assert_eq!(store.drop_far(&page.list, band1), 5);
    assert!(store.is_empty());
    // Scroll back: the bitmaps are decoded again from the cached bytes (no new download).
    let rep = store.prepare(&page.list, band0, 1.0, &dec, &mut lookup);
    assert_eq!(rep.decoded, 5);
    assert_eq!(dec.decodes.load(std::sync::atomic::Ordering::SeqCst), 10);
    // Bytes that fell out of the response cache are reported, not guessed.
    store.drop_far(&page.list, band1);
    bytes.clear();
    let mut gone = |u: &str| bytes.get(u).cloned();
    let rep = store.prepare(&page.list, band0, 1.0, &dec, &mut gone);
    assert_eq!((rep.decoded, rep.missing.len()), (0, 5));
    // The painter draws the bitmaps the store holds, for the size they were decoded at.
    let mut store = ImageStore::new();
    let node = page.res.images[0].node;
    store.set_url(node, "http://x.test/i0.png");
    let mut ok = |u: &str| (u == "http://x.test/i0.png").then(|| Arc::new(Response { url: Url::parse(u, None).unwrap(), status: 200, headers: vec![], body: b"png".to_vec() }));
    store.prepare(&page.list, band0, 1.0, &dec, &mut ok);
    let (f, stats) = draw(&page, 100, 200, 1.0, 0.0, &mut store);
    assert_eq!(stats.images, 1);
    expect(&f, &[(50, 50, RED), (50, 150, WHITE)]);
}

#[test]
fn a_viewport_drawn_in_bands_is_the_same_picture() {
    let html = "<body style=\"margin:0\"><div style=\"position:fixed;top:30px;left:10px;width:50px;height:20px;background:#00f\"></div>                <div style=\"height:40px;background:#f00\"></div><p style=\"margin:0;font-size:16px\">text text text text text</p>                <div style=\"height:900px;background:#0f0\"></div>";
    for dpr in [1.0f32, 1.5] {
        let page = lay(html, 200, 120, dpr, None);
        for scroll in [0.0f32, 37.0, 64.0] {
            let (whole, _) = draw(&page, 200, 120, dpr, scroll, &mut NoBitmaps);
            let (w, h) = (whole.width, whole.height);
            let mut data = Vec::new();
            let mut y0 = 0;
            while y0 < h {
                let bh = 37.min(h - y0);
                let mut band = Frame::new(w, bh);
                render_band(&page.list, &TestPlatform, &mut NoBitmaps, View { scroll_x: 0.0, scroll_y: scroll, dpr }, y0, &mut band);
                data.extend_from_slice(&band.data);
                y0 += bh;
            }
            assert!(data == whole.data, "bands differ from the whole frame at dpr {dpr}, scroll {scroll}");
        }
    }
}

#[test]
fn bytes_that_never_stay_in_the_cache_are_asked_for_only_twice() {
    // An image whose bytes are gone each time they are asked for (too big for the response cache) is given up on, not
    // downloaded over and over.
    let page = lay(&image_page(1, &[]), 100, 200, 1.0, None);
    let mut store = ImageStore::new();
    let node = page.res.images[0].node;
    store.set_url(node, "http://x.test/i0.png");
    let dec = FakeDecoder { decodes: Default::default() };
    let mut gone = |_: &str| None;
    let band = Band::new(0.0, 200.0);
    let counts: Vec<usize> = (0..5).map(|_| store.prepare(&page.list, band, 1.0, &dec, &mut gone).missing.len()).collect();
    assert_eq!(counts, [1, 1, 0, 0, 0]);
    // A new address starts afresh, and bytes that do come back reset the count.
    store.set_url(node, "http://x.test/i1.png");
    assert_eq!(store.prepare(&page.list, band, 1.0, &dec, &mut gone).missing.len(), 1);
    let mut ok = |u: &str| Some(Arc::new(Response { url: Url::parse(u, None).unwrap(), status: 200, headers: vec![], body: b"png".to_vec() }));
    assert_eq!(store.prepare(&page.list, band, 1.0, &dec, &mut ok).decoded, 1);
}

#[test]
fn a_frame_placeholder_is_a_border_and_a_label_not_a_cover() {
    // A fixed frame over green: the inside stays green (an opaque grey box hid the text of mdBook pages).
    let html = "<body style='margin:0;font:10px/10px Ahem;color:#00f'><div style='height:120px;background:#0f0'></div>\
        <iframe style='position:fixed;left:10px;top:10px;width:100px;height:50px;border:0'></iframe></body>";
    let f = paint(html, 140, 100);
    expect(&f, &[(60, 45, GREEN), (105, 30, GREEN), (30, 56, GREEN), (10, 30, [0x76, 0x76, 0x76, 255]), (109, 30, [0x76, 0x76, 0x76, 255]), (60, 10, [0x76, 0x76, 0x76, 255]), (60, 59, [0x76, 0x76, 0x76, 255])]);
}

// ---- decoding on worker threads (step 2-0) ----------------------------------------------------------------------

/// A decoder that waits at a gate, so that a test decides when a decode finishes.
struct GateDecoder {
    open: std::sync::Mutex<bool>,
    cv: std::sync::Condvar,
    started: std::sync::atomic::AtomicUsize,
}

impl GateDecoder {
    fn new() -> Arc<GateDecoder> {
        Arc::new(GateDecoder { open: std::sync::Mutex::new(false), cv: std::sync::Condvar::new(), started: Default::default() })
    }
    fn open(&self) {
        *self.open.lock().unwrap() = true;
        self.cv.notify_all();
    }
    fn wait_started(&self, n: usize) {
        let t = std::time::Instant::now();
        while self.started.load(std::sync::atomic::Ordering::SeqCst) < n {
            assert!(t.elapsed() < std::time::Duration::from_secs(5), "the workers did not start");
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
    }
}

impl ImageDecoder for GateDecoder {
    fn probe(&self, _bytes: &[u8]) -> Result<ImageInfo, ImageError> {
        Ok(ImageInfo { width: 10, height: 10 })
    }
    fn decode(&self, _bytes: &[u8], width: u32, height: u32) -> Result<DecodedImage, ImageError> {
        self.started.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let mut open = self.open.lock().unwrap();
        while !*open {
            open = self.cv.wait(open).unwrap();
        }
        Ok(DecodedImage { width, height, natural_width: 10, natural_height: 10, bgra: RED.repeat((width * height) as usize) })
    }
}

/// `n` stacked images of 100 x 100, their addresses set in `store`, and a response cache that has all of them.
fn async_fixture(n: usize, store: &mut ImageStore) -> (Page, HashMap<String, Arc<Response>>) {
    let page = lay(&image_page(n, &[]), 100, 200, 1.0, None);
    let mut bytes = HashMap::new();
    for (i, r) in page.res.images.iter().enumerate() {
        let url = format!("http://x.test/i{i}.png");
        store.set_url(r.node, &url);
        bytes.insert(url.clone(), Arc::new(Response { url: Url::parse(&url, None).unwrap(), status: 200, headers: vec![], body: b"png".to_vec() }));
    }
    (page, bytes)
}

/// What one decode of 100 x 100 takes of the budget.
const SIZE_100: usize = 100 * 100 * 4 + 64;

fn drain(pool: &mut DecodePool, n: usize) -> Vec<Finished> {
    (0..n).map(|_| pool.wait(std::time::Duration::from_secs(5)).expect("a decode result")).collect()
}

#[test]
fn a_decode_that_finishes_after_navigation_is_dropped() {
    let gate = GateDecoder::new();
    let mut pool = DecodePool::new(gate.clone(), 2, Arc::new(|| {}));
    let mut old = ImageStore::new();
    let (page, bytes) = async_fixture(40, &mut old);
    let mut lookup = |u: &str| bytes.get(u).cloned();
    let band = Band::new(0.0, 200.0);
    let rep = old.prepare_async(&page.list, band, 1.0, &mut pool, &mut lookup);
    assert!(rep.missing.is_empty());
    assert_eq!(old.in_flight(), 5, "images 0..=4 are within a screen");
    assert_eq!(old.reserved_bytes(), 5 * SIZE_100);
    assert!(old.is_empty(), "nothing is drawn before the bitmap is there");
    // The page is left while two decodes run and three wait: the waiting ones are called off, and the new page's store
    // (it asks for the same addresses) does not know the running ones.
    gate.wait_started(2);
    pool.cancel_all();
    assert_eq!(pool.queued(), 0);
    let mut new = ImageStore::new();
    let (_, _) = async_fixture(40, &mut new);
    let rep = new.prepare_async(&page.list, band, 1.0, &mut pool, &mut lookup);
    assert!(rep.missing.is_empty());
    assert_eq!(new.in_flight(), 5);
    gate.open();
    let results = drain(&mut pool, 7);
    let (stale, fresh): (Vec<_>, Vec<_>) = results.into_iter().partition(|f| f.id <= 5);
    assert_eq!((stale.len(), fresh.len()), (2, 5), "the old page's two running decodes, the new page's five");
    for f in stale {
        assert!(new.accept(f).nodes.is_empty());
    }
    assert!(new.is_empty() && new.bytes() == 0, "nothing of the old page was stored");
    assert_eq!(new.in_flight(), 5);
    for f in fresh {
        assert_eq!(new.accept(f).nodes.len(), 1);
    }
    assert_eq!((new.len(), new.in_flight(), new.reserved_bytes()), (5, 0, 0));
    // A store that was cleared (the page was left) takes nothing in either.
    old.clear();
    assert_eq!((old.in_flight(), old.reserved_bytes(), old.len()), (0, 0, 0));
}

#[test]
fn the_cache_budget_counts_decodes_in_flight() {
    let gate = GateDecoder::new();
    let mut pool = DecodePool::new(gate.clone(), 2, Arc::new(|| {}));
    // Room for three bitmaps of 100 x 100.
    let budget = 3 * SIZE_100 + 10;
    let mut store = ImageStore::with_cache(DecodedCache::with_budget(budget));
    let (page, bytes) = async_fixture(40, &mut store);
    let mut lookup = |u: &str| bytes.get(u).cloned();
    // Images 0..=2 are on screen, 3 and 4 a little below: only three decodes may be started.
    store.prepare_async(&page.list, Band::new(0.0, 200.0), 1.0, &mut pool, &mut lookup);
    assert_eq!(store.in_flight(), 3);
    assert_eq!(store.reserved_bytes(), 3 * SIZE_100);
    assert!(store.deferred());
    assert!(store.bytes() + store.reserved_bytes() <= budget);
    gate.open();
    for f in drain(&mut pool, 3) {
        store.accept(f);
    }
    assert_eq!((store.len(), store.in_flight(), store.reserved_bytes()), (3, 0, 0));
    assert!(store.bytes() <= budget);
    // The two that did not fit do not push out the nearer ones that are shown (no thrashing).
    store.prepare_async(&page.list, Band::new(0.0, 200.0), 1.0, &mut pool, &mut lookup);
    assert_eq!((store.in_flight(), store.len()), (0, 3));
    assert!(store.deferred());
    // Scrolled to 300: images 3 and 4 are on screen now, 0 and 1 are farther away and make room for them; image 5,
    // just as near, has to wait because image 2 (still on screen) is not given up for it.
    let gate2 = GateDecoder::new();
    let mut pool2 = DecodePool::new(gate2.clone(), 2, Arc::new(|| {}));
    store.prepare_async(&page.list, Band::new(300.0, 200.0), 1.0, &mut pool2, &mut lookup);
    assert_eq!(store.in_flight(), 2);
    assert!(store.deferred());
    assert!(store.bytes() + store.reserved_bytes() <= budget, "{} + {} > {budget}", store.bytes(), store.reserved_bytes());
    gate2.open();
    for f in drain(&mut pool2, 2) {
        store.accept(f);
    }
    assert!(store.bytes() <= budget);
    assert_eq!((store.len(), store.in_flight(), store.reserved_bytes()), (3, 0, 0));
}

#[test]
fn queued_decodes_of_images_that_went_far_are_called_off() {
    let gate = GateDecoder::new();
    let mut pool = DecodePool::new(gate.clone(), 1, Arc::new(|| {}));
    let mut store = ImageStore::new();
    let (page, bytes) = async_fixture(40, &mut store);
    let mut lookup = |u: &str| bytes.get(u).cloned();
    store.prepare_async(&page.list, Band::new(0.0, 200.0), 1.0, &mut pool, &mut lookup);
    gate.wait_started(1);
    assert_eq!((store.in_flight(), pool.queued()), (5, 4));
    // Scrolled far away: the four that have not started are taken back; the running one finishes and is dropped.
    store.prepare_async(&page.list, Band::new(2000.0, 200.0), 1.0, &mut pool, &mut lookup);
    let new_jobs = store.in_flight() - 1;
    assert_eq!(pool.queued(), new_jobs, "only the jobs for the new place are queued");
    assert!(new_jobs > 0);
    gate.open();
    for f in drain(&mut pool, 1 + new_jobs) {
        store.accept(f);
    }
    assert_eq!((store.in_flight(), store.reserved_bytes()), (0, 0));
    assert!(!store.has(page.res.images[0].node, &page.list.images[0].rect, 1.0), "the bitmap of the image that was left behind is not kept");
    assert_eq!(store.len(), new_jobs);
}

/// Probes fine (the header was readable), never decodes.
struct BrokenBody;

impl ImageDecoder for BrokenBody {
    fn probe(&self, _bytes: &[u8]) -> Result<ImageInfo, ImageError> {
        Ok(ImageInfo { width: 10, height: 10 })
    }
    fn decode(&self, _bytes: &[u8], _width: u32, _height: u32) -> Result<DecodedImage, ImageError> {
        Err(ImageError::Unsupported)
    }
}

#[test]
fn a_decode_that_fails_is_reported_once_so_the_host_can_show_the_alt_text() {
    // Step 2a-1: the header of a WebP says how big it is, the bytes then do not decode (no WebP codec on the machine).
    let mut store = ImageStore::new();
    let (page, bytes) = async_fixture(2, &mut store);
    let mut lookup = |u: &str| bytes.get(u).cloned();
    let band = Band::new(0.0, 200.0);
    // The pool's way (the window).
    let mut pool = DecodePool::new(Arc::new(BrokenBody), 2, Arc::new(|| {}));
    store.prepare_async(&page.list, band, 1.0, &mut pool, &mut lookup);
    for f in drain(&mut pool, 2) {
        assert!(store.accept(f).nodes.is_empty());
    }
    let mut failed = store.take_decode_failed();
    failed.sort_unstable();
    assert_eq!(failed.len(), 2);
    assert!(store.take_decode_failed().is_empty(), "reported once");
    // The images are not tried again.
    store.prepare_async(&page.list, band, 1.0, &mut pool, &mut lookup);
    assert_eq!(store.in_flight(), 0);
    // The synchronous way (`qinglan-cli render`).
    let mut sync = ImageStore::new();
    let (page, bytes) = async_fixture(2, &mut sync);
    sync.prepare(&page.list, band, 1.0, &BrokenBody, &mut |u| bytes.get(u).cloned());
    assert_eq!(sync.take_decode_failed().len(), 2);
}

/// CSS2 appendix E.2: the backgrounds of the in-flow blocks, then the floats, then the inline content on top.
#[test]
fn floats_paint_over_block_backgrounds_and_under_inline_text() {
    let html = "<body style='margin:0;font:10px/10px Ahem;color:#00f'>\
        <div style='width:100px;background:#ff0'>aaaaaaaaaa<br>bbbbbbbbbb</div>\
        <div style='float:left;width:30px;height:20px;margin-top:-10px;background:#f00'></div>\
        <div style='height:40px;background:#0f0'></div></body>";
    let f = paint(html, 100, 70);
    // The float is y 10..30, x 0..30. Line 2 of the text (y 10..20) is on top of it, the green block (y 20..60)
    // is below it.
    expect(&f, &[(5, 15, BLUE), (35, 15, BLUE), (5, 25, RED), (29, 29, RED), (30, 25, GREEN), (5, 30, GREEN), (5, 5, BLUE)]);
}

#[test]
fn a_nested_stacking_context_keeps_its_positioned_children_inside_it() {
    // A (z-index 1) holds B (z-index 100); C (z-index 2) is outside A. A's whole context is below C, so C covers B even though 100 > 2.
    // D (z-index -1) is inside A: it is above A's own background, not below the flow of the page.
    let f = paint(
        "<body style='margin:0'>\
         <div style='position:relative;z-index:1;width:60px;height:60px;background:#ff0'>\
         <div style='position:absolute;z-index:100;left:0;top:0;width:20px;height:20px;background:#0f0'></div>\
         <div style='position:absolute;z-index:-1;left:40px;top:40px;width:10px;height:10px;background:#f00'></div></div>\
         <div style='position:absolute;z-index:2;left:0;top:0;width:20px;height:20px;background:#00f'></div>",
        100,
        100,
    );
    expect(&f, &[(5, 5, BLUE), (45, 45, RED), (30, 30, YELLOW)]);
}
