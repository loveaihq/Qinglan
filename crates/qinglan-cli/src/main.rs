//! qinglan-cli: development and test tool (not shipped to users).
//!
//! Step 1a commands:
//!   qinglan-cli dom <file>                           print the DOM tree in the html5lib test format
//!   qinglan-cli style <file> [--width W] [--height H] [--time]
//!                                                    print the computed style of every element
//!
//! Step 1b commands:
//!   qinglan-cli layout <file> [--width W] [--height H] [--test-font] [--time]
//!                                                    print the box tree: position and size of every
//!                                                    block, the line boxes and the text fragments
//!
//! `style` and `layout` use the Windows platform (DirectWrite fonts, legacy encodings) unless
//! `--test-font` asks for the test platform (every glyph a 1em square, no legacy encodings).
#![forbid(unsafe_code)]

use std::process::ExitCode;
use std::time::Instant;

use qinglan_engine::css::{ResourceLoader, StyleOptions, dump::dump_styles, style_document_with};
use qinglan_engine::encoding::{Encoding, decode_document, decode_stylesheet};
use qinglan_engine::html::dump::dump_children;
use qinglan_engine::html::treebuilder::parse_document;
use qinglan_engine::layout::{LayoutOptions, dump::dump_layout, layout_document};
use qinglan_engine::layout::FragKind;
use qinglan_engine::platform::{BasicPlatform, Platform, TestPlatform};
use qinglan_engine::url::{Url, percent_decode};
use qinglan_win::WinPlatform;

/// Reads `file:` style sheets from disk, decoding them like the browser will.
struct FileLoader<'a> {
    page_encoding: Encoding,
    platform: &'a dyn Platform,
}

fn url_to_path(url: &Url) -> Option<std::path::PathBuf> {
    if url.scheme != "file" {
        return None;
    }
    let raw = percent_decode(url.pathname().as_bytes());
    let mut p = String::from_utf8_lossy(&raw).into_owned();
    // `/C:/dir/file` -> `C:/dir/file` on Windows.
    let b = p.as_bytes();
    if b.len() >= 3 && b.first() == Some(&b'/') && b.get(2) == Some(&b':') {
        p.remove(0);
    }
    Some(std::path::PathBuf::from(p))
}

impl ResourceLoader for FileLoader<'_> {
    fn load_stylesheet(&mut self, url: &Url) -> Option<String> {
        let path = url_to_path(url)?;
        let bytes = std::fs::read(path).ok()?;
        Some(decode_stylesheet(&bytes, None, self.page_encoding, self.platform))
    }
}

fn file_url(path: &str) -> Option<Url> {
    let abs = std::fs::canonicalize(path).ok()?;
    let s = abs.to_string_lossy().replace('\\', "/");
    let s = s.strip_prefix("//?/").unwrap_or(&s);
    Url::parse(&format!("file:///{s}"), None).ok()
}

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
                && chars.len() < 8 && !chars.contains(&c) {
                    chars.push(c);
                }
        }
    }
    (count, chars.iter().map(|c| format!("U+{:04X}", *c as u32)).collect::<Vec<_>>().join(" "))
}

fn usage() -> ExitCode {
    eprintln!(
        "usage:\n  qinglan-cli dom <file>\n  qinglan-cli style <file> [--width W] [--height H] [--time]\n  qinglan-cli layout <file> [--width W] [--height H] [--test-font]"
    );
    ExitCode::from(2)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(cmd) = args.first() else { return usage() };
    let Some(file) = args.get(1) else { return usage() };
    let mut width = 1280.0f32;
    let mut height = 1000.0f32;
    let mut timing = false;
    let mut test_font = false;
    let mut i = 2;
    while let Some(a) = args.get(i) {
        match a.as_str() {
            "--width" => {
                i += 1;
                match args.get(i).and_then(|v| v.parse().ok()) {
                    Some(v) => width = v,
                    None => return usage(),
                }
            }
            "--height" => {
                i += 1;
                match args.get(i).and_then(|v| v.parse().ok()) {
                    Some(v) => height = v,
                    None => return usage(),
                }
            }
            "--time" => timing = true,
            "--test-font" => test_font = true,
            _ => return usage(),
        }
        i += 1;
    }
    let bytes = match std::fs::read(file) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("cannot read {file}: {e}");
            return ExitCode::FAILURE;
        }
    };
    let win;
    let platform: &dyn Platform = if test_font {
        &TestPlatform
    } else {
        match WinPlatform::new() {
            Some(p) => {
                win = p;
                &win
            }
            None if cmd == "dom" => &BasicPlatform,
            None => {
                eprintln!("error: DirectWrite is not available; use --test-font");
                return ExitCode::FAILURE;
            }
        }
    };
    let t0 = Instant::now();
    let decoded = decode_document(&bytes, None, platform);
    if let Some(w) = &decoded.warning {
        eprintln!("warning: {w}");
    }
    let t_decode = t0.elapsed();
    let encoding = decoded.encoding;
    let mut doc = parse_document(decoded.text);
    let t_parse = t0.elapsed();
    match cmd.as_str() {
        "dom" => {
            print!("{}", dump_children(&doc, 0));
            if doc.too_large {
                eprintln!("note: page too large, only the first part was parsed");
            }
        }
        "style" => {
            let url = file_url(file);
            let mut loader = FileLoader { page_encoding: encoding, platform };
            let sopts = StyleOptions { viewport_w: width, viewport_h: height, ..StyleOptions::default() };
            let out = style_document_with(&mut doc, url.as_ref(), &mut loader, &sopts, Some(platform));
            let t_style = t0.elapsed();
            print!("{}", dump_styles(&doc, &out.tree));
            if doc.too_large {
                eprintln!("note: page too large, only the first part was parsed");
            }
            for m in &out.missing {
                eprintln!("warning: style sheet not found: {m}");
            }
            if timing {
                eprintln!(
                    "decode {:.1} ms, parse {:.1} ms, style {:.1} ms (total {:.1} ms); {} sheets, {} author rules, {} nodes",
                    t_decode.as_secs_f64() * 1000.0,
                    (t_parse - t_decode).as_secs_f64() * 1000.0,
                    (t_style - t_parse).as_secs_f64() * 1000.0,
                    t_style.as_secs_f64() * 1000.0,
                    out.sheets,
                    out.author_rules,
                    doc.nodes.len()
                );
            }
        }
        "layout" => {
            let url = file_url(file);
            let mut loader = FileLoader { page_encoding: encoding, platform };
            let sopts = StyleOptions { viewport_w: width, viewport_h: height, ..StyleOptions::default() };
            let out = style_document_with(&mut doc, url.as_ref(), &mut loader, &sopts, Some(platform));
            let t_style = t0.elapsed();
            let res = layout_document(&doc, &out.tree, platform, &LayoutOptions { viewport_w: width, viewport_h: height, ..LayoutOptions::default() });
            let t_layout = t0.elapsed();
            print!("{}", dump_layout(&doc, &res));
            if doc.too_large {
                eprintln!("note: page too large, only the first part was parsed");
            }
            for m in &out.missing {
                eprintln!("warning: style sheet not found: {m}");
            }
            if timing {
                let (count, samples) = count_notdef(&res);
                eprintln!("style {:.1} ms, layout {:.1} ms", (t_style - t_parse).as_secs_f64() * 1000.0, (t_layout - t_style).as_secs_f64() * 1000.0);
                eprintln!(".notdef glyphs after fallback: {count}{}", if count > 0 { format!(" (first characters: {samples})") } else { String::new() });
            }
        }
        _ => return usage(),
    }
    ExitCode::SUCCESS
}
