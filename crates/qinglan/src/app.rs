//! The browser: what the window's events do. One [`Browser`] lives on the UI thread; the network runs on the loader's
//! threads and reaches it through the wake-up message, the UI thread never waits for it.

use std::any::Any;
use std::io::Write as _;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use qinglan_engine::encoding::Encoding;
use qinglan_engine::html::treebuilder::parse_document;
use qinglan_engine::net::blocking::{self, OpenResult, Opened};
use qinglan_engine::net::content::ContentKind;
use qinglan_engine::net::loader::{LoadEvent, LoadRequest, Loader, Ticket};
use qinglan_engine::net::policy::{self, Refusal};
use qinglan_engine::net::{NetError, ResourceKind};
use qinglan_engine::paint::raster::Frame;
use qinglan_engine::url::{Url, UrlPath};
use qinglan_win::WinPlatform;
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
const USER_BENCH_STEP: u32 = 1;
/// How long the page waits for the style sheets of `<head>` (L1 task 17).
const CSS_WAIT_MS: u32 = 5000;
/// Images change the layout at most this often.
const RELAYOUT_GAP: Duration = Duration::from_millis(100);
/// A wheel "line" in CSS px (Chrome scrolls 100 px for the default 3 lines).
const LINE_PX: f32 = 100.0 / 3.0;
const ARROW_PX: f32 = 40.0;
const PAGE_FRACTION: f32 = 0.875;
const BENCH_SCROLL_STEP: f32 = 100.0;
const BENCH_TIMEOUT_MS: u32 = 120_000;
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
    (BAND_BYTES / (w.max(1) * 4)).clamp(64, h.max(64))
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
    platform: Option<Rc<WinPlatform>>,
    history: History,
    nav: Option<Nav>,
    page: Option<Page>,
    /// The page area in device pixels, and device pixels per CSS px.
    dev: (u32, u32),
    dpr: f32,
    /// Scroll position in CSS px (a whole number of device pixels).
    scroll: f32,
    frame: Frame,
    relayout: Throttle,
    /// The viewport changed: styles (media queries) and layout are made again.
    restyle: bool,
    /// Layouts started by arriving images since the page was opened or the window last changed.
    image_relayouts: u32,
    hover: Option<Url>,
    /// A message that stays in the status bar until the next navigation ("stopped").
    note: Option<String>,
    last_status: String,
    last_loading: bool,
    bench: Option<Bench>,
    event_start: Instant,
    prewarm: Option<JoinHandle<usize>>,
}

impl Browser {
    pub fn new(ui: Ui, opts: Options, loader: Loader, lang: Lang) -> Browser {
        let bench = opts.bench.clone().map(Bench::new);
        let dpr = bench.as_ref().map_or(ui.dpi() as f32 / 96.0, |b| b.opts.dpr);
        Browser {
            ui,
            s: lang.strings(),
            opts,
            loader,
            platform: None,
            history: History::new(),
            nav: None,
            page: None,
            dev: (0, 0),
            dpr,
            scroll: 0.0,
            frame: Frame::new(0, 0),
            relayout: Throttle::new(RELAYOUT_GAP),
            restyle: false,
            image_relayouts: 0,
            hover: None,
            note: None,
            last_status: String::new(),
            last_loading: false,
            bench,
            event_start: Instant::now(),
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
            Event::Click { x, y, button, ctrl, shift } => self.on_click(x, y, button, ctrl, shift),
            Event::Wheel { notches, lines, ctrl } => self.on_wheel(notches, lines, ctrl),
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
            TIMER_BENCH => self.bench_fail("timed out"),
            _ => {}
        }
    }

    fn on_wake(&mut self) {
        for ev in self.loader.poll() {
            self.on_load_event(ev);
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
            None => {}
        }
    }

    // ---- navigation ---------------------------------------------------------------------------------------

    /// Open what was typed in the address bar.
    fn open_typed(&mut self, text: &str) {
        let Some(addr) = address::classify(text) else { return };
        match Url::parse(&addr, None) {
            Ok(url) => self.navigate(url, NavKind::New, 0.0),
            Err(_) => self.show_failure(Failure::BadAddress, addr, NavKind::New),
        }
    }

