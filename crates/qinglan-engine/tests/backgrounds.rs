//! Backgrounds, gradients, border radius, border styles and outlines (step 2c-1), with the test platform. The numbers
//! are worked out by hand in the comments; where a shape is round, the covered area (the sum of the coverage of all
//! pixels) is compared with the area of the shape.
#![allow(clippy::indexing_slicing, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use qinglan_engine::css::{NoResources, StyleOptions, style_document_with};
use qinglan_engine::encoding::decode_document;
use qinglan_engine::html::dom::NodeId;
use qinglan_engine::html::treebuilder::parse_document;
use qinglan_engine::layout::{ImageSizes, LayoutOptions, LayoutResult, layout_document_with};
use qinglan_engine::layout::image::Partial;
use qinglan_engine::paint::images::{ImageSource, NoBitmaps};
use qinglan_engine::paint::raster::{Frame, View, render_frame};
use qinglan_engine::paint::{DisplayList, build_display_list};
use qinglan_engine::platform::image::DecodedImage;
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

fn lay(html: &str, vw: u32, vh: u32, dpr: f32, base: Option<&Url>, sizes: &ImageSizes) -> Page {
    let (vw, vh) = (vw as f32, vh as f32);
    let platform = TestPlatform;
    let decoded = decode_document(html.as_bytes(), None, &BasicPlatform);
    let mut doc = parse_document(decoded.text);
    let sopts = StyleOptions { viewport_w: vw, viewport_h: vh, ..StyleOptions::default() };
    let styled = style_document_with(&mut doc, base, &mut NoResources, &sopts, Some(&platform as &dyn Platform));
    let res = layout_document_with(&doc, &styled.tree, &platform, &LayoutOptions { viewport_w: vw, viewport_h: vh, dpr, ..LayoutOptions::default() }, sizes);
    let list = build_display_list(&doc, &res, &platform);
    Page { res, list }
}

fn draw(page: &Page, vw: u32, vh: u32, dpr: f32, scroll: f32, images: &mut dyn ImageSource) -> Frame {
    let mut frame = Frame::new((vw as f32 * dpr).round() as u32, (vh as f32 * dpr).round() as u32);
    render_frame(&page.list, &TestPlatform, images, View { scroll_x: 0.0, scroll_y: scroll, dpr }, &mut frame);
    frame
}

