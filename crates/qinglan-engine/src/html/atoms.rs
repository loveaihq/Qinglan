//! Interned names: tag names, attribute names, class names and ids become small integers so that
//! the parser and the selector matcher compare them with one integer comparison.
//!
//! The names the parser itself needs are static atoms (`a::P`, `a::DIV`, ...); the interner is
//! pre-filled with them so their numbers are fixed at compile time.

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};
use std::rc::Rc;

/// An interned string. Equal atoms from the same [`Interner`] mean equal strings.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct Atom(pub u32);

/// FxHash-style hasher: fast and good enough for short names (std has no fast hasher; no crates).
#[derive(Default, Clone, Copy)]
pub struct FxHasher {
    hash: u64,
}

impl Hasher for FxHasher {
    fn write(&mut self, bytes: &[u8]) {
        const K: u64 = 0x517c_c1b7_2722_0a95;
        let (chunks, rest) = bytes.as_chunks::<8>();
        for c in chunks {
            self.hash = (self.hash.rotate_left(5) ^ u64::from_le_bytes(*c)).wrapping_mul(K);
        }
        if !rest.is_empty() {
            let mut v = 0u64;
            for (i, &b) in rest.iter().enumerate() {
                v |= (b as u64) << (8 * i);
            }
            self.hash = (self.hash.rotate_left(5) ^ v).wrapping_mul(K);
        }
    }
    fn write_u8(&mut self, i: u8) {
        self.hash = (self.hash.rotate_left(5) ^ i as u64).wrapping_mul(0x517c_c1b7_2722_0a95);
    }
    fn write_u32(&mut self, i: u32) {
        self.hash = (self.hash.rotate_left(5) ^ i as u64).wrapping_mul(0x517c_c1b7_2722_0a95);
    }
    fn write_u64(&mut self, i: u64) {
        self.hash = (self.hash.rotate_left(5) ^ i).wrapping_mul(0x517c_c1b7_2722_0a95);
    }
    fn write_usize(&mut self, i: usize) {
        self.write_u64(i as u64);
    }
    fn finish(&self) -> u64 {
        self.hash
    }
}

pub type FxBuild = BuildHasherDefault<FxHasher>;
pub type FxHashMap<K, V> = HashMap<K, V, FxBuild>;

/// The string table.
#[derive(Clone)]
pub struct Interner {
    strings: Vec<Rc<str>>,
    map: FxHashMap<Rc<str>, u32>,
}

macro_rules! static_atoms {
    ($($id:ident => $s:expr),* $(,)?) => {
        #[allow(non_camel_case_types, clippy::upper_case_acronyms)]
        #[repr(u32)]
        enum Index { $($id),* }
        /// Static atoms, usable in `match` patterns.
        pub mod a {
            use super::{Atom, Index};
            $(pub const $id: Atom = Atom(Index::$id as u32);)*
        }
        const STATIC_NAMES: &[&str] = &[$($s),*];
    };
}

