//! Reading a style sheet as a list of rules without tokenizing what the page cannot use.
//!
//! Real sites ship tens of thousands of rules and a page uses a few hundred (see [`super::facts`]). Tokenizing every
//! selector and declaration block and building their component values only to throw most of them away was the
//! biggest cost of the style phase on GitHub pages. Here a rule is read like this:
//!
//! - the `{}` block of a style rule is only *skimmed* to find where it ends (the tokenizer's own scan with the text
//!   of the tokens not kept, so the end is where tokenizing the whole text would put it) and remembered as a byte
//!   range; whoever keeps the rule tokenizes the range ([`super::stylesheet`]);
//! - the prelude (the selectors) is skimmed first too, looking for a class or an id at the top level of each selector
//!   that no element of the document has ([`super::facts`]); a rule all of whose selectors have one is dropped
//!   right there, and only the others get their prelude tokenized.
//!
//! The rules and preludes are the ones `syntax::parse_rule_list` finds in the component values of the same text,
//! except for the dropped ones that the style sheet would drop anyway (the tests at the end check both).

use super::facts::DocFacts;
use super::media::{MediaEnv, MediaQueryList};
use super::syntax::{Cv, CvBuilder, MAX_NESTING};
use super::tokenizer::{Lexer, Token, normalize};
use std::borrow::Cow;

/// How deep `@media`, `@supports` and `@layer` blocks are read as lists of rules; deeper ones are ignored by the
/// reader of the rules ([`super::stylesheet`], which uses the same number).
pub const MAX_RULE_NESTING: usize = 32;

/// Below this many bytes of style sheet text in all, scanning on several threads costs more than it saves.
const PARALLEL_MIN_BYTES: usize = 256 * 1024;
/// At most this many worker threads for [`scan_sheets`].
const PARALLEL_MAX_THREADS: usize = 4;
/// Stack of a worker: the main thread has 8 MB and the parser may recurse as deep as the nesting limits allow.
const WORKER_STACK: usize = 8 << 20;

/// What follows the prelude of a rule.
pub enum LazyBlock {
    /// No block (an at-rule ending in `;` or at the end of the text).
    None,
    /// The contents of the `{}` block, not tokenized: a byte range of the (normalized) text of the sheet.
    Text(usize, usize),
    /// The block of an `@media`, `@supports` or `@layer`, read as a list of rules.
    Rules(Vec<LazyRule>),
}

pub struct LazyRule {
    /// `Some(name)` for an at-rule (lower-cased), `None` for a qualified rule.
    pub at_name: Option<String>,
    pub prelude: Vec<Cv>,
    pub block: LazyBlock,
}

/// A style sheet read up to the blocks of its style rules.
pub struct ScannedSheet<'a> {
    /// The text with the preprocessing of css-syntax-3 3.3 done; the ranges of [`LazyBlock::Text`] are in it.
    pub text: Cow<'a, str>,
    pub rules: Vec<LazyRule>,
}

/// What a rule can be dropped by while it is read: what the document has (`None`: nothing is dropped).
#[derive(Clone, Copy, Default)]
pub struct Filter<'a> {
    pub env: Option<&'a MediaEnv>,
    pub facts: Option<&'a DocFacts>,
    /// Quirks mode: classes and ids are not compared as written, so rules are not dropped for them.
    pub quirks: bool,
}

/// What the next thing in a list of component values is.
enum Item {
    Value(Cv),
    /// A `{}` block of the list itself (not nested in anything), as a byte range of its contents.
    Block(usize, usize),
}

struct Scanner<'a> {
    /// The text of the whole sheet.
    text: &'a str,
    lx: Lexer<'a>,
    /// Where `lx`'s text starts in the text of the sheet.
    base: usize,
    /// How many `{}` blocks the list is inside of; they count for the nesting limit as they would in the whole sheet.
    depth: usize,
    filter: Filter<'a>,
}

/// Read `text[range]` as a list of rules. `text` is normalized; the byte ranges in the result are in `text`.
pub(super) fn scan_range<'a>(text: &'a str, range: std::ops::Range<usize>, top_level: bool, depth: usize, filter: Filter<'a>) -> Vec<LazyRule> {
    let Some(part) = text.get(range.clone()) else { return Vec::new() };
    let mut sc = Scanner { text, lx: Lexer::new(part), base: range.start, depth, filter };
    sc.rules(top_level)
}

