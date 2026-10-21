//! Blocking drivers for the command-line tools and tests: they run the event-driven pieces
//! ([`Loader`], [`PageResources`]) to completion on the calling thread. The window does not use this
//! module; it feeds the same events from its message loop.

use std::time::{Duration, Instant};

use super::content::{self, ContentKind};
use super::loader::{LoadEvent, LoadRequest, Loader, Ticket};
use super::page::{PageResources, Update, apply_probe, probe_partial, probe_response};
use super::{ResourceKind, Response};
use crate::css::StyleOutput;
use crate::encoding::{Encoding, decode_document};
use crate::html::dom::Document;
use crate::html::treebuilder::parse_document;
use crate::layout::image::{ImageRequest, ImageSizes};
use crate::platform::Platform;
use crate::platform::image::ImageDecoder;
use crate::url::Url;

/// A document that was fetched and parsed.
pub struct Opened {
    /// Where the redirects ended; the base for everything the page loads.
    pub url: Url,
    pub status: u16,
    pub doc: Document,
    pub encoding: Encoding,
    /// `Html`, `PlainText` (shown as `<pre>`) or `Image` (a page with just the picture).
    pub kind: ContentKind,
    /// A note about the document (encoding not available ...).
    pub warning: Option<String>,
}

pub enum OpenResult {
    Page(Box<Opened>),
    /// A type layer 1 does not show: the window says so and gives the address.
    Download { url: Url, mime: String },
    /// Refused or failed; the text is for the error page.
    Failed(String),
}

/// Wait for the event of `ticket`; anything else that arrives meanwhile is dropped (callers use this
/// with one request outstanding).
pub fn wait_for(loader: &mut Loader, ticket: Ticket, timeout: Duration) -> Option<LoadEvent> {
    let deadline = Instant::now() + timeout;
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return None;
        }
        let events = loader.wait(left);
        let none_left = events.is_empty() && loader.pending() == 0;
        if let Some(e) = events.into_iter().find(|e| e.ticket == ticket) {
            return Some(e);
        }
        if none_left {
            return None;
        }
    }
}

fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + s.len() / 8);
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            c => out.push(c),
        }
    }
    out
}

/// Open `url` as the user would from the address bar: fetch, classify, decode and parse.
pub fn open(loader: &mut Loader, url: &Url, platform: &dyn Platform, timeout: Duration) -> OpenResult {
    let req = LoadRequest { url: url.clone(), kind: ResourceKind::Document, page: None, use_cache: false };
    let ticket = match loader.submit(req) {
        Ok(t) => t,
        Err(refusal) => return OpenResult::Failed(refusal.to_string()),
    };
    let Some(ev) = wait_for(loader, ticket, timeout) else {
        loader.cancel_all();
        return OpenResult::Failed("timed out".into());
    };
    let resp: std::sync::Arc<Response> = match ev.result {
        Ok(r) => r,
        Err(e) => return OpenResult::Failed(e.to_string()),
    };
    open_response(&resp, platform)
}

/// The second half of [`open`] for callers that fetched the page themselves (the window): classify the response,
/// decode its text and parse it.
pub fn open_response(resp: &Response, platform: &dyn Platform) -> OpenResult {
    let kind = content::classify(resp);
    let (text, encoding, warning) = match &kind {
        ContentKind::Html => {
            let d = decode_document(&resp.body, resp.charset().as_deref(), platform);
            (d.text, d.encoding, d.warning)
        }
        ContentKind::PlainText => {
            let d = decode_document(&resp.body, resp.charset().as_deref(), platform);
            (format!("<!DOCTYPE html><pre>\n{}</pre>", escape_html(&d.text)), d.encoding, d.warning)
        }
        ContentKind::Image => {
            // An SVG file is a document of its own (it fills the window); a broken one is the picture of a broken image.
            if content::is_svg(resp)
                && let Ok(doc) = crate::svg::xml::parse(&resp.body)
            {
                return OpenResult::Page(Box::new(Opened { url: resp.url.clone(), status: resp.status, doc, encoding: Encoding::UTF_8, kind, warning: None }));
            }
            (format!("<!DOCTYPE html><img src=\"{}\">", escape_html(&resp.url.href())), Encoding::UTF_8, None)
        }
        ContentKind::Unsupported { mime } => return OpenResult::Download { url: resp.url.clone(), mime: mime.clone() },
    };
    let doc = parse_document(text);
    OpenResult::Page(Box::new(Opened { url: resp.url.clone(), status: resp.status, doc, encoding, kind, warning }))
}

