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
/// * `file`: only as a page the user asked for. A local page may load `file:` sub-resources (style
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
        "file" => match page {
            None => Ok(Plan::File),
            Some(p) if !navigation && p.scheme == "file" => Ok(Plan::File),
            Some(_) => Err(Refusal::LocalFile),
        },
        "data" if navigation => Err(Refusal::DataDocument),
        "data" => Ok(Plan::Data),
        "mailto" => Err(Refusal::Mailto),
        "javascript" => Err(Refusal::Javascript),
        other => Err(Refusal::UnsupportedScheme(other.to_string())),
    }
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
