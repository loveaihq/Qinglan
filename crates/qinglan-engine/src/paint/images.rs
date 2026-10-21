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

use super::decode::{DecodeKey, DecodePool, Finished, JobId};
use super::raster::device_size;
use super::{DisplayList, Rect};
use crate::html::atoms::FxHashMap;
use crate::html::dom::NodeId;
use crate::layout::LayoutResult;
use crate::layout::image::ImageRequest;
use crate::layout::units::LayoutUnit as LU;
use crate::net::Response;
use crate::platform::Platform;
use crate::platform::image::{DecodedCache, DecodedImage, ImageDecoder, MAX_DECODED_PIXELS};

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
    list.images.iter().filter(|p| !p.fixed && (band.distance(p.span.0, p.span.1) + list.scope_gap(p.scope, &p.area)) > DROP_SCREENS * band.height).map(|p| p.node).collect()
}

/// Where the rasterizer gets decoded pixels from.
pub trait ImageSource {
    /// The bitmap of the image of `node` for a destination of `width` x `height` device pixels, if it is ready.
    fn bitmap(&mut self, node: NodeId, width: u32, height: u32) -> Option<Arc<DecodedImage>>;

    /// Called before a frame is drawn, on the thread that has the platform: the place to draw what the decode threads could
    /// not (the text of SVG pictures).
    fn finish(&mut self, _platform: &dyn Platform) {}
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
    /// Nodes whose decode failed since [`ImageStore::take_decode_failed`] was last called: their pictures are broken even
    /// though the header said they had a size (a WebP on a system without a WebP codec), so the host shows their `alt`.
    decode_failed: Vec<NodeId>,
    /// How many times in a row the bytes of a node were gone from the response cache after being asked for again.
    refetched: FxHashMap<NodeId, u8>,
    /// Decodes handed to a [`DecodePool`] and not taken in yet (step 2-0), by what they decode.
    /// A handful at a time (the images near the viewport), so a list: no hash table for one more key type (the
    /// executable is size-limited).
    inflight: Vec<(DecodeKey, Pending)>,
    /// What the jobs in `inflight` will take of the budget when they are done.
    reserved: usize,
    /// The last [`ImageStore::prepare_async`] left images undone for lack of room.
    deferred: bool,
    /// Bitmaps in the cache that are SVGs with text not drawn yet ([`DecodedImage::text`]); [`ImageSource::finish`] does it.
    unfinished: Vec<(String, u32, u32)>,
}

/// A decode on a worker thread.
struct Pending {
    id: JobId,
    /// What the bitmap will cost (an upper bound: a decode that comes out smaller costs less).
    cost: usize,
    /// The nodes near enough to want it, as of the last `prepare_async`. A job nobody wants any more is taken back if
    /// it has not started; one that has finishes and is dropped.
    nodes: Vec<NodeId>,
}

/// What [`ImageStore::accept`] did: the nodes that have a bitmap now (to be drawn).
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Accepted {
    pub nodes: Vec<NodeId>,
}

/// What a decode of `width` x `height` device pixels takes of the cache budget.
fn decode_cost(width: u32, height: u32) -> usize {
    let px = (u64::from(width) * u64::from(height)).min(MAX_DECODED_PIXELS);
    usize::try_from(px).unwrap_or(usize::MAX).saturating_mul(4).saturating_add(64)
}

