//! The WinHTTP transport: implements the engine's [`Transport`] for `http` and `https`.
//!
//! Redirects are followed here (not by WinHTTP) so that every hop is checked: at most 10, only to
//! http and https, never from https to http. The size cap is enforced on the decompressed bytes while
//! streaming, the connect and total time limits are enforced per request, and a certificate problem
//! is a typed error with no way around it.

use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use qinglan_engine::net::{NetError, Request, Response, Transport, USER_AGENT};
use qinglan_engine::url::{Host, Url};

use crate::sys::winhttp::{HttpRequest, Session, Timeouts};

/// Limits of the transport. The defaults are the ones in the task sheet.
#[derive(Clone, Debug)]
pub struct WinHttpConfig {
    /// Resolving the name and connecting.
    pub connect_timeout: Duration,
    /// Everything, redirects and body included.
    pub total_timeout: Duration,
    pub max_redirects: u32,
    pub user_agent: String,
}

impl Default for WinHttpConfig {
    fn default() -> Self {
        WinHttpConfig { connect_timeout: Duration::from_secs(15), total_timeout: Duration::from_secs(60), max_redirects: 10, user_agent: USER_AGENT.to_string() }
    }
}

/// Status, headers and the open request of one hop.
type Hop = (u16, Vec<(String, String)>, HttpRequest);

pub struct WinHttpTransport {
    session: Session,
    cfg: WinHttpConfig,
}

impl WinHttpTransport {
    pub fn new() -> Result<WinHttpTransport, NetError> {
        WinHttpTransport::with_config(WinHttpConfig::default())
    }

    pub fn with_config(cfg: WinHttpConfig) -> Result<WinHttpTransport, NetError> {
        let session = Session::open(&cfg.user_agent).map_err(|c| map_error(c, ""))?;
        Ok(WinHttpTransport { session, cfg })
    }
}

fn ms(d: Duration) -> u32 {
    u32::try_from(d.as_millis()).unwrap_or(u32::MAX).max(1)
}

/// WinHTTP error code to a typed error. 12000 is `WINHTTP_ERROR_BASE`.
pub fn map_error(code: u32, host: &str) -> NetError {
    match code {
        12002 => NetError::Timeout,
        12007 => NetError::Dns(host.to_string()),
        12017 => NetError::Cancelled,
        12029 | 12030 | 12031 | 12163 => NetError::Connect(format!("connection failed (code {code})")),
        12152 | 12181 | 12182 => NetError::Protocol(format!("invalid server response (code {code})")),
        12156 => NetError::BadRedirect("the redirect was refused".into()),
        // Certificate and TLS problems: date, name, unknown CA, revoked, wrong usage, generic failure.
        12037 | 12038 | 12044 | 12045 | 12057 | 12157 | 12169 | 12170 | 12175 | 12179 => NetError::Tls(match code {
            12037 => "the certificate has expired or is not valid yet".to_string(),
            12038 => "the certificate is for a different site name".to_string(),
            12045 => "the certificate authority is not trusted".to_string(),
            12170 => "the certificate was revoked".to_string(),
            12044 => "the server wants a client certificate".to_string(),
            other => format!("certificate or TLS error (code {other})"),
        }),
        other => NetError::Other(format!("network error (WinHTTP code {other})")),
    }
}

/// Parse the header block of `WinHttpQueryHeaders(RAW_HEADERS_CRLF)`: the status line is skipped;
/// repeated names stay repeated (`Set-Cookie` is never acted on).
fn parse_headers(raw: &str) -> Vec<(String, String)> {
    raw.split("\r\n")
        .skip(1)
        .filter_map(|line| {
            let (n, v) = line.split_once(':')?;
            let n = n.trim();
            (!n.is_empty()).then(|| (n.to_string(), v.trim().to_string()))
        })
        .collect()
}

fn header<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers.iter().find(|(n, _)| n.eq_ignore_ascii_case(name)).map(|(_, v)| v.as_str())
}

fn request_target(url: &Url) -> String {
    let mut s = url.pathname();
    if s.is_empty() {
        s.push('/');
    }
    s.push_str(&url.search());
    s
}

