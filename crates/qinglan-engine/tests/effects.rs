//! Shadows, opacity and 2D transforms (step 2c-2) with the test platform (every glyph a solid 1em square). The numbers are
//! worked out by hand in the comments. Colors are BGRA.
#![allow(clippy::indexing_slicing, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use qinglan_engine::css::{NoResources, StyleOptions, style_document_with};
use qinglan_engine::html::dom::Document;
use qinglan_engine::html::treebuilder::parse_document;
use qinglan_engine::interact::{PageIndex, link_target};
use qinglan_engine::layout::{ImageSizes, LayoutOptions, LayoutResult, layout_document_with};
use qinglan_engine::paint::images::NoBitmaps;
use qinglan_engine::paint::raster::{Frame, View, render_frame};
use qinglan_engine::paint::{DisplayList, build_display_list};
use qinglan_engine::platform::{Platform, TestPlatform};

const RED: [u8; 4] = [0, 0, 255, 255];
const BLUE: [u8; 4] = [255, 0, 0, 255];
const LIME: [u8; 4] = [0, 255, 0, 255];
const WHITE: [u8; 4] = [255, 255, 255, 255];

struct Page {
    doc: Document,
    res: LayoutResult,
    list: DisplayList,
}

fn lay(body: &str, vw: u32, vh: u32) -> Page {
    let html = format!("<body style='margin:0'>{body}");
    let platform = TestPlatform;
    let mut doc = parse_document(html);
    let sopts = StyleOptions { viewport_w: vw as f32, viewport_h: vh as f32, ..StyleOptions::default() };
    let styled = style_document_with(&mut doc, None, &mut NoResources, &sopts, Some(&platform as &dyn Platform));
    let res = layout_document_with(&doc, &styled.tree, &platform, &LayoutOptions { viewport_w: vw as f32, viewport_h: vh as f32, ..LayoutOptions::default() }, &ImageSizes::new());
    let list = build_display_list(&doc, &res, &platform);
    Page { doc, res, list }
}

fn draw(page: &Page, vw: u32, vh: u32, scroll: f32) -> Frame {
    let mut frame = Frame::new(vw, vh);
    render_frame(&page.list, &TestPlatform, &mut NoBitmaps, View { scroll_y: scroll, dpr: 1.0 }, &mut frame);
    frame
}

