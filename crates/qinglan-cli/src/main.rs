//! qinglan-cli: development and test tool (not shipped to users).
//!
//! Step 1a commands:
//!   qinglan-cli dom <file|url>                       print the DOM tree in the html5lib test format
//!   qinglan-cli style <file|url> [--width W] [--height H] [--time]
//!                                                    print the computed style of every element
//!
//! Step 1b commands:
//!   qinglan-cli layout <file|url> [--width W] [--height H] [--test-font] [--time]
//!                                                    print the box tree: position and size of every
//!                                                    block, the line boxes and the text fragments
//!
//! Step 1c-1 commands:
//!   qinglan-cli record <url> <dir>                   load a page once (style sheets, imports, images)
//!                                                    and save every response into <dir>
//!   ... --replay <dir>                               (dom, style, layout) answer only from a recording;
//!                                                    anything not in it is a 404, the network is never used
//!
//! An argument that starts with `http://` or `https://` is a page address, anything else a file. Style
//! sheets and images are loaded the way the browser will (policy, size caps, Referer, WinHTTP).
//!
//! Step 1c-2b command:
//!   qinglan-cli bench <url> --replay <dir> [--width W] [--height H] [--dpr D]
//!                                                    the phase timings and memory of the window's --bench, without
//!                                                    a window (for regression tests)
//!
//! Step 1c-2a command:
//!   qinglan-cli render <file|url> -o out.png [--width 1280] [--height 1000 | --full] [--dpr 1] [--scroll Y]
//!                    [--replay <dir>] [--test-font] [--time] [--scroll-bench] [--no-prewarm]
//!                    [--no-first-screen-first]
//!                                                    paint the page with the engine's rasterizer into a PNG
//!
//! `style` and `layout` use the Windows platform (DirectWrite fonts, legacy encodings) unless
//! `--test-font` asks for the test platform (every glyph a 1em square, no legacy encodings).
#![forbid(unsafe_code)]

mod bench;
mod render;

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::{Duration, Instant};

use qinglan_engine::css::{StyleOptions, dump::dump_styles, style_document_with};
use qinglan_engine::html::dump::dump_children;
use qinglan_engine::layout::FragKind;
use qinglan_engine::layout::image::ImageSizes;
use qinglan_engine::layout::{LayoutOptions, dump::dump_layout, layout_document, layout_document_with};
use qinglan_engine::net::blocking::{self, OpenResult};
use qinglan_engine::net::file::url_from_path;
use qinglan_engine::net::loader::Loader;
use qinglan_engine::net::replay::{Recorder, ReplayTransport};
use qinglan_engine::net::{Transport, USER_AGENT};
use qinglan_engine::platform::{BasicPlatform, Platform, TestPlatform};
use qinglan_engine::url::Url;
use qinglan_win::{WicDecoder, WinHttpTransport, WinPlatform};

/// How long the command-line tools wait: the page, then its style sheets, then its images.
const OPEN_TIMEOUT: Duration = Duration::from_secs(70);
const SHEET_TIMEOUT: Duration = Duration::from_secs(30);
const IMAGE_TIMEOUT: Duration = Duration::from_secs(60);

/// Glyphs drawn as the font's missing-glyph box (index 0) in the text fragments, and the first few
/// distinct characters they stand for.
fn count_notdef(res: &qinglan_engine::layout::LayoutResult) -> (usize, String) {
    let mut count = 0usize;
    let mut chars: Vec<char> = Vec::new();
    for f in &res.fragments {
        let (text, range, glyphs) = match f.kind {
            FragKind::Text { text, start, end, glyphs, .. } | FragKind::Marker { text, start, end, glyphs, .. } => (text, start as usize..end as usize, glyphs),
            _ => continue,
        };
        let Some(gs) = res.glyphs.get(glyphs.0 as usize..glyphs.1 as usize) else { continue };
        // Layout keeps no cluster ranges: the character is taken to be the one at the glyph's position in the fragment
        // (exact unless the text has multi-character clusters before it).
        let frag_text = res.texts.get(text as usize).and_then(|t| t.get(range.clone()));
        for (k, _) in gs.iter().enumerate().filter(|(_, g)| g.glyph == 0) {
            count += 1;
            let c = frag_text.and_then(|s| s.chars().nth(k));
            if let Some(c) = c
                && chars.len() < 8
                && !chars.contains(&c)
            {
                chars.push(c);
            }
        }
    }
    (count, chars.iter().map(|c| format!("U+{:04X}", *c as u32)).collect::<Vec<_>>().join(" "))
}

