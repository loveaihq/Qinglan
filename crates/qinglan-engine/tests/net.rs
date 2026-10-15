//! Network policy and plumbing without a network (step 1c-1): schemes, mixed content, `Referer`,
//! limits, cache, discovery of style sheets and images, record and replay. The transport is a fake.

#![allow(clippy::indexing_slicing, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use qinglan_engine::css::{ResourceLoader, StyleOptions};
use qinglan_engine::encoding::Encoding;
use qinglan_engine::html::treebuilder::parse_document;
use qinglan_engine::layout::image::{ImageRequest, ImageSizes, ImageState};
use qinglan_engine::net::blocking::{self, OpenResult};
use qinglan_engine::net::content::ContentKind;
use qinglan_engine::net::loader::{LoadRequest, Loader};
use qinglan_engine::net::page::PageResources;
use qinglan_engine::net::policy::{self, Plan, Refusal};
use qinglan_engine::net::replay::{INDEX_FILE, Recorder, ReplayTransport};
use qinglan_engine::net::{NetError, Request, ResourceKind, Response, Transport};
use qinglan_engine::platform::image::{DecodedImage, ImageDecoder, ImageError, ImageInfo};
use qinglan_engine::platform::{BasicPlatform, DecodedCache};
use qinglan_engine::url::Url;

fn u(s: &str) -> Url {
    Url::parse(s, None).unwrap()
}

fn resp(url: &Url, status: u16, ct: Option<&str>, body: &[u8]) -> Response {
    Response {
        url: url.clone(),
        status,
        headers: ct.map(|c| vec![("Content-Type".to_string(), c.to_string())]).unwrap_or_default(),
        body: body.to_vec(),
    }
}

type Handler = Box<dyn Fn(&Request) -> Result<Response, NetError> + Send + Sync>;

/// A transport that records what it was asked and how many requests ran at once.
struct Fake {
    handler: Handler,
    calls: Mutex<Vec<Request>>,
    delay: Duration,
    active: AtomicUsize,
    peak: AtomicUsize,
    per_host: Mutex<HashMap<String, (usize, usize)>>,
}

impl Fake {
    fn new(delay_ms: u64, handler: impl Fn(&Request) -> Result<Response, NetError> + Send + Sync + 'static) -> Arc<Fake> {
        Arc::new(Fake {
            handler: Box::new(handler),
            calls: Mutex::new(Vec::new()),
            delay: Duration::from_millis(delay_ms),
            active: AtomicUsize::new(0),
            peak: AtomicUsize::new(0),
            per_host: Mutex::new(HashMap::new()),
        })
    }
    fn count(&self) -> usize {
        self.calls.lock().unwrap().len()
    }
    fn host_peak(&self) -> usize {
        self.per_host.lock().unwrap().values().map(|v| v.1).max().unwrap_or(0)
    }
}

impl Transport for Fake {
    fn fetch(&self, req: &Request) -> Result<Response, NetError> {
        self.calls.lock().unwrap().push(req.clone());
        let now = self.active.fetch_add(1, Ordering::SeqCst) + 1;
        self.peak.fetch_max(now, Ordering::SeqCst);
        let host = req.url.host_with_port();
        {
            let mut m = self.per_host.lock().unwrap();
            let e = m.entry(host.clone()).or_insert((0, 0));
            e.0 += 1;
            e.1 = e.1.max(e.0);
        }
        std::thread::sleep(self.delay);
        let r = (self.handler)(req);
        self.per_host.lock().unwrap().get_mut(&host).unwrap().0 -= 1;
        self.active.fetch_sub(1, Ordering::SeqCst);
        r
    }
}

fn ok_html(req: &Request) -> Result<Response, NetError> {
    Ok(resp(&req.url, 200, Some("text/html; charset=utf-8"), b"<p>hi"))
}

