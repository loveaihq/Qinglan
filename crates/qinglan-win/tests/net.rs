//! WinHTTP transport against a local HTTP server (step 1c-1): redirects, decompression, the size cap on
//! decompressed bytes (gzip bomb), responses that never end, timeouts, header charset, no cookies,
//! record and replay. The server is `std::net::TcpListener` on 127.0.0.1 inside the test.
#![allow(clippy::indexing_slicing, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use qinglan_engine::net::loader::{LoadRequest, Loader};
use qinglan_engine::net::replay::{Recorder, ReplayTransport};
use qinglan_engine::net::{NetError, Request, ResourceKind, Response, Transport};
use qinglan_engine::url::Url;
use qinglan_win::{WinHttpConfig, WinHttpTransport, decode_legacy};

const BOMB: &[u8] = include_bytes!("fixtures/bomb-100m.gz");

/// A gzip stream that inflates to `total` zero bytes (CRC and size in the trailer are not real: nobody gets that far).
/// One literal 0, then matches of length 258 at distance 1 in a fixed-Huffman block, 13 bits per 258 bytes: 1 GiB is
/// 6.7 MB on the wire.
fn gzip_zeros(total: u64) -> Vec<u8> {
    struct Bits {
        out: Vec<u8>,
        acc: u32,
        n: u32,
    }
    impl Bits {
        fn put(&mut self, v: u32, bits: u32) {
            self.acc |= v << self.n;
            self.n += bits;
            while self.n >= 8 {
                self.out.push(self.acc as u8);
                self.acc >>= 8;
                self.n -= 8;
            }
        }
        /// Huffman codes go in most significant bit first.
        fn code(&mut self, c: u32, len: u32) {
            for i in (0..len).rev() {
                self.put((c >> i) & 1, 1);
            }
        }
    }
    let mut b = Bits { out: vec![0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 0xff], acc: 0, n: 0 };
    b.put(1, 1); // final block
    b.put(1, 2); // fixed Huffman codes
    b.code(0x30, 8); // literal 0
    for _ in 0..(total - 1) / 258 {
        b.code(0xC5, 8); // length 258 (symbol 285)
        b.code(0, 5); // distance 1
    }
    b.code(0, 7); // end of block
    b.put(0, 7); // pad to a byte
    b.out.extend([0; 8]);
    b.out
}
const HELLO_GZ: &[u8] = include_bytes!("fixtures/hello.gz");

#[derive(Clone, Debug)]
struct Seen {
    path: String,
    headers: Vec<(String, String)>,
}

impl Seen {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(n, _)| n.eq_ignore_ascii_case(name)).map(|(_, v)| v.as_str())
    }
}

struct Server {
    port: u16,
    log: Arc<Mutex<Vec<Seen>>>,
    stop: Arc<AtomicBool>,
}

impl Server {
    fn start() -> Server {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let log = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let (log2, stop2) = (log.clone(), stop.clone());
        std::thread::spawn(move || {
            for conn in listener.incoming() {
                if stop2.load(Ordering::SeqCst) {
                    break;
                }
                let Ok(stream) = conn else { continue };
                let log = log2.clone();
                std::thread::spawn(move || handle(stream, &log));
            }
        });
        Server { port, log, stop }
    }
    fn url(&self, path: &str) -> Url {
        Url::parse(&format!("http://127.0.0.1:{}{path}", self.port), None).unwrap()
    }
    fn seen(&self, path: &str) -> Vec<Seen> {
        self.log.lock().unwrap().iter().filter(|s| s.path == path).cloned().collect()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(("127.0.0.1", self.port));
    }
}

fn reply(s: &mut TcpStream, status: &str, headers: &[(&str, &str)], body: &[u8]) {
    let mut head = format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n", body.len());
    for (n, v) in headers {
        head.push_str(&format!("{n}: {v}\r\n"));
    }
    head.push_str("\r\n");
    let _ = s.write_all(head.as_bytes());
    let _ = s.write_all(body);
}

