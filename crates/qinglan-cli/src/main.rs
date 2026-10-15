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
//! `style` and `layout` use the Windows platform (DirectWrite fonts, legacy encodings) unless
//! `--test-font` asks for the test platform (every glyph a 1em square, no legacy encodings).
#![forbid(unsafe_code)]

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
        let (text, glyphs) = match f.kind {
            FragKind::Text { text, glyphs, .. } | FragKind::Marker { text, glyphs, .. } => (text, glyphs),
            _ => continue,
        };
        let Some(gs) = res.glyphs.get(glyphs.0 as usize..glyphs.1 as usize) else { continue };
        for g in gs.iter().filter(|g| g.glyph == 0) {
            count += 1;
            let c = res.texts.get(text as usize).and_then(|t| t.get(g.cluster_start as usize..g.cluster_end as usize)).and_then(|s| s.chars().next());
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
        "usage:\n  qinglan-cli dom <file|url> [--replay <dir>]\n  qinglan-cli style <file|url> [--width W] [--height H] [--time] [--replay <dir>]\n  qinglan-cli layout <file|url> [--width W] [--height H] [--test-font] [--time] [--replay <dir>]\n  qinglan-cli record <url> <dir> [--width W] [--height H]"
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
}

fn parse_args() -> Option<Args> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first()?.clone();
    let input = args.get(1)?.clone();
    let mut a = Args { cmd, input, out_dir: None, width: 1280.0, height: 1000.0, timing: false, test_font: false, replay: None };
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

fn main() -> ExitCode {
    let Some(args) = parse_args() else { return usage() };
    if !matches!(args.cmd.as_str(), "dom" | "style" | "layout" | "record") {
        return usage();
    }
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
    let win = if test_font { None } else { WinPlatform::new() };
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

    // Style sheets: fetch the linked ones (and what they import), then compute the styles.
    let sopts = StyleOptions { viewport_w: args.width, viewport_h: args.height, ..StyleOptions::default() };
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