fn page_req(url: &str, kind: ResourceKind, page: Option<&str>) -> LoadRequest {
    LoadRequest { url: u(url), kind, page: page.map(u), use_cache: false }
}

fn drain(loader: &mut Loader, n: usize) -> Vec<qinglan_engine::net::loader::LoadEvent> {
    let mut out = Vec::new();
    let t0 = std::time::Instant::now();
    while out.len() < n && t0.elapsed() < Duration::from_secs(20) {
        out.extend(loader.wait(Duration::from_millis(200)));
    }
    out
}

#[test]
fn scheme_and_mixed_content_rules() {
    let https = u("https://a.example/page");
    let http = u("http://a.example/page");
    let file = u("file:///C:/x/page.html");
    let sub = ResourceKind::Image;
    let nav = ResourceKind::Document;
    // https page: no http subresources, but https and data are fine, and http *navigation* is allowed.
    assert_eq!(policy::plan(&u("http://x.example/i.png"), sub, Some(&https)), Err(Refusal::MixedContent));
    assert_eq!(policy::plan(&u("http://x.example/s.css"), ResourceKind::Style, Some(&https)), Err(Refusal::MixedContent));
    assert_eq!(policy::plan(&u("https://x.example/i.png"), sub, Some(&https)), Ok(Plan::Network));
    assert_eq!(policy::plan(&u("http://x.example/next"), nav, Some(&https)), Ok(Plan::Network));
    assert_eq!(policy::plan(&u("http://x.example/i.png"), sub, Some(&http)), Ok(Plan::Network));
    assert_eq!(policy::plan(&u("data:image/gif;base64,R0lGOD"), sub, Some(&https)), Ok(Plan::Data));
    // data: is never a page.
    assert_eq!(policy::plan(&u("data:text/html,hi"), nav, None), Err(Refusal::DataDocument));
    // file: only from the user; a local page may load local sub-resources, no other page may.
    assert_eq!(policy::plan(&file, nav, None), Ok(Plan::File));
    assert_eq!(policy::plan(&file, nav, Some(&https)), Err(Refusal::LocalFile));
    assert_eq!(policy::plan(&file, nav, Some(&file)), Err(Refusal::LocalFile), "links to file: are not followed");
    assert_eq!(policy::plan(&u("file:///C:/x/a.png"), sub, Some(&https)), Err(Refusal::LocalFile));
    assert_eq!(policy::plan(&u("file:///C:/x/a.png"), sub, Some(&http)), Err(Refusal::LocalFile));
    assert_eq!(policy::plan(&u("file:///C:/x/a.png"), sub, Some(&file)), Ok(Plan::File));
    // Everything else is not fetched.
    assert_eq!(policy::plan(&u("mailto:a@b.example"), nav, Some(&https)), Err(Refusal::Mailto));
    assert_eq!(policy::plan(&u("javascript:alert(1)"), nav, Some(&https)), Err(Refusal::Javascript));
    assert_eq!(policy::plan(&u("ftp://a.example/x"), nav, None), Err(Refusal::UnsupportedScheme("ftp".into())));
    assert!(matches!(policy::plan(&u("about:blank"), nav, None), Err(Refusal::UnsupportedScheme(_))));
}

#[test]
fn referer_is_strict_origin_when_cross_origin() {
    let r = |page: &str, target: &str| policy::referer(&u(page), &u(target));
    // Same origin: the whole address, without fragment and credentials.
    assert_eq!(r("https://user:pw@a.example/p/q?x=1#frag", "https://a.example/img.png").as_deref(), Some("https://a.example/p/q?x=1"));
    // Another origin (host, scheme or port): only the origin.
    assert_eq!(r("https://a.example/p/q?x=1", "https://b.example/i.png").as_deref(), Some("https://a.example/"));
    assert_eq!(r("https://a.example:8443/p", "https://a.example/i.png").as_deref(), Some("https://a.example:8443/"));
    assert_eq!(r("http://a.example/p", "https://a.example/i.png").as_deref(), Some("http://a.example/"));
    // https to http: nothing.
    assert_eq!(r("https://a.example/p", "http://a.example/i.png"), None);
    // Local and data pages send none.
    assert_eq!(r("file:///C:/x.html", "https://a.example/"), None);
    assert_eq!(r("https://a.example/p", "file:///C:/x.html"), None);
    // A very long address is cut to the origin.
    let long = format!("https://a.example/{}", "x".repeat(5000));
    assert_eq!(r(&long, "https://a.example/i.png").as_deref(), Some("https://a.example/"));
}

