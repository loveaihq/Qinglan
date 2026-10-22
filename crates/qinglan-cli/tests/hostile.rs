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
/// The tighter limit of the invalid UTF-8 case (release builds only: a debug build allocates differently).
const PEAK_LIMIT_GARBAGE: u64 = 120 * 1024 * 1024;

struct Case {
    item: &'static str,
    html: Vec<u8>,
    content_type: &'static str,
    /// More responses: (path, content type, bytes).
    extra: Vec<(String, &'static str, Vec<u8>)>,
    /// Peak private bytes allowed; `PEAK_LIMIT` unless the case sets less.
    peak_limit: u64,
    /// Web fonts that must have been fetched, handed to the decoders and refused (the bench prints `web-fonts:`); 0: not checked.
    fonts_failed: usize,
}

fn page(item: &'static str, html: impl Into<Vec<u8>>) -> Case {
    Case { item, html: html.into(), content_type: "text/html; charset=utf-8", extra: Vec::new(), peak_limit: PEAK_LIMIT, fonts_failed: 0 }
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
    let limit = if cfg!(debug_assertions) { PEAK_LIMIT } else { case.peak_limit };
    let result = if hung {
        "HUNG"
    } else if !ok {
        "FAILED"
    } else if cfg!(not(debug_assertions)) && wall > TIME_LIMIT {
        "TOO SLOW"
    } else if peak > limit {
        "TOO BIG"
    } else {
        "ok"
    };
    println!("hostile-result: {} | {test} | {} ms | {:.0} MB | {result}", case.item, wall.as_millis(), peak as f64 / 1_048_576.0);
    assert!(!hung, "{}: still running after {hard:?}\n{stdout}", case.item);
    assert!(ok, "{}: exit {:?}\n{stdout}\n{stderr}", case.item, status);
    assert!(cfg!(debug_assertions) || wall <= TIME_LIMIT, "{}: took {wall:?}", case.item);
    assert!(peak <= limit, "{}: peak {peak} bytes (limit {limit})", case.item);
    assert!(stdout.lines().any(|l| l.starts_with("scroll-frames: ")), "{}: bench did not get to the scroll\n{stdout}\n{stderr}", case.item);
    if case.fonts_failed > 0 {
        let want = format!("web-fonts: {0} declared, 0 loaded, {0} failed", case.fonts_failed);
        assert!(stdout.lines().any(|l| l == want), "{}: expected `{want}`\n{stdout}\n{stderr}", case.item);
    }
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
    html.extend((0..n(4_000_000)).map(|_| lcg(&mut seed) | 0x80));
    html.extend(b"</p><p>after the garbage</p>");
    // 4 MB of garbage is a million shaped characters (the page's text allowance): that case is held to 120 MB.
    let mut case = page("invalid UTF-8", html);
    case.peak_limit = PEAK_LIMIT_GARBAGE;
    run(case, "bad_utf8");
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

/// A number in `lo..hi` that looks arbitrary but is the same on every run.
fn pick(seed: &mut u64, lo: usize, hi: usize) -> usize {
    let (a, b) = (usize::from(lcg(seed)), usize::from(lcg(seed)));
    lo + ((a << 8) | b) % (hi - lo)
}

// The cases below are the 2a/2b review's: layout work that nothing used to count (grid auto-placement, spans over
// thousands of tracks, floats that never fit beside each other) and boxes that nothing used to count (generated content).

#[test]
fn grid_dense_200k_items() {
    let html = format!(
        "<!DOCTYPE html><style>.g{{display:grid;grid-auto-flow:row dense;grid-template-columns:repeat(1000,1px);grid-template-rows:repeat(1000,1px)}}</style><body><div class=g>{}</div>{SOME_TEXT}",
        "<i></i>".repeat(n(200_000))
    );
    run(page("200 000 items in a dense 1000 x 1000 grid", html), "grid_dense_200k_items");
}

#[test]
fn grid_dense_wide_spans() {
    let mut seed = 5;
    let mut html = String::from("<!DOCTYPE html><style>.g{display:grid;grid-auto-flow:row dense;grid-template-columns:repeat(1000,1px);grid-template-rows:repeat(1000,1px)}</style><body><div class=g>");
    for _ in 0..n(100_000) {
        write!(html, "<i style='grid-column:span {}'></i>", [1, 2, 500, 999, 1000][pick(&mut seed, 0, 5)]).unwrap();
    }
    html.push_str("</div>");
    run(page("100 000 grid items spanning up to 1000 columns, dense", html), "grid_dense_wide_spans");
}

#[test]
fn grid_one_column_items_dense() {
    let html = format!(
        "<!DOCTYPE html><style>.g{{display:grid;grid-auto-flow:dense;grid-template-columns:1px}}</style><body><div class=g>{}</div>{SOME_TEXT}",
        "<i style='grid-column:1'></i>".repeat(n(100_000))
    );
    run(page("100 000 dense grid items in one column", html), "grid_one_column_items_dense");
}

#[test]
fn grid_areas_1000_by_1000_lines() {
    let mut seed = 9;
    let mut html = String::from("<!DOCTYPE html><style>.g{display:grid;grid-auto-rows:1px;grid-auto-columns:1px}</style><body><div class=g>");
    for _ in 0..n(50_000) {
        write!(html, "<i style='grid-area:{}/{}/span 1000/span 1000'></i>", pick(&mut seed, 1, 2000), pick(&mut seed, 1, 2000)).unwrap();
    }
    html.push_str("</div>");
    run(page("50 000 grid areas of 1000 x 1000 cells", html), "grid_areas_1000_by_1000_lines");
}

#[test]
fn grid_10000_items_spanning_1000_auto_tracks() {
    let mut seed = 3;
    let mut html = String::from("<!DOCTYPE html><style>.g{display:grid;grid-template-columns:repeat(1000,auto);grid-template-rows:repeat(1000,auto);width:3000px}</style><body><div class=g>");
    for _ in 0..n(10_000) {
        write!(html, "<i style='grid-column:1/span 1000;grid-row:{}/span 1000'>x y z</i>", pick(&mut seed, 1, 1000)).unwrap();
    }
    html.push_str("</div>");
    run(page("10 000 grid items, each over 1000 auto tracks", html), "grid_10000_items_spanning_1000_auto_tracks");
}

#[test]
fn before_and_after_on_100k_elements() {
    let html = format!("<!DOCTYPE html><style>b::before{{content:'a'}}b::after{{content:'b'}}</style><body>{}", "<b>x</b>".repeat(n(100_000)));
    run(page("::before and ::after on 100 000 elements", html), "before_and_after_on_100k_elements");
}

#[test]
fn quotes_100k() {
    run(page("100 000 <q> elements", format!("<!DOCTYPE html><body>{}", "<q>x</q>".repeat(n(100_000)))), "quotes_100k");
}

#[test]
fn floats_100k() {
    let mut seed = 13;
    let mut html = String::from("<!DOCTYPE html><style>body{width:600px}</style><body>");
    for i in 0..n(100_000) {
        write!(html, "<i style='float:{};width:{}px;height:{}px'></i>", ["left", "right"][i % 2], pick(&mut seed, 100, 300), pick(&mut seed, 1, 50)).unwrap();
    }
    run(page("100 000 floats that never fit side by side", html), "floats_100k");
}

// ---- web fonts (step 2d-1): the decoders are fed hostile font files through the whole pipeline ----------------------

/// Bits written the way deflate and Brotli read them: the first bit goes to the low end of the first byte.
struct Bits {
    bytes: Vec<u8>,
    n: usize,
}

impl Bits {
    fn new() -> Bits {
        Bits { bytes: Vec::new(), n: 0 }
    }
    fn put(&mut self, value: u64, count: usize) {
        for i in 0..count {
            if self.n.is_multiple_of(8) {
                self.bytes.push(0);
            }
            if i < 64 && (value >> i) & 1 == 1 {
                let last = self.bytes.len() - 1;
                self.bytes[last] |= 1 << (self.n % 8);
            }
            self.n += 1;
        }
    }
    /// A prefix code, most significant bit first (deflate's Huffman codes).
    fn code(&mut self, value: u32, len: usize) {
        for i in (0..len).rev() {
            self.put(u64::from((value >> i) & 1), 1);
        }
    }
}

/// A Brotli stream of `blocks` meta-blocks of 958 * 1095 bytes each (zeros): every command inserts one literal and
/// copies 1094 bytes from distance 1, with all the prefix codes one symbol long, so a command costs 11 bits.
fn brotli_zeros(blocks: usize) -> Vec<u8> {
    let mut b = Bits::new();
    b.put(0, 1); // WBITS 16
    let mlen: u64 = 958 * 1095;
    for _ in 0..blocks {
        b.put(0, 1); // ISLAST
        b.put(2, 2); // 6 nibbles
        b.put(mlen - 1, 24);
        b.put(0, 1); // ISUNCOMPRESSED
        b.put(0, 3); // one block type each for literals, commands, distances
        b.put(0, 2); // NPOSTFIX
        b.put(0, 4); // NDIRECT
        b.put(0, 2); // context mode of the literal block type
        b.put(0, 2); // one literal tree, one distance tree
        for (alphabet_bits, symbol) in [(8, 0u64), (10, 398), (6, 16)] {
            b.put(1, 2); // simple prefix code
            b.put(0, 2); // one symbol
            b.put(symbol, alphabet_bits);
        }
        b.put(0, 11 * 958);
    }
    b.put(1, 1); // ISLAST
    b.put(1, 1); // ISLASTEMPTY
    b.bytes
}

/// A zlib stream of one literal and `matches` copies of 258 bytes (distance 1): a fixed-Huffman block, 13 bits each.
fn zlib_zeros(matches: usize) -> Vec<u8> {
    let mut b = Bits::new();
    b.put(1, 1); // BFINAL
    b.put(1, 2); // fixed Huffman
    b.code(0x30 + 0x61, 8); // literal 'a'
    for _ in 0..matches {
        b.code(0xC0 + 5, 8); // length symbol 285 = 258
        b.code(0, 5); // distance symbol 0 = 1
    }
    b.code(0, 7); // end of block
    let mut z = vec![0x78, 0x9C];
    z.extend_from_slice(&b.bytes);
    z.extend_from_slice(&[0, 0, 0, 0]);
    z
}

fn font_case(item: &'static str, mime: &'static str, font: Vec<u8>) -> Case {
    let html = "<!doctype html><title>x</title><style>@font-face { font-family: hostile; src: url(/f.font) } p { font-family: hostile, serif; font-size: 24px }</style>\
                <p>Hello, web font. The quick brown fox jumps over the lazy dog.</p>";
    let mut c = page(item, html);
    c.extra.push(("/f.font".to_string(), mime, font));
    c.fonts_failed = 1;
    c
}

fn woff2_with(total_sfnt: u32, directory: &[u8], compressed: &[u8]) -> Vec<u8> {
    let mut f = Vec::new();
    f.extend_from_slice(b"wOF2");
    f.extend_from_slice(&[0, 1, 0, 0]);
    f.extend_from_slice(&0u32.to_be_bytes()); // length: patched below
    f.extend_from_slice(&1u16.to_be_bytes());
    f.extend_from_slice(&[0, 0]);
    f.extend_from_slice(&total_sfnt.to_be_bytes());
    f.extend_from_slice(&(compressed.len() as u32).to_be_bytes());
    f.extend_from_slice(&[0; 24]);
    f.extend_from_slice(directory);
    f.extend_from_slice(compressed);
    let len = f.len() as u32;
    f[8..12].copy_from_slice(&len.to_be_bytes());
    f
}

#[test]
fn woff2_claims_a_1gb_table() {
    // flags 1 (head), origLength 1 GB as UIntBase128, an empty Brotli stream.
    let font = woff2_with(1 << 30, &[1, 0x84, 0x80, 0x80, 0x80, 0x00], &[0x06]);
    run(font_case("web font: WOFF2 that claims a 1 GB table", "font/woff2", font), "woff2_1gb");
}

#[test]
fn brotli_bomb_in_a_woff2() {
    // 8,000,000 bytes declared (head), 600 meta-blocks of 1 MB behind them (about 750 KB of input, 600 MB of output).
    let stream = brotli_zeros(600);
    let font = woff2_with(8_000_000, &[1, 0xBD, 0x84, 0x80, 0x00], &stream);
    run(font_case("web font: Brotli bomb (600 MB from 750 KB)", "font/woff2", font), "brotli_bomb");
}

#[test]
fn zlib_bomb_in_a_woff() {
    // One table of 4,000,000 declared bytes whose zlib stream goes on for 100 MB (400,000 copies of 258 bytes).
    let z = zlib_zeros(400_000);
    let mut f = vec![0u8; 64];
    f[..4].copy_from_slice(b"wOFF");
    f[4..8].copy_from_slice(&[0, 1, 0, 0]);
    f[12..14].copy_from_slice(&1u16.to_be_bytes());
    f[16..20].copy_from_slice(&4_000_000u32.to_be_bytes());
    f[44..48].copy_from_slice(b"head");
    f[48..52].copy_from_slice(&64u32.to_be_bytes());
    f[52..56].copy_from_slice(&(z.len() as u32).to_be_bytes());
    f[56..60].copy_from_slice(&4_000_000u32.to_be_bytes());
    f.extend_from_slice(&z);
    let len = f.len() as u32;
    f[8..12].copy_from_slice(&len.to_be_bytes());
    run(font_case("web font: zlib bomb (100 MB from 650 KB)", "font/woff", f), "zlib_bomb");
}

// ---- 2c/2d review: decoders and amplification ---------------------------------------------------------------------------

/// A Brotli stream of `blocks` meta-blocks of one byte each, every one of them asking for what costs most to set up: 256
/// literal block types (so a 16,384-entry literal context map) whose map is one run of zeros with the move-to-front
/// transform undone, in about 82 bytes. Before the work budget and the O(i) move-to-front, a block cost 0.7 ms: 12,000 of
/// them (a 984 KB font) took 8.5 s.
fn brotli_context_maps(blocks: usize) -> Vec<u8> {
    let mut b = Bits::new();
    b.put(0, 1); // WBITS 16
    for _ in 0..blocks {
        b.put(0, 1); // ISLAST
        b.put(0, 2); // 4 nibbles
        b.put(0, 16); // one byte
        b.put(0, 1); // ISUNCOMPRESSED
        b.put(1, 1); // NBLTYPESL = 256
        b.put(7, 3);
        b.put(127, 7);
        b.put(1, 2); // block type code: simple, one symbol
        b.put(0, 2);
        b.put(0, 9);
        b.put(1, 2); // block count code: simple, one symbol
        b.put(0, 2);
        b.put(0, 5);
        b.put(3, 2); // the first literal block: 4 symbols
        b.put(0, 1); // NBLTYPESI = 1
        b.put(0, 1); // NBLTYPESD = 1
        b.put(0, 2); // NPOSTFIX
        b.put(0, 4); // NDIRECT
        for _ in 0..256 {
            b.put(0, 2); // context mode
        }
        b.put(1, 1); // NTREESL = 2
        b.put(0, 3);
        b.put(1, 1); // RLEMAX = 14
        b.put(13, 4);
        b.put(1, 2); // the code of the map: simple, one symbol, 14 = a run
        b.put(0, 2);
        b.put(14, 4);
        b.put(0, 14); // the run: 16,384 zeros
        b.put(1, 1); // IMTF
        b.put(0, 1); // NTREESD = 1
        for _ in 0..2 {
            b.put(1, 2); // two literal codes: one symbol, 'A'
            b.put(0, 2);
            b.put(65, 8);
        }
        b.put(1, 2); // the command code: one symbol
        b.put(0, 2);
        b.put(8, 10);
        b.put(1, 2); // the distance code: one symbol
        b.put(0, 2);
        b.put(0, 6);
    }
    b.put(1, 1); // ISLAST
    b.put(1, 1); // ISLASTEMPTY
    b.bytes
}

fn base128(mut v: usize) -> Vec<u8> {
    let mut out = vec![(v & 0x7F) as u8];
    v >>= 7;
    while v != 0 {
        out.push((v & 0x7F) as u8 | 0x80);
        v >>= 7;
    }
    out.reverse();
    out
}

fn woff2_of_context_maps(blocks: usize) -> Vec<u8> {
    let stream = brotli_context_maps(blocks);
    let mut directory = vec![0u8]; // flags 0: cmap
    directory.extend(base128(blocks));
    woff2_with(blocks as u32, &directory, &stream)
}

#[test]
fn brotli_context_maps_3000_blocks() {
    run(font_case("web font: 3,000 Brotli meta-blocks with 16K-entry context maps", "font/woff2", woff2_of_context_maps(n(3000))), "brotli_maps_3000");
}

#[test]
fn brotli_context_maps_12000_blocks() {
    run(font_case("web font: 12,000 Brotli meta-blocks with 16K-entry context maps (984 KB)", "font/woff2", woff2_of_context_maps(n(12000))), "brotli_maps_12000");
}

#[test]
fn brotli_context_maps_in_32_faces() {
    // The most faces a page fetches, each a different file of the 12,000-block font: 32 MB of fonts.
    let font = woff2_of_context_maps(n(12000));
    let (mut css, mut body) = (String::new(), String::new());
    for i in 0..32 {
        write!(css, "@font-face {{ font-family: h{i}; src: url(/f{i}.font) }} .c{i} {{ font-family: h{i}, serif; font-size: 24px }} ").unwrap();
        write!(body, "<p class=c{i}>Hello, web font {i}.</p>").unwrap();
    }
    let mut c = page("web font: 32 faces of 12,000 Brotli meta-blocks each", format!("<!doctype html><title>x</title><style>{css}</style>{body}"));
    for i in 0..32 {
        c.extra.push((format!("/f{i}.font"), "font/woff2", font.clone()));
    }
    c.fonts_failed = 32;
    run(c, "brotli_maps_32_faces");
}

#[test]
fn ruby_100k_pairs() {
    let html = format!("<!doctype html><meta charset=utf-8><p style=\"width:800px\">{}</p>", "<ruby>字<rt>注音</rt></ruby>".repeat(n(100_000)));
    run(page("ruby: 100,000 pairs", html), "ruby_100k");
}

// ---- 2c review: shadow masks ---------------------------------------------------------------------------------------------

#[test]
fn shadows_on_300_screen_sized_boxes() {
    // Each shadow window is over the 1 Mpx a mask was kept at, so every frame made them again: 300 of them took 10 s.
    let html = format!(
        "<!doctype html><style>.b{{position:absolute;left:0;top:0;width:1270px;height:1000px;box-shadow:0 0 4px #000}}</style>{}",
        "<div class=b></div>".repeat(n(300))
    );
    run(page("box-shadow: 300 boxes of 1270 x 1000", html), "shadows_300_big");
}

#[test]
fn shadows_on_400_nested_boxes() {
    let depth = n(400);
    let html = format!(
        "<!doctype html><style>.n{{padding:2px;background:rgba(255,0,0,.05);box-shadow:0 0 4px #000;border-radius:5px}}</style>{}x{}",
        "<div class=n>".repeat(depth),
        "</div>".repeat(depth)
    );
    run(page("box-shadow: 400 nested boxes with rounded corners", html), "shadows_400_nested");
}

#[test]
fn shadow_around_a_page_sized_box() {
    // The shadow window is 1330 x 3370: over what a whole-window mask could be (4 Mpx), so it used to be left out.
    let html = "<!doctype html><style>body{margin:0}.w{width:1260px;margin:10px;height:3300px;background:#fff;border-radius:12px;box-shadow:0 4px 24px rgba(0,0,0,.3), inset 0 0 30px #00f}</style><div class=w><p>x</p></div>";
    run(page("box-shadow: 1260 x 3300 wrapper", html), "shadow_page_sized");
}

// ---- SVG (step 2e-1): inline `<svg>` and SVG files are page content like any other ---------------------------------

const SVG_NS: &str = "xmlns='http://www.w3.org/2000/svg'";

/// Ten levels of ten `use` elements each: 10^10 instances if they were all expanded.
fn use_bomb() -> String {
    let mut s = String::from("<defs><rect id='g0' width='1' height='1' fill='#f00'/>");
    for level in 1..=10 {
        write!(s, "<g id='g{level}'>").unwrap();
        for _ in 0..10 {
            write!(s, "<use href='#g{}'/>", level - 1).unwrap();
        }
        s.push_str("</g>");
    }
    s.push_str("</defs><use href='#g10'/>");
    s
}

fn svg_file(body: &str) -> Vec<u8> {
    format!("<svg {SVG_NS} width='300' height='300' viewBox='0 0 100 100'>{body}</svg>").into_bytes()
}

fn svg_image_case(item: &'static str, file: Vec<u8>) -> Case {
    let mut c = page(item, "<!doctype html><body style='margin:0'><img src='/x.svg' width='300' height='300'><img src='/x.svg' width='40' height='40'>");
    c.extra.push(("/x.svg".to_string(), "image/svg+xml", file));
    c
}

#[test]
fn svg_use_bomb_inline() {
    run(page("svg use bomb (inline)", format!("<!doctype html><svg width='300' height='300' viewBox='0 0 100 100'>{}</svg>", use_bomb())), "svguseinline");
}

#[test]
fn svg_use_bomb_image() {
    run(svg_image_case("svg use bomb (image)", svg_file(&use_bomb())), "svguseimage");
}

fn many_commands(count: usize) -> String {
    let mut d = String::from("M0 0");
    for i in 0..count {
        write!(d, "L{} {}", i % 97, (i * 7) % 89).unwrap();
    }
    d
}

#[test]
fn svg_million_path_commands_inline() {
    let d = many_commands(n(1_000_000));
    run(
        page("svg 1M path commands (inline)", format!("<!doctype html><svg width='300' height='300' viewBox='0 0 100 100'><path d='{d}' fill='#00f' stroke='#000' stroke-width='0.5' stroke-dasharray='1 1'/></svg>")),
        "svgpathinline",
    );
}

#[test]
fn svg_million_path_commands_image() {
    let d = many_commands(n(1_000_000));
    run(svg_image_case("svg 1M path commands (image)", svg_file(&format!("<path d='{d}' fill='#00f' stroke='#000' stroke-width='0.5'/>"))), "svgpathimage");
}

#[test]
fn svg_deep_nesting() {
    let k = n(5000);
    let body = format!("{}<rect width='50' height='50' fill='#f00'/>{}", "<g opacity='.9' transform='translate(.1 .1)'>".repeat(k), "</g>".repeat(k));
    // Inline: the HTML parser stops nesting at 512, the scene at its own depth limit. As a file: more than 256 is broken.
    run(page("svg 5000 nested groups (inline)", format!("<!doctype html><svg width='300' height='300' viewBox='0 0 100 100'>{body}</svg>")), "svgdeepinline");
    run(svg_image_case("svg 5000 nested groups (image)", svg_file(&body)), "svgdeepimage");
}

#[test]
fn svg_huge_viewbox_scales_and_tiny_dashes() {
    let shapes = "<rect width='1e30' height='1e30' fill='#f00' stroke='#000' stroke-width='1e20' stroke-dasharray='0.0001 0.0001'/>\
                  <circle r='1e9' fill='#00f' stroke='#0f0' stroke-width='1e-9' stroke-dasharray='1e-9'/><path d='M0 0 L1e38 1e38 L-1e38 5 Z' stroke='#000' stroke-width='1e30' stroke-linejoin='round'/>";
    for (i, vb) in ["0 0 1e-30 1e-30", "0 0 1e30 1e30", "-1e9 -1e9 2e9 2e9", "0 0 0.0001 100000"].into_iter().enumerate() {
        let item: &'static str = ["svg viewBox 1e-30", "svg viewBox 1e30", "svg viewBox 2e9", "svg viewBox 1e-4 x 1e5"][i];
        let inline = format!("<!doctype html><svg width='300' height='300' viewBox='{vb}'>{shapes}</svg>");
        run(page(item, inline), &format!("svgbigvb{i}"));
        run(svg_image_case(item, format!("<svg {SVG_NS} width='300' height='300' viewBox='{vb}'>{shapes}</svg>").into_bytes()), &format!("svgbigvbimg{i}"));
    }
}

#[test]
fn svg_malformed_xml() {
    let bad: Vec<(&'static str, Vec<u8>)> = vec![
        ("svg unterminated tags", format!("<svg {SVG_NS}>{}", "<g><rect width='1' ".repeat(n(200_000))).into_bytes()),
        ("svg a million open brackets", "<".repeat(n(1_000_000)).into_bytes()),
        ("svg unmatched close tags", format!("<svg {SVG_NS}>{}</svg>", "</g>".repeat(n(200_000))).into_bytes()),
        ("svg entity soup", format!("<svg {SVG_NS}><title>{}</title></svg>", "&#x110000;&bogus;&amp".repeat(n(100_000))).into_bytes()),
        ("svg binary garbage", (0..n(2_000_000)).map(|i| (i * 31 % 251) as u8).collect()),
    ];
    for (i, (item, file)) in bad.into_iter().enumerate() {
        run(svg_image_case(item, file), &format!("svgbad{i}"));
    }
}

#[test]
fn svg_20000_inline_icons() {
    let icon = "<svg width='16' height='16' viewBox='0 0 16 16'><path d='M8 0a8 8 0 100 16A8 8 0 008 0zm1 12H7V7h2z' fill='currentColor'/></svg>";
    run(page("svg 20000 inline icons", format!("<!doctype html><body>{}", icon.repeat(n(20_000)))), "svgicons");
}

// ---- scroll containers and sticky boxes (step 2e-2) -------------------------------------------------------------

#[test]
fn scroll_containers_20000_siblings() {
    let one = "<div style='overflow:auto;width:40px;height:20px'><div style='width:80px;height:60px;background:#08f'><a href=x>t</a></div></div>";
    run(page("20000 overflow:auto boxes", format!("<!doctype html><body>{}", one.repeat(n(20_000)))), "scrollers20000");
}

#[test]
fn scroll_containers_nested_300_deep_with_positioned_and_floated_descendants() {
    let depth = n(300);
    let level = "<div style='overflow:auto;height:300px;position:relative'><div style='position:absolute;left:3px;top:3px;width:9px;height:9px;background:#f00'></div><div style='float:left;width:5px;height:5px;background:#0a0'></div><div style='position:sticky;top:1px;height:4px;background:#00f'></div>";
    run(page("300 nested scroll containers with abs, float and sticky children", format!("<!doctype html><body style='margin:0'>{}x{}", level.repeat(depth), "</div>".repeat(depth))), "scrollers_nested");
}

#[test]
fn sticky_boxes_10000() {
    let one = "<div style='height:30px'><div style='position:sticky;top:0;bottom:0;left:0;height:10px;background:#0f0'>s</div></div>";
    run(page("10000 sticky boxes", format!("<!doctype html><body>{}", one.repeat(n(10_000)))), "sticky10000");
}

// ---- review fixes after 2e + 2f ----------------------------------------------------------------------------------------

#[test]
fn svg_namespace_declarations_255_deep_by_250() {
    // 255 open elements, each declaring 250 prefixes: 64k bindings in scope at once. Every lookup used to walk them all
    // (about 110 s of CPU for this file); now the file is refused once 1024 are in scope.
    let decls: String = (0..250).fold(String::new(), |mut s, k| {
        write!(s, " xmlns:p{k}='u{k}'").unwrap();
        s
    });
    let depth = 255;
    let body = format!("{}<rect width='50' height='50' fill='#f00' p0:a='1'/>{}", format!("<g{decls} p1:b='1'>").repeat(depth), "</g>".repeat(depth));
    run(svg_image_case("svg 255 x 250 namespace declarations", svg_file(&body)), "svgnsflood");
}

#[test]
fn svg_many_prefixed_attributes_under_1024_live_namespace_declarations() {
    // Just under the limit (1024): 60 levels of 16 prefixes, then 20000 elements with prefixed attributes at the bottom.
    let decls = |lv: usize| (0..16).fold(String::new(), |mut s, k| {
        write!(s, " xmlns:p{lv}_{k}='u'").unwrap();
        s
    });
    let mut body = String::new();
    for lv in 0..60 {
        write!(body, "<g{}>", decls(lv)).unwrap();
    }
    body.push_str(&"<rect width='1' height='1' p0_0:a='1' p30_5:b='2' p59_15:c='3' p3_3:d='4'/>".repeat(n(20_000)));
    body.push_str(&"</g>".repeat(60));
    run(svg_image_case("svg 960 live namespace declarations, 80k prefixed attributes", svg_file(&body)), "svgnslookups");
}

fn svg_texts(count: usize, chars: usize) -> String {
    let mut seed = 7u64;
    let mut s = String::new();
    for i in 0..count {
        write!(s, "<text x='0' y='{}' font-size='10'>", 10 + i * 10).unwrap();
        for _ in 0..chars {
            let c = lcg(&mut seed) % 27;
            s.push(if c == 26 { ' ' } else { (b'a' + c) as char });
        }
        s.push_str("</text>");
    }
    s
}

#[test]
fn svg_40_texts_of_100k_characters_image() {
    // 4 MB of text: shaping and drawing every glyph took 6.6 s, on the UI thread. 200k glyphs are drawn, the rest is not.
    run(svg_image_case("svg 40 x 100k characters of text (image)", svg_file(&svg_texts(40, n(100_000)))), "svgtextimage");
}

#[test]
fn svg_40_texts_of_100k_characters_inline() {
    run(page("svg 40 x 100k characters of text (inline)", format!("<!doctype html><svg width='300' height='300' viewBox='0 0 100 100'>{}</svg>", svg_texts(40, n(100_000)))), "svgtextinline");
}