fn handle(mut s: TcpStream, log: &Mutex<Vec<Seen>>) {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    while !buf.windows(4).any(|w| w == b"\r\n\r\n") {
        match s.read(&mut chunk) {
            Ok(0) | Err(_) => return,
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
        }
    }
    let text = String::from_utf8_lossy(&buf).into_owned();
    let mut lines = text.split("\r\n");
    let path = lines.next().unwrap_or("").split(' ').nth(1).unwrap_or("/").to_string();
    let headers: Vec<(String, String)> = lines.filter_map(|l| l.split_once(':')).map(|(n, v)| (n.trim().to_string(), v.trim().to_string())).collect();
    log.lock().unwrap().push(Seen { path: path.clone(), headers });
    match path.as_str() {
        "/hello" => reply(&mut s, "200 OK", &[("Content-Type", "text/html; charset=GBK"), ("Set-Cookie", "a=1; Path=/")], &[0xD6, 0xD0, 0xCE, 0xC4]),
        "/gzip" => reply(&mut s, "200 OK", &[("Content-Type", "text/html"), ("Content-Encoding", "gzip")], HELLO_GZ),
        "/bomb" => reply(&mut s, "200 OK", &[("Content-Type", "text/html"), ("Content-Encoding", "gzip")], BOMB),
        "/bomb1g" => reply(&mut s, "200 OK", &[("Content-Type", "text/html"), ("Content-Encoding", "gzip")], &gzip_zeros(1 << 30)),
        "/redir1" => reply(&mut s, "302 Found", &[("Location", "/redir2")], b""),
        "/redir2" => reply(&mut s, "301 Moved Permanently", &[("Location", "/hello#frag")], b""),
        "/loop" => reply(&mut s, "302 Found", &[("Location", "/loop")], b""),
        "/to-ftp" => reply(&mut s, "302 Found", &[("Location", "ftp://example.com/x")], b""),
        "/missing" => reply(&mut s, "404 Not Found", &[("Content-Type", "text/html")], b"<p>gone"),
        "/forever" => {
            let _ = s.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nConnection: close\r\n\r\n");
            let block = vec![b'x'; 64 * 1024];
            while s.write_all(&block).is_ok() {}
        }
        "/dribble" => {
            let _ = s.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nConnection: close\r\n\r\n");
            while s.write_all(b"x").is_ok() {
                std::thread::sleep(Duration::from_millis(100));
            }
        }
        "/silent" => std::thread::sleep(Duration::from_secs(6)),
        "/bigcl" => {
            let _ = s.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 104857600\r\nConnection: close\r\n\r\nshort");
            std::thread::sleep(Duration::from_secs(6));
        }
        _ => reply(&mut s, "200 OK", &[("Content-Type", "text/plain")], path.as_bytes()),
    }
}

fn transport() -> WinHttpTransport {
    WinHttpTransport::new().expect("WinHTTP")
}

fn quick() -> WinHttpTransport {
    WinHttpTransport::with_config(WinHttpConfig { connect_timeout: Duration::from_secs(2), total_timeout: Duration::from_millis(1500), ..WinHttpConfig::default() }).unwrap()
}

fn req(url: Url, max: usize) -> Request {
    Request { url, kind: ResourceKind::Document, headers: vec![("Accept".into(), "text/html".into()), ("Referer".into(), "http://127.0.0.1/ref".into())], max_bytes: max, cancel: Arc::new(AtomicBool::new(false)) }
}

const MAX: usize = 16 * 1024 * 1024;