#[test]
fn loader_applies_policy_before_the_transport() {
    let fake = Fake::new(0, ok_html);
    let mut loader = Loader::without_wake(fake.clone());
    let page = "https://a.example/page#top";
    assert_eq!(loader.submit(page_req("http://x.example/i.png", ResourceKind::Image, Some(page))).unwrap_err(), Refusal::MixedContent);
    assert!(loader.submit(page_req("file:///C:/secret.txt", ResourceKind::Image, Some(page))).is_err());
    assert!(loader.submit(page_req("javascript:1", ResourceKind::Document, Some(page))).is_err());
    assert!(loader.submit(page_req("data:text/html,x", ResourceKind::Document, None)).is_err());
    assert_eq!(fake.count(), 0, "refused requests never reach the transport");

    // A data: sub-resource is answered without the transport.
    let t = loader.submit(page_req("data:text/css;base64,Ym9keXt9", ResourceKind::Style, Some(page))).unwrap();
    let ev = drain(&mut loader, 1);
    assert_eq!(ev.len(), 1);
    assert_eq!(ev[0].ticket, t);
    let r = ev[0].result.as_ref().unwrap();
    assert_eq!(r.body, b"body{}");
    assert_eq!(r.mime().as_deref(), Some("text/css"));
    assert_eq!(fake.count(), 0);
    // Broken and oversize data: URLs are errors, not panics.
    loader.submit(page_req("data:;base64,@@@", ResourceKind::Image, Some(page))).unwrap();
    assert!(drain(&mut loader, 1)[0].result.is_err());

    // A real request carries Accept and the Referer for the page (cross-origin: origin only), no fragment.
    loader.submit(page_req("https://cdn.example/s.css#x", ResourceKind::Style, Some(page))).unwrap();
    assert_eq!(drain(&mut loader, 1).len(), 1);
    let calls = fake.calls.lock().unwrap();
    let c = &calls[0];
    assert_eq!(c.url.href(), "https://cdn.example/s.css");
    let h = |n: &str| c.headers.iter().find(|(k, _)| k == n).map(|(_, v)| v.clone());
    assert_eq!(h("Referer").as_deref(), Some("https://a.example/"));
    assert!(h("Accept").unwrap().starts_with("text/css"));
    assert_eq!(c.max_bytes, 16 * 1024 * 1024);
}

#[test]
fn size_caps_by_type_and_a_transport_that_ignores_them() {
    let fake = Fake::new(0, |req| {
        // Pretend the transport forgot the cap: it hands back 17 MB.
        Ok(resp(&req.url, 200, Some("text/css"), &vec![b'a'; 17 * 1024 * 1024]))
    });
    let mut loader = Loader::without_wake(fake.clone());
    loader.submit(page_req("https://a.example/s.css", ResourceKind::Style, None)).unwrap();
    loader.submit(page_req("https://a.example/i.png", ResourceKind::Image, None)).unwrap();
    let evs = drain(&mut loader, 2);
    assert_eq!(evs.len(), 2);
    for e in &evs {
        match e.url.pathname().as_str() {
            "/s.css" => assert_eq!(e.result.as_ref().unwrap_err(), &NetError::TooLarge { limit: 16 * 1024 * 1024 }),
            _ => assert!(e.result.is_ok(), "17 MB is fine for an image (cap 32 MB)"),
        }
    }
    let caps: HashMap<ResourceKind, usize> = fake.calls.lock().unwrap().iter().map(|r| (r.kind, r.max_bytes)).collect();
    assert_eq!(caps[&ResourceKind::Style], 16 * 1024 * 1024);
    assert_eq!(caps[&ResourceKind::Image], 32 * 1024 * 1024);
}