    fn is_same_document(&self, url: &Url, kind: NavKind) -> bool {
        let Some(page) = &self.page else { return false };
        kind != NavKind::Reload && !page.is_error && page.layout.is_some() && (kind == NavKind::History || url.fragment.is_some()) && page.url.serialize(true) == url.serialize(true)
    }

    fn navigate(&mut self, url: Url, kind: NavKind, scroll: f32) {
        if self.is_same_document(&url, kind) {
            self.same_document(url, kind, scroll);
            return;
        }
        // Refusals (mailto:, javascript:, ...) leave the page and its loading alone.
        if let Err(refusal) = policy::plan(&url, ResourceKind::Document, None) {
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
                let addr = url.href();
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
                let y = url.fragment.as_deref().and_then(|f| self.page.as_ref().and_then(|p| p.fragment_y(f)));
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
        // page is built (L1 task 17).
        self.page = None;
        self.hover = None;
        self.ui.set_hand_cursor(false);
        self.relayout.reset();
        self.restyle = false;
        self.image_relayouts = 0;
        self.ui.kill_timer(TIMER_RELAYOUT);
        self.scroll = 0.0;
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
            Ok(url) => self.navigate(url, NavKind::Reload, scroll),
            Err(_) => self.show_failure(Failure::BadAddress, display, NavKind::Reload),
        }
    }

