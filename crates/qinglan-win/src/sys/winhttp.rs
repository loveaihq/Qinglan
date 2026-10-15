//! WinHTTP (`winhttp.dll`): plain C functions, declared by hand from `winhttp.h` (Windows SDK
//! 10.0.26100.0), and a small safe wrapper: a [`Session`] opens [`HttpRequest`]s that have already sent
//! the request and received the response headers.
//!
//! Choices made here (decisions.md "WinHTTP"): HTTP/2 on, automatic system proxy, response
//! decompression (gzip, deflate) by WinHTTP, cookies, automatic redirects and authentication off.
//! Errors are the `GetLastError()` code of the failed call (12000..12199 are WinHTTP's own).

use core::ffi::c_void;
use core::ptr::{NonNull, null, null_mut};

type P = *mut c_void;

#[link(name = "winhttp")]
unsafe extern "system" {
    fn WinHttpOpen(agent: *const u16, access_type: u32, proxy: *const u16, bypass: *const u16, flags: u32) -> P;
    fn WinHttpConnect(session: P, server: *const u16, port: u16, reserved: u32) -> P;
    fn WinHttpOpenRequest(connect: P, verb: *const u16, object: *const u16, version: *const u16, referrer: *const u16, accept: *const *const u16, flags: u32) -> P;
    fn WinHttpSendRequest(req: P, headers: *const u16, headers_len: u32, optional: *const c_void, optional_len: u32, total_len: u32, context: usize) -> i32;
    fn WinHttpReceiveResponse(req: P, reserved: P) -> i32;
    fn WinHttpQueryHeaders(req: P, level: u32, name: *const u16, buffer: *mut c_void, buffer_len: *mut u32, index: *mut u32) -> i32;
    fn WinHttpQueryDataAvailable(req: P, available: *mut u32) -> i32;
    fn WinHttpReadData(req: P, buffer: *mut c_void, to_read: u32, read: *mut u32) -> i32;
    fn WinHttpSetOption(h: P, option: u32, buffer: *const c_void, len: u32) -> i32;
    fn WinHttpSetTimeouts(h: P, resolve: i32, connect: i32, send: i32, receive: i32) -> i32;
    fn WinHttpCloseHandle(h: P) -> i32;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetLastError() -> u32;
}

const WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY: u32 = 4;
const WINHTTP_ACCESS_TYPE_DEFAULT_PROXY: u32 = 0;
const WINHTTP_FLAG_SECURE: u32 = 0x0080_0000;
const WINHTTP_FLAG_ESCAPE_DISABLE: u32 = 0x0000_0040;
const WINHTTP_OPTION_CONNECT_RETRIES: u32 = 4;
const WINHTTP_OPTION_RECEIVE_TIMEOUT: u32 = 6;
const WINHTTP_OPTION_RECEIVE_RESPONSE_TIMEOUT: u32 = 7;
const WINHTTP_OPTION_DISABLE_FEATURE: u32 = 63;
const WINHTTP_OPTION_MAX_CONNS_PER_SERVER: u32 = 73;
const WINHTTP_OPTION_REDIRECT_POLICY: u32 = 88;
const WINHTTP_OPTION_DECOMPRESSION: u32 = 118;
const WINHTTP_OPTION_ENABLE_HTTP_PROTOCOL: u32 = 133;
const WINHTTP_OPTION_REDIRECT_POLICY_NEVER: u32 = 0;
const WINHTTP_DISABLE_COOKIES: u32 = 0x1;
const WINHTTP_DISABLE_REDIRECTS: u32 = 0x2;
const WINHTTP_DISABLE_AUTHENTICATION: u32 = 0x4;
const WINHTTP_DECOMPRESSION_FLAG_ALL: u32 = 0x3;
const WINHTTP_PROTOCOL_FLAG_HTTP2: u32 = 0x1;
const WINHTTP_QUERY_STATUS_CODE: u32 = 19;
const WINHTTP_QUERY_RAW_HEADERS_CRLF: u32 = 22;
const WINHTTP_QUERY_FLAG_NUMBER: u32 = 0x2000_0000;

fn last_error() -> u32 {
    // SAFETY: GetLastError has no preconditions.
    unsafe { GetLastError() }
}

fn wide_nul(s: &str) -> Vec<u16> {
    let mut v: Vec<u16> = s.encode_utf16().collect();
    v.push(0);
    v
}

/// An owned `HINTERNET`, closed on drop.
struct Handle(NonNull<c_void>);

// SAFETY: WinHTTP handles are not tied to the thread that created them; the documentation allows a
// session, its connections and its requests to be used from several threads (one thread per request at
// a time, which our `&mut`-free wrappers guarantee by never sharing a request).
unsafe impl Send for Handle {}
// SAFETY: see above; a `Session` is only used to open new connections, which WinHTTP serialises itself.
unsafe impl Sync for Handle {}

