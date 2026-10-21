//! The browser window (step 1c-2b): a frame with back / forward / reload buttons, an address bar (the system EDIT
//! control, so input methods work), a notice strip, the page area (a child window with a native scroll bar that the
//! application paints into) and a status bar. `unsafe` stays in `crate::sys::window`; this module turns window messages
//! into [`Event`]s for an [`App`] and gives the application a safe [`Ui`] to drive the window with.
//!
//! The application lives on the UI thread. While it handles an event, events that the window generates itself
//! (a resize caused by a resize it asked for, a timer, a wake-up) are queued and delivered right after it returns;
//! the ones that only make sense live (mouse, keys, paint) are dropped.

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicIsize, Ordering};

use crate::sys::window::{self as w, Hwnd, Raw, Target};

const ID_BACK: isize = 101;
const ID_FORWARD: isize = 102;
const ID_RELOAD: isize = 103;

/// Posted by the loader's wake callback (any thread).
const WM_WAKE: u32 = w::WM_APP + 1;
/// Posted by [`Ui::post_user`].
const WM_USER_EVENT: u32 = w::WM_APP + 2;
/// Posted once when the window is up.
const WM_READY: u32 = w::WM_APP + 3;

const VK_RETURN: u32 = 0x0D;
const VK_ESCAPE: u32 = 0x1B;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Back,
    Forward,
    /// The reload button; it is the stop button while a page loads.
    ReloadOrStop,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Middle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollCode {
    LineUp,
    LineDown,
    PageUp,
    PageDown,
    Top,
    Bottom,
    /// The thumb is at this position (device pixels).
    Thumb(i32),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// The window is on screen and the first messages have been handled.
    Ready,
    /// The page area is now `width` x `height` device pixels (without the scroll bar).
    Size { width: u32, height: u32 },
    Dpi { dpi: u32 },
    /// The page area needs painting: [`Ui::blit`] a frame.
    Paint,
    MouseMove { x: i32, y: i32 },
    MouseLeave,
    /// The left button went down at this place (the mouse is captured until it goes up).
    MouseDown { x: i32, y: i32 },
    MouseUp,
    Click { x: i32, y: i32, button: MouseButton, ctrl: bool, shift: bool },
    /// `notches` wheel clicks (positive = up, may be fractional for fine wheels); `lines` is the system's lines per
    /// notch, `u32::MAX` for one page.
    Wheel { notches: f32, lines: u32, ctrl: bool, shift: bool },
    /// A key went down in the page area or the address bar. Return true when handled.
    Key { vk: u32, ctrl: bool, shift: bool, alt: bool, in_edit: bool },
    Scroll(ScrollCode),
    Command(Command),
    Timer(usize),
    /// The loader has something (see [`Waker`]).
    Wake,
    /// [`Ui::post_user`].
    User(u32),
    /// The user closed the window; return true to keep it open.
    Close,
}

/// The application behind the window.
pub trait App {
    /// Handle `ev`. The return value matters for [`Event::Key`] (handled) and [`Event::Close`] (veto).
    fn event(&mut self, ui: &Ui, ev: Event) -> bool;
}

/// Lets any thread wake the UI thread: the loader's wake callback calls [`Waker::wake`], the window then delivers
/// [`Event::Wake`]. Wake-ups that arrive before the UI thread handled the last one are merged.
pub struct Waker {
    hwnd: AtomicIsize,
    pending: AtomicBool,
}

impl Waker {
    pub fn new() -> Arc<Waker> {
        Arc::new(Waker { hwnd: AtomicIsize::new(0), pending: AtomicBool::new(false) })
    }

    pub fn wake(&self) {
        let h = self.hwnd.load(Ordering::Acquire);
        if h != 0 && !self.pending.swap(true, Ordering::AcqRel) && !w::post(h, WM_WAKE, 0, 0) {
            self.pending.store(false, Ordering::Release);
        }
    }
}

#[derive(Clone, Copy, Default)]
struct State {
    frame: Hwnd,
    page: Hwnd,
    edit: Hwnd,
    back: Hwnd,
    forward: Hwnd,
    reload: Hwnd,
    status: Hwnd,
    secure: Hwnd,
    notice: Hwnd,
    dpi: u32,
    notice_on: bool,
    ui_font: isize,
    symbol_font: isize,
    notice_brush: isize,
    hand: bool,
    tracking: bool,
    swallow_char: bool,
}