    fn go_history(&mut self, back: bool) {
        self.history.save_scroll(self.scroll);
        let Some(entry) = (if back { self.history.back() } else { self.history.forward() }) else { return };
        self.update_nav_buttons();
        match Url::parse(&entry.url, None) {
            Ok(url) => self.navigate(url, NavKind::History, entry.scroll),
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
        if let Err(refusal) = policy::plan(url, ResourceKind::Document, None) {
            self.on_refusal(refusal, url.clone(), NavKind::New);
            return;
        }
        self.spawn_window(Some(&url.href()));
    }

    /// A new window is a new process (L1 task 17).
    fn spawn_window(&self, address: Option<&str>) {
        let Ok(exe) = std::env::current_exe() else { return };
        let mut cmd = std::process::Command::new(exe);
        if let Some(a) = address {
            cmd.arg(a);
        }
        if let Some(r) = &self.opts.replay {
            cmd.arg("--replay").arg(r);
        }
        let _ = cmd.spawn();
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
            Some(f) => page.fragment_y(f).unwrap_or(0.0),
            None => self.scroll,
        };
        self.scroll = self.snap(target.clamp(0.0, max));
        self.mark_full_layout();
        self.mem_mark("full-layout");
        self.update_scrollbar();
        self.update_notice();
        self.request_images();
        self.ui.invalidate();
        if fragment.is_some() {
            self.paint();
        }
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
        let (w, h) = self.dev;
        let vp = self.vp();
        let rows = band_rows(w, h);
        let platform = self.platform.clone();
        let mut y0 = 0;
        while y0 < h {
            let bh = rows.min(h - y0);
            self.frame.set_size(w, bh);
            match (&platform, self.page.as_mut()) {
                (Some(p), Some(page)) if page.list.is_some() => page.raster(&**p, vp, scroll, y0, &mut self.frame),
                _ => self.frame.data.fill(255),
            }
            self.ui.blit_at(y0, w, bh, &self.frame.data);
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
            need_images = page.prepare(vp, scroll, &mut self.loader);
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
        let (vp, scroll, all) = (self.vp(), self.scroll, self.bench.is_some());
        let Some(page) = self.page.as_mut() else { return };
        if page.request_images(&mut self.loader, vp, scroll, all) {
            self.request_image_relayout();
        }
    }

    /// An image changed what the layout needs: lay out again, at most [`MAX_IMAGE_RELAYOUTS`] times per page.
    fn request_image_relayout(&mut self) {
        if self.image_relayouts >= MAX_IMAGE_RELAYOUTS {
            self.ui.invalidate();
            return;
        }
        self.image_relayouts += 1;
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
        let anchored = page.relayout(&*platform, vp, scroll);
        if let Some(b) = self.bench.as_mut() {
            b.relayouts += 1;
        }
        let max = page.max_scroll(vp);
        self.scroll = self.snap(anchored.clamp(0.0, max));
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

    fn update_scrollbar(&self) {
        let page_h = self.dev.1;
        let total = self.page.as_ref().map_or(page_h, |p| ((p.height() * self.dpr).ceil() as u32).max(page_h));
        self.ui.set_scroll(total, page_h, (self.scroll * self.dpr).round() as u32);
    }

    fn scroll_to(&mut self, y: f32) {
        let max = self.page.as_ref().map_or(0.0, |p| p.max_scroll(self.vp()));
        let y = self.snap(if y.is_finite() { y.clamp(0.0, max) } else { 0.0 });
        if y == self.scroll {
            return;
        }
        self.scroll = y;
        self.update_scrollbar();
        // Images that came near start loading (L1 task 15).
        self.request_images();
        self.ui.invalidate();
    }

    fn scroll_by(&mut self, dy: f32) {
        self.scroll_to(self.scroll + dy);
    }

    fn on_wheel(&mut self, notches: f32, lines: u32, ctrl: bool) {
        if ctrl {
            return;
        }
        let dy = if lines == u32::MAX { notches * self.vp().h * PAGE_FRACTION } else { notches * lines as f32 * LINE_PX };
        self.scroll_by(-dy);
    }

    fn on_scroll_code(&mut self, code: ScrollCode) {
        let page = self.vp().h * PAGE_FRACTION;
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
        self.page.as_ref()?.link_at(x as f32 / self.dpr, y as f32 / self.dpr, self.scroll)
    }

    fn on_mouse_move(&mut self, x: i32, y: i32) {
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
        let Some((url, blank)) = self.link_under(x, y) else { return };
        match button {
            MouseButton::Middle => self.open_in_new_window(&url),
            MouseButton::Left if ctrl || shift || blank => self.open_in_new_window(&url),
            MouseButton::Left => {
                self.hover = None;
                self.navigate(url, NavKind::New, 0.0);
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
            self.ui.post_user(USER_BENCH_STEP);
        }
    }

    /// One frame of the scroll: the next position, decode what came near, paint it, put it on the window.
    fn bench_step(&mut self) {
        let Some(next_y) = self.bench.as_ref().filter(|b| b.stage == Stage::Scrolling).map(|b| b.next_y) else { return };
        if self.platform.is_none() {
            return;
        }
        let vp = self.vp();
        // Positions are whole device pixels: compare against the snapped end, or a bottom that rounds down is never
        // reached (the walk would go on for ever).
        let max = self.snap(self.page.as_ref().map_or(0.0, |p| p.max_scroll(vp)));
        let y = self.snap(next_y.min(max));
        self.scroll = y;
        self.update_scrollbar();
        let t_decode = Instant::now();
        let mut decoded = 0;
        if let Some(p) = self.page.as_mut() {
            p.prepare(vp, y, &mut self.loader);
            decoded = p.store.bytes();
        }
        if let Some(b) = self.bench.as_mut() {
            b.peak_decoded = b.peak_decoded.max(decoded);
        }
        let decode = ms(t_decode.elapsed());
        let t = Instant::now();
        self.draw_bands(y);
        self.ui.flush();
        let frame_ms = ms(t.elapsed());
        if let Some(b) = self.bench.as_mut() {
            b.frames.push(frame_ms);
            b.decode_ms += decode;
            b.next_y = y + BENCH_SCROLL_STEP;
        }
        let too_long = self.bench.as_ref().is_some_and(|b| b.frames.len() > MAX_BENCH_FRAMES || b.started.elapsed() > Duration::from_millis(u64::from(BENCH_TIMEOUT_MS)));
        if y >= max {
            self.bench_finish();
        } else if too_long {
            // Posted steps starve the (low priority) timer message, so the deadline is checked here as well.
            self.bench_fail("timed out while scrolling");
        } else {
            self.ui.post_user(USER_BENCH_STEP);
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
        let heap_stats = qinglan_win::heap_summary();
        let breakdown = self.memory_breakdown();
        let torn_down = qinglan_win::window::process_memory();
        let Some(b) = self.bench.as_mut() else { return };
        b.stage = Stage::Done;
        b.breakdown = breakdown;
        if let Some((allocated, committed)) = heap_stats {
            b.breakdown.push_str(&format!("mem-win-heap-allocated: {allocated}
mem-win-heap-committed: {committed}
"));
        }
        if let Some((private, _)) = torn_down {
            b.breakdown.push_str(&format!("mem-private-after-teardown: {private}
"));
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
            b.event_done(start, Instant::now());
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
}