#[test]
fn six_per_host_and_sixteen_in_all() {
    let fake = Fake::new(40, ok_html);
    let mut loader = Loader::without_wake(fake.clone());
    for i in 0..30 {
        loader.submit(page_req(&format!("https://one.example/{i}.png"), ResourceKind::Image, None)).unwrap();
    }
    assert_eq!(drain(&mut loader, 30).len(), 30);
    assert!(fake.host_peak() <= 6, "per-host peak {}", fake.host_peak());
    assert!(fake.host_peak() >= 2, "requests did run in parallel");

    let fake = Fake::new(40, ok_html);
    let mut loader = Loader::without_wake(fake.clone());
    for i in 0..60 {
        loader.submit(page_req(&format!("https://h{i}.example/x.png"), ResourceKind::Image, None)).unwrap();
    }
    assert_eq!(drain(&mut loader, 60).len(), 60);
    let peak = fake.peak.load(Ordering::SeqCst);
    assert!(peak <= 16, "total peak {peak}");
    assert!(peak >= 8, "total peak {peak}: requests did run in parallel");
}

#[test]
fn documents_and_styles_jump_the_image_queue() {
    let fake = Fake::new(60, ok_html);
    let mut loader = Loader::without_wake(fake.clone());
    // 16 slots are taken by slow image requests to different hosts; then images and a style sheet queue up.
    for i in 0..16 {
        loader.submit(page_req(&format!("https://h{i}.example/a.png"), ResourceKind::Image, None)).unwrap();
    }
    for i in 0..4 {
        loader.submit(page_req(&format!("https://z.example/late{i}.png"), ResourceKind::Image, None)).unwrap();
    }
    loader.submit(page_req("https://z.example/style.css", ResourceKind::Style, None)).unwrap();
    assert_eq!(drain(&mut loader, 21).len(), 21);
    let order: Vec<String> = fake.calls.lock().unwrap().iter().map(|r| r.url.pathname()).collect();
    let css = order.iter().position(|p| p == "/style.css").unwrap();
    let first_late = order.iter().position(|p| p.starts_with("/late")).unwrap();
    assert!(css < first_late, "style sheet starts before the queued images: {order:?}");
}

#[test]
fn cache_serves_history_and_respects_budget() {
    let fake = Fake::new(0, |req| {
        let size = match req.url.pathname().as_str() {
            p if p.starts_with("/big") => 900 * 1024,
            "/huge" => 3 * 512 * 1024,
            _ => 10,
        };
        let mut r = resp(&req.url, if req.url.pathname() == "/missing" { 404 } else { 200 }, Some("text/html"), &vec![b'x'; size]);
        if req.url.pathname() == "/nostore" {
            r.headers.push(("Cache-Control".into(), "private, No-Store".into()));
        }
        Ok(r)
    });
    let mut loader = Loader::without_wake(fake.clone());
    let get = |loader: &mut Loader, path: &str, use_cache: bool| {
        let mut r = page_req(&format!("https://a.example{path}"), ResourceKind::Document, None);
        r.use_cache = use_cache;
        loader.submit(r).unwrap();
        drain(loader, 1).remove(0)
    };
    assert!(!get(&mut loader, "/p", false).from_cache);
    assert_eq!(fake.count(), 1);
    assert!(get(&mut loader, "/p", true).from_cache, "history navigation is answered from memory");
    assert_eq!(fake.count(), 1);
    assert!(!get(&mut loader, "/p", false).from_cache, "a reload goes to the network");
    assert_eq!(fake.count(), 2);
    get(&mut loader, "/nostore", false);
    assert!(!get(&mut loader, "/nostore", true).from_cache, "no-store is not kept");
    get(&mut loader, "/missing", false);
    assert!(!get(&mut loader, "/missing", true).from_cache, "errors are not kept");
    get(&mut loader, "/huge", false);
    assert!(!get(&mut loader, "/huge", true).from_cache, "a response over 1 MB is not kept");
    for i in 0..12 {
        get(&mut loader, &format!("/big{i}"), false);
    }
    let bytes = loader.cache().bytes();
    assert!(bytes <= 8 * 1024 * 1024, "cache holds {bytes} bytes");
    assert!(bytes >= 6 * 1024 * 1024);
    assert!(get(&mut loader, "/big11", true).from_cache, "the newest big page is still there");
    assert!(!get(&mut loader, "/big0", true).from_cache, "the oldest was evicted");
}