thread_local! {
    static STATE: Cell<State> = const { Cell::new(State {
        frame: 0, page: 0, edit: 0, back: 0, forward: 0, reload: 0, status: 0, secure: 0, notice: 0, dpi: 96,
        notice_on: false, ui_font: 0, symbol_font: 0, notice_brush: 0, hand: false, tracking: false, swallow_char: false,
    }) };
    static APP: RefCell<Option<Box<dyn App>>> = const { RefCell::new(None) };
    static QUEUE: RefCell<VecDeque<Event>> = const { RefCell::new(VecDeque::new()) };
    static WAKER: RefCell<Option<Arc<Waker>>> = const { RefCell::new(None) };
}

static EXIT_CODE: AtomicI32 = AtomicI32::new(0);

fn state() -> State {
    STATE.with(Cell::get)
}

fn update_state(f: impl FnOnce(&mut State)) {
    STATE.with(|s| {
        let mut v = s.get();
        f(&mut v);
        s.set(v);
    });
}

/// Events that are worth keeping when the application is busy.
fn deferrable(ev: &Event) -> bool {
    matches!(ev, Event::Size { .. } | Event::Dpi { .. } | Event::Timer(_) | Event::Wake | Event::User(_) | Event::Ready | Event::Command(_) | Event::Scroll(_) | Event::Close)
}

/// Hand an event to the application, then everything that was queued while it ran. A panic in the application is
/// caught (a panic must not cross the window procedure) and reported on stderr.
fn deliver(ev: Event) -> bool {
    let taken = APP.with(|a| a.borrow_mut().take());
    let Some(mut app) = taken else {
        if deferrable(&ev) {
            QUEUE.with(|q| {
                let mut q = q.borrow_mut();
                // Only the latest size matters.
                if matches!(ev, Event::Size { .. }) {
                    q.retain(|e| !matches!(e, Event::Size { .. }));
                }
                q.push_back(ev);
            });
        }
        return false;
    };
    let ui = Ui(());
    let mut result = run_event(app.as_mut(), &ui, ev);
    while let Some(next) = QUEUE.with(|q| q.borrow_mut().pop_front()) {
        let r = run_event(app.as_mut(), &ui, next);
        if matches!(next, Event::Close) {
            result = r;
        }
    }
    APP.with(|a| *a.borrow_mut() = Some(app));
    result
}

fn run_event(app: &mut dyn App, ui: &Ui, ev: Event) -> bool {
    match catch_unwind(AssertUnwindSafe(|| app.event(ui, ev))) {
        Ok(r) => r,
        Err(_) => {
            eprintln!("qinglan: the application panicked while handling {ev:?}");
            false
        }
    }
}

fn lo(v: usize) -> u32 {
    (v & 0xFFFF) as u32
}

fn hi(v: usize) -> u32 {
    ((v >> 16) & 0xFFFF) as u32
}

fn point(lparam: isize) -> (i32, i32) {
    (i32::from(lparam as u16 as i16), i32::from((lparam >> 16) as u16 as i16))
}

fn modifiers() -> (bool, bool, bool) {
    (w::key_down(0x11), w::key_down(0x10), w::key_down(0x12))
}

/// Window messages in, events out (called from the window procedures in `sys::window`). `None` means "not
/// handled, use the default".
pub(crate) fn dispatch(raw: Raw) -> Option<isize> {
    match catch_unwind(AssertUnwindSafe(|| dispatch_inner(raw))) {
        Ok(r) => r,
        Err(_) => {
            eprintln!("qinglan: panic while handling window message {:#x}", raw.msg);
            None
        }
    }
}

fn dispatch_inner(raw: Raw) -> Option<isize> {
    match raw.target {
        Target::Frame => dispatch_frame(raw),
        Target::Page => dispatch_page(raw),
        Target::Edit => dispatch_edit(raw),
    }
}