/// Read a normalized style sheet; see [`ScannedSheet`].
pub fn scan_normalized<'a>(text: &'a str, filter: Filter<'a>) -> Vec<LazyRule> {
    scan_range(text, 0..text.len(), true, 0, filter)
}

/// Is `b` a byte of a name (a non-ASCII character is all name bytes)?
fn is_name_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b >= 0x80
}

/// Says whether every selector of a prelude has a class or an id the document lacks, from the tokens at its top
/// level (what is inside parentheses and brackets, `:is(.a)` or `[x=.a]`, is not looked at; a selector with a class
/// that has an escape is not either).
struct Needs<'a> {
    facts: &'a DocFacts,
    /// Every selector so far has a class or an id that does not exist.
    all_missing: bool,
    /// The current selector has one.
    this_missing: bool,
    /// The last token was `.`, ending at this offset of the sheet text.
    dot_end: Option<usize>,
}

impl Needs<'_> {
    fn end_selector(&mut self) {
        self.all_missing &= self.this_missing;
        self.this_missing = false;
    }
}

impl<'a> Scanner<'a> {
    /// The next component value of the list; `lazy_curly`: a `{}` block is skimmed instead of built.
    fn item(&mut self, lazy_curly: bool) -> Option<Item> {
        let t = self.lx.next_token()?;
        if lazy_curly && t == Token::OpenCurly {
            return Some(self.skim_block());
        }
        let mut b = CvBuilder::at_depth(self.depth);
        b.feed(t);
        while b.is_open() {
            match self.lx.next_token() {
                Some(t) => b.feed(t),
                None => break,
            }
        }
        b.finish().into_iter().next().map(Item::Value)
    }

    /// How many more openers the nesting limit allows at the top of a list.
    fn room(&self) -> usize {
        MAX_NESTING.saturating_sub(self.depth)
    }

    /// The `{` is read: skip to the matching `}` (the end of the text closes the block too), following the nesting
    /// rules of `CvBuilder`.
    fn skim_block(&mut self) -> Item {
        let start = self.lx.pos();
        self.lx.skim = true;
        // 0: a function or `(`, 1: `[`, 2: `{`.
        let mut stack: Vec<u8> = vec![2];
        let room = self.room();
        let end = loop {
            let Some(t) = self.lx.next_token() else { break self.lx.pos() };
            match t {
                Token::Function(_) | Token::OpenParen if stack.len() < room => stack.push(0),
                Token::OpenSquare if stack.len() < room => stack.push(1),
                Token::OpenCurly if stack.len() < room => stack.push(2),
                Token::CloseParen if stack.last() == Some(&0) => {
                    stack.pop();
                }
                Token::CloseSquare if stack.last() == Some(&1) => {
                    stack.pop();
                }
                Token::CloseCurly if stack.last() == Some(&2) => {
                    stack.pop();
                }
                _ => {}
            }
            if stack.is_empty() {
                break self.lx.pos().saturating_sub(1);
            }
        };
        self.lx.skim = false;
        Item::Block(self.base + start, self.base + end)
    }

    /// "Consume a list of rules" (css-syntax-3 5.4.1) in the shape of `syntax::parse_rule_list`.
    fn rules(&mut self, top_level: bool) -> Vec<LazyRule> {
        let mut rules = Vec::new();
        loop {
            let start = self.lx.pos();
            // Which kind of rule starts here? (Skimmed: the names are only needed for an at-rule, read again below.)
            self.lx.skim = true;
            let first = self.lx.next_token();
            self.lx.skim = false;
            let Some(first) = first else { break };
            match first {
                Token::Whitespace => {}
                Token::Cdo | Token::Cdc if top_level => {}
                Token::AtKeyword(_) => {
                    self.lx.set_pos(start);
                    if let Some(rule) = self.at_rule() {
                        rules.push(rule);
                    }
                }
                _ => {
                    self.lx.set_pos(start);
                    if let Some(rule) = self.qualified_rule(start, first == Token::OpenCurly) {
                        rules.push(rule);
                    }
                }
            }
        }
        rules
    }