fn usage() -> ExitCode {
    eprintln!(
        "usage:\n  qinglan-cli dom <file|url> [--replay <dir>]\n  qinglan-cli style <file|url> [--width W] [--height H] [--time] [--replay <dir>]\n  qinglan-cli layout <file|url> [--width W] [--height H] [--test-font] [--time] [--replay <dir>]\n  qinglan-cli record <url> <dir> [--width W] [--height H]
  qinglan-cli render <file|url> -o out.png [--width W] [--height H | --full] [--dpr D] [--scroll Y] [--replay <dir>] [--test-font] [--time] [--scroll-bench]
  qinglan-cli bench <url> --replay <dir> [--width W] [--height H] [--dpr D] [--max-frames N] [--mem]"
    );
    ExitCode::from(2)
}

struct Args {
    cmd: String,
    input: String,
    /// `record`: the directory to write.
    out_dir: Option<PathBuf>,
    width: f32,
    height: f32,
    timing: bool,
    test_font: bool,
    replay: Option<PathBuf>,
    /// `render`: the PNG to write, `--full` for the whole page, device pixel ratio, scroll position.
    out_png: Option<PathBuf>,
    full: bool,
    dpr: f32,
    scroll: f32,
    scroll_bench: bool,
    no_prewarm: bool,
    no_stop: bool,
    /// `bench`: stop scrolling after this many frames (a page a billion pixels high would scroll for hours).
    max_frames: usize,
}

fn parse_args() -> Option<Args> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first()?.clone();
    let input = args.get(1)?.clone();
    let mut a = Args {
        cmd,
        input,
        out_dir: None,
        width: 1280.0,
        height: 1000.0,
        timing: false,
        test_font: false,
        replay: None,
        out_png: None,
        full: false,
        dpr: 1.0,
        scroll: 0.0,
        scroll_bench: false,
        no_prewarm: false,
        no_stop: false,
        max_frames: bench::DEFAULT_MAX_FRAMES,
    };
    let mut i = 2;
    if a.cmd == "record" {
        a.out_dir = Some(PathBuf::from(args.get(2)?));
        i = 3;
    }
    while let Some(arg) = args.get(i) {
        match arg.as_str() {
            "--width" => {
                i += 1;
                a.width = args.get(i)?.parse().ok()?;
            }
            "--height" => {
                i += 1;
                a.height = args.get(i)?.parse().ok()?;
            }
            "--replay" => {
                i += 1;
                a.replay = Some(PathBuf::from(args.get(i)?));
            }
            "-o" => {
                i += 1;
                a.out_png = Some(PathBuf::from(args.get(i)?));
            }
            "--dpr" => {
                i += 1;
                a.dpr = args.get(i)?.parse().ok().filter(|d: &f32| (0.25..=8.0).contains(d))?;
            }
            "--scroll" => {
                i += 1;
                a.scroll = args.get(i)?.parse().ok().filter(|s: &f32| s.is_finite() && *s >= 0.0)?;
            }
            "--max-frames" => {
                i += 1;
                a.max_frames = args.get(i)?.parse().ok().filter(|n: &usize| *n >= 1)?;
            }
            "--full" => a.full = true,
            "--scroll-bench" => a.scroll_bench = true,
            "--mem" => {}
            "--no-prewarm" => a.no_prewarm = true,
            "--no-first-screen-first" => a.no_stop = true,
            "--time" => a.timing = true,
            "--test-font" => a.test_font = true,
            _ => return None,
        }
        i += 1;
    }
    Some(a)
}