impl Handle {
    fn new(p: P) -> Result<Handle, u32> {
        match NonNull::new(p) {
            Some(p) => Ok(Handle(p)),
            None => Err(last_error()),
        }
    }
    fn raw(&self) -> P {
        self.0.as_ptr()
    }
    fn set_u32(&self, option: u32, value: u32) -> bool {
        // SAFETY: the handle is open; `value` is a live u32 and its size is passed.
        unsafe { WinHttpSetOption(self.raw(), option, (&raw const value).cast(), 4) != 0 }
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        // SAFETY: we own the handle and close it exactly once.
        unsafe {
            WinHttpCloseHandle(self.raw());
        }
    }
}

/// Limits applied to one request, in milliseconds (0 means none).
#[derive(Clone, Copy, Debug)]
pub struct Timeouts {
    pub resolve_ms: u32,
    pub connect_ms: u32,
    pub send_ms: u32,
    pub receive_ms: u32,
}

/// A WinHTTP session: connection pool, proxy settings, HTTP/2 negotiation.
pub struct Session(Handle);

impl Session {
    /// Open a session with `user_agent`. Windows 8.1 and later resolve the proxy automatically
    /// (WPAD / PAC / system settings); older systems use the machine default.
    pub fn open(user_agent: &str) -> Result<Session, u32> {
        let ua = wide_nul(user_agent);
        // SAFETY: `ua` is a NUL-terminated UTF-16 string alive for the call; the other pointers are null.
        let mut p = unsafe { WinHttpOpen(ua.as_ptr(), WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY, null(), null(), 0) };
        if p.is_null() {
            // SAFETY: as above.
            p = unsafe { WinHttpOpen(ua.as_ptr(), WINHTTP_ACCESS_TYPE_DEFAULT_PROXY, null(), null(), 0) };
        }
        let h = Handle::new(p)?;
        // Best effort: HTTP/2 needs Windows 10 1607 or later; the connection limit matches the loader's.
        h.set_u32(WINHTTP_OPTION_ENABLE_HTTP_PROTOCOL, WINHTTP_PROTOCOL_FLAG_HTTP2);
        h.set_u32(WINHTTP_OPTION_MAX_CONNS_PER_SERVER, 6);
        Ok(Session(h))
    }

    /// Send `GET path_and_query` to `host:port` with `headers` and wait for the response headers.
    /// `host` has no brackets (IPv6 literals are passed bare).
    pub fn get(&self, host: &str, port: u16, secure: bool, path_and_query: &str, headers: &[(String, String)], timeouts: Timeouts) -> Result<HttpRequest, u32> {
        let server = wide_nul(host);
        // SAFETY: the session handle is open and `server` is a NUL-terminated UTF-16 string.
        let connect = Handle::new(unsafe { WinHttpConnect(self.0.raw(), server.as_ptr(), port, 0) })?;
        let verb = wide_nul("GET");
        let object = wide_nul(path_and_query);
        let flags = WINHTTP_FLAG_ESCAPE_DISABLE | if secure { WINHTTP_FLAG_SECURE } else { 0 };
        // SAFETY: `connect` is open; verb and object are NUL-terminated UTF-16; null selects defaults.
        let req = Handle::new(unsafe { WinHttpOpenRequest(connect.raw(), verb.as_ptr(), object.as_ptr(), null(), null(), null(), flags) })?;
        // Cookies, redirects and credentials off: these protect the user and keep requests independent.
        if !req.set_u32(WINHTTP_OPTION_DISABLE_FEATURE, WINHTTP_DISABLE_COOKIES | WINHTTP_DISABLE_REDIRECTS | WINHTTP_DISABLE_AUTHENTICATION) {
            return Err(last_error());
        }
        req.set_u32(WINHTTP_OPTION_REDIRECT_POLICY, WINHTTP_OPTION_REDIRECT_POLICY_NEVER);
        // Ask for and undo gzip / deflate (a failure just means the server gets no Accept-Encoding).
        req.set_u32(WINHTTP_OPTION_DECOMPRESSION, WINHTTP_DECOMPRESSION_FLAG_ALL);
        let to = |ms: u32| i32::try_from(ms).unwrap_or(i32::MAX);
        // SAFETY: the request handle is open; the timeouts are plain integers.
        let ok = unsafe { WinHttpSetTimeouts(req.raw(), to(timeouts.resolve_ms), to(timeouts.connect_ms), to(timeouts.send_ms), to(timeouts.receive_ms)) };
        if ok == 0 {
            return Err(last_error());
        }
        // Waiting for the response headers has its own limit (default 90 s), and a timed-out connect is
        // not retried on the next address: the caller's total time limit is the one that counts.
        req.set_u32(WINHTTP_OPTION_RECEIVE_RESPONSE_TIMEOUT, timeouts.receive_ms.max(1));
        req.set_u32(WINHTTP_OPTION_CONNECT_RETRIES, 0);
        let mut header_text = String::new();
        for (n, v) in headers {
            // Header names and values come from our own code, but strip line breaks so that nothing
            // can smuggle a second header into the request.
            let clean = |s: &str| s.replace(['\r', '\n'], " ");
            header_text.push_str(&clean(n));
            header_text.push_str(": ");
            header_text.push_str(&clean(v));
            header_text.push_str("\r\n");
        }
        let wide_headers = wide_nul(&header_text);
        let (hp, hl) = if header_text.is_empty() { (null(), 0) } else { (wide_headers.as_ptr(), u32::MAX) };
        // SAFETY: `hp` is null or NUL-terminated UTF-16 (length -1 means "up to the NUL"); no body is sent.
        if unsafe { WinHttpSendRequest(req.raw(), hp, hl, null(), 0, 0, 0) } == 0 {
            return Err(last_error());
        }
        // SAFETY: the request was sent; the reserved argument must be null.
        if unsafe { WinHttpReceiveResponse(req.raw(), null_mut()) } == 0 {
            return Err(last_error());
        }
        Ok(HttpRequest { req, _connect: connect })
    }
}