#[test]
fn cancel_drops_results_and_wake_is_called() {
    let fake = Fake::new(100, ok_html);
    let woke = Arc::new(AtomicUsize::new(0));
    let w = woke.clone();
    let mut loader = Loader::new(fake, Arc::new(move || {
        w.fetch_add(1, Ordering::SeqCst);
    }));
    loader.submit(page_req("https://a.example/1", ResourceKind::Document, None)).unwrap();
    loader.cancel_all();
    assert!(drain_for(&mut loader, Duration::from_millis(400)).is_empty(), "a cancelled request delivers nothing");
    assert_eq!(loader.pending(), 0);
    // And the loader keeps working afterwards.
    loader.submit(page_req("https://a.example/2", ResourceKind::Document, None)).unwrap();
    let evs = drain(&mut loader, 1);
    assert_eq!(evs.len(), 1);
    assert!(woke.load(Ordering::SeqCst) >= 2, "the worker wakes the owner");
}

fn drain_for(loader: &mut Loader, d: Duration) -> Vec<qinglan_engine::net::loader::LoadEvent> {
    let t0 = std::time::Instant::now();
    let mut out = Vec::new();
    while t0.elapsed() < d {
        out.extend(loader.wait(Duration::from_millis(50)));
        std::thread::sleep(Duration::from_millis(20));
    }
    out
}

#[test]
fn opening_pages_classifies_the_response() {
    let routes: HashMap<&str, (&str, Vec<u8>)> = HashMap::from([
        ("/html", ("text/html; charset=utf-8", b"<title>T</title><p>hi".to_vec())),
        ("/text", ("text/plain; charset=utf-8", b"a < b & c".to_vec())),
        ("/pdf", ("application/pdf", b"%PDF-1.4".to_vec())),
        ("/img", ("image/png", b"\x89PNG\r\n\x1a\n".to_vec())),
    ]);
    let fake = Fake::new(0, move |req| {
        let (ct, body) = routes.get(req.url.pathname().as_str()).cloned().unwrap_or(("text/plain", b"?".to_vec()));
        Ok(resp(&req.url, 200, Some(ct), &body))
    });
    let mut loader = Loader::without_wake(fake);
    let open = |loader: &mut Loader, path: &str| blocking::open(loader, &u(&format!("https://a.example{path}")), &BasicPlatform, Duration::from_secs(10));
    match open(&mut loader, "/html") {
        OpenResult::Page(p) => assert_eq!(p.kind, ContentKind::Html),
        _ => panic!("html page"),
    }
    match open(&mut loader, "/text") {
        OpenResult::Page(p) => {
            assert_eq!(p.kind, ContentKind::PlainText);
            let dump = qinglan_engine::html::dump::dump_children(&p.doc, 0);
            assert!(dump.contains("<pre>") && dump.contains("a < b & c"), "{dump}");
        }
        _ => panic!("text page"),
    }
    match open(&mut loader, "/pdf") {
        OpenResult::Download { mime, url } => {
            assert_eq!(mime, "application/pdf");
            assert_eq!(url.href(), "https://a.example/pdf");
        }
        _ => panic!("pdf is an unsupported download"),
    }
    match open(&mut loader, "/img") {
        OpenResult::Page(p) => {
            assert_eq!(p.kind, ContentKind::Image);
            assert!(qinglan_engine::html::dump::dump_children(&p.doc, 0).contains("https://a.example/img"));
        }
        _ => panic!("image page"),
    }
    assert!(matches!(blocking::open(&mut loader, &u("data:text/html,x"), &BasicPlatform, Duration::from_secs(1)), OpenResult::Failed(_)));
}