/// Bytes that are gone from the response cache are asked for again this many times in a row; then the image is given up
/// (an image too big for the cache would otherwise be downloaded again and again).
const MAX_REFETCH: u8 = 2;

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
        ImageStore {
            urls: FxHashMap::default(),
            cache,
            keys: FxHashMap::default(),
            failed: FxHashMap::default(),
            decode_failed: Vec::new(),
            refetched: FxHashMap::default(),
            inflight: Vec::new(),
            reserved: 0,
            deferred: false,
            unfinished: Vec::new(),
        }
    }

    /// The image of `node` comes from `url` (the key of the response cache: `Url::serialize(true)`).
    pub fn set_url(&mut self, node: NodeId, url: &str) {
        if self.urls.get(&node).map(String::as_str) != Some(url) {
            self.urls.insert(node, url.to_string());
            self.failed.remove(&node);
            self.refetched.remove(&node);
        }
    }

    /// Decode, at the size they are drawn, the images of `list` that are within [`PREPARE_SCREENS`] of `band`
    /// and not decoded yet. `bytes` gives the response for an address (the response cache).
    pub fn prepare(&mut self, list: &DisplayList, band: Band, dpr: f32, dec: &dyn ImageDecoder, bytes: &mut dyn FnMut(&str) -> Option<Arc<Response>>) -> Prepared {
        let mut report = Prepared::default();
        for p in &list.images {
            if !p.fixed && (band.distance(p.span.0, p.span.1) + list.scope_gap(p.scope, &p.area)) > PREPARE_SCREENS * band.height {
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
                let n = self.refetched.entry(p.node).or_insert(0);
                if *n >= MAX_REFETCH {
                    self.failed.insert(p.node, ());
                } else {
                    *n += 1;
                    report.missing.push(p.node);
                }
                continue;
            };
            match self.cache.get_or_decode(dec, &url, &resp.body, w, h) {
                Ok(img) => {
                    if img.text.is_some() && !self.unfinished.contains(&(url.clone(), w, h)) {
                        self.unfinished.push((url.clone(), w, h));
                    }
                    self.refetched.remove(&p.node);
                    if let Some((u, ow, oh)) = self.keys.insert(p.node, (url.clone(), w, h))
                        && (u != url || ow != w || oh != h)
                    {
                        self.cache.remove(&u, ow, oh);
                    }
                    report.decoded += 1;
                }
                Err(_) => {
                    self.failed.insert(p.node, ());
                    self.decode_failed.push(p.node);
                }
            }
        }
        report
    }

    /// The nodes whose decode failed since the last call (see `decode_failed`). The host marks them broken
    /// ([`crate::layout::ImageSizes::set_broken`]) and lays out again.
    pub fn take_decode_failed(&mut self) -> Vec<NodeId> {
        std::mem::take(&mut self.decode_failed)
    }

    /// [`ImageStore::prepare`] without waiting (step 2-0): the decodes that are due go to `pool`, nearest to the viewport
    /// first, and the bitmaps come back through [`ImageStore::accept`]; until then the image is not drawn. `decoded` of
    /// the report stays 0 (nothing is decoded here), `missing` is as in `prepare`.
    ///
    /// The budget counts the decodes on their way: nothing is started that would not fit next to what is cached and
    /// what is running. Room is made by dropping cached bitmaps of images *farther* from the viewport than the one that
    /// waits, never nearer ones (two images that do not fit together would throw each other out for ever). What still
    /// does not fit waits for a decode to finish (`deferred`). A queued decode that no image near the viewport wants
    /// any more (it scrolled away, its size changed) is taken back; a running one finishes and is dropped by `accept`.
    pub fn prepare_async(&mut self, list: &DisplayList, band: Band, dpr: f32, pool: &mut DecodePool, bytes: &mut dyn FnMut(&str) -> Option<Arc<Response>>) -> Prepared {
        let mut report = Prepared::default();
        for (_, p) in &mut self.inflight {
            p.nodes.clear();
        }
        // The nearest distance of each (address, size) that is wanted, and what has to be started (nearest first).
        let mut dist: Vec<(DecodeKey, f32)> = Vec::new();
        let mut todo: Vec<(f32, NodeId, DecodeKey)> = Vec::new();
        for p in &list.images {
            let d = if p.fixed { 0.0 } else { band.distance(p.span.0, p.span.1) + list.scope_gap(p.scope, &p.area) };
            if d > PREPARE_SCREENS * band.height {
                continue;
            }
            let (w, h) = device_size(&p.rect, dpr);
            if w == 0 || h == 0 || self.failed.contains_key(&p.node) {
                continue;
            }
            let Some(url) = self.urls.get(&p.node) else { continue };
            let key = (url.clone(), w, h);
            match dist.iter_mut().find(|(k, _)| *k == key) {
                Some((_, nearest)) => *nearest = nearest.min(d),
                None => dist.push((key.clone(), d)),
            }
            if let Some((_, pending)) = self.inflight.iter_mut().find(|(k, _)| *k == key) {
                pending.nodes.push(p.node);
            } else if self.cache.get(&key.0, w, h).is_some() {
                self.keys.insert(p.node, key);
            } else {
                let at = todo.partition_point(|t| t.0 <= d);
                todo.insert(at, (d, p.node, key));
            }
        }
        let reserved = &mut self.reserved;
        self.inflight.retain(|(_, p)| {
            let back = p.nodes.is_empty() && pool.cancel(p.id);
            if back {
                *reserved = reserved.saturating_sub(p.cost);
            }
            !back
        });
        self.deferred = false;
        for (d, node, key) in todo {
            if let Some((_, p)) = self.inflight.iter_mut().find(|(k, _)| *k == key) {
                p.nodes.push(node);
                continue;
            }
            let cost = decode_cost(key.1, key.2);
            let budget = self.cache.budget();
            if cost > budget {
                // Would never fit, whatever is dropped: not drawn (as with `prepare`, which decodes it for nothing).
                continue;
            }
            while self.cache.bytes().saturating_add(self.reserved).saturating_add(cost) > budget {
                if !self.cache.evict_oldest_where(&|k| dist.iter().find(|(dk, _)| dk == k).is_none_or(|(_, near)| *near > d)) {
                    break;
                }
            }
            if self.cache.bytes().saturating_add(self.reserved).saturating_add(cost) > budget {
                self.deferred = true;
                continue;
            }
            let Some(resp) = bytes(&key.0) else {
                let n = self.refetched.entry(node).or_insert(0);
                if *n >= MAX_REFETCH {
                    self.failed.insert(node, ());
                } else {
                    *n += 1;
                    report.missing.push(node);
                }
                continue;
            };
            let id = pool.submit(key.clone(), resp);
            self.reserved = self.reserved.saturating_add(cost);
            self.inflight.push((key, Pending { id, cost, nodes: vec![node] }));
        }
        report
    }

    /// Take in a decode that a [`DecodePool`] finished. Dropped (nothing is stored) when this store did not start it or
    /// does not want it any more: the page was left (a new store has no such job), the image went far away, its size
    /// changed. Returns the nodes that have a bitmap now.
    pub fn accept(&mut self, done: Finished) -> Accepted {
        let Finished { id, key, result } = done;
        let Some(at) = self.inflight.iter().position(|(k, p)| *k == key && p.id == id) else { return Accepted::default() };
        let (_, p) = self.inflight.swap_remove(at);
        self.reserved = self.reserved.saturating_sub(p.cost);
        if p.nodes.is_empty() {
            return Accepted::default();
        }
        match result {
            Ok(img) => {
                if img.text.is_some() && !self.unfinished.contains(&key) {
                    self.unfinished.push(key.clone());
                }
                self.cache.insert(&key.0, key.1, key.2, Arc::new(img));
                for &node in &p.nodes {
                    self.refetched.remove(&node);
                    if let Some((u, ow, oh)) = self.keys.insert(node, key.clone())
                        && (u != key.0 || ow != key.1 || oh != key.2)
                    {
                        self.cache.remove(&u, ow, oh);
                    }
                }
                Accepted { nodes: p.nodes }
            }
            Err(_) => {
                for node in p.nodes {
                    self.failed.insert(node, ());
                    self.decode_failed.push(node);
                }
                Accepted::default()
            }
        }
    }

    /// Decodes handed to the pool and not taken in yet.
    pub fn in_flight(&self) -> usize {
        self.inflight.len()
    }

    /// Bytes the decodes on their way will take, counted against the budget together with [`ImageStore::bytes`].
    pub fn reserved_bytes(&self) -> usize {
        self.reserved
    }

    /// The last `prepare_async` left images undone because the budget was full: ask again when a decode finishes.
    pub fn deferred(&self) -> bool {
        self.deferred
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
    fn finish(&mut self, platform: &dyn Platform) {
        for (u, w, h) in std::mem::take(&mut self.unfinished) {
            let Some(img) = self.cache.get(&u, w, h) else { continue };
            if let Some(done) = crate::svg::add_text(&img, platform) {
                self.cache.insert(&u, w, h, Arc::new(done));
            }
        }
    }

    fn bitmap(&mut self, node: NodeId, width: u32, height: u32) -> Option<Arc<DecodedImage>> {
        let url = self.urls.get(&node)?;
        self.cache.get(url, width, height)
    }
}
