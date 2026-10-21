//! The browser: what the window's events do. One [`Browser`] lives on the UI thread; the network runs on the loader's
//! threads and reaches it through the wake-up message, the UI thread never waits for it.

use qinglan_engine::fmath::Fclamp;
use std::any::Any;
use std::io::Write as _;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use qinglan_engine::encoding::Encoding;
use qinglan_engine::html::treebuilder::parse_document;
use qinglan_engine::net::blocking::{self, OpenResult, Opened};
use qinglan_engine::net::content::ContentKind;
use qinglan_engine::net::loader::{LoadEvent, LoadRequest, Loader, Ticket};
use qinglan_engine::net::policy::{self, Refusal};
use qinglan_engine::net::{NetError, ResourceKind};
use qinglan_engine::paint::decode::{DECODE_WORKERS, DecodePool};
use qinglan_engine::paint::{LANE, Press, ScrollAxis};
use qinglan_engine::paint::raster::Frame;
use qinglan_engine::url::{Url, UrlPath};
use qinglan_win::{WicDecoder, WinPlatform};
use qinglan_win::window::{App, Command, Event, MouseButton, ScrollCode, Ui};

use crate::address;
use crate::args::Options;
use crate::bench::{Bench, Report, Stage, ms};
use crate::errorpage::{self, Failure};
use crate::history::History;
use crate::i18n::{Lang, Strings, fill, window_title};
use crate::page::{Effect, Page, PageUpdate, Phase, Viewport};
use crate::throttle::{Plan, Throttle};

const TIMER_CSS: usize = 1;
const TIMER_RELAYOUT: usize = 2;
const TIMER_BENCH: usize = 3;
const TIMER_BENCH_STEP: usize = 4;
/// The end of a `font-display` block period: invisible text is shown in the fallback font.
const TIMER_FONT: usize = 5;
const USER_BENCH_STEP: u32 = 1;
/// How long the page waits for the style sheets of `<head>` (L1 task 17).
const CSS_WAIT_MS: u32 = 5000;
/// Images change the layout at most this often.
const RELAYOUT_GAP: Duration = Duration::from_millis(100);
/// A relayout caused by an image waits until this long after the last scroll input (step 2-0).
const RELAYOUT_QUIET: Duration = Duration::from_millis(150);
/// A wheel "line" in CSS px (Chrome scrolls 100 px for the default 3 lines).
const LINE_PX: f32 = 100.0 / 3.0;
const ARROW_PX: f32 = 40.0;
const PAGE_FRACTION: f32 = 0.875;
const BENCH_SCROLL_STEP: f32 = 100.0;
const BENCH_TIMEOUT_MS: u32 = 240_000;
/// The bench scrolls the way a hand on a wheel does: `BENCH_BURST` notches of `BENCH_SCROLL_STEP` px, one every timer
/// tick (`BENCH_STEP_MS`, the system rounds it up to 16 or 31 ms), then the reader looks for `BENCH_PAUSE_MS`.
const BENCH_BURST: u32 = 10;
const BENCH_STEP_MS: u32 = 16;
const BENCH_PAUSE_MS: u32 = 200;
/// At the bottom the bench waits this long at most for the layout and the images on their way.
const BENCH_SETTLE_MS: u128 = 5000;
/// A page of 2^22 px scrolled 100 px at a time has 42 000 frames; more than this means the walk is stuck.
const MAX_BENCH_FRAMES: usize = 100_000;
/// Image arrivals may start this many layouts per page; a page whose images keep changing their own layout (a `srcset`
/// choice that depends on the width the previous choice made) would otherwise lay out for ever. The sizes learned
/// after the limit are used by the next layout the window asks for (a resize).
const MAX_IMAGE_RELAYOUTS: u32 = 24;

const VK_RETURN: u32 = 0x0D;
const VK_ESCAPE: u32 = 0x1B;
const VK_SPACE: u32 = 0x20;
const VK_PRIOR: u32 = 0x21;
const VK_NEXT: u32 = 0x22;
const VK_END: u32 = 0x23;
const VK_HOME: u32 = 0x24;
const VK_LEFT: u32 = 0x25;
const VK_UP: u32 = 0x26;
const VK_RIGHT: u32 = 0x27;
const VK_DOWN: u32 = 0x28;
const VK_L: u32 = 0x4C;
const VK_N: u32 = 0x4E;
const VK_R: u32 = 0x52;
const VK_F5: u32 = 0x74;
const VK_F6: u32 = 0x75;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum NavKind {
    /// Typed address or clicked link: a new history entry.
    New,
    /// Fetch again, keep the place.
    Reload,
    /// Back or forward: answered from the response cache when possible, the place is restored.
    History,
}

/// Who started a navigation. It decides which schemes may be opened: only the user may open `file:`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Origin {
    /// Typed address, command line, history, reload: the user asked for exactly this address.
    User,
    /// A link (or `target=_blank`, or a new window for a link) on the page on screen: page-controlled.
    Page,
}

/// May `url` be opened as a page, given who asked? A navigation the page started carries the page's
/// address, so the scheme policy refuses `file:` (a link, `<base href="file:///...">`, ...).
fn plan_navigation(url: &Url, origin: Origin, page: Option<&Url>) -> Result<policy::Plan, Refusal> {
    match origin {
        Origin::User => policy::plan(url, ResourceKind::Document, None),
        Origin::Page => {
            let blank = blank_url();
            policy::plan(url, ResourceKind::Document, Some(page.unwrap_or(&blank)))
        }
    }
}

/// A scroll bar held with the left button.
#[derive(Clone, Copy)]
enum Drag {
    Scope { scope: u32, axis: u8, grab: f32 },
    Page { grab: f32 },
}

struct Nav {
    ticket: Ticket,
    url: Url,
    kind: NavKind,
    scroll: f32,
}

/// Run `f`; a panic comes back as its message (a stack overflow cannot be caught: see the 8 MB stack).
pub fn catch<R>(f: impl FnOnce() -> R) -> Result<R, String> {
    catch_unwind(AssertUnwindSafe(f)).map_err(panic_text)
}

fn panic_text(p: Box<dyn Any + Send>) -> String {
    if let Some(s) = p.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = p.downcast_ref::<String>() {
        s.clone()
    } else {
        "panic".to_string()
    }
}

/// A frame buffer holds this many bytes at most: the page is drawn and copied to the window a band of rows at a time
/// (a full 1920 x 1080 frame is 8 MB, a band 2 MB, four bands).
const BAND_BYTES: u32 = 2 << 20;

/// Rows of a band for a viewport `w` x `h` device pixels.
fn band_rows(w: u32, h: u32) -> u32 {
    (BAND_BYTES / (w.max(1) * 4)).fclamp(64, h.max(64))
}

/// `about:blank`, made by hand: the address of an error page whose own address cannot be parsed.
fn blank_url() -> Url {
    Url { scheme: "about".into(), username: String::new(), password: String::new(), host: None, port: None, path: UrlPath::Opaque("blank".into()), query: None, fragment: None }
}

