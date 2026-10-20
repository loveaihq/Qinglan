//! One page in the window: its DOM, styles, layout and display list, the images it has asked for, and the steps that
//! keep them current. Everything here is the engine's own code; the window and the network are the caller's.

use qinglan_engine::css::{StyleOptions, StyleOutput, style_document_with};
use qinglan_engine::html::dom::{Document, NodeId};
use qinglan_engine::interact::{PageIndex, document_title, fragment_target, link_target};
use qinglan_engine::layout::image::{ImageSizes, ImageState};
use qinglan_engine::layout::{LayoutOptions, LayoutResult, layout_document_with};
use qinglan_engine::net::Response;
use qinglan_engine::net::blocking::{self, Opened};
use qinglan_engine::net::loader::Loader;
use qinglan_engine::net::page::{PageResources, Update, probe_response};
use qinglan_engine::paint::decode::{DecodePool, Finished};
use qinglan_engine::paint::images::{Band, ImageStore};
use qinglan_engine::paint::raster::{Frame, View, render_band};
use qinglan_engine::paint::{DisplayList, build_display_list, wanted_images};
use qinglan_engine::platform::Platform;
use qinglan_engine::url::Url;
use qinglan_win::WicDecoder;

/// The visible area in CSS px and its scale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewport {
    pub w: f32,
    pub h: f32,
    pub dpr: f32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    /// The HTML is in; waiting for the style sheets in `<head>` (at most 5 s).
    Sheets,
    /// Styled, laid out and shown.
    Shown,
}

/// What an arrived image changes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Effect {
    /// A size is known or changed: lay out again.
    Relayout,
    /// Only the picture changed (or nothing did).
    Repaint,
}

pub struct Page {
    /// The document's address (after redirects, no fragment).
    pub url: Url,
    /// What the address bar shows.
    pub display: String,
    pub doc: Document,
    pub res: PageResources,
    pub styled: Option<StyleOutput>,
    pub sizes: ImageSizes,
    pub store: ImageStore,
    pub layout: Option<LayoutResult>,
    pub list: Option<DisplayList>,
    /// Hit-testing data of a complete (not first-screen-only) layout.
    pub index: Option<PageIndex>,
    pub phase: Phase,
    pub is_error: bool,
    pub title: String,
    pub sheets_missing: usize,
    /// Scroll position to restore once laid out (back/forward, reload).
    pub restore: Option<f32>,
    /// `#fragment` to scroll to once laid out.
    pub fragment: Option<String>,
}

impl Page {
    pub fn from_opened(o: Opened, display: String, is_error: bool) -> Page {
        let res = blocking::resources_for(&o);
        let title = document_title(&o.doc);
        Page {
            url: o.url,
            display,
            doc: o.doc,
            res,
            styled: None,
            sizes: ImageSizes::new(),
            store: ImageStore::new(),
            layout: None,
            list: None,
            index: None,
            phase: Phase::Sheets,
            is_error,
            title,
            sheets_missing: 0,
            restore: None,
            fragment: None,
        }
    }

    /// Compute the styles for `vp` (media queries look at its size).
    pub fn style(&mut self, platform: &dyn Platform, vp: Viewport) {
        let opts = StyleOptions { viewport_w: vp.w, viewport_h: vp.h, ..StyleOptions::default() };
        self.styled = None;
        let out = style_document_with(&mut self.doc, Some(&self.url), &mut self.res, &opts, Some(platform));
        self.sheets_missing = out.missing.len();
        self.styled = Some(out);
    }

    /// Declare the page's `@font-face` faces to the platform and fetch the ones its text uses.
    pub fn start_fonts(&mut self, loader: &mut Loader, platform: &dyn Platform) {
        if let Some(styled) = &self.styled {
            self.res.start_fonts(&self.doc, styled, loader, platform);
        }
    }

    /// Lay out and make the display list. With `stop_y` the layout stops that far down the page (first-screen-first);
    /// a layout that stopped has no hit-testing index.
    pub fn layout(&mut self, platform: &dyn Platform, vp: Viewport, stop_y: Option<f32>) {
        // The old results go first: a big page's layout and display list are the largest things in memory.
        self.index = None;
        self.list = None;
        self.layout = None;
        let Some(styled) = &self.styled else { return };
        let opts = LayoutOptions { viewport_w: vp.w, viewport_h: vp.h, dpr: vp.dpr, stop_y, ..LayoutOptions::default() };
        let mut res = layout_document_with(&self.doc, &styled.tree, platform, &opts, &self.sizes);
        let list = build_display_list(&self.doc, &res, platform);
        self.index = (!res.partial).then(|| PageIndex::new(&res));
        // Only the display list needs the glyphs and texts; the page keeps the boxes, lines and fragments.
        res.release_text_data();
        self.list = Some(list);
        self.layout = Some(res);
    }