fn paint(html: &str, vw: u32, vh: u32) -> Frame {
    let page = lay(html, vw, vh, 1.0, None, &ImageSizes::new());
    draw(&page, vw, vh, 1.0, 0.0, &mut NoBitmaps)
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

/// Every channel of pixel (`x`, `y`) within `tol` of `want` (BGRA).
#[track_caller]
fn near(f: &Frame, x: u32, y: u32, want: [u8; 4], tol: i32) {
    let p = at(f, x, y);
    assert!(p.iter().zip(want).all(|(a, b)| (i32::from(*a) - i32::from(b)).abs() <= tol), "pixel ({x},{y}) is {p:?}, wanted {want:?} +-{tol}");
}

/// The summed coverage of `channel` going from white (255) to 0, in pixels: the area of something drawn in a color that
/// has 0 in that channel on white.
fn area(f: &Frame, channel: usize) -> f32 {
    let mut s = 0.0;
    for y in 0..f.height {
        for x in 0..f.width {
            s += (255.0 - f32::from(at(f, x, y)[channel])) / 255.0;
        }
    }
    s
}

struct Gen<F: Fn(u32, u32, u32, u32) -> [u8; 4]>(F, Vec<(u32, u32)>);

impl<F: Fn(u32, u32, u32, u32) -> [u8; 4]> ImageSource for Gen<F> {
    fn bitmap(&mut self, _node: NodeId, w: u32, h: u32) -> Option<Arc<DecodedImage>> {
        self.1.push((w, h));
        let mut bgra = Vec::new();
        for y in 0..h {
            for x in 0..w {
                bgra.extend_from_slice(&(self.0)(x, y, w, h));
            }
        }
        Some(Arc::new(DecodedImage { width: w, height: h, natural_width: w, natural_height: h, bgra }))
    }
}

/// A page whose first `url()` background has the natural size `nat`, drawn with `Gen` bitmaps: left half red, right half blue.
fn image_page(style: &str, nat: (u32, u32), vw: u32, vh: u32) -> (Frame, Vec<(u32, u32)>) {
    let html = format!("<body style='margin:0'><div style=\"{style}\"></div>");
    let first = lay(&html, vw, vh, 1.0, None, &ImageSizes::new());
    let mut sizes = ImageSizes::new();
    sizes.set_loaded(first.res.images[0].node, nat.0, nat.1);
    let page = lay(&html, vw, vh, 1.0, None, &sizes);
    let mut src = Gen(|x, _, w, _| if x * 2 < w { RED } else { BLUE }, Vec::new());
    let f = draw(&page, vw, vh, 1.0, 0.0, &mut src);
    (f, src.1)
}

// ---- gradients ---------------------------------------------------------------------------------------------------

#[test]
fn a_linear_gradient_runs_from_one_color_to_the_other() {
    // 100 px wide, black to white: pixel x has t = (x + 0.5) / 100 and so the value 255 t.
    let f = paint("<body style='margin:0'><div style='width:100px;height:10px;background:linear-gradient(to right,#000,#fff)'></div>", 120, 20);
    near(&f, 0, 5, [1, 1, 1, 255], 2);
    near(&f, 49, 5, [126, 126, 126, 255], 2);
    near(&f, 50, 5, [129, 129, 129, 255], 2);
    near(&f, 99, 5, [254, 254, 254, 255], 2);
    expect(&f, &[(100, 5, WHITE), (50, 10, WHITE)]);
    // 180deg is `to bottom`; an angle of 0 goes up.
    let f = paint("<body style='margin:0'><div style='width:10px;height:100px;background:linear-gradient(0deg,#000,#fff)'></div>", 20, 120);
    near(&f, 5, 0, [254, 254, 254, 255], 2);
    near(&f, 5, 99, [1, 1, 1, 255], 2);
}

#[test]
fn hard_stops_repeating_gradients_and_positions_from_the_gradient_line() {
    let f = paint("<body style='margin:0'><div style='width:100px;height:10px;background:linear-gradient(90deg,#f00 50%,#00f 50%)'></div>", 120, 20);
    expect(&f, &[(0, 5, RED), (49, 5, RED), (50, 5, BLUE), (99, 5, BLUE)]);
    let f = paint("<body style='margin:0'><div style='width:40px;height:10px;background:repeating-linear-gradient(90deg,#f00 0 10px,#00f 10px 20px)'></div>", 60, 20);
    expect(&f, &[(0, 5, RED), (9, 5, RED), (10, 5, BLUE), (19, 5, BLUE), (20, 5, RED), (30, 5, BLUE), (39, 5, BLUE)]);
    // Stops without a position are spread evenly: red, green, blue at 0, 50 and 100 px.
    let f = paint("<body style='margin:0'><div style='width:100px;height:10px;background:linear-gradient(to right,#f00,#0f0,#00f)'></div>", 120, 20);
    near(&f, 50, 5, [3, 252, 3, 255], 6);
    near(&f, 25, 5, [1, 128, 127, 255], 4);
}

#[test]
fn a_corner_direction_puts_the_halfway_color_on_the_other_diagonal() {
    // `to bottom right` on a square: the 50% line is the anti-diagonal, so both ends of it have the same (middle) color.
    let f = paint("<body style='margin:0'><div style='width:100px;height:100px;background:linear-gradient(to bottom right,#000,#fff)'></div>", 120, 120);
    near(&f, 99, 0, [128, 128, 128, 255], 4);
    near(&f, 0, 99, [128, 128, 128, 255], 4);
    near(&f, 2, 2, [6, 6, 6, 255], 6);
    near(&f, 97, 97, [250, 250, 250, 255], 6);
}

#[test]
fn radial_and_conic_gradients() {
    let f = paint("<body style='margin:0'><div style='width:100px;height:100px;background:radial-gradient(circle closest-side,#f00 50%,#00f 50%)'></div>", 120, 120);
    // Closest side is 50 px, the hard stop at 25 px.
    expect(&f, &[(50, 50, RED), (70, 50, RED), (80, 50, BLUE), (50, 80, BLUE), (2, 2, BLUE)]);
    let f = paint("<body style='margin:0'><div style='width:100px;height:100px;background:conic-gradient(#f00 25%,#00f 0 50%,#0f0 0 75%,#ff0 0)'></div>", 120, 120);
    expect(&f, &[(75, 25, RED), (75, 75, BLUE), (25, 75, GREEN), (25, 25, YELLOW)]);
}

#[test]
fn layers_are_painted_from_the_last_to_the_first_over_the_color() {
    let f = paint("<body style='margin:0'><div style='width:100px;height:10px;background:linear-gradient(to right,transparent 50%,#00f 50%),#f00'></div>", 120, 20);
    expect(&f, &[(10, 5, RED), (90, 5, BLUE)]);
    let f = paint("<body style='margin:0'><div style='width:100px;height:10px;background:linear-gradient(#00f,#00f) 0 0/50% 100% no-repeat,linear-gradient(#0f0,#0f0) 100% 0/50% 100% no-repeat,#f00'></div>", 120, 20);
    expect(&f, &[(10, 5, BLUE), (90, 5, GREEN)]);
}

// ---- background images -------------------------------------------------------------------------------------------

#[test]
fn background_images_tile_by_repeat_position_and_size() {
    // A 10x10 image (left half red, right half blue) in a 40x20 box.
    let (f, asked) = image_page("width:40px;height:20px;background:url(a.png)", (10, 10), 60, 30);
    assert_eq!(asked, vec![(10, 10)]);
    expect(&f, &[(2, 2, RED), (7, 2, BLUE), (12, 2, RED), (37, 2, BLUE), (2, 12, RED), (37, 17, BLUE), (40, 2, WHITE), (2, 20, WHITE)]);
    let (f, _) = image_page("width:40px;height:20px;background:url(a.png) no-repeat", (10, 10), 60, 30);
    expect(&f, &[(2, 2, RED), (7, 2, BLUE), (12, 2, WHITE), (2, 12, WHITE)]);
    let (f, _) = image_page("width:40px;height:20px;background:url(a.png) repeat-x", (10, 10), 60, 30);
    expect(&f, &[(12, 2, RED), (37, 2, BLUE), (2, 12, WHITE)]);
    // 10px 5px: the tile starts at x 10, y 5; `repeat-y` only repeats down.
    let (f, _) = image_page("width:40px;height:20px;background:url(a.png) 10px 5px no-repeat", (10, 10), 60, 30);
    expect(&f, &[(12, 7, RED), (17, 7, BLUE), (9, 7, WHITE), (12, 4, WHITE), (12, 16, WHITE)]);
    // Percentages place the middle of the image at the middle of the box: (40 - 10) * 50% = 15.
    let (f, _) = image_page("width:40px;height:20px;background:url(a.png) 50% 50% no-repeat", (10, 10), 60, 30);
    expect(&f, &[(16, 6, RED), (22, 6, BLUE), (14, 6, WHITE), (26, 6, WHITE)]);
    // `right 5px bottom 0` ends 5 px from the right edge.
    let (f, _) = image_page("width:40px;height:20px;background:url(a.png) right 5px bottom 0 no-repeat", (10, 10), 60, 30);
    expect(&f, &[(26, 12, RED), (31, 12, BLUE), (24, 12, WHITE), (36, 12, WHITE), (26, 9, WHITE)]);
}

#[test]
fn background_size_cover_contain_lengths_space_and_round() {
    // 20x10 image, 40x40 box: cover scales by 4 (80x40, clipped), contain by 2 (40x20 at the top).
    let (f, asked) = image_page("width:40px;height:40px;background:url(a.png) 0 0/cover no-repeat", (20, 10), 60, 50);
    assert_eq!(asked, vec![(80, 40)]);
    expect(&f, &[(5, 5, RED), (35, 5, RED), (35, 35, RED)]);
    let (f, asked) = image_page("width:40px;height:40px;background:url(a.png) 0 0/contain no-repeat", (20, 10), 60, 50);
    assert_eq!(asked, vec![(40, 20)]);
    expect(&f, &[(5, 5, RED), (30, 5, BLUE), (5, 25, WHITE)]);
    // One length keeps the ratio: 30px wide gives 15 px high.
    let (f, asked) = image_page("width:40px;height:40px;background:url(a.png) 0 0/30px no-repeat", (20, 10), 60, 50);
    assert_eq!(asked, vec![(30, 15)]);
    expect(&f, &[(5, 5, RED), (25, 5, BLUE), (5, 20, WHITE), (35, 5, WHITE)]);
    // `space`: two 15 px tiles in 40 px leave a 10 px gap between them; they start at the edges.
    let (f, _) = image_page("width:40px;height:15px;background:url(a.png) 0 0/15px 15px space no-repeat", (15, 15), 60, 30);
    expect(&f, &[(3, 5, RED), (12, 5, BLUE), (20, 5, WHITE), (27, 5, RED), (37, 5, BLUE)]);
    let (f, _) = image_page("width:40px;height:15px;background:url(a.png) 0 0/15px 15px space no-repeat", (15, 15), 60, 30);
    expect(&f, &[(15, 5, WHITE), (24, 5, WHITE), (25, 5, RED)]);
    // `round`: 40 / 15 = 2.67 rounds to 3 tiles of 13.33 px (asked for at 13 px).
    let (f, asked) = image_page("width:40px;height:15px;background:url(a.png) 0 0/15px 15px round no-repeat", (15, 15), 60, 30);
    assert!(asked.iter().all(|a| a.0 == 13), "{asked:?}");
    expect(&f, &[(2, 5, RED), (9, 5, BLUE), (15, 5, RED), (22, 5, BLUE), (28, 5, RED), (35, 5, BLUE)]);
}

#[test]
fn origin_and_clip_choose_the_boxes() {
    // Box: border 0..5 (transparent), padding 5..10, content 10..30 (x and y).
    let div = "width:20px;height:20px;padding:5px;border:5px solid transparent;background:#f00";
    let f = paint(&format!("<body style='margin:0'><div style='{div}'></div>"), 60, 60);
    expect(&f, &[(2, 2, RED), (7, 7, RED), (12, 12, RED), (40, 40, WHITE)]);
    let f = paint(&format!("<body style='margin:0'><div style='{div};background-clip:padding-box'></div>"), 60, 60);
    expect(&f, &[(2, 2, WHITE), (7, 7, RED), (33, 33, RED), (37, 37, WHITE)]);
    let f = paint(&format!("<body style='margin:0'><div style='{div};background-clip:content-box'></div>"), 60, 60);
    expect(&f, &[(7, 7, WHITE), (12, 12, RED), (29, 29, RED), (32, 32, WHITE)]);
    // Gradient positioned from the content box (origin) but painted all over the border box (clip): it repeats.
    let g = "background:linear-gradient(90deg,#f00 0 50%,#00f 0) 0 0/20px 20px repeat-x;background-origin:content-box";
    let f = paint(&format!("<body style='margin:0'><div style='width:20px;height:20px;padding:5px;border:5px solid transparent;{g}'></div>"), 60, 60);
    // Tile 0 starts at x 10 (red 10..20, blue 20..30); the tile before it ends at 10 (blue 0..10); repeat-x only fills y 10..30.
    expect(&f, &[(12, 12, RED), (22, 12, BLUE), (5, 12, BLUE), (35, 12, RED), (12, 5, WHITE), (12, 35, WHITE)]);
}

#[test]
fn urls_are_made_absolute_against_the_sheet_and_requested() {
    let html = "<style>div{background:url(img/a.png)}</style><div>x</div>";
    let base = Url::parse("http://example.com/dir/page.html", None).unwrap();
    let page = lay(html, 100, 50, 1.0, Some(&base), &ImageSizes::new());
    let srcs: Vec<&str> = page.res.images.iter().map(|r| r.src.as_str()).collect();
    assert_eq!(srcs, vec!["http://example.com/dir/img/a.png"]);
    // Nothing is drawn until the picture's size is known.
    let f = draw(&page, 100, 50, 1.0, 0.0, &mut NoBitmaps);
    assert_eq!(at(&f, 5, 5), WHITE);
}

#[test]
fn attachment_fixed_is_treated_as_scroll_and_the_canvas_takes_the_root_background() {
    let f = paint("<body style='margin:0'><div style='width:20px;height:10px;background:#f00 fixed'></div>", 40, 20);
    expect(&f, &[(5, 5, RED)]);
    // The gradient of <html> is positioned by the root's box (50 px high, the body) and repeats over the rest of the canvas.
    let f = paint("<html style='background:linear-gradient(#000,#fff)'><body style='margin:0;height:50px'>", 40, 120);
    near(&f, 5, 0, [2, 2, 2, 255], 3);
    near(&f, 5, 49, [253, 253, 253, 255], 3);
    near(&f, 5, 50, [2, 2, 2, 255], 3);
    // 119 - 100 = 19 rows into the third copy: (19.5 / 50) * 255.
    near(&f, 5, 119, [100, 100, 100, 255], 5);
}

// ---- border radius ------------------------------------------------------------------------------------------------

#[test]
fn a_radius_rounds_the_background_with_exact_coverage() {
    let f = paint("<body style='margin:0'><div style='width:40px;height:40px;border-radius:20px;background:#f00'></div>", 50, 50);
    expect(&f, &[(20, 20, RED), (6, 6, RED), (0, 0, WHITE), (3, 3, WHITE), (39, 39, WHITE)]);
    near(&f, 0, 20, RED, 8);
    near(&f, 20, 0, RED, 8);
    // Pixel (2,10): the circle's edge runs from x 2.68 (top of the pixel) to 2.14 (bottom): 3 - 2.41 = 59% covered.
    let edge = at(&f, 2, 10);
    assert!(edge[1] > 85 && edge[1] < 125, "{edge:?}");
    let a = area(&f, 1);
    assert!((a - 1256.64).abs() < 2.0, "{a}");
    // Elliptical, 20px / 10px on a 40x20 box; and percentages, and radii that overlap are scaled down together.
    let f = paint("<body style='margin:0'><div style='width:40px;height:20px;border-radius:20px/10px;background:#f00'></div>", 50, 30);
    assert!((area(&f, 1) - 628.32).abs() < 2.0);
    let f = paint("<body style='margin:0'><div style='width:40px;height:40px;border-radius:50%;background:#f00'></div>", 50, 50);
    assert!((area(&f, 1) - 1256.64).abs() < 2.0);
    let f = paint("<body style='margin:0'><div style='width:40px;height:40px;border-radius:1000px;background:#f00'></div>", 50, 50);
    assert!((area(&f, 1) - 1256.64).abs() < 2.0, "css-backgrounds-3 5.5: 1000px is scaled to 20px");
    // One round corner: top-left 20px of a 40x40 box loses (1 - pi/4) * 400 = 85.84 px.
    let f = paint("<body style='margin:0'><div style='width:40px;height:40px;border-top-left-radius:20px;background:#f00'></div>", 50, 50);
    assert!((area(&f, 1) - (1600.0 - 85.84)).abs() < 1.5);
    expect(&f, &[(0, 0, WHITE), (39, 0, RED), (0, 39, RED), (39, 39, RED)]);
}

#[test]
fn a_rounded_border_is_a_ring() {
    // Outer 40x40, radius 20, border 4: blue ring area = pi (20^2 - 16^2) = 452.39; the inside is yellow.
    let f = paint("<body style='margin:0'><div style='width:32px;height:32px;border:4px solid #00f;border-radius:20px;background:#ff0'></div>", 50, 50);
    let blue = {
        let mut s = 0.0;
        for y in 0..f.height {
            for x in 0..f.width {
                s += 1.0 - f32::from(at(&f, x, y)[2]) / 255.0;
            }
        }
        s
    };
    assert!((blue - 452.39).abs() < 3.0, "{blue}");
    expect(&f, &[(20, 20, YELLOW), (20, 1, BLUE), (1, 20, BLUE), (0, 0, WHITE), (20, 38, BLUE)]);
}

#[test]
fn sides_with_different_colors_and_widths_meet_on_the_diagonal() {
    // Top 10 px red, left 10 px blue on a 40x40 box: the corner square 0..10 is cut along its diagonal.
    let f = paint("<body style='margin:0'><div style='width:30px;height:30px;border:10px solid;border-top-color:#f00;border-left-color:#00f;border-right-color:#00f;border-bottom-color:#00f'></div>", 50, 50);
    expect(&f, &[(5, 1, RED), (1, 5, BLUE), (20, 5, RED), (5, 20, BLUE)]);
    near(&f, 2, 8, [255, 0, 0, 255], 0);
    near(&f, 8, 2, [0, 0, 255, 255], 0);
    // Thin top, thick sides: the top border's wedge is the trapezoid between the corner points.
    let f = paint("<body style='margin:0'><div style='width:20px;height:20px;border:solid;border-width:2px 10px;border-color:#f00 #00f'></div>", 50, 50);
    expect(&f, &[(20, 0, RED), (20, 1, RED), (20, 2, WHITE), (5, 20, BLUE), (1, 20, BLUE)]);
}

#[test]
fn overflow_hidden_clips_children_to_the_rounded_padding_edge() {
    let f = paint("<body style='margin:0'><div style='overflow:hidden;width:40px;height:40px;border-radius:20px'><div style='width:100px;height:100px;background:#f00'></div></div>", 60, 60);
    expect(&f, &[(20, 20, RED), (0, 0, WHITE), (39, 0, WHITE), (3, 3, WHITE), (6, 6, RED), (45, 20, WHITE), (20, 45, WHITE)]);
    assert!((area(&f, 1) - 1256.64).abs() < 2.0, "{}", area(&f, 1));
    // With a border the clip is the inner edge: radius 20 - 5 = 15 inside a 5 px border.
    let f = paint("<body style='margin:0'><div style='overflow:hidden;width:40px;height:40px;border:5px solid #00f;border-radius:25px'><div style='width:100px;height:100px;background:#f00'></div></div>", 60, 60);
    // The inner area is a circle of radius 20: red area pi 400 = 1256.64.
    let red = {
        let mut s = 0.0;
        for y in 0..f.height {
            for x in 0..f.width {
                let p = at(&f, x, y);
                s += (f32::from(p[0]) - f32::from(p[1]).min(f32::from(p[0]))).max(0.0) * 0.0 + if p[0] < 128 && p[2] > 128 { 1.0 } else { 0.0 };
            }
        }
        s
    };
    assert!((red - 1256.64).abs() < 40.0, "{red}");
    assert_eq!(at(&f, 25, 25), RED);
    assert_eq!(at(&f, 2, 25), BLUE);
}

#[test]
fn text_inside_a_rounded_clip_is_cut_at_the_corner() {
    // A text line of 1em test glyphs (full squares) in a clip with a big radius: the glyph at the corner is cut away.
    let f = paint("<body style='margin:0'><div style='overflow:hidden;width:30px;height:30px;border-radius:30px;font:20px/20px sans-serif'>MMMM</div>", 40, 40);
    assert_eq!(at(&f, 0, 0), WHITE);
    assert_eq!(at(&f, 15, 8), [0, 0, 0, 255]);
}

// ---- border styles and outlines -----------------------------------------------------------------------------------

#[test]
fn dashed_borders_fit_whole_dashes_into_each_side() {
    // Top 3 px dashed, 60 px long: dashes of 6 and gaps of 3 (2w and w for widths from 3 px).
    let f = paint("<body style='margin:0'><div style='width:60px;height:10px;border-top:3px dashed #00f'></div>", 80, 30);
    expect(&f, &[(2, 1, BLUE), (7, 1, WHITE), (11, 1, BLUE), (14, 1, BLUE), (16, 1, WHITE), (56, 1, BLUE), (59, 1, BLUE), (61, 1, WHITE), (11, 4, WHITE)]);
    // Dotted: round dots of the border width, one width apart.
    let f = paint("<body style='margin:0'><div style='width:60px;height:10px;border-top:4px dotted #00f'></div>", 80, 30);
    near(&f, 2, 2, BLUE, 2);
    expect(&f, &[(6, 2, WHITE)]);
}

#[test]
fn double_groove_ridge_inset_and_outset_borders() {
    // 9 px double: lines of 3 px at the outer and inner edge, a gap between.
    let f = paint("<body style='margin:0'><div style='width:20px;height:20px;border:9px double #00f'></div>", 60, 60);
    expect(&f, &[(1, 19, BLUE), (4, 19, WHITE), (7, 19, BLUE), (10, 19, WHITE), (19, 1, BLUE), (19, 4, WHITE), (19, 7, BLUE)]);
    // #808080: darkened to 43 and lightened to 212 (the way Chromium does it).
    let div = "width:20px;height:20px;border:6px groove #808080";
    let f = paint(&format!("<body style='margin:0'><div style='{div}'></div>"), 60, 60);
    near(&f, 1, 19, [43, 43, 43, 255], 1);
    near(&f, 4, 19, [212, 212, 212, 255], 1);
    near(&f, 31, 19, [212, 212, 212, 255], 1);
    near(&f, 28, 19, [43, 43, 43, 255], 1);
    let f = paint(&format!("<body style='margin:0'><div style='{}'></div>", div.replace("groove", "ridge")), 60, 60);
    near(&f, 1, 19, [212, 212, 212, 255], 1);
    near(&f, 4, 19, [43, 43, 43, 255], 1);
    let f = paint("<body style='margin:0'><div style='width:20px;height:20px;border:4px inset #808080'></div>", 60, 60);
    near(&f, 1, 14, [43, 43, 43, 255], 1);
    near(&f, 27, 14, [212, 212, 212, 255], 1);
    let f = paint("<body style='margin:0'><div style='width:20px;height:20px;border:4px outset #808080'></div>", 60, 60);
    near(&f, 1, 14, [212, 212, 212, 255], 1);
    near(&f, 27, 14, [43, 43, 43, 255], 1);
}

#[test]
fn an_outline_is_drawn_outside_the_box_without_taking_room() {
    // Border box 10..30; offset 2 and width 3: the ring is 5..8 and 32..35, drawn over what is next to it.
    let f = paint("<body style='margin:0'><div style='margin:10px 10px 0;width:20px;height:20px;background:#ff0;outline:3px solid #0f0;outline-offset:2px'></div><p style='margin:0;height:5px;background:#f00'></p>", 60, 60);
    expect(&f, &[(7, 20, GREEN), (8, 20, WHITE), (4, 20, WHITE), (20, 20, YELLOW), (31, 20, WHITE), (32, 20, GREEN), (34, 20, GREEN), (35, 20, WHITE), (20, 33, GREEN)]);
    // The paragraph starts where the box ends (y 30): the outline is on top of it and the box took no room for it.
    expect(&f, &[(20, 31, RED), (20, 33, GREEN), (20, 35, WHITE)]);
    // `auto` is a solid line; a negative offset draws inside the border box.
    let f = paint("<body style='margin:0'><div style='margin:10px;width:20px;height:20px;outline:2px auto #00f;outline-offset:-2px'></div>", 60, 60);
    expect(&f, &[(10, 20, BLUE), (11, 20, BLUE), (12, 20, WHITE), (9, 20, WHITE)]);
}

#[test]
fn hostile_values_do_not_panic() {
    let big = "background:linear-gradient(red 1e30px,blue -1e30px) 1e30px -1e30px/1e30px 1px repeat;border-radius:1e30px;border:1e9px dashed red;outline:1e9px dotted red;outline-offset:1e9px";
    let layers = (0..40).map(|_| "radial-gradient(circle 1e-30px at 1e30px 0,red,blue)").collect::<Vec<_>>().join(",");
    for html in [
        format!("<body style='margin:0'><div style='width:50px;height:50px;{big}'></div>"),
        format!("<body style='margin:0'><div style='width:50px;height:50px;background:{layers}'></div>"),
        "<body style='margin:0'><div style='width:50px;height:50px;background:url() 0 0/0 0 space round;background-size:0 1e9px'></div>".to_string(),
        "<body style='margin:0'><div style='width:1px;height:1px;background:repeating-linear-gradient(red 0,blue 0.0001px)'></div>".to_string(),
    ] {
        let page = lay(&html, 60, 60, 1.5, None, &ImageSizes::new());
        let _ = draw(&page, 60, 60, 1.5, 0.0, &mut NoBitmaps);
    }
}

#[test]
fn a_radius_clips_a_picture_to_the_rounded_edge() {
    // A 40x40 picture (all red) with border-radius 50% is a disc: 1256.6 px of red.
    let html = "<body style='margin:0'><img src='a.png' width='40' height='40' style='display:block;border-radius:50%'>";
    let first = lay(html, 50, 50, 1.0, None, &ImageSizes::new());
    let mut sizes = ImageSizes::new();
    sizes.set_loaded(first.res.images[0].node, 40, 40);
    let page = lay(html, 50, 50, 1.0, None, &sizes);
    let mut src = Gen(|_, _, _, _| RED, Vec::new());
    let f = draw(&page, 50, 50, 1.0, 0.0, &mut src);
    expect(&f, &[(20, 20, RED), (0, 0, WHITE), (39, 39, WHITE)]);
    assert!((area(&f, 1) - 1256.64).abs() < 2.0, "{}", area(&f, 1));
}

#[test]
fn a_collapsed_cell_places_its_background_from_its_halves_of_the_edges() {
    // One cell, 4px border all round, no padding, a 20px wide content, collapsed: the edge is 4px, the cell takes 2px of it on
    // every side (the table the other 2px), so the cell's box is x 2..26 and its padding box (the origin) x 4..24. The
    // tile is 20px wide, red up to its middle (x 14), blue after, and repeats: x 2..4 is the end of the tile before (blue).
    let html = "<body style='margin:0'><table style='border-collapse:collapse'><tr><td style='padding:0;border:4px solid transparent;width:20px;height:20px;\
        background:linear-gradient(to right,red 50%,blue 50%)'></td></tr></table>";
    let f = paint(html, 40, 40);
    expect(&f, &[(3, 10, BLUE), (6, 10, RED), (15, 10, BLUE), (23, 10, BLUE), (25, 10, RED), (27, 10, WHITE)]);
}

#[test]
fn a_table_with_direction_rtl_puts_its_first_column_on_the_right() {
    // Cells 30px (red) and 50px (blue) wide, no spacing: the table is 80px wide; in an rtl table the red cell is x 50..80.
    let html = "<body style='margin:0'><table style='direction:rtl;border-spacing:0'><tr><td style='padding:0;width:30px;height:10px;background:red'></td>\
        <td style='padding:0;width:50px;height:10px;background:blue'></td></tr></table>";
    let f = paint(html, 100, 20);
    expect(&f, &[(5, 5, BLUE), (49, 5, BLUE), (51, 5, RED), (79, 5, RED), (85, 5, WHITE)]);
}

/// The bitmap sizes asked for by an SVG background that leaves part of its natural size open (`partial`), in a 100x100 box.
fn partial_bg_sizes(size: &str, partial: Partial) -> Vec<(u32, u32)> {
    let html = format!("<body style='margin:0'><div style=\"width:100px;height:100px;background:url(a.svg) no-repeat;background-size:{size}\"></div>");
    let first = lay(&html, 120, 120, 1.0, None, &ImageSizes::new());
    let node = first.res.images[0].node;
    let mut sizes = ImageSizes::new();
    sizes.set_loaded(node, 300, 150);
    sizes.set_partial(node, Some(partial));
    let page = lay(&html, 120, 120, 1.0, None, &sizes);
    let mut src = Gen(|_, _, _, _| RED, Vec::new());
    let _ = draw(&page, 120, 120, 1.0, 0.0, &mut src);
    src.1
}

#[test]
fn a_background_svg_with_a_ratio_but_no_size_is_contained_in_the_positioning_area() {
    // css-images-3 5.3: a ratio alone is not 300 x 150; with `auto` the image is the largest of its ratio that fits the box.
    let ratio2 = Partial { w: None, h: None, ratio: Some(2.0) };
    assert_eq!(partial_bg_sizes("auto", ratio2), [(100, 50)]);
    assert_eq!(partial_bg_sizes("contain", ratio2), [(100, 50)]);
    assert_eq!(partial_bg_sizes("cover", ratio2), [(200, 100)]);
    // One side given by the size property, the other follows the ratio.
    assert_eq!(partial_bg_sizes("40px auto", ratio2), [(40, 20)]);
    assert_eq!(partial_bg_sizes("auto 30px", ratio2), [(60, 30)]);
    // One side given by the file and a ratio: the other follows it.
    assert_eq!(partial_bg_sizes("auto", Partial { w: Some(40.0), h: None, ratio: Some(2.0) }), [(40, 20)]);
    // One side given by the file and no ratio: the other is the box's.
    assert_eq!(partial_bg_sizes("auto", Partial { w: Some(40.0), h: None, ratio: None }), [(40, 100)]);
    assert_eq!(partial_bg_sizes("auto", Partial { w: None, h: Some(30.0), ratio: None }), [(100, 30)]);
    // Nothing at all: the box.
    assert_eq!(partial_bg_sizes("auto", Partial { w: None, h: None, ratio: None }), [(100, 100)]);
}
