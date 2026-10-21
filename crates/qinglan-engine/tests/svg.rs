//! SVG (step 2e-1): standalone files rasterized through the decoder interface, and inline `<svg>` in a page. The numbers
//! are worked out by hand in the comments.
#![allow(clippy::indexing_slicing, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use qinglan_engine::css::{NoResources, StyleOptions, style_document_with};
use qinglan_engine::html::treebuilder::parse_document;
use qinglan_engine::layout::{ImageSizes, LayoutOptions, layout_document_with};
use qinglan_engine::paint::build_display_list;
use qinglan_engine::paint::images::NoBitmaps;
use qinglan_engine::paint::raster::{Frame, View, render_frame};
use qinglan_engine::platform::image::{DecodedImage, ImageError};
use qinglan_engine::platform::{Platform, TestPlatform};
use qinglan_engine::svg;

const NS: &str = "xmlns='http://www.w3.org/2000/svg'";

fn raster(body: &str, w: u32, h: u32) -> DecodedImage {
    svg::rasterize(format!("<svg {NS} {body}</svg>").as_bytes(), w, h).unwrap()
}

fn px(i: &DecodedImage, x: u32, y: u32) -> [u8; 4] {
    let at = ((y * i.width + x) * 4) as usize;
    i.bgra[at..at + 4].try_into().unwrap()
}

fn coverage(i: &DecodedImage) -> f32 {
    i.bgra.as_chunks::<4>().0.iter().map(|p| f32::from(p[3])).sum::<f32>() / 255.0
}

const RED: [u8; 4] = [0, 0, 255, 255];
const CLEAR: [u8; 4] = [0, 0, 0, 0];
const WHITE: [u8; 4] = [255, 255, 255, 255];

#[test]
fn a_rect_covers_exactly_its_pixels() {
    let i = raster("width='10' height='10'><rect x='2' y='2' width='4' height='4' fill='#f00'/>", 10, 10);
    assert_eq!((px(&i, 2, 2), px(&i, 5, 5), px(&i, 6, 6), px(&i, 1, 1)), (RED, RED, CLEAR, CLEAR));
    // Half-pixel edges: x 2.5..6.5 covers the first and last column halfway.
    let i = raster("width='10' height='10'><rect x='2.5' y='2' width='4' height='4' fill='#f00'/>", 10, 10);
    assert_eq!((px(&i, 2, 3)[3], px(&i, 3, 3)[3], px(&i, 6, 3)[3], px(&i, 7, 3)[3]), (128, 255, 128, 0));
}

#[test]
fn the_view_box_is_mapped_by_preserve_aspect_ratio() {
    // 20 x 10 viewport, viewBox 10 x 10: scale 1, centered (xMidYMid meet): the box covers x 5..15.
    let body = "width='20' height='10' viewBox='0 0 10 10'><rect width='10' height='10' fill='#f00'/>";
    let i = raster(body, 20, 10);
    assert_eq!((px(&i, 4, 5), px(&i, 5, 5), px(&i, 14, 5), px(&i, 15, 5)), (CLEAR, RED, RED, CLEAR));
    // xMinYMid slice: scale 2 (20 / 10), so the 10-unit height overflows (cropped); x 0..20 all covered.
    let i = raster(&body.replace("viewBox", "preserveAspectRatio='xMinYMid slice' viewBox"), 20, 10);
    assert_eq!((px(&i, 0, 0), px(&i, 19, 9)), (RED, RED));
    // none: stretched.
    let i = raster(&body.replace("viewBox", "preserveAspectRatio='none' viewBox"), 40, 10);
    assert_eq!((px(&i, 0, 5), px(&i, 39, 5)), (RED, RED));
    // The image is drawn at the size asked for: the same viewBox into 40 x 20 is a 2x scale (box x 10..30).
    let i = raster(body, 40, 20);
    assert_eq!((px(&i, 9, 10), px(&i, 10, 10), px(&i, 29, 10), px(&i, 30, 10)), (CLEAR, RED, RED, CLEAR));
}

