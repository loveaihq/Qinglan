//! A small XML parser for standalone SVG files (`<img src=x.svg>`, CSS backgrounds, `data:` images).
//!
//! XML 1.0 (5th ed.) 2 and 3 as far as SVG files use them: elements, attributes, namespaces (`xmlns`, `xmlns:p`), character
//! data, CDATA sections, comments, processing instructions, the five predefined entities (4.6) and numeric character
//! references (4.1). The DOCTYPE is skipped and **no entity is ever expanded from it**: a reference to a name that is not one
//! of the five is a well-formedness error, except when the DOCTYPE declares entities (Illustrator files write
//! `xmlns="&ns_svg;"`), where it is dropped. Every well-formedness error is an `Err`: the image is broken, like in browsers.
//! The result is a [`Document`] of the same kind the HTML parser makes, so the CSS engine styles it unchanged.

use crate::html::atoms::{Interner, a};
use crate::html::dom::{Attrs, Document, ElementData, NONE, Namespace, NodeData, NodeId};

const SVG_NS: &str = "http://www.w3.org/2000/svg";
const XLINK_NS: &str = "http://www.w3.org/1999/xlink";
const XHTML_NS: &str = "http://www.w3.org/1999/xhtml";
const XML_NS: &str = "http://www.w3.org/XML/1998/namespace";

/// Nodes (elements and text) per document; more is an error.
pub const MAX_NODES: usize = 150_000;
/// Open elements.
pub const MAX_DEPTH: usize = 256;
/// Attributes per element.
const MAX_ATTRS: usize = 256;
/// Namespace declarations in scope at once (those of all the open elements together); more is an error. It bounds the memory
/// of the bindings (256 open elements x 256 attributes would be 65k) and the worst lookup (see [`P::lookup`]).
const MAX_NS_DECLS: usize = 1024;
/// Buckets of the prefix table, and the end of a bucket's chain.
const BUCKETS: usize = 256;
const NIL: u32 = u32::MAX;

/// A namespace declaration in scope; `prev` is the next older one whose prefix is in the same bucket.
struct Decl {
    prefix: String,
    uri: String,
    prev: u32,
}

fn bucket(prefix: &str) -> usize {
    prefix.bytes().fold(0usize, |h, b| h.wrapping_mul(31).wrapping_add(usize::from(b))) % BUCKETS
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum XmlError {
    /// Not well-formed XML.
    Syntax,
    /// More nodes or a deeper nesting than the limits.
    TooLarge,
    /// Well-formed, but the root is not an `svg` element.
    NotSvg,
}

struct P<'a> {
    s: &'a str,
    i: usize,
    doc: Document,
    /// The declarations in scope, innermost last (the empty prefix is the default namespace). Closing an element pops its own.
    decls: Vec<Decl>,
    /// The newest declaration of each bucket of prefixes (an index into `decls`), `NIL` for none: a lookup walks one short chain
    /// instead of every scope.
    heads: [u32; BUCKETS],
    loose: bool,
}