    fn at_rule(&mut self) -> Option<LazyRule> {
        let Some(Item::Value(Cv::Tok(Token::AtKeyword(name)))) = self.item(false) else { return None };
        let mut prelude = Vec::new();
        let mut block = None;
        while let Some(next) = self.item(true) {
            match next {
                Item::Value(Cv::Tok(Token::Semicolon)) => break,
                Item::Block(a, b) => {
                    block = Some((a, b));
                    break;
                }
                Item::Value(v) => prelude.push(v),
            }
        }
        let name = name.to_ascii_lowercase();
        let block = self.at_rule_block(&name, &prelude, block);
        Some(LazyRule { at_name: Some(name), prelude, block })
    }

    /// A qualified rule: the prelude is everything up to the first `{}` block; no block means the rule is dropped.
    /// The lexer is at the start of the rule (`start`, an offset in its text).
    fn qualified_rule(&mut self, start: usize, starts_with_block: bool) -> Option<LazyRule> {
        if starts_with_block {
            // A rule that starts with a block: the block is a part of the prelude (as in `parse_rule_list`).
            let Some(Item::Value(first)) = self.item(false) else { return None };
            let mut prelude = vec![first];
            let mut block = None;
            while let Some(next) = self.item(true) {
                match next {
                    Item::Block(a, b) => {
                        block = Some((a, b));
                        break;
                    }
                    Item::Value(v) => prelude.push(v),
                }
            }
            return block.map(|(a, b)| LazyRule { at_name: None, prelude, block: LazyBlock::Text(a, b) });
        }
        // Skim the prelude up to its `{`, noting what it needs.
        let mut needs = self.filter.facts.filter(|_| !self.filter.quirks).map(|facts| Needs { facts, all_missing: true, this_missing: false, dot_end: None });
        let room = self.room();
        let mut stack: Vec<u8> = Vec::new();
        self.lx.skim = true;
        let prelude_end = loop {
            let Some(t) = self.lx.next_token() else {
                // The end of the text: no block, no rule.
                self.lx.skim = false;
                return None;
            };
            let end = self.lx.pos();
            if stack.is_empty() {
                match &t {
                    Token::OpenCurly => break end - 1,
                    Token::Comma => {
                        if let Some(n) = needs.as_mut() {
                            n.end_selector();
                            n.dot_end = None;
                        }
                    }
                    _ => {
                        if let Some(n) = needs.as_mut() {
                            self.note(&t, end, n);
                        }
                    }
                }
            }
            match t {
                Token::Function(_) | Token::OpenParen if stack.len() < room => stack.push(0),
                Token::OpenSquare if stack.len() < room => stack.push(1),
                Token::OpenCurly if stack.len() < room => stack.push(2),
                Token::CloseParen if stack.last() == Some(&0) => {
                    stack.pop();
                }
                Token::CloseSquare if stack.last() == Some(&1) => {
                    stack.pop();
                }
                Token::CloseCurly if stack.last() == Some(&2) => {
                    stack.pop();
                }
                _ => {}
            }
        };
        let Item::Block(a, b) = self.skim_block() else { return None };
        if let Some(mut n) = needs {
            n.end_selector();
            if n.all_missing {
                return None;
            }
        }
        // Kept: the prelude is tokenized now.
        let mut prelude = Vec::new();
        let mut lx = Lexer::new(self.text.get(self.base + start..self.base + prelude_end)?);
        while let Some(t) = lx.next_token() {
            let mut b = CvBuilder::at_depth(self.depth);
            b.feed(t);
            while b.is_open() {
                match lx.next_token() {
                    Some(t) => b.feed(t),
                    None => break,
                }
            }
            prelude.extend(b.finish());
        }
        Some(LazyRule { at_name: None, prelude, block: LazyBlock::Text(a, b) })
    }