pub struct Browser {
    ui: Ui,
    s: &'static Strings,
    opts: Options,
    loader: Loader,
    /// Decodes images off the UI thread; its results arrive with the loader's wake-up (step 2-0).
    pool: DecodePool,
    platform: Option<Rc<WinPlatform>>,
    history: History,
    nav: Option<Nav>,
    page: Option<Page>,
    /// The page area in device pixels, and device pixels per CSS px.
    dev: (u32, u32),
    dpr: f32,
    /// Scroll position in CSS px (a whole number of device pixels).
    scroll: f32,
    /// How far a horizontal page is scrolled sideways (CSS px).
    cross: f32,
    /// The pointer in the page area (device px), a scroll bar being dragged, a press on a bar that is not a click.
    mouse: (i32, i32),
    drag: Option<Drag>,
    swallow: bool,
    vbar: std::cell::Cell<bool>,
    frame: Frame,
    relayout: Throttle,
    /// The viewport changed: styles (media queries) and layout are made again.
    restyle: bool,
    /// Layouts made because images arrived since the page was opened or the window last changed.
    image_relayouts: u32,
    hover: Option<Url>,
    /// A message that stays in the status bar until the next navigation ("stopped").
    note: Option<String>,
    last_status: String,
    last_loading: bool,
    bench: Option<Bench>,
    event_start: Instant,
    /// Time the current event has spent in the GDI calls that put pixels on the window (the system's part of a frame).
    present_ms: f64,
    prewarm: Option<JoinHandle<usize>>,
}

impl Browser {
    pub fn new(ui: Ui, opts: Options, loader: Loader, lang: Lang) -> Browser {
        let bench = opts.bench.clone().map(Bench::new);
        let dpr = bench.as_ref().map_or(ui.dpi() as f32 / 96.0, |b| b.opts.dpr);
        let pool = DecodePool::new(Arc::new(WicDecoder), DECODE_WORKERS, loader.waker());
        Browser {
            ui,
            s: lang.strings(),
            opts,
            loader,
            pool,
            platform: None,
            history: History::new(),
            nav: None,
            page: None,
            dev: (0, 0),
            dpr,
            scroll: 0.0,
            cross: 0.0,
            mouse: (-1, -1),
            drag: None,
            swallow: false,
            vbar: std::cell::Cell::new(true),
            frame: Frame::new(0, 0),
            relayout: Throttle::new(RELAYOUT_GAP, RELAYOUT_QUIET),
            restyle: false,
            image_relayouts: 0,
            hover: None,
            note: None,
            last_status: String::new(),
            last_loading: false,
            bench,
            event_start: Instant::now(),
            present_ms: 0.0,
            prewarm: None,
        }
    }

    fn vp(&self) -> Viewport {
        Viewport { w: self.dev.0 as f32 / self.dpr, h: self.dev.1 as f32 / self.dpr, dpr: self.dpr }
    }

    // ---- events -------------------------------------------------------------------------------------------

    fn handle(&mut self, ev: Event) -> bool {
        match ev {
            Event::Ready => self.on_ready(),
            Event::Size { width, height } => self.on_size(width, height),
            Event::Dpi { dpi } => self.on_dpi(dpi),
            Event::Paint => self.paint(),
            Event::MouseMove { x, y } => self.on_mouse_move(x, y),
            Event::MouseLeave => self.set_hover(None),
            Event::MouseDown { x, y } => self.on_mouse_down(x, y),
            Event::MouseUp => self.drag = None,
            Event::Click { x, y, button, ctrl, shift } => self.on_click(x, y, button, ctrl, shift),
            Event::Wheel { notches, lines, ctrl, shift } => self.on_wheel(notches, lines, ctrl, shift),
            Event::Key { vk, ctrl, shift, alt, in_edit } => return self.on_key(vk, ctrl, shift, alt, in_edit),
            Event::Scroll(code) => self.on_scroll_code(code),
            Event::Command(c) => self.on_command(c),
            Event::Timer(id) => self.on_timer(id),
            Event::Wake => self.on_wake(),
            Event::User(n) => {
                if n == USER_BENCH_STEP {
                    self.bench_step();
                }
            }
            Event::Close => {}
        }
        false
    }

    fn on_ready(&mut self) {
        if let Some(b) = self.bench.as_mut() {
            b.startup_ms = qinglan_win::window::since_process_creation_ms();
        }
        self.mem_mark("ready");
        self.prewarm = std::thread::Builder::new().name("prewarm".into()).stack_size(8 << 20).spawn(qinglan_win::prewarm).ok();
        let start = if let Some(b) = &self.bench {
            let (w, h) = ((b.opts.width * b.opts.dpr).round() as u32, (b.opts.height * b.opts.dpr).round() as u32);
            self.ui.set_page_size(w, h);
            self.ui.set_timer(TIMER_BENCH, BENCH_TIMEOUT_MS);
            // A replay answers at once; the real network takes longer than the 5 ms the prewarm needs, so it is
            // finished before the content arrives (L1 task 17). Wait for it here to get the same order.
            if let Some(h) = self.prewarm.take() {
                let _ = h.join();
            }
            Some(b.opts.address.clone())
        } else {
            self.opts.address.clone()
        };
        self.mem_mark("prewarmed");
        self.platform = WinPlatform::new().map(Rc::new);
        self.mem_mark("platform");
        self.ui.set_nav_enabled(false, false);
        self.ui.set_loading(false);
        if self.platform.is_none() {
            self.note = Some(self.s.no_font_support.to_string());
        }
        match start {
            Some(a) => {
                self.open_typed(&a);
                // An address given on the command line is read, not edited: keys go to the page.
                self.ui.focus_page();
            }
            None => {
                self.ui.set_title(self.s.app_name);
                self.ui.focus_address();
                self.paint();
            }
        }
    }

    fn on_size(&mut self, w: u32, h: u32) {
        if w == 0 || h == 0 || (w, h) == self.dev {
            return;
        }
        self.dev = (w, h);
        self.image_relayouts = 0;
        if self.page.as_ref().is_some_and(|p| p.layout.is_some()) {
            self.restyle = true;
            self.relayout.request();
            self.plan_relayout();
        }
        self.update_scrollbar();
        self.ui.invalidate();
    }

    fn on_dpi(&mut self, dpi: u32) {
        if self.bench.is_some() {
            return;
        }
        let dpr = dpi as f32 / 96.0;
        if (dpr - self.dpr).abs() < 0.001 {
            return;
        }
        self.dpr = dpr;
        self.image_relayouts = 0;
        if self.page.as_ref().is_some_and(|p| p.layout.is_some()) {
            self.restyle = true;
            self.relayout.request();
            self.plan_relayout();
        }
        self.ui.invalidate();
    }

    fn on_timer(&mut self, id: usize) {
        match id {
            TIMER_CSS => {
                self.ui.kill_timer(TIMER_CSS);
                if self.page.as_ref().is_some_and(|p| p.phase == Phase::Sheets) {
                    self.begin_display();
                }
            }
            TIMER_RELAYOUT => {
                self.ui.kill_timer(TIMER_RELAYOUT);
                self.plan_relayout();
            }
            TIMER_FONT => {
                self.ui.kill_timer(TIMER_FONT);
                self.font_tick();
            }
            TIMER_BENCH => self.bench_fail("timed out"),
            TIMER_BENCH_STEP => {
                self.ui.kill_timer(TIMER_BENCH_STEP);
                self.bench_step();
            }
            _ => {}
        }
    }

    fn on_wake(&mut self) {
        for ev in self.loader.poll() {
            self.on_load_event(ev);
        }
        self.on_decoded();
    }

