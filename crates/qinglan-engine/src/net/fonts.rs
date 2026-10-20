//! The web fonts of one page (step 2d-1): which `@font-face` faces to fetch, fetching them through the [`Loader`] as
//! sub-resources (the same mixed-content and `file:` rules as images; no CORS: this browser has no scripts and does not
//! check cross-origin font access, decisions.md "2d-1"), decoding them, handing them to the platform, and the
//! `font-display` clock.
//!
//! The platform keeps the table of faces ([`Platform::set_web_faces`]); each face is `Fallback` (not usable: text goes
//! on to the next family), `Hidden` (loading inside the block period: text in its `unicode-range` takes the room
//! of the fallback font but is not drawn) or loaded.

use std::sync::Arc;
use std::time::{Duration, Instant};

use super::loader::{LoadEvent, LoadRequest, Loader, Ticket};
use super::{ResourceKind, Response};
use crate::css::StyleOutput;
use crate::css::computed::slant_of;
use crate::fonts::face::{Blocks, FontFaceRule, Need, Src, faces_to_fetch};
use crate::fonts::{MAX_PAGE_FONT_BYTES, decode_font};
use crate::html::atoms::FxHashMap;
use std::collections::HashSet;
use crate::html::dom::{Document, NodeData};
use crate::platform::{Platform, WebFaceState};
use crate::url::Url;

/// Faces fetched for one page (a page may declare up to `fonts::face::MAX_FACES`).
pub const MAX_FETCHED_FACES: usize = 32;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum State {
    /// Declared, not needed (or not started).
    Idle,
    Loading,
    Ready,
    Failed,
}

struct Load {
    state: State,
    /// The `src` entry being tried.
    src: usize,
    /// When the request for the face was first made (the `font-display` clock).
    started: Option<Instant>,
    /// The platform is hiding text in this face (block period).
    hidden: bool,
}

#[derive(Default)]
pub struct FontLoads {
    rules: Vec<FontFaceRule>,
    loads: Vec<Load>,
    tickets: FxHashMap<Ticket, usize>,
    /// Decoded bytes handed to the platform for this page.
    decoded: usize,
    fetched: usize,
}

impl FontLoads {
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Faces requested and not yet answered.
    pub fn pending(&self) -> usize {
        self.loads.iter().filter(|l| l.state == State::Loading).count()
    }

    /// Some face is loaded (or a local font stands in for it): styles computed before that measured `ex` and `ch` in
    /// the fallback font.
    pub fn any_ready(&self) -> bool {
        self.loads.iter().any(|l| l.state == State::Ready)
    }

    /// (faces declared, faces loaded, faces that failed) for the reports of the command line.
    pub fn counts(&self) -> (usize, usize, usize) {
        let n = |s: State| self.loads.iter().filter(|l| l.state == s).count();
        (self.rules.len(), n(State::Ready), n(State::Failed))
    }

    pub fn owns(&self, t: Ticket) -> bool {
        self.tickets.contains_key(&t)
    }

    /// Forget everything (a new page, or the page was left): the platform drops its faces.
    pub fn clear(&mut self, platform: &dyn Platform) {
        if !self.rules.is_empty() {
            platform.set_web_faces(&[]);
        }
        *self = FontLoads::default();
    }

    /// Declare the faces to the platform and start fetching those the page's text can use.
    pub fn start(&mut self, doc: &Document, styled: &StyleOutput, page: &Url, use_cache: bool, loader: &mut Loader, platform: &dyn Platform) {
        let now = Instant::now();
        *self = FontLoads::default();
        let descs: Vec<_> = styled.font_faces.iter().map(|r| r.desc.clone()).collect();
        // Also with no faces: that is what drops the fonts of the page that was shown before.
        platform.set_web_faces(&descs);
        if descs.is_empty() {
            return;
        }
        self.rules = styled.font_faces.clone();
        self.loads = self.rules.iter().map(|_| Load { state: State::Idle, src: 0, started: None, hidden: false }).collect();

        let mut blocks = Blocks::default();
        for n in &doc.nodes {
            if let NodeData::Text(t) = &n.data {
                for c in t.chars() {
                    blocks.add(c);
                }
            }
        }
        let mut seen: HashSet<(usize, u32, u8)> = HashSet::new();
        let mut needs: Vec<Need<'_>> = Vec::new();
        for s in styled.tree.styles.iter().flatten() {
            let f = &s.font;
            let k = (std::rc::Rc::as_ptr(&f.family) as usize, f.weight.to_bits(), f.style as u8);
            if seen.insert(k) {
                needs.push(Need { families: f.family.as_slice(), weight: f.weight, slant: slant_of(f.style) });
            }
        }
        let want = faces_to_fetch(&self.rules, &needs, &blocks);
        for i in want.into_iter().take(MAX_FETCHED_FACES) {
            self.begin(i, page, use_cache, loader, platform, now);
        }
    }

