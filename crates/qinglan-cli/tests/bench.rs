//! `qinglan-cli bench` on a small recording: the phase timings and memory are printed and the exit code is 0.
#![allow(clippy::indexing_slicing, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::process::Command;

#[test]
fn bench_prints_the_phases_without_a_window() {
    let dir = std::env::temp_dir().join(format!("qinglan-cli-bench-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut html = String::from("<!DOCTYPE html><title>t</title><style>p{margin:0 0 8px}</style>");
    for i in 0..80 {
        html.push_str(&format!("<p>Paragraph {i}: the quick brown fox jumps over the lazy dog.</p>"));
    }
    std::fs::write(dir.join("0001.html"), html).unwrap();
    std::fs::write(dir.join("index.txt"), "url: http://bench.test/\nstatus: 200\nheader: Content-Type: text/html\nbody: 0001.html\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_qinglan-cli"))
        .args(["bench", "http://bench.test/", "--replay"])
        .arg(&dir)
        .args(["--width", "800", "--height", "300", "--test-font"])
        .output()
        .unwrap();
    std::fs::remove_dir_all(&dir).ok();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{stdout}\n{}", String::from_utf8_lossy(&out.stderr));
    for key in [
        "parse-ms",
        "style-ms",
        "first-layout-ms",
        "display-list-ms",
        "raster-ms",
        "first-screen-ms",
        "full-layout-ms",
        "scroll-frames",
        "scroll-median-ms",
        "scroll-p95-ms",
        "peak-private-bytes",
        "private-bytes-after-scroll",
    ] {
        assert!(stdout.lines().any(|l| l.starts_with(&format!("{key}: "))), "{key} missing in:\n{stdout}");
    }
    let frames: usize = stdout.lines().find(|l| l.starts_with("scroll-frames: ")).unwrap()[15..].parse().unwrap();
    assert!(frames > 1);
}

#[test]
fn bench_needs_a_recording() {
    let out = Command::new(env!("CARGO_BIN_EXE_qinglan-cli")).args(["bench", "http://bench.test/"]).output().unwrap();
    assert_eq!(out.status.code(), Some(2));
}

/// A vertical-rl page scrolls along its own axis: sideways, from the right edge to the leftmost column.
#[test]
fn bench_scrolls_a_vertical_page_sideways() {
    let dir = std::env::temp_dir().join(format!("qinglan-cli-bench-vertical-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut html = String::from("<!DOCTYPE html><title>t</title><body style='margin:0;writing-mode:vertical-rl'>");
    for _ in 0..60 {
        html.push_str("<div style='width:50px;height:100px;background:#00f'></div>");
    }
    std::fs::write(dir.join("0001.html"), html).unwrap();
    std::fs::write(dir.join("index.txt"), "url: http://bench.test/\nstatus: 200\nheader: Content-Type: text/html\nbody: 0001.html\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_qinglan-cli"))
        .args(["bench", "http://bench.test/", "--replay"])
        .arg(&dir)
        .args(["--width", "400", "--height", "100", "--test-font"])
        .output()
        .unwrap();
    std::fs::remove_dir_all(&dir).ok();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{stdout}\n{}", String::from_utf8_lossy(&out.stderr));
    // 60 columns of 50 px are 3000 px wide: 2600 px to scroll in steps of 100 px, 27 frames. (Vertically it would be 1 frame.)
    let frames: usize = stdout.lines().find(|l| l.starts_with("scroll-frames: ")).unwrap()[15..].parse().unwrap();
    assert_eq!(frames, 27, "{stdout}");
}