    /// Images the pool finished: the page takes the ones it still wants into its cache and the rows they cover are drawn
    /// again (nothing else is). A result of a page that is gone is dropped.
    fn on_decoded(&mut self) {
        let (scroll, vp, height) = (self.scroll, self.vp(), self.dev.1);
        let mut rows: Option<(u32, u32)> = None;
        let mut again = false;
        let mut relayout = false;
        for done in self.pool.poll() {
            let Some(page) = self.page.as_mut() else { continue };
            let (taken, broke) = page.take_decoded(done, scroll, vp, height);
            if let Some((a, b)) = taken {
                rows = Some(rows.map_or((a, b), |(r0, r1)| (r0.min(a), r1.max(b))));
            }
            relayout |= broke;
            again |= page.store.deferred();
        }
        if let Some((a, b)) = rows {
            self.draw_rows(scroll, a, b);
        }
        if relayout {
            self.request_image_relayout();
        }
        // A decode that finished made room for one that had to wait.
        if again {
            self.ui.invalidate();
        }
    }

    fn on_load_event(&mut self, ev: LoadEvent) {
        if self.nav.as_ref().is_some_and(|n| n.ticket == ev.ticket) {
            if let Some(b) = self.bench.as_mut() {
                b.html_arrived(self.event_start);
            }
            self.on_document(ev);
            return;
        }
        let Some(platform) = self.platform.clone() else { return };
        let Some(page) = self.page.as_mut() else { return };
        let update = page.handle_update(&mut self.loader, ev, &*platform);
        match update {
            Some(PageUpdate::Sheet) => {
                if page.phase == Phase::Sheets && page.res.sheets_pending() == 0 {
                    self.begin_display();
                }
            }
            Some(PageUpdate::Image(Effect::Relayout)) => self.request_image_relayout(),
            Some(PageUpdate::Image(Effect::Repaint)) => self.ui.invalidate(),
            Some(PageUpdate::Font(changed)) => {
                if changed {
                    self.relayout.request();
                    self.plan_relayout();
                }
                self.arm_font_timer();
            }
            None => {}
        }
    }

    // ---- navigation ---------------------------------------------------------------------------------------

    /// Open what was typed in the address bar.
    fn open_typed(&mut self, text: &str) {
        let Some(addr) = address::classify(text) else { return };
        match Url::parse(&addr, None) {
            Ok(url) => self.navigate(url, NavKind::New, 0.0, Origin::User),
            Err(_) => self.show_failure(Failure::BadAddress, addr, NavKind::New),
        }
    }

    fn is_same_document(&self, url: &Url, kind: NavKind) -> bool {
        let Some(page) = &self.page else { return false };
        kind != NavKind::Reload && !page.is_error && page.layout.is_some() && (kind == NavKind::History || url.fragment.is_some()) && page.url.serialize(true) == url.serialize(true)
    }

    fn navigate(&mut self, url: Url, kind: NavKind, scroll: f32, origin: Origin) {
        if self.is_same_document(&url, kind) {
            self.same_document(url, kind, scroll);
            return;
        }
        // Refusals (mailto:, javascript:, ...) leave the page and its loading alone.
        if let Err(refusal) = plan_navigation(&url, origin, self.page.as_ref().map(|p| &p.url)) {
            self.on_refusal(refusal, url, kind);
            return;
        }
        if self.platform.is_none() {
            self.note = Some(self.s.no_font_support.to_string());
            return;
        }
        if kind == NavKind::New {
            self.history.save_scroll(self.scroll);
        }
        self.cancel_loading();
        self.note = None;
        let req = LoadRequest { url: url.clone(), kind: ResourceKind::Document, page: None, use_cache: kind == NavKind::History };
        match self.loader.submit(req) {
            Ok(ticket) => {
                self.ui.set_address(&url.href());
                self.nav = Some(Nav { ticket, url, kind, scroll });
            }
            Err(refusal) => self.on_refusal(refusal, url, kind),
        }
    }

    /// Stop everything that is in flight: the page request, the images and sheets of the page on screen.
    fn cancel_loading(&mut self) {
        self.loader.cancel_all();
        self.nav = None;
        if let Some(p) = self.page.as_mut() {
            p.res.abandon_pending();
        }
        self.ui.kill_timer(TIMER_CSS);
    }

    fn on_refusal(&mut self, refusal: Refusal, url: Url, kind: NavKind) {
        match refusal {
            Refusal::Mailto => {
                // What is shown is exactly what is handed over, rebuilt so that it cannot carry arguments.
                let addr = policy::sanitize_mailto(&url);
                if self.ui.confirm(self.s.mailto_caption, &fill(self.s.mailto_text, &addr)) {
                    self.ui.open_external(&addr);
                }
            }
            Refusal::Javascript => self.note = Some(self.s.js_blocked.to_string()),
            other => self.show_failure(Failure::Refused(other), url.href(), kind),
        }
        if let Some(p) = &self.page {
            self.ui.set_address(&p.display);
        }
    }

    /// `#fragment` navigation inside the page that is open, or back/forward between two places in it.
    fn same_document(&mut self, url: Url, kind: NavKind, scroll: f32) {
        self.note = None;
        match kind {
            NavKind::New => {
                self.history.save_scroll(self.scroll);
                self.history.push(url.href());
                let vw = self.vp().w;
                let y = url.fragment.as_deref().and_then(|f| self.page.as_ref().and_then(|p| p.fragment_y(f, vw)));
                if let Some(y) = y {
                    self.scroll_to(y);
                }
            }
            NavKind::History | NavKind::Reload => self.scroll_to(scroll),
        }
        if let Some(p) = self.page.as_mut() {
            p.display = url.href();
        }
        self.ui.set_address(&url.href());
        self.update_nav_buttons();
    }

    /// The page's bytes are here and decoded: replace the page on screen with the new one.
    fn commit(&mut self, kind: NavKind, scroll: f32, fragment: Option<String>, opened: Opened, display: String, is_error: bool) {
        // The old page goes first: DOM, styles, layout, display list and decoded images are freed before the new
        // page is built (L1 task 17). Decodes of its images that have not started are called off; those that run
        // finish and are dropped (the new page's store does not know them).
        self.pool.cancel_all();
        self.page = None;
        self.hover = None;
        self.ui.set_hand_cursor(false);
        self.relayout.reset();
        self.restyle = false;
        self.image_relayouts = 0;
        self.ui.kill_timer(TIMER_RELAYOUT);
        self.scroll = 0.0;
        self.cross = 0.0;
        match kind {
            NavKind::New => self.history.push(display.clone()),
            NavKind::Reload | NavKind::History => self.history.set_current_url(display.clone()),
        }
        let mut page = Page::from_opened(opened, display.clone(), is_error);
        self.mem_mark("parsed");
        page.restore = (kind != NavKind::New && !is_error).then_some(scroll);
        page.fragment = fragment.filter(|f| !f.is_empty());
        page.res.use_cache |= kind == NavKind::History;
        self.ui.set_title(&window_title(&page.title, self.s));
        self.ui.set_address(&display);
        self.ui.set_secure(if page.url.scheme == "http" && !is_error { self.s.insecure } else { "" });
        self.ui.show_notice(None);
        self.update_nav_buttons();
        let vp = self.vp();
        let sopts = qinglan_engine::css::StyleOptions { viewport_w: vp.w, viewport_h: vp.h, ..Default::default() };
        let pending = page.res.start_sheets(&page.doc, &mut self.loader, &sopts);
        self.page = Some(page);
        if pending == 0 {
            self.begin_display();
        } else {
            self.ui.set_timer(TIMER_CSS, CSS_WAIT_MS);
        }
    }

