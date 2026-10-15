//! Recording and replaying pages (L1 task 14).
//!
//! A recording is a directory of plain files: one file per response body (`0001.html`, `0002.css` ...)
//! and `index.txt`, a text file that can be edited by hand to build hostile responses. One block per
//! response, blocks separated by blank lines, `#` starts a comment:
//!
//! ```text
//! url: https://example.com/style.css
//! final: https://cdn.example.com/style.css    (only when redirects ended elsewhere)
//! status: 200
//! body: 0002.css                              (a file in this directory; no line = empty body)
//! header: Content-Type: text/css; charset=utf-8
//! ```
//!
//! [`Recorder`] wraps a real transport and writes what passes through. [`ReplayTransport`] answers only
//! from the directory: an address that is not in the index gets a 404, and the network is never used
//! (the type has no other transport inside).

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use super::{NetError, Request, Response, Transport};
use crate::url::Url;

pub const INDEX_FILE: &str = "index.txt";

const INDEX_HEADER: &str = "# qinglan recording. One block per response; blocks are separated by blank lines.\n\
# url = requested address, final = where redirects ended (default: same), status, body = a file in this\n\
# directory (no line = empty body), header = response header lines. Edit freely.\n";

/// One response in the index.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub url: String,
    pub final_url: Option<String>,
    pub status: u16,
    pub body_file: Option<String>,
    pub headers: Vec<(String, String)>,
}

/// Parse the text of `index.txt`. Lines that are not understood are skipped.
pub fn parse_index(text: &str) -> Vec<Entry> {
    let mut out = Vec::new();
    let mut cur: Option<Entry> = None;
    let flush = |cur: &mut Option<Entry>, out: &mut Vec<Entry>| {
        if let Some(e) = cur.take()
            && !e.url.is_empty()
        {
            out.push(e);
        }
    };
    for line in text.lines() {
        let line = line.trim_end();
        if line.trim().is_empty() {
            flush(&mut cur, &mut out);
            continue;
        }
        if line.trim_start().starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once(':') else { continue };
        let value = value.trim();
        let e = cur.get_or_insert_with(|| Entry { url: String::new(), final_url: None, status: 200, body_file: None, headers: Vec::new() });
        match key.trim().to_ascii_lowercase().as_str() {
            "url" => {
                if !e.url.is_empty() {
                    // A new `url:` line without a blank line before it starts the next block.
                    let next = Entry { url: value.to_string(), final_url: None, status: 200, body_file: None, headers: Vec::new() };
                    let done = std::mem::replace(e, next);
                    out.push(done);
                } else {
                    e.url = value.to_string();
                }
            }
            "final" => e.final_url = Some(value.to_string()),
            "status" => e.status = value.parse().unwrap_or(200),
            "body" => e.body_file = Some(value.to_string()),
            "header" => {
                if let Some((n, v)) = value.split_once(':') {
                    e.headers.push((n.trim().to_string(), v.trim().to_string()));
                }
            }
            _ => {}
        }
    }
    flush(&mut cur, &mut out);
    out
}

fn format_entry(e: &Entry) -> String {
    let mut s = format!("\nurl: {}\n", e.url);
    if let Some(f) = &e.final_url {
        s.push_str(&format!("final: {f}\n"));
    }
    s.push_str(&format!("status: {}\n", e.status));
    if let Some(b) = &e.body_file {
        s.push_str(&format!("body: {b}\n"));
    }
    for (n, v) in &e.headers {
        // Header values cannot hold line breaks; keep the file parseable whatever the server sent.
        let v = v.replace(['\r', '\n'], " ");
        s.push_str(&format!("header: {n}: {v}\n"));
    }
    s
}

/// The headers worth keeping: the body is stored decoded, so its length and encoding headers would
/// lie, and cookies are personal data that layer 1 never uses.
fn recordable_headers(headers: &[(String, String)]) -> Vec<(String, String)> {
    const DROP: [&str; 5] = ["content-length", "content-encoding", "transfer-encoding", "set-cookie", "set-cookie2"];
    headers.iter().filter(|(n, _)| !DROP.iter().any(|d| n.eq_ignore_ascii_case(d))).cloned().collect()
}

fn extension_for(resp: &Response) -> &'static str {
    match resp.mime().as_deref() {
        Some("text/html" | "application/xhtml+xml") => "html",
        Some("text/css") => "css",
        Some("text/plain") => "txt",
        Some("image/png") => "png",
        Some("image/jpeg") => "jpg",
        Some("image/gif") => "gif",
        Some("image/webp") => "webp",
        Some("image/svg+xml") => "svg",
        Some("image/bmp") => "bmp",
        Some("image/x-icon" | "image/vnd.microsoft.icon") => "ico",
        _ => "bin",
    }
}

/// A body file name is a bare file name: no directories, no drive letters, no `..`.
fn safe_name(name: &str) -> bool {
    !name.is_empty() && !name.contains(['/', '\\', ':']) && name != "." && name != ".." && !name.starts_with("..")
}

struct RecorderState {
    next: u32,
    seen: HashMap<String, ()>,
    files: usize,
    bytes: u64,
}

/// Records every successful transfer of the wrapped transport into a directory. Network errors are not
/// recorded (replay answers 404 for them). Each address is recorded once.
pub struct Recorder {
    inner: Arc<dyn Transport>,
    dir: PathBuf,
    state: Mutex<RecorderState>,
}