/// The resource tracker for an opened page. A picture opened alone is already in the loader's cache,
/// so its `<img>` is answered from there.
pub fn resources_for(opened: &Opened) -> PageResources {
    let mut r = PageResources::new(&opened.url, &opened.doc, opened.encoding);
    r.use_cache = opened.kind == ContentKind::Image;
    r
}

/// Run the loader until every submitted style sheet (and their imports) arrived or `timeout` passed.
/// Returns whether all arrived.
pub fn load_sheets(loader: &mut Loader, res: &mut PageResources, platform: &dyn Platform, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while res.sheets_pending() > 0 {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return false;
        }
        let events = loader.wait(left);
        let stalled = events.is_empty() && loader.pending() == 0;
        for ev in events {
            res.handle(loader, ev, platform);
        }
        if stalled {
            return res.sheets_pending() == 0;
        }
    }
    true
}

/// Fetch the web fonts of a styled page and wait for them (or `timeout`). Returns how many faces were still loading.
pub fn load_fonts(loader: &mut Loader, res: &mut PageResources, doc: &Document, styled: &StyleOutput, platform: &dyn Platform, timeout: Duration) -> usize {
    res.start_fonts(doc, styled, loader, platform);
    let deadline = Instant::now() + timeout;
    while res.fonts.pending() > 0 {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            break;
        }
        let events = loader.wait(left);
        let stalled = events.is_empty() && loader.pending() == 0;
        for ev in events {
            res.handle(loader, ev, platform);
        }
        if stalled {
            break;
        }
    }
    res.fonts.pending()
}

/// What happened to the images of a page.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ImageReport {
    pub requested: usize,
    pub loaded: usize,
    pub broken: usize,
    /// Still not answered when the time ran out.
    pub pending: usize,
}

/// Fetch the images of a layout and record their sizes (or that they are broken) in `sizes`.
pub fn load_images(
    loader: &mut Loader,
    res: &mut PageResources,
    images: &[ImageRequest],
    dec: &dyn ImageDecoder,
    sizes: &mut ImageSizes,
    platform: &dyn Platform,
    timeout: Duration,
) -> ImageReport {
    let mut report = ImageReport { requested: images.len(), ..ImageReport::default() };
    for n in res.request_images(loader, images) {
        sizes.set_broken(n);
        report.broken += 1;
    }
    let deadline = Instant::now() + timeout;
    while res.images_pending() > 0 {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            break;
        }
        let events = loader.wait(left);
        let stalled = events.is_empty() && loader.pending() == 0;
        for ev in events {
            if let Some(Update::Image { nodes, result, .. }) = res.handle(loader, ev, platform) {
                match result {
                    Ok(resp) => {
                        let probe = probe_response(dec, &resp);
                        if probe.is_ok() {
                            report.loaded += nodes.len();
                        } else {
                            report.broken += nodes.len();
                        }
                        apply_probe(sizes, &nodes, &probe, probe_partial(&resp));
                    }
                    Err(_) => {
                        for n in &nodes {
                            sizes.set_broken(*n);
                        }
                        report.broken += nodes.len();
                    }
                }
            }
        }
        if stalled {
            break;
        }
    }
    report.pending = res.images_pending();
    report
}