/// A request whose response headers have arrived; the body is read with [`HttpRequest::read`].
pub struct HttpRequest {
    // Field order is drop order: the request closes before its connection.
    req: Handle,
    _connect: Handle,
}

impl HttpRequest {
    /// The HTTP status code.
    pub fn status(&self) -> Result<u32, u32> {
        let mut status = 0u32;
        let mut len = 4u32;
        // SAFETY: the buffer is a live u32 and `len` says 4 bytes (a NUMBER query writes a u32).
        let ok = unsafe { WinHttpQueryHeaders(self.req.raw(), WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER, null(), (&raw mut status).cast(), &raw mut len, null_mut()) };
        if ok == 0 { Err(last_error()) } else { Ok(status) }
    }

    /// The status line and all header lines, CRLF separated.
    pub fn raw_headers(&self) -> Result<String, u32> {
        let mut len = 0u32;
        // SAFETY: a null buffer with length 0 asks WinHTTP for the needed size (the call then fails with
        // ERROR_INSUFFICIENT_BUFFER and sets `len`, in bytes).
        unsafe {
            WinHttpQueryHeaders(self.req.raw(), WINHTTP_QUERY_RAW_HEADERS_CRLF, null(), null_mut(), &raw mut len, null_mut());
        }
        if len == 0 || len > 4 * 1024 * 1024 {
            return Err(last_error());
        }
        let mut buf = vec![0u16; len as usize / 2 + 1];
        let mut bytes = (buf.len() * 2) as u32;
        // SAFETY: `buf` has `bytes` bytes of room.
        let ok = unsafe { WinHttpQueryHeaders(self.req.raw(), WINHTTP_QUERY_RAW_HEADERS_CRLF, null(), buf.as_mut_ptr().cast(), &raw mut bytes, null_mut()) };
        if ok == 0 {
            return Err(last_error());
        }
        let units = (bytes as usize / 2).min(buf.len());
        Ok(String::from_utf16_lossy(buf.get(..units).unwrap_or(&[])))
    }

    /// Limit the next `read` (and the wait for it) to `ms` milliseconds.
    pub fn set_receive_timeout(&self, ms: u32) {
        self.req.set_u32(WINHTTP_OPTION_RECEIVE_TIMEOUT, ms.max(1));
    }

    /// Read what has arrived, up to `buf.len()` bytes of the (decompressed) body; 0 means the end.
    ///
    /// `WinHttpReadData` alone waits until the whole buffer is full, so a server that sends a byte now
    /// and then would keep a read going for as long as it likes. Asking `WinHttpQueryDataAvailable`
    /// first (it returns as soon as some data is there) and reading only that much keeps every call
    /// short, so the caller's deadline is checked often.
    pub fn read(&self, buf: &mut [u8]) -> Result<usize, u32> {
        let mut avail = 0u32;
        // SAFETY: `avail` is a live u32.
        if unsafe { WinHttpQueryDataAvailable(self.req.raw(), &raw mut avail) } == 0 {
            return Err(last_error());
        }
        if avail == 0 {
            return Ok(0);
        }
        let want = avail.min(u32::try_from(buf.len()).unwrap_or(u32::MAX));
        let mut got = 0u32;
        // SAFETY: `buf` is valid for `want` <= buf.len() bytes; `got` is a live u32.
        let ok = unsafe { WinHttpReadData(self.req.raw(), buf.as_mut_ptr().cast(), want, &raw mut got) };
        if ok == 0 { Err(last_error()) } else { Ok((got as usize).min(buf.len())) }
    }
}