#[test]
fn plain_get_charset_headers_and_no_cookies() {
    let srv = Server::start();
    let t = transport();
    let r = t.fetch(&req(srv.url("/hello"), MAX)).unwrap();
    assert_eq!(r.status, 200);
    assert_eq!(r.mime().as_deref(), Some("text/html"));
    assert_eq!(r.charset().as_deref(), Some("GBK"), "charset comes from the header");
    assert_eq!(decode_legacy("GBK", &r.body).as_deref(), Some("中文"));
    assert!(r.header("set-cookie").is_some(), "headers are passed on, but never acted on");
    // The server set a cookie; the next request must not carry it.
    t.fetch(&req(srv.url("/second"), MAX)).unwrap();
    let seen = srv.seen("/second");
    assert_eq!(seen.len(), 1);
    assert!(seen[0].header("cookie").is_none(), "cookies are off: {:?}", seen[0].headers);
    // What the server saw: our User-Agent, the Referer and Accept we passed, compressed encodings accepted.
    let first = &srv.seen("/hello")[0];
    assert_eq!(first.header("user-agent"), Some("Mozilla/5.0 (Windows NT 10.0; Win64; x64) qinglan/0.1"));
    assert_eq!(first.header("referer"), Some("http://127.0.0.1/ref"));
    assert_eq!(first.header("accept"), Some("text/html"));
    // The language header comes from the system's display languages.
    let want = qinglan_win::accept_language(&qinglan_win::ui_languages());
    assert_eq!(first.header("accept-language"), Some(want.as_str()));
    assert!(want.contains(",en"), "{want}");
    let enc = first.header("accept-encoding").unwrap_or("").to_ascii_lowercase();
    assert!(enc.contains("gzip") && enc.contains("deflate"), "Accept-Encoding: {enc}");
    // HTTP error statuses come back as responses.
    let r = t.fetch(&req(srv.url("/missing"), MAX)).unwrap();
    assert_eq!((r.status, r.body.as_slice()), (404, &b"<p>gone"[..]));
}

#[test]
fn gzip_is_decompressed() {
    let srv = Server::start();
    let r = transport().fetch(&req(srv.url("/gzip"), MAX)).unwrap();
    assert_eq!(r.body, b"<p>hello gzip</p>");
}

#[test]
fn gzip_bomb_is_capped_on_decompressed_bytes() {
    let srv = Server::start();
    assert!(BOMB.len() < 200 * 1024, "the compressed bomb is small ({} bytes)", BOMB.len());
    let t0 = Instant::now();
    let e = transport().fetch(&req(srv.url("/bomb"), MAX)).unwrap_err();
    assert_eq!(e, NetError::TooLarge { limit: MAX }, "100 MB of zeros must stop at the cap");
    assert!(t0.elapsed() < Duration::from_secs(10), "took {:?}", t0.elapsed());
    assert!(e.to_string().contains("16 MB"), "{e}");
}

#[test]
fn gzip_bomb_of_one_gigabyte_is_capped_in_seconds() {
    let srv = Server::start();
    let t0 = Instant::now();
    let e = transport().fetch(&req(srv.url("/bomb1g"), MAX)).unwrap_err();
    assert_eq!(e, NetError::TooLarge { limit: MAX }, "1 GiB of zeros must stop at the cap");
    // Time budgets only in release builds: a debug build on a busy machine can take three times as long.
    assert!(cfg!(debug_assertions) || t0.elapsed() < Duration::from_secs(5), "took {:?}", t0.elapsed());
}

#[test]
fn redirects_are_followed_checked_and_limited() {
    let srv = Server::start();
    let t = transport();
    let r = t.fetch(&req(srv.url("/redir1"), MAX)).unwrap();
    assert_eq!(r.url.pathname(), "/hello", "final URL after two redirects, fragment dropped");
    assert!(r.url.fragment.is_none());
    assert_eq!(r.body.len(), 4);
    let t0 = Instant::now();
    assert_eq!(t.fetch(&req(srv.url("/loop"), MAX)).unwrap_err(), NetError::TooManyRedirects);
    assert_eq!(srv.seen("/loop").len(), 11, "the first request plus 10 redirects");
    assert!(cfg!(debug_assertions) || t0.elapsed() < Duration::from_secs(10));
    assert!(matches!(t.fetch(&req(srv.url("/to-ftp"), MAX)).unwrap_err(), NetError::BadRedirect(_)));
}

