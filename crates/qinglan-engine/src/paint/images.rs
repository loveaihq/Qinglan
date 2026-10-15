//! Lazy loading and eviction of images (L1 task 15) and the decoded bitmaps the rasterizer draws.
//!
//! The rules, all measured in screens (the viewport height) from the viewport:
//!
//! * an image is requested when it is within [`EAGER_SCREENS`] (two) of the viewport, or within
//!   [`LAZY_SCREENS`] (one) when it has `loading="lazy"`; `render --full` / `load_all` requests everything;
//! * a decoded bitmap is dropped when its image is more than [`DROP_SCREENS`] (four) away; the compressed bytes
//!   stay in the response cache, and the bitmap is decoded again when the image comes back near;
//! * bitmaps are decoded at the size they are drawn (device pixels), at most [`PREPARE_SCREENS`] (one screen)
//!   from the viewport, before the frame is drawn: the rasterizer itself never decodes.
//!
//! All three are pure functions of the display list and the viewport band, so they can be tested without a
//! window, and the window calls them after every layout and every scroll.

use std::sync::Arc;

use super::raster::device_size;
use super::{DisplayList, Rect};
use crate::html::atoms::FxHashMap;
use crate::html::dom::NodeId;
use crate::layout::LayoutResult;
use crate::layout::image::ImageRequest;
use crate::layout::units::LayoutUnit as LU;
use crate::net::Response;
use crate::platform::image::{DecodedCache, DecodedImage, ImageDecoder};

/// Images this far from the viewport (in screens) are requested up front.
pub const EAGER_SCREENS: f32 = 2.0;
/// `loading="lazy"` images wait until they are this near.
pub const LAZY_SCREENS: f32 = 1.0;
/// Decoded bitmaps of images further away than this are dropped.
pub const DROP_SCREENS: f32 = 4.0;
/// Bitmaps are decoded up front for images this near.
pub const PREPARE_SCREENS: f32 = 1.0;

/// The part of the page that is on screen, in CSS px.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Band {
    pub top: f32,
    pub height: f32,
}

impl Band {
    pub fn new(scroll_y: f32, viewport_h: f32) -> Band {
        Band { top: scroll_y.max(0.0), height: viewport_h.max(1.0) }
    }

    /// How far (CSS px) the page range `top..bottom` is from the band; 0 when they overlap.
    fn distance(&self, top: LU, bottom: LU) -> f32 {
        let (t, b) = (top.to_f32(), bottom.to_f32());
        let band_bottom = self.top + self.height;
        if b < self.top {
            self.top - b
        } else if t > band_bottom {
            t - band_bottom
        } else {
            0.0
        }
    }
}

/// Which of the layout's images should be requested now (clones of `LayoutResult::images` entries). Asking
/// again after a layout or a scroll gives a growing set; the page's resource tracker skips what it has.
pub fn wanted_images(res: &LayoutResult, list: &DisplayList, band: Band, load_all: bool) -> Vec<ImageRequest> {
    let mut out = Vec::new();
    for (i, r) in res.images.iter().enumerate() {
        let want = load_all
            || match list.spots.get(i) {
                None => true,
                Some(s) => {
                    let screens = if s.lazy { LAZY_SCREENS } else { EAGER_SCREENS };
                    band.distance(s.top, s.bottom) <= screens * band.height
                }
            };
        if want {
            out.push(r.clone());
        }
    }
    out
}

/// Images shown more than [`DROP_SCREENS`] away from the band.
pub fn far_images(list: &DisplayList, band: Band) -> Vec<NodeId> {
    list.images.iter().filter(|p| !p.fixed && band.distance(p.rect.y, p.rect.bottom()) > DROP_SCREENS * band.height).map(|p| p.node).collect()
}

/// Where the rasterizer gets decoded pixels from.
pub trait ImageSource {
    /// The bitmap of the image of `node` for a destination of `width` x `height` device pixels, if it is ready.
    fn bitmap(&mut self, node: NodeId, width: u32, height: u32) -> Option<Arc<DecodedImage>>;
}

/// No images are ready.
pub struct NoBitmaps;

impl ImageSource for NoBitmaps {
    fn bitmap(&mut self, _node: NodeId, _width: u32, _height: u32) -> Option<Arc<DecodedImage>> {
        None
    }
}

