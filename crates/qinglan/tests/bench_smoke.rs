//! Smoke test of `qinglan.exe --bench`: it opens the real window on a small recording, scrolls it and prints every
//! number (so a desktop session is needed). Also the command-line mistakes that must not open a window.
#![allow(clippy::indexing_slicing, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;
use std::process::Command;

const KEYS: [&str; 14] = [
    "bench-address",
    "bench-viewport",
    "startup-ms",
    "first-screen-ms",
    "first-screen-wall-ms",
    "full-layout-ms",
    "full-layout-wall-ms",
    "scroll-frames",
    "scroll-median-ms",
    "scroll-p95-ms",
    "scroll-max-ms",
    "scroll-decode-ms",
    "peak-private-bytes",
    "private-bytes-after-scroll",
];

/// A one-page recording with a style sheet in the page, a link and enough text to scroll.
fn recording(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("qinglan-bench-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut html = String::from("<!DOCTYPE html><html><head><meta charset=utf-8><title>Bench page</title><style>body{font-family:sans-serif;margin:20px}p{margin:0 0 12px}a{color:#06c}</style></head><body><h1>Bench</h1>");
    for i in 0..120 {
        html.push_str(&format!("<p>Paragraph {i}: the quick brown fox jumps over the lazy dog, \u{4e2d}\u{6587}\u{6bb5}\u{843d} \u{ff0c}<a href=\"#p{i}\" id=p{i}>link {i}</a> and some more words to fill the line.</p>"));
    }
    html.push_str("</body></html>");
    std::fs::write(dir.join("0001.html"), html).unwrap();
    std::fs::write(dir.join("index.txt"), "url: http://bench.test/\nstatus: 200\nheader: Content-Type: text/html; charset=utf-8\nbody: 0001.html\n").unwrap();
    dir
}

fn run(args: &[&str]) -> (Option<i32>, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_qinglan")).args(args).output().expect("start qinglan.exe");
    (out.status.code(), String::from_utf8_lossy(&out.stdout).into_owned(), String::from_utf8_lossy(&out.stderr).into_owned())
}

#[test]
fn bench_opens_the_window_scrolls_and_prints_every_field() {
    let dir = recording("ok");
    let (code, out, err) = run(&["--bench", "http://bench.test/", "--replay", dir.to_str().unwrap(), "--width", "800", "--height", "600"]);
    std::fs::remove_dir_all(&dir).ok();
    assert_eq!(code, Some(0), "stdout:\n{out}\nstderr:\n{err}");
    for key in KEYS {
        let line = out.lines().find(|l| l.starts_with(&format!("{key}: "))).unwrap_or_else(|| panic!("{key} missing in:\n{out}"));
        assert!(!line.ends_with("n/a"), "{key} was not measured: {line}");
    }
    let number = |key: &str| -> f64 { out.lines().find(|l| l.starts_with(&format!("{key}: "))).unwrap().split(' ').nth(1).unwrap().parse().unwrap() };
    assert!(number("scroll-frames") >= 2.0, "the page scrolled: {out}");
    assert!(number("first-screen-ms") > 0.0 && number("full-layout-ms") >= number("first-screen-ms"));
    assert!(number("private-bytes-after-scroll") > 1_000_000.0);
    assert!(!out.contains("bench-error"), "{out}");
}

#[test]
fn a_page_that_cannot_be_opened_fails_the_bench() {
    let dir = recording("fail");
    let (code, out, _) = run(&["--bench", "ftp://bench.test/", "--replay", dir.to_str().unwrap(), "--width", "400", "--height", "300"]);
    std::fs::remove_dir_all(&dir).ok();
    assert_eq!(code, Some(1), "{out}");
    assert!(out.contains("bench-error: "), "{out}");
}

#[test]
fn mistakes_on_the_command_line_do_not_open_a_window() {
    for args in [&["--bench"][..], &["--nope"], &["--width", "800"], &["--bench", "x", "--dpr", "99"]] {
        let (code, _, err) = run(args);
        assert_eq!(code, Some(2), "{args:?}: {err}");
    }
    let (code, _, _) = run(&["--replay", "this-directory-does-not-exist", "http://x/"]);
    assert_eq!(code, Some(1));
}
