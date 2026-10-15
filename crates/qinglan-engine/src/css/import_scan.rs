//! Finding the `@import` addresses of a style sheet without parsing it: the network code needs them
//! to start fetches early, and a sheet can be 200 KB.
//!
//! `@import` is only valid at the top of a sheet, after `@charset` and `@layer a, b;` statements and
//! comments, so only that prefix is scanned.

use super::MAX_SHEETS;

/// The URLs (as written, not resolved) of the `@import` rules at the top of `text`, in order.
pub fn scan_imports(text: &str) -> Vec<String> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;
    // A byte order mark that survived decoding.
    if text.starts_with('\u{feff}') {
        i = 3;
    }
    while out.len() < MAX_SHEETS {
        i = skip_space_and_comments(b, i);
        match b.get(i) {
            Some(b'<') if b.get(i..i + 4) == Some(b"<!--") => {
                i += 4;
                continue;
            }
            Some(b'-') if b.get(i..i + 3) == Some(b"-->") => {
                i += 3;
                continue;
            }
            Some(b'@') => {}
            _ => break,
        }
        let start = i + 1;
        let mut j = start;
        while b.get(j).is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'-' || *c == b'_') {
            j += 1;
        }
        let name = text.get(start..j).unwrap_or("").to_ascii_lowercase();
        let (end, term) = skip_prelude(b, j);
        match name.as_str() {
            "charset" if term == Some(b';') => {}
            "layer" if term == Some(b';') => {}
            "import" => {
                if let Some(u) = import_url(text.get(j..end).unwrap_or("")) {
                    out.push(u);
                }
                if term != Some(b';') {
                    break;
                }
            }
            _ => break,
        }
        i = end + 1;
    }
    out
}

fn skip_space_and_comments(b: &[u8], mut i: usize) -> usize {
    loop {
        while b.get(i).is_some_and(u8::is_ascii_whitespace) {
            i += 1;
        }
        if b.get(i..i + 2) == Some(b"/*") {
            i += 2;
            while i < b.len() && b.get(i..i + 2) != Some(b"*/") {
                i += 1;
            }
            i = (i + 2).min(b.len());
        } else {
            return i;
        }
    }
}

/// Skip a string starting at the quote `b[i]`; returns the index after the closing quote.
fn skip_string(b: &[u8], i: usize) -> usize {
    let Some(&q) = b.get(i) else { return i };
    let mut j = i + 1;
    while let Some(&c) = b.get(j) {
        match c {
            b'\\' => j += 2,
            c if c == q => return j + 1,
            b'\n' => return j,
            _ => j += 1,
        }
    }
    b.len()
}

/// From `from`, find the `;` or `{` that ends the at-rule prelude (outside strings, comments and
/// parentheses). Returns its index and which one it was; `(len, None)` at the end of the text.
fn skip_prelude(b: &[u8], from: usize) -> (usize, Option<u8>) {
    let mut i = from;
    let mut depth = 0u32;
    while let Some(&c) = b.get(i) {
        match c {
            b'"' | b'\'' => i = skip_string(b, i),
            b'/' if b.get(i + 1) == Some(&b'*') => i = skip_space_and_comments(b, i),
            b'(' => {
                depth += 1;
                i += 1;
            }
            b')' => {
                depth = depth.saturating_sub(1);
                i += 1;
            }
            b';' | b'{' if depth == 0 => return (i, Some(c)),
            _ => i += 1,
        }
    }
    (b.len(), None)
}

fn read_quoted(s: &str) -> Option<String> {
    let b = s.as_bytes();
    let end = skip_string(b, 0);
    let inner = s.get(1..end.saturating_sub(1).max(1))?;
    // `skip_string` stops after the closing quote; an unterminated string keeps what there is.
    let closed = end >= 2 && b.get(end - 1) == b.first();
    let inner = if closed { inner } else { s.get(1..end)? };
    Some(inner.to_string())
}

/// The URL of one `@import` prelude: `"x.css" screen`, `url(x.css)`, `url("x.css") layer`.
fn import_url(prelude: &str) -> Option<String> {
    let b = prelude.as_bytes();
    let i = skip_space_and_comments(b, 0);
    let rest = prelude.get(i..)?;
    let url = if rest.starts_with('"') || rest.starts_with('\'') {
        read_quoted(rest)?
    } else if rest.len() >= 4 && rest.get(..4).is_some_and(|p| p.eq_ignore_ascii_case("url(")) {
        let inner = rest.get(4..)?.trim_start();
        if inner.starts_with('"') || inner.starts_with('\'') {
            read_quoted(inner)?
        } else {
            let end = inner.find(|c: char| c == ')' || c.is_ascii_whitespace()).unwrap_or(inner.len());
            inner.get(..end)?.to_string()
        }
    } else {
        return None;
    };
    let url = url.trim().to_string();
    (!url.is_empty()).then_some(url)
}

#[cfg(test)]
mod tests {
    use super::scan_imports;

    #[test]
    fn finds_leading_imports_in_all_forms() {
        let css = "@charset \"utf-8\";\n/* c */ @import 'a.css';\n@import url(b.css) screen;\n@import url( \"c d.css\" ) layer(x);@layer base;\n@import url(e.css);\nbody{color:red}\n@import url(late.css);";
        assert_eq!(scan_imports(css), ["a.css", "b.css", "c d.css", "e.css"]);
    }

    #[test]
    fn stops_at_the_first_rule_and_survives_garbage() {
        assert!(scan_imports("body{} @import 'x.css';").is_empty());
        assert!(scan_imports("@import").is_empty());
        assert!(scan_imports("@import url(").is_empty());
        assert!(scan_imports("@import \"unterminated").len() <= 1);
        assert_eq!(scan_imports("@import url(x.css)"), ["x.css"]);
        assert_eq!(scan_imports("\u{feff}@import \"y.css\";"), ["y.css"]);
        assert_eq!(scan_imports("@media print{} @import 'z.css';").len(), 0);
    }
}