#[test]
fn circles_strokes_and_dashes() {
    let i = raster("width='40' height='40'><circle cx='20' cy='20' r='10' fill='#000'/>", 40, 40);
    assert!((coverage(&i) - 314.16).abs() < 2.0, "{}", coverage(&i));
    // A 2 px line along y = 5: rows 4 and 5 are covered over x 0..10, nothing else.
    let i = raster("width='10' height='10'><line x1='0' y1='5' x2='10' y2='5' stroke='#f00' stroke-width='2'/>", 10, 10);
    assert_eq!((px(&i, 0, 4), px(&i, 9, 5), px(&i, 5, 3), px(&i, 5, 6)), (RED, RED, CLEAR, CLEAR));
    // Dashes 2 on, 2 off: pixels 0-1 and 4-5 are on, 2-3 and 6-7 are off.
    let i = raster("width='10' height='10'><line x1='0' y1='5' x2='10' y2='5' stroke='#f00' stroke-width='2' stroke-dasharray='2 2'/>", 10, 10);
    assert_eq!((px(&i, 1, 5), px(&i, 2, 5), px(&i, 4, 5), px(&i, 6, 5)), (RED, CLEAR, RED, CLEAR));
    // Square caps stick out by half the width.
    let i = raster("width='20' height='10'><line x1='5' y1='5' x2='15' y2='5' stroke='#f00' stroke-width='4' stroke-linecap='square'/>", 20, 10);
    assert_eq!((px(&i, 3, 5), px(&i, 2, 5), px(&i, 16, 5), px(&i, 17, 5)), (RED, CLEAR, RED, CLEAR));
}

#[test]
fn gradients_interpolate_and_inherit() {
    let defs = "<defs><linearGradient id='g'><stop offset='0' stop-color='#000'/><stop offset='1' stop-color='#fff'/></linearGradient>\
                <linearGradient id='h' href='#g' x1='1' x2='0'/></defs>";
    let i = raster(&format!("width='100' height='10'>{defs}<rect width='100' height='10' fill='url(#g)'/>"), 100, 10);
    // Pixel centers: t = (x + .5) / 100, so x = 49 is 0.495 -> 126, and the ends are 0.005 -> 1 and 0.995 -> 254.
    assert_eq!((px(&i, 49, 5)[2], px(&i, 0, 5)[2], px(&i, 99, 5)[2]), (126, 1, 254));
    // `h` takes its stops from `g` and runs the other way.
    let i = raster(&format!("width='100' height='10'>{defs}<rect width='100' height='10' fill='url(#h)'/>"), 100, 10);
    assert_eq!((px(&i, 0, 5)[2], px(&i, 99, 5)[2]), (254, 1));
    // A missing paint server with a fallback uses the fallback; without one, nothing is painted.
    let i = raster("width='10' height='10'><rect width='10' height='10' fill='url(#nope) #f00'/>", 10, 10);
    assert_eq!(px(&i, 5, 5), RED);
    let i = raster("width='10' height='10'><rect width='10' height='10' fill='url(#nope)'/>", 10, 10);
    assert_eq!(px(&i, 5, 5), CLEAR);
}

#[test]
fn style_precedence_inheritance_and_current_color() {
    let doc = |style: &str, g: &str, r: &str| format!("width='10' height='10'><style>{style}</style><g {g}><rect width='10' height='10' {r}/></g>");
    // Presentation attribute < author sheet < style attribute.
    let i = raster(&doc("", "", "fill='#f00'"), 10, 10);
    assert_eq!(px(&i, 5, 5), RED);
    let i = raster(&doc("rect{fill:#0f0}", "", "fill='#f00'"), 10, 10);
    assert_eq!(px(&i, 5, 5), [0, 255, 0, 255]);
    let i = raster(&doc("rect{fill:#0f0}", "", "fill='#f00' style='fill:#00f'"), 10, 10);
    assert_eq!(px(&i, 5, 5), [255, 0, 0, 255]);
    // Inherited from the group; currentColor takes the `color` of the element.
    let i = raster(&doc("", "fill='#f00'", ""), 10, 10);
    assert_eq!(px(&i, 5, 5), RED);
    let i = raster(&doc("", "color='#f00'", "fill='currentColor'"), 10, 10);
    assert_eq!(px(&i, 5, 5), RED);
    // `fill-opacity`, and display none.
    let i = raster(&doc("", "", "fill='#f00' fill-opacity='.5'"), 10, 10);
    assert_eq!(px(&i, 5, 5), [0, 0, 128, 128]);
    let i = raster(&doc("rect{display:none}", "", "fill='#f00'"), 10, 10);
    assert_eq!(px(&i, 5, 5), CLEAR);
}

