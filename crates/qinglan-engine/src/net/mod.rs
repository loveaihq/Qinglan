//! Fetching resources (L1 task 13 and 14): the policy half of networking, in pure Rust.
//!
//! * [`policy`] decides *whether and how* a URL may be fetched (schemes, mixed content, `Referer`).
//! * [`loader::Loader`] schedules the fetches (per-host and total limits, priorities), keeps the
//!   back/forward response cache and hands results back as events. It never blocks the caller.
//! * A [`Transport`] does the actual transfer on a worker thread: WinHTTP in `qinglan-win`, or the
//!   [`replay`] transports. A transport is a blocking `fetch`; the loader owns the threads.
//! * [`page::PageResources`] finds the style sheets (`<link>`, `@import`) and images of one page and
//!   feeds them back to the style and layout code; [`blocking`] drives all of it synchronously for the
//!   command-line tools.
//!
//! Page data is hostile: every limit here (sizes, counts, redirect depth) is enforced, and a failure
//! is always a typed [`NetError`] or [`policy::Refusal`], never a panic.

pub mod blocking;
pub mod fonts;
pub mod cache;
pub mod content;
pub mod data_url;
pub mod file;
pub mod loader;
pub mod page;
pub mod policy;
pub mod replay;

use std::fmt;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crate::url::Url;

/// `User-Agent` of every request (decisions.md "User-Agent").
pub const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) qinglan/0.1";
/// Largest HTML or CSS response (L1 task 13).
pub const MAX_TEXT_BYTES: usize = 16 * 1024 * 1024;
/// Largest image response (L1 task 13).
pub const MAX_IMAGE_BYTES: usize = 32 * 1024 * 1024;
/// Concurrent requests per host.
pub const MAX_PER_HOST: usize = 6;
/// Concurrent requests in total.
pub const MAX_IN_FLIGHT: usize = 16;
/// Requests waiting for a free slot; more are refused (a page with 100 000 images).
pub const MAX_QUEUED: usize = 4096;
/// Back/forward response cache budget. The zhwiki page set (styles, 130 small images) is 2.6 MB without the page.
pub const CACHE_BUDGET: usize = 8 * 1024 * 1024;
/// A response bigger than this is not kept: the page itself is parsed and then dead weight (zhwiki's is 2.4 MB, more
/// than the rest together), and going back to a page that big fetches it again.
pub const CACHE_ENTRY_MAX: usize = 1024 * 1024;
/// The same limit for an image (`Content-Type: image/*`): its compressed bytes are what the painter decodes from
/// again and again, so an image that is not kept can never be shown (a 1.6 MB photo was fetched over and over and never
/// drawn). Half the budget, so one image cannot push everything else out.
pub const CACHE_IMAGE_ENTRY_MAX: usize = CACHE_BUDGET / 2;

/// What a request is for. It sets the size cap, the `Accept` header and the queue priority.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum ResourceKind {
    /// A page being opened (HTML, plain text, an image shown alone, or something we cannot show).
    Document,
    /// A style sheet (`<link>` or `@import`).
    Style,
    Image,
    /// A web font (`@font-face`).
    Font,
}

impl ResourceKind {
    pub fn max_bytes(self) -> usize {
        match self {
            ResourceKind::Document | ResourceKind::Style => MAX_TEXT_BYTES,
            ResourceKind::Image => MAX_IMAGE_BYTES,
            ResourceKind::Font => crate::fonts::MAX_FONT_BYTES,
        }
    }

    pub fn accept(self) -> &'static str {
        match self {
            ResourceKind::Document => "text/html,application/xhtml+xml;q=0.9,text/plain;q=0.8,image/*;q=0.7,*/*;q=0.5",
            ResourceKind::Style => "text/css,*/*;q=0.1",
            ResourceKind::Font => "font/woff2,font/woff,font/ttf,font/otf,application/font-woff,*/*;q=0.1",
            ResourceKind::Image => "image/png,image/jpeg,image/gif,image/bmp,image/x-icon,image/webp,image/*;q=0.8,*/*;q=0.5",
        }
    }

    /// Lower is served first.
    pub(crate) fn priority(self) -> usize {
        match self {
            ResourceKind::Document => 0,
            ResourceKind::Style | ResourceKind::Font => 1,
            ResourceKind::Image => 2,
        }
    }
}

