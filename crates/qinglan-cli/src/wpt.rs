//! `qinglan-cli wpt`: the reftests of the Web Platform Tests (step 2a-1, docs/L2-TASK.md "WPT reftest").
//!
//! ```text
//! qinglan-cli wpt [--root tests/wpt] [--update] <dir>...   run the reftests under <root>/<dir>, write RESULTS-<dir>.md
//! qinglan-cli wpt [--root tests/wpt] --check               run the tests listed in <root>/passing.txt; exit 1 if one fails
//! ```
//!
//! A test is a file with `<link rel=match|mismatch href=...>`. The test and its reference are rendered at 800 x 600
//! (the WPT default) with the test platform (every glyph a 1em square, which is what the Ahem font looks like), and the
//! pixels are compared, within `<meta name=fuzzy>` when the test has one. The tree under `<root>` is served as
//! `http://wpt.test/`, so root-relative URLs (`/fonts/ahem.css`, `/css/support/x.png`) work as they do on a WPT server.
//! Tests that need JS, wait for `reftest-wait` or are for print are left out and counted as skipped.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use std::time::{Duration, Instant};

use qinglan_engine::css::{StyleOptions, style_document_with};
use qinglan_engine::html::atoms::a;
use qinglan_engine::html::dom::{Document, NodeId};
use qinglan_engine::html::treebuilder::parse_document;
use qinglan_engine::layout::image::ImageSizes;
use qinglan_engine::layout::{LayoutOptions, layout_document_with};
use qinglan_engine::net::blocking::{self, OpenResult};
use qinglan_engine::net::loader::Loader;
use qinglan_engine::net::{NetError, Request, Response, Transport};
use qinglan_engine::paint::build_display_list;
use qinglan_engine::paint::images::{Band, ImageStore};
use qinglan_engine::paint::raster::{Frame, View, render_frame};
use qinglan_engine::platform::{Platform, TestPlatform};
use qinglan_win::WinPlatform;
use qinglan_engine::url::Url;
use qinglan_win::WicDecoder;

use crate::render::{Ctx, MAX_ROUNDS, fetch_images};

const W: u32 = 800;
const H: u32 = 600;
const HOST: &str = "http://wpt.test";
const OPEN_TIMEOUT: Duration = Duration::from_secs(20);
/// One test (both renders and the comparison) may take this long; a hang kills the run.
const TEST_LIMIT: Duration = Duration::from_secs(60);

