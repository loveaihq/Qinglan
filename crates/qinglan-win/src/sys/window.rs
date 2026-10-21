//! Win32 windows, GDI and process information (step 1c-2b), declared by hand from the Windows SDK headers
//! (`winuser.h`, `wingdi.h`, `processthreadsapi.h`, `psapi.h`, `shellapi.h`, `consoleapi.h`).
//!
//! Window and GDI handles are plain integers here (`isize`): an invalid or stale handle makes the call fail, it
//! does not make it unsound, so the wrappers below are safe functions. The pointers the system hands to a window
//! procedure (`WM_DPICHANGED`'s `RECT`, `WM_GETMINMAXINFO`'s `MINMAXINFO`) are read and written only inside the
//! two `extern "system"` procedures. The safe code in `crate::window` sees decoded values only.

use core::ffi::c_void;

use super::BOOL;

pub type Hwnd = isize;

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    pub fn width(&self) -> i32 {
        self.right - self.left
    }
    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Point {
    x: i32,
    y: i32,
}

#[repr(C)]
struct Msg {
    hwnd: Hwnd,
    message: u32,
    wparam: usize,
    lparam: isize,
    time: u32,
    pt: Point,
    private: u32,
}

type WndProc = unsafe extern "system" fn(Hwnd, u32, usize, isize) -> isize;

#[repr(C)]
struct WndClassExW {
    size: u32,
    style: u32,
    proc: Option<WndProc>,
    cls_extra: i32,
    wnd_extra: i32,
    instance: isize,
    icon: isize,
    cursor: isize,
    background: isize,
    menu_name: *const u16,
    class_name: *const u16,
    icon_sm: isize,
}

#[repr(C)]
struct PaintStruct {
    hdc: isize,
    erase: BOOL,
    paint: Rect,
    restore: BOOL,
    inc_update: BOOL,
    reserved: [u8; 32],
}

#[repr(C)]
struct BitmapInfoHeader {
    size: u32,
    width: i32,
    height: i32,
    planes: u16,
    bit_count: u16,
    compression: u32,
    size_image: u32,
    x_pels: i32,
    y_pels: i32,
    clr_used: u32,
    clr_important: u32,
}

#[repr(C)]
struct BitmapInfo {
    header: BitmapInfoHeader,
    colors: [u32; 1],
}

#[repr(C)]
struct ScrollInfo {
    size: u32,
    mask: u32,
    min: i32,
    max: i32,
    page: u32,
    pos: i32,
    track_pos: i32,
}

#[repr(C)]
struct TrackMouseEventData {
    size: u32,
    flags: u32,
    hwnd: Hwnd,
    hover_time: u32,
}

#[repr(C)]
struct MinMaxInfo {
    reserved: Point,
    max_size: Point,
    max_position: Point,
    min_track: Point,
    max_track: Point,
}

/// `LOGFONTW` (92 bytes).
#[repr(C)]
#[derive(Clone, Copy)]
struct LogFont {
    height: i32,
    width: i32,
    escapement: i32,
    orientation: i32,
    weight: i32,
    italic: u8,
    underline: u8,
    strike_out: u8,
    char_set: u8,
    out_precision: u8,
    clip_precision: u8,
    quality: u8,
    pitch_and_family: u8,
    face_name: [u16; 32],
}

