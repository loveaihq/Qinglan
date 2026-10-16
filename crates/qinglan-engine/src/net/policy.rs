//! What may be fetched, and how (L1 task 13): schemes, mixed content, `Referer`.
//!
//! Pure functions of URLs, so they are tested without any network.

use std::fmt;

use super::ResourceKind;
use crate::url::Url;

/// How a permitted URL is fetched.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Plan {
    /// Through the transport (`http`, `https`).
    Network,
    /// Read from disk (`file`).
    File,
    /// Decoded from the URL itself (`data`).
    Data,
}

/// Why a URL is not fetched. Not an error of the network: the caller shows a status-bar message, asks
/// the user (`mailto:`), or marks the resource broken.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Refusal {
    /// An `http:` sub-resource of an `https:` page.
    MixedContent,
    /// A `file:` link, or a `file:` sub-resource of a page that is not itself a local file.
    LocalFile,
    /// A `file:` address that is not a plain local file: a network share, a device path.
    NotLocalFile,
    /// `data:` cannot be opened as a page (only as a sub-resource).
    DataDocument,
    /// `mailto:`: the window asks the user before handing it to the mail program.
    Mailto,
    /// `javascript:` (no scripts in layer 1).
    Javascript,
    /// Any other scheme.
    UnsupportedScheme(String),
    /// Too many requests are already waiting.
    QueueFull,
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Refusal::MixedContent => write!(f, "blocked: an insecure (http) resource on a secure (https) page"),
            Refusal::LocalFile => write!(f, "blocked: pages cannot load local files"),
            Refusal::NotLocalFile => write!(f, "blocked: only files on this computer can be opened (no network shares or devices)"),
            Refusal::DataDocument => write!(f, "data: addresses cannot be opened as a page"),
            Refusal::Mailto => write!(f, "mailto: links are handed to the mail program"),
            Refusal::Javascript => write!(f, "javascript: links are not supported"),
            Refusal::UnsupportedScheme(s) => write!(f, "the address scheme \"{s}\" is not supported"),
            Refusal::QueueFull => write!(f, "too many resources requested"),
        }
    }
}

/// Decide how `url` may be fetched. `page` is the address of the page asking (`None`: the user typed
/// the address, or it came from the command line); `kind` says whether it is a navigation
/// ([`ResourceKind::Document`]) or a sub-resource of `page`.
///
/// * `http`/`https`: always, except an `http` sub-resource of an `https` page (mixed content).
/// * `file`: only as a page the user asked for (`page` is `None`: typed, command line, history, reload;
///   a link, a new window started from a link or a sub-resource carries the page it came from). A local page may load `file:` sub-resources (style
///   sheets, images) so that saved pages work; no other page may, and no link to `file:` is followed.
/// * `data`: sub-resources only (the bytes are in the URL; no transport involved).
/// * `mailto`, `javascript` and everything else: not fetched.
pub fn plan(url: &Url, kind: ResourceKind, page: Option<&Url>) -> Result<Plan, Refusal> {
    let navigation = kind == ResourceKind::Document;
    match url.scheme.as_str() {
        "http" | "https" => {
            if url.host.is_none() {
                return Err(Refusal::UnsupportedScheme(url.scheme.clone()));
            }
            if !navigation && url.scheme == "http" && page.is_some_and(|p| p.scheme == "https") {
                return Err(Refusal::MixedContent);
            }
            Ok(Plan::Network)
        }
        "file" => {
            match page {
                None => {}
                Some(p) if !navigation && p.scheme == "file" => {}
                Some(_) => return Err(Refusal::LocalFile),
            }
            // Whoever asks: local files only (no network share, no device path).
            if super::file::url_to_path(url).is_none() {
                return Err(Refusal::NotLocalFile);
            }
            Ok(Plan::File)
        }
        "data" if navigation => Err(Refusal::DataDocument),
        "data" => Ok(Plan::Data),
        "mailto" => Err(Refusal::Mailto),
        "javascript" => Err(Refusal::Javascript),
        other => Err(Refusal::UnsupportedScheme(other.to_string())),
    }
}