    /// Start face `i`: try its sources in order.
    fn begin(&mut self, i: usize, page: &Url, use_cache: bool, loader: &mut Loader, platform: &dyn Platform, now: Instant) {
        if let Some(l) = self.loads.get_mut(i) {
            l.started = Some(now);
        }
        self.next_source(i, page, use_cache, loader, platform);
    }

    fn next_source(&mut self, i: usize, page: &Url, use_cache: bool, loader: &mut Loader, platform: &dyn Platform) {
        loop {
            let src = match self.loads.get(i) {
                Some(l) => l.src,
                None => return,
            };
            let Some(entry) = self.rules.get(i).and_then(|r| r.src.get(src)).cloned() else {
                self.fail(i, platform);
                return;
            };
            if let Some(l) = self.loads.get_mut(i) {
                l.src += 1;
            }
            match entry {
                Src::Local(name) => {
                    if platform.use_local_face(i, &name) {
                        if let Some(l) = self.loads.get_mut(i) {
                            l.state = State::Ready;
                            l.hidden = false;
                        }
                        return;
                    }
                }
                Src::Url(url) => {
                    if self.fetched >= MAX_FETCHED_FACES * 4 {
                        self.fail(i, platform);
                        return;
                    }
                    self.fetched += 1;
                    let req = LoadRequest { url, kind: ResourceKind::Font, page: Some(page.clone()), use_cache };
                    if let Ok(t) = loader.submit(req) {
                        self.tickets.insert(t, i);
                        let block = self.rules.get(i).map_or(Duration::ZERO, |r| r.display.block_period());
                        if let Some(l) = self.loads.get_mut(i) {
                            let first = l.state != State::Loading;
                            l.state = State::Loading;
                            // Text waits invisibly for the block period (none for `swap`).
                            if first && !block.is_zero() {
                                l.hidden = true;
                                platform.set_web_face_state(i, WebFaceState::Hidden);
                            }
                        }
                        return;
                    }
                }
            }
        }
    }

    fn fail(&mut self, i: usize, platform: &dyn Platform) {
        if let Some(l) = self.loads.get_mut(i) {
            l.state = State::Failed;
            l.hidden = false;
        }
        platform.set_web_face_state(i, WebFaceState::Fallback);
    }

    /// A font response arrived. True when what the page looks like changed (a face became usable, or text that was
    /// invisible is shown in the fallback font).
    pub fn handle(&mut self, ev: LoadEvent, page: &Url, use_cache: bool, loader: &mut Loader, platform: &dyn Platform, now: Instant) -> bool {
        let Some(i) = self.tickets.remove(&ev.ticket) else { return false };
        let was_hidden = self.loads.get(i).is_some_and(|l| l.hidden);
        let ok = match ev.result {
            Ok(resp) => self.accept(i, &resp, platform, now),
            Err(_) => false,
        };
        if ok {
            if let Some(l) = self.loads.get_mut(i) {
                l.state = State::Ready;
                l.hidden = false;
            }
            return true;
        }
        // Not usable: the next source, or the fallback font.
        self.next_source(i, page, use_cache, loader, platform);
        let now_hidden = self.loads.get(i).is_some_and(|l| l.hidden);
        was_hidden && !now_hidden
    }

    fn accept(&mut self, i: usize, resp: &Arc<Response>, platform: &dyn Platform, now: Instant) -> bool {
        if !(200..300).contains(&resp.status) {
            return false;
        }
        // `font-display: optional` (and `fallback` after its swap period): a font that arrives too late is not used.
        if let (Some(r), Some(l)) = (self.rules.get(i), self.loads.get(i))
            && let (Some(swap), Some(t0)) = (r.display.swap_period(), l.started)
            && now.saturating_duration_since(t0) > r.display.block_period() + swap
        {
            return false;
        }
        let Ok(sfnt) = decode_font(&resp.body) else { return false };
        if self.decoded + sfnt.len() > MAX_PAGE_FONT_BYTES {
            return false;
        }
        if platform.load_web_face(i, &sfnt) {
            self.decoded += sfnt.len();
            true
        } else {
            false
        }
    }

    /// The block periods that have run out: their text is shown in the fallback font. True when anything changed.
    pub fn tick(&mut self, now: Instant, platform: &dyn Platform) -> bool {
        let mut changed = false;
        for (i, l) in self.loads.iter_mut().enumerate() {
            if l.hidden
                && let (Some(t0), Some(r)) = (l.started, self.rules.get(i))
                && now.saturating_duration_since(t0) >= r.display.block_period()
            {
                l.hidden = false;
                platform.set_web_face_state(i, WebFaceState::Fallback);
                changed = true;
            }
        }
        changed
    }

    /// When the next block period ends.
    pub fn next_deadline(&self) -> Option<Instant> {
        self.loads.iter().enumerate().filter(|(_, l)| l.hidden).filter_map(|(i, l)| Some(l.started? + self.rules.get(i)?.display.block_period())).min()
    }
}
