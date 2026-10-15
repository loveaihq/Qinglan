//! The sub-resources of one page: style sheets (`<link>` and `@import`) and images, found with the
//! existing style and layout code and fetched through a [`Loader`].
//!
//! Event driven, so the window can use it without blocking:
//!
//! 1. After parsing: [`PageResources::start_sheets`] submits the `<link>` sheets.
//! 2. For every [`LoadEvent`] of the loader: [`PageResources::handle`]. A sheet that arrives is decoded
//!    (header charset, BOM, `@charset`, page encoding) and its `@import`s are submitted in turn.
//! 3. When [`PageResources::sheets_pending`] is 0 (or the window gave up waiting), run
//!    `style_document_with(.., &mut resources, ..)`: `PageResources` is the [`ResourceLoader`], serving
//!    the texts it collected.
//! 4. After layout: [`PageResources::request_images`] with `LayoutResult::images`. Each arriving image
//!    comes back from `handle` as [`Update::Image`]; [`apply_probe`] turns its header into an
//!    `ImageSizes` entry, and the page is laid out again.

use std::sync::Arc;

use super::loader::{LoadEvent, LoadRequest, Loader, Ticket};
use super::{ResourceKind, Response, content};
use crate::css::{MAX_SHEETS, ResourceLoader, StyleOptions, base_url, linked_stylesheets, scan_imports};
use crate::encoding::{Encoding, decode_stylesheet};
use crate::html::atoms::FxHashMap;
use crate::html::dom::{Document, NodeId};
use crate::layout::image::{ImageRequest, ImageSizes};
use crate::platform::Platform;
use crate::platform::image::{ImageDecoder, ImageError, ImageInfo};
use crate::url::Url;

/// `@import` chains deeper than this are not followed.
pub const MAX_IMPORT_DEPTH: u8 = 8;
/// Distinct image addresses fetched for one page (a page with 100 000 images gets the first ones).
pub const MAX_IMAGES: usize = 2000;

enum SheetState {
    Pending,
    Ready(String),
    Failed(String),
}

enum Waiter {
    Sheet { key: String, depth: u8 },
    Image { key: String },
}

/// What a loader event meant for the page. Returned one at a time, so the size of the image variant
/// does not matter.
#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum Update {
    /// A style sheet arrived (`ok`) or failed. Its `@import`s have been submitted.
    Sheet { url: String, ok: bool },
    /// The bytes (or the failure) of an image for every node that uses it.
    Image { nodes: Vec<NodeId>, url: Url, result: Result<Arc<Response>, String> },
}

pub struct PageResources {
    page: Url,
    base: Url,
    fallback_encoding: Encoding,
    sheets: FxHashMap<String, SheetState>,
    pending_sheets: usize,
    waiting: FxHashMap<Ticket, Waiter>,
    /// Image addresses in flight and the nodes waiting for each.
    images: FxHashMap<String, Vec<NodeId>>,
    /// The address each node's image was last requested from.
    node_urls: FxHashMap<NodeId, String>,
    image_count: usize,
    pending_images: usize,
    /// Answer from the loader's back/forward cache when it can (history navigation).
    pub use_cache: bool,
}

impl PageResources {
    /// `page` is the address of the document (for `Referer`, mixed content, relative links);
    /// `encoding` is the document's encoding, the fallback for style sheets that do not declare one.
    pub fn new(page: &Url, doc: &Document, encoding: Encoding) -> PageResources {
        let base = base_url(doc, Some(page)).unwrap_or_else(|| page.clone());
        PageResources {
            page: page.clone(),
            base,
            fallback_encoding: encoding,
            sheets: FxHashMap::default(),
            pending_sheets: 0,
            waiting: FxHashMap::default(),
            images: FxHashMap::default(),
            node_urls: FxHashMap::default(),
            image_count: 0,
            pending_images: 0,
            use_cache: false,
        }
    }

    /// The document base URL (images and `<a>` links resolve against it).
    pub fn base(&self) -> &Url {
        &self.base
    }

    /// Submit the `<link rel=stylesheet>` sheets that apply at `opts`' viewport. Returns how many.
    pub fn start_sheets(&mut self, doc: &Document, loader: &mut Loader, opts: &StyleOptions) -> usize {
        let mut n = 0;
        for u in linked_stylesheets(doc, Some(&self.page), opts) {
            if self.add_sheet(loader, u, 0) {
                n += 1;
            }
        }
        n
    }

    fn add_sheet(&mut self, loader: &mut Loader, url: Url, depth: u8) -> bool {
        let key = url.href();
        if self.sheets.contains_key(&key) || self.sheets.len() >= MAX_SHEETS {
            return false;
        }
        let req = LoadRequest { url, kind: ResourceKind::Style, page: Some(self.page.clone()), use_cache: self.use_cache };
        match loader.submit(req) {
            Ok(t) => {
                self.sheets.insert(key.clone(), SheetState::Pending);
                self.waiting.insert(t, Waiter::Sheet { key, depth });
                self.pending_sheets += 1;
                true
            }
            Err(refusal) => {
                self.sheets.insert(key, SheetState::Failed(refusal.to_string()));
                false
            }
        }
    }