fn paint(body: &str, vw: u32, vh: u32) -> Frame {
    draw(&lay(body, vw, vh), vw, vh, 0.0)
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

#[track_caller]
fn near(f: &Frame, x: u32, y: u32, want: [u8; 4], tol: i32) {
    let p = at(f, x, y);
    assert!(p.iter().zip(want).all(|(a, b)| (i32::from(*a) - i32::from(b)).abs() <= tol), "pixel ({x},{y}) is {p:?}, wanted {want:?} +-{tol}");
}

/// The summed darkness of the frame, in pixels (black on white: 1 per black pixel), by the green channel.
fn mass(f: &Frame) -> f32 {
    f.data.as_chunks::<4>().0.iter().map(|p| (255.0 - f32::from(p[1])) / 255.0).sum()
}

/// The summed red-ness, for red on white (green channel goes 255 to 0).
const BOX: &str = "width:20px;height:20px;";

#[test]
fn an_outer_shadow_is_offset_and_never_inside_the_box() {
    // Box (10,10)-(30,30), shadow (20,20)-(40,40). No background: the box stays empty, the shadow is clipped out of it.
    let f = paint(&format!("<div style='position:absolute;left:10px;top:10px;{BOX}box-shadow:10px 10px red'></div>"), 60, 60);
    expect(&f, &[(35, 35, RED), (25, 35, RED), (35, 25, RED), (25, 25, WHITE), (15, 35, WHITE), (45, 35, WHITE), (35, 45, WHITE), (22, 38, RED)]);
}

#[test]
fn spread_grows_the_shadow_in_every_direction() {
    // Shadow (5,5)-(35,35) around the box (10,10)-(30,30).
    let f = paint(&format!("<div style='position:absolute;left:10px;top:10px;{BOX}box-shadow:0 0 0 5px red'></div>"), 60, 60);
    expect(&f, &[(5, 20, RED), (4, 20, WHITE), (20, 5, RED), (20, 4, WHITE), (34, 20, RED), (35, 20, WHITE), (20, 34, RED), (20, 35, WHITE), (20, 20, WHITE)]);
}

#[test]
fn an_inset_shadow_is_the_padding_box_minus_the_moved_hole() {
    // Padding box (10,10)-(30,30), hole moved by (5,5): (15,15)-(35,35). Shadow where the box is and the hole is not.
    let f = paint(&format!("<div style='position:absolute;left:10px;top:10px;{BOX}background:lime;box-shadow:inset 5px 5px 0 red'></div>"), 60, 60);
    expect(&f, &[(12, 20, RED), (20, 12, RED), (14, 29, RED), (29, 14, RED), (15, 15, LIME), (20, 20, LIME), (29, 29, LIME), (35, 35, WHITE), (9, 20, WHITE)]);
}

#[test]
fn an_inset_shadow_stays_below_the_border() {
    // 2px blue border; padding box (12,12)-(28,28); inset shadow of spread 3 and no offset: a 3px red frame inside it.
    let f = paint("<div style='position:absolute;left:10px;top:10px;width:16px;height:16px;border:2px solid blue;background:lime;box-shadow:inset 0 0 0 3px red'></div>", 60, 60);
    expect(&f, &[(10, 20, BLUE), (11, 20, BLUE), (12, 20, RED), (14, 20, RED), (15, 20, LIME), (20, 20, LIME), (24, 20, LIME), (25, 20, RED), (27, 20, RED), (28, 20, BLUE), (29, 20, BLUE)]);
}

#[test]
fn the_first_of_several_shadows_is_on_top() {
    // Red (20,20)-(40,40) over blue (30,30)-(50,50).
    let f = paint(&format!("<div style='position:absolute;left:10px;top:10px;{BOX}box-shadow:10px 10px red, 20px 20px blue'></div>"), 70, 70);
    expect(&f, &[(35, 25, RED), (25, 35, RED), (35, 35, RED), (45, 45, BLUE), (45, 35, BLUE), (35, 45, BLUE), (55, 55, WHITE), (25, 25, WHITE)]);
}

#[test]
fn a_shadow_follows_the_radius() {
    // Circle of radius 20 at (30,30)-(70,70) (center (50,50)); shadow spread 4: radius 24. Pixel (34,34) centers at distance
    // 21.9 from the center: in the ring 20..24. Pixel (31,31): 26.2, outside; (50,50) is inside the (empty) box.
    let f = paint("<div style='position:absolute;left:30px;top:30px;width:40px;height:40px;border-radius:20px;box-shadow:0 0 0 4px red'></div>", 100, 100);
    expect(&f, &[(34, 34, RED), (31, 31, WHITE), (50, 50, WHITE), (50, 27, RED), (50, 25, WHITE)]);
}

#[test]
fn blur_is_a_gaussian_of_half_the_radius() {
    // Box (20,20)-(80,80), shadow moved by 20: right edge at x = 100, blur 10px so sigma 5. At the edge the coverage is 1/2:
    // the pixel centered 0.5 inside (x = 99) is Phi(0.1) = 0.54 of black, the one outside (x = 100) 0.46. Three box blurs
    // are within the 5% the spec asks for. Two sigma inside (x = 90) it is 0.977; at 3 sigma outside (x = 115) almost 0.
    let f = paint("<div style='position:absolute;left:20px;top:20px;width:60px;height:60px;box-shadow:20px 0 10px black'></div>", 140, 100);
    let g = |x: u32| at(&f, x, 50)[1];
    assert!((i32::from(g(99)) - 117).abs() <= 13, "x=99: {}", g(99));
    assert!((i32::from(g(100)) - 138).abs() <= 13, "x=100: {}", g(100));
    assert!(g(90) < 15, "x=90: {}", g(90));
    assert!(g(118) > 245, "x=118: {}", g(118));
    assert_eq!(at(&f, 50, 50), WHITE, "inside the box: nothing");
}

#[test]
fn blur_keeps_the_amount_of_shadow() {
    // A 20x20 shape moved far from its box, blurred by 8px: the sum of the coverage stays 400.
    let f = paint("<div style='position:absolute;left:10px;top:40px;width:20px;height:20px;box-shadow:100px 0 8px black'></div>", 200, 100);
    let m = mass(&f);
    assert!((m - 400.0).abs() < 400.0 * 0.02, "mass {m}");
}

#[test]
fn blur_masks_are_reused_while_scrolling() {
    let page = lay("<div style='position:absolute;left:10px;top:200px;width:30px;height:30px;box-shadow:0 0 6px black'></div><div style='height:400px'></div>", 100, 100);
    let a = draw(&page, 100, 100, 150.0);
    let b = draw(&page, 100, 100, 150.0);
    assert_eq!(a.data, b.data);
    // Scrolled by 20: the whole picture moves up by 20 rows.
    let c = draw(&page, 100, 100, 170.0);
    for y in 0..60u32 {
        for x in 0..100u32 {
            assert_eq!(at(&a, x, y + 20), at(&c, x, y), "({x},{y})");
        }
    }
}

#[test]
fn text_shadow_is_offset_and_below_the_text() {
    // "X" is a 10x10 square at (0,0); red shadow at (5,5)-(15,15).
    let f = paint("<p style='margin:0;font:10px/10px sans-serif;color:black;text-shadow:5px 5px red'>X</p>", 30, 30);
    expect(&f, &[(2, 2, [0, 0, 0, 255]), (7, 7, [0, 0, 0, 255]), (12, 12, RED), (12, 3, WHITE), (3, 12, WHITE), (17, 12, WHITE)]);
}

#[test]
fn several_text_shadows_the_first_on_top() {
    let f = paint("<p style='margin:0;font:10px/10px sans-serif;color:black;text-shadow:10px 0 red, 12px 0 blue'>X</p>", 40, 20);
    expect(&f, &[(15, 5, RED), (21, 5, BLUE), (25, 5, WHITE)]);
}

#[test]
fn a_blurred_text_shadow_keeps_its_mass() {
    // The glyph is 100 px of ink, moved far away and blurred by 4: the mass stays 100.
    let f = paint("<p style='margin:20px 0 0;font:10px/10px sans-serif;color:transparent;text-shadow:60px 0 4px black'>X</p>", 100, 50);
    let m = mass(&f);
    assert!((m - 100.0).abs() < 4.0, "mass {m}");
}

#[test]
fn opacity_mixes_the_whole_group_once() {
    // Red at 128/255 over white: 127 in blue and green, red stays 255.
    let f = paint(&format!("<div style='{BOX}background:red;opacity:.5'></div>"), 40, 40);
    expect(&f, &[(5, 5, [127, 127, 255, 255]), (25, 5, WHITE)]);
    // The overlapping children of a half transparent group do not show through each other: blue on top, once.
    let f = paint(&format!("<div style='opacity:.5;width:40px;height:40px'><div style='{BOX}background:red'></div><div style='{BOX}background:blue;margin-top:-10px'></div></div>"), 60, 60);
    expect(&f, &[(5, 5, [127, 127, 255, 255]), (5, 15, [255, 127, 127, 255]), (5, 25, [255, 127, 127, 255])]);
}

#[test]
fn opacity_zero_and_one_and_percentages() {
    let f = paint(&format!("<div style='{BOX}background:red;opacity:0'></div>"), 30, 30);
    expect(&f, &[(5, 5, WHITE)]);
    let f = paint(&format!("<div style='{BOX}background:red;opacity:1'></div>"), 30, 30);
    expect(&f, &[(5, 5, RED)]);
    let f = paint(&format!("<div style='{BOX}background:red;opacity:50%'></div>"), 30, 30);
    expect(&f, &[(5, 5, [127, 127, 255, 255])]);
    // Out of range is clamped; nonsense is dropped.
    let f = paint(&format!("<div style='{BOX}background:red;opacity:3'></div><div style='{BOX}background:blue;opacity:abc'></div>"), 30, 50);
    expect(&f, &[(5, 5, RED), (5, 25, BLUE)]);
}

#[test]
fn opacity_makes_a_stacking_context_above_the_blocks_that_follow() {
    // The half transparent red block is painted with the positioned boxes, over the blue block that comes after it:
    // red 50% over blue is (b 127, g 0, r 128).
    let f = paint(&format!("<div style='{BOX}background:red;opacity:.5'></div><div style='{BOX}background:blue;margin-top:-20px'></div>"), 40, 40);
    expect(&f, &[(5, 5, [127, 0, 128, 255])]);
}

#[test]
fn translate_moves_the_painting_not_the_layout() {
    let f = paint(&format!("<div style='position:absolute;left:10px;top:10px;{BOX}background:red;transform:translate(30px,20px)'></div>"), 80, 60);
    expect(&f, &[(45, 35, RED), (40, 30, RED), (59, 49, RED), (60, 49, WHITE), (39, 35, WHITE), (15, 15, WHITE)]);
}

#[test]
fn translate_percentages_refer_to_the_box() {
    // The usual centering: left/top 50% of 100, then back by half of its own 20.
    let f = paint(&format!("<div style='position:relative;width:100px;height:100px'><div style='position:absolute;left:50%;top:50%;{BOX}background:red;transform:translate(-50%,-50%)'></div></div>"), 100, 100);
    expect(&f, &[(40, 40, RED), (59, 59, RED), (39, 50, WHITE), (60, 50, WHITE), (50, 39, WHITE), (50, 60, WHITE)]);
}

#[test]
fn matrix_and_the_3d_functions_keep_their_2d_part() {
    for t in ["matrix(1,0,0,1,15,5)", "translate3d(15px,5px,40px)", "translateX(15px) translateY(5px) translateZ(9px)", "matrix3d(1,0,0,0, 0,1,0,0, 0,0,1,0, 15,5,0,1)", "translate(15px,5px) rotateZ(0deg) scaleZ(2) perspective(100px)"] {
        let f = paint(&format!("<div style='position:absolute;left:10px;top:10px;{BOX}background:red;transform:{t}'></div>"), 60, 50);
        expect(&f, &[(25, 15, RED), (44, 34, RED), (45, 34, WHITE), (24, 15, WHITE), (25, 14, WHITE)]);
    }
}

#[test]
fn scale_about_the_center_and_about_an_origin() {
    // 20x20 at (40,40), scale(2) about the center (50,50): (30,30)-(70,70).
    let f = paint(&format!("<div style='position:absolute;left:40px;top:40px;{BOX}background:red;transform:scale(2)'></div>"), 100, 100);
    expect(&f, &[(30, 30, RED), (69, 69, RED), (29, 50, WHITE), (70, 50, WHITE), (50, 29, WHITE), (50, 70, WHITE)]);
    // About the top left corner (10,10): (10,10)-(50,50).
    let f = paint(&format!("<div style='position:absolute;left:10px;top:10px;{BOX}background:red;transform-origin:0 0;transform:scale(2)'></div>"), 100, 100);
    expect(&f, &[(10, 10, RED), (49, 49, RED), (50, 30, WHITE), (30, 50, WHITE), (9, 9, WHITE)]);
    // scaleX(-1) about the center flips in place: a box with a blue left half stays where it is, mirrored.
    let f = paint("<div style='position:absolute;left:10px;top:10px;width:20px;height:10px;background:linear-gradient(90deg,blue 50%,red 50%);transform:scaleX(-1)'></div>", 50, 30);
    expect(&f, &[(12, 12, RED), (27, 12, BLUE), (9, 12, WHITE), (30, 12, WHITE)]);
}

#[test]
fn rotate_a_quarter_turn_is_exact() {
    // 40x20 at (30,40), center (50,50), turned 90 degrees: 20 wide, 40 tall, (40,30)-(60,70).
    let f = paint("<div style='position:absolute;left:30px;top:40px;width:40px;height:20px;background:red;transform:rotate(90deg)'></div>", 100, 100);
    expect(&f, &[(40, 30, RED), (59, 69, RED), (50, 35, RED), (35, 50, WHITE), (65, 50, WHITE), (50, 29, WHITE), (50, 70, WHITE), (39, 50, WHITE), (60, 50, WHITE)]);
}

#[test]
fn rotate_and_skew_keep_the_area() {
    // A 20x20 square turned 45 degrees is still 400 px of red (by the green channel), and so is a skewed one.
    // (The area scales by the determinant: 1 - tan 20deg * tan 10deg = 0.936 for the last.)
    for (t, det) in [("rotate(45deg)", 1.0), ("skewX(45deg)", 1.0), ("rotate(0.1turn) scale(1)", 1.0), ("skew(20deg, 10deg)", 0.9358)] {
        let f = paint(&format!("<div style='position:absolute;left:40px;top:40px;{BOX}background:red;transform:{t}'></div>"), 100, 100);
        let m = mass(&f);
        assert!((m - 400.0 * det).abs() < 400.0 * 0.04, "{t}: {m}");
    }
    // The diamond: center red, a corner of the old square white.
    let f = paint(&format!("<div style='position:absolute;left:40px;top:40px;{BOX}background:red;transform:rotate(45deg)'></div>"), 100, 100);
    expect(&f, &[(50, 50, RED), (50, 38, RED), (41, 41, WHITE), (59, 41, WHITE)]);
    // skewX(45deg) about the center (50,50): row y = 41 (center 41.5) moves by -8.5 -> x from 31.5 to 51.5.
    let f = paint(&format!("<div style='position:absolute;left:40px;top:40px;{BOX}background:red;transform:skewX(45deg)'></div>"), 100, 100);
    near(&f, 40, 41, RED, 2);
    near(&f, 33, 41, RED, 2);
    near(&f, 55, 41, WHITE, 2);
}

#[test]
fn a_transformed_box_contains_its_absolute_descendants() {
    // The absolute child is placed against the transformed parent (at 20,20), not the page.
    let f = paint("<div style='position:absolute;left:20px;top:20px;width:50px;height:50px;transform:translate(0,0)'><div style='position:absolute;left:5px;top:5px;width:10px;height:10px;background:red'></div></div>", 60, 60);
    expect(&f, &[(25, 25, RED), (34, 34, RED), (35, 35, WHITE), (5, 5, WHITE)]);
    // Fixed descendants too: they scroll with the transformed box and are not stuck to the viewport.
    let page = lay("<div style='margin-top:100px;width:50px;height:50px;transform:translate(0,0)'><div style='position:fixed;left:5px;top:5px;width:10px;height:10px;background:red'></div></div><div style='height:500px'></div>", 60, 60);
    let f = draw(&page, 60, 60, 80.0);
    expect(&f, &[(8, 25, RED), (8, 5, WHITE)]);
}

#[test]
fn children_and_clips_go_through_the_transform() {
    // overflow:hidden inside a translated parent clips in the parent's coordinates.
    let f = paint("<div style='position:absolute;left:0;top:0;width:60px;height:60px;transform:translate(10px,10px)'><div style='width:20px;height:20px;overflow:hidden'><div style='width:50px;height:50px;background:red'></div></div></div>", 60, 60);
    expect(&f, &[(10, 10, RED), (29, 29, RED), (30, 15, WHITE), (15, 30, WHITE), (9, 9, WHITE)]);
}

#[test]
fn text_in_a_scaled_box_is_drawn_at_the_scaled_size() {
    // "X" at 10px is a 10x10 square at (0,0); scale(2) about the origin 0 0: (0,0)-(20,20), drawn with a 20px glyph.
    let f = paint("<p style='margin:0;font:10px/10px sans-serif;width:10px;transform-origin:0 0;transform:scale(2)'>X</p>", 40, 40);
    expect(&f, &[(1, 1, [0, 0, 0, 255]), (19, 19, [0, 0, 0, 255]), (21, 10, WHITE), (10, 21, WHITE)]);
}

#[test]
fn the_page_grows_to_the_transformed_bounds() {
    let page = lay("<div style='width:20px;height:20px;background:red;transform:translateY(500px)'></div>", 100, 100);
    assert!(page.list.height.to_f32() >= 520.0, "height {}", page.list.height.to_f32());
}

#[test]
fn the_pointer_is_mapped_back_through_the_transform() {
    let page = lay("<div style='font:10px/10px sans-serif;transform:translate(100px,20px);width:200px'><a href='/x'>AAAA</a></div><div style='font:10px/10px sans-serif;transform:rotate(90deg);width:60px;margin-top:100px'><a href='/y'>BBBBBB</a></div>", 400, 300);
    let index = PageIndex::new(&page.res);
    let hit = |x: f32, y: f32| index.link_at(&page.doc, &page.res, x, y, y).and_then(|n| link_target(&page.doc, n)).map(|t| t.0);
    // "AAAA" is 40x10 at (0,0), moved to (100,20)-(140,30).
    assert_eq!(hit(110.0, 25.0), Some("/x".to_string()));
    assert_eq!(hit(10.0, 5.0), None, "where it used to be");
    assert_eq!(hit(110.0, 5.0), None);
    // "BBBBBB" is 60x10 at (0,110) (the first block is 10 high, plus the 100px margin), turned about its center (30,115):
    // 10 wide, 60 tall, x 25..35, y 85..145.
    assert_eq!(hit(30.0, 90.0), Some("/y".to_string()));
    assert_eq!(hit(30.0, 140.0), Some("/y".to_string()));
    assert_eq!(hit(5.0, 115.0), None, "the old horizontal strip");
    assert_eq!(hit(55.0, 115.0), None);
}

#[test]
fn hostile_values_do_not_hang_or_panic() {
    for style in [
        "box-shadow:0 0 1e9px 1e9px red",
        "box-shadow:inset 0 0 1e9px 1e9px red",
        "box-shadow:0 0 100000px red",
        "box-shadow:1e9px 1e9px 5px red;border-radius:1e9px",
        "transform:scale(1e30)",
        "transform:scale(1e-30)",
        "transform:scale(0)",
        "transform:rotate(1e20turn)",
        "transform:matrix(1e38,1e38,1e38,1e38,1e38,1e38)",
        "transform:translate(1e30px,-1e30px)",
        "transform:skewX(89.9999deg)",
        "transform:skew(90deg,90deg)",
        "opacity:.5;transform:rotate(3deg);box-shadow:0 0 30px red;border-radius:10px",
        "text-shadow:0 0 1e9px red",
    ] {
        let f = paint(&format!("<div style='position:absolute;left:10px;top:10px;{BOX}background:blue;{style}'>text</div>"), 80, 80);
        assert_eq!(f.data.len(), 80 * 80 * 4, "{style}");
    }
    // Nested groups beyond the depth limit.
    let mut html = String::new();
    for _ in 0..20 {
        html.push_str("<div style='opacity:.9;transform:rotate(1deg)'>");
    }
    html.push_str("<div style='width:10px;height:10px;background:red'></div>");
    let f = paint(&html, 60, 60);
    assert_eq!(f.data.len(), 60 * 60 * 4);
}

#[test]
fn lime_is_lime() {
    // The shadow list and the transform list parse in the rules (a bad value drops the declaration).
    let f = paint(&format!("<div style='{BOX}background:lime;box-shadow:nonsense;transform:wobble(3)'></div>"), 30, 30);
    expect(&f, &[(5, 5, LIME), (25, 5, WHITE)]);
}

#[test]
fn a_transformed_table_and_float_are_painted_once_through_the_transform() {
    // The text is the top left 10x10 of a 100x100 table; turned half a turn it is at the bottom right.
    let f = paint("<div style='display:table;width:100px;height:100px;transform:rotate(180deg);font:10px/10px sans-serif'>X</div>", 120, 120);
    let b = [0, 0, 0, 255];
    expect(&f, &[(95, 95, b), (5, 5, WHITE), (95, 5, WHITE)]);
    // A float with a transform is painted with the positioned boxes, once: (30,0)-(40,10) black moved by 20 to the right only.
    let f = paint("<div style='float:left;margin-left:30px;width:10px;height:10px;background:black;transform:translateX(20px)'></div>", 80, 20);
    expect(&f, &[(55, 5, b), (35, 5, WHITE)]);
}

#[test]
fn groups_nested_past_the_layer_limit_are_drawn_without_their_own_opacity() {
    // Eight nested groups of opacity .9 around a red box. The first six get a layer (.9^6 = .53 of red over white); the last two have
    // no layer left and are drawn straight: the red is there, not missing.
    let mut html = String::new();
    for _ in 0..8 {
        html.push_str("<div style='width:30px;height:30px;opacity:.9'>");
    }
    html.push_str(&format!("<div style='{BOX}background:red'></div>"));
    html.push_str(&"</div>".repeat(8));
    let f = paint(&html, 40, 40);
    near(&f, 5, 5, [118, 118, 255, 255], 8);
    expect(&f, &[(35, 35, WHITE)]);
    // Seventy nested ones are over the recursion limit and are not drawn; the page still comes out (and quickly).
    let mut deep = String::new();
    for _ in 0..70 {
        deep.push_str("<div style='opacity:.9'>");
    }
    deep.push_str(&format!("<div style='{BOX}background:red'></div>"));
    deep.push_str(&"</div>".repeat(70));
    let f = paint(&deep, 40, 40);
    assert_eq!(f.data.len(), 40 * 40 * 4);
}

#[test]
fn nested_groups_past_the_layer_limit_keep_their_scale_and_drop_a_turn() {
    // The seventh group scales by 2 about its top left (a shift and a scale need no layer); the eighth turns, which a flat drawing cannot.
    let mut html = String::new();
    for _ in 0..6 {
        html.push_str("<div style='width:60px;height:60px;opacity:.9'>");
    }
    html.push_str("<div style='width:30px;height:30px;transform-origin:0 0;transform:scale(2)'><div style='width:10px;height:10px;transform:rotate(90deg);background:red'></div></div>");
    html.push_str(&"</div>".repeat(6));
    let f = paint(&html, 70, 70);
    // Red 10x10 doubled to 20x20, seen through six layers of .9 (the turn is dropped: the box stays at the top left).
    near(&f, 15, 15, [118, 118, 255, 255], 8);
    expect(&f, &[(25, 25, WHITE)]);
}

#[test]
fn a_huge_skewed_element_is_drawn_in_bands_not_dropped() {
    // skewX(89.5deg) about the top left corner: the part of the element that lands in the 100x100 window is a strip of
    // 11.5k x 100 pixels, more than one layer may hold (about 520k pixels), so it is drawn in bands of rows.
    let f = paint("<div style='position:absolute;left:-12000px;top:0;width:12100px;height:100px;background:lime;transform-origin:0 0;transform:skewX(89.5deg)'></div>", 100, 100);
    expect(&f, &[(50, 50, LIME), (3, 3, LIME), (97, 97, LIME), (50, 99, LIME), (2, 98, LIME)]);
    // With opacity: the same, mixed once.
    let f = paint("<div style='position:absolute;left:-12000px;top:0;width:12100px;height:100px;background:lime;opacity:.5;transform-origin:0 0;transform:skewX(89.5deg)'></div>", 100, 100);
    near(&f, 50, 50, [127, 255, 127, 255], 2);
    near(&f, 50, 99, [127, 255, 127, 255], 2);
}

#[test]
fn a_blurred_text_shadow_is_kept_while_scrolling_and_equals_a_fresh_one() {
    let body = "<p style='margin:200px 0 0 5px;font:10px/10px sans-serif;color:black;text-shadow:3px 2px 4px blue'>XXXXXX</p><div style='height:400px'></div>";
    // The mask cache is per thread: one thread draws three frames (the later ones from the cache), another only the last.
    let frames = |scrolls: Vec<f32>| {
        std::thread::spawn(move || {
            let page = lay(body, 100, 100);
            scrolls.iter().map(|&s| draw(&page, 100, 100, s)).collect::<Vec<_>>()
        })
        .join()
        .unwrap()
    };
    let warm = frames(vec![150.0, 160.0, 170.0]);
    let fresh = frames(vec![170.0]);
    assert_eq!(warm[2].data, fresh[0].data);
    // It is the same picture ten rows higher, and the shadow is there at all (blue in the green channel).
    for y in 0..60u32 {
        for x in 0..100u32 {
            assert_eq!(at(&warm[1], x, y + 10), at(&warm[2], x, y), "({x},{y})");
        }
    }
    assert!(mass(&warm[2]) > 10.0);
    let blue = (0..100u32).any(|x| (0..100u32).any(|y| i32::from(at(&warm[2], x, y)[0]) > i32::from(at(&warm[2], x, y)[2]) + 40));
    assert!(blue, "a blue shadow");
}
