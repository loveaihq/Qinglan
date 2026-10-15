//! `qinglan-cli render`: load a page, lay it out, paint it with the engine's own rasterizer and write a PNG.
//!
//! The steps are the ones the window will take (step 1c-2b): the sheets are in, then style, a layout that stops one
//! screen below the viewport (first-screen-first), the display list and the first frame without images (`--time`
//! reports this "content ready -> first-screen pixels" time), then the images that are near (or all of them with
//! `--full`), the full layout, the decoded bitmaps and the final frame.

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use qinglan_engine::css::{StyleOptions, style_document_with};
use qinglan_engine::layout::image::ImageSizes;
use qinglan_engine::layout::{LayoutOptions, LayoutResult, layout_document_with};
use qinglan_engine::net::blocking::Opened;
use qinglan_engine::net::loader::Loader;
use qinglan_engine::net::page::{PageResources, Update, apply_probe, probe_response};
use qinglan_engine::paint::images::{Band, ImageStore, NoBitmaps};
use qinglan_engine::paint::raster::{Frame, View, render_frame};
use qinglan_engine::paint::{DisplayList, build_display_list, wanted_images};
use qinglan_engine::platform::Platform;
use qinglan_win::{WicDecoder, WinPlatform};

pub const IMAGE_TIMEOUT: Duration = Duration::from_secs(60);
/// Rounds of "request the images that are near, lay out again with their sizes" before giving up on a page
/// whose layout keeps moving images in and out of range.
pub const MAX_ROUNDS: usize = 4;
/// `--full` pages are cut so that the frame stays under this many device pixels (128 MB of BGRA).
const MAX_FRAME_PIXELS: u64 = 32_000_000;
/// The scroll benchmark moves the page by this many CSS px per frame.
const SCROLL_STEP: f32 = 100.0;

pub struct RenderArgs {
    pub out: PathBuf,
    pub full: bool,
    pub dpr: f32,
    pub scroll: f32,
    pub width: f32,
    pub height: f32,
    pub timing: bool,
    pub scroll_bench: bool,
    /// `--time`: stop the first layout one screen below the viewport (the default; off to see what it saves).
    pub first_screen_first: bool,
}

pub struct Ctx<'a> {
    pub opened: &'a mut Opened,
    pub res: &'a mut PageResources,
    pub loader: &'a mut Loader,
    pub platform: &'a dyn Platform,
    pub win: Option<&'a WinPlatform>,
}

pub fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

/// The images the page wants that are near `band` (or all of them): request, wait, record their sizes. Returns
/// whether any size changed (the page must be laid out again).
pub fn fetch_images(cx: &mut Ctx<'_>, layout: &LayoutResult, list: &DisplayList, band: Band, all: bool, sizes: &mut ImageSizes, store: &mut ImageStore) -> bool {
    let wanted = wanted_images(layout, list, band, all);
    let mut changed = false;
    for n in cx.res.request_images(cx.loader, &wanted) {
        changed |= sizes.set_broken(n);
    }
    let deadline = Instant::now() + IMAGE_TIMEOUT;
    while cx.res.images_pending() > 0 {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            break;
        }
        let events = cx.loader.wait(left);
        let stalled = events.is_empty() && cx.loader.pending() == 0;
        for ev in events {
            let Some(Update::Image { nodes, url, result }) = cx.res.handle(cx.loader, ev, cx.platform) else { continue };
            match result {
                Ok(resp) => {
                    let probe = probe_response(&WicDecoder, &resp);
                    changed |= apply_probe(sizes, &nodes, &probe);
                    if probe.is_ok() {
                        for n in &nodes {
                            store.set_url(*n, &url.serialize(true));
                        }
                    }
                }
                Err(_) => {
                    for n in nodes {
                        changed |= sizes.set_broken(n);
                    }
                }
            }
        }
        if stalled {
            break;
        }
    }
    changed
}