/// `NONCLIENTMETRICSW` (504 bytes on Windows Vista and later).
#[repr(C)]
struct NonClientMetrics {
    size: u32,
    border_width: i32,
    scroll_width: i32,
    scroll_height: i32,
    caption_width: i32,
    caption_height: i32,
    caption_font: LogFont,
    sm_caption_width: i32,
    sm_caption_height: i32,
    sm_caption_font: LogFont,
    menu_width: i32,
    menu_height: i32,
    menu_font: LogFont,
    status_font: LogFont,
    message_font: LogFont,
    padded_border_width: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct FileTime {
    low: u32,
    high: u32,
}

impl FileTime {
    fn ticks(self) -> u64 {
        (u64::from(self.high) << 32) | u64::from(self.low)
    }
}

/// `PROCESS_MEMORY_COUNTERS_EX`.
#[repr(C)]
#[derive(Default)]
struct ProcessMemoryCountersEx {
    cb: u32,
    page_fault_count: u32,
    peak_working_set_size: usize,
    working_set_size: usize,
    quota_peak_paged_pool_usage: usize,
    quota_paged_pool_usage: usize,
    quota_peak_non_paged_pool_usage: usize,
    quota_non_paged_pool_usage: usize,
    pagefile_usage: usize,
    peak_pagefile_usage: usize,
    private_usage: usize,
}

#[link(name = "user32")]
unsafe extern "system" {
    fn RegisterClassExW(class: *const WndClassExW) -> u16;
    fn CreateWindowExW(
        ex_style: u32,
        class: *const u16,
        name: *const u16,
        style: u32,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        parent: Hwnd,
        menu: isize,
        instance: isize,
        param: *mut c_void,
    ) -> Hwnd;
    fn DefWindowProcW(hwnd: Hwnd, msg: u32, wparam: usize, lparam: isize) -> isize;
    fn CallWindowProcW(prev: isize, hwnd: Hwnd, msg: u32, wparam: usize, lparam: isize) -> isize;
    fn SetWindowLongPtrW(hwnd: Hwnd, index: i32, value: isize) -> isize;
    fn DestroyWindow(hwnd: Hwnd) -> BOOL;
    fn ShowWindow(hwnd: Hwnd, cmd: i32) -> BOOL;
    fn UpdateWindow(hwnd: Hwnd) -> BOOL;
    fn GetMessageW(msg: *mut Msg, hwnd: Hwnd, min: u32, max: u32) -> BOOL;
    fn TranslateMessage(msg: *const Msg) -> BOOL;
    fn DispatchMessageW(msg: *const Msg) -> isize;
    fn PostMessageW(hwnd: Hwnd, msg: u32, wparam: usize, lparam: isize) -> BOOL;
    fn PostQuitMessage(code: i32);
    fn SendMessageW(hwnd: Hwnd, msg: u32, wparam: usize, lparam: isize) -> isize;
    fn SetWindowTextW(hwnd: Hwnd, text: *const u16) -> BOOL;
    fn GetWindowTextLengthW(hwnd: Hwnd) -> i32;
    fn GetWindowTextW(hwnd: Hwnd, buf: *mut u16, max: i32) -> i32;
    fn MoveWindow(hwnd: Hwnd, x: i32, y: i32, w: i32, h: i32, repaint: BOOL) -> BOOL;
    fn SetWindowPos(hwnd: Hwnd, after: isize, x: i32, y: i32, cx: i32, cy: i32, flags: u32) -> BOOL;
    fn GetClientRect(hwnd: Hwnd, rect: *mut Rect) -> BOOL;
    fn GetWindowRect(hwnd: Hwnd, rect: *mut Rect) -> BOOL;
    fn SetFocus(hwnd: Hwnd) -> Hwnd;
    fn GetFocus() -> Hwnd;
    fn EnableWindow(hwnd: Hwnd, enable: BOOL) -> BOOL;
    fn InvalidateRect(hwnd: Hwnd, rect: *const Rect, erase: BOOL) -> BOOL;
    fn SetCursor(cursor: isize) -> isize;
    fn LoadCursorW(instance: isize, name: *const u16) -> isize;
    fn SetTimer(hwnd: Hwnd, id: usize, ms: u32, proc: isize) -> usize;
    fn KillTimer(hwnd: Hwnd, id: usize) -> BOOL;
    fn GetKeyState(vk: i32) -> i16;
    fn GetDpiForWindow(hwnd: Hwnd) -> u32;
    fn SetProcessDpiAwarenessContext(context: isize) -> BOOL;
    fn SystemParametersInfoW(action: u32, param: u32, pv: *mut c_void, win_ini: u32) -> BOOL;
    fn SystemParametersInfoForDpi(action: u32, param: u32, pv: *mut c_void, win_ini: u32, dpi: u32) -> BOOL;
    fn SetScrollInfo(hwnd: Hwnd, bar: i32, info: *const ScrollInfo, redraw: BOOL) -> i32;
    fn GetScrollInfo(hwnd: Hwnd, bar: i32, info: *mut ScrollInfo) -> BOOL;
    fn MessageBoxW(hwnd: Hwnd, text: *const u16, caption: *const u16, flags: u32) -> i32;
    fn GetDC(hwnd: Hwnd) -> isize;
    fn ReleaseDC(hwnd: Hwnd, hdc: isize) -> i32;
    fn BeginPaint(hwnd: Hwnd, ps: *mut PaintStruct) -> isize;
    fn EndPaint(hwnd: Hwnd, ps: *const PaintStruct) -> BOOL;
    fn TrackMouseEvent(data: *mut TrackMouseEventData) -> BOOL;
    fn SetCapture(hwnd: Hwnd) -> Hwnd;
    fn ReleaseCapture() -> BOOL;
    fn ShowScrollBar(hwnd: Hwnd, bar: i32, show: BOOL) -> BOOL;
}

#[link(name = "gdi32")]
unsafe extern "system" {
    fn SetDIBitsToDevice(
        hdc: isize,
        x: i32,
        y: i32,
        w: u32,
        h: u32,
        src_x: i32,
        src_y: i32,
        start_scan: u32,
        lines: u32,
        bits: *const c_void,
        info: *const BitmapInfo,
        usage: u32,
    ) -> i32;
    fn CreateFontIndirectW(font: *const LogFont) -> isize;
    fn DeleteObject(object: isize) -> BOOL;
    fn CreateSolidBrush(color: u32) -> isize;
    fn SetBkColor(hdc: isize, color: u32) -> u32;
    fn SetTextColor(hdc: isize, color: u32) -> u32;
    fn GdiFlush() -> BOOL;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetModuleHandleW(name: *const u16) -> isize;
    fn GetCurrentProcess() -> isize;
    fn GetProcessTimes(process: isize, creation: *mut FileTime, exit: *mut FileTime, kernel: *mut FileTime, user: *mut FileTime) -> BOOL;
    fn GetSystemTimeAsFileTime(out: *mut FileTime);
    fn K32GetProcessMemoryInfo(process: isize, counters: *mut ProcessMemoryCountersEx, cb: u32) -> BOOL;
    fn AttachConsole(pid: u32) -> BOOL;
    fn GetStdHandle(which: u32) -> isize;
    fn SetStdHandle(which: u32, handle: isize) -> BOOL;
}

#[link(name = "shell32")]
unsafe extern "system" {
    fn ShellExecuteW(hwnd: Hwnd, verb: *const u16, file: *const u16, params: *const u16, dir: *const u16, show: i32) -> isize;
}

// Window styles.
pub const WS_OVERLAPPEDWINDOW: u32 = 0x00CF_0000;
pub const WS_CHILD: u32 = 0x4000_0000;
pub const WS_VISIBLE: u32 = 0x1000_0000;
pub const WS_CLIPCHILDREN: u32 = 0x0200_0000;
pub const WS_CLIPSIBLINGS: u32 = 0x0400_0000;
pub const WS_VSCROLL: u32 = 0x0020_0000;
pub const WS_TABSTOP: u32 = 0x0001_0000;
pub const WS_EX_CLIENTEDGE: u32 = 0x0000_0200;
pub const BS_PUSHBUTTON: u32 = 0;
pub const ES_AUTOHSCROLL: u32 = 0x80;
pub const SS_NOPREFIX: u32 = 0x80;
pub const SS_CENTERIMAGE: u32 = 0x200;
pub const SS_ENDELLIPSIS: u32 = 0x4000;
pub const SS_RIGHT: u32 = 2;

// Messages.
pub const WM_DESTROY: u32 = 0x0002;
pub const WM_SIZE: u32 = 0x0005;
pub const WM_SETFOCUS: u32 = 0x0007;
pub const WM_PAINT: u32 = 0x000F;
pub const WM_CLOSE: u32 = 0x0010;
pub const WM_ERASEBKGND: u32 = 0x0014;
pub const WM_SETCURSOR: u32 = 0x0020;
pub const WM_GETMINMAXINFO: u32 = 0x0024;
pub const WM_SETFONT: u32 = 0x0030;
pub const WM_KEYDOWN: u32 = 0x0100;
pub const WM_CHAR: u32 = 0x0102;
pub const WM_SYSKEYDOWN: u32 = 0x0104;
pub const WM_COMMAND: u32 = 0x0111;
pub const WM_TIMER: u32 = 0x0113;
pub const WM_VSCROLL: u32 = 0x0115;
pub const WM_CTLCOLORSTATIC: u32 = 0x0138;
pub const WM_MOUSEMOVE: u32 = 0x0200;
pub const WM_LBUTTONUP: u32 = 0x0202;
pub const WM_LBUTTONDOWN: u32 = 0x0201;
pub const WM_MBUTTONUP: u32 = 0x0208;
pub const WM_MOUSEWHEEL: u32 = 0x020A;
pub const WM_MOUSELEAVE: u32 = 0x02A3;
pub const WM_DPICHANGED: u32 = 0x02E0;
pub const WM_APP: u32 = 0x8000;
pub const EM_SETSEL: u32 = 0x00B1;

pub const SW_HIDE: i32 = 0;
pub const SW_SHOW: i32 = 5;
pub const SB_VERT: i32 = 1;
pub const HTCLIENT: isize = 1;

/// Which of the window procedures a message came through.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Target {
    Frame,
    Page,
    Edit,
}

