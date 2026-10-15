//! `qinglan-cli render` end to end with the test font (no fonts of the machine involved): the PNG has the
//! requested size, the page's colours are in it at the right places, and bad arguments are refused.
#![allow(clippy::indexing_slicing, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;
use std::process::Command;

use qinglan_win::WicDecoder;

use qinglan_engine::platform::image::ImageDecoder;

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_qinglan-cli"))
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("qinglan-render-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

fn pixel(png: &[u8], w: u32, h: u32, x: u32, y: u32) -> [u8; 4] {
    let img = WicDecoder.decode(png, w, h).unwrap();
    let i = ((y * w + x) * 4) as usize;
    [img.bgra[i], img.bgra[i + 1], img.bgra[i + 2], img.bgra[i + 3]]
}

const PAGE: &str = "<!doctype html><body style='margin:0;font:10px/10px Ahem;color:#00f'><div style='height:40px;width:30px;background:#f00'></div>\
    <div style='height:2000px;background:#0f0'></div><div style='position:fixed;top:0;right:0;width:10px;height:10px;background:#00f'></div></body>";

#[test]
fn render_writes_a_png_of_the_viewport_a_scrolled_view_and_the_whole_page() {
    let html = scratch("page.html");
    std::fs::write(&html, PAGE).unwrap();

    let png = scratch("a.png");
    let out = cli().args(["render", html.to_str().unwrap(), "-o", png.to_str().unwrap(), "--test-font", "--width", "100", "--height", "60"]).output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let bytes = std::fs::read(&png).unwrap();
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    assert_eq!(pixel(&bytes, 100, 60, 5, 5), [0, 0, 255, 255], "red box");
    assert_eq!(pixel(&bytes, 100, 60, 50, 5), [255, 255, 255, 255], "canvas");
    assert_eq!(pixel(&bytes, 100, 60, 50, 50), [0, 255, 0, 255], "green block below the red one");
    assert_eq!(pixel(&bytes, 100, 60, 95, 5), [255, 0, 0, 255], "fixed box in the corner");

    // Scrolled by 30, at dpr 2: the frame is 200x120 device pixels, the red box ends at css y 40 - 30 = 10 = device 20.
    let out = cli()
        .args(["render", html.to_str().unwrap(), "-o", png.to_str().unwrap(), "--test-font", "--width", "100", "--height", "60", "--dpr", "2", "--scroll", "30"])
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let bytes = std::fs::read(&png).unwrap();
    assert_eq!(pixel(&bytes, 200, 120, 10, 10), [0, 0, 255, 255]);
    assert_eq!(pixel(&bytes, 200, 120, 10, 30), [0, 255, 0, 255]);
    assert_eq!(pixel(&bytes, 200, 120, 190, 10), [255, 0, 0, 255], "the fixed box does not scroll");

    // The whole page: 2040 css px high.
    let out = cli().args(["render", html.to_str().unwrap(), "-o", png.to_str().unwrap(), "--test-font", "--width", "100", "--height", "60", "--full"]).output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let bytes = std::fs::read(&png).unwrap();
    assert_eq!(pixel(&bytes, 100, 2040, 50, 2035), [0, 255, 0, 255]);

    let _ = std::fs::remove_dir_all(png.parent().unwrap());
}

#[test]
fn render_refuses_bad_arguments() {
    for args in [vec!["render", "x.html"], vec!["render", "x.html", "-o", "o.png", "--dpr", "0"], vec!["render", "x.html", "-o", "o.png", "--width", "0"], vec!["render", "x.html", "-o", "o.png", "--scroll", "-3"]] {
        let out = cli().args(&args).output().unwrap();
        assert_eq!(out.status.code(), Some(2), "{args:?}");
    }
    // A file that does not exist is an error, not a crash.
    let out = cli().args(["render", "no-such-file.html", "-o", "o.png", "--test-font"]).output().unwrap();
    assert_eq!(out.status.code(), Some(1));
    // Absurd sizes are refused before anything is allocated.
    let html = scratch("tiny.html");
    std::fs::write(&html, "<p>x").unwrap();
    let out = cli().args(["render", html.to_str().unwrap(), "-o", scratch("o.png").to_str().unwrap(), "--test-font", "--width", "1e9", "--height", "1e9"]).output().unwrap();
    assert_eq!(out.status.code(), Some(1), "{}", String::from_utf8_lossy(&out.stderr));
}