#[test]
fn response_that_never_ends_hits_the_cap() {
    let srv = Server::start();
    let t0 = Instant::now();
    let e = transport().fetch(&req(srv.url("/forever"), 4 * 1024 * 1024)).unwrap_err();
    assert_eq!(e, NetError::TooLarge { limit: 4 * 1024 * 1024 });
    assert!(t0.elapsed() < Duration::from_secs(10), "took {:?}", t0.elapsed());
}

#[test]
fn slow_server_times_out_overall() {
    let srv = Server::start();
    let t = quick();
    let t0 = Instant::now();
    assert_eq!(t.fetch(&req(srv.url("/dribble"), MAX)).unwrap_err(), NetError::Timeout, "one byte every 100 ms is still too slow overall");
    assert!(t0.elapsed() < Duration::from_millis(2500), "the 1.5 s total limit is enforced by the watchdog: {:?}", t0.elapsed());
}

#[test]
fn silent_server_times_out() {
    let srv = Server::start();
    let t = quick();
    let t0 = Instant::now();
    assert_eq!(t.fetch(&req(srv.url("/silent"), MAX)).unwrap_err(), NetError::Timeout);
    eprintln!("silent server: gave up after {:?} (total limit 1.5 s)", t0.elapsed());
    // WinHTTP alone overshot this by up to ~4 s; the watchdog closes the request at the deadline.
    assert!(t0.elapsed() >= Duration::from_millis(1400) && t0.elapsed() < Duration::from_millis(2500), "took {:?}", t0.elapsed());
    assert_eq!(t.active_requests(), 0);
}

#[test]
fn oversized_content_length_is_refused_before_the_body() {
    let srv = Server::start();
    let t0 = Instant::now();
    let e = transport().fetch(&req(srv.url("/bigcl"), MAX)).unwrap_err();
    assert_eq!(e, NetError::TooLarge { limit: MAX });
    assert!(t0.elapsed() < Duration::from_secs(3), "no waiting for 100 MB: {:?}", t0.elapsed());
}

#[test]
fn closed_port_is_a_connect_error_and_cancel_works() {
    let port = {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        l.local_addr().unwrap().port()
    };
    let url = Url::parse(&format!("http://127.0.0.1:{port}/"), None).unwrap();
    let e = quick().fetch(&req(url, MAX)).unwrap_err();
    assert!(matches!(e, NetError::Connect(_) | NetError::Timeout), "{e:?}");
    let srv = Server::start();
    let r = req(srv.url("/hello"), MAX);
    r.cancel.store(true, Ordering::SeqCst);
    assert_eq!(transport().fetch(&r).unwrap_err(), NetError::Cancelled);
}

#[test]
fn loader_over_winhttp_wakes_and_sends_referer() {
    let srv = Server::start();
    let woke = Arc::new(AtomicUsize::new(0));
    let w = woke.clone();
    let mut loader = Loader::new(Arc::new(transport()), Arc::new(move || {
        w.fetch_add(1, Ordering::SeqCst);
    }));
    let page = srv.url("/page.html");
    for i in 0..10 {
        loader.submit(LoadRequest { url: srv.url(&format!("/img{i}.png")), kind: ResourceKind::Image, page: Some(page.clone()), use_cache: false }).unwrap();
    }
    let mut got = 0;
    let t0 = Instant::now();
    while got < 10 && t0.elapsed() < Duration::from_secs(10) {
        for ev in loader.wait(Duration::from_millis(200)) {
            assert_eq!(ev.result.unwrap().status, 200);
            got += 1;
        }
    }
    assert_eq!(got, 10);
    assert!(woke.load(Ordering::SeqCst) >= 1);
    let seen = srv.seen("/img3.png");
    assert_eq!(seen[0].header("referer"), Some(page.href().as_str()), "same origin: the whole page address");
    assert!(seen[0].header("accept").unwrap().starts_with("image/png"));
}