fn sheet_server() -> Arc<Fake> {
    Fake::new(0, |req| {
        let css = match req.url.pathname().as_str() {
            "/a.css" => "@import \"b.css\";\n@import url(sub/c.css);\na{color:red}",
            "/b.css" => "@import 'a.css'; b{color:blue}", // cycle back to a.css
            "/sub/c.css" => "@import '../d.css'; c{color:green}",
            "/d.css" => "d{color:black}",
            "/g.css" => "g{color:gray}",
            _ => return Ok(resp(&req.url, 404, Some("text/html"), b"nope")),
        };
        let ct = if req.url.pathname() == "/g.css" { "text/css; charset=gbk" } else { "text/css" };
        Ok(resp(&req.url, 200, Some(ct), css.as_bytes()))
    })
}

#[test]
fn stylesheets_and_imports_are_discovered_once_each() {
    let fake = sheet_server();
    let mut loader = Loader::without_wake(fake.clone());
    let page = u("https://a.example/dir/index.html");
    let doc = parse_document(
        "<link rel=stylesheet href=/a.css><link rel=stylesheet href=/missing.css><link rel=stylesheet media=print href=/print.css>\
         <link rel=alternate href=/alt.css><link rel=stylesheet href='http://insecure.example/x.css'><style>@import '/d.css';</style>"
            .to_string(),
    );
    let mut res = PageResources::new(&page, &doc, Encoding::UTF_8);
    let n = res.start_sheets(&doc, &mut loader, &StyleOptions::default());
    assert_eq!(n, 2, "a.css and missing.css; print media, alternate and the mixed-content sheet are not requested");
    assert!(blocking::load_sheets(&mut loader, &mut res, &BasicPlatform, Duration::from_secs(10)));
    assert_eq!(res.sheets_pending(), 0);
    let mut paths: Vec<String> = fake.calls.lock().unwrap().iter().map(|r| r.url.pathname()).collect();
    paths.sort();
    assert_eq!(paths, ["/a.css", "/b.css", "/d.css", "/missing.css", "/sub/c.css"], "each sheet once, cycles stop");
    for p in ["a", "b", "d"] {
        assert!(res.load_stylesheet(&u(&format!("https://a.example/{p}.css"))).is_some(), "{p}.css");
    }
    assert!(res.load_stylesheet(&u("https://a.example/sub/c.css")).is_some());
    assert!(res.load_stylesheet(&u("https://a.example/missing.css")).is_none());
    let failed = res.failed_sheets();
    assert_eq!(failed.len(), 2, "{failed:?}");
    assert!(failed.iter().any(|(k, why)| k.ends_with("/missing.css") && why.contains("404")), "{failed:?}");
    assert!(failed.iter().any(|(k, why)| k.starts_with("http://insecure") && why.contains("insecure")), "{failed:?}");
    // The Referer of a same-origin sub-resource is the whole page address.
    let calls = fake.calls.lock().unwrap();
    let referer = calls[0].headers.iter().find(|(k, _)| k == "Referer").map(|(_, v)| v.as_str());
    assert_eq!(referer, Some("https://a.example/dir/index.html"));
}

struct SizeDecoder;

