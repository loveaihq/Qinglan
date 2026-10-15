//! Performance check on a large real page (a 2.4 MB zh.wikipedia article with its three style sheets,
//! `tests/corpus/local/zhwiki/`, kept out of git; see `tools/prepare_zhwiki.py`).
//! Goal (docs/decisions.md, performance): decode + parse + style <= 70 ms, median of 10 runs, release, plugged in.
//! Run with: `cargo test --release -p qinglan-engine --test perf -- --nocapture`
#![allow(clippy::indexing_slicing, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::time::{Duration, Instant};

use qinglan_engine::css::{ResourceLoader, StyleOptions, StyleOutput, style_document};
use qinglan_engine::encoding::{decode_document, decode_stylesheet};
use qinglan_engine::html::treebuilder::parse_document;
use qinglan_engine::platform::BasicPlatform;
use qinglan_engine::url::Url;

const RUNS: usize = 10;

struct DirLoader(std::path::PathBuf);

impl ResourceLoader for DirLoader {
    fn load_stylesheet(&mut self, url: &Url) -> Option<String> {
        let name = url.pathname();
        let bytes = std::fs::read(self.0.join(name.trim_start_matches('/'))).ok()?;
        Some(decode_stylesheet(&bytes, None, qinglan_engine::encoding::Encoding::UTF_8, &BasicPlatform))
    }
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

/// One measured series: the values of one phase in each run.
struct Series {
    name: &'static str,
    values: Vec<f64>,
}

impl Series {
    fn new(name: &'static str) -> Series {
        Series { name, values: Vec::new() }
    }

    fn sorted(&self) -> Vec<f64> {
        let mut v = self.values.clone();
        v.sort_by(f64::total_cmp);
        v
    }

    fn median(&self) -> f64 {
        let v = self.sorted();
        match v.len() {
            0 => 0.0,
            n if n % 2 == 1 => v[n / 2],
            n => (v[n / 2 - 1] + v[n / 2]) / 2.0,
        }
    }

