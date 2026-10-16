//! Generated content (step 2a-1): the counters of CSS2 12.4 and css-lists-3 4 (`counter-reset`, `counter-increment`,
//! `counter-set`, the `list-item` counter), the quote nesting of CSS2 12.3, and the text a `content` value makes
//! (CSS2 12.2). The box builder walks the document in order and calls into this for every element.
//!
//! Scope (CSS2 12.4.1, css-lists-3 4.4.1): a counter made by an element is visible to the element, its descendants and
//! its following siblings with their descendants, except that a later sibling does not inherit a sibling's counter when
//! the counters of their parent already have one of that name (css-lists-3; the CSS2.1 tests expect the older rule).
//! The stack of counter instances models that: an instance is pushed when its element is met and dropped when the
//! parent of that element is left; an element that resets a counter its earlier sibling made replaces that instance,
//! while one made by an ancestor stays below it (that is what `counters()` shows).

use std::rc::Rc;

use super::lists::representation;
use crate::css::computed::{ComputedStyle, GenStyle};
use crate::css::types::{ContentItem, Quotes};

/// Counter instances alive at one time; further `counter-reset`s of new instances are ignored (hostile pages).
const MAX_INSTANCES: usize = 4096;
/// Instances of one name that `counters()` joins.
const MAX_JOINED: usize = 64;
/// Longest text a `content` value makes in one pseudo-element, and longest `attr()` value taken.
const MAX_TEXT: usize = 8192;
const MAX_ATTR: usize = 2048;
/// Quote nesting is counted up to this (deeper pairs repeat the innermost anyway).
const MAX_QUOTE_DEPTH: u32 = 1 << 16;

struct Inst {
    name: Rc<str>,
    value: i32,
    /// The frame (the children list of the parent element) of the element that made it.
    frame: u32,
    /// The element that made it.
    owner: u32,
}

/// The state of the walk: the counters in scope, the frame the current element is in, the quote depth.
pub(super) struct Counters {
    stack: Vec<Inst>,
    frame: u32,
    next_frame: u32,
    quote_depth: u32,
    /// Where the current frame starts in `stack`, and the element being processed.
    frame_start: usize,
    cur_owner: u32,
    next_owner: u32,
}

/// What [`Counters::enter`] saves for [`Counters::leave`].
pub(super) struct Saved {
    mark: usize,
    frame: u32,
    frame_start: usize,
}

/// One piece of generated content: text, or an image (its `url()`).
pub(super) enum Piece {
    Text(String),
    Image(Rc<str>),
}

impl Counters {
    pub fn new() -> Counters {
        Counters { stack: Vec::new(), frame: 0, next_frame: 1, quote_depth: 0, frame_start: 0, cur_owner: 0, next_owner: 1 }
    }

    /// Start the children of an element: they are in a frame of their own.
    pub fn enter(&mut self) -> Saved {
        let saved = Saved { mark: self.stack.len(), frame: self.frame, frame_start: self.frame_start };
        self.frame_start = self.stack.len();
        self.frame = self.next_frame;
        self.next_frame = self.next_frame.wrapping_add(1);
        saved
    }

    /// The children are done: what they made goes out of scope.
    pub fn leave(&mut self, saved: Saved) {
        self.stack.truncate(saved.mark);
        self.frame = saved.frame;
        self.frame_start = saved.frame_start;
    }

    /// The next element (or pseudo-element) of the current frame is being processed.
    pub fn begin_element(&mut self) {
        self.cur_owner = self.next_owner;
        self.next_owner = self.next_owner.wrapping_add(1);
    }

    /// Can the current element see instance `i`? Those of its ancestors and its own always; one a sibling made, unless
    /// the counters of the parent already have one of that name (css-lists-3 4.4.1).
    fn visible(&self, i: &Inst) -> bool {
        i.frame != self.frame || i.owner == self.cur_owner || !self.stack.get(..self.frame_start).is_some_and(|outer| outer.iter().any(|o| o.name == i.name))
    }