/// What [`ImageStore::prepare`] did.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Prepared {
    pub decoded: usize,
    /// Nodes whose compressed bytes were not available (evicted from the response cache): request them again.
    pub missing: Vec<NodeId>,
}

/// The decoded bitmaps of a page: [`DecodedCache`] (24 MB, LRU) plus the address and size each node's
/// bitmap was decoded at, so that bitmaps of far images can be dropped by node.
pub struct ImageStore {
    urls: FxHashMap<NodeId, String>,
    cache: DecodedCache,
    keys: FxHashMap<NodeId, (String, u32, u32)>,
    /// Nodes whose bytes would not decode: not tried again.
    failed: FxHashMap<NodeId, ()>,
}

impl Default for ImageStore {
    fn default() -> Self {
        ImageStore::new()
    }
}

impl ImageStore {
    pub fn new() -> ImageStore {
        ImageStore::with_cache(DecodedCache::new())
    }

    pub fn with_cache(cache: DecodedCache) -> ImageStore {
        ImageStore { urls: FxHashMap::default(), cache, keys: FxHashMap::default(), failed: FxHashMap::default() }
    }

    /// The image of `node` comes from `url` (the key of the response cache: `Url::serialize(true)`).
    pub fn set_url(&mut self, node: NodeId, url: &str) {
        if self.urls.get(&node).map(String::as_str) != Some(url) {
            self.urls.insert(node, url.to_string());
            self.failed.remove(&node);
        }
    }

    /// Decode, at the size they are drawn, the images of `list` that are within [`PREPARE_SCREENS`] of `band`
    /// and not decoded yet. `bytes` gives the response for an address (the response cache).
    pub fn prepare(&mut self, list: &DisplayList, band: Band, dpr: f32, dec: &dyn ImageDecoder, bytes: &mut dyn FnMut(&str) -> Option<Arc<Response>>) -> Prepared {
        let mut report = Prepared::default();
        for p in &list.images {
            if !p.fixed && band.distance(p.rect.y, p.rect.bottom()) > PREPARE_SCREENS * band.height {
                continue;
            }
            let (w, h) = device_size(&p.rect, dpr);
            if w == 0 || h == 0 || self.failed.contains_key(&p.node) {
                continue;
            }
            let Some(url) = self.urls.get(&p.node).cloned() else { continue };
            if self.cache.get(&url, w, h).is_some() {
                self.keys.insert(p.node, (url, w, h));
                continue;
            }
            let Some(resp) = bytes(&url) else {
                report.missing.push(p.node);
                continue;
            };
            match self.cache.get_or_decode(dec, &url, &resp.body, w, h) {
                Ok(_) => {
                    if let Some((u, ow, oh)) = self.keys.insert(p.node, (url.clone(), w, h))
                        && (u != url || ow != w || oh != h)
                    {
                        self.cache.remove(&u, ow, oh);
                    }
                    report.decoded += 1;
                }
                Err(_) => {
                    self.failed.insert(p.node, ());
                }
            }
        }
        report
    }

    /// Drop the bitmaps of the images that are more than [`DROP_SCREENS`] from `band`. Returns how many.
    pub fn drop_far(&mut self, list: &DisplayList, band: Band) -> usize {
        let mut n = 0;
        for node in far_images(list, band) {
            if let Some((u, w, h)) = self.keys.remove(&node) {
                self.cache.remove(&u, w, h);
                n += 1;
            }
        }
        n
    }

    /// Forget everything (a new page).
    pub fn clear(&mut self) {
        *self = ImageStore::new();
    }

    /// Bitmap bytes held.
    pub fn bytes(&self) -> usize {
        self.cache.bytes()
    }

    pub fn len(&self) -> usize {
        self.cache.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// Is the bitmap for this destination size ready?
    pub fn has(&mut self, node: NodeId, rect: &Rect, dpr: f32) -> bool {
        let (w, h) = device_size(rect, dpr);
        self.urls.get(&node).is_some_and(|u| self.cache.get(u, w, h).is_some())
    }
}

impl ImageSource for ImageStore {
    fn bitmap(&mut self, node: NodeId, width: u32, height: u32) -> Option<Arc<DecodedImage>> {
        let url = self.urls.get(&node)?;
        self.cache.get(url, width, height)
    }
}
