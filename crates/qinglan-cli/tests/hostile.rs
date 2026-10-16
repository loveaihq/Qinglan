//! Hostile pages (task sheet section 19) through the whole pipeline: each case is written as a recording and opened
//! with `qinglan-cli bench` (HTML parse, style, layout, display list, raster, a few scroll frames), in a child process
//! so that the peak memory is the page's own. Every case must finish within 5 s (release build; a debug build is
//! much slower, so there the sizes are cut to a tenth and only the memory limit is checked), must not crash, and its
//! peak private bytes must stay at or under 200 MB.
//!
//! `cargo test --release -p qinglan-cli --test hostile -- --nocapture --test-threads=1` prints one
//! `hostile-result:` line per case (the table in `tests/corpus/RESULTS.md`). The cases that need a real HTTP server
//! (gzip bomb, redirect loop, a response that never ends) are in `crates/qinglan-win/tests/net.rs`.
#![allow(clippy::indexing_slicing, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fmt::Write as _;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// The cases run one at a time even when the test harness runs tests in parallel: the time limit is for one page.
static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

/// Sizes: the task sheet's, or a tenth of them in a debug build.
const FULL: usize = if cfg!(debug_assertions) { 10 } else { 1 };
fn n(x: usize) -> usize {
    x / FULL
}

const TIME_LIMIT: Duration = Duration::from_secs(5);
const PEAK_LIMIT: u64 = 200 * 1024 * 1024;

