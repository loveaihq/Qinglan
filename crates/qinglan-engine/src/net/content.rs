//! What a response is (L1 task 13 "non-HTML responses"): HTML, plain text, an image, or something
//! the first layer cannot show.

use super::Response;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ContentKind {
    /// `text/html`, `application/xhtml+xml`, or no type at all and nothing that looks like another type.
    Html,
    /// `text/plain`: shown as pre-formatted text.
    PlainText,
    /// `image/*`: shown alone on a page.
    Image,
    /// Anything else (PDF, archives, `application/octet-stream` ...): the window says "layer 1 does
    /// not download files" and shows the address.
    Unsupported { mime: String },
}

/// Classify a response to a page navigation by its `Content-Type`; when there is none, by the first
/// bytes (images) and otherwise as HTML.
pub fn classify(resp: &Response) -> ContentKind {
    match resp.mime().as_deref() {
        Some("text/html" | "application/xhtml+xml") => ContentKind::Html,
        Some("text/plain") => ContentKind::PlainText,
        Some(m) if m.starts_with("image/") => ContentKind::Image,
        Some(m) => ContentKind::Unsupported { mime: m.to_string() },
        None if sniff_image(&resp.body).is_some() || is_svg_bytes(&resp.body) => ContentKind::Image,
        None => ContentKind::Html,
    }
}

/// The image format a byte string starts with, by its signature.
pub fn sniff_image(b: &[u8]) -> Option<&'static str> {
    if b.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if b.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else if b.starts_with(b"GIF87a") || b.starts_with(b"GIF89a") {
        Some("image/gif")
    } else if b.starts_with(b"BM") && b.len() > 14 {
        Some("image/bmp")
    } else if b.starts_with(&[0, 0, 1, 0]) && b.len() > 6 {
        Some("image/x-icon")
    } else if b.starts_with(b"RIFF") && b.get(8..12) == Some(b"WEBP") {
        Some("image/webp")
    } else {
        None
    }
}

/// Whether the bytes start like an SVG document (`<svg`, or `<?xml` / a doctype / comments first).
pub fn is_svg_bytes(b: &[u8]) -> bool {
    let head = b.get(..b.len().min(1024)).unwrap_or(&[]);
    let text = String::from_utf8_lossy(head);
    let t = text.trim_start_matches('\u{feff}').trim_start();
    if t.starts_with("<svg") {
        return true;
    }
    (t.starts_with("<?xml") || t.starts_with("<!--") || t.starts_with("<!DOCTYPE svg")) && t.contains("<svg")
}

/// Whether an image response is SVG (not decoded in layer 1: the page shows the `alt` text).
pub fn is_svg(resp: &Response) -> bool {
    resp.mime().as_deref() == Some("image/svg+xml") || is_svg_bytes(&resp.body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::url::Url;

    fn resp(ct: Option<&str>, body: &[u8]) -> Response {
        Response {
            url: Url::parse("https://a.example/x", None).expect("url"),
            status: 200,
            headers: ct.map(|c| vec![("Content-Type".to_string(), c.to_string())]).unwrap_or_default(),
            body: body.to_vec(),
        }
    }

    #[test]
    fn classifies_by_type_then_by_bytes() {
        assert_eq!(classify(&resp(Some("text/html; charset=utf-8"), b"")), ContentKind::Html);
        assert_eq!(classify(&resp(Some("TEXT/PLAIN"), b"")), ContentKind::PlainText);
        assert_eq!(classify(&resp(Some("image/png"), b"")), ContentKind::Image);
        assert_eq!(classify(&resp(Some("application/pdf"), b"%PDF")), ContentKind::Unsupported { mime: "application/pdf".into() });
        assert_eq!(classify(&resp(None, b"\x89PNG\r\n\x1a\nxxxx")), ContentKind::Image);
        assert_eq!(classify(&resp(None, b"<p>hi")), ContentKind::Html);
        assert!(is_svg(&resp(Some("image/svg+xml"), b"")));
        assert!(is_svg(&resp(Some("image/png"), b"<?xml version=\"1.0\"?><svg xmlns=\"x\"/>")));
        assert!(!is_svg(&resp(Some("image/png"), b"\x89PNG")));
    }
}