#[test]
fn record_over_winhttp_then_replay_without_the_server() {
    let dir = std::env::temp_dir().join(format!("qinglan-win-rec-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let (url_hello, url_redir): (Url, Url);
    let recorded: Vec<Response>;
    {
        let srv = Server::start();
        url_hello = srv.url("/hello");
        url_redir = srv.url("/redir1");
        let rec = Recorder::new(Arc::new(transport()), &dir).unwrap();
        recorded = vec![rec.fetch(&req(url_hello.clone(), MAX)).unwrap(), rec.fetch(&req(url_redir.clone(), MAX)).unwrap(), rec.fetch(&req(srv.url("/gzip"), MAX)).unwrap()];
        assert_eq!(rec.stats().0, 3);
    } // the server is gone from here on
    let replay = ReplayTransport::open(&dir).unwrap();
    let again = replay.fetch(&req(url_hello.clone(), MAX)).unwrap();
    assert_eq!((again.status, again.body.as_slice(), again.charset().as_deref()), (200, &recorded[0].body[..], Some("GBK")));
    let redirected = replay.fetch(&req(url_redir, MAX)).unwrap();
    assert_eq!(redirected.url.pathname(), "/hello", "the final address of a redirect is recorded");
    assert_eq!(replay.fetch(&req(Url::parse("http://127.0.0.1:9/never", None).unwrap(), MAX)).unwrap().status, 404);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cancelling_aborts_a_blocked_request_quickly() {
    let srv = Server::start();
    let t = transport(); // the default total limit is 60 s: only the abort can end this in time
    let r = req(srv.url("/silent"), MAX);
    let cancel = r.cancel.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(300));
        cancel.store(true, Ordering::SeqCst);
    });
    let t0 = Instant::now();
    assert_eq!(t.fetch(&r).unwrap_err(), NetError::Cancelled);
    assert!(t0.elapsed() < Duration::from_millis(1500), "took {:?}", t0.elapsed());
    assert_eq!(t.active_requests(), 0);
}

#[test]
fn cancel_all_frees_the_slots_and_aborts_the_blocked_workers() {
    // Three servers (the per-host limit is 6), 18 requests: 16 run, all blocked on a server that says nothing.
    let silent: Vec<Server> = (0..3).map(|_| Server::start()).collect();
    let live = Server::start();
    let t = Arc::new(transport());
    let mut loader = Loader::without_wake(t.clone());
    for i in 0..18 {
        loader.submit(LoadRequest { url: silent[i % 3].url("/silent"), kind: ResourceKind::Image, page: None, use_cache: false }).unwrap();
    }
    assert_eq!(loader.in_flight(), 16);
    let t0 = Instant::now();
    while t.active_requests() < 16 && t0.elapsed() < Duration::from_secs(5) {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(t.active_requests(), 16, "all sixteen are on the wire");

    loader.cancel_all();
    assert_eq!((loader.in_flight(), loader.pending()), (0, 0), "the slots are free at once");
    let t1 = Instant::now();
    loader.submit(LoadRequest { url: live.url("/hello"), kind: ResourceKind::Document, page: None, use_cache: false }).unwrap();
    assert_eq!(loader.in_flight(), 1, "the new document starts at once");
    let mut got = Vec::new();
    while got.is_empty() && t1.elapsed() < Duration::from_secs(3) {
        got = loader.wait(Duration::from_millis(100));
    }
    assert_eq!(got.len(), 1, "only the new request is answered");
    assert_eq!(got[0].result.as_ref().unwrap().status, 200);
    assert!(t1.elapsed() < Duration::from_secs(1), "{:?}", t1.elapsed());
    // The cancelled workers do not stay blocked for the server's 6 s (or the 60 s limit): the requests are closed.
    while t.active_requests() > 0 && t1.elapsed() < Duration::from_secs(3) {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(t.active_requests(), 0, "aborted after {:?}", t1.elapsed());
    assert!(loader.poll().is_empty(), "nothing from the cancelled generation is delivered");
}