    fn on_document(&mut self, ev: LoadEvent) {
        let Some(nav) = self.nav.take() else { return };
        let Some(platform) = self.platform.clone() else { return };
        let typed = nav.url.href();
        let resp = match ev.result {
            Ok(r) => r,
            Err(e) => {
                self.show_failure(Failure::Net(e), typed, nav.kind);
                return;
            }
        };
        if resp.status >= 400 && resp.body.is_empty() {
            self.show_failure(Failure::EmptyStatus(resp.status), typed, nav.kind);
            return;
        }
        match blocking::open_response(&resp, &*platform) {
            OpenResult::Page(o) => {
                let mut shown = o.url.clone();
                shown.fragment = nav.url.fragment.clone();
                self.commit(nav.kind, nav.scroll, nav.url.fragment.clone(), *o, shown.href(), false);
            }
            OpenResult::Download { mime, .. } => self.show_failure(Failure::Unsupported { mime }, typed, nav.kind),
            OpenResult::Failed(why) => self.show_failure(Failure::Net(NetError::Other(why)), typed, nav.kind),
        }
    }

    /// Show the built-in error page for `failure` (made by our own engine like any page).
    fn show_failure(&mut self, failure: Failure, display: String, kind: NavKind) {
        let ep = errorpage::build(&failure, &display, self.s);
        let url = Url::parse(&display, None).unwrap_or_else(|_| blank_url());
        let opened = Opened { url, status: 0, doc: parse_document(ep.html), encoding: Encoding::UTF_8, kind: ContentKind::Html, warning: None };
        self.commit(kind, 0.0, None, opened, display, true);
    }

    /// Something panicked while loading, laying out or painting: say so in the window instead of dying.
    fn internal_error(&mut self, text: String) {
        eprintln!("qinglan: internal error: {text}");
        let (was_error, display) = self.page.as_ref().map_or((false, String::new()), |p| (p.is_error, p.display.clone()));
        self.cancel_loading();
        self.pool.cancel_all();
        self.page = None;
        if was_error {
            // The error page itself failed; show nothing rather than loop.
            self.paint();
            return;
        }
        let kind = if self.history.is_empty() { NavKind::New } else { NavKind::Reload };
        if catch(|| self.show_failure(Failure::Internal(text), display, kind)).is_err() {
            self.page = None;
            self.paint();
        }
    }

    fn reload(&mut self) {
        let Some(display) = self.page.as_ref().map(|p| p.display.clone()) else { return };
        let scroll = self.scroll;
        match Url::parse(&display, None) {
            Ok(url) => self.navigate(url, NavKind::Reload, scroll, Origin::User),
            Err(_) => self.show_failure(Failure::BadAddress, display, NavKind::Reload),
        }
    }

    fn go_history(&mut self, back: bool) {
        self.history.save_scroll(self.scroll);
        let Some(entry) = (if back { self.history.back() } else { self.history.forward() }) else { return };
        self.update_nav_buttons();
        match Url::parse(&entry.url, None) {
            Ok(url) => self.navigate(url, NavKind::History, entry.scroll, Origin::User),
            Err(_) => self.show_failure(Failure::BadAddress, entry.url, NavKind::History),
        }
    }

    fn stop(&mut self) {
        let waiting_for_sheets = self.page.as_ref().is_some_and(|p| p.phase == Phase::Sheets);
        let was_loading = self.is_loading();
        self.cancel_loading();
        if waiting_for_sheets {
            self.begin_display();
        }
        if was_loading {
            self.note = Some(self.s.stopped.to_string());
        }
        if let Some(p) = &self.page {
            self.ui.set_address(&p.display);
        }
    }

    fn update_nav_buttons(&self) {
        self.ui.set_nav_enabled(self.history.can_back(), self.history.can_forward());
    }

    fn open_in_new_window(&mut self, url: &Url) {
        // A new window starts with its address as if typed, so the page-started rules are applied here.
        if let Err(refusal) = plan_navigation(url, Origin::Page, self.page.as_ref().map(|p| &p.url)) {
            self.on_refusal(refusal, url.clone(), NavKind::New);
            return;
        }
        self.spawn_window(Some(&url.href()));
    }

    /// A new window is a new process (L1 task 17).
    fn spawn_window(&self, address: Option<&str>) {
        let Ok(exe) = std::env::current_exe() else { return };
        let mut args: Vec<&std::ffi::OsStr> = Vec::new();
        if let Some(a) = address {
            args.push(a.as_ref());
        }
        if let Some(r) = &self.opts.replay {
            args.push("--replay".as_ref());
            args.push(r.as_ref());
        }
        let _ = qinglan_win::spawn_process(&exe, &args);
    }

    // ---- showing the page ---------------------------------------------------------------------------------

    /// The HTML and the `<head>` sheets are in (or the wait ran out): style, lay out the first screen and show it,
    /// then lay out the rest.
    fn begin_display(&mut self) {
        self.ui.kill_timer(TIMER_CSS);
        let Some(platform) = self.platform.clone() else { return };
        let vp = self.vp();
        let Some(page) = self.page.as_mut() else { return };
        if page.phase == Phase::Shown {
            return;
        }
        page.phase = Phase::Shown;
        page.style(&*platform, vp);
        page.start_fonts(&mut self.loader, &*platform);
        let restore = page.restore.take();
        // A saved position wins over a fragment.
        let fragment = if restore.is_some() { None } else { page.fragment.take() };
        let first_scroll = restore.unwrap_or(0.0).max(0.0);
        // First-screen-first: lay out only to one screen below the viewport (from the place to be shown), then paint.
        // A fragment's place is not known before the layout, so those pages are laid out whole at once.
        let stop = fragment.is_none().then_some(first_scroll + 2.0 * vp.h);
        page.layout(&*platform, vp, stop);
        let (partial, max) = (page.layout.as_ref().is_some_and(|l| l.partial), page.max_scroll(vp));
        self.scroll = self.snap(if partial { first_scroll } else { first_scroll.min(max) });
        self.update_scrollbar();
        self.paint();
        self.mark_first_screen();
        self.mem_mark("first-screen");
        // Start the images of the first screen, then make the rest of the layout while they come.
        self.request_images();
        if partial && let Some(page) = self.page.as_mut() {
            page.layout(&*platform, vp, None);
        }
        let Some(page) = self.page.as_ref() else { return };
        let max = page.max_scroll(vp);
        let target = match &fragment {
            Some(f) => page.fragment_y(f, vp.w).unwrap_or(0.0),
            None => self.scroll,
        };
        self.scroll = self.snap(target.fclamp(0.0, max));
        self.mark_full_layout();
        self.mem_mark("full-layout");
        self.update_scrollbar();
        self.update_notice();
        self.request_images();
        self.arm_font_timer();
        self.ui.invalidate();
        if fragment.is_some() {
            self.paint();
        }
    }

    /// Wake up when the next `font-display` block period ends.
    fn arm_font_timer(&mut self) {
        let deadline = self.page.as_ref().and_then(|p| p.res.fonts.next_deadline());
        match deadline {
            Some(d) => {
                let ms = d.saturating_duration_since(Instant::now()).as_millis().min(60_000) as u32;
                self.ui.set_timer(TIMER_FONT, ms.max(1));
            }
            None => self.ui.kill_timer(TIMER_FONT),
        }
    }

    /// A block period ran out: show that text in the fallback font.
    fn font_tick(&mut self) {
        let Some(platform) = self.platform.clone() else { return };
        let changed = self.page.as_mut().is_some_and(|p| p.res.fonts.tick(Instant::now(), &*platform));
        if changed {
            self.relayout.request();
            self.plan_relayout();
        }
        self.arm_font_timer();
    }