/// XML comments are not content; in HTML a `<style>` keeps the text of a comment (`<!-- ... -->` around a remark in the
/// middle of a style sheet would swallow the rule after it).
fn strip_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut rest = src;
    while let Some(a) = rest.find("<!--") {
        let (head, tail) = rest.split_at(a);
        out.push_str(head);
        match tail.find("-->") {
            Some(b) => rest = tail.get(b + 3..).unwrap_or(""),
            None => {
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    out
}

/// XML has `<div/>`; HTML ignores the slash, so an XHTML test would nest what follows. Every self-closed element that is
/// not a void element becomes `<div></div>`.
fn open_self_closed(src: &str) -> String {
    const VOID: [&str; 16] = ["br", "img", "hr", "input", "meta", "link", "col", "area", "base", "param", "wbr", "source", "track", "embed", "basefont", "frame"];
    let mut out = String::with_capacity(src.len() + 64);
    let mut rest = src;
    while let Some(i) = rest.find("/>") {
        let (head, tail) = rest.split_at(i);
        let tag = head.rfind('<').map(|lt| head.get(lt + 1..).unwrap_or("").chars().take_while(char::is_ascii_alphanumeric).collect::<String>().to_ascii_lowercase());
        let closing = head.rfind('>').zip(head.rfind('<')).is_some_and(|(gt, lt)| gt > lt);
        match tag {
            Some(t) if !t.is_empty() && !closing && !VOID.contains(&t.as_str()) && !head.get(head.rfind('<').unwrap_or(0)..).is_some_and(|h| h.starts_with("<!") || h.starts_with("<?")) => {
                out.push_str(head);
                out.push_str("></");
                out.push_str(&t);
                out.push('>');
            }
            _ => {
                out.push_str(head);
                out.push_str("/>");
            }
        }
        rest = tail.get(2..).unwrap_or("");
    }
    out.push_str(rest);
    out
}

/// Serves the files under a directory as the paths of `http://wpt.test/`.
struct WptTransport {
    root: PathBuf,
}

fn mime_of(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
        // XHTML tests are parsed as HTML (the engine has no XML parser).
        "html" | "htm" | "xht" | "xhtml" => "text/html; charset=utf-8",
        "css" => "text/css",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "ttf" => "font/ttf",
        _ => "application/octet-stream",
    }
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while let Some(&c) = b.get(i) {
        let hex = |k: usize| b.get(k).and_then(|&d| (d as char).to_digit(16));
        if c == b'%'
            && let (Some(h), Some(l)) = (hex(i + 1), hex(i + 2))
        {
            out.push((h * 16 + l) as u8);
            i += 3;
        } else {
            out.push(c);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

impl Transport for WptTransport {
    fn fetch(&self, req: &Request) -> Result<Response, NetError> {
        let rel = percent_decode(req.url.pathname().trim_start_matches('/'));
        let mut file = self.root.clone();
        let mut ok = true;
        for part in rel.split('/') {
            if part == ".." || part.contains(['\\', ':']) {
                ok = false;
            }
            file.push(part);
        }
        let mut found = if ok { std::fs::read(&file).ok() } else { None };
        // `<link rel=match href="about:blank">`: the blank page, resolved against the test's address.
        if rel.ends_with("about:blank") {
            return Ok(Response { url: req.url.clone(), status: 200, headers: vec![("Content-Type".to_string(), "text/html".to_string())], body: b"<!doctype html>".to_vec() });
        }
        // XHTML tests are parsed as HTML here (the engine has no XML parser); what XML would have unwrapped is unwrapped
        // by hand: `<style><![CDATA[ ... ]]></style>`.
        if file.extension().is_some_and(|e| e == "xht" || e == "xhtml")
            && let Some(body) = &found
        {
            let text = String::from_utf8_lossy(body).replace("<![CDATA[", "").replace("]]>", "");
            found = Some(open_self_closed(&strip_comments(&text)).into_bytes());
        }
        let (status, body, headers) = match found {
            Some(body) => (200, body, vec![("Content-Type".to_string(), mime_of(&file).to_string())]),
            None => (404, Vec::new(), vec![("Content-Type".to_string(), "text/plain".to_string())]),
        };
        Ok(Response { url: req.url.clone(), status, headers, body })
    }
}

/// What a test file says about itself.
#[derive(Default)]
struct TestInfo {
    /// `(is_match, href)` of every `<link rel=match|mismatch>`.
    refs: Vec<(bool, String)>,
    /// `maxDifference` and `totalPixels` upper bounds of `<meta name=fuzzy>`.
    fuzzy: Option<(u32, u32)>,
    script: bool,
    wait_or_print: bool,
}

fn attr_of(doc: &Document, id: NodeId, name: &str) -> Option<String> {
    let e = doc.element(id)?;
    e.attrs.iter().find(|x| doc.atoms.get(x.name) == name).map(|x| doc.value(x).to_string())
}

/// The numbers after `maxDifference=` / `totalPixels=` (or the bare `a-b;c-d` form): the upper bound of each range.
fn parse_fuzzy(content: &str) -> Option<(u32, u32)> {
    let upper = |s: &str| -> Option<u32> { s.rsplit('-').next()?.trim().parse().ok() };
    let body = content.rsplit_once(':').map_or(content, |(_, rest)| rest);
    let (mut diff, mut total) = (None, None);
    for (i, part) in body.split(';').enumerate() {
        let part = part.trim();
        if let Some(v) = part.strip_prefix("maxDifference=") {
            diff = upper(v);
        } else if let Some(v) = part.strip_prefix("totalPixels=") {
            total = upper(v);
        } else if i == 0 {
            diff = upper(part);
        } else if i == 1 {
            total = upper(part);
        }
    }
    Some((diff.unwrap_or(0), total.unwrap_or(0)))
}

fn scan(text: String) -> TestInfo {
    let doc = parse_document(text);
    let mut info = TestInfo::default();
    for id in 0..doc.nodes.len() as NodeId {
        let Some(e) = doc.element(id) else { continue };
        if e.name == a::SCRIPT {
            info.script = true;
        } else if e.name == a::LINK {
            let rel = attr_of(&doc, id, "rel").unwrap_or_default().to_ascii_lowercase();
            if (rel == "match" || rel == "mismatch")
                && let Some(href) = attr_of(&doc, id, "href")
            {
                info.refs.push((rel == "match", href));
            }
        } else if e.name == a::META && attr_of(&doc, id, "name").is_some_and(|n| n == "fuzzy") {
            info.fuzzy = attr_of(&doc, id, "content").and_then(|c| parse_fuzzy(&c));
        } else if e.name == a::HTML {
            let class = attr_of(&doc, id, "class").unwrap_or_default();
            info.wait_or_print = class.split_whitespace().any(|c| c == "reftest-wait" || c.starts_with("reftest-paged"));
        }
    }
    info
}

/// `--win`: render with the Windows platform (DirectWrite) instead of the test platform. Needed for what depends on real
/// fonts, the `@font-face` tests; its passing list is `passing-win.txt`.
static WIN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn win_mode() -> bool {
    WIN.load(std::sync::atomic::Ordering::Relaxed)
}

struct Runner {
    root: PathBuf,
    win: Option<WinPlatform>,
    loader: Loader,
    frames: HashMap<String, Arc<Frame>>,
}

impl Runner {
    fn new(root: &Path) -> Runner {
        Runner { root: root.to_path_buf(), win: if win_mode() { WinPlatform::new() } else { None }, loader: Loader::without_wake(Arc::new(WptTransport { root: root.to_path_buf() })), frames: HashMap::new() }
    }

    fn source(&self, rel: &str) -> String {
        let bytes = std::fs::read(self.root.join(rel)).unwrap_or_default();
        String::from_utf8_lossy(&bytes).into_owned()
    }

    /// The page at `url` painted at 800 x 600.
    fn render(&mut self, url: &Url) -> Result<Arc<Frame>, String> {
        let key = url.href();
        if let Some(f) = self.frames.get(&key) {
            return Ok(f.clone());
        }
        let platform: &dyn Platform = match &self.win {
            Some(w) => w,
            None => &TestPlatform,
        };
        let mut opened = match blocking::open(&mut self.loader, url, platform, OPEN_TIMEOUT) {
            OpenResult::Page(p) if p.status < 400 => p,
            OpenResult::Page(p) => return Err(format!("{} answered HTTP {}", url.pathname(), p.status)),
            OpenResult::Download { mime, .. } => return Err(format!("{} is {mime}, not a page", url.pathname())),
            OpenResult::Failed(why) => return Err(format!("cannot open {}: {why}", url.pathname())),
        };
        let mut res = blocking::resources_for(&opened);
        let sopts = StyleOptions { viewport_w: W as f32, viewport_h: H as f32, ..StyleOptions::default() };
        res.start_sheets(&opened.doc, &mut self.loader, &sopts);
        blocking::load_sheets(&mut self.loader, &mut res, platform, Duration::from_secs(10));
        let doc_url = opened.url.clone();
        let styled = style_document_with(&mut opened.doc, Some(&doc_url), &mut res, &sopts, Some(platform));
        blocking::load_fonts(&mut self.loader, &mut res, &opened.doc, &styled, platform, Duration::from_secs(10));
        // `ex` and `ch` were measured before the fonts were there.
        let styled = if res.fonts.any_ready() { style_document_with(&mut opened.doc, Some(&doc_url), &mut res, &sopts, Some(platform)) } else { styled };
        let lopts = LayoutOptions { viewport_w: W as f32, viewport_h: H as f32, ..LayoutOptions::default() };
        let mut sizes = ImageSizes::new();
        let mut store = ImageStore::new();
        let band = Band::new(0.0, H as f32);
        let mut round = 0;
        let list = loop {
            let layout = layout_document_with(&opened.doc, &styled.tree, platform, &lopts, &sizes);
            let list = build_display_list(&opened.doc, &layout, platform);
            if round >= MAX_ROUNDS {
                break list;
            }
            let mut cx = Ctx { opened: &mut opened, res: &mut res, loader: &mut self.loader, platform, win: None };
            let mut changed = fetch_images(&mut cx, &layout, &list, band, false, &mut sizes, &mut store);
            round += 1;
            if !changed {
                let loader = &mut self.loader;
                store.prepare(&list, band, 1.0, &WicDecoder, &mut |u| loader.cache_mut().get(u));
                for n in store.take_decode_failed() {
                    changed |= sizes.set_broken(n);
                }
            }
            if !changed {
                break list;
            }
        };
        let mut frame = Frame::new(W, H);
        render_frame(&list, platform, &mut store, View { scroll_x: 0.0, scroll_y: 0.0, dpr: 1.0 }, &mut frame);
        let frame = Arc::new(frame);
        if self.frames.len() > 24 {
            self.frames.clear();
        }
        self.frames.insert(key, frame.clone());
        Ok(frame)
    }

    fn url_of(&self, rel: &str) -> Option<Url> {
        Url::parse(&format!("{HOST}/{rel}"), None).ok()
    }

    fn render_rel(&mut self, rel: &str) -> Result<Arc<Frame>, String> {
        let url = self.url_of(rel).ok_or_else(|| format!("bad path {rel}"))?;
        self.render(&url)
    }

    /// Does `ref_rel` match `test`? A reference with `rel=match` links of its own must match those too (the chain).
    fn matches(&mut self, test: &Frame, ref_rel: &str, fuzzy: Option<(u32, u32)>, depth: u32) -> Result<(), String> {
        let rf = self.render_rel(ref_rel)?;
        compare(test, &rf, fuzzy)?;
        if depth < 3 {
            let info = scan(self.source(ref_rel));
            for (is_match, href) in info.refs {
                if is_match {
                    let next = resolve(ref_rel, &href);
                    self.matches(test, &next, fuzzy, depth + 1).map_err(|e| format!("reference {ref_rel} is not like its own reference: {e}"))?;
                }
            }
        }
        Ok(())
    }

    /// `Ok(())` when the test passes; `Err(reason)` when it does not.
    fn run(&mut self, rel: &str, info: &TestInfo) -> Result<(), String> {
        let test = self.render_rel(rel)?;
        let mut first_err: Option<String> = None;
        let mut any_match = false;
        let mut has_match = false;
        for (is_match, href) in &info.refs {
            let r = resolve(rel, href);
            if *is_match {
                has_match = true;
                match self.matches(&test, &r, info.fuzzy, 0) {
                    Ok(()) => any_match = true,
                    Err(e) => {
                        first_err.get_or_insert(e);
                    }
                }
            } else {
                let rf = self.render_rel(&r)?;
                if compare(&test, &rf, info.fuzzy).is_ok() {
                    return Err(format!("renders like {r}, which it must not"));
                }
            }
        }
        if has_match && !any_match {
            return Err(first_err.unwrap_or_default());
        }
        Ok(())
    }
}

/// The path (relative to the root) `href` names from the test `base`.
fn resolve(base: &str, href: &str) -> String {
    let href = href.split(['#', '?']).next().unwrap_or("");
    let mut parts: Vec<&str> = if href.starts_with('/') {
        Vec::new()
    } else {
        let mut p: Vec<&str> = base.split('/').collect();
        p.pop();
        p
    };
    for seg in href.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            s => parts.push(s),
        }
    }
    parts.join("/")
}

/// Pixel comparison (the alpha channel is not looked at: the canvas is opaque). `fuzzy` = (largest channel difference
/// allowed, number of pixels allowed to differ).
fn compare(x: &Frame, y: &Frame, fuzzy: Option<(u32, u32)>) -> Result<(), String> {
    if (x.width, x.height) != (y.width, y.height) {
        return Err("frames of different sizes".into());
    }
    let (max_diff, max_pixels) = fuzzy.unwrap_or((0, 0));
    let (mut count, mut worst) = (0u32, 0u32);
    let mut first: Option<(u32, u32, [u8; 3], [u8; 3])> = None;
    for (i, (p, q)) in x.data.as_chunks::<4>().0.iter().zip(y.data.as_chunks::<4>().0).enumerate() {
        let d = (0..3).map(|k| u32::from(p[k].abs_diff(q[k]))).max().unwrap_or(0);
        if d > 0 {
            count += 1;
            worst = worst.max(d);
            if first.is_none() {
                first = Some((i as u32 % x.width, i as u32 / x.width, [p[2], p[1], p[0]], [q[2], q[1], q[0]]));
            }
        }
    }
    if count == 0 || (worst <= max_diff && count <= max_pixels) {
        return Ok(());
    }
    let (px, py, got, want) = first.unwrap_or_default();
    Err(format!(
        "{count} pixels differ (largest difference {worst}); first at ({px},{py}): #{:02x}{:02x}{:02x}, the reference has #{:02x}{:02x}{:02x}",
        got[0], got[1], got[2], want[0], want[1], want[2]
    ))
}

/// Why a failing test is expected to fail: features of later steps found in the test or its references, or nothing
/// ("bug" until someone looks at it).
fn classify(src: &str) -> Option<(&'static str, &'static str)> {
    let s = src.to_ascii_lowercase().replace(": ", ":");
    let has = |ns: &[&str]| ns.iter().any(|n| s.contains(n));
    if has(&["grid-lanes", "masonry", "subgrid"]) {
        return Some(("none", "subgrid and masonry (grid-lanes) are not planned"));
    }
    if has(&["translate:", "rotate:", "scale:", "perspective", "preserve-3d", "transform-style", "backface"]) {
        return Some(("none", "individual transform properties and real 3D transforms are not planned"));
    }
    if has(&["mix-blend", "background-blend", "cross-fade(", "image-set(", "element(", "paint(", "-webkit-gradient", "-moz-", "border-image", "color-mix(", "light-dark(", "color(", "currentcolor/", "sibling-index", "anchor("]) {
        return Some(("none", "a feature layer 2 does not do (blend modes, border-image, cross-fade, image-set, element(), color-mix ...)"));
    }
    if has(&["<rtc", "ruby-align", "ruby-merge", "ruby-overhang"]) {
        return Some(("2d", "complex ruby (rtc, ruby-align, overhang)"));
    }
    if has(&["<svg", "position:sticky", "overflow:auto", "overflow:scroll", "<input", "<button", "<select", "<textarea"]) {
        return Some(("2e", "svg, scrolling, sticky or form controls"));
    }
    // `columns:` as a property of its own, not the end of `grid-template-columns:`.
    let columns = s.match_indices("columns:").any(|(i, _)| s[..i].chars().next_back().is_none_or(|c| !(c.is_ascii_alphanumeric() || c == '-')));
    if columns || has(&["text-wrap", "flex-wrap:balance", "aspect-ratio", "column-count", "column-width", "column-rule", "column-fill", "column-span", "shape-outside", ":has(", "@container", "animation", "transition", "clip-path", "mask", "filter:"]) {
        return Some(("none", "a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...)"));
    }
    None
}

/// `path<TAB>class<TAB>reason` lines: what was found out by looking at a failure (overrides `classify`).
fn read_notes(root: &Path) -> HashMap<String, (String, String)> {
    let mut m = HashMap::new();
    for line in std::fs::read_to_string(root.join("notes.txt")).unwrap_or_default().lines() {
        let mut f = line.splitn(3, '\t');
        if let (Some(p), Some(c), Some(r)) = (f.next(), f.next(), f.next()) {
            m.insert(p.to_string(), (c.to_string(), r.to_string()));
        }
    }
    m
}

fn walk(dir: &Path, rel: &str, out: &mut Vec<String>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<_> = rd.filter_map(Result::ok).collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let name = e.file_name().to_string_lossy().into_owned();
        let path = e.path();
        let child = format!("{rel}/{name}");
        if path.is_dir() {
            if !matches!(name.as_str(), "support" | "reference" | "references" | "crashtests" | "resources" | "reftests" | "tools") {
                walk(&path, &child, out);
            }
            continue;
        }
        let lower = name.to_ascii_lowercase();
        let page = [".html", ".htm", ".xht", ".xhtml"].iter().any(|x| lower.ends_with(x));
        let is_ref = ["-ref.", "-ref2.", "-ref3.", "-notref.", "-notref2.", "_ref.", "-ref-"].iter().any(|x| lower.contains(x));
        if page && !is_ref {
            out.push(child);
        }
    }
}

enum Outcome {
    Pass,
    Fail(String),
    Skip(&'static str),
}

fn run_one(r: &mut Runner, rel: &str) -> Outcome {
    let info = scan(r.source(rel));
    if info.refs.is_empty() {
        return Outcome::Skip("no reftest");
    }
    if info.script {
        return Outcome::Skip("needs JS");
    }
    if info.wait_or_print {
        return Outcome::Skip("reftest-wait or print");
    }
    match r.run(rel, &info) {
        Ok(()) => Outcome::Pass,
        Err(e) => Outcome::Fail(e),
    }
}

/// Stop the run if one test hangs.
fn watchdog() -> std::sync::mpsc::Sender<()> {
    let (tx, rx) = std::sync::mpsc::channel::<()>();
    std::thread::spawn(move || {
        loop {
            match rx.recv_timeout(TEST_LIMIT) {
                Ok(()) => {}
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return,
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    eprintln!("error: a test took more than {} s; giving up", TEST_LIMIT.as_secs());
                    std::process::exit(2);
                }
            }
        }
    });
    tx
}

pub fn main(args: &[String]) -> ExitCode {
    let mut root = PathBuf::from("tests/wpt");
    let (mut update, mut check) = (false, false);
    let mut dirs: Vec<String> = Vec::new();
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--root" => match it.next() {
                Some(v) => root = PathBuf::from(v),
                None => return usage(),
            },
            "--update" => update = true,
            "--check" => check = true,
            "--win" => WIN.store(true, std::sync::atomic::Ordering::Relaxed),
            d if !d.starts_with('-') => dirs.push(d.trim_matches('/').to_string()),
            _ => return usage(),
        }
    }
    if dirs.is_empty() && !check {
        return usage();
    }
    // The layout code recurses; give it room (a debug build's main thread has 1 MB).
    let handle = std::thread::Builder::new().stack_size(64 << 20).spawn(move || if check { run_check(&root) } else { run_dirs(&root, &dirs, update) });
    match handle.map(|h| h.join()) {
        Ok(Ok(code)) => code,
        _ => {
            eprintln!("error: the WPT run panicked");
            ExitCode::FAILURE
        }
    }
}