/// One request as a [`Transport`] sees it: the loader has already applied the policy.
#[derive(Clone, Debug)]
pub struct Request {
    /// `http` or `https`, without fragment.
    pub url: Url,
    pub kind: ResourceKind,
    /// Extra request headers (`Accept`, `Referer`). The transport adds `User-Agent` itself and asks for
    /// `Accept-Encoding: gzip, deflate`.
    pub headers: Vec<(String, String)>,
    /// The body (after decompression) may not be longer than this; the transport stops reading and
    /// returns [`NetError::TooLarge`] when it would be.
    pub max_bytes: usize,
    /// Set when the page was closed or the load stopped: the transport should give up soon.
    pub cancel: Arc<AtomicBool>,
}

/// A finished response.
#[derive(Clone, Debug)]
pub struct Response {
    /// Where the redirects ended (the requested URL when there were none).
    pub url: Url,
    pub status: u16,
    /// Header names keep their case; look up with [`Response::header`].
    pub headers: Vec<(String, String)>,
    /// Decompressed body.
    pub body: Vec<u8>,
}

impl Response {
    /// The first header called `name` (case-insensitive).
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(n, _)| n.eq_ignore_ascii_case(name)).map(|(_, v)| v.as_str())
    }

    /// The media type without parameters, lower-cased (`text/html`).
    pub fn mime(&self) -> Option<String> {
        let ct = self.header("content-type")?;
        let m = ct.split(';').next().unwrap_or("").trim().to_ascii_lowercase();
        (!m.is_empty()).then_some(m)
    }

    /// The `charset` parameter of `Content-Type`, without quotes.
    pub fn charset(&self) -> Option<String> {
        let ct = self.header("content-type")?;
        ct.split(';').skip(1).find_map(|p| {
            let (k, v) = p.split_once('=')?;
            k.trim().eq_ignore_ascii_case("charset").then(|| v.trim().trim_matches(|c| c == '"' || c == '\'').to_string())
        })
    }

    /// What this response costs the cache.
    pub fn cost(&self) -> usize {
        self.body.len() + self.headers.iter().map(|(n, v)| n.len() + v.len() + 4).sum::<usize>() + self.url.href().len() + 128
    }
}

/// Why a transfer failed. Every variant has a plain sentence ([`fmt::Display`]) for the error page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NetError {
    /// Connect or total time limit passed.
    Timeout,
    /// The host name did not resolve.
    Dns(String),
    /// Could not connect or the connection broke.
    Connect(String),
    /// The certificate or TLS handshake was rejected (there is no way to continue anyway).
    Tls(String),
    /// The body (after decompression) is longer than the cap.
    TooLarge { limit: usize },
    /// More than 10 redirects (or a loop).
    TooManyRedirects,
    /// A redirect to somewhere we do not go (another scheme, https to http, a bad address).
    BadRedirect(String),
    /// The server's answer was not HTTP we understand.
    Protocol(String),
    /// The load was stopped.
    Cancelled,
    Other(String),
}

impl fmt::Display for NetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetError::Timeout => write!(f, "the server took too long to answer (timeout)"),
            NetError::Dns(h) => write!(f, "cannot find the server {h}"),
            NetError::Connect(m) => write!(f, "cannot connect: {m}"),
            NetError::Tls(m) => write!(f, "the secure connection failed: {m}"),
            NetError::TooLarge { limit } => write!(f, "the response is larger than the limit of {} MB", limit / (1024 * 1024)),
            NetError::TooManyRedirects => write!(f, "too many redirects"),
            NetError::BadRedirect(m) => write!(f, "refused redirect: {m}"),
            NetError::Protocol(m) => write!(f, "bad response: {m}"),
            NetError::Cancelled => write!(f, "cancelled"),
            NetError::Other(m) => write!(f, "{m}"),
        }
    }
}

/// The transfer half of networking. `fetch` blocks the calling thread until the response is complete or
/// has failed; the [`loader::Loader`] calls it from worker threads, so implementations must be usable
/// from several threads at once.
///
/// A transport follows redirects itself (at most 10, http and https only, never https to http) and
/// reports the final URL in [`Response::url`]. It does not look at the policy: the loader already did.
pub trait Transport: Send + Sync {
    fn fetch(&self, req: &Request) -> Result<Response, NetError>;
}