    fn find(&self, name: &str) -> Option<usize> {
        (0..self.stack.len()).rev().find(|&k| self.stack.get(k).is_some_and(|i| &*i.name == name && self.visible(i)))
    }

    /// `counter-reset` (css-lists-3 4.4): a new counter, replacing one an earlier sibling made.
    pub fn reset(&mut self, name: &Rc<str>, value: i32) {
        let frame = self.frame;
        self.stack.retain(|i| !(i.frame == frame && i.name == *name));
        if self.stack.len() < MAX_INSTANCES {
            self.stack.push(Inst { name: name.clone(), value, frame, owner: self.cur_owner });
        }
    }

    fn innermost(&mut self, name: &Rc<str>) -> Option<&mut Inst> {
        let k = self.find(name)?;
        self.stack.get_mut(k)
    }

    /// `counter-increment` / `counter-set` on a counter that does not exist make one on the element first (css-lists-3 4.3).
    fn instance(&mut self, name: &Rc<str>) -> Option<&mut Inst> {
        if self.innermost(name).is_none() {
            self.reset(name, 0);
        }
        self.innermost(name)
    }

    pub fn increment(&mut self, name: &Rc<str>, by: i32) {
        if let Some(i) = self.instance(name) {
            i.value = i.value.saturating_add(by);
        }
    }

    pub fn set(&mut self, name: &Rc<str>, value: i32) {
        if let Some(i) = self.instance(name) {
            i.value = value;
        }
    }

    /// The counter properties of an element, in the order of css-lists-3 4.4: reset, increment, set.
    pub fn apply(&mut self, g: &GenStyle) {
        for (n, v) in g.counter_reset.iter() {
            self.reset(n, *v);
        }
        for (n, v) in g.counter_increment.iter() {
            self.increment(n, *v);
        }
        for (n, v) in g.counter_set.iter() {
            self.set(n, *v);
        }
    }

    /// The value `counter(name)` shows: the innermost counter, or 0 when there is none (CSS2 12.4).
    pub fn value(&self, name: &str) -> i32 {
        self.find(name).and_then(|k| self.stack.get(k)).map_or(0, |i| i.value)
    }

    /// All the values of counters named `name` the element can see, outermost first, for `counters()`.
    fn values(&self, name: &str) -> Vec<i32> {
        let mut v: Vec<i32> = self.stack.iter().rev().filter(|i| &*i.name == name && self.visible(i)).take(MAX_JOINED).map(|i| i.value).collect();
        v.reverse();
        v
    }

    /// `counter()` and `counters()` on a name that has no counter make one with the value 0 on the element (css-lists-3 4.4).
    fn ensure(&mut self, name: &Rc<str>) {
        if self.find(name).is_none() {
            self.reset(name, 0);
        }
    }