    /// A token at the top level of a prelude, ending at offset `end` of the lexer's text.
    fn note(&self, t: &Token, end: usize, n: &mut Needs<'_>) {
        let bytes = self.text.as_bytes();
        let abs_end = self.base + end;
        let dot_end = n.dot_end.take();
        match t {
            Token::Delim('.') => n.dot_end = Some(abs_end),
            // `.name`: the name follows the dot at once and has no escape.
            Token::Ident(_) => {
                if let Some(from) = dot_end
                    && bytes.get(from).is_some_and(|&b| is_name_byte(b))
                    && let Some(name) = self.text.get(from..abs_end).filter(|n| !n.contains('\\'))
                    && n.facts.lacks_class(name)
                {
                    n.this_missing = true;
                }
            }
            // `#name`: the hash starts where this token started, which is `end` minus its length.
            Token::Hash { is_id: true, .. } => {
                let mut from = abs_end;
                while from > 0 && bytes.get(from - 1).is_some_and(|&b| is_name_byte(b)) {
                    from -= 1;
                }
                if from > 0
                    && bytes.get(from - 1) == Some(&b'#')
                    && let Some(name) = self.text.get(from..abs_end)
                    && n.facts.lacks_id(name)
                {
                    n.this_missing = true;
                }
            }
            _ => {}
        }
    }

    /// The blocks of the at-rules that hold rules are read as rules now (on the thread of the scan, not the one that
    /// builds the style sheet); a `@media` that does not apply is not read.
    fn at_rule_block(&self, name: &str, prelude: &[Cv], block: Option<(usize, usize)>) -> LazyBlock {
        let Some((a, b)) = block else { return LazyBlock::None };
        let holds_rules = match name {
            "media" => self.filter.env.is_none_or(|env| MediaQueryList::parse(prelude).matches(env)),
            "supports" | "layer" => true,
            _ => false,
        };
        if holds_rules && self.depth < MAX_RULE_NESTING {
            LazyBlock::Rules(scan_range(self.text, a..b, false, self.depth + 1, self.filter))
        } else {
            LazyBlock::Text(a, b)
        }
    }
}

