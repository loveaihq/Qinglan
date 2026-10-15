//! Layout of real pages with real fonts (1b acceptance 3 and 4). The pages live in
//! `tests/corpus/local/` (kept out of git); a test is skipped with a note when its page is missing.
//! Run with: `cargo test --release -p qinglan-win --test corpus -- --nocapture`
#![allow(clippy::indexing_slicing, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;
use std::time::Instant;

use qinglan_engine::css::{ResourceLoader, StyleOptions, style_document_with};
use qinglan_engine::encoding::{Encoding, decode_document, decode_stylesheet};
use qinglan_engine::html::treebuilder::parse_document;
use qinglan_engine::layout::{FragKind, LayoutOptions, LayoutResult, layout_document};
use qinglan_engine::platform::Platform;
use qinglan_engine::url::Url;
use qinglan_win::WinPlatform;

struct DirLoader<'a> {
    dir: PathBuf,
    encoding: Encoding,
    platform: &'a WinPlatform,
}

impl ResourceLoader for DirLoader<'_> {
    fn load_stylesheet(&mut self, url: &Url) -> Option<String> {
        let name = url.pathname();
        let bytes = std::fs::read(self.dir.join(name.trim_start_matches('/'))).ok()?;
        Some(decode_stylesheet(&bytes, None, self.encoding, self.platform))
    }
}

fn notdef(res: &LayoutResult) -> (usize, usize) {
    let (mut missing, mut total) = (0, 0);
    for f in &res.fragments {
        if let FragKind::Text { glyphs, .. } | FragKind::Marker { glyphs, .. } = f.kind {
            for g in &res.glyphs[glyphs.0 as usize..glyphs.1 as usize] {
                total += 1;
                missing += usize::from(g.glyph == 0);
            }
        }
    }
    (missing, total)
}

fn median(v: &[f64]) -> f64 {
    let mut s = v.to_vec();
    s.sort_by(f64::total_cmp);
    s[s.len() / 2]
}

#[test]
fn zhwiki_layout_with_real_fonts() {
    let dir = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/corpus/local/zhwiki"));
    let Ok(bytes) = std::fs::read(dir.join("page-local.html")) else {
        println!("corpus: tests/corpus/local/zhwiki/page-local.html not found, skipped");
        return;
    };
    let platform = WinPlatform::new().unwrap();
    let decoded = decode_document(&bytes, None, &platform);
    let mut doc = parse_document(decoded.text);
    let base = Url::parse("http://zh.example/", None).unwrap();
    let sopts = StyleOptions { viewport_w: 1280.0, viewport_h: 1000.0, ..StyleOptions::default() };
    let mut loader = DirLoader { dir, encoding: decoded.encoding, platform: &platform };
    let out = style_document_with(&mut doc, Some(&base), &mut loader, &sopts, Some(&platform as &dyn Platform));
    let opts = LayoutOptions { viewport_w: 1280.0, viewport_h: 1000.0, ..LayoutOptions::default() };
    let mut times = Vec::new();
    let mut last = None;
    for run in 0..10 {
        let t = Instant::now();
        let res = layout_document(&doc, &out.tree, &platform, &opts);
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        times.push(ms);
        println!("corpus: run {run}: {ms:.1} ms");
        last = Some(res);
    }
    let res = last.unwrap();
    let (missing, total) = notdef(&res);
    let s = platform.cache_stats();
    println!(
        "corpus: zhwiki layout, 10 runs, same process: first (cold) {:.1} ms, median {:.1} ms, min {:.1} ms, max {:.1} ms",
        times[0],
        median(&times),
        times.iter().copied().fold(f64::MAX, f64::min),
        times.iter().copied().fold(0.0, f64::max)
    );
    println!("corpus: {} lines, {} fragments, {total} glyphs shown, {missing} .notdef, incomplete: {}", res.lines.len(), res.fragments.len(), res.incomplete);
    println!("corpus: {s:?}");
    assert!(!res.incomplete);
    assert_eq!(missing, 0, "glyphs that stay .notdef after fallback");
    if !cfg!(debug_assertions) {
        assert!(median(&times) <= 100.0, "zhwiki layout median {:.1} ms is over the 100 ms goal", median(&times));
    }
}

/// An old GBK page, `tests/corpus/local/gbk/index.html` (+ its CSS), if one was put there.
#[test]
fn gbk_page_lays_out_without_missing_glyphs() {
    let dir = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/corpus/local/gbk"));
    let Ok(bytes) = std::fs::read(dir.join("index.html")) else {
        println!("corpus: tests/corpus/local/gbk/index.html not found, skipped");
        return;
    };
    let platform = WinPlatform::new().unwrap();
    let decoded = decode_document(&bytes, None, &platform);
    println!("corpus: gbk page decoded as {}, warning: {:?}", decoded.encoding.name(), decoded.warning);
    assert!(decoded.warning.is_none());
    assert!(["GBK", "gb18030"].contains(&decoded.encoding.name()), "{}", decoded.encoding.name());
    assert!(!decoded.text.contains('\u{FFFD}'), "decoded text has replacement characters");
    let mut doc = parse_document(decoded.text);
    let base = Url::parse("http://gbk.example/", None).unwrap();
    let sopts = StyleOptions { viewport_w: 1280.0, viewport_h: 1000.0, ..StyleOptions::default() };
    let mut loader = DirLoader { dir, encoding: decoded.encoding, platform: &platform };
    let out = style_document_with(&mut doc, Some(&base), &mut loader, &sopts, Some(&platform as &dyn Platform));
    let res = layout_document(&doc, &out.tree, &platform, &LayoutOptions { viewport_w: 1280.0, viewport_h: 1000.0, ..LayoutOptions::default() });
    let (missing, total) = notdef(&res);
    println!("corpus: gbk page: {} lines, {total} glyphs shown, {missing} .notdef", res.lines.len());
    assert!(total > 100, "page too small to mean anything");
    assert_eq!(missing, 0);
}
