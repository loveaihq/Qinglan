//! The built-in error pages (L1 task 17): HTML made here and shown by our own engine like any other page.

use qinglan_engine::net::NetError;
use qinglan_engine::net::policy::Refusal;

use crate::i18n::{Strings, fill};

/// Why there is nothing to show.
#[derive(Clone, Debug)]
pub enum Failure {
    Net(NetError),
    Refused(Refusal),
    /// A response type layer 1 does not show.
    Unsupported { mime: String },
    /// The address could not be parsed.
    BadAddress,
    /// The server answered with an error status and no body.
    EmptyStatus(u16),
    /// A panic while loading, laying out or painting the page; the text is the panic message.
    Internal(String),
}

pub struct ErrorPage {
    /// The `<title>` of the page (the window takes it from the document; the tests compare against it).
    #[allow(dead_code)]
    pub title: String,
    pub html: String,
}

pub fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + s.len() / 8);
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

/// The page for `failure` while opening `address`.
pub fn build(failure: &Failure, address: &str, s: &Strings) -> ErrorPage {
    let (title, message, detail): (String, String, String) = match failure {
        Failure::Net(e) => {
            let msg = match e {
                NetError::Timeout => s.err_timeout,
                NetError::Dns(_) => s.err_dns,
                NetError::Connect(_) => s.err_connect,
                NetError::Tls(_) => s.err_tls,
                NetError::TooLarge { .. } => s.err_too_large,
                NetError::TooManyRedirects | NetError::BadRedirect(_) => s.err_redirects,
                NetError::Protocol(_) => s.err_protocol,
                NetError::Cancelled | NetError::Other(_) => s.err_other,
            };
            (s.err_title.to_string(), msg.to_string(), e.to_string())
        }
        Failure::Refused(r) => (s.err_title.to_string(), s.err_refused_body.to_string(), r.to_string()),
        Failure::Unsupported { mime } => (s.err_unsupported_title.to_string(), fill(s.err_unsupported_body, mime), String::new()),
        Failure::BadAddress => (s.err_bad_address_title.to_string(), s.err_bad_address_body.to_string(), String::new()),
        Failure::EmptyStatus(code) => (s.err_title.to_string(), fill(s.err_http_body, &code.to_string()), String::new()),
        Failure::Internal(text) => (s.err_internal_title.to_string(), s.err_internal_body.to_string(), text.chars().take(300).collect()),
    };
    let hint = if matches!(failure, Failure::Internal(_) | Failure::Unsupported { .. }) { String::new() } else { format!("<p>{}</p>", escape_html(s.err_hint)) };
    let detail_html = if detail.is_empty() { String::new() } else { format!("<p class=detail>{}: {}</p>", escape_html(s.err_details), escape_html(&detail)) };
    let html = format!(
        "<!DOCTYPE html><html lang=\"{lang}\"><head><meta charset=\"utf-8\"><title>{title}</title><style>\
         body{{margin:0;padding:48px 32px;font-family:\"Microsoft YaHei UI\",\"Segoe UI\",sans-serif;color:#202124;background:#fff}}\
         main{{max-width:640px;margin:0 auto}}\
         h1{{font-size:24px;font-weight:normal;margin:0 0 16px}}\
         p{{font-size:15px;line-height:1.6;margin:0 0 12px}}\
         .addr{{color:#5f6368}}\
         .detail{{color:#80868b;font-size:13px}}\
         </style></head><body><main><h1>{title}</h1><p>{message}</p><p class=addr>{address}</p>{hint}{detail_html}</main></body></html>",
        lang = s.lang_tag,
        title = escape_html(&title),
        message = escape_html(&message),
        address = escape_html(address),
    );
    ErrorPage { title, html }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::{EN, ZH};
    use qinglan_engine::html::treebuilder::parse_document;
    use qinglan_engine::interact::document_title;

    fn failures() -> Vec<Failure> {
        vec![
            Failure::Net(NetError::Timeout),
            Failure::Net(NetError::Dns("nohost.example".into())),
            Failure::Net(NetError::Connect("refused".into())),
            Failure::Net(NetError::Tls("expired".into())),
            Failure::Net(NetError::TooLarge { limit: 16 << 20 }),
            Failure::Net(NetError::TooManyRedirects),
            Failure::Net(NetError::BadRedirect("ftp".into())),
            Failure::Net(NetError::Protocol("bad".into())),
            Failure::Net(NetError::Cancelled),
            Failure::Net(NetError::Other("odd".into())),
            Failure::Refused(Refusal::MixedContent),
            Failure::Refused(Refusal::UnsupportedScheme("gopher".into())),
            Failure::Unsupported { mime: "application/pdf".into() },
            Failure::BadAddress,
            Failure::EmptyStatus(503),
            Failure::Internal("index out of bounds".into()),
        ]
    }

    #[test]
    fn every_failure_makes_a_page_in_both_languages() {
        for s in [&ZH, &EN] {
            for f in failures() {
                let p = build(&f, "https://example.com/a", s);
                assert!(!p.title.is_empty(), "{f:?}");
                let doc = parse_document(p.html.clone());
                assert_eq!(document_title(&doc), p.title, "{f:?}");
                assert!(p.html.contains("https://example.com/a"));
                assert!(p.html.contains(&format!("lang=\"{}\"", s.lang_tag)));
            }
        }
    }

    #[test]
    fn the_address_and_details_are_escaped() {
        let evil = "http://x/\"><script>alert(1)</script>&";
        let p = build(&Failure::Net(NetError::Other("<b>boom</b>".into())), evil, &EN);
        assert!(!p.html.contains("<script>"));
        assert!(!p.html.contains("<b>"));
        assert!(p.html.contains("&lt;script&gt;") && p.html.contains("&amp;"));
    }

    #[test]
    fn the_internal_error_page_says_so() {
        let p = build(&Failure::Internal("x".into()), "https://a.b/", &ZH);
        assert_eq!(p.title, ZH.err_internal_title);
        assert!(p.html.contains(&escape_html(ZH.err_internal_body)));
        let p = build(&Failure::Internal("x".into()), "https://a.b/", &EN);
        assert_eq!(p.title, "This page triggered an error in Qinglan");
    }

    #[test]
    fn messages_follow_the_failure() {
        let p = build(&Failure::EmptyStatus(503), "a", &EN);
        assert!(p.html.contains("HTTP 503"));
        let p = build(&Failure::Unsupported { mime: "application/pdf".into() }, "a", &EN);
        assert!(p.html.contains("application/pdf"));
        let p = build(&Failure::Net(NetError::Dns("h".into())), "a", &EN);
        assert!(p.html.contains(&escape_html(EN.err_dns)) && p.html.contains(&escape_html(EN.err_hint)));
    }
}