fn usage() -> ExitCode {
    eprintln!("usage: qinglan-cli wpt [--root tests/wpt] [--update] [--win] <dir>... | --check");
    ExitCode::FAILURE
}

fn run_check(root: &Path) -> ExitCode {
    let mut failed = 0;
    let mut total = 0;
    // The test-platform list, then the Windows-platform list (the @font-face tests) when there is one.
    for (name, win) in [("passing.txt", false), ("passing-win.txt", true)] {
        let list = std::fs::read_to_string(root.join(name)).unwrap_or_default();
        let tests: Vec<&str> = list.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')).collect();
        if tests.is_empty() {
            continue;
        }
        WIN.store(win, std::sync::atomic::Ordering::Relaxed);
        let mut r = Runner::new(root);
        let dog = watchdog();
        total += tests.len();
        for t in &tests {
            let _ = dog.send(());
            match run_one(&mut r, t) {
                Outcome::Pass => {}
                Outcome::Fail(why) => {
                    failed += 1;
                    println!("FAIL {t}: {why}");
                }
                Outcome::Skip(why) => {
                    failed += 1;
                    println!("FAIL {t}: not a test any more ({why})");
                }
            }
        }
    }
    println!("wpt: {} of {} listed tests pass", total - failed, total);
    if failed == 0 { ExitCode::SUCCESS } else { ExitCode::FAILURE }
}