impl ImageDecoder for SizeDecoder {
    fn probe(&self, bytes: &[u8]) -> Result<ImageInfo, ImageError> {
        match bytes {
            [w, h, ..] => Ok(ImageInfo { width: u32::from(*w), height: u32::from(*h) }),
            _ => Err(ImageError::Corrupt),
        }
    }
    fn decode(&self, bytes: &[u8], width: u32, height: u32) -> Result<DecodedImage, ImageError> {
        self.probe(bytes)?;
        Ok(DecodedImage { width, height, natural_width: 1, natural_height: 1, bgra: vec![0; (width * height * 4) as usize] })
    }
}

#[test]
fn images_are_requested_once_per_address_and_sizes_fed_back() {
    let fake = Fake::new(0, |req| match req.url.pathname().as_str() {
        "/dir/a.png" => Ok(resp(&req.url, 200, Some("image/png"), &[30, 20, 0])),
        "/b.svg" => Ok(resp(&req.url, 200, Some("image/svg+xml"), b"<svg/>")),
        "/gone.png" => Ok(resp(&req.url, 404, Some("text/html"), b"x")),
        "/bad.png" => Ok(resp(&req.url, 200, Some("image/png"), &[1])),
        _ => Err(NetError::Timeout),
    });
    let mut loader = Loader::without_wake(fake.clone());
    let page = u("https://a.example/dir/index.html");
    let doc = parse_document("<base href='/dir/'><p>".to_string());
    let mut res = PageResources::new(&page, &doc, Encoding::UTF_8);
    assert_eq!(res.base().href(), "https://a.example/dir/");
    let img = |node: u32, src: &str| ImageRequest { node, src: src.to_string(), density: 1.0 };
    let reqs = vec![
        img(1, "a.png"),
        img(2, "a.png"), // same address: one fetch for both nodes
        img(3, "/b.svg"),
        img(4, "/gone.png"),
        img(5, "/bad.png"),
        img(6, "http://insecure.example/i.png"), // mixed content
        img(7, "data:image/png;base64,AQIDBA=="),
        img(8, "/slow.png"), // transport error
        img(9, "http://[bad"),
    ];
    let mut sizes = ImageSizes::new();
    let report = blocking::load_images(&mut loader, &mut res, &reqs, &SizeDecoder, &mut sizes, &BasicPlatform, Duration::from_secs(10));
    assert_eq!(report.pending, 0);
    assert_eq!(sizes.get(1), Some(ImageState::Loaded { width: 30, height: 20 }));
    assert_eq!(sizes.get(2), Some(ImageState::Loaded { width: 30, height: 20 }));
    assert_eq!(sizes.get(3), Some(ImageState::Broken), "SVG is not decoded: alt text");
    assert_eq!(sizes.get(4), Some(ImageState::Broken));
    assert_eq!(sizes.get(5), Some(ImageState::Broken));
    assert_eq!(sizes.get(6), Some(ImageState::Broken));
    assert_eq!(sizes.get(7), Some(ImageState::Loaded { width: 1, height: 2 }), "data: images need no network");
    assert_eq!(sizes.get(8), Some(ImageState::Broken));
    assert_eq!(sizes.get(9), Some(ImageState::Broken));
    let a_png = fake.calls.lock().unwrap().iter().filter(|r| r.url.pathname() == "/dir/a.png").count();
    assert_eq!(a_png, 1);
    // Asking again for the same nodes and addresses does nothing.
    let before = fake.count();
    let again = blocking::load_images(&mut loader, &mut res, &reqs, &SizeDecoder, &mut sizes, &BasicPlatform, Duration::from_secs(1));
    assert_eq!(fake.count(), before);
    assert_eq!(again.loaded, 0);
}

