//! `qinglan-cli bench <url> --replay <dir>`: the phase timings and memory of `qinglan.exe --bench`, without a window.
//!
//! Same steps and the same clock rules as the window (see `crates/qinglan/src/bench.rs`): the clock starts when the
//! HTML is in hand (the replay's answer, before it is decoded and parsed), so parsing is counted; waiting for the
//! sheets is not. There is no blit; `first-screen-ms` ends when the first screen's pixels are in the buffer.

use std::process::ExitCode;
use std::time::{Duration, Instant};

use qinglan_engine::css::{StyleOptions, style_document_with};
use qinglan_engine::layout::image::ImageSizes;
use qinglan_engine::layout::{LayoutOptions, layout_document_with};
use qinglan_engine::net::blocking::{self, OpenResult};
use qinglan_engine::net::loader::{LoadRequest, Loader};
use qinglan_engine::net::{ResourceKind, Response};
use qinglan_engine::paint::images::{Band, ImageStore, NoBitmaps};
use qinglan_engine::paint::raster::{Frame, View, render_frame};
use qinglan_engine::paint::build_display_list;
use qinglan_engine::platform::Platform;
use qinglan_engine::url::Url;
use qinglan_win::WicDecoder;
#[cfg(feature = "mem-stats")]
use qinglan_win::memstats::{Breakdown, dom_parts, layout_parts};

use crate::render::{self, MAX_ROUNDS, ms};

const OPEN_TIMEOUT: Duration = Duration::from_secs(70);
const SHEET_TIMEOUT: Duration = Duration::from_secs(30);
const SCROLL_STEP: f32 = 100.0;

/// Scroll frames at most (page height / 100 px): the longest real page is a few thousand frames.
pub const DEFAULT_MAX_FRAMES: usize = 20_000;

pub struct BenchArgs {
    pub width: f32,
    pub height: f32,
    pub dpr: f32,
    pub max_frames: usize,
}

pub struct Ctx<'a> {
    pub loader: &'a mut Loader,
    pub platform: &'a dyn Platform,
    pub win: Option<&'a qinglan_win::WinPlatform>,
}

fn memory() -> Option<(u64, u64)> {
    qinglan_win::window::process_memory()
}

fn mib(b: Option<(u64, u64)>) -> String {
    b.map_or_else(|| "n/a".to_string(), |(private, _)| format!("{:.1} MB", private as f64 / 1_048_576.0))
}