fn run_dirs(root: &Path, dirs: &[String], update: bool) -> ExitCode {
    let mut r = Runner::new(root);
    let notes = read_notes(root);
    let dog = watchdog();
    let mut passing_new: Vec<String> = Vec::new();
    for dir in dirs {
        let mut tests = Vec::new();
        walk(&root.join(dir), dir, &mut tests);
        let started = Instant::now();
        let (mut pass, mut skipped) = (Vec::new(), HashMap::<&str, u32>::new());
        let mut fails: Vec<(String, String, String)> = Vec::new();
        for t in &tests {
            let _ = dog.send(());
            match run_one(&mut r, t) {
                Outcome::Pass => pass.push(t.clone()),
                Outcome::Skip(why) => *skipped.entry(why).or_insert(0) += 1,
                Outcome::Fail(why) => {
                    let (class, reason) = match notes.get(t) {
                        Some((c, why_not)) => (c.clone(), why_not.clone()),
                        None => {
                            let mut src = r.source(t);
                            for (_, href) in &scan(src.clone()).refs {
                                src.push_str(&r.source(&resolve(t, href)));
                            }
                            match classify(&src) {
                                Some(("none", what)) => ("not planned".to_string(), format!("uses {what}")),
                                Some((step, what)) => (format!("step {step}"), format!("uses {what}")),
                                None => ("bug".to_string(), String::new()),
                            }
                        }
                    };
                    let reason = if reason.is_empty() { why } else { format!("{reason} ({why})") };
                    fails.push((t.clone(), class, reason));
                }
            }
        }
        let judged = pass.len() + fails.len();
        let bugs = fails.iter().filter(|f| f.1.starts_with("bug")).count();
        let pct = if judged == 0 { 0.0 } else { 100.0 * pass.len() as f64 / judged as f64 };
        let mut md = format!("# WPT `{dir}`\n\n");
        md.push_str(&format!("{}，{W}×{H}，`qinglan-cli wpt {dir}`，用时 {:.1} 秒。\n\n", if win_mode() { "Windows 平台（DirectWrite，真字体）" } else { "测试平台（每个字 1em 见方）" }, started.elapsed().as_secs_f64()));
        md.push_str(&format!("- 通过 **{} / {judged}**（{pct:.1}%）\n- bug 类失败：**{bugs}**\n", pass.len()));
        let mut skip_list: Vec<_> = skipped.iter().collect();
        skip_list.sort();
        let skip_text: Vec<String> = skip_list.iter().map(|(k, v)| format!("{k} {v}")).collect();
        md.push_str(&format!("- 跳过：{}\n\n", if skip_text.is_empty() { "无".to_string() } else { skip_text.join("，") }));
        let mut by_class: HashMap<&str, u32> = HashMap::new();
        for f in &fails {
            *by_class.entry(f.1.as_str()).or_insert(0) += 1;
        }
        let mut classes: Vec<_> = by_class.into_iter().collect();
        classes.sort();
        md.push_str(&format!("失败分类：{}\n\n", classes.iter().map(|(c, n)| format!("{c} {n}")).collect::<Vec<_>>().join("，")));
        md.push_str("## 失败\n\n| 测试 | 类别 | 说明 |\n|---|---|---|\n");
        for (t, c, why) in &fails {
            md.push_str(&format!("| {} | {c} | {} |\n", t.trim_start_matches(&format!("{dir}/")), why.replace('|', "/").replace('\n', " ")));
        }
        md.push_str("\n## 通过\n\n");
        for t in &pass {
            md.push_str(&format!("- {}\n", t.trim_start_matches(&format!("{dir}/"))));
        }
        let out = root.join(format!("RESULTS-{}{}.md", dir.replace('/', "-"), if win_mode() { "-win" } else { "" }));
        if let Err(e) = std::fs::write(&out, md) {
            eprintln!("error: cannot write {}: {e}", out.display());
            return ExitCode::FAILURE;
        }
        println!("{dir}: {} / {judged} pass ({pct:.1}%), {bugs} bug-class failures, skipped {}", pass.len(), skip_text.join(", "));
        passing_new.extend(pass);
        if update {
            let path = root.join(if win_mode() { "passing-win.txt" } else { "passing.txt" });
            let old = std::fs::read_to_string(&path).unwrap_or_default();
            let mut keep: Vec<String> = old.lines().filter(|l| !l.is_empty() && !l.starts_with(&format!("{dir}/"))).map(str::to_string).collect();
            keep.append(&mut passing_new);
            keep.sort();
            keep.dedup();
            if let Err(e) = std::fs::write(&path, keep.join("\n") + "\n") {
                eprintln!("error: cannot write {}: {e}", path.display());
                return ExitCode::FAILURE;
            }
        }
    }
    ExitCode::SUCCESS
}