#[test]
fn decoded_cache_is_counted_and_bounded() {
    let mut cache = DecodedCache::with_budget(1000);
    let dec = SizeDecoder;
    let bytes = [4u8, 4, 0];
    for i in 0..5 {
        cache.get_or_decode(&dec, &format!("u{i}"), &bytes, 10, 10).unwrap(); // 400 bytes + 64 each
    }
    assert!(cache.bytes() <= 1000, "{}", cache.bytes());
    assert_eq!(cache.len(), 2);
    assert!(cache.get("u4", 10, 10).is_some() && cache.get("u0", 10, 10).is_none());
    assert!(cache.get_or_decode(&dec, "huge", &bytes, 100, 100).is_ok(), "an image over the budget is returned but not kept");
    assert!(cache.get("huge", 100, 100).is_none());
}

fn temp_dir(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("qinglan-net-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

#[test]
fn record_then_replay_never_touches_the_network() {
    let dir = temp_dir("rec");
    let fake = sheet_server();
    let rec = Arc::new(Recorder::new(fake.clone(), &dir).unwrap());
    let mut loader = Loader::without_wake(rec.clone());
    let page = u("https://a.example/index.html");
    let doc = parse_document("<link rel=stylesheet href=/a.css><link rel=stylesheet href=/g.css>".to_string());
    let mut res = PageResources::new(&page, &doc, Encoding::UTF_8);
    res.start_sheets(&doc, &mut loader, &StyleOptions::default());
    assert!(blocking::load_sheets(&mut loader, &mut res, &BasicPlatform, Duration::from_secs(10)));
    let (files, bytes) = rec.stats();
    assert_eq!(files, 5, "a, b, c, d and g");
    assert!(bytes > 50);
    let recorded_calls = fake.count();
    assert!(dir.join(INDEX_FILE).exists() && dir.join("0001.css").exists());
    let index = std::fs::read_to_string(dir.join(INDEX_FILE)).unwrap();
    assert!(index.contains("url: https://a.example/g.css") && index.contains("header: Content-Type: text/css; charset=gbk"), "{index}");

    // Replay: same texts, from files; an unknown address is a 404; the fake is never called again.
    let replay = Arc::new(ReplayTransport::open(&dir).unwrap());
    assert_eq!(replay.len(), 5);
    let mut loader = Loader::without_wake(replay.clone());
    let mut res2 = PageResources::new(&page, &doc, Encoding::UTF_8);
    res2.start_sheets(&doc, &mut loader, &StyleOptions::default());
    assert!(blocking::load_sheets(&mut loader, &mut res2, &BasicPlatform, Duration::from_secs(10)));
    for p in ["a", "b", "g", "d"] {
        let url = u(&format!("https://a.example/{p}.css"));
        assert_eq!(res.load_stylesheet(&url), res2.load_stylesheet(&url), "{p}.css");
    }
    loader.submit(page_req("https://unknown.example/x", ResourceKind::Document, None)).unwrap();
    let ev = drain(&mut loader, 1).remove(0);
    assert_eq!(ev.result.unwrap().status, 404);
    assert_eq!(fake.count(), recorded_calls, "replay does not call the recorded transport");

    // A hand-edited index can lie about files, but not leave the directory or exceed the cap.
    std::fs::write(dir.join(INDEX_FILE), "url: https://evil.example/\nbody: ../../secret.txt\n\nurl: https://evil.example/2\nbody: 0001.css\n").unwrap();
    let replay = ReplayTransport::open(&dir).unwrap();
    let req = |url: &str, max: usize| Request { url: u(url), kind: ResourceKind::Style, headers: vec![], max_bytes: max, cancel: Default::default() };
    assert!(replay.fetch(&req("https://evil.example/", 1 << 20)).is_err());
    assert_eq!(replay.fetch(&req("https://evil.example/2", 3)).unwrap_err(), NetError::TooLarge { limit: 3 });
    assert!(replay.fetch(&req("https://evil.example/2", 1 << 20)).is_ok());
    let _ = std::fs::remove_dir_all(&dir);
}