/// [`scan_normalized`] for several texts, in order. Large sets of sheets are scanned on up to four threads (the
/// biggest first): the texts are independent.
pub fn scan_sheets<'a>(texts: &[&'a str], filter: Filter<'_>) -> Vec<ScannedSheet<'a>> {
    let one = |t: &'a str| {
        let text = normalize(t);
        let rules = scan_normalized(&text, filter);
        ScannedSheet { text, rules }
    };
    let total: usize = texts.iter().map(|t| t.len()).sum();
    let threads = std::thread::available_parallelism().map_or(1, usize::from).min(PARALLEL_MAX_THREADS).min(texts.len());
    if threads < 2 || total < PARALLEL_MIN_BYTES {
        return texts.iter().map(|t| one(t)).collect();
    }
    let mut order: Vec<usize> = (0..texts.len()).collect();
    crate::sorting::sort_by_u64_key(&mut order, |&i| !(texts.get(i).map_or(0, |t| t.len()) as u64));
    let next = std::sync::atomic::AtomicUsize::new(0);
    let slots: Vec<std::sync::Mutex<Option<ScannedSheet<'a>>>> = texts.iter().map(|_| std::sync::Mutex::new(None)).collect();
    let work = || {
        while let Some(&i) = order.get(next.fetch_add(1, std::sync::atomic::Ordering::Relaxed)) {
            let (Some(&text), Some(slot)) = (texts.get(i), slots.get(i)) else { break };
            let scanned = one(text);
            *slot.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = Some(scanned);
        }
    };
    std::thread::scope(|s| {
        for _ in 1..threads {
            // A worker that cannot start just leaves its share to the others (and to this thread).
            let _ = std::thread::Builder::new().stack_size(WORKER_STACK).spawn_scoped(s, work);
        }
        work();
    });
    // A text whose worker panicked is scanned again here: the panic then happens on the caller's thread, as without threads.
    slots.into_iter().zip(texts).map(|(m, &t)| m.into_inner().unwrap_or_else(std::sync::PoisonError::into_inner).unwrap_or_else(|| one(t))).collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::css::selector::{ParseCtx, parse_selector_list};
    use crate::css::syntax::{RawRule, component_values_at, parse_cvs, parse_rule_list, serialize_cvs};
    use crate::css::tokenizer::tokenize_normalized;
    use crate::html::treebuilder::parse_document;

    /// Both readings agree: same rules, same preludes, same block contents (a lazy block is tokenized here).
    fn same(old: &[RawRule], new: &[LazyRule], text: &str, depth: usize) {
        assert_eq!(old.len(), new.len(), "number of rules in {text:?}");
        for (o, n) in old.iter().zip(new) {
            assert_eq!(o.at_name, n.at_name);
            assert_eq!(serialize_cvs(&o.prelude), serialize_cvs(&n.prelude));
            match (&o.block, &n.block) {
                (None, LazyBlock::None) => {}
                (Some(items), LazyBlock::Text(a, b)) => {
                    let again = component_values_at(tokenize_normalized(&text[*a..*b]), depth + 1);
                    assert_eq!(serialize_cvs(items), serialize_cvs(&again), "block in {text:?}")
                }
                (Some(items), LazyBlock::Rules(inner)) => same(&parse_rule_list(items.clone(), false), inner, text, depth + 1),
                _ => panic!("block kinds differ in {text:?}"),
            }
        }
    }

    fn check(text: &str) {
        let text = normalize(text);
        let old = parse_rule_list(parse_cvs(&text), true);
        let new = scan_normalized(&text, Filter::default());
        same(&old, &new, &text, 0);
    }

    #[test]
    fn lazy_scan_finds_what_the_component_value_list_has() {
        for text in [
            "a{color:red} @media screen{b{c:d} e{f:g}} @import 'x'; e f",
            "a { color: red",
            "@media (min-width: 5px) { a { b: c } @media print { d { e: f } } } g { h: i }",
            "@layer a, b; @layer c { .x { y: z } } @supports (display: grid) { .g { display: grid } } @font-face { src: url(x.woff) }",
            // Brackets in strings, comments, urls and escapes do not count; unbalanced ones inside a block do.
            "a { b: '}' ; c: url(}) ; d: url( ) ) ; e: \\} ; /* } */ f: g } h { i: j }",
            "a { b: ( } c { d: e } f { g: h } ) x { y: z }",
            "a { b: [ } ] } c { d: e }",
            "a { b: calc( 1px + ( 2px } ) } c { d: e }",
            "a[b='}'] .c\\{ , d:not(.e) { f: g } h:is(.i, .j) > k { l: m }",
            "a:is({ } ) { b: c } d:not([ { ] ) { e: f }",
            "/* c */ a /* { */ { b: c } /* d */ /**/ e { f: g } h /* } */",
            "{ a: b } c { d: e }",
            "} ) ] a { b: c }",
            "<!-- a { b: c } --> d { e: f }",
            "a { b: c } @media screen { <!-- d { e: f } --> } @charset 'x';",
            "@media screen { a { b: c }",
            "@media screen { a { b: c } } } d { e: f }",
            "a b c",
            "@x { y { z: w } } @y; @z",
            "a\r\n{\rb:c;\x0cd:e\0}",
            "a { b: url(\\75rl(x) ; c: d } e { f: g }",
            "日本 .类 { 色: 赤 } .é\\{ x { y: z }",
        ] {
            check(text);
        }
        // Unbalanced and deeply nested input.
        check(&format!("a {{ b: {} c: d }} e {{ f: g }}", "( ".repeat(600)));
        check(&format!("a {{ {} }} e {{ f: g }}", "{ ".repeat(40)));
        check(&format!("@media screen {{ @layer x {{ a {{ b: {} c: d }} e {{ f: g }} }} }}", "[ ".repeat(515)));
        check(&format!("@media screen {{ @layer x {{ a {} {{ b: c }} e {{ f: g }} }} }}", "( ".repeat(520)));
    }

    #[test]
    fn media_blocks_that_do_not_apply_are_not_read() {
        let text = "@media (min-width: 5000px) { a { b: c } } @media (min-width: 100px) { a { b: c } }";
        let env = MediaEnv::screen(1280.0, 800.0);
        let rules = scan_normalized(text, Filter { env: Some(&env), ..Filter::default() });
        assert!(matches!(rules[0].block, LazyBlock::Text(..)));
        assert!(matches!(rules[1].block, LazyBlock::Rules(_)));
    }

    #[test]
    fn rules_with_a_class_or_an_id_the_document_lacks_are_dropped_while_reading() {
        let mut doc = parse_document("<body class='a b\\c'><div id=x class='k'>t</div>".to_string());
        let facts = DocFacts::collect(&doc);
        let sheet = ".a{x:y} .zz{x:y} #x{x:y} #nope{x:y} div.k{x:y} div.nope .a{x:y} .a .nope{x:y} .zz,.a{x:y} .zz,.yy{x:y} .zz, {x:y} \
            :is(.zz){x:y} [class=zz]{x:y} .a:not(.zz){x:y} .\\7a z{x:y} .a\\31 b{x:y} /**/.zz{x:y} ./**/zz{x:y} . zz{x:y} #1a{x:y} .zz:hover::before{x:y} \
            @media screen { .zz{x:y} .a{x:y} } @layer l { #nope{x:y} .b{x:y} } .b\\c{x:y} .b\\c .zz{x:y}";
        let text = normalize(sheet);
        let all = scan_normalized(&text, Filter::default());
        let some = scan_normalized(&text, Filter { facts: Some(&facts), ..Filter::default() });
        let prelude_of = |r: &LazyRule| serialize_cvs(&r.prelude);
        fn flat(rules: &[LazyRule], out: &mut Vec<String>) {
            for r in rules {
                if r.at_name.is_none() {
                    out.push(serialize_cvs(&r.prelude));
                }
                if let LazyBlock::Rules(inner) = &r.block {
                    flat(inner, out);
                }
            }
        }
        let (mut all_names, mut some_names) = (Vec::new(), Vec::new());
        flat(&all, &mut all_names);
        flat(&some, &mut some_names);
        let _ = prelude_of;
        let dropped: Vec<&String> = all_names.iter().filter(|n| !some_names.contains(n)).collect();
        // Everything it dropped is something the full test drops too.
        let mut atoms = std::mem::take(&mut doc.atoms);
        for name in &dropped {
            let mut pc = ParseCtx { atoms: &mut atoms, quirks: false, depth: 0 };
            let sels = parse_selector_list(&parse_cvs(name), &mut pc, false, true);
            assert!(sels.is_none_or(|s| s.iter().all(|s| !facts.possible(s, &atoms, false))), "{name} could match");
        }
        let kept: Vec<String> = some_names.iter().map(|s| s.replace(' ', "")).collect();
        // Kept: what may match, what has an escape or a comment in the way (not judged), what is not a class or an id.
        for must in [".a", "#x", "div.k", ".zz,.a", ".zz,", ":is(.zz)", "[class=zz]", ".a:not(.zz)", "#1a"] {
            assert!(kept.iter().any(|k| k == must), "{must} was dropped: {kept:?}");
        }
        assert_eq!(kept.iter().filter(|k| *k == ".zz").count(), 3, "with an escape, with a comment, and `. zz`: {kept:?}");
        for gone in ["#nope", "div.nope.a", ".a.nope", ".zz,.yy", ".zz:hover::before"] {
            assert!(!kept.iter().any(|k| k == gone), "{gone} was kept: {kept:?}");
        }
        // Quirks mode compares classes and ids without case: nothing is dropped.
        let quirky = scan_normalized(&text, Filter { facts: Some(&facts), quirks: true, ..Filter::default() });
        let mut quirky_names = Vec::new();
        flat(&quirky, &mut quirky_names);
        assert_eq!(quirky_names, all_names);
    }

    #[test]
    fn scanning_several_sheets_on_threads_gives_the_same_rules_in_the_same_order() {
        let texts: Vec<String> = (0..9).map(|i| format!("a{i}{{color:red;margin:{i}px}}\n").repeat(3000 + i * 500)).collect();
        let slices: Vec<&str> = texts.iter().map(String::as_str).collect();
        assert!(slices.iter().map(|t| t.len()).sum::<usize>() > PARALLEL_MIN_BYTES, "big enough to use threads");
        let many = scan_sheets(&slices, Filter::default());
        assert_eq!(many.len(), 9);
        for (m, t) in many.iter().zip(&slices) {
            same(&parse_rule_list(parse_cvs(t), true), &m.rules, &m.text, 0);
        }
    }
}