    /// The text and the images `items` make for an element with computed style `style` (the pseudo-element's own).
    /// `attr` gives the attribute values of the element the pseudo-element belongs to.
    pub fn content(&mut self, items: &[ContentItem], style: &ComputedStyle, attr: &dyn Fn(&str) -> Option<String>) -> Vec<Piece> {
        let mut out: Vec<Piece> = Vec::new();
        let mut text = String::new();
        fn add(text: &mut String, s: &str) {
            if text.len() < MAX_TEXT {
                text.push_str(s);
            }
        }
        for item in items {
            match item {
                ContentItem::Str(s) => add(&mut text, s),
                ContentItem::Attr(n) => {
                    if let Some(v) = attr(n) {
                        let cut = v.char_indices().nth(MAX_ATTR).map_or(v.len(), |(i, _)| i);
                        add(&mut text, v.get(..cut).unwrap_or(""));
                    }
                }
                ContentItem::Counter(name, ty) => {
                    self.ensure(name);
                    let s = representation(ty, self.value(name));
                    add(&mut text, &s);
                }
                ContentItem::Counters(name, sep, ty) => {
                    self.ensure(name);
                    let parts: Vec<String> = self.values(name).into_iter().map(|v| representation(ty, v)).collect();
                    let s = if parts.is_empty() { representation(ty, 0) } else { parts.join(sep) };
                    add(&mut text, &s);
                }
                ContentItem::OpenQuote | ContentItem::NoOpenQuote => {
                    if matches!(item, ContentItem::OpenQuote)
                        && let Some(q) = quote(&style.text.quotes, style.text.lang.as_deref(), self.quote_depth, true)
                    {
                        add(&mut text, &q);
                    }
                    self.quote_depth = (self.quote_depth + 1).min(MAX_QUOTE_DEPTH);
                }
                ContentItem::CloseQuote | ContentItem::NoCloseQuote => {
                    // At depth 0 a closing quote does nothing (css-content-3 3.2).
                    if self.quote_depth > 0 {
                        self.quote_depth -= 1;
                        if matches!(item, ContentItem::CloseQuote)
                            && let Some(q) = quote(&style.text.quotes, style.text.lang.as_deref(), self.quote_depth, false)
                        {
                            add(&mut text, &q);
                        }
                    }
                }
                ContentItem::Url(u) => {
                    if !text.is_empty() {
                        out.push(Piece::Text(std::mem::take(&mut text)));
                    }
                    out.push(Piece::Image(u.clone()));
                }
            }
        }
        if !text.is_empty() {
            out.push(Piece::Text(text));
        }
        out
    }
}

/// The quotation mark for nesting level `depth` (CSS2 12.3.1): the pair at that index, the last pair for deeper levels.
fn quote(q: &Quotes, lang: Option<&str>, depth: u32, open: bool) -> Option<Rc<str>> {
    let pick = |pairs: &[(Rc<str>, Rc<str>)]| {
        let (o, c) = pairs.get((depth as usize).min(pairs.len().saturating_sub(1)))?;
        Some(if open { o.clone() } else { c.clone() })
    };
    match q {
        Quotes::None => None,
        Quotes::Pairs(p) => pick(p),
        Quotes::Auto => {
            let l = lang.unwrap_or("").to_ascii_lowercase();
            let cjk = l.starts_with("ja") || l.starts_with("zh-tw") || l.starts_with("zh-hk") || l.starts_with("zh-hant");
            let pairs: [(&str, &str); 2] = if cjk {
                [("\u{300C}", "\u{300D}"), ("\u{300E}", "\u{300F}")]
            } else if l.starts_with("fr") {
                [("\u{AB}", "\u{BB}"), ("\u{2039}", "\u{203A}")]
            } else if l.starts_with("de") {
                [("\u{201E}", "\u{201C}"), ("\u{201A}", "\u{2018}")]
            } else {
                [("\u{201C}", "\u{201D}"), ("\u{2018}", "\u{2019}")]
            };
            let (o, c) = pairs.get((depth as usize).min(1))?;
            Some(Rc::from(if open { *o } else { *c }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn name(s: &str) -> Rc<str> {
        Rc::from(s)
    }

    #[test]
    fn nested_counters_and_sibling_replacement() {
        let mut c = Counters::new();
        let x = name("x");
        let top = c.enter();
        c.begin_element();
        c.reset(&x, 0); // element A in the top frame
        c.increment(&x, 1);
        let kids = c.enter(); // A's children
        c.begin_element();
        c.reset(&x, 5); // a nested counter
        assert_eq!(c.values("x"), vec![1, 5]);
        c.leave(kids);
        assert_eq!(c.values("x"), vec![1]);
        c.begin_element();
        c.reset(&x, 7); // a following sibling of A replaces A's counter
        assert_eq!(c.values("x"), vec![7]);
        c.leave(top);
        assert_eq!(c.value("x"), 0);
    }

    #[test]
    fn increment_makes_a_counter() {
        let mut c = Counters::new();
        c.begin_element();
        c.increment(&name("n"), 2);
        c.increment(&name("n"), 2);
        assert_eq!(c.value("n"), 4);
        c.set(&name("n"), -3);
        assert_eq!(c.value("n"), -3);
    }
}