    /// Lay out the whole page again (an image arrived, the window changed) and return the scroll position that keeps
    /// what the user is looking at where it was: the first text or image at or below the top edge stays put
    /// (simple scroll anchoring). At the very top nothing is anchored.
    pub fn relayout(&mut self, platform: &dyn Platform, vp: Viewport, scroll: f32) -> f32 {
        let anchor = match (&self.index, &self.layout) {
            (Some(index), Some(layout)) if scroll > 0.0 => index.anchor(layout, scroll),
            _ => None,
        };
        self.layout(platform, vp, None);
        match (anchor, &self.index, &self.layout) {
            (Some((node, offset)), Some(index), Some(layout)) => index.node_y(layout, node).map_or(scroll, |y| y - offset),
            _ => scroll,
        }
    }

    pub fn is_complete(&self) -> bool {
        self.layout.as_ref().is_some_and(|l| !l.partial)
    }

    pub fn height(&self) -> f32 {
        self.list.as_ref().map_or(0.0, |l| l.height.to_f32())
    }

    pub fn max_scroll(&self, vp: Viewport) -> f32 {
        // `overflow: hidden` on the root or the body goes to the viewport: the page cannot be scrolled by the user.
        if self.layout.as_ref().is_some_and(|l| l.tree.viewport_overflow.1 == qinglan_engine::css::types::Overflow::Hidden) {
            return 0.0;
        }
        (self.height() - vp.h).max(0.0)
    }

    /// Ask for the images near the viewport (all of them when `all`). True when a broken image changed a size, so the
    /// page needs a new layout.
    pub fn request_images(&mut self, loader: &mut Loader, vp: Viewport, scroll: f32, all: bool) -> bool {
        let (Some(layout), Some(list)) = (&self.layout, &self.list) else { return false };
        let wanted = wanted_images(layout, list, Band::new(scroll, vp.h), all);
        let mut changed = false;
        for n in self.res.request_images(loader, &wanted) {
            changed |= self.break_image(n);
        }
        changed
    }

    /// An image (or its failure) came in. A picture whose size leaves every box it fills as it is (the `width` is given
    /// and the height follows from it the same way as before: the usual `width`/`height` attributes of a thumbnail)
    /// only needs painting.
    pub fn apply_image(&mut self, nodes: &[NodeId], url: &Url, result: Result<std::sync::Arc<Response>, String>) -> Effect {
        let changed = match result {
            Ok(resp) => match probe_response(&WicDecoder, &resp) {
                Ok(info) => {
                    let keeps_boxes = nodes.iter().all(|&n| self.keeps_box(n, info.width, info.height));
                    let changed = nodes.iter().fold(false, |c, &n| c | self.sizes.set_loaded(n, info.width, info.height));
                    for n in nodes {
                        self.store.set_url(*n, &url.serialize(true));
                    }
                    changed && !keeps_boxes
                }
                Err(_) => nodes.iter().fold(false, |c, &n| c | self.break_image(n)),
            },
            Err(_) => nodes.iter().fold(false, |c, &n| c | self.break_image(n)),
        };
        if changed { Effect::Relayout } else { Effect::Repaint }
    }

    /// The image of `node` cannot be shown. True when that needs a new layout: a box that was made for the image (it has a
    /// size from the page) stays as it is, only an image shown as its alt text turns into something else.
    fn break_image(&mut self, node: NodeId) -> bool {
        let has_box = self.layout.as_ref().is_some_and(|l| l.fits.contains_key(&node));
        self.sizes.set_broken(node) && !has_box
    }

    /// Would the last layout's box for the image of `node` come out the same with an image of this pixel size? A node
    /// that was broken, or that the layout did not make a box for, always needs a new layout.
    fn keeps_box(&self, node: NodeId, width: u32, height: u32) -> bool {
        !matches!(self.sizes.get(node), Some(ImageState::Broken)) && self.layout.as_ref().and_then(|l| l.fits.get(&node)).is_some_and(|f| f.keeps_size(width, height))
    }