fn dispatch_frame(raw: Raw) -> Option<isize> {
    let s = state();
    match raw.msg {
        w::WM_SIZE => {
            if s.page != 0 {
                arrange();
            }
            Some(0)
        }
        w::WM_COMMAND => {
            if hi(raw.wparam) == 0 && raw.lparam != 0 {
                let cmd = match lo(raw.wparam) as isize {
                    ID_BACK => Some(Command::Back),
                    ID_FORWARD => Some(Command::Forward),
                    ID_RELOAD => Some(Command::ReloadOrStop),
                    _ => None,
                };
                if let Some(c) = cmd {
                    w::set_focus(s.page);
                    deliver(Event::Command(c));
                }
            }
            Some(0)
        }
        w::WM_TIMER => {
            deliver(Event::Timer(raw.wparam));
            Some(0)
        }
        WM_WAKE => {
            WAKER.with(|k| {
                if let Some(k) = k.borrow().as_ref() {
                    k.pending.store(false, Ordering::Release);
                }
            });
            deliver(Event::Wake);
            Some(0)
        }
        WM_USER_EVENT => {
            deliver(Event::User(raw.wparam as u32));
            Some(0)
        }
        WM_READY => {
            deliver(Event::Ready);
            Some(0)
        }
        w::WM_CLOSE => {
            if !deliver(Event::Close) {
                w::destroy(s.frame);
            }
            Some(0)
        }
        w::WM_DESTROY => {
            w::quit(EXIT_CODE.load(Ordering::Relaxed));
            Some(0)
        }
        w::WM_DPICHANGED => {
            let dpi = lo(raw.wparam).max(1);
            update_state(|st| st.dpi = dpi);
            w::set_window_rect(s.frame, raw.rect);
            apply_fonts();
            arrange();
            deliver(Event::Dpi { dpi });
            Some(0)
        }
        w::WM_SETFOCUS => {
            w::set_focus(s.page);
            Some(0)
        }
        w::WM_CTLCOLORSTATIC if raw.lparam == s.notice && s.notice != 0 => {
            // A pale yellow strip with dark text.
            w::set_static_colors(raw.wparam as isize, 0x0020_2020, 0x00C8_F0FF);
            Some(s.notice_brush)
        }
        _ => None,
    }
}