#[test]
fn opacity_groups_composite_as_one() {
    // Two overlapping opaque rects in a group at .5: where they overlap the top one shows at 50%, not both.
    let i = raster(
        "width='10' height='10'><g opacity='.5'><rect width='6' height='10' fill='#f00'/><rect x='4' width='6' height='10' fill='#00f'/></g>",
        10,
        10,
    );
    assert_eq!((px(&i, 1, 5), px(&i, 5, 5), px(&i, 8, 5)), ([0, 0, 128, 128], [128, 0, 0, 128], [128, 0, 0, 128]));
}

#[test]
fn use_symbol_clip_path_and_transform() {
    let defs = "<defs><rect id='r' width='4' height='4' fill='#f00'/><symbol id='s' viewBox='0 0 2 2'><rect width='2' height='2' fill='#f00'/></symbol>\
                <clipPath id='c'><rect x='0' y='0' width='5' height='10'/></clipPath></defs>";
    // use with x/y; a symbol is scaled into its width/height (2x2 viewBox into 6x6: 3x).
    let i = raster(&format!("width='20' height='10'>{defs}<use href='#r' x='1' y='1'/><use href='#s' x='10' y='2' width='6' height='6'/>"), 20, 10);
    assert_eq!((px(&i, 1, 1), px(&i, 4, 4), px(&i, 5, 5), px(&i, 10, 2), px(&i, 15, 7), px(&i, 16, 7)), (RED, RED, CLEAR, RED, RED, CLEAR));
    // The clip cuts the rect at x = 5.
    let i = raster(&format!("width='10' height='10'>{defs}<rect width='10' height='10' fill='#f00' clip-path='url(#c)'/>"), 10, 10);
    assert_eq!((px(&i, 4, 5), px(&i, 5, 5)), (RED, CLEAR));
    // transform: translate then scale (applied right to left).
    let i = raster("width='10' height='10'><rect width='2' height='2' fill='#f00' transform='translate(4 4) scale(2)'/>", 10, 10);
    assert_eq!((px(&i, 3, 3), px(&i, 4, 4), px(&i, 7, 7), px(&i, 8, 8)), (CLEAR, RED, RED, CLEAR));
    // The CSS transform property works on SVG elements, about 0 0 by default.
    let i = raster("width='10' height='10'><rect width='2' height='2' fill='#f00' style='transform: translate(4px, 4px) scale(2)'/>", 10, 10);
    assert_eq!((px(&i, 3, 3), px(&i, 4, 4), px(&i, 7, 7), px(&i, 8, 8)), (CLEAR, RED, RED, CLEAR));
}

#[test]
fn paths_render_up_to_the_first_error() {
    let i = raster("width='10' height='10'><path d='M2 2 h6 v6 h-6 z L x' fill='#f00'/>", 10, 10);
    assert_eq!((px(&i, 5, 5), px(&i, 1, 1)), (RED, CLEAR));
    // An arc: a half disc of radius 5 under a chord from (0, 5) to (10, 5): area 39.27.
    let i = raster("width='10' height='10'><path d='M0 5 A5 5 0 0 0 10 5 Z' fill='#000'/>", 10, 10);
    assert!((coverage(&i) - 39.27).abs() < 0.6, "{}", coverage(&i));
    // Even-odd leaves the inner square empty; non-zero fills it.
    let d = "M0 0H10V10H0ZM3 3H7V7H3Z";
    let i = raster(&format!("width='10' height='10'><path d='{d}' fill='#f00' fill-rule='evenodd'/>"), 10, 10);
    assert_eq!((px(&i, 5, 5), px(&i, 1, 1)), (CLEAR, RED));
    let i = raster(&format!("width='10' height='10'><path d='{d}' fill='#f00'/>"), 10, 10);
    assert_eq!(px(&i, 5, 5), RED);
}

#[test]
fn sizes_of_standalone_files() {
    let probe = |s: &str| svg::probe(format!("<svg {NS} {s}/>").as_bytes());
    assert_eq!(probe("width='20' height='10'"), Ok((20, 10)));
    assert_eq!(probe("width='20' viewBox='0 0 4 2'"), Ok((20, 10)));
    assert_eq!(probe("viewBox='0 0 1 1'"), Ok((150, 150)));
    assert_eq!(probe("viewBox='0 0 4 1'"), Ok((300, 75)));
    assert_eq!(probe("width='50%' height='1in'"), Ok((300, 96)));
    assert_eq!(probe(""), Ok((300, 150)));
    assert_eq!(svg::probe(b"<svg><g></svg>"), Err(ImageError::Corrupt));
    assert_eq!(svg::probe(b"<html/>"), Err(ImageError::Corrupt));
}