impl Recorder {
    /// Create `dir` (and an empty index) and wrap `inner`.
    pub fn new(inner: Arc<dyn Transport>, dir: &Path) -> std::io::Result<Recorder> {
        fs::create_dir_all(dir)?;
        fs::write(dir.join(INDEX_FILE), INDEX_HEADER)?;
        Ok(Recorder { inner, dir: dir.to_path_buf(), state: Mutex::new(RecorderState { next: 1, seen: HashMap::new(), files: 0, bytes: 0 }) })
    }

    /// Responses recorded so far and their body bytes.
    pub fn stats(&self) -> (usize, u64) {
        self.state.lock().map(|s| (s.files, s.bytes)).unwrap_or((0, 0))
    }

    fn record(&self, req: &Request, resp: &Response) -> std::io::Result<()> {
        let key = req.url.serialize(true);
        let Ok(mut st) = self.state.lock() else { return Ok(()) };
        if st.seen.contains_key(&key) {
            return Ok(());
        }
        st.seen.insert(key.clone(), ());
        let body_file = if resp.body.is_empty() {
            None
        } else {
            let name = format!("{:04}.{}", st.next, extension_for(resp));
            st.next += 1;
            fs::write(self.dir.join(&name), &resp.body)?;
            Some(name)
        };
        let final_url = resp.url.serialize(true);
        let entry = Entry {
            url: key.clone(),
            final_url: (final_url != key).then_some(final_url),
            status: resp.status,
            body_file,
            headers: recordable_headers(&resp.headers),
        };
        let mut f = fs::OpenOptions::new().append(true).open(self.dir.join(INDEX_FILE))?;
        f.write_all(format_entry(&entry).as_bytes())?;
        st.files += 1;
        st.bytes += resp.body.len() as u64;
        Ok(())
    }
}

impl Transport for Recorder {
    fn fetch(&self, req: &Request) -> Result<Response, NetError> {
        let resp = self.inner.fetch(req)?;
        self.record(req, &resp).map_err(|e| NetError::Other(format!("cannot write the recording: {e}")))?;
        Ok(resp)
    }
}

/// Answers from a recording directory only.
pub struct ReplayTransport {
    dir: PathBuf,
    entries: HashMap<String, Entry>,
}

impl ReplayTransport {
    /// Read `dir/index.txt`. The first block for an address wins.
    pub fn open(dir: &Path) -> std::io::Result<ReplayTransport> {
        let text = fs::read(dir.join(INDEX_FILE))?;
        let mut entries = HashMap::new();
        for e in parse_index(&String::from_utf8_lossy(&text)) {
            // Normalise the key the way requests are: parsed and serialised without the fragment.
            let key = Url::parse(&e.url, None).map(|u| u.serialize(true)).unwrap_or_else(|_| e.url.clone());
            entries.entry(key).or_insert(e);
        }
        Ok(ReplayTransport { dir: dir.to_path_buf(), entries })
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn not_found(url: &Url) -> Response {
        Response { url: url.clone(), status: 404, headers: vec![("Content-Type".into(), "text/plain".into())], body: b"Not in the recording".to_vec() }
    }
}

impl Transport for ReplayTransport {
    fn fetch(&self, req: &Request) -> Result<Response, NetError> {
        let Some(e) = self.entries.get(&req.url.serialize(true)) else { return Ok(ReplayTransport::not_found(&req.url)) };
        let body = match &e.body_file {
            None => Vec::new(),
            Some(name) if safe_name(name) => {
                let path = self.dir.join(name);
                let len = fs::metadata(&path).map(|m| m.len()).map_err(|err| NetError::Other(format!("recording: cannot read {name}: {err}")))?;
                if len > req.max_bytes as u64 {
                    return Err(NetError::TooLarge { limit: req.max_bytes });
                }
                fs::read(&path).map_err(|err| NetError::Other(format!("recording: cannot read {name}: {err}")))?
            }
            Some(name) => return Err(NetError::Other(format!("recording: bad body file name {name}"))),
        };
        let url = e.final_url.as_deref().and_then(|f| Url::parse(f, None).ok()).unwrap_or_else(|| req.url.clone());
        Ok(Response { url, status: e.status, headers: e.headers.clone(), body })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_round_trips_and_tolerates_hand_edits() {
        let e = Entry {
            url: "https://a.example/x".into(),
            final_url: Some("https://b.example/y".into()),
            status: 301,
            body_file: Some("0001.html".into()),
            headers: vec![("Content-Type".into(), "text/html; charset=GBK".into())],
        };
        let text = format!("{INDEX_HEADER}{}\nurl: https://c.example/\nstatus: nonsense\n", format_entry(&e));
        let got = parse_index(&text);
        assert_eq!(got.len(), 2);
        assert_eq!(got.first(), Some(&e));
        assert_eq!(got.get(1).map(|x| x.status), Some(200));
    }

    #[test]
    fn body_names_cannot_leave_the_directory() {
        assert!(safe_name("0001.html"));
        for bad in ["../x", "a/b", "a\\b", "C:x", "..", ".", "", "..x"] {
            assert!(!safe_name(bad), "{bad}");
        }
    }
}