fn page_url(input: &str) -> Option<Url> {
    if input.starts_with("http://") || input.starts_with("https://") {
        Url::parse(input, None).ok()
    } else {
        url_from_path(input)
    }
}

// Dev builds (`--features mem-stats`) count live heap bytes once `bench --mem` switches it on (the memory
// breakdown); release builds use the system allocator as is.
#[cfg(feature = "mem-stats")]
#[global_allocator]
static ALLOC: qinglan_win::CountingAlloc = qinglan_win::CountingAlloc;

fn main() -> ExitCode {
    // `bench ... --mem` (dev builds only) also prints what each structure holds; counting every allocation costs a
    // little time.
    let flags: Vec<String> = std::env::args().collect();
    if flags.get(1).map(String::as_str) == Some("bench") && flags.iter().any(|a| a == "--mem") {
        #[cfg(feature = "mem-stats")]
        qinglan_win::count_heap();
        #[cfg(not(feature = "mem-stats"))]
        eprintln!("note: --mem needs a build with --features mem-stats; ignored");
    }
    let Some(args) = parse_args() else { return usage() };
    if !matches!(args.cmd.as_str(), "dom" | "style" | "layout" | "record" | "render" | "bench") || (args.cmd == "bench" && args.replay.is_none()) || (args.cmd == "render" && (args.out_png.is_none() || args.width < 1.0 || args.height < 1.0)) {
        return usage();
    }
    // DirectWrite warms up on its own thread while the page loads (L1 task 17); `render` waits for it before
    // the clock for "content ready -> first screen" starts, as the window will have it done by then.
    let prewarm_started = Instant::now();
    let prewarm = (matches!(args.cmd.as_str(), "render" | "bench") && !args.test_font && !args.no_prewarm).then(|| {
        std::thread::spawn(|| {
            let t = Instant::now();
            (qinglan_win::prewarm(), t.elapsed())
        })
    });
    let Some(url) = page_url(&args.input) else {
        eprintln!("cannot open {}: not an http(s) address and not a file that exists", args.input);
        return ExitCode::FAILURE;
    };

    // The transport: a recording, a recorder around WinHTTP, or WinHTTP itself.
    let mut recorder: Option<Arc<Recorder>> = None;
    let transport: Arc<dyn Transport> = if let Some(dir) = &args.replay {
        match ReplayTransport::open(dir) {
            Ok(t) => Arc::new(t),
            Err(e) => {
                eprintln!("cannot read the recording in {}: {e}", dir.display());
                return ExitCode::FAILURE;
            }
        }
    } else {
        let net = match WinHttpTransport::new() {
            Ok(t) => Arc::new(t),
            Err(e) => {
                eprintln!("error: cannot start WinHTTP: {e}");
                return ExitCode::FAILURE;
            }
        };
        if let Some(dir) = &args.out_dir {
            match Recorder::new(net, dir) {
                Ok(r) => {
                    let r = Arc::new(r);
                    recorder = Some(r.clone());
                    r
                }
                Err(e) => {
                    eprintln!("cannot create {}: {e}", dir.display());
                    return ExitCode::FAILURE;
                }
            }
        } else {
            net
        }
    };
    let mut loader = Loader::without_wake(transport);

    let test_font = args.test_font && args.cmd != "record";
    // In the window the prewarm runs while the page is on its way; here the replay answers at once, so the
    // prewarm is waited for before the platform is made, which is the same order as "prewarm done before content".
    let warmed = prewarm.and_then(|h| h.join().ok());
    let t_platform = Instant::now();
    let win = if test_font { None } else { WinPlatform::new() };
    if args.timing && args.cmd == "render" {
        eprintln!(
            "prewarm {}; platform created in {:.1} ms",
            match warmed {
                Some((n, d)) => format!("opened {n} font faces in {:.1} ms (waited {:.1} ms)", d.as_secs_f64() * 1000.0, prewarm_started.elapsed().as_secs_f64() * 1000.0),
                None => "off".into(),
            },
            t_platform.elapsed().as_secs_f64() * 1000.0
        );
    }
    let platform: &dyn Platform = if test_font {
        &TestPlatform
    } else {
        match &win {
            Some(p) => p,
            None if args.cmd == "dom" => &BasicPlatform,
            None => {
                eprintln!("error: DirectWrite is not available; use --test-font");
                return ExitCode::FAILURE;
            }
        }
    };

    if args.cmd == "bench" {
        let cx = bench::Ctx { loader: &mut loader, platform, win: win.as_ref() };
        return bench::run(cx, &url, &bench::BenchArgs { width: args.width, height: args.height, dpr: args.dpr, max_frames: args.max_frames });
    }
    let t0 = Instant::now();
    let mut opened = match blocking::open(&mut loader, &url, platform, OPEN_TIMEOUT) {
        OpenResult::Page(p) => p,
        OpenResult::Download { url, mime } => {
            eprintln!("this kind of file ({mime}) is not shown in layer 1: {}", url.href());
            return ExitCode::FAILURE;
        }
        OpenResult::Failed(why) => {
            eprintln!("cannot open {}: {why}", url.href());
            return ExitCode::FAILURE;
        }
    };
    if let Some(w) = &opened.warning {
        eprintln!("warning: {w}");
    }
    if opened.status >= 400 {
        eprintln!("note: the server answered HTTP {}", opened.status);
    }
    let t_open = t0.elapsed();

    if args.cmd == "dom" {
        print!("{}", dump_children(&opened.doc, 0));
        if opened.doc.too_large {
            eprintln!("note: page too large, only the first part was parsed");
        }
        return ExitCode::SUCCESS;
    }

    if args.cmd == "render" {
        let mut res = blocking::resources_for(&opened);
        let sopts = StyleOptions { viewport_w: args.width, viewport_h: args.height, ..StyleOptions::default() };
        res.start_sheets(&opened.doc, &mut loader, &sopts);
        if !blocking::load_sheets(&mut loader, &mut res, platform, SHEET_TIMEOUT) {
            eprintln!("warning: some style sheets did not arrive in {} s", SHEET_TIMEOUT.as_secs());
        }
        let t_content = t0.elapsed();
        if args.timing {
            eprintln!("content ready after {:.1} ms (page and style sheets, from the platform on)", t_content.as_secs_f64() * 1000.0);
        }
        let rargs = render::RenderArgs {
            out: args.out_png.clone().unwrap_or_default(),
            full: args.full,
            dpr: args.dpr,
            scroll: args.scroll,
            width: args.width,
            height: args.height,
            timing: args.timing,
            scroll_bench: args.scroll_bench,
            first_screen_first: !args.no_stop,
        };
        return render::run(render::Ctx { opened: &mut opened, res: &mut res, loader: &mut loader, platform, win: win.as_ref() }, &rargs);
    }

    // Style sheets: fetch the linked ones (and what they import), then compute the styles.
    let sopts = StyleOptions { viewport_w: args.width, viewport_h: args.height, profile: args.timing && args.cmd == "style" };
    let mut res = blocking::resources_for(&opened);
    res.start_sheets(&opened.doc, &mut loader, &sopts);
    if !blocking::load_sheets(&mut loader, &mut res, platform, SHEET_TIMEOUT) {
        eprintln!("warning: some style sheets did not arrive in {} s", SHEET_TIMEOUT.as_secs());
    }
    let t_sheets = t0.elapsed();
    let doc_url = opened.url.clone();
    let out = style_document_with(&mut opened.doc, Some(&doc_url), &mut res, &sopts, Some(platform));
    let t_style = t0.elapsed();
    let failed = res.failed_sheets();
    for (u, why) in &failed {
        eprintln!("warning: style sheet not loaded: {u} ({why})");
    }
    for m in &out.missing {
        if !failed.iter().any(|(u, _)| u == m) {
            eprintln!("warning: style sheet not found: {m}");
        }
    }

    if args.cmd == "style" {
        print!("{}", dump_styles(&opened.doc, &out.tree));
        if opened.doc.too_large {
            eprintln!("note: page too large, only the first part was parsed");
        }
        if args.timing {
            eprintln!(
                "open {:.1} ms, sheets {:.1} ms, style {:.1} ms (total {:.1} ms); {} sheets, {} author rules, {} nodes",
                t_open.as_secs_f64() * 1000.0,
                (t_sheets - t_open).as_secs_f64() * 1000.0,
                (t_style - t_sheets).as_secs_f64() * 1000.0,
                t_style.as_secs_f64() * 1000.0,
                out.sheets,
                out.author_rules,
                opened.doc.nodes.len()
            );
            eprintln!("{:?}\n{:?}", out.timings, out.tree.timings);
        }
        return ExitCode::SUCCESS;
    }

    // Layout, then the images it asked for, then layout again with their sizes.
    let lopts = LayoutOptions { viewport_w: args.width, viewport_h: args.height, ..LayoutOptions::default() };
    let mut layout = layout_document(&opened.doc, &out.tree, platform, &lopts);
    let t_layout1 = t0.elapsed();
    let mut sizes = ImageSizes::new();
    let wanted = layout.images.clone();
    let report = if wanted.is_empty() { Default::default() } else { blocking::load_images(&mut loader, &mut res, &wanted, &WicDecoder, &mut sizes, platform, IMAGE_TIMEOUT) };
    let t_images = t0.elapsed();
    if !sizes.is_empty() {
        layout = layout_document_with(&opened.doc, &out.tree, platform, &lopts, &sizes);
    }
    let t_layout = t0.elapsed();

    if args.cmd == "record" {
        let (files, bytes) = recorder.as_ref().map(|r| r.stats()).unwrap_or((0, 0));
        println!("recorded {files} responses ({bytes} body bytes) into {}", args.out_dir.as_ref().map(|d| d.display().to_string()).unwrap_or_default());
        println!("user agent: {USER_AGENT}");
        println!(
            "page {} (HTTP {}), {} style sheets ({} failed), {} images requested: {} loaded, {} broken, {} pending",
            opened.url.href(),
            opened.status,
            out.sheets,
            failed.len(),
            report.requested,
            report.loaded,
            report.broken,
            report.pending
        );
        return ExitCode::SUCCESS;
    }

    print!("{}", dump_layout(&opened.doc, &layout));
    if opened.doc.too_large {
        eprintln!("note: page too large, only the first part was parsed");
    }
    if report.broken > 0 || report.pending > 0 {
        eprintln!("note: {} images broken, {} not arrived", report.broken, report.pending);
    }
    if args.timing {
        let (count, samples) = count_notdef(&layout);
        eprintln!(
            "open {:.1} ms, sheets {:.1} ms, style {:.1} ms, layout {:.1} ms, images {:.1} ms ({} requested, {} loaded), relayout {:.1} ms",
            t_open.as_secs_f64() * 1000.0,
            (t_sheets - t_open).as_secs_f64() * 1000.0,
            (t_style - t_sheets).as_secs_f64() * 1000.0,
            (t_layout1 - t_style).as_secs_f64() * 1000.0,
            (t_images - t_layout1).as_secs_f64() * 1000.0,
            report.requested,
            report.loaded,
            (t_layout - t_images).as_secs_f64() * 1000.0
        );
        eprintln!(".notdef glyphs after fallback: {count}{}", if count > 0 { format!(" (first characters: {samples})") } else { String::new() });
        if let Some(w) = &win {
            eprintln!("{}", w.cache_stats().report());
        }
    }
    ExitCode::SUCCESS
}
