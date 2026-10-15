//! `file:` URLs: reading local files with the same size caps as the network (only the user, or a
//! local page, may cause this; see [`super::policy::plan`]).

use std::io::Read;
use std::path::PathBuf;

use super::{NetError, Response};
use crate::url::{Url, percent_decode};

/// The path a `file:` URL names (`file:///C:/dir/a.html` gives `C:/dir/a.html`).
pub fn url_to_path(url: &Url) -> Option<PathBuf> {
    if url.scheme != "file" {
        return None;
    }
    let raw = percent_decode(url.pathname().as_bytes());
    let mut p = String::from_utf8_lossy(&raw).into_owned();
    let b = p.as_bytes();
    if b.len() >= 3 && b.first() == Some(&b'/') && b.get(2) == Some(&b':') {
        p.remove(0);
    }
    Some(PathBuf::from(p))
}

/// The `file:` URL of an existing path (made absolute).
pub fn url_from_path(path: &str) -> Option<Url> {
    let abs = std::fs::canonicalize(path).ok()?;
    let s = abs.to_string_lossy().replace('\\', "/");
    let s = s.strip_prefix("//?/").unwrap_or(&s);
    Url::parse(&format!("file:///{s}"), None).ok()
}

/// A `Content-Type` guessed from the file name; `None` for unknown extensions (the classifier then
/// looks at the bytes and falls back to HTML).
fn guess_type(path: &std::path::Path) -> Option<&'static str> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    Some(match ext.as_str() {
        "html" | "htm" | "xhtml" | "xht" => "text/html",
        "txt" | "text" | "log" | "md" => "text/plain",
        "css" => "text/css",
        "png" => "image/png",
        "jpg" | "jpeg" | "jpe" => "image/jpeg",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        _ => return None,
    })
}

/// Read the file `url` names. A missing file is a 404 response (so the page shows "not found"), any
/// other failure a [`NetError`]; a file longer than `max_bytes` is [`NetError::TooLarge`].
pub fn read(url: &Url, max_bytes: usize) -> Result<Response, NetError> {
    let path = url_to_path(url).ok_or_else(|| NetError::Other("not a file: address".into()))?;
    let not_found = |url: &Url| Response {
        url: url.clone(),
        status: 404,
        headers: vec![("Content-Type".into(), "text/plain".into())],
        body: b"File not found".to_vec(),
    };
    let file = match std::fs::File::open(&path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(not_found(url)),
        Err(e) => return Err(NetError::Other(format!("cannot read {}: {e}", path.display()))),
    };
    if file.metadata().is_ok_and(|m| m.is_dir()) {
        return Ok(not_found(url));
    }
    let mut body = Vec::new();
    // One byte more than the cap tells "exactly at the cap" from "over".
    let mut limited = file.take(max_bytes as u64 + 1);
    limited.read_to_end(&mut body).map_err(|e| NetError::Other(format!("cannot read {}: {e}", path.display())))?;
    if body.len() > max_bytes {
        return Err(NetError::TooLarge { limit: max_bytes });
    }
    let headers = guess_type(&path).map(|t| vec![("Content-Type".to_string(), t.to_string())]).unwrap_or_default();
    Ok(Response { url: url.clone(), status: 200, headers, body })
}