impl P<'_> {
    fn rest(&self) -> &str {
        self.s.get(self.i..).unwrap_or("")
    }
    fn at(&self) -> u8 {
        self.s.as_bytes().get(self.i).copied().unwrap_or(0)
    }
    fn starts(&self, pat: &str) -> bool {
        self.rest().starts_with(pat)
    }
    fn ws(&mut self) {
        while matches!(self.at(), b' ' | b'\t' | b'\n' | b'\r') {
            self.i += 1;
        }
    }
    /// Skip to just after `pat`.
    fn skip_past(&mut self, pat: &str) -> Result<(), XmlError> {
        let at = self.rest().find(pat).ok_or(XmlError::Syntax)?;
        self.i += at + pat.len();
        Ok(())
    }
    fn name(&mut self) -> Result<&str, XmlError> {
        let start = self.i;
        while self.at() != 0 && !matches!(self.at(), b' ' | b'\t' | b'\n' | b'\r' | b'/' | b'>' | b'=' | b'<' | b'"' | b'\'') {
            self.i += 1;
        }
        let n = self.s.get(start..self.i).unwrap_or("");
        let first = n.chars().next().ok_or(XmlError::Syntax)?;
        if first.is_ascii_digit() || matches!(first, '-' | '.') {
            return Err(XmlError::Syntax);
        }
        Ok(n)
    }
    /// What `prefix` is bound to: the newest declaration with that prefix, found along its bucket's chain (a few entries; at most
    /// `MAX_NS_DECLS` if every prefix of the file lands in one bucket).
    fn lookup(&self, prefix: &str) -> Option<&str> {
        let mut i = self.heads.get(bucket(prefix)).copied()?;
        while let Some(d) = self.decls.get(i as usize) {
            if d.prefix == prefix {
                return Some(&d.uri);
            }
            i = d.prev;
        }
        None
    }
    /// Bind `prefix` to `uri` until the element being opened is closed.
    fn declare(&mut self, prefix: &str, uri: &str) -> Result<(), XmlError> {
        if self.decls.len() >= MAX_NS_DECLS {
            return Err(XmlError::TooLarge);
        }
        let head = self.heads.get_mut(bucket(prefix)).ok_or(XmlError::Syntax)?;
        self.decls.push(Decl { prefix: prefix.to_string(), uri: uri.to_string(), prev: *head });
        *head = (self.decls.len() - 1) as u32;
        Ok(())
    }
    /// Close the scopes opened after the first `len` declarations: the newest declaration of a bucket is the head of its chain.
    fn undeclare_to(&mut self, len: usize) {
        while self.decls.len() > len {
            let Some(d) = self.decls.pop() else { break };
            if let Some(h) = self.heads.get_mut(bucket(&d.prefix)) {
                *h = d.prev;
            }
        }
    }
}

/// XML 1.0 2.2 `Char`.
fn xml_char(c: u32) -> Option<char> {
    match c {
        0x9 | 0xA | 0xD | 0x20..=0xD7FF | 0xE000..=0xFFFD | 0x1_0000..=0x10_FFFF => char::from_u32(c),
        _ => None,
    }
}

/// Replace the references in `raw`; with `attr`, the white space characters become spaces (3.3.3).
fn decode(raw: &str, attr: bool, loose: bool) -> Result<String, XmlError> {
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(at) = rest.find('&') {
        out.push_str(rest.get(..at).unwrap_or(""));
        let after = rest.get(at + 1..).unwrap_or("");
        let end = after.find(';').filter(|e| *e <= 12).ok_or(XmlError::Syntax)?;
        let body = after.get(..end).unwrap_or("");
        match body {
            "lt" => out.push('<'),
            "gt" => out.push('>'),
            "amp" => out.push('&'),
            "quot" => out.push('"'),
            "apos" => out.push('\''),
            _ => {
                if let Some(num) = body.strip_prefix('#') {
                    let v = match num.strip_prefix('x') {
                        Some(h) => u32::from_str_radix(h, 16),
                        None => num.parse::<u32>(),
                    };
                    out.push(v.ok().and_then(xml_char).ok_or(XmlError::Syntax)?);
                } else if !loose {
                    return Err(XmlError::Syntax);
                }
            }
        }
        rest = after.get(end + 1..).unwrap_or("");
    }
    out.push_str(rest);
    if attr {
        out = out.replace(['\n', '\t', '\r'], " ");
    }
    Ok(out)
}