fn dispatch_page(raw: Raw) -> Option<isize> {
    let s = state();
    match raw.msg {
        w::WM_PAINT => {
            deliver(Event::Paint);
            Some(0)
        }
        w::WM_ERASEBKGND => Some(1),
        w::WM_SIZE => {
            let r = w::client_rect(raw.hwnd);
            deliver(Event::Size { width: r.width().max(0) as u32, height: r.height().max(0) as u32 });
            Some(0)
        }
        w::WM_MOUSEMOVE => {
            if !s.tracking {
                update_state(|st| st.tracking = true);
                w::track_mouse_leave(raw.hwnd);
            }
            let (x, y) = point(raw.lparam);
            // The button is up (the capture was lost, say to another window): a drag is over.
            if raw.wparam & 0x1 == 0 {
                deliver(Event::MouseUp);
            }
            deliver(Event::MouseMove { x, y });
            Some(0)
        }
        w::WM_MOUSELEAVE => {
            update_state(|st| st.tracking = false);
            deliver(Event::MouseLeave);
            Some(0)
        }
        w::WM_LBUTTONDOWN => {
            w::set_focus(raw.hwnd);
            w::set_capture(raw.hwnd);
            let (x, y) = point(raw.lparam);
            deliver(Event::MouseDown { x, y });
            Some(0)
        }
        w::WM_LBUTTONUP | w::WM_MBUTTONUP => {
            if raw.msg == w::WM_LBUTTONUP {
                w::release_capture();
                deliver(Event::MouseUp);
            }
            let (x, y) = point(raw.lparam);
            let button = if raw.msg == w::WM_LBUTTONUP { MouseButton::Left } else { MouseButton::Middle };
            deliver(Event::Click { x, y, button, ctrl: raw.wparam & 0x8 != 0, shift: raw.wparam & 0x4 != 0 });
            Some(0)
        }
        w::WM_MOUSEWHEEL => {
            let delta = i32::from(hi(raw.wparam) as u16 as i16);
            deliver(Event::Wheel { notches: delta as f32 / 120.0, lines: w::wheel_scroll_lines(), ctrl: raw.wparam & 0x8 != 0, shift: raw.wparam & 0x4 != 0 });
            Some(0)
        }
        w::WM_VSCROLL => {
            let code = match lo(raw.wparam) {
                0 => Some(ScrollCode::LineUp),
                1 => Some(ScrollCode::LineDown),
                2 => Some(ScrollCode::PageUp),
                3 => Some(ScrollCode::PageDown),
                4 | 5 => Some(ScrollCode::Thumb(w::scroll_track_pos(raw.hwnd))),
                6 => Some(ScrollCode::Top),
                7 => Some(ScrollCode::Bottom),
                _ => None,
            };
            if let Some(c) = code {
                deliver(Event::Scroll(c));
            }
            Some(0)
        }
        w::WM_KEYDOWN | w::WM_SYSKEYDOWN => {
            let (ctrl, shift, alt) = modifiers();
            let handled = deliver(Event::Key { vk: raw.wparam as u32, ctrl, shift, alt: alt || raw.msg == w::WM_SYSKEYDOWN, in_edit: false });
            handled.then_some(0)
        }
        w::WM_SETCURSOR => {
            if (raw.lparam & 0xFFFF) == w::HTCLIENT {
                w::set_cursor(s.hand);
                Some(1)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn dispatch_edit(raw: Raw) -> Option<isize> {
    let s = state();
    match raw.msg {
        w::WM_KEYDOWN | w::WM_SYSKEYDOWN => {
            let (ctrl, shift, alt) = modifiers();
            let vk = raw.wparam as u32;
            if ctrl && !alt && vk == u32::from(b'A') {
                w::send_value(raw.hwnd, w::EM_SETSEL, 0, -1);
                return Some(0);
            }
            let handled = deliver(Event::Key { vk, ctrl, shift, alt: alt || raw.msg == w::WM_SYSKEYDOWN, in_edit: true });
            if handled {
                // The character that goes with a handled Ctrl+key would otherwise reach the edit control.
                update_state(|st| st.swallow_char = ctrl);
                return Some(0);
            }
            update_state(|st| st.swallow_char = false);
            None
        }
        w::WM_CHAR => {
            // Enter and Escape are shortcuts, not text (the control would beep).
            if s.swallow_char || raw.wparam as u32 == VK_RETURN || raw.wparam as u32 == VK_ESCAPE {
                update_state(|st| st.swallow_char = false);
                return Some(0);
            }
            None
        }
        _ => None,
    }
}

/// Place the children for the frame's current size, DPI and notice state.
fn arrange() {
    let s = state();
    let scale = s.dpi as f32 / 96.0;
    let px = |v: f32| (v * scale).round() as i32;
    let c = w::client_rect(s.frame);
    let (cw, ch) = (c.width().max(0), c.height().max(0));
    let (pad, btn) = (px(4.0), px(30.0));
    let toolbar = btn + 2 * pad;
    w::move_window(s.back, pad, pad, btn, btn);
    w::move_window(s.forward, 2 * pad + btn, pad, btn, btn);
    w::move_window(s.reload, 3 * pad + 2 * btn, pad, btn, btn);
    let edit_x = 4 * pad + 3 * btn;
    let edit_h = px(26.0);
    w::move_window(s.edit, edit_x, pad + (btn - edit_h) / 2, (cw - edit_x - pad).max(0), edit_h);
    let notice_h = if s.notice_on { px(24.0) } else { 0 };
    w::move_window(s.notice, 0, toolbar, cw, notice_h);
    let status_h = px(22.0);
    let secure_w = px(110.0).min(cw);
    w::move_window(s.status, px(6.0), ch - status_h, (cw - secure_w - px(6.0)).max(0), status_h);
    w::move_window(s.secure, cw - secure_w, ch - status_h, secure_w - px(6.0), status_h);
    let top = toolbar + notice_h;
    w::move_window(s.page, 0, top, cw, (ch - top - status_h).max(0));
}

fn apply_fonts() {
    let s = state();
    let (ui, sym) = w::make_fonts(s.dpi);
    for h in [s.edit, s.status, s.secure, s.notice] {
        w::send_value(h, w::WM_SETFONT, ui as usize, 1);
    }
    for h in [s.back, s.forward, s.reload] {
        w::send_value(h, w::WM_SETFONT, sym as usize, 1);
    }
    w::delete_object(s.ui_font);
    w::delete_object(s.symbol_font);
    update_state(|st| {
        st.ui_font = ui;
        st.symbol_font = sym;
    });
}

/// The window, as the application drives it. A handle with no data: it reads the window state when used.
#[derive(Clone, Copy)]
pub struct Ui(());

impl Ui {
    pub fn set_title(&self, title: &str) {
        w::set_text(state().frame, title);
    }

    pub fn set_address(&self, text: &str) {
        w::set_text(state().edit, text);
    }

    pub fn address(&self) -> String {
        w::get_text(state().edit)
    }

    /// Focus the address bar with everything selected.
    pub fn focus_address(&self) {
        let s = state();
        w::set_focus(s.edit);
        w::send_value(s.edit, w::EM_SETSEL, 0, -1);
    }

    pub fn focus_page(&self) {
        w::set_focus(state().page);
    }

    pub fn set_status(&self, text: &str) {
        w::set_text(state().status, text);
    }

    /// The right-hand part of the status bar ("not secure").
    pub fn set_secure(&self, text: &str) {
        w::set_text(state().secure, text);
    }

    /// Show a strip under the toolbar with `text`, or hide it.
    pub fn show_notice(&self, text: Option<&str>) {
        let s = state();
        let on = text.is_some();
        if let Some(t) = text {
            w::set_text(s.notice, t);
        }
        if on != s.notice_on {
            update_state(|st| st.notice_on = on);
            w::show(s.notice, if on { w::SW_SHOW } else { w::SW_HIDE });
            arrange();
        }
    }

    pub fn set_nav_enabled(&self, back: bool, forward: bool) {
        let s = state();
        w::enable(s.back, back);
        w::enable(s.forward, forward);
    }

    /// The reload button turns into a stop button while loading.
    pub fn set_loading(&self, loading: bool) {
        w::set_text(state().reload, if loading { "\u{2715}" } else { "\u{21BB}" });
    }

    pub fn set_hand_cursor(&self, hand: bool) {
        if state().hand != hand {
            update_state(|st| st.hand = hand);
            w::set_cursor(hand);
        }
    }

    /// Request a repaint of the page area (it arrives as [`Event::Paint`]).
    pub fn invalidate(&self) {
        w::invalidate(state().page);
    }

    /// Copy a top-down BGRA picture of `width` x `height` pixels to the page area.
    pub fn blit(&self, width: u32, height: u32, bgra: &[u8]) -> bool {
        w::blit(state().page, 0, width, height, bgra)
    }

    /// Like [`Ui::blit`], with the picture's top row at row `y` of the page area (a band of the page).
    pub fn blit_at(&self, y: u32, width: u32, height: u32, bgra: &[u8]) -> bool {
        w::blit(state().page, y, width, height, bgra)
    }

    /// Wait until the drawing done so far has reached the window (for timing).
    pub fn flush(&self) {
        w::gdi_flush();
    }

    /// The page area in device pixels.
    pub fn page_size(&self) -> (u32, u32) {
        let r = w::client_rect(state().page);
        (r.width().max(0) as u32, r.height().max(0) as u32)
    }

    /// Resize the window so that the page area is `width` x `height` device pixels.
    pub fn set_page_size(&self, width: u32, height: u32) {
        let s = state();
        let page = w::client_rect(s.page);
        let frame = w::window_rect(s.frame);
        w::resize_window(s.frame, frame.width() + width as i32 - page.width(), frame.height() + height as i32 - page.height());
    }

    pub fn dpi(&self) -> u32 {
        state().dpi
    }

    /// The scroll bar: the page is `total` device pixels high, `page` are visible, the top is at `pos`.
    pub fn set_scroll(&self, total: u32, page: u32, pos: u32) {
        w::set_scroll(state().page, i32::try_from(total.saturating_sub(1)).unwrap_or(i32::MAX), page, i32::try_from(pos).unwrap_or(i32::MAX));
    }

    /// Show or hide the vertical scroll bar (a page that scrolls along its lines has none).
    pub fn show_vscroll(&self, show: bool) {
        w::show_vscroll(state().page, show);
    }

    pub fn set_timer(&self, id: usize, ms: u32) {
        w::set_timer(state().frame, id, ms);
    }

    pub fn kill_timer(&self, id: usize) {
        w::kill_timer(state().frame, id);
    }

    /// Queue [`Event::User`] for later (after the messages already waiting).
    pub fn post_user(&self, n: u32) {
        w::post(state().frame, WM_USER_EVENT, n as usize, 0);
    }

    /// OK / Cancel box over the window.
    pub fn confirm(&self, caption: &str, text: &str) -> bool {
        w::confirm(state().frame, caption, text)
    }

    pub fn address_has_focus(&self) -> bool {
        w::focus() == state().edit
    }

    /// Hand `target` to the program Windows has for it.
    pub fn open_external(&self, target: &str) -> bool {
        w::shell_open(target)
    }

    /// Close the window (the program ends with `exit_code`).
    pub fn close(&self, exit_code: i32) {
        EXIT_CODE.store(exit_code, Ordering::Relaxed);
        w::post(state().frame, w::WM_CLOSE, 0, 0);
    }
}

/// Create the window and run the message loop until it is closed. `make` builds the application once the window
/// exists. Returns the exit code given to [`Ui::close`].
pub fn run(waker: &Arc<Waker>, make: impl FnOnce(&Ui) -> Box<dyn App>) -> i32 {
    if !w::register_classes() {
        eprintln!("qinglan: cannot register the window classes");
        return 1;
    }
    let frame = w::create_frame("\u{8f7b}\u{89c8}", 1100, 780);
    if frame == 0 {
        eprintln!("qinglan: cannot create the window");
        return 1;
    }
    let btn_style = w::WS_VISIBLE | w::BS_PUSHBUTTON;
    let label = |style: u32| w::WS_VISIBLE | style | w::SS_NOPREFIX | w::SS_CENTERIMAGE | w::SS_ENDELLIPSIS;
    let s = State {
        frame,
        page: w::create_page(frame),
        edit: w::create_control("EDIT", "", w::WS_VISIBLE | w::WS_TABSTOP | w::ES_AUTOHSCROLL, w::WS_EX_CLIENTEDGE, frame, 0),
        back: w::create_control("BUTTON", "\u{2190}", btn_style, 0, frame, ID_BACK),
        forward: w::create_control("BUTTON", "\u{2192}", btn_style, 0, frame, ID_FORWARD),
        reload: w::create_control("BUTTON", "\u{21BB}", btn_style, 0, frame, ID_RELOAD),
        status: w::create_control("STATIC", "", label(0), 0, frame, 0),
        secure: w::create_control("STATIC", "", label(w::SS_RIGHT), 0, frame, 0),
        notice: w::create_control("STATIC", "", label(0) & !w::WS_VISIBLE, 0, frame, 0),
        dpi: w::dpi_for_window(frame),
        notice_brush: w::create_brush(0x00C8_F0FF),
        ..State::default()
    };
    STATE.with(|c| c.set(s));
    w::subclass_edit(s.edit);
    apply_fonts();
    waker.hwnd.store(frame, Ordering::Release);
    WAKER.with(|k| *k.borrow_mut() = Some(waker.clone()));
    let scale = s.dpi as f32 / 96.0;
    w::resize_window(frame, (1100.0 * scale) as i32, (780.0 * scale) as i32);
    arrange();
    let ui = Ui(());
    let app = make(&ui);
    APP.with(|a| *a.borrow_mut() = Some(app));
    w::show(frame, w::SW_SHOW);
    w::update(frame);
    w::set_focus(s.edit);
    let (width, height) = ui.page_size();
    deliver(Event::Size { width, height });
    w::post(frame, WM_READY, 0, 0);
    w::message_loop()
}

/// Make the process per-monitor DPI aware (version 2). Call before the window exists.
pub fn set_dpi_aware() -> bool {
    w::set_per_monitor_dpi_v2()
}

/// Make `println!` work in a GUI process started from a console.
pub fn attach_console() {
    w::attach_parent_console();
}

/// Milliseconds since this process was created.
pub fn since_process_creation_ms() -> Option<f64> {
    w::since_process_creation_ms()
}

/// `(private bytes now, peak pagefile usage)` of this process.
pub fn process_memory() -> Option<(u64, u64)> {
    w::process_memory()
}

/// Which UI language to use: true when the first display language is Chinese.
pub fn ui_is_chinese() -> bool {
    crate::ui_languages().first().is_some_and(|l| l.to_ascii_lowercase().starts_with("zh"))
}