/// A window message as the safe layer sees it. `rect` is the suggested window rectangle of `WM_DPICHANGED`.
#[derive(Clone, Copy, Debug)]
pub struct Raw {
    pub target: Target,
    pub hwnd: Hwnd,
    pub msg: u32,
    pub wparam: usize,
    pub lparam: isize,
    pub rect: Rect,
}

pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(core::iter::once(0)).collect()
}

static EDIT_PREV_PROC: core::sync::atomic::AtomicIsize = core::sync::atomic::AtomicIsize::new(0);

unsafe extern "system" fn frame_proc(hwnd: Hwnd, msg: u32, wparam: usize, lparam: isize) -> isize {
    if msg == WM_GETMINMAXINFO {
        // SAFETY: for this message `lparam` points to a MINMAXINFO that the system owns for the duration of the call.
        unsafe {
            if let Some(info) = (lparam as *mut MinMaxInfo).as_mut() {
                // Let the window grow past the screen (the benchmark opens windows of the size it was asked for).
                info.max_track = Point { x: 16384, y: 16384 };
            }
        }
        return 0;
    }
    let mut rect = Rect::default();
    if msg == WM_DPICHANGED {
        // SAFETY: for this message `lparam` points to the suggested window RECT, valid during the call.
        rect = unsafe { (lparam as *const Rect).as_ref().copied().unwrap_or_default() };
    }
    match crate::window::dispatch(Raw { target: Target::Frame, hwnd, msg, wparam, lparam, rect }) {
        Some(r) => r,
        // SAFETY: forwards the message of this procedure's own window unchanged.
        None => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

unsafe extern "system" fn page_proc(hwnd: Hwnd, msg: u32, wparam: usize, lparam: isize) -> isize {
    if msg == WM_PAINT {
        let mut ps = PaintStruct { hdc: 0, erase: 0, paint: Rect::default(), restore: 0, inc_update: 0, reserved: [0; 32] };
        // SAFETY: `ps` is a valid out value; BeginPaint/EndPaint are paired around the dispatch.
        unsafe {
            BeginPaint(hwnd, &mut ps);
        }
        crate::window::dispatch(Raw { target: Target::Page, hwnd, msg, wparam, lparam, rect: Rect::default() });
        // SAFETY: pairs with the BeginPaint above on the same window and the same PAINTSTRUCT.
        unsafe {
            EndPaint(hwnd, &ps);
        }
        return 0;
    }
    match crate::window::dispatch(Raw { target: Target::Page, hwnd, msg, wparam, lparam, rect: Rect::default() }) {
        Some(r) => r,
        // SAFETY: forwards the message of this procedure's own window unchanged.
        None => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

unsafe extern "system" fn edit_proc(hwnd: Hwnd, msg: u32, wparam: usize, lparam: isize) -> isize {
    if let Some(r) = crate::window::dispatch(Raw { target: Target::Edit, hwnd, msg, wparam, lparam, rect: Rect::default() }) {
        return r;
    }
    let prev = EDIT_PREV_PROC.load(core::sync::atomic::Ordering::Relaxed);
    // SAFETY: `prev` is the procedure SetWindowLongPtrW returned when the edit control was subclassed (the control's
    // own), or 0 if that failed, in which case the default procedure is the right fallback.
    unsafe {
        if prev == 0 { DefWindowProcW(hwnd, msg, wparam, lparam) } else { CallWindowProcW(prev, hwnd, msg, wparam, lparam) }
    }
}

fn instance() -> isize {
    // SAFETY: a null name asks for the module of the running executable.
    unsafe { GetModuleHandleW(core::ptr::null()) }
}

const COLOR_BTNFACE: isize = 15;
const IDC_ARROW: usize = 32512;
const IDC_HAND: usize = 32649;
const CS_DBLCLKS: u32 = 0x8;

/// Register the two window classes (frame and page). Returns false when the system refuses.
pub fn register_classes() -> bool {
    let frame = wide("QinglanFrame");
    let page = wide("QinglanPage");
    // SAFETY: LoadCursorW with a predefined cursor id (a small integer disguised as a pointer, as the API defines).
    let arrow = unsafe { LoadCursorW(0, IDC_ARROW as *const u16) };
    let mut ok = true;
    for (name, proc, bg, style) in [(&frame, frame_proc as WndProc, COLOR_BTNFACE + 1, 0), (&page, page_proc as WndProc, 0, CS_DBLCLKS)] {
        let class = WndClassExW {
            size: core::mem::size_of::<WndClassExW>() as u32,
            style,
            proc: Some(proc),
            cls_extra: 0,
            wnd_extra: 0,
            instance: instance(),
            icon: 0,
            cursor: arrow,
            background: bg,
            menu_name: core::ptr::null(),
            class_name: name.as_ptr(),
            icon_sm: 0,
        };
        // SAFETY: `class` is fully initialised and `name` outlives the call; the procedures are `extern "system"`.
        ok &= unsafe { RegisterClassExW(&class) } != 0;
    }
    ok
}

fn create(ex_style: u32, class: &str, text: &str, style: u32, (x, y, w, h): (i32, i32, i32, i32), parent: Hwnd, id: isize) -> Hwnd {
    let (class, text) = (wide(class), wide(text));
    // SAFETY: both strings are NUL-terminated and outlive the call; no creation parameter is passed.
    unsafe { CreateWindowExW(ex_style, class.as_ptr(), text.as_ptr(), style, x, y, w, h, parent, id, instance(), core::ptr::null_mut()) }
}

const CW_USEDEFAULT: i32 = i32::MIN;

/// The top-level window (not shown yet).
pub fn create_frame(title: &str, w: i32, h: i32) -> Hwnd {
    create(0, "QinglanFrame", title, WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN, (CW_USEDEFAULT, CW_USEDEFAULT, w, h), 0, 0)
}

/// The page window: a child of `parent` with a vertical scroll bar.
pub fn create_page(parent: Hwnd) -> Hwnd {
    create(0, "QinglanPage", "", WS_CHILD | WS_VISIBLE | WS_VSCROLL | WS_CLIPSIBLINGS, (0, 0, 100, 100), parent, 0)
}

/// A control of a system class (`BUTTON`, `EDIT`, `STATIC`) as a child of `parent` with command id `id`.
pub fn create_control(class: &str, text: &str, style: u32, ex_style: u32, parent: Hwnd, id: isize) -> Hwnd {
    create(ex_style, class, text, WS_CHILD | style, (0, 0, 10, 10), parent, id)
}

/// Route an edit control's messages through [`crate::window::dispatch`] first.
pub fn subclass_edit(edit: Hwnd) {
    // SAFETY: replaces the window procedure of a window this thread created with `edit_proc`, which forwards what it
    // does not handle to the previous one (stored for it just below); GWLP_WNDPROC is -4.
    let prev = unsafe { SetWindowLongPtrW(edit, -4, edit_proc as WndProc as usize as isize) };
    EDIT_PREV_PROC.store(prev, core::sync::atomic::Ordering::Relaxed);
}

/// Run the message loop until `WM_QUIT`; returns its exit code.
pub fn message_loop() -> i32 {
    let mut msg = Msg { hwnd: 0, message: 0, wparam: 0, lparam: 0, time: 0, pt: Point::default(), private: 0 };
    loop {
        // SAFETY: `msg` is a valid out value. A return of 0 is WM_QUIT, -1 an error: both end the loop.
        let r = unsafe { GetMessageW(&mut msg, 0, 0, 0) };
        if r <= 0 {
            return msg.wparam as i32;
        }
        // SAFETY: `msg` was filled in by GetMessageW.
        unsafe {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

pub fn quit(code: i32) {
    // SAFETY: plain value argument.
    unsafe { PostQuitMessage(code) }
}

pub fn destroy(hwnd: Hwnd) {
    // SAFETY: handle values only; failure is harmless.
    unsafe {
        DestroyWindow(hwnd);
    }
}

pub fn show(hwnd: Hwnd, cmd: i32) {
    // SAFETY: handle value and a show command.
    unsafe {
        ShowWindow(hwnd, cmd);
    }
}

pub fn update(hwnd: Hwnd) {
    // SAFETY: handle value only.
    unsafe {
        UpdateWindow(hwnd);
    }
}

pub fn post(hwnd: Hwnd, msg: u32, wparam: usize, lparam: isize) -> bool {
    // SAFETY: PostMessage copies the values; they are not pointers the receiver would dereference.
    unsafe { PostMessageW(hwnd, msg, wparam, lparam) != 0 }
}

/// `SendMessage` for messages whose parameters are plain values (`WM_SETFONT`, `EM_SETSEL`).
pub fn send_value(hwnd: Hwnd, msg: u32, wparam: usize, lparam: isize) -> isize {
    // SAFETY: only used with messages whose parameters are integers or handles, never pointers.
    unsafe { SendMessageW(hwnd, msg, wparam, lparam) }
}

pub fn set_text(hwnd: Hwnd, text: &str) {
    let w = wide(text);
    // SAFETY: `w` is NUL-terminated and outlives the call.
    unsafe {
        SetWindowTextW(hwnd, w.as_ptr());
    }
}

pub fn get_text(hwnd: Hwnd) -> String {
    // SAFETY: handle value only.
    let n = unsafe { GetWindowTextLengthW(hwnd) };
    if n <= 0 || n > 1 << 20 {
        return String::new();
    }
    let mut buf = vec![0u16; n as usize + 1];
    // SAFETY: `buf` holds n+1 units and the call is told so.
    let got = unsafe { GetWindowTextW(hwnd, buf.as_mut_ptr(), n + 1) };
    String::from_utf16_lossy(buf.get(..got.max(0) as usize).unwrap_or(&[]))
}

pub fn move_window(hwnd: Hwnd, x: i32, y: i32, w: i32, h: i32) {
    // SAFETY: handle value and plain integers.
    unsafe {
        MoveWindow(hwnd, x, y, w.max(0), h.max(0), 1);
    }
}

pub fn set_window_rect(hwnd: Hwnd, r: Rect) {
    const SWP_NOZORDER: u32 = 0x4;
    const SWP_NOACTIVATE: u32 = 0x10;
    // SAFETY: handle value and plain integers; 0 is "no insert-after window" and SWP_NOZORDER ignores it.
    unsafe {
        SetWindowPos(hwnd, 0, r.left, r.top, r.width(), r.height(), SWP_NOZORDER | SWP_NOACTIVATE);
    }
}

pub fn resize_window(hwnd: Hwnd, w: i32, h: i32) {
    const SWP_NOMOVE: u32 = 0x2;
    const SWP_NOZORDER: u32 = 0x4;
    const SWP_NOACTIVATE: u32 = 0x10;
    // SAFETY: handle value and plain integers.
    unsafe {
        SetWindowPos(hwnd, 0, 0, 0, w, h, SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE);
    }
}

pub fn client_rect(hwnd: Hwnd) -> Rect {
    let mut r = Rect::default();
    // SAFETY: `r` is a valid out value.
    unsafe {
        GetClientRect(hwnd, &mut r);
    }
    r
}

pub fn window_rect(hwnd: Hwnd) -> Rect {
    let mut r = Rect::default();
    // SAFETY: `r` is a valid out value.
    unsafe {
        GetWindowRect(hwnd, &mut r);
    }
    r
}

pub fn set_focus(hwnd: Hwnd) {
    // SAFETY: handle value only.
    unsafe {
        SetFocus(hwnd);
    }
}

pub fn focus() -> Hwnd {
    // SAFETY: no arguments.
    unsafe { GetFocus() }
}

pub fn enable(hwnd: Hwnd, on: bool) {
    // SAFETY: handle value and a flag.
    unsafe {
        EnableWindow(hwnd, BOOL::from(on));
    }
}

pub fn invalidate(hwnd: Hwnd) {
    // SAFETY: a null rectangle means the whole client area.
    unsafe {
        InvalidateRect(hwnd, core::ptr::null(), 0);
    }
}

/// Set the mouse cursor to the arrow or the hand.
pub fn set_cursor(hand: bool) {
    let id = if hand { IDC_HAND } else { IDC_ARROW };
    // SAFETY: predefined cursor ids as in `register_classes`; SetCursor takes the handle LoadCursorW returned.
    unsafe {
        let c = LoadCursorW(0, id as *const u16);
        SetCursor(c);
    }
}

pub fn set_timer(hwnd: Hwnd, id: usize, ms: u32) {
    // SAFETY: a null timer procedure means WM_TIMER messages to the window.
    unsafe {
        SetTimer(hwnd, id, ms, 0);
    }
}

pub fn kill_timer(hwnd: Hwnd, id: usize) {
    // SAFETY: handle value and an id.
    unsafe {
        KillTimer(hwnd, id);
    }
}

/// Whether the key is held down right now.
pub fn key_down(vk: i32) -> bool {
    // SAFETY: plain value argument.
    unsafe { GetKeyState(vk) < 0 }
}

pub fn dpi_for_window(hwnd: Hwnd) -> u32 {
    // SAFETY: handle value only.
    let d = unsafe { GetDpiForWindow(hwnd) };
    if d == 0 { 96 } else { d }
}

/// Per-monitor DPI awareness version 2 (`DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2` is -4). Call before any window
/// exists. Returns whether it took effect.
pub fn set_per_monitor_dpi_v2() -> bool {
    // SAFETY: plain value argument.
    unsafe { SetProcessDpiAwarenessContext(-4) != 0 }
}

/// Lines to scroll per wheel notch (`SPI_GETWHEELSCROLLLINES`); `u32::MAX` means one page.
pub fn wheel_scroll_lines() -> u32 {
    let mut lines: u32 = 3;
    // SAFETY: SPI_GETWHEELSCROLLLINES (0x68) writes one UINT through the pointer.
    let ok = unsafe { SystemParametersInfoW(0x68, 0, (&mut lines as *mut u32).cast(), 0) };
    if ok == 0 { 3 } else { lines }
}

/// The system message font at `dpi` and a symbol font of the same size, as GDI font handles (0 when the system
/// would not make them). The caller deletes them with [`delete_object`].
pub fn make_fonts(dpi: u32) -> (isize, isize) {
    let mut ncm: NonClientMetrics = NonClientMetrics {
        size: core::mem::size_of::<NonClientMetrics>() as u32,
        border_width: 0,
        scroll_width: 0,
        scroll_height: 0,
        caption_width: 0,
        caption_height: 0,
        caption_font: EMPTY_FONT,
        sm_caption_width: 0,
        sm_caption_height: 0,
        sm_caption_font: EMPTY_FONT,
        menu_width: 0,
        menu_height: 0,
        menu_font: EMPTY_FONT,
        status_font: EMPTY_FONT,
        message_font: EMPTY_FONT,
        padded_border_width: 0,
    };
    // SAFETY: SPI_GETNONCLIENTMETRICS (0x29) fills the NONCLIENTMETRICSW whose size is passed in both places.
    let ok = unsafe { SystemParametersInfoForDpi(0x29, ncm.size, (&mut ncm as *mut NonClientMetrics).cast(), 0, dpi) };
    let mut font = ncm.message_font;
    if ok == 0 || font.height == 0 {
        font = EMPTY_FONT;
        font.height = -(12 * dpi as i32 / 96);
        font.weight = 400;
        for (d, s) in font.face_name.iter_mut().zip("Segoe UI".encode_utf16()) {
            *d = s;
        }
    }
    // SAFETY: `font` is a fully initialised LOGFONTW.
    let ui = unsafe { CreateFontIndirectW(&font) };
    let mut symbol = font;
    symbol.height = symbol.height * 5 / 4;
    symbol.face_name = [0; 32];
    for (d, s) in symbol.face_name.iter_mut().zip("Segoe UI Symbol".encode_utf16()) {
        *d = s;
    }
    // SAFETY: as above.
    let sym = unsafe { CreateFontIndirectW(&symbol) };
    (ui, sym)
}

const EMPTY_FONT: LogFont =
    LogFont { height: 0, width: 0, escapement: 0, orientation: 0, weight: 0, italic: 0, underline: 0, strike_out: 0, char_set: 0, out_precision: 0, clip_precision: 0, quality: 0, pitch_and_family: 0, face_name: [0; 32] };

pub fn delete_object(object: isize) {
    if object != 0 {
        // SAFETY: handle value; the caller owns the object and no longer uses it.
        unsafe {
            DeleteObject(object);
        }
    }
}

pub fn create_brush(rgb: u32) -> isize {
    // SAFETY: plain value argument; COLORREF is 0x00BBGGRR.
    unsafe { CreateSolidBrush(rgb) }
}

/// Set the text and background colours of a device context handed over by `WM_CTLCOLORSTATIC`.
pub fn set_static_colors(hdc: isize, text: u32, background: u32) {
    // SAFETY: handle value and colours.
    unsafe {
        SetTextColor(hdc, text);
        SetBkColor(hdc, background);
    }
}

/// Set the range, page size and position of the page window's vertical scroll bar (always shown, disabled when
/// everything fits).
pub fn set_scroll(hwnd: Hwnd, max: i32, page: u32, pos: i32) {
    const SIF_RANGE: u32 = 1;
    const SIF_PAGE: u32 = 2;
    const SIF_POS: u32 = 4;
    const SIF_DISABLENOSCROLL: u32 = 8;
    let info = ScrollInfo { size: core::mem::size_of::<ScrollInfo>() as u32, mask: SIF_RANGE | SIF_PAGE | SIF_POS | SIF_DISABLENOSCROLL, min: 0, max, page, pos, track_pos: 0 };
    // SAFETY: `info` is fully initialised with its size.
    unsafe {
        SetScrollInfo(hwnd, SB_VERT, &info, 1);
    }
}

/// The 32-bit position of the thumb while it is dragged.
pub fn scroll_track_pos(hwnd: Hwnd) -> i32 {
    const SIF_TRACKPOS: u32 = 0x10;
    let mut info = ScrollInfo { size: core::mem::size_of::<ScrollInfo>() as u32, mask: SIF_TRACKPOS, min: 0, max: 0, page: 0, pos: 0, track_pos: 0 };
    // SAFETY: `info` is a valid in/out value with its size set.
    unsafe {
        GetScrollInfo(hwnd, SB_VERT, &mut info);
    }
    info.track_pos
}

/// Send the mouse to `hwnd` until [`release_capture`] (a drag goes on outside the window).
pub fn set_capture(hwnd: Hwnd) {
    // SAFETY: handle value only.
    unsafe {
        SetCapture(hwnd);
    }
}

pub fn release_capture() {
    // SAFETY: no arguments.
    unsafe {
        ReleaseCapture();
    }
}

/// Show or hide the vertical scroll bar of `hwnd`.
pub fn show_vscroll(hwnd: Hwnd, show: bool) {
    // SAFETY: handle value and flags.
    unsafe {
        ShowScrollBar(hwnd, SB_VERT, BOOL::from(show));
    }
}

/// Ask for a `WM_MOUSELEAVE` when the pointer leaves `hwnd`.
pub fn track_mouse_leave(hwnd: Hwnd) {
    let mut data = TrackMouseEventData { size: core::mem::size_of::<TrackMouseEventData>() as u32, flags: 2, hwnd, hover_time: 0 };
    // SAFETY: `data` is initialised with its size; TME_LEAVE is 2.
    unsafe {
        TrackMouseEvent(&mut data);
    }
}

/// OK/Cancel question box; true for OK.
pub fn confirm(hwnd: Hwnd, caption: &str, text: &str) -> bool {
    const MB_OKCANCEL: u32 = 1;
    const MB_ICONQUESTION: u32 = 0x20;
    const IDOK: i32 = 1;
    let (c, t) = (wide(caption), wide(text));
    // SAFETY: both strings are NUL-terminated and outlive the call.
    unsafe { MessageBoxW(hwnd, t.as_ptr(), c.as_ptr(), MB_OKCANCEL | MB_ICONQUESTION) == IDOK }
}

/// Copy a top-down 32-bit BGRA picture of `w` x `h` pixels to the window's client area, its top row at row `y`.
pub fn blit(hwnd: Hwnd, y: u32, w: u32, h: u32, bgra: &[u8]) -> bool {
    if w == 0 || h == 0 || h > i32::MAX as u32 || y > i32::MAX as u32 || (bgra.len() as u64) < u64::from(w) * u64::from(h) * 4 {
        return false;
    }
    let info = BitmapInfo {
        header: BitmapInfoHeader {
            size: core::mem::size_of::<BitmapInfoHeader>() as u32,
            width: w as i32,
            height: -(h as i32),
            planes: 1,
            bit_count: 32,
            compression: 0,
            size_image: 0,
            x_pels: 0,
            y_pels: 0,
            clr_used: 0,
            clr_important: 0,
        },
        colors: [0],
    };
    // SAFETY: the window's own DC is released below; `bgra` holds at least w*h*4 bytes (checked above), which is
    // what a top-down 32-bit DIB of that size reads; `info` is fully initialised.
    unsafe {
        let hdc = GetDC(hwnd);
        if hdc == 0 {
            return false;
        }
        let lines = SetDIBitsToDevice(hdc, 0, y as i32, w, h, 0, 0, 0, h, bgra.as_ptr().cast(), &info, 0);
        ReleaseDC(hwnd, hdc);
        lines != 0
    }
}

/// Wait until GDI has executed every call made so far (so a timing taken after it includes the drawing).
pub fn gdi_flush() {
    // SAFETY: no arguments.
    unsafe {
        GdiFlush();
    }
}

/// Open `target` with the program Windows has registered for it (`mailto:` links).
pub fn shell_open(target: &str) -> bool {
    let (verb, file) = (wide("open"), wide(target));
    // SAFETY: NUL-terminated strings that outlive the call; null for the optional ones. A result above 32 is success.
    unsafe { ShellExecuteW(0, verb.as_ptr(), file.as_ptr(), core::ptr::null(), core::ptr::null(), SW_SHOW) > 32 }
}

/// Milliseconds from the creation of this process to now (`GetProcessTimes` against the system clock).
pub fn since_process_creation_ms() -> Option<f64> {
    let (mut created, mut exited, mut kernel, mut user) = (FileTime::default(), FileTime::default(), FileTime::default(), FileTime::default());
    let mut now = FileTime::default();
    // SAFETY: all four out values are valid; the pseudo handle of the current process needs no closing.
    let ok = unsafe { GetProcessTimes(GetCurrentProcess(), &mut created, &mut exited, &mut kernel, &mut user) };
    // SAFETY: `now` is a valid out value.
    unsafe { GetSystemTimeAsFileTime(&mut now) };
    (ok != 0 && now.ticks() >= created.ticks()).then(|| (now.ticks() - created.ticks()) as f64 / 10_000.0)
}

/// (private bytes now, peak pagefile usage): `PrivateUsage` and `PeakPagefileUsage` of
/// `PROCESS_MEMORY_COUNTERS_EX`.
pub fn process_memory() -> Option<(u64, u64)> {
    let mut c = ProcessMemoryCountersEx { cb: core::mem::size_of::<ProcessMemoryCountersEx>() as u32, ..ProcessMemoryCountersEx::default() };
    // SAFETY: `c` is a valid out value whose size is passed.
    let ok = unsafe { K32GetProcessMemoryInfo(GetCurrentProcess(), &mut c, c.cb) };
    (ok != 0).then_some((c.private_usage as u64, c.peak_pagefile_usage as u64))
}

/// A GUI process has no console of its own. When started from a console without redirected output, attach to the
/// parent's console and point stdout and stderr at it so `println!` is seen. Redirected output (a pipe or a file)
/// already works and is left alone.
pub fn attach_parent_console() {
    const STD_OUTPUT_HANDLE: u32 = -11i32 as u32;
    const STD_ERROR_HANDLE: u32 = -12i32 as u32;
    const ATTACH_PARENT_PROCESS: u32 = u32::MAX;
    use std::os::windows::io::IntoRawHandle;
    // SAFETY: plain value arguments; the returned handle is only compared.
    let out = unsafe { GetStdHandle(STD_OUTPUT_HANDLE) };
    if out != 0 && out != -1 {
        return;
    }
    // SAFETY: plain value argument.
    if unsafe { AttachConsole(ATTACH_PARENT_PROCESS) } == 0 {
        return;
    }
    for which in [STD_OUTPUT_HANDLE, STD_ERROR_HANDLE] {
        if let Ok(f) = std::fs::OpenOptions::new().write(true).open("CONOUT$") {
            // The handle is given to the process for good, so the file is not closed.
            let h = f.into_raw_handle() as isize;
            // SAFETY: `h` is an open console output handle that stays open.
            unsafe {
                SetStdHandle(which, h);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn struct_sizes_match_the_sdk() {
        assert_eq!(core::mem::size_of::<LogFont>(), 92);
        assert_eq!(core::mem::size_of::<NonClientMetrics>(), 504);
        assert_eq!(core::mem::size_of::<WndClassExW>(), 80);
        assert_eq!(core::mem::size_of::<Msg>(), 48);
        assert_eq!(core::mem::size_of::<PaintStruct>(), 72);
        assert_eq!(core::mem::size_of::<ScrollInfo>(), 28);
        assert_eq!(core::mem::size_of::<BitmapInfoHeader>(), 40);
        assert_eq!(core::mem::size_of::<MinMaxInfo>(), 40);
        assert_eq!(core::mem::size_of::<ProcessMemoryCountersEx>(), 80);
    }

    #[test]
    fn process_numbers_are_there() {
        let (private, peak) = process_memory().expect("memory counters");
        assert!(private > 0 && peak >= private / 2);
        assert!(since_process_creation_ms().expect("process times") >= 0.0);
    }
}