fn page(html: &str, vw: u32, vh: u32) -> Frame {
    let mut doc = parse_document(html.to_string());
    let sopts = StyleOptions { viewport_w: vw as f32, viewport_h: vh as f32, ..StyleOptions::default() };
    let styled = style_document_with(&mut doc, None, &mut NoResources, &sopts, Some(&TestPlatform as &dyn Platform));
    let lopts = LayoutOptions { viewport_w: vw as f32, viewport_h: vh as f32, ..LayoutOptions::default() };
    let res = layout_document_with(&doc, &styled.tree, &TestPlatform, &lopts, &ImageSizes::new());
    let list = build_display_list(&doc, &res, &TestPlatform);
    let mut frame = Frame::new(vw, vh);
    render_frame(&list, &TestPlatform, &mut NoBitmaps, View { scroll_x: 0.0, scroll_y: 0.0, dpr: 1.0 }, &mut frame);
    frame
}

#[test]
fn inline_svg_is_a_replaced_element_styled_by_the_page() {
    let html = "<body style='margin:0'><style>.i{fill:#f00} svg{display:block}</style>\
                <svg width='20' height='10' viewBox='0 0 2 1'><rect class='i' width='1' height='1'/></svg>\
                <svg style='width:10px;height:10px' viewBox='0 0 1 1'><rect width='1' height='1' fill='#00f'/></svg>";
    let f = page(html, 40, 40);
    // The first svg is 20 x 10: the rect covers its left half (scale 10): x 0..10, y 0..10.
    assert_eq!((f.pixel(5, 5), f.pixel(15, 5), f.pixel(5, 12)), (Some(RED), Some(WHITE), Some([255, 0, 0, 255])));
    // The second is below it (y 10..20), sized by CSS, filled blue.
    assert_eq!((f.pixel(5, 15), f.pixel(15, 15)), (Some([255, 0, 0, 255]), Some(WHITE)));
}

#[test]
fn an_svg_without_size_is_300_by_150_and_a_ratio_gives_the_height() {
    let f = page("<body style='margin:0'><svg style='display:block'><rect width='500' height='500' fill='#f00'/></svg>", 400, 200);
    assert_eq!((f.pixel(299, 149), f.pixel(300, 100), f.pixel(100, 150)), (Some(RED), Some(WHITE), Some(WHITE)));
    let f = page("<body style='margin:0'><svg width='100' viewBox='0 0 2 1' style='display:block'><rect width='2' height='1' fill='#f00'/></svg>", 200, 100);
    assert_eq!((f.pixel(99, 49), f.pixel(99, 50)), (Some(RED), Some(WHITE)));
}

#[test]
fn links_inside_an_inline_svg_are_found_by_their_box() {
    use qinglan_engine::interact::PageIndex;
    let html = "<body style='margin:0'><svg width='40' height='20' viewBox='0 0 4 2'><a href='/x'><rect x='1' width='1' height='2'/></a></svg><a href='/y'><svg width='10' height='10'></svg></a>";
    let mut doc = parse_document(html.to_string());
    let sopts = StyleOptions { viewport_w: 100.0, viewport_h: 100.0, ..StyleOptions::default() };
    let styled = style_document_with(&mut doc, None, &mut NoResources, &sopts, Some(&TestPlatform as &dyn Platform));
    let res = layout_document_with(&doc, &styled.tree, &TestPlatform, &LayoutOptions { viewport_w: 100.0, viewport_h: 100.0, ..LayoutOptions::default() }, &ImageSizes::new());
    let index = PageIndex::new(&res);
    let href = |x: f32, y: f32| index.link_at(&doc, &res, x, y, y).and_then(|n| doc.attr(n, doc.atoms.lookup("href").unwrap()).map(str::to_string));
    // The rect covers x 10..20 (scale 10) of the 40 x 20 box; the second svg (x 40..50) sits inside an HTML `a`.
    assert_eq!((href(15.0, 5.0), href(5.0, 5.0), href(30.0, 5.0), href(45.0, 15.0)), (Some("/x".to_string()), None, None, Some("/y".to_string())));
}