pub fn run(mut cx: Ctx<'_>, a: &RenderArgs) -> ExitCode {
    let dpr = a.dpr;
    let doc_url = cx.opened.url.clone();
    let sopts = StyleOptions { viewport_w: a.width, viewport_h: a.height, ..StyleOptions::default() };
    let lopts = LayoutOptions { viewport_w: a.width, viewport_h: a.height, dpr, ..LayoutOptions::default() };
    let (frame_w, view_h) = ((a.width * dpr).round() as u32, (a.height * dpr).round() as u32);
    if frame_w == 0 || view_h == 0 || u64::from(frame_w) * u64::from(view_h) > MAX_FRAME_PIXELS {
        eprintln!("error: a picture of {frame_w}x{view_h} pixels is not made (at most {MAX_FRAME_PIXELS} pixels)");
        return ExitCode::FAILURE;
    }

    // Content is ready. Style, then (for --time) the first screen exactly as the window will paint it.
    let t_ready = Instant::now();
    let styled = style_document_with(&mut cx.opened.doc, Some(&doc_url), cx.res, &sopts, Some(cx.platform));
    let d_style = t_ready.elapsed();
    for m in &styled.missing {
        eprintln!("warning: style sheet not found: {m}");
    }
    if a.timing {
        let t = Instant::now();
        let stop = a.first_screen_first.then_some(a.scroll + 2.0 * a.height);
        let first = layout_document_with(&cx.opened.doc, &styled.tree, cx.platform, &LayoutOptions { stop_y: stop, ..lopts }, &ImageSizes::new());
        let d_layout = t.elapsed();
        let t = Instant::now();
        let list = build_display_list(&cx.opened.doc, &first, cx.platform);
        let d_list = t.elapsed();
        let t = Instant::now();
        let mut frame = Frame::new(frame_w, view_h);
        let stats = render_frame(&list, cx.platform, &mut NoBitmaps, View { scroll_y: a.scroll, dpr }, &mut frame);
        let d_raster = t.elapsed();
        let total = t_ready.elapsed();
        println!("first-screen-ms: {:.1}", ms(total));
        println!(
            "first screen (content ready -> pixels in the buffer): {:.1} ms = style {:.1} + layout {:.1} + display list {:.1} + raster {:.1}; partial: {} ({} lines, {} items, {} glyphs drawn)",
            ms(total),
            ms(d_style),
            ms(d_layout),
            ms(d_list),
            ms(d_raster),
            first.partial,
            first.lines.len(),
            list.items.len(),
            stats.glyphs
        );
    }

    // The whole page, with the images that are near (or all, for --full).
    let t = Instant::now();
    let mut sizes = ImageSizes::new();
    let mut store = ImageStore::new();
    let band = Band::new(a.scroll, a.height);
    let mut round = 0;
    let (mut d_lay, mut d_list_full) = (Duration::ZERO, Duration::ZERO);
    let (layout, list) = loop {
        let t1 = Instant::now();
        let layout = layout_document_with(&cx.opened.doc, &styled.tree, cx.platform, &lopts, &sizes);
        let t2 = Instant::now();
        let list = build_display_list(&cx.opened.doc, &layout, cx.platform);
        d_lay += t2 - t1;
        d_list_full += t2.elapsed();
        if round >= MAX_ROUNDS {
            break (layout, list);
        }
        let changed = fetch_images(&mut cx, &layout, &list, band, a.full, &mut sizes, &mut store);
        round += 1;
        if !changed {
            break (layout, list);
        }
    };
    let d_full = t.elapsed();
    if layout.incomplete {
        eprintln!("note: page too complex, only part of it is laid out");
    }
    if cx.opened.doc.too_large {
        eprintln!("note: page too large, only the first part was parsed");
    }

    // The frame: the viewport, or with --full the whole page (cut to a sane size).
    let (frame_h, scroll) = if a.full {
        let page_h = list.height.to_f32().ceil().max(a.height);
        let mut h = (page_h * dpr).round() as u64;
        if u64::from(frame_w) * h > MAX_FRAME_PIXELS {
            h = MAX_FRAME_PIXELS / u64::from(frame_w).max(1);
            eprintln!("note: the page is {page_h} px high; the picture is cut at {} px", h as f32 / dpr);
        }
        (h as u32, 0.0)
    } else {
        (view_h, a.scroll)
    };
    let t = Instant::now();
    let dec_band = if a.full { Band::new(0.0, (frame_h as f32 / dpr).max(1.0)) } else { Band::new(scroll, a.height) };
    let prepared = store.prepare(&list, dec_band, dpr, &WicDecoder, &mut |u| cx.loader.cache_mut().get(u));
    let d_decode = t.elapsed();
    let t = Instant::now();
    let mut frame = Frame::new(frame_w, frame_h);
    let stats = render_frame(&list, cx.platform, &mut store, View { scroll_y: scroll, dpr }, &mut frame);
    let d_raster = t.elapsed();
    let t = Instant::now();
    if let Err(e) = qinglan_win::write_png(&a.out, frame.width, frame.height, &frame.data) {
        eprintln!("error: {e}");
        return ExitCode::FAILURE;
    }
    let d_png = t.elapsed();
    println!(
        "wrote {} ({}x{} px, dpr {dpr}, scroll {scroll}): {} images in the page, {} drawn, {} bitmaps decoded ({} KB)",
        a.out.display(),
        frame.width,
        frame.height,
        layout.images.len(),
        stats.images,
        prepared.decoded,
        store.bytes() / 1024
    );
    if a.timing {
        println!(
            "full page: layout, images and display list {:.1} ms ({} rounds; layouts {:.1} ms, display lists {:.1} ms), decode {:.1} ms, raster {:.1} ms, png {:.1} ms; {} lines, {} items, page {} px high",
            ms(d_full),
            round,
            ms(d_lay),
            ms(d_list_full),
            ms(d_decode),
            ms(d_raster),
            ms(d_png),
            layout.lines.len(),
            list.items.len() + list.fixed.len(),
            list.height.to_f32()
        );
        if let Some(w) = cx.win {
            println!("{}", w.cache_stats().report());
        }
    }
    if a.scroll_bench {
        scroll_bench(cx.platform, cx.loader, &list, &mut store, a);
    }
    ExitCode::SUCCESS
}