    /// Hand a loader event of this page's sub-resources to the resource tracker.
    pub fn handle_update(&mut self, loader: &mut Loader, ev: qinglan_engine::net::loader::LoadEvent, platform: &dyn Platform) -> Option<PageUpdate> {
        match self.res.handle(loader, ev, platform)? {
            Update::Sheet { .. } => Some(PageUpdate::Sheet),
            Update::Image { nodes, url, result } => Some(PageUpdate::Image(self.apply_image(&nodes, &url, result))),
            Update::Font { changed } => Some(PageUpdate::Font(changed)),
        }
    }

    /// Start decoding the images that came near (on `pool`, nothing waits for it) and drop the ones that went far. True
    /// when some image's bytes are gone from the response cache (they are asked for again).
    pub fn prepare(&mut self, vp: Viewport, scroll: f32, loader: &mut Loader, pool: &mut DecodePool) -> bool {
        let Some(list) = &self.list else { return false };
        let band = Band::new(scroll, vp.h);
        // Far bitmaps go first: their room is what the near ones are started with.
        self.store.drop_far(list, band);
        let prepared = self.store.prepare_async(list, band, vp.dpr, pool, &mut |u| loader.cache_mut().get(u));
        if prepared.missing.is_empty() {
            false
        } else {
            self.res.forget_images(&prepared.missing);
            true
        }
    }

    /// A decode of the pool is done. The rows (device pixels, in the viewport scrolled to `scroll`, `height` of them
    /// visible) that now show a picture, if any is on screen; and whether the page needs a new layout because a picture
    /// that has a size from its header turned out not to decode (a WebP on a system without a WebP codec): it is a
    /// broken image then, shown as its `alt`, as one whose header is unreadable.
    pub fn take_decoded(&mut self, done: Finished, scroll: f32, dpr: f32, height: u32) -> (Option<(u32, u32)>, bool) {
        let accepted = self.store.accept(done);
        let relayout = self.store.take_decode_failed().into_iter().fold(false, |c, n| c | self.break_image(n));
        let Some(list) = self.list.as_ref() else { return (None, relayout) };
        let mut rows: Option<(u32, u32)> = None;
        for p in list.images.iter().filter(|p| accepted.nodes.contains(&p.node)) {
            let (top, bottom) = (p.rect.y.to_f32() - if p.fixed { 0.0 } else { scroll }, p.rect.bottom().to_f32() - if p.fixed { 0.0 } else { scroll });
            // A row of margin each side for the rounding of the rectangle to device pixels.
            let a = ((top * dpr).floor().max(0.0) as u32).saturating_sub(1);
            let b = ((bottom * dpr).ceil().max(0.0) as u32).saturating_add(1).min(height);
            if a < b {
                rows = Some(rows.map_or((a, b), |(r0, r1)| (r0.min(a), r1.max(b))));
            }
        }
        (rows, relayout)
    }

    /// Paint rows `y0 ..` (device pixels) of the viewport at `scroll` into `frame` (a band of the viewport).
    pub fn raster(&mut self, platform: &dyn Platform, vp: Viewport, scroll: f32, y0: u32, frame: &mut Frame) {
        if let Some(list) = &self.list {
            render_band(list, platform, &mut self.store, View { scroll_y: scroll, dpr: vp.dpr }, y0, frame);
        }
    }

    /// The link under the point (CSS px in the viewport): its address resolved against the page's base, and whether it
    /// asks for a new window.
    pub fn link_at(&self, x: f32, y: f32, scroll: f32) -> Option<(Url, bool)> {
        let (index, layout) = (self.index.as_ref()?, self.layout.as_ref()?);
        let node = index.link_at(&self.doc, layout, x, scroll + y, y)?;
        let (href, blank) = link_target(&self.doc, node)?;
        Some((self.res.base().join(&href).ok()?, blank))
    }

    /// The page y (CSS px) of a `#fragment`; the top for an empty one or `top` that matches nothing.
    pub fn fragment_y(&self, fragment: &str) -> Option<f32> {
        if let (Some(node), Some(index), Some(layout)) = (fragment_target(&self.doc, fragment), &self.index, &self.layout)
            && let Some(y) = index.node_y(layout, node)
        {
            return Some(y);
        }
        (fragment.is_empty() || fragment.eq_ignore_ascii_case("top")).then_some(0.0)
    }
}

/// What a sub-resource event did to the page.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PageUpdate {
    Sheet,
    Image(Effect),
    /// A web font arrived or failed; true when the text looks different (lay out again).
    Font(bool),
}