    /// Style sheets submitted and not yet answered (imports included).
    pub fn sheets_pending(&self) -> usize {
        self.pending_sheets
    }

    /// Images submitted and not yet answered.
    pub fn images_pending(&self) -> usize {
        self.pending_images
    }

    /// Why sheets failed, as `(address, reason)`.
    pub fn failed_sheets(&self) -> Vec<(String, String)> {
        let mut v: Vec<(String, String)> = self
            .sheets
            .iter()
            .filter_map(|(k, s)| match s {
                SheetState::Failed(why) => Some((k.clone(), why.clone())),
                _ => None,
            })
            .collect();
        v.sort();
        v
    }

    /// Submit the images of a layout. Returns the nodes that cannot be fetched at all (bad address,
    /// refused by the policy, over the per-page limit): mark them broken. A node already requested from
    /// the same address is skipped, so the call can be repeated after every layout.
    pub fn request_images(&mut self, loader: &mut Loader, images: &[ImageRequest]) -> Vec<NodeId> {
        let mut broken = Vec::new();
        for r in images {
            let Ok(url) = self.base.join(r.src.trim()) else {
                self.node_urls.insert(r.node, r.src.clone());
                broken.push(r.node);
                continue;
            };
            let key = url.href();
            if self.node_urls.get(&r.node) == Some(&key) {
                continue;
            }
            self.node_urls.insert(r.node, key.clone());
            if let Some(nodes) = self.images.get_mut(&key) {
                nodes.push(r.node);
                continue;
            }
            if self.image_count >= MAX_IMAGES {
                broken.push(r.node);
                continue;
            }
            self.image_count += 1;
            let req = LoadRequest { url, kind: ResourceKind::Image, page: Some(self.page.clone()), use_cache: self.use_cache };
            match loader.submit(req) {
                Ok(t) => {
                    self.images.insert(key.clone(), vec![r.node]);
                    self.waiting.insert(t, Waiter::Image { key });
                    self.pending_images += 1;
                }
                Err(_) => broken.push(r.node),
            }
        }
        broken
    }

    /// Process one loader event. `None` when the ticket is not one of this page's (the caller's own).
    pub fn handle(&mut self, loader: &mut Loader, ev: LoadEvent, platform: &dyn Platform) -> Option<Update> {
        let waiter = self.waiting.remove(&ev.ticket)?;
        match waiter {
            Waiter::Sheet { key, depth } => {
                self.pending_sheets = self.pending_sheets.saturating_sub(1);
                let outcome = match ev.result {
                    Ok(resp) if (200..300).contains(&resp.status) => Ok(resp),
                    Ok(resp) => Err(format!("HTTP {}", resp.status)),
                    Err(e) => Err(e.to_string()),
                };
                match outcome {
                    Ok(resp) => {
                        let text = decode_stylesheet(&resp.body, resp.charset().as_deref(), self.fallback_encoding, platform);
                        if depth < MAX_IMPORT_DEPTH {
                            for imp in scan_imports(&text) {
                                if let Ok(u) = ev.url.join(&imp) {
                                    self.add_sheet(loader, u, depth + 1);
                                }
                            }
                        }
                        self.sheets.insert(key.clone(), SheetState::Ready(text));
                        Some(Update::Sheet { url: key, ok: true })
                    }
                    Err(why) => {
                        self.sheets.insert(key.clone(), SheetState::Failed(why));
                        Some(Update::Sheet { url: key, ok: false })
                    }
                }
            }
            Waiter::Image { key } => {
                self.pending_images = self.pending_images.saturating_sub(1);
                let nodes = self.images.remove(&key).unwrap_or_default();
                let result = match ev.result {
                    Ok(resp) if (200..300).contains(&resp.status) => Ok(resp),
                    Ok(resp) => Err(format!("HTTP {}", resp.status)),
                    Err(e) => Err(e.to_string()),
                };
                Some(Update::Image { nodes, url: ev.url, result })
            }
        }
    }
}

/// The style sheets the page collected, for `style_document`. Sheets that failed or never came are
/// "missing" (the page is styled without them).
impl ResourceLoader for PageResources {
    fn load_stylesheet(&mut self, url: &Url) -> Option<String> {
        match self.sheets.get(&url.href()) {
            Some(SheetState::Ready(t)) => Some(t.clone()),
            _ => None,
        }
    }
}

/// Read the header of an image response: SVG is refused (`alt` text instead), anything else goes to
/// the decoder's [`ImageDecoder::probe`].
pub fn probe_response(dec: &dyn ImageDecoder, resp: &Response) -> Result<ImageInfo, ImageError> {
    if content::is_svg(resp) {
        return Err(ImageError::Svg);
    }
    dec.probe(&resp.body)
}

/// Record what the probe found in the layout's image table. Returns whether layout needs to run again.
pub fn apply_probe(sizes: &mut ImageSizes, nodes: &[NodeId], probe: &Result<ImageInfo, ImageError>) -> bool {
    let mut changed = false;
    for &n in nodes {
        changed |= match probe {
            Ok(info) => sizes.set_loaded(n, info.width, info.height),
            Err(_) => sizes.set_broken(n),
        };
    }
    changed
}