    fn mark_first_screen(&mut self) {
        let (start, now) = (self.event_start, Instant::now());
        if let Some(b) = self.bench.as_mut()
            && b.first_screen.is_none()
        {
            b.first_screen = b.span(start, now);
        }
    }

    fn mark_full_layout(&mut self) {
        let (start, now) = (self.event_start, Instant::now());
        if let Some(b) = self.bench.as_mut()
            && b.full_layout.is_none()
        {
            b.full_layout = b.span(start, now);
            b.counting = false;
        }
    }

    /// Round a scroll position to whole device pixels.
    fn snap(&self, y: f32) -> f32 {
        (y * self.dpr).round() / self.dpr
    }

    /// Draw the page at `scroll` into the window, a band of rows at a time through the one small frame buffer.
    fn draw_bands(&mut self, scroll: f32) {
        self.draw_rows(scroll, 0, self.dev.1);
    }

    /// Draw rows `r0..r1` (device pixels) of the page at `scroll` into the window, a band at a time.
    fn draw_rows(&mut self, scroll: f32, r0: u32, r1: u32) {
        let (w, h) = self.dev;
        let (vp, cross) = (self.vp(), self.cross);
        let rows = band_rows(w, h);
        let platform = self.platform.clone();
        let (mut y0, end) = (r0, r1.min(h));
        while y0 < end {
            let bh = rows.min(end - y0);
            self.frame.set_size(w, bh);
            match (&platform, self.page.as_mut()) {
                (Some(p), Some(page)) if page.list.is_some() => page.raster(&**p, vp, (scroll, cross), y0, &mut self.frame),
                _ => self.frame.data.fill(255),
            }
            let t = Instant::now();
            self.ui.blit_at(y0, w, bh, &self.frame.data);
            self.present_ms += ms(t.elapsed());
            y0 += bh;
        }
    }

    fn paint(&mut self) {
        let (w, h) = self.dev;
        if w == 0 || h == 0 {
            return;
        }
        let (vp, scroll) = (self.vp(), self.scroll);
        let mut need_images = false;
        if self.platform.is_some()
            && let Some(page) = self.page.as_mut()
            && page.list.is_some()
        {
            need_images = page.prepare(vp, scroll, &mut self.loader, &mut self.pool);
        }
        self.draw_bands(scroll);
        if self.bench.as_ref().is_some_and(|b| b.first_screen.is_none()) {
            // The clock for the first screen stops when the pixels are on the window, not when the call returned
            // to a queue.
            self.ui.flush();
        }
        if need_images {
            self.request_images();
        }
    }

    fn request_images(&mut self) {
        let (vp, scroll) = (self.vp(), self.scroll);
        let Some(page) = self.page.as_mut() else { return };
        if page.request_images(&mut self.loader, vp, scroll, false) {
            self.request_image_relayout();
        }
    }

    /// An image changed what the layout needs: lay out again (once for all that arrive meanwhile, and not while the user
    /// scrolls), at most [`MAX_IMAGE_RELAYOUTS`] times per page.
    fn request_image_relayout(&mut self) {
        if self.image_relayouts >= MAX_IMAGE_RELAYOUTS {
            self.ui.invalidate();
            return;
        }
        self.relayout.request();
        self.plan_relayout();
    }

    fn plan_relayout(&mut self) {
        match self.relayout.plan(Instant::now()) {
            Plan::Nothing => {}
            Plan::RunNow => self.do_relayout(),
            Plan::WaitMs(n) => self.ui.set_timer(TIMER_RELAYOUT, n),
        }
    }

    /// Lay the page out again with what is known now, keeping the first visible thing where it is.
    fn do_relayout(&mut self) {
        self.ui.kill_timer(TIMER_RELAYOUT);
        let Some(platform) = self.platform.clone() else { return };
        let (vp, scroll) = (self.vp(), self.scroll);
        let restyle = std::mem::take(&mut self.restyle);
        let Some(page) = self.page.as_mut() else {
            self.relayout.reset();
            return;
        };
        if page.phase != Phase::Shown || page.layout.is_none() {
            self.relayout.reset();
            return;
        }
        if restyle {
            page.style(&*platform, vp);
        }
        let started = Instant::now();
        let anchored = page.relayout(&*platform, vp, scroll);
        if !restyle {
            self.image_relayouts += 1;
        }
        if let Some(b) = self.bench.as_mut() {
            b.relayout_done(ms(started.elapsed()));
        }
        let (max, max_cross) = (page.max_scroll(vp), page.max_cross(vp));
        self.scroll = self.snap(anchored.fclamp(0.0, max));
        self.cross = self.cross.min(max_cross);
        self.relayout.done(Instant::now());
        self.update_scrollbar();
        self.update_notice();
        self.request_images();
        self.ui.invalidate();
    }

    fn update_notice(&self) {
        let Some(page) = &self.page else { return };
        let notice = if page.layout.as_ref().is_some_and(|l| l.incomplete) {
            Some(self.s.notice_incomplete.to_string())
        } else if page.doc.too_large {
            Some(self.s.notice_too_large.to_string())
        } else if page.sheets_missing > 0 && !page.is_error {
            Some(fill(self.s.notice_sheets, &page.sheets_missing.to_string()))
        } else {
            None
        };
        self.ui.show_notice(notice.as_deref());
    }

    // ---- scrolling and the pointer ------------------------------------------------------------------------

    /// The way the page scrolls (down, or along the lines of a vertical page) and the viewport's size that way (CSS px).
    fn axis(&self) -> ScrollAxis {
        self.page.as_ref().map_or(ScrollAxis::Down, |p| p.axis())
    }

    fn axis_extent(&self) -> f32 {
        let vp = self.vp();
        if self.axis() == ScrollAxis::Down { vp.h } else { vp.w }
    }

    fn update_scrollbar(&self) {
        let page_h = self.dev.1;
        // A page that scrolls along its lines has no vertical bar (its horizontal one is drawn over its bottom edge).
        let vertical = self.axis() != ScrollAxis::Down;
        if self.vbar.get() == vertical {
            self.vbar.set(!vertical);
            self.ui.show_vscroll(!vertical);
        }
        let total = self.page.as_ref().filter(|p| p.axis() == ScrollAxis::Down).map_or(page_h, |p| ((p.height() * self.dpr).ceil() as u32).max(page_h));
        self.ui.set_scroll(total, page_h, (self.scroll * self.dpr).round() as u32);
    }

    fn scroll_to(&mut self, y: f32) {
        let max = self.page.as_ref().map_or(0.0, |p| p.max_scroll(self.vp()));
        let y = self.snap(if y.is_finite() { y.fclamp(0.0, max) } else { 0.0 });
        if y == self.scroll {
            return;
        }
        self.scroll = y;
        // Layouts caused by images wait for the scrolling to stop (step 2-0).
        self.relayout.scrolled(Instant::now());
        self.update_scrollbar();
        // Images that came near start loading (L1 task 15).
        self.request_images();
        self.ui.invalidate();
    }

    fn scroll_by(&mut self, dy: f32) {
        self.scroll_to(self.scroll + dy);
    }