pub fn run(cx: Ctx<'_>, url: &Url, a: &BenchArgs) -> ExitCode {
    let (platform, loader) = (cx.platform, cx.loader);
    let dpr = a.dpr;
    let (frame_w, frame_h) = ((a.width * dpr).round() as u32, (a.height * dpr).round() as u32);
    println!("bench-address: {}", url.href());
    println!("bench-viewport: {}x{} css px, dpr {dpr} ({frame_w}x{frame_h} device px)", a.width, a.height);
    let mem_start = memory();

    // The page itself (the replay answers at once).
    let req = LoadRequest { url: url.clone(), kind: ResourceKind::Document, page: None, use_cache: false };
    let Ok(ticket) = loader.submit(req) else {
        eprintln!("error: the address is refused");
        return ExitCode::FAILURE;
    };
    let Some(ev) = blocking::wait_for(loader, ticket, OPEN_TIMEOUT) else {
        eprintln!("error: the page did not arrive");
        return ExitCode::FAILURE;
    };
    let resp: std::sync::Arc<Response> = match ev.result {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };
    let t_html = Instant::now();
    let mut opened = match blocking::open_response(&resp, platform) {
        OpenResult::Page(p) => p,
        OpenResult::Download { mime, .. } => {
            eprintln!("error: not a page ({mime})");
            return ExitCode::FAILURE;
        }
        OpenResult::Failed(why) => {
            eprintln!("error: {why}");
            return ExitCode::FAILURE;
        }
    };
    let d_parse = t_html.elapsed();
    let mem_parse = memory();

    // The sheets of <head>: their wait is the network's time, not counted.
    let sopts = StyleOptions { viewport_w: a.width, viewport_h: a.height, ..StyleOptions::default() };
    let lopts = LayoutOptions { viewport_w: a.width, viewport_h: a.height, dpr, ..LayoutOptions::default() };
    let mut res = blocking::resources_for(&opened);
    let t = Instant::now();
    res.start_sheets(&opened.doc, loader, &sopts);
    let d_start = t.elapsed();
    let t_wait = Instant::now();
    let all_in = blocking::load_sheets(loader, &mut res, platform, SHEET_TIMEOUT);
    let d_sheets_wall = t_wait.elapsed();

    // First screen: style, layout to one screen below the viewport, display list, raster.
    let doc_url = opened.url.clone();
    let t = Instant::now();
    let styled = style_document_with(&mut opened.doc, Some(&doc_url), &mut res, &sopts, Some(platform));
    let d_style = t.elapsed();
    let t = Instant::now();
    let first = layout_document_with(&opened.doc, &styled.tree, platform, &LayoutOptions { stop_y: Some(2.0 * a.height), ..lopts }, &ImageSizes::new());
    let d_layout = t.elapsed();
    let t = Instant::now();
    let list = build_display_list(&opened.doc, &first, platform);
    let d_list = t.elapsed();
    let t = Instant::now();
    let mut frame = Frame::new(frame_w, frame_h);
    render_frame(&list, platform, &mut NoBitmaps, View { scroll_x: 0.0, scroll_y: 0.0, dpr }, &mut frame);
    let d_raster = t.elapsed();
    let first_screen = d_parse + d_start + d_style + d_layout + d_list + d_raster;
    let first_wall = t_html.elapsed();
    let mem_first = memory();
    drop((first, list));

    // The web fonts, which the window fetches after the first screen: decoded and registered here too, so that the hostile
    // font files of `tests/hostile.rs` reach the decoders (a page without `@font-face` does no work here).
    blocking::load_fonts(&mut *loader, &mut res, &opened.doc, &styled, platform, SHEET_TIMEOUT);
    let (faces, faces_ready, faces_failed) = res.fonts.counts();
    if faces > 0 {
        println!("web-fonts: {faces} declared, {faces_ready} loaded, {faces_failed} failed");
    }

    // The whole page, with every image (as the window's bench does), until no size changes.
    let mut sizes = ImageSizes::new();
    let mut store = ImageStore::new();
    let band = Band::new(0.0, a.height);
    let mut rounds = 0;
    let (mut d_full_first, mut full_wall, mut d_images) = (Duration::ZERO, Duration::ZERO, Duration::ZERO);
    let mut cx2 = render::Ctx { opened: &mut opened, res: &mut res, loader: &mut *loader, platform, win: cx.win };
    let (layout, list) = loop {
        let t = Instant::now();
        let layout = layout_document_with(&cx2.opened.doc, &styled.tree, platform, &lopts, &sizes);
        let list = build_display_list(&cx2.opened.doc, &layout, platform);
        if rounds == 0 {
            // The window's "full layout done": the first whole-page layout, before any image is known.
            d_full_first = t.elapsed();
            full_wall = t_html.elapsed();
        }
        if rounds >= MAX_ROUNDS {
            break (layout, list);
        }
        let t = Instant::now();
        let changed = render::fetch_images(&mut cx2, &layout, &list, band, true, &mut sizes, &mut store);
        d_images += t.elapsed();
        rounds += 1;
        if !changed {
            break (layout, list);
        }
    };
    let mem_layout = memory();
    let full_layout = first_screen + d_full_first;

    // Scroll from the top to the bottom, 100 px a frame; raster only (no window to blit to).
    let mut frames: Vec<f64> = Vec::new();
    let mut decode = 0.0;
    let mut peak_decoded = 0;
    let last = list.max_progress(a.width, a.height);
    let mut y = 0.0f32;
    loop {
        let band = list.band(y, a.width, a.height);
        let t = Instant::now();
        store.prepare(&list, band, dpr, &WicDecoder, &mut |u| loader.cache_mut().get(u));
        store.drop_far(&list, band);
        peak_decoded = peak_decoded.max(store.bytes());
        decode += ms(t.elapsed());
        let t = Instant::now();
        let (scroll_x, scroll_y) = list.axis.offset(y);
        render_frame(&list, platform, &mut store, View { scroll_x, scroll_y, dpr }, &mut frame);
        frames.push(ms(t.elapsed()));
        if y >= last || frames.len() >= a.max_frames {
            break;
        }
        y = (y + SCROLL_STEP).min(last);
    }
    let mem_end = memory();
    frames.sort_by(f64::total_cmp);
    let q = |q: f64| frames.get(((frames.len() as f64 - 1.0) * q).round() as usize).copied().unwrap_or(0.0);

    if !all_in {
        eprintln!("warning: some style sheets did not arrive");
    }
    println!("parse-ms: {:.1}", ms(d_parse));
    println!("sheets-wait-ms: {:.1} (not counted)", ms(d_sheets_wall));
    println!("style-ms: {:.1}", ms(d_style));
    println!("first-layout-ms: {:.1}", ms(d_layout));
    println!("display-list-ms: {:.1}", ms(d_list));
    println!("raster-ms: {:.1}", ms(d_raster));
    println!("first-screen-ms: {:.1}", ms(first_screen));
    println!("first-screen-wall-ms: {:.1}", ms(first_wall));
    println!("full-layout-ms: {:.1}", ms(full_layout));
    println!("full-layout-wall-ms: {:.1}", ms(full_wall));
    println!("images-ms: {:.1} ({rounds} rounds)", ms(d_images));
    println!("scroll-frames: {}", frames.len());
    println!("scroll-complete: {}", y >= last);
    println!("scroll-median-ms: {:.1}", q(0.5));
    println!("scroll-p95-ms: {:.1}", q(0.95));
    println!("scroll-max-ms: {:.1}", frames.last().copied().unwrap_or(0.0));
    println!("scroll-decode-ms: {decode:.1}");
    println!("peak-private-bytes: {}", mem_end.map_or_else(|| "n/a".into(), |m| m.1.to_string()));
    println!("private-bytes-after-scroll: {}", mem_end.map_or_else(|| "n/a".into(), |m| m.0.to_string()));
    println!("private-after-parse: {}", mib(mem_parse));
    println!("private-after-first-screen: {}", mib(mem_first));
    println!("private-after-full-layout-and-images: {}", mib(mem_layout));
    println!("private-at-start: {}", mib(mem_start));
    if layout.incomplete {
        eprintln!("note: page too complex, only part of it is laid out");
    }
    // Dev builds only: what each structure holds (`--mem`).
    #[cfg(feature = "mem-stats")]
    if qinglan_win::heap_counting() {
        let (heap, heap_peak, allocs) = (qinglan_win::heap_live(), qinglan_win::heap_peak(), qinglan_win::heap_allocs());
        let mut bd = Breakdown::default();
        bd.note("heap-live", heap);
        bd.note("heap-peak", heap_peak);
        bd.note("allocations", allocs as usize);
        bd.note("live-blocks", qinglan_win::heap_blocks());
        bd.note("peak-decoded-images", peak_decoded);
        bd.note("response-cache-accounted", loader.cache().bytes());
        let (nodes, text, attrs) = dom_parts(&opened.doc);
        bd.note("dom-detail-node-array", nodes);
        bd.note("dom-detail-text", text);
        bd.note("dom-detail-attributes", attrs);
        bd.note("dom-node-count", opened.doc.nodes.len());
        for (name, count, bytes) in layout_parts(&layout) {
            bd.note(&format!("layout-detail-{name}"), bytes);
            bd.note(&format!("layout-count-{name}"), count);
        }
        bd.freed("display-list", || drop(list));
        bd.freed("box-and-fragment-tree", || drop(layout));
        bd.freed("computed-styles", || drop(styled));
        bd.freed("decoded-images", || drop(store));
        bd.freed("response-cache", || loader.cache_mut().clear());
        bd.freed("parsed-sheets-and-resources", || drop(res));
        if let Some(w) = cx.win {
            let stats = w.cache_stats();
            bd.note("shaping-cache-accounted", stats.shape_bytes);
            bd.note("glyph-cache-accounted", stats.glyph_bytes);
            bd.freed("shaping-cache", || w.clear_shape_cache());
            bd.freed("glyph-cache", || w.clear_glyph_cache());
            bd.freed("word-tables", || w.clear_word_caches());
        }
        bd.freed("dom", || drop(opened));
        bd.freed("frame-buffers", || drop(frame));
        print!("{}", bd.lines());
        println!("private-after-teardown: {}", mib(memory()));
    }
    ExitCode::SUCCESS
}