/// Parse `bytes` (UTF-8) as an XML document whose root is an `svg` element.
pub fn parse(bytes: &[u8]) -> Result<Document, XmlError> {
    let text = std::str::from_utf8(bytes).map_err(|_| XmlError::Syntax)?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut p = P { s: text, i: 0, doc: Document::new(Interner::new()), decls: Vec::new(), heads: [NIL; BUCKETS], loose: false };
    let mut stack: Vec<(NodeId, usize)> = Vec::new();
    let mut root = NONE;
    let mut pending = String::new();
    while p.i < p.s.len() {
        if p.at() != b'<' {
            let end = p.rest().find('<').map_or(p.s.len(), |e| p.i + e);
            let raw = p.s.get(p.i..end).unwrap_or("");
            if stack.is_empty() {
                if !raw.trim_matches([' ', '\t', '\n', '\r']).is_empty() {
                    return Err(XmlError::Syntax);
                }
            } else {
                pending.push_str(&decode(raw, false, p.loose)?);
            }
            p.i = end;
            continue;
        }
        if p.starts("<![CDATA[") {
            if stack.is_empty() {
                return Err(XmlError::Syntax);
            }
            let start = p.i + 9;
            p.skip_past("]]>")?;
            pending.push_str(p.s.get(start..p.i - 3).unwrap_or(""));
            continue;
        }
        if p.starts("<!--") {
            p.skip_past("-->")?;
            continue;
        }
        if p.starts("<?") {
            p.skip_past("?>")?;
            continue;
        }
        if p.starts("<!") {
            // DOCTYPE: skipped up to its closing `>`, past quoted strings and the internal subset.
            let (mut depth, mut quote) = (0i32, 0u8);
            let start = p.i;
            while p.i < p.s.len() {
                // Comments and processing instructions of the internal subset: their quotes are text, not strings (XML 1.0 2.8).
                if quote == 0 && depth > 0 && p.starts("<!--") {
                    p.skip_past("-->")?;
                    continue;
                }
                if quote == 0 && depth > 0 && p.starts("<?") {
                    p.skip_past("?>")?;
                    continue;
                }
                let c = p.at();
                p.i += 1;
                match (quote, c) {
                    (0, b'"' | b'\'') => quote = c,
                    (q, c) if q != 0 && q == c => quote = 0,
                    (0, b'[') => depth += 1,
                    (0, b']') => depth -= 1,
                    (0, b'>') if depth <= 0 => break,
                    _ => {}
                }
            }
            p.loose |= p.s.get(start..p.i).is_some_and(|d| d.contains("<!ENTITY"));
            continue;
        }
        // A tag. Text before it becomes a node first.
        if !pending.is_empty() {
            if let Some(&(parent, _)) = stack.last() {
                let t = p.doc.alloc(NodeData::Text(std::mem::take(&mut pending)));
                p.doc.append_child(parent, t);
            }
            pending.clear();
        }
        if p.doc.nodes.len() > MAX_NODES {
            return Err(XmlError::TooLarge);
        }
        if p.starts("</") {
            p.i += 2;
            let n = p.name()?.to_string();
            p.ws();
            if p.at() != b'>' {
                return Err(XmlError::Syntax);
            }
            p.i += 1;
            let (node, scope_len) = stack.pop().ok_or(XmlError::Syntax)?;
            let open = p.doc.element(node).map(|e| p.doc.atoms.get(e.name).to_string()).unwrap_or_default();
            // The qualified name was kept only as the local name: compare that.
            let local = n.rsplit(':').next().unwrap_or("");
            if local != open {
                return Err(XmlError::Syntax);
            }
            p.undeclare_to(scope_len);
            continue;
        }
        p.i += 1;
        let qname = p.name()?.to_string();
        let mut raw: Vec<(String, String)> = Vec::new();
        let empty;
        loop {
            p.ws();
            match p.at() {
                b'>' => {
                    p.i += 1;
                    empty = false;
                    break;
                }
                b'/' => {
                    p.i += 1;
                    if p.at() != b'>' {
                        return Err(XmlError::Syntax);
                    }
                    p.i += 1;
                    empty = true;
                    break;
                }
                0 => return Err(XmlError::Syntax),
                _ => {}
            }
            let before = p.i;
            let an = p.name()?.to_string();
            p.ws();
            if p.at() != b'=' {
                return Err(XmlError::Syntax);
            }
            p.i += 1;
            p.ws();
            let q = p.at();
            if q != b'"' && q != b'\'' {
                return Err(XmlError::Syntax);
            }
            p.i += 1;
            let end = p.rest().find(q as char).ok_or(XmlError::Syntax)? + p.i;
            let value = p.s.get(p.i..end).unwrap_or("");
            if value.contains('<') || raw.len() >= MAX_ATTRS || raw.iter().any(|(n, _)| *n == an) || p.i == before {
                return Err(XmlError::Syntax);
            }
            let value = decode(value, true, p.loose)?;
            p.i = end + 1;
            // Attributes must be separated by white space.
            if !matches!(p.at(), b' ' | b'\t' | b'\n' | b'\r' | b'/' | b'>') {
                return Err(XmlError::Syntax);
            }
            raw.push((an, value));
        }
        let scope_len = p.decls.len();
        for (n, v) in &raw {
            if n == "xmlns" {
                p.declare("", v)?;
            } else if let Some(px) = n.strip_prefix("xmlns:") {
                p.declare(px, v)?;
            }
        }
        let (prefix, local) = qname.split_once(':').unwrap_or(("", &qname));
        let ns = match p.lookup(prefix) {
            Some(SVG_NS) => Namespace::Svg,
            Some(XHTML_NS) => Namespace::Html,
            Some(_) if !prefix.is_empty() => Namespace::None,
            _ if prefix.is_empty() => Namespace::Svg,
            None => return Err(XmlError::Syntax),
            _ => Namespace::None,
        };
        let mut attrs = Attrs::default();
        for (n, v) in &raw {
            if n == "xmlns" || n.starts_with("xmlns:") {
                continue;
            }
            let (apx, al) = n.split_once(':').unwrap_or(("", n));
            let ans = if apx.is_empty() {
                Namespace::None
            } else {
                match p.lookup(apx) {
                    Some(XLINK_NS) => Namespace::XLink,
                    Some(XML_NS) => Namespace::Xml,
                    _ if apx == "xml" => Namespace::Xml,
                    Some(_) => Namespace::None,
                    None => return Err(XmlError::Syntax),
                }
            };
            let pa = if apx.is_empty() { a::EMPTY } else { p.doc.atoms.intern(apx) };
            let na = p.doc.atoms.intern(al);
            attrs.push(ans, pa, na, v);
        }
        let name = p.doc.atoms.intern(local);
        if stack.is_empty() && (root != NONE || ns != Namespace::Svg || local != "svg") {
            return Err(if root != NONE { XmlError::Syntax } else { XmlError::NotSvg });
        }
        let stored = p.doc.store_attrs(attrs);
        let id = p.doc.alloc(NodeData::Element(ElementData { name, ns, attrs: stored, template_contents: NONE }));
        let parent = stack.last().map_or(crate::html::dom::DOCUMENT, |s| s.0);
        p.doc.append_child(parent, id);
        if root == NONE {
            root = id;
        }
        if empty {
            p.undeclare_to(scope_len);
        } else {
            if stack.len() >= MAX_DEPTH {
                return Err(XmlError::TooLarge);
            }
            stack.push((id, scope_len));
        }
    }
    if !stack.is_empty() || root == NONE {
        return Err(XmlError::Syntax);
    }
    Ok(p.doc)
}