    /// The wheel: the innermost scroll container under the pointer that can still move that way, then the ones around it, then the
    /// page. Shift turns it sideways.
    fn on_wheel(&mut self, notches: f32, lines: u32, ctrl: bool, shift: bool) {
        if ctrl {
            return;
        }
        let d = -if lines == u32::MAX { notches * self.axis_extent() * PAGE_FRACTION } else { notches * lines as f32 * LINE_PX };
        let (dx, dy) = if shift { (d, 0.0) } else { (0.0, d) };
        let mut rest = (dx, dy);
        if let Some(page) = &self.page
            && let Some(list) = &page.list
            && !list.scopes.is_empty()
        {
            let pt = page.to_page(self.mouse.0 as f32 / self.dpr, self.mouse.1 as f32 / self.dpr, self.scroll, self.cross);
            rest = list.wheel(pt, dx, dy);
            if rest != (dx, dy) {
                self.relayout.scrolled(Instant::now());
                self.ui.invalidate();
            }
        }
        if shift && self.axis() == ScrollAxis::Down {
            self.cross_to(self.cross + rest.0);
        } else if !shift {
            self.scroll_by(rest.1);
        }
    }

    fn cross_to(&mut self, x: f32) {
        let max = self.page.as_ref().map_or(0.0, |p| p.max_cross(self.vp()));
        let x = self.snap(if x.is_finite() { x.fclamp(0.0, max) } else { 0.0 });
        if x != self.cross {
            self.cross = x;
            self.ui.invalidate();
        }
    }

    /// The left button went down: on a scroll bar (the page's along the bottom edge, or a container's) it takes hold of the thumb or
    /// pages towards the pointer.
    fn on_mouse_down(&mut self, x: i32, y: i32) {
        // A press whose release never came (the capture went to another window) must not eat this click.
        self.swallow = false;
        let (cx, cy, vp) = (x as f32 / self.dpr, y as f32 / self.dpr, self.vp());
        let Some(page) = &self.page else { return };
        let Some(list) = &page.list else { return };
        let axis = list.axis;
        let p = if axis == ScrollAxis::Down { self.cross } else { self.scroll };
        if cy >= vp.h - LANE
            && let Some((_, ts, tl, max)) = list.page_bar(p)
        {
            self.swallow = true;
            if cx >= ts && cx < ts + tl {
                self.drag = Some(Drag::Page { grab: cx - ts });
            } else {
                // Towards the pointer: on the screen to the left or right; a page that starts at its right edge moves the other way.
                let step = (if cx < ts { -0.875 } else { 0.875 }) * vp.w * if axis == ScrollAxis::Left { -1.0 } else { 1.0 };
                let target = (p + step).fclamp(0.0, max);
                if axis == ScrollAxis::Down {
                    self.cross_to(target);
                } else {
                    self.scroll_to(target);
                }
            }
            return;
        }
        let pt = page.to_page(cx, cy, self.scroll, self.cross);
        match list.bar_press(pt) {
            Some(Press::Thumb { scope, axis, grab }) => {
                self.swallow = true;
                self.drag = Some(Drag::Scope { scope, axis, grab });
            }
            Some(Press::Track) => {
                self.swallow = true;
                self.ui.invalidate();
            }
            None => {}
        }
    }

    /// The pointer moved with a scroll bar held.
    fn on_drag(&mut self, d: Drag, x: i32, y: i32) {
        let (cx, cy) = (x as f32 / self.dpr, y as f32 / self.dpr);
        let Some(page) = &self.page else { return };
        let Some(list) = &page.list else { return };
        match d {
            Drag::Scope { scope, axis, grab } => {
                list.bar_drag(scope, axis, grab, page.to_page(cx, cy, self.scroll, self.cross));
                self.relayout.scrolled(Instant::now());
                self.ui.invalidate();
            }
            Drag::Page { grab } => {
                let kind = list.axis;
                let p = if kind == ScrollAxis::Down { self.cross } else { self.scroll };
                if let Some((lane, _, tl, max)) = list.page_bar(p) {
                    let frac = ((cx - grab) / (lane - tl).max(1.0)).fclamp(0.0, 1.0);
                    let v = max * if kind == ScrollAxis::Left { 1.0 - frac } else { frac };
                    if kind == ScrollAxis::Down {
                        self.cross_to(v);
                    } else {
                        self.scroll_to(v);
                    }
                }
            }
        }
    }

    fn on_scroll_code(&mut self, code: ScrollCode) {
        let page = self.axis_extent() * PAGE_FRACTION;
        match code {
            ScrollCode::LineUp => self.scroll_by(-ARROW_PX),
            ScrollCode::LineDown => self.scroll_by(ARROW_PX),
            ScrollCode::PageUp => self.scroll_by(-page),
            ScrollCode::PageDown => self.scroll_by(page),
            ScrollCode::Top => self.scroll_to(0.0),
            ScrollCode::Bottom => self.scroll_to(f32::MAX),
            ScrollCode::Thumb(pos) => self.scroll_to(pos as f32 / self.dpr),
        }
    }

    fn link_under(&self, x: i32, y: i32) -> Option<(Url, bool)> {
        self.page.as_ref()?.link_at(x as f32 / self.dpr, y as f32 / self.dpr, self.scroll, self.cross)
    }

    fn on_mouse_move(&mut self, x: i32, y: i32) {
        self.mouse = (x, y);
        if let Some(d) = self.drag {
            self.on_drag(d, x, y);
            return;
        }
        let link = self.link_under(x, y).map(|(u, _)| u);
        self.set_hover(link);
    }

    fn set_hover(&mut self, link: Option<Url>) {
        if link == self.hover {
            return;
        }
        self.ui.set_hand_cursor(link.is_some());
        self.hover = link;
    }

    fn on_click(&mut self, x: i32, y: i32, button: MouseButton, ctrl: bool, shift: bool) {
        if std::mem::take(&mut self.swallow) {
            return;
        }
        let Some((url, blank)) = self.link_under(x, y) else { return };
        match button {
            MouseButton::Middle => self.open_in_new_window(&url),
            MouseButton::Left if ctrl || shift || blank => self.open_in_new_window(&url),
            MouseButton::Left => {
                self.hover = None;
                self.navigate(url, NavKind::New, 0.0, Origin::Page);
            }
        }
    }

    // ---- keys and buttons ---------------------------------------------------------------------------------

    fn on_command(&mut self, c: Command) {
        match c {
            Command::Back => self.go_history(true),
            Command::Forward => self.go_history(false),
            Command::ReloadOrStop => {
                if self.is_loading() {
                    self.stop();
                } else {
                    self.reload();
                }
            }
        }
    }

    fn on_key(&mut self, vk: u32, ctrl: bool, shift: bool, alt: bool, in_edit: bool) -> bool {
        match (vk, ctrl, alt) {
            (VK_L, true, false) | (VK_F6, false, false) => self.ui.focus_address(),
            (VK_RETURN, false, false) if in_edit => {
                let text = self.ui.address();
                self.ui.focus_page();
                self.open_typed(&text);
            }
            (VK_ESCAPE, false, false) => {
                self.stop();
                if in_edit {
                    if let Some(p) = &self.page {
                        self.ui.set_address(&p.display);
                    }
                    self.ui.focus_page();
                }
            }
            (VK_F5, false, false) | (VK_R, true, false) => self.reload(),
            (VK_N, true, false) => self.spawn_window(None),
            (VK_LEFT, false, true) => self.go_history(true),
            (VK_RIGHT, false, true) => self.go_history(false),
            _ if in_edit || ctrl || alt => return false,
            (VK_SPACE, ..) => self.on_scroll_code(if shift { ScrollCode::PageUp } else { ScrollCode::PageDown }),
            (VK_PRIOR, ..) => self.on_scroll_code(ScrollCode::PageUp),
            (VK_NEXT, ..) => self.on_scroll_code(ScrollCode::PageDown),
            (VK_HOME, ..) => self.on_scroll_code(ScrollCode::Top),
            (VK_END, ..) => self.on_scroll_code(ScrollCode::Bottom),
            (VK_UP, ..) => self.on_scroll_code(ScrollCode::LineUp),
            (VK_DOWN, ..) => self.on_scroll_code(ScrollCode::LineDown),
            // On a vertical page the lines run down and the page moves sideways: the arrow towards the end of the text goes on.
            (VK_LEFT, ..) | (VK_RIGHT, ..) if self.axis() != ScrollAxis::Down => {
                let forward = (vk == VK_LEFT) == (self.axis() == ScrollAxis::Left);
                self.on_scroll_code(if forward { ScrollCode::LineDown } else { ScrollCode::LineUp });
            }
            _ => return false,
        }
        true
    }

