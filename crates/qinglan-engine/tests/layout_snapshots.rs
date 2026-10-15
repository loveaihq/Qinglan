//! Layout snapshot tests: every `tests/pages/*.html` is laid out with the test platform (1em square
//! glyphs, ascent 0.8em, descent 0.2em) and the dump is compared with `<page>.layout.txt` next to it.
//!
//! Each page starts with an HTML comment that states, in prose, the key numbers worked out by hand
//! from the spec. Lines of the form
//!
//! ```text
//! check: block div#a :: (8,8 100x32)
//! ```
//!
//! make those numbers executable: the first dump line that starts (after its indentation) with the text
//! before `::` must contain the text after it. Because the numbers in the comment come from the spec and
//! not from the engine, a snapshot that only records what the engine happens to do cannot pass.
//! A `viewport: WxH` line in the comment sets the viewport (default 800x600).
//!
//! To (re)write the expected files after checking the hand numbers: `UPDATE_SNAPSHOTS=1 cargo test
//! -p qinglan-engine --test layout_snapshots`.
#![allow(clippy::indexing_slicing, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

use qinglan_engine::css::{NoResources, StyleOptions, style_document_with};
use qinglan_engine::encoding::decode_document;
use qinglan_engine::html::treebuilder::parse_document;
use qinglan_engine::layout::{LayoutOptions, dump::dump_layout, layout_document};
use qinglan_engine::platform::{BasicPlatform, Platform, TestPlatform};

fn pages_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/pages")
}

/// The first `<!-- ... -->` comment of the page.
fn header_comment(src: &str) -> &str {
    let Some(a) = src.find("<!--") else { return "" };
    let rest = &src[a + 4..];
    match rest.find("-->") {
        Some(b) => &rest[..b],
        None => rest,
    }
}

fn viewport(comment: &str) -> (f32, f32) {
    for line in comment.lines() {
        if let Some(v) = line.trim().strip_prefix("viewport:") {
            let mut it = v.trim().split('x');
            let w = it.next().and_then(|x| x.trim().parse().ok());
            let h = it.next().and_then(|x| x.trim().parse().ok());
            if let (Some(w), Some(h)) = (w, h) {
                return (w, h);
            }
        }
    }
    (800.0, 600.0)
}

pub fn layout_page(src: &str, vw: f32, vh: f32) -> String {
    let platform = TestPlatform;
    let decoded = decode_document(src.as_bytes(), None, &BasicPlatform);
    let mut doc = parse_document(decoded.text);
    let sopts = StyleOptions { viewport_w: vw, viewport_h: vh };
    let styled = style_document_with(&mut doc, None, &mut NoResources, &sopts, Some(&platform as &dyn Platform));
    let res = layout_document(&doc, &styled.tree, &platform, &LayoutOptions { viewport_w: vw, viewport_h: vh });
    dump_layout(&doc, &res)
}

fn run_checks(name: &str, comment: &str, dump: &str, failures: &mut Vec<String>) -> usize {
    let mut n = 0;
    for line in comment.lines() {
        let Some(rest) = line.trim().strip_prefix("check:") else { continue };
        let Some((needle, expected)) = rest.split_once("::") else {
            failures.push(format!("{name}: malformed check line: {line}"));
            continue;
        };
        let (needle, expected) = (needle.trim(), expected.trim());
        n += 1;
        match dump.lines().find(|l| l.trim_start().starts_with(needle)) {
            None => failures.push(format!("{name}: no dump line starts with `{needle}`")),
            Some(l) if !l.contains(expected) => {
                failures.push(format!("{name}: hand check failed for `{needle}`: expected `{expected}` in\n    {}", l.trim()))
            }
            Some(_) => {}
        }
    }
    n
}

fn first_difference(a: &str, b: &str) -> String {
    for (i, (x, y)) in a.lines().zip(b.lines()).enumerate() {
        if x != y {
            return format!("line {}:\n  expected: {x}\n  actual:   {y}", i + 1);
        }
    }
    format!("line counts differ: expected {}, actual {}", a.lines().count(), b.lines().count())
}

#[test]
fn layout_snapshots() {
    let update = std::env::var_os("UPDATE_SNAPSHOTS").is_some();
    let mut pages: Vec<PathBuf> = std::fs::read_dir(pages_dir())
        .expect("tests/pages exists")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "html") && !p.file_name().is_some_and(|n| n.to_string_lossy().starts_with('_')))
        .collect();
    pages.sort();
    assert!(!pages.is_empty(), "no pages in tests/pages");
    let mut failures = Vec::new();
    let mut checks = 0;
    for page in &pages {
        let name = page.file_name().unwrap().to_string_lossy().into_owned();
        let src = std::fs::read_to_string(page).unwrap();
        let comment = header_comment(&src);
        let (vw, vh) = viewport(comment);
        let dump = layout_page(&src, vw, vh);
        let n = run_checks(&name, comment, &dump, &mut failures);
        if n == 0 {
            failures.push(format!("{name}: the header comment has no `check:` lines (hand-computed numbers)"));
        }
        checks += n;
        let expected_path = page.with_extension("layout.txt");
        if update {
            if std::fs::read_to_string(&expected_path).ok().as_deref() != Some(dump.as_str()) {
                std::fs::write(&expected_path, &dump).unwrap();
                eprintln!("updated {}", expected_path.display());
            }
            continue;
        }
        match std::fs::read_to_string(&expected_path) {
            Err(_) => failures.push(format!("{name}: no expected dump (run with UPDATE_SNAPSHOTS=1)")),
            Ok(exp) if exp.replace("\r\n", "\n") != dump => {
                failures.push(format!("{name}: dump differs from {}\n  {}", expected_path.display(), first_difference(&exp.replace("\r\n", "\n"), &dump)))
            }
            Ok(_) => {}
        }
    }
    eprintln!("{} pages, {} hand-checked numbers", pages.len(), checks);
    assert!(failures.is_empty(), "{} problem(s):\n{}", failures.len(), failures.join("\n"));
}