impl WinHttpTransport {
    /// One request without following redirects: status, headers and the open request for the body.
    fn hop(&self, url: &Url, req: &Request, deadline: Instant) -> Result<Hop, NetError> {
        let host = match &url.host {
            Some(Host::Ipv6(_)) => url.hostname().trim_matches(['[', ']']).to_string(),
            Some(_) => url.hostname(),
            None => return Err(NetError::Other("address without a host".into())),
        };
        let secure = url.scheme == "https";
        let port = url.port.unwrap_or(if secure { 443 } else { 80 });
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(NetError::Timeout);
        }
        let to = Timeouts { resolve_ms: ms(self.cfg.connect_timeout.min(left)), connect_ms: ms(self.cfg.connect_timeout.min(left)), send_ms: ms(left), receive_ms: ms(left) };
        let http = self.session.get(&host, port, secure, &request_target(url), &req.headers, to).map_err(|c| {
            if Instant::now() >= deadline && c != 12007 { NetError::Timeout } else { map_error(c, &host) }
        })?;
        let status = http.status().map_err(|c| map_error(c, &host))?;
        let raw = http.raw_headers().map_err(|c| map_error(c, &host))?;
        let status = u16::try_from(status).map_err(|_| NetError::Protocol(format!("status {status}")))?;
        Ok((status, parse_headers(&raw), http))
    }

    fn read_body(&self, http: &HttpRequest, headers: &[(String, String)], req: &Request, deadline: Instant, host: &str) -> Result<Vec<u8>, NetError> {
        let limit = req.max_bytes;
        // A Content-Length over the cap is refused before reading anything.
        if let Some(len) = header(headers, "content-length").and_then(|v| v.trim().parse::<u64>().ok())
            && len > limit as u64
        {
            return Err(NetError::TooLarge { limit });
        }
        let mut body: Vec<u8> = Vec::new();
        let mut buf = vec![0u8; 64 * 1024];
        loop {
            if req.cancel.load(Ordering::Relaxed) {
                return Err(NetError::Cancelled);
            }
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Err(NetError::Timeout);
            }
            http.set_receive_timeout(ms(left));
            let n = http.read(&mut buf).map_err(|c| if Instant::now() >= deadline { NetError::Timeout } else { map_error(c, host) })?;
            if n == 0 {
                return Ok(body);
            }
            // The cap counts decompressed bytes: a gzip bomb stops here, not in memory.
            if body.len() + n > limit {
                return Err(NetError::TooLarge { limit });
            }
            body.extend_from_slice(buf.get(..n).unwrap_or(&[]));
        }
    }
}

fn is_redirect(status: u16) -> bool {
    matches!(status, 301 | 302 | 303 | 307 | 308)
}

impl Transport for WinHttpTransport {
    fn fetch(&self, req: &Request) -> Result<Response, NetError> {
        let deadline = Instant::now() + self.cfg.total_timeout;
        let mut url = req.url.clone();
        let mut hops = 0u32;
        loop {
            if req.cancel.load(Ordering::Relaxed) {
                return Err(NetError::Cancelled);
            }
            let (status, headers, http) = self.hop(&url, req, deadline)?;
            if is_redirect(status)
                && let Some(loc) = header(&headers, "location")
            {
                hops += 1;
                if hops > self.cfg.max_redirects {
                    return Err(NetError::TooManyRedirects);
                }
                let mut next = url.join(loc).map_err(|_| NetError::BadRedirect(format!("bad address in Location: {loc}")))?;
                if next.scheme != "http" && next.scheme != "https" {
                    return Err(NetError::BadRedirect(format!("redirect to {}: is not followed", next.scheme)));
                }
                if url.scheme == "https" && next.scheme == "http" {
                    return Err(NetError::BadRedirect("redirect from https to http".into()));
                }
                next.fragment = None;
                url = next;
                continue;
            }
            let body = self.read_body(&http, &headers, req, deadline, &url.hostname())?;
            return Ok(Response { url, status, headers, body });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_codes_map_to_typed_errors() {
        assert_eq!(map_error(12002, "h"), NetError::Timeout);
        assert_eq!(map_error(12007, "h"), NetError::Dns("h".into()));
        assert!(matches!(map_error(12175, "h"), NetError::Tls(_)));
        assert!(matches!(map_error(12038, "h"), NetError::Tls(m) if m.contains("different site")));
        assert!(matches!(map_error(12029, "h"), NetError::Connect(_)));
        assert!(matches!(map_error(99999, "h"), NetError::Other(_)));
    }

    #[test]
    fn header_block_parsing() {
        let h = parse_headers("HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=GBK\r\nX-A:  b \r\nX-A: c\r\n\r\n");
        assert_eq!(h, [("Content-Type".to_string(), "text/html; charset=GBK".to_string()), ("X-A".to_string(), "b".to_string()), ("X-A".to_string(), "c".to_string())]);
    }
}