    // ---- status bar and buttons ---------------------------------------------------------------------------

    fn is_loading(&self) -> bool {
        self.nav.is_some() || self.page.as_ref().is_some_and(|p| p.phase == Phase::Sheets || p.res.images_pending() > 0)
    }

    fn progress(&self) -> Option<String> {
        if self.nav.is_some() {
            return Some(self.s.connecting.to_string());
        }
        let page = self.page.as_ref()?;
        if page.phase == Phase::Sheets {
            return Some(fill(self.s.loading_sheets, &page.res.sheets_pending().to_string()));
        }
        let n = page.res.images_pending();
        (n > 0).then(|| fill(self.s.loading_images, &n.to_string()))
    }

    /// Bring the status bar and the reload/stop button in line with the state.
    fn refresh_chrome(&mut self) {
        let status = self.hover.as_ref().map(Url::href).or_else(|| self.note.clone()).or_else(|| self.progress()).unwrap_or_default();
        if status != self.last_status {
            self.ui.set_status(&status);
            self.last_status = status;
        }
        let loading = self.is_loading();
        if loading != self.last_loading {
            self.ui.set_loading(loading);
            self.last_loading = loading;
        }
    }

    // ---- benchmark ----------------------------------------------------------------------------------------

    /// Is the page settled (everything loaded and laid out)? Then start scrolling.
    fn bench_check(&mut self) {
        if self.bench.as_ref().is_none_or(|b| b.stage != Stage::Loading) {
            return;
        }
        if self.page.as_ref().is_some_and(|p| p.is_error) {
            self.bench_fail("the page did not load");
            return;
        }
        let settled = self.nav.is_none()
            && self.page.as_ref().is_some_and(|p| p.phase == Phase::Shown && p.is_complete() && p.res.images_pending() == 0 && p.res.sheets_pending() == 0)
            && !self.relayout.pending()
            && self.loader.pending() == 0;
        if settled && let Some(b) = self.bench.as_mut() {
            b.stage = Stage::Scrolling;
            b.next_y = 0.0;
            b.burst_left = BENCH_BURST;
            self.ui.post_user(USER_BENCH_STEP);
        }
    }

    /// Is something still on its way to the page (a layout waiting, images loading)?
    fn bench_busy(&self) -> bool {
        self.relayout.pending() || self.loader.pending() > 0 || self.page.as_ref().is_some_and(|p| p.res.images_pending() > 0 || p.store.in_flight() > 0)
    }

    /// One notch of the scroll, as a hand on a wheel makes it: the next position, the images that came near are
    /// requested (and decoded), the page is painted and put on the window. Notches come in bursts with a pause between
    /// them, so images arrive and layouts are due while the scroll goes on.
    fn bench_step(&mut self) {
        let Some(next_y) = self.bench.as_ref().filter(|b| b.stage == Stage::Scrolling).map(|b| b.next_y) else { return };
        if self.platform.is_none() {
            return;
        }
        let vp = self.vp();
        // Positions are whole device pixels: compare against the snapped end, or a bottom that rounds down is never
        // reached (the walk would go on for ever).
        let max = self.snap(self.page.as_ref().map_or(0.0, |p| p.max_scroll(vp)));
        let waiting = self.bench.as_ref().is_some_and(|b| b.bottom_at.is_some());
        if waiting && self.scroll >= max {
            // At the bottom: the layout and the images that are still on their way settle (bounded), then the run ends.
            let waited = self.bench.as_ref().and_then(|b| b.bottom_at).map_or(0, |t| t.elapsed().as_millis());
            if self.bench_busy() && waited < BENCH_SETTLE_MS {
                self.ui.set_timer(TIMER_BENCH_STEP, 100);
            } else {
                self.bench_finish();
            }
            return;
        }
        // A layout while waiting made the page longer: the walk goes on.
        if let Some(b) = self.bench.as_mut() {
            b.bottom_at = None;
        }
        let y = self.snap(next_y.min(max));
        self.scroll = y;
        // What the window does when the user scrolls (`scroll_to`): the layouts wait, images that came near start.
        self.relayout.scrolled(Instant::now());
        self.update_scrollbar();
        self.request_images();
        let t_decode = Instant::now();
        let mut decoded = 0;
        if let Some(p) = self.page.as_mut() {
            p.prepare(vp, y, &mut self.loader, &mut self.pool);
            decoded = p.store.bytes();
        }
        if let Some(b) = self.bench.as_mut() {
            b.peak_decoded = b.peak_decoded.max(decoded);
        }
        let decode = ms(t_decode.elapsed());
        let t = Instant::now();
        self.draw_bands(y);
        let t_flush = Instant::now();
        self.ui.flush();
        self.present_ms += ms(t_flush.elapsed());
        let frame_ms = ms(t.elapsed());
        let mut delay = BENCH_STEP_MS;
        if let Some(b) = self.bench.as_mut() {
            b.frames.push(frame_ms);
            b.frame_y.push(y);
            b.present_ms.push(self.present_ms);
            b.decode_ms += decode;
            b.next_y = y + BENCH_SCROLL_STEP;
            b.burst_left = b.burst_left.saturating_sub(1);
            if b.burst_left == 0 {
                b.burst_left = BENCH_BURST;
                delay = BENCH_PAUSE_MS;
            }
        }
        let too_long = self.bench.as_ref().is_some_and(|b| b.frames.len() > MAX_BENCH_FRAMES || b.started.elapsed() > Duration::from_millis(u64::from(BENCH_TIMEOUT_MS)));
        if y >= max {
            if let Some(b) = self.bench.as_mut() {
                b.bottom_at = Some(Instant::now());
            }
            self.ui.set_timer(TIMER_BENCH_STEP, 100);
        } else if too_long {
            self.bench_fail("timed out while scrolling");
        } else {
            self.ui.set_timer(TIMER_BENCH_STEP, delay);
        }
    }

    /// Remember the process's private bytes under `name` (bench runs only).
    fn mem_mark(&mut self, name: &'static str) {
        if let Some(b) = self.bench.as_mut()
            && let Some((private, _)) = qinglan_win::window::process_memory()
        {
            b.marks.push((name, private));
        }
    }

