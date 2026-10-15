//! Performance check on a large real page (a 2.4 MB zh.wikipedia article with its three style sheets,
//! `tests/corpus/local/zhwiki/`, kept out of git; see `tools/prepare_zhwiki.py`).
//! L1 task sheet 1a acceptance 5: parse + style <= 80 ms (release, plugged in).
//! Run with: `cargo test --release -p qinglan-engine --test perf -- --nocapture`
#![allow(clippy::indexing_slicing, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::time::Instant;

use qinglan_engine::css::{ResourceLoader, StyleOptions, style_document};
use qinglan_engine::encoding::{decode_document, decode_stylesheet};
use qinglan_engine::html::treebuilder::parse_document;
use qinglan_engine::platform::BasicPlatform;
use qinglan_engine::url::Url;

struct DirLoader(std::path::PathBuf);

impl ResourceLoader for DirLoader {
    fn load_stylesheet(&mut self, url: &Url) -> Option<String> {
        let name = url.pathname();
        let bytes = std::fs::read(self.0.join(name.trim_start_matches('/'))).ok()?;
        Some(decode_stylesheet(&bytes, None, qinglan_engine::encoding::Encoding::UTF_8, &BasicPlatform))
    }
}

#[test]
fn zhwiki_parse_and_style() {
    let dir = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/corpus/local/zhwiki"));
    let Ok(bytes) = std::fs::read(dir.join("page-local.html")) else {
        println!("perf: tests/corpus/local/zhwiki/page-local.html not found, skipped");
        return;
    };
    let base = Url::parse("http://zh.example/", None).unwrap();
    let mut best = f64::MAX;
    let (mut parse_ms, mut style_ms) = (0.0, 0.0);
    for _ in 0..10 {
        let t0 = Instant::now();
        let decoded = decode_document(&bytes, None, &BasicPlatform);
        let mut doc = parse_document(decoded.text);
        let t1 = Instant::now();
        let out = style_document(&mut doc, Some(&base), &mut DirLoader(dir.clone()), &StyleOptions::default());
        let t2 = Instant::now();
        assert!(out.tree.styles.iter().any(Option::is_some));
        let total = (t2 - t0).as_secs_f64() * 1000.0;
        if total < best {
            best = total;
            parse_ms = (t1 - t0).as_secs_f64() * 1000.0;
            style_ms = (t2 - t1).as_secs_f64() * 1000.0;
        }
    }
    println!("perf: zhwiki page, best of 10: decode+parse {parse_ms:.1} ms, style {style_ms:.1} ms, total {best:.1} ms (goal: 80 ms, release)");
    if !cfg!(debug_assertions) {
        // Regression guard only; the goal itself is reported, not asserted, because it depends on the machine.
        assert!(best < 400.0, "far too slow: {best:.1} ms");
    }
}