static_atoms! {
    EMPTY => "",
    // HTML elements the tree builder knows about
    A => "a", ADDRESS => "address", ANNOTATION_XML => "annotation-xml", APPLET => "applet", AREA => "area",
    ARTICLE => "article", ASIDE => "aside", B => "b", BASE => "base", BASEFONT => "basefont", BGSOUND => "bgsound",
    BIG => "big", BLOCKQUOTE => "blockquote", BODY => "body", BR => "br", BUTTON => "button", CAPTION => "caption",
    CENTER => "center", CODE => "code", COL => "col", COLGROUP => "colgroup", DD => "dd", DESC => "desc",
    DETAILS => "details", DIALOG => "dialog", DIR => "dir", DIV => "div", DL => "dl", DT => "dt", EM => "em",
    EMBED => "embed", FIELDSET => "fieldset", FIGCAPTION => "figcaption", FIGURE => "figure", FONT => "font",
    FOOTER => "footer", FOREIGNOBJECT => "foreignObject", FORM => "form", FRAME => "frame", FRAMESET => "frameset",
    H1 => "h1", H2 => "h2", H3 => "h3", H4 => "h4", H5 => "h5", H6 => "h6", HEAD => "head", HEADER => "header",
    HGROUP => "hgroup", HR => "hr", HTML => "html", I => "i", IFRAME => "iframe", IMAGE => "image", IMG => "img",
    INPUT => "input", KEYGEN => "keygen", LEGEND => "legend", LI => "li", LINK => "link", LISTING => "listing", MAIN => "main",
    MALIGNMARK => "malignmark", MARQUEE => "marquee", MATH => "math", MENU => "menu", META => "meta", MGLYPH => "mglyph",
    MI => "mi", MN => "mn", MO => "mo", MS => "ms", MTEXT => "mtext", NAV => "nav", NOBR => "nobr",
    NOEMBED => "noembed", NOFRAMES => "noframes", NOSCRIPT => "noscript", OBJECT => "object", OL => "ol",
    OPTGROUP => "optgroup", OPTION => "option", P => "p", PARAM => "param", PLAINTEXT => "plaintext", PRE => "pre",
    RB => "rb", RP => "rp", RT => "rt", RTC => "rtc", RUBY => "ruby", S => "s", SCRIPT => "script",
    SEARCH => "search", SECTION => "section", SELECT => "select", SMALL => "small", SOURCE => "source", SPAN => "span",
    STRIKE => "strike", STRONG => "strong", STYLE => "style", SUB => "sub", SUMMARY => "summary", SUP => "sup",
    SVG => "svg", TABLE => "table", TBODY => "tbody", TD => "td", TEMPLATE => "template", TEXTAREA => "textarea",
    TFOOT => "tfoot", TH => "th", THEAD => "thead", TITLE => "title", TR => "tr", TRACK => "track", TT => "tt",
    U => "u", UL => "ul", VAR => "var", WBR => "wbr", XMP => "xmp",
    // attribute names
    ACTION => "action", CLASS => "class", COLOR => "color", CONTENT => "content", DEFINITIONURL => "definitionurl",
    ENCODING => "encoding", FACE => "face", HREF => "href", HTTP_EQUIV => "http-equiv", ID => "id",
    LANG => "lang", NAME => "name", PROMPT => "prompt", SIZE => "size", SRC => "src", TYPE => "type",
    XLINK_HREF => "xlink:href", XMLNS => "xmlns", XMLNS_XLINK => "xmlns:xlink",
}

impl Default for Interner {
    fn default() -> Self {
        Self::new()
    }
}

impl Interner {
    pub fn new() -> Interner {
        let mut it = Interner { strings: Vec::with_capacity(1024), map: FxHashMap::default() };
        for &s in STATIC_NAMES {
            let rc: Rc<str> = Rc::from(s);
            let id = it.strings.len() as u32;
            it.strings.push(rc.clone());
            it.map.insert(rc, id);
        }
        it
    }

    pub fn intern(&mut self, s: &str) -> Atom {
        if let Some(&id) = self.map.get(s) {
            return Atom(id);
        }
        let rc: Rc<str> = Rc::from(s);
        let id = self.strings.len() as u32;
        self.strings.push(rc.clone());
        self.map.insert(rc, id);
        Atom(id)
    }

    /// Look up without adding (an atom that was never interned cannot be on any element).
    pub fn lookup(&self, s: &str) -> Option<Atom> {
        self.map.get(s).map(|&id| Atom(id))
    }

    /// The string of an atom (the empty string for an atom from another interner).
    pub fn get(&self, a: Atom) -> &str {
        self.strings.get(a.0 as usize).map(|s| &**s).unwrap_or("")
    }

    pub fn len(&self) -> usize {
        self.strings.len()
    }

    pub fn is_empty(&self) -> bool {
        self.strings.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_atoms_match_their_strings() {
        let mut it = Interner::new();
        assert_eq!(it.intern("div"), a::DIV);
        assert_eq!(it.get(a::FOREIGNOBJECT), "foreignObject");
        assert_eq!(it.get(a::HTTP_EQUIV), "http-equiv");
        let x = it.intern("x-custom");
        assert_eq!(it.intern("x-custom"), x);
        assert_eq!(it.get(x), "x-custom");
    }
}