    /// What each structure holds, by dropping them one at a time and seeing how much the live heap shrinks (the
    /// page is gone afterwards, so this is the last thing a bench run does).
    #[cfg(feature = "mem-stats")]
    fn memory_breakdown(&mut self) -> String {
        use qinglan_win::memstats::{Breakdown, dom_parts, layout_parts};
        if !qinglan_win::heap_counting() {
            return String::new();
        }
        let mut bd = Breakdown::default();
        bd.note("heap-live", qinglan_win::heap_live());
        bd.note("heap-peak", qinglan_win::heap_peak());
        bd.note("allocations", qinglan_win::heap_allocs() as usize);
        bd.note("live-blocks", qinglan_win::heap_blocks());
        bd.note("peak-decoded-images", self.bench.as_ref().map_or(0, |b| b.peak_decoded));
        bd.note("response-cache-accounted", self.loader.cache().bytes());
        let Some(mut page) = self.page.take() else { return bd.lines() };
        let (nodes, text, attrs) = dom_parts(&page.doc);
        bd.note("dom-node-count", page.doc.nodes.len());
        bd.note("dom-detail-node-array", nodes);
        bd.note("dom-detail-text", text);
        bd.note("dom-detail-attributes", attrs);
        if let Some(l) = &page.layout {
            for (name, count, bytes) in layout_parts(l) {
                bd.note(&format!("layout-detail-{name}"), bytes);
                bd.note(&format!("layout-count-{name}"), count);
            }
        }
        bd.freed("hit-test-index", || page.index = None);
        bd.freed("display-list", || page.list = None);
        bd.freed("box-and-fragment-tree", || page.layout = None);
        bd.freed("computed-styles", || page.styled = None);
        bd.freed("decoded-images", || page.store = qinglan_engine::paint::images::ImageStore::new());
        bd.freed("response-cache", || self.loader.cache_mut().clear());
        if let Some(w) = &self.platform {
            let stats = w.cache_stats();
            bd.note("shaping-cache-accounted", stats.shape_bytes);
            bd.note("glyph-cache-accounted", stats.glyph_bytes);
            bd.freed("shaping-cache", || w.clear_shape_cache());
            bd.freed("glyph-cache", || w.clear_glyph_cache());
            bd.freed("word-tables", || w.clear_word_caches());
        }
        bd.freed("dom-and-sheets", || drop(page));
        bd.freed("frame-buffer", || self.frame = Frame::new(0, 0));
        bd.lines()
    }

    fn bench_finish(&mut self) {
        self.ui.kill_timer(TIMER_BENCH);
        if self.bench.as_ref().is_none_or(|b| b.stage == Stage::Done) {
            return;
        }
        let memory = qinglan_win::window::process_memory();
        #[cfg(feature = "mem-stats")]
        let (heap_stats, pool_stats, breakdown) = (qinglan_win::heap_summary(), qinglan_win::pool_report(), self.memory_breakdown());
        #[cfg(feature = "mem-stats")]
        let torn_down = qinglan_win::window::process_memory();
        let Some(b) = self.bench.as_mut() else { return };
        b.stage = Stage::Done;
        #[cfg(feature = "mem-stats")]
        {
            b.breakdown = breakdown;
            let (chunks, on_free_lists, by_class) = pool_stats;
            b.breakdown.push_str(&format!("mem-pool-chunks: {chunks}
mem-pool-free-lists: {on_free_lists}
mem-pool-free-by-class:{by_class}
"));
            if let Some((allocated, committed)) = heap_stats {
                b.breakdown.push_str(&format!("mem-win-heap-allocated: {allocated}
mem-win-heap-committed: {committed}
"));
            }
            if let Some((private, _)) = torn_down {
                b.breakdown.push_str(&format!("mem-private-after-teardown: {private}
"));
            }
        }
        let code = i32::from(b.error.is_some());
        let report = Report { bench: b, memory }.format();
        print!("{report}");
        let _ = std::io::stdout().flush();
        self.ui.close(code);
    }

    fn bench_fail(&mut self, why: &str) {
        if let Some(b) = self.bench.as_mut() {
            if b.stage == Stage::Done {
                return;
            }
            b.error = Some(why.to_string());
        }
        let detail = format!(
            " (stage {:?}, {} relayouts, relayout pending {}, images pending {}, loader pending {}, nav {})",
            self.bench.as_ref().map(|b| b.stage),
            self.bench.as_ref().map_or(0, |b| b.relayouts),
            self.relayout.pending(),
            self.page.as_ref().map_or(0, |p| p.res.images_pending()),
            self.loader.pending(),
            self.nav.is_some()
        );
        if let Some(b) = self.bench.as_mut() {
            b.error = b.error.take().map(|e| e + &detail);
        }
        self.bench_finish();
    }
}

impl App for Browser {
    fn event(&mut self, _ui: &Ui, ev: Event) -> bool {
        let start = Instant::now();
        self.event_start = start;
        let scrolling = self.bench.as_ref().is_some_and(|b| b.stage == Stage::Scrolling);
        self.present_ms = 0.0;
        let handled = match catch(|| self.handle(ev)) {
            Ok(h) => h,
            Err(text) => {
                self.internal_error(text);
                false
            }
        };
        if let Err(text) = catch(|| {
            self.refresh_chrome();
            self.bench_check();
        }) {
            eprintln!("qinglan: {text}");
        }
        if let Some(b) = self.bench.as_mut() {
            let end = Instant::now();
            b.event_done(start, end);
            if scrolling {
                let took = ms(end.saturating_duration_since(start));
                b.scroll_event_max_ms = b.scroll_event_max_ms.max(took);
                b.scroll_ours_max_ms = b.scroll_ours_max_ms.max(took - self.present_ms);
            }
        }
        handled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_panic_is_caught_and_its_message_kept() {
        assert_eq!(catch(|| 5), Ok(5));
        let r: Result<(), String> = catch(|| panic!("layout went wrong"));
        assert_eq!(r, Err("layout went wrong".to_string()));
        let r: Result<(), String> = catch(|| panic!("{}", String::from("formatted")));
        assert_eq!(r, Err("formatted".to_string()));
    }

    #[test]
    fn the_blank_address_is_about_blank() {
        assert_eq!(blank_url().href(), "about:blank");
    }

    fn url(s: &str) -> Url {
        Url::parse(s, None).unwrap_or_else(|_| blank_url())
    }

    #[test]
    fn only_the_user_can_open_a_file_address() {
        let page = url("https://evil.example/");
        let local = url("file:///C:/Users/x/page.html");
        // Typed, from the command line, history, reload: the user's own request.
        assert_eq!(plan_navigation(&local, Origin::User, Some(&page)), Ok(policy::Plan::File));
        assert_eq!(plan_navigation(&local, Origin::User, None), Ok(policy::Plan::File));
        // A link, Ctrl/Shift/middle click, target=_blank (all carry Origin::Page), a new window for a link.
        for target in ["file:///C:/Users/x/page.html", "file:////host/share/a.html", "file://server/share/a", "FILE:///C:/a.html"] {
            assert_eq!(plan_navigation(&url(target), Origin::Page, Some(&page)), Err(Refusal::LocalFile), "{target}");
        }
        // Also when the page is itself a local file, and when there is no page at all.
        assert_eq!(plan_navigation(&local, Origin::Page, Some(&local)), Err(Refusal::LocalFile));
        assert_eq!(plan_navigation(&local, Origin::Page, None), Err(Refusal::LocalFile));
        // Even typed, a network share or a device path is not opened.
        assert_eq!(plan_navigation(&url("file:////host/share/a.html"), Origin::User, None), Err(Refusal::NotLocalFile));
        // Web addresses are fine whoever asks; mailto: and javascript: are still refusals for both.
        assert_eq!(plan_navigation(&url("https://ok.example/"), Origin::Page, Some(&page)), Ok(policy::Plan::Network));
        assert_eq!(plan_navigation(&url("mailto:a@b.c"), Origin::Page, Some(&page)), Err(Refusal::Mailto));
    }
}