/// Mixed content upgrade: an `http:` image on an `https:` page is fetched over `https:` instead, as browsers
/// do for images (the same host, the default port 80 becomes 443, another port stays). If the server does not
/// answer over https the image is broken; nothing falls back to http. Style sheets and everything else are
/// not upgraded and stay blocked ([`plan`]).
pub fn upgrade_mixed_image(url: &mut Url, page: &Url) {
    if url.scheme == "http" && page.scheme == "https" && url.host.is_some() {
        url.scheme = "https".to_string();
        if url.port == Some(80) {
            url.port = None;
        }
    }
}

/// Longest `mailto:` address handed to the mail program (what the page asks for is cut here).
pub const MAX_MAILTO: usize = 2048;

/// The `mailto:` address to show the user and hand to the mail program, rebuilt from `url` so that only
/// the characters of RFC 6068 get through: letters, digits, `- . _ ~` and the delimiters `! $ ' ( ) * + , ; : @`
/// and `? = &` (they structure the address); a valid `%XX` escape stays, and every other byte (space, `"`,
/// `\`, `/`, `%` not followed by two hex digits, control characters, non-ASCII) becomes `%XX`, so the
/// address cannot break out of the quoted argument in the mail program's command line. The fragment is
/// dropped and the result is cut at [`MAX_MAILTO`] bytes (between two units, never inside an escape).
pub fn sanitize_mailto(url: &Url) -> String {
    let full = url.serialize(true);
    let rest = full.strip_prefix("mailto:").unwrap_or("");
    let bytes = rest.as_bytes();
    let mut out = String::from("mailto:");
    let mut i = 0;
    while let Some(&b) = bytes.get(i) {
        let valid_escape = b == b'%' && bytes.get(i + 1).is_some_and(u8::is_ascii_hexdigit) && bytes.get(i + 2).is_some_and(u8::is_ascii_hexdigit);
        let unit = if valid_escape {
            i += 3;
            rest.get(i - 3..i).unwrap_or("%25").to_string()
        } else {
            i += 1;
            if b.is_ascii_alphanumeric() || b"-._~!$'()*+,;:@?=&".contains(&b) { char::from(b).to_string() } else { format!("%{b:02X}") }
        };
        if out.len() + unit.len() > MAX_MAILTO {
            break;
        }
        out.push_str(&unit);
    }
    out
}

/// Longest `Referer` sent whole; a longer one is cut to the origin (as browsers do).
const MAX_REFERER: usize = 4096;

/// The `Referer` header for a request from `page` to `target`: referrer policy
/// `strict-origin-when-cross-origin` (Referrer Policy 3.1):
///
/// * same origin: the page address without credentials and fragment;
/// * cross origin: only the origin of the page (`https://example.com/`);
/// * https page to an http target (a downgrade): nothing.
///
/// Pages that are not http(s) (`file:`, `data:`) send none.
pub fn referer(page: &Url, target: &Url) -> Option<String> {
    let web = |u: &Url| u.scheme == "http" || u.scheme == "https";
    if !web(page) || !web(target) || page.host.is_none() {
        return None;
    }
    if page.scheme == "https" && target.scheme == "http" {
        return None;
    }
    let origin = format!("{}/", page.origin());
    if page.origin() != target.origin() {
        return Some(origin);
    }
    let mut p = page.clone();
    p.username.clear();
    p.password.clear();
    p.fragment = None;
    let full = p.serialize(true);
    Some(if full.len() > MAX_REFERER { origin } else { full })
}

/// The key for the per-host connection limit: scheme, host and port.
pub fn host_key(url: &Url) -> String {
    format!("{}://{}", url.scheme, url.host_with_port())
}