struct Case {
    item: &'static str,
    html: Vec<u8>,
    content_type: &'static str,
    /// More responses: (path, content type, bytes).
    extra: Vec<(String, &'static str, Vec<u8>)>,
}

fn page(item: &'static str, html: impl Into<Vec<u8>>) -> Case {
    Case { item, html: html.into(), content_type: "text/html; charset=utf-8", extra: Vec::new() }
}

fn temp_dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("qinglan-hostile-{}-{name}", std::process::id()));
    std::fs::remove_dir_all(&d).ok();
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn run(case: Case, test: &str) {
    let _g = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let dir = temp_dir(test);
    let mut index = String::new();
    std::fs::write(dir.join("0001.html"), &case.html).unwrap();
    writeln!(index, "url: http://hostile.test/\nstatus: 200\nheader: Content-Type: {}\nbody: 0001.html\n", case.content_type).unwrap();
    for (i, (path, ct, body)) in case.extra.iter().enumerate() {
        let file = format!("x{i:05}.bin");
        std::fs::write(dir.join(&file), body).unwrap();
        writeln!(index, "url: http://hostile.test{path}\nstatus: 200\nheader: Content-Type: {ct}\nbody: {file}\n").unwrap();
    }
    std::fs::write(dir.join("index.txt"), index).unwrap();

    let started = Instant::now();
    let mut child = Command::new(env!("CARGO_BIN_EXE_qinglan-cli"))
        .args(["bench", "http://hostile.test/", "--replay"])
        .arg(&dir)
        .args(["--width", "1280", "--height", "1000", "--max-frames", "10"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // A hard stop well above the limit, so that a hang fails the test instead of stalling it.
    let hard = if cfg!(debug_assertions) { Duration::from_secs(300) } else { Duration::from_secs(60) };
    let mut hung = false;
    let status = loop {
        if let Some(s) = child.try_wait().unwrap() {
            break Some(s);
        }
        if started.elapsed() > hard {
            child.kill().ok();
            hung = true;
            break None;
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    let wall = started.elapsed();
    let out = child.wait_with_output().unwrap();
    std::fs::remove_dir_all(&dir).ok();
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    let peak: u64 = stdout.lines().find_map(|l| l.strip_prefix("peak-private-bytes: ")).and_then(|v| v.trim().parse().ok()).unwrap_or(0);
    let ok = status.is_some_and(|s| s.success());
    let result = if hung {
        "HUNG"
    } else if !ok {
        "FAILED"
    } else if cfg!(not(debug_assertions)) && wall > TIME_LIMIT {
        "TOO SLOW"
    } else if peak > PEAK_LIMIT {
        "TOO BIG"
    } else {
        "ok"
    };
    println!("hostile-result: {} | {test} | {} ms | {:.0} MB | {result}", case.item, wall.as_millis(), peak as f64 / 1_048_576.0);
    assert!(!hung, "{}: still running after {hard:?}\n{stdout}", case.item);
    assert!(ok, "{}: exit {:?}\n{stdout}\n{stderr}", case.item, status);
    assert!(cfg!(debug_assertions) || wall <= TIME_LIMIT, "{}: took {wall:?}", case.item);
    assert!(peak <= PEAK_LIMIT, "{}: peak {peak} bytes", case.item);
    assert!(stdout.lines().any(|l| l.starts_with("scroll-frames: ")), "{}: bench did not get to the scroll\n{stdout}\n{stderr}", case.item);
}

fn lcg(seed: &mut u64) -> u8 {
    *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    (*seed >> 33) as u8
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc ^= u32::from(b);
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
        }
    }
    !crc
}

/// A structurally complete PNG whose header claims `w` x `h` and that has no pixels.
fn png_claiming(w: u32, h: u32) -> Vec<u8> {
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut chunk = |kind: &[u8; 4], data: &[u8]| {
        out.extend((data.len() as u32).to_be_bytes());
        let mut body = kind.to_vec();
        body.extend(data);
        out.extend(&body);
        out.extend(crc32(&body).to_be_bytes());
    };
    let mut ihdr = Vec::new();
    ihdr.extend(w.to_be_bytes());
    ihdr.extend(h.to_be_bytes());
    ihdr.extend([8, 2, 0, 0, 0]);
    chunk(b"IHDR", &ihdr);
    chunk(b"IDAT", &[0x78, 0x9c, 0x03, 0x00, 0x00, 0x00, 0x00, 0x01]);
    chunk(b"IEND", &[]);
    out
}

const SOME_TEXT: &str = "<p>Some text, some more text, and the end of the sentence.</p>";

#[test]
fn nesting_100k() {
    run(page("100 000 nested elements", format!("<!DOCTYPE html><body>{}x{}", "<div>".repeat(n(100_000)), "</div>".repeat(n(100_000)))), "nesting_100k");
}

#[test]
fn elements_2m() {
    run(page("2 000 000 elements", format!("<!DOCTYPE html><body>{}{SOME_TEXT}", "<i></i>".repeat(n(2_000_000)))), "elements_2m");
}

#[test]
fn text_10mb_without_spaces() {
    run(page("10 MB of text without a space", format!("<!DOCTYPE html><body><p>{}</p>{SOME_TEXT}", "a".repeat(n(10_000_000)))), "text_10mb_without_spaces");
}

#[test]
fn css_100k_rules() {
    let mut css = String::new();
    for i in 0..n(100_000) {
        writeln!(css, ".c{i}{{color:#{:06x};margin:{}px}}", i % 0xFFFFFF, i % 7).unwrap();
    }
    run(page("100 000 CSS rules", format!("<!DOCTYPE html><style>{css}</style><body><div class=c5><p class=c77>{SOME_TEXT}</p></div>")), "css_100k_rules");
}

#[test]
fn selectors_very_long() {
    // A compound selector of 100 000 classes, a descendant chain of 100 000 `div`s, and a list of 20 000 selectors.
    let compound = format!("div{}", ".k".repeat(n(100_000)));
    let chain = format!("{}p", "div ".repeat(n(100_000)));
    let list = (0..n(20_000)).map(|i| format!("div.a{i} > p")).collect::<Vec<_>>().join(",");
    run(page("very long selectors", format!("<!DOCTYPE html><style>{compound}{{color:red}}{chain}{{color:blue}}{list}{{color:green}}</style><body><div class=k><div><p>x</p></div></div>{SOME_TEXT}")), "selectors_very_long");
}

#[test]
fn var_expands_exponentially() {
    // --a0 is one word; each custom property after it is two copies of the one before: 2^60 words if expanded.
    let mut css = String::from(":root{--a0:word;");
    for i in 1..=60 {
        write!(css, "--a{i}:var(--a{}) var(--a{});", i - 1, i - 1).unwrap();
    }
    css.push_str("}p{font-family:var(--a60);width:var(--a60);margin:var(--a60);content:var(--a60)}");
    run(page("var() exponential expansion", format!("<!DOCTYPE html><style>{css}</style><body>{SOME_TEXT}")), "var_expands_exponentially");
}

#[test]
fn import_loop_and_1000_imports() {
    let mut case = page(
        "@import loop and 1000 @imports",
        format!("<!DOCTYPE html><link rel=stylesheet href=/a.css><link rel=stylesheet href=/many.css><body>{SOME_TEXT}"),
    );
    case.extra.push(("/a.css".into(), "text/css", b"@import url(/b.css); p{color:red}".to_vec()));
    case.extra.push(("/b.css".into(), "text/css", b"@import url(/a.css); p{margin:1px}".to_vec()));
    let mut many = String::new();
    for i in 0..1000 {
        writeln!(many, "@import url(/s{i}.css);").unwrap();
        case.extra.push((format!("/s{i}.css"), "text/css", format!("p{{padding:{}px}}", i % 5).into_bytes()));
    }
    case.extra.push(("/many.css".into(), "text/css", many.into_bytes()));
    run(case, "import_loop_and_1000_imports");
}

#[test]
fn calc_nested_10000() {
    let levels = 10_000;
    let calc = format!("{}1px{}", "calc(".repeat(levels), ")".repeat(levels));
    run(page("calc() nested 10 000 deep", format!("<!DOCTYPE html><style>p{{width:{calc};margin:{calc}}}</style><body><p style=\"height:{calc}\">x</p>{SOME_TEXT}")), "calc_nested_10000");
}

#[test]
fn images_100k() {
    let mut html = String::from("<!DOCTYPE html><body>");
    for i in 0..n(100_000) {
        write!(html, "<img src=/i{i}.png width=4 height=4>").unwrap();
    }
    run(page("100 000 images", html), "images_100k");
}

#[test]
fn png_header_claims_100k_by_100k() {
    let mut case = page("PNG header claiming 100 000 x 100 000 pixels", format!("<!DOCTYPE html><body><img src=/big.png><img src=/big.png width=50>{SOME_TEXT}"));
    case.extra.push(("/big.png".into(), "image/png", png_claiming(100_000, 100_000)));
    run(case, "png_header_claims_100k_by_100k");
}

#[test]
fn element_with_100k_attributes() {
    let mut html = String::from("<!DOCTYPE html><body><div");
    for i in 0..n(100_000) {
        write!(html, " a{i}=1").unwrap();
    }
    html.push_str(&format!(">{SOME_TEXT}</div>"));
    run(page("one element with 100 000 attributes", html), "element_with_100k_attributes");
}

#[test]
fn bad_utf8() {
    let mut seed = 7;
    let mut html = b"<!DOCTYPE html><meta charset=utf-8><body><p>".to_vec();
    html.extend((0..n(2_000_000)).map(|_| lcg(&mut seed) | 0x80));
    html.extend(b"</p><p>after the garbage</p>");
    run(page("invalid UTF-8", html), "bad_utf8");
}

#[test]
fn bad_gbk() {
    let mut seed = 11;
    let mut html = b"<!DOCTYPE html><meta charset=gbk><body><p>".to_vec();
    // Lead bytes with no trail byte, trail bytes out of range, and the odd valid pair.
    for _ in 0..n(2_500_000) {
        html.extend([0x81 + lcg(&mut seed) % 0x7E, 0x20 + lcg(&mut seed) % 0x10]);
    }
    html.extend(b"</p><p>after the garbage</p>");
    run(page("invalid GBK", html), "bad_gbk");
}

#[test]
fn nul_bytes() {
    let mut html = b"<!DOCTYPE html><body><p>".to_vec();
    html.extend(std::iter::repeat_n(0u8, n(1_000_000)));
    html.extend(b"</p><di\0v cl\0ass=\0x>text<style>p\0{col\0or:red}</style></div>");
    html.extend(SOME_TEXT.as_bytes());
    run(page("NUL bytes", html), "nul_bytes");
}

#[test]
fn font_size_1e9px() {
    run(
        page(
            "font-size: 1e9px",
            format!("<!DOCTYPE html><style>html{{font-size:1e9px}}p{{font-size:1e9px;line-height:1e9px}}</style><body><p>Some text, some more text.</p><p style=\"font-size:1e9em\">more</p>{}", SOME_TEXT.repeat(50)),
        ),
        "font_size_1e9px",
    );
}

#[test]
fn huge_margins() {
    run(
        page(
            "huge positive and negative margins",
            format!(
                "<!DOCTYPE html><style>div{{margin:1e9px -1e9px 1e9px -1e9px;padding:1e9px;width:1e9px;height:1e9px;border:1e9px solid red}}p{{margin:-1e9px;padding:-5px}}</style><body><div><div><p>x</p>{SOME_TEXT}</div></div><p style=\"margin-top:3e38px;margin-left:-3e38px\">y</p>{SOME_TEXT}",
            ),
        ),
        "huge_margins",
    );
}

#[test]
fn inline_nesting_10000() {
    run(page("10 000 nested inline elements", format!("<!DOCTYPE html><body>{}text inside {}<p>after</p>", "<span>".repeat(n(10_000)), "</span>".repeat(n(10_000)))), "inline_nesting_10000");
}

#[test]
fn position_offsets_1e9() {
    run(
        page(
            "positioned elements offset by 1e9",
            format!(
                "<!DOCTYPE html><style>.a{{position:absolute;left:1e9px;top:-1e9px}}.b{{position:fixed;right:-1e9px;bottom:1e9px;width:1e9px}}.c{{position:relative;left:-1e9px;top:1e9px}}.d{{position:absolute;inset:1e9px;z-index:2147483647}}</style><body><div class=a>A{SOME_TEXT}</div><div class=b>B</div><div class=c>C{SOME_TEXT}</div><div class=d>D<div class=a>E</div></div>{SOME_TEXT}",
            ),
        ),
        "position_offsets_1e9",
    );
}
