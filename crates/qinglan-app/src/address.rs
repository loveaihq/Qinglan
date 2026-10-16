//! What the address bar accepts (L1 task 17): an address with a scheme is opened as it is, something that looks
//! like a host name gets `https://` in front, anything else is a search.

/// The search page used for anything that is not an address.
pub const SEARCH_URL: &str = "https://html.duckduckgo.com/html/?q=";

/// Schemes that are recognised without `//` after the colon.
const BARE_SCHEMES: [&str; 8] = ["mailto", "data", "about", "javascript", "file", "view-source", "tel", "blob"];

/// Turn what the user typed into the address to open. `None` for empty input.
pub fn classify(input: &str) -> Option<String> {
    let t = input.trim();
    if t.is_empty() {
        return None;
    }
    if has_scheme(t) {
        return Some(t.to_string());
    }
    if let Some(u) = file_path_url(t) {
        return Some(u);
    }
    if looks_like_host(t) {
        return Some(format!("https://{t}"));
    }
    Some(format!("{SEARCH_URL}{}", encode_query(t)))
}

/// `scheme://...` for any scheme, or one of the well-known schemes that are written without slashes.
fn has_scheme(t: &str) -> bool {
    let Some(colon) = t.find(':') else { return false };
    let scheme = t.get(..colon).unwrap_or("");
    let valid = scheme.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) && scheme.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'));
    if !valid {
        return false;
    }
    t.get(colon + 1..).is_some_and(|rest| rest.starts_with("//")) || BARE_SCHEMES.iter().any(|s| scheme.eq_ignore_ascii_case(s))
}

/// `C:\dir\page.html`, `C:/dir/page.html` and `\\server\share\page.html` become `file:` addresses.
fn file_path_url(t: &str) -> Option<String> {
    let b = t.as_bytes();
    if b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && (b[2] == b'\\' || b[2] == b'/') {
        return Some(format!("file:///{}", t.replace('\\', "/")));
    }
    t.strip_prefix("\\\\").map(|rest| format!("file://{}", rest.replace('\\', "/")))
}

/// No white space, and the host part (without user name, port, path) is `localhost`, has a dot, or is an IPv6 literal.
fn looks_like_host(t: &str) -> bool {
    if t.chars().any(char::is_whitespace) {
        return false;
    }
    let authority = t.split(['/', '?', '#']).next().unwrap_or("");
    let host_port = authority.rsplit('@').next().unwrap_or("");
    if host_port.starts_with('[') {
        return host_port.contains(']');
    }
    let host = match host_port.rsplit_once(':') {
        Some((h, port)) if !port.is_empty() && port.bytes().all(|c| c.is_ascii_digit()) => h,
        _ => host_port,
    };
    !host.is_empty() && (host.eq_ignore_ascii_case("localhost") || (host.contains('.') && !host.starts_with('.')))
}

/// Percent-encode a query for the search address: unreserved characters stay, a space is `+`, everything else is
/// percent-encoded UTF-8.
pub fn encode_query(q: &str) -> String {
    let mut out = String::with_capacity(q.len() * 2);
    for b in q.trim().bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => out.push(b as char),
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(s: &str) -> String {
        classify(s).unwrap_or_default()
    }

    #[test]
    fn addresses_with_a_scheme_are_opened_as_typed() {
        assert_eq!(c("https://example.com/a?b=c#d"), "https://example.com/a?b=c#d");
        assert_eq!(c("  http://example.com  "), "http://example.com");
        assert_eq!(c("ftp://example.com/x"), "ftp://example.com/x");
        assert_eq!(c("file:///C:/x.html"), "file:///C:/x.html");
        assert_eq!(c("mailto:a@b.c"), "mailto:a@b.c");
        assert_eq!(c("javascript:alert(1)"), "javascript:alert(1)");
        assert_eq!(c("about:blank"), "about:blank");
        assert_eq!(c("HTTPS://EXAMPLE.COM"), "HTTPS://EXAMPLE.COM");
    }

    #[test]
    fn host_names_get_https() {
        assert_eq!(c("example.com"), "https://example.com");
        assert_eq!(c("www.example.com/path?q=1"), "https://www.example.com/path?q=1");
        assert_eq!(c("localhost"), "https://localhost");
        assert_eq!(c("localhost:8080/x"), "https://localhost:8080/x");
        assert_eq!(c("LocalHost"), "https://LocalHost");
        assert_eq!(c("127.0.0.1:3000"), "https://127.0.0.1:3000");
        assert_eq!(c("user@host.example/x"), "https://user@host.example/x");
        assert_eq!(c("[::1]:8080"), "https://[::1]:8080");
        assert_eq!(c("\u{4e2d}\u{6587}.\u{7f51}\u{7ad9}"), "https://\u{4e2d}\u{6587}.\u{7f51}\u{7ad9}");
    }

    #[test]
    fn everything_else_is_a_search() {
        assert_eq!(c("rust"), format!("{SEARCH_URL}rust"));
        assert_eq!(c("rust lang"), format!("{SEARCH_URL}rust+lang"));
        assert_eq!(c("example.com is down"), format!("{SEARCH_URL}example.com+is+down"));
        assert_eq!(c("a&b=c d"), format!("{SEARCH_URL}a%26b%3Dc+d"));
        assert_eq!(c("\u{4e2d}\u{6587}"), format!("{SEARCH_URL}%E4%B8%AD%E6%96%87"));
        assert_eq!(c(".hidden"), format!("{SEARCH_URL}.hidden"));
        assert_eq!(c("what's up?"), format!("{SEARCH_URL}what%27s+up%3F"));
    }

    #[test]
    fn windows_paths_become_file_addresses() {
        assert_eq!(c("C:\\Users\\me\\a b.html"), "file:///C:/Users/me/a b.html");
        assert_eq!(c("d:/x/y.html"), "file:///d:/x/y.html");
        assert_eq!(c("\\\\server\\share\\p.html"), "file://server/share/p.html");
    }

    #[test]
    fn empty_input_is_nothing() {
        assert_eq!(classify(""), None);
        assert_eq!(classify("   \t "), None);
    }
}