/// Raster time per frame while scrolling from the top to the bottom in [`SCROLL_STEP`] px steps. Pass 1 meets
/// every glyph for the first time (the platform's mask cache fills as it goes), pass 2 is the same walk warm.
/// Decoding the images that come near is done before each frame and is not counted.
fn scroll_bench(platform: &dyn Platform, loader: &mut Loader, list: &DisplayList, store: &mut ImageStore, a: &RenderArgs) {
    let dpr = a.dpr;
    let mut frame = Frame::new((a.width * dpr).round() as u32, (a.height * dpr).round() as u32);
    let last = (list.height.to_f32() - a.height).max(0.0);
    for pass in 1..=2 {
        let mut times: Vec<f64> = Vec::new();
        let (mut items, mut glyphs) = (0usize, 0usize);
        let mut y = 0.0f32;
        loop {
            let band = Band::new(y, a.height);
            store.prepare(list, band, dpr, &WicDecoder, &mut |u| loader.cache_mut().get(u));
            store.drop_far(list, band);
            let t = Instant::now();
            let s = render_frame(list, platform, store, View { scroll_y: y, dpr }, &mut frame);
            times.push(ms(t.elapsed()));
            items += s.items;
            glyphs += s.glyphs;
            if y >= last {
                break;
            }
            y = (y + SCROLL_STEP).min(last);
        }
        let mut sorted = times.clone();
        sorted.sort_by(f64::total_cmp);
        let at = |q: f64| sorted.get(((sorted.len() as f64 - 1.0) * q).round() as usize).copied().unwrap_or(0.0);
        println!(
            "scroll raster dpr {dpr} pass {pass} ({}): {} frames, median {:.2} ms, p95 {:.2} ms, max {:.2} ms; per frame {} items, {} glyphs",
            if pass == 1 { "first sight of every glyph" } else { "warm" },
            times.len(),
            at(0.5),
            at(0.95),
            sorted.last().copied().unwrap_or(0.0),
            items / times.len().max(1),
            glyphs / times.len().max(1)
        );
    }
}