#[cfg(test)]
#[allow(clippy::indexing_slicing, clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn ok(s: &str) -> Document {
        parse(s.as_bytes()).unwrap()
    }

    #[test]
    fn elements_attributes_text_and_namespaces() {
        let d = ok("<?xml version='1.0'?><!-- c --><svg xmlns='http://www.w3.org/2000/svg' xmlns:x='http://www.w3.org/1999/xlink'>\
                    <use x:href='#a' width=\"1&amp;2\"/><title>a &lt; b &#65;&#x42;<![CDATA[<c>]]></title></svg>");
        let root = d.document_element();
        assert_eq!(d.local_name(root), "svg");
        let u = d.first_child(root);
        let e = d.element(u).unwrap();
        assert_eq!(e.ns, Namespace::Svg);
        let hrefs: Vec<(Namespace, &str)> = e.attrs.iter().map(|a| (a.ns, d.value(a))).collect();
        assert_eq!(hrefs, vec![(Namespace::XLink, "#a"), (Namespace::None, "1&2")]);
        assert_eq!(d.text_content(d.next_sibling(u)), "a < b AB<c>");
    }

    #[test]
    fn malformed_documents_are_errors() {
        for bad in [
            "", "<svg>", "<svg></g>", "<svg><a></svg>", "<svg a=1/>", "<svg a='1' a='2'/>", "<svg>&nope;</svg>", "<svg>&#0;</svg>",
            "<svg/><svg/>", "text<svg/>", "<svg/>text", "<svg a='<'/>", "<x:svg/>", "<svg a='1'b='2'/>", "<svg>&amp</svg>", "<svg",
        ] {
            assert!(parse(bad.as_bytes()).is_err(), "{bad}");
        }
        assert_eq!(parse(b"<html/>").err(), Some(XmlError::NotSvg));
        assert_eq!(parse(&[b'<', b's', b'v', b'g', b'/', b'>', 0xff]).err(), Some(XmlError::Syntax));
    }

    #[test]
    fn doctype_entities_are_never_expanded() {
        let d = ok("<!DOCTYPE svg [ <!ENTITY e 'x<y'> ]><svg xmlns='&ns;'>&e;t</svg>");
        assert_eq!(d.text_content(d.document_element()), "t");
        assert!(parse(b"<!DOCTYPE svg><svg>&e;</svg>").is_err());
    }

    #[test]
    fn doctype_comments_and_instructions_may_hold_quotes() {
        let svg = "<svg xmlns=\"http://www.w3.org/2000/svg\"/>";
        for subset in ["<!-- don't -->", "<!-- \" ] > -->", "<?pi it's ?>", "<!ENTITY a \"<!-- x\"><!-- ' -->", "<!-- a --><!-- b'c -->"] {
            let d = parse(format!("<!DOCTYPE svg [{subset}]>{svg}").as_bytes()).unwrap();
            assert_eq!(d.local_name(d.document_element()), "svg", "{subset}");
        }
        // A comment that never ends is an error, not a swallowed document.
        assert!(parse(format!("<!DOCTYPE svg [<!-- x ]>{svg}").as_bytes()).is_err());
    }

    #[test]
    fn prefixes_follow_the_open_elements() {
        // The inner element rebinds `p`; after it closes the outer binding is back (XML Names 6.1).
        let svg = "<svg xmlns='http://www.w3.org/2000/svg' xmlns:p='http://www.w3.org/1999/xlink'>\
                   <g xmlns:p='urn:other' p:href='1'/><g p:href='2'><g xmlns:p='urn:x'><rect p:href='3'/></g><use p:href='4'/></g>\
                   <g xmlns='urn:none'><x/></g><rect/></svg>";
        let d = ok(svg);
        let mut seen = Vec::new();
        let mut stack = vec![d.document_element()];
        while let Some(n) = stack.pop() {
            for a in d.element(n).unwrap().attrs.iter() {
                seen.push((d.value(a).to_string(), a.ns));
            }
            let mut c = d.first_child(n);
            let mut kids = Vec::new();
            while c != NONE {
                if d.element(c).is_some() {
                    kids.push(c);
                }
                c = d.next_sibling(c);
            }
            stack.extend(kids.into_iter().rev());
        }
        let want = vec![("1".to_string(), Namespace::None), ("2".to_string(), Namespace::XLink), ("3".to_string(), Namespace::None), ("4".to_string(), Namespace::XLink)];
        assert_eq!(seen, want);
        // A prefix is unbound again after its element closes.
        assert_eq!(parse(b"<svg xmlns='http://www.w3.org/2000/svg'><q:r xmlns:q='urn:q'/><q:r/></svg>").err(), Some(XmlError::Syntax));
    }

    #[test]
    fn namespace_declarations_are_bounded_and_lookups_stay_cheap() {
        // 256 open elements x 256 declarations used to make every lookup walk 65k bindings.
        let decls: String = (0..250).map(|k| format!(" xmlns:p{k}='u{k}'")).collect();
        let deep = format!("<svg xmlns='http://www.w3.org/2000/svg'>{}{}</svg>", format!("<g{decls} p0:a='1'>").repeat(255), "</g>".repeat(255));
        let t = std::time::Instant::now();
        assert_eq!(parse(deep.as_bytes()).err(), Some(XmlError::TooLarge));
        // Under the limit, lots of lookups: 60 levels x 16 declarations, each element using three prefixes.
        let few: String = (0..16).map(|k| format!(" xmlns:p{k}='u{k}'")).collect();
        let ok_deep = format!("<svg xmlns='http://www.w3.org/2000/svg'>{}{}</svg>", format!("<g{few} p0:a='1' p15:b='2' p7:c='3'>").repeat(60), "</g>".repeat(60));
        assert!(parse(ok_deep.as_bytes()).is_ok());
        assert!(t.elapsed() < std::time::Duration::from_secs(5), "{:?}", t.elapsed());
    }

    #[test]
    fn deep_and_big_documents_stop() {
        let deep = format!("<svg>{}{}</svg>", "<g>".repeat(300), "</g>".repeat(300));
        assert_eq!(parse(deep.as_bytes()).err(), Some(XmlError::TooLarge));
        let wide = format!("<svg>{}</svg>", "<g/>".repeat(MAX_NODES + 10));
        assert_eq!(parse(wide.as_bytes()).err(), Some(XmlError::TooLarge));
    }
}