    fn print(&self) {
        let v = self.sorted();
        println!(
            "perf:   {:<22} median {:>6.1}  min {:>6.1}  max {:>6.1} ms",
            self.name,
            self.median(),
            v.first().copied().unwrap_or(0.0),
            v.last().copied().unwrap_or(0.0)
        );
    }
}

struct Run {
    decode: Duration,
    parse: Duration,
    style: Duration,
    out: StyleOutput,
}

fn run_once(bytes: &[u8], dir: &std::path::Path, base: &Url, profile: bool) -> Run {
    let t0 = Instant::now();
    let decoded = decode_document(bytes, None, &BasicPlatform);
    let t1 = Instant::now();
    let mut doc = parse_document(decoded.text);
    let t2 = Instant::now();
    let out = style_document(&mut doc, Some(base), &mut DirLoader(dir.to_path_buf()), &StyleOptions { profile, ..StyleOptions::default() });
    let t3 = Instant::now();
    assert!(out.tree.styles.iter().any(Option::is_some));
    Run { decode: t1 - t0, parse: t2 - t1, style: t3 - t2, out }
}

#[test]
fn zhwiki_parse_and_style() {
    let dir = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/corpus/local/zhwiki"));
    let Ok(bytes) = std::fs::read(dir.join("page-local.html")) else {
        println!("perf: tests/corpus/local/zhwiki/page-local.html not found, skipped");
        return;
    };
    let base = Url::parse("http://zh.example/", None).unwrap();

    // Headline numbers: without profiling, so the clock reads do not distort them.
    let (mut decode, mut parse, mut style, mut total) = (Series::new("decode"), Series::new("parse"), Series::new("style"), Series::new("total"));
    for _ in 0..RUNS {
        let r = run_once(&bytes, &dir, &base, false);
        decode.values.push(ms(r.decode));
        parse.values.push(ms(r.parse));
        style.values.push(ms(r.style));
        total.values.push(ms(r.decode + r.parse + r.style));
    }
    println!("perf: zhwiki page, {RUNS} runs (goal: median total <= 70 ms, release)");
    for s in [&decode, &parse, &style, &total] {
        s.print();
    }

    // Breakdown of the style phase: with profiling on (it adds a few ms of clock reads).
    let names = [
        "load sheets",
        "parse sheets",
        "index rules",
        "style attrs",
        "cascade (all)",
        "  signature",
        "  match plain",
        "  entries+sort",
        "  match sibling-dep",
        "  hints",
        "  share lookup",
        "  computed values",
    ];
    let mut series: Vec<Series> = names.iter().map(|n| Series::new(n)).collect();
    let mut counts = None;
    for _ in 0..RUNS {
        let r = run_once(&bytes, &dir, &base, true);
        let t = &r.out.timings;
        let c = &r.out.tree.timings;
        let vals = [
            t.load_sheets, t.parse_sheets, t.index_rules, t.style_attrs, t.cascade, c.signature, c.match_plain, c.entries, c.match_sensitive,
            c.hints, c.share_lookup, c.compute,
        ];
        for (s, v) in series.iter_mut().zip(vals) {
            s.values.push(ms(v));
        }
        counts = Some((c.elements, c.match_cache_hits, c.match_cache_misses, c.fast_hits, c.share_hits, c.share_misses, r.out.sheets, r.out.author_rules));
    }
    println!("perf: style breakdown (profiling on, so a little slower than above)");
    for s in &series {
        s.print();
    }
    if let Some((el, mh, mm, fast, sh, sm, sheets, rules)) = counts {
        println!("perf: {el} elements, match cache {mh} hits / {mm} misses, {fast} styles straight from the cache, style sharing {sh} hits / {sm} misses, {sheets} sheets, {rules} author rules");
    }

    if !cfg!(debug_assertions) {
        // Regression guard only; the goal itself is reported, not asserted, because it depends on the machine.
        let m = total.median();
        assert!(m <= 150.0, "far too slow: median {m:.1} ms");
    }
}

/// Layout of the same page with the test font at 1280 wide (1b acceptance: <= 100 ms, release, plugged in).
/// Run with: `cargo test --release -p qinglan-engine --test perf zhwiki_layout -- --nocapture`
#[test]
fn zhwiki_layout() {
    use qinglan_engine::css::style_document_with;
    use qinglan_engine::layout::{LayoutOptions, layout_document};
    use qinglan_engine::platform::{Platform, TestPlatform};

    let dir = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/corpus/local/zhwiki"));
    let Ok(bytes) = std::fs::read(dir.join("page-local.html")) else {
        println!("perf: tests/corpus/local/zhwiki/page-local.html not found, skipped");
        return;
    };
    let base = Url::parse("http://zh.example/", None).unwrap();
    let decoded = decode_document(&bytes, None, &BasicPlatform);
    let mut doc = parse_document(decoded.text);
    let platform = TestPlatform;
    let sopts = StyleOptions { viewport_w: 1280.0, viewport_h: 1000.0, ..StyleOptions::default() };
    let out = style_document_with(&mut doc, Some(&base), &mut DirLoader(dir.clone()), &sopts, Some(&platform as &dyn Platform));
    let opts = LayoutOptions { viewport_w: 1280.0, viewport_h: 1000.0, ..LayoutOptions::default() };
    let mut boxes = Series::new("box tree");
    for _ in 0..RUNS {
        let t = Instant::now();
        let tree = qinglan_engine::layout::build_box_tree(&doc, &out.tree);
        boxes.values.push(ms(t.elapsed()));
        assert!(!tree.boxes.is_empty());
    }
    boxes.print();
    let mut layout = Series::new("layout");
    let mut info = (0, 0, 0u64, false);
    for _ in 0..RUNS {
        let t = Instant::now();
        let res = layout_document(&doc, &out.tree, &platform, &opts);
        layout.values.push(ms(t.elapsed()));
        info = (res.lines.len(), res.fragments.len(), res.work, res.incomplete);
    }
    println!("perf: zhwiki layout (test font, 1280 wide), {RUNS} runs (goal: median <= 100 ms, release)");
    layout.print();
    println!("perf: {} lines, {} fragments, {} work units of {}, incomplete: {}", info.0, info.1, info.2, qinglan_engine::layout::DEFAULT_WORK_BUDGET, info.3);
    assert!(!info.3, "the work budget must not cut the Chinese Wikipedia article");
    if !cfg!(debug_assertions) {
        let m = layout.median();
        assert!(m <= 200.0, "far too slow: median {m:.1} ms");
    }
}
