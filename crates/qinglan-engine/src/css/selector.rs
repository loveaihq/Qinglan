//! Selectors: parsing (selectors-4) and matching.
//!
//! Supported: type, universal, class, id, attribute (7 operators, `i`/`s` flags), the four combinators,
//! structural pseudo-classes, `:not()`, `:is()`, `:where()`, `:lang()`, `:link`, `:checked`, `:disabled`,
//! `:enabled`, `:dir()`. `:visited`, `:hover`, `:active`, `:focus` and similar user-state classes parse
//! but never match. `:has()` and unknown pseudo-classes make a selector invalid. Pseudo-elements parse
//! (the rule is valid) but never match an element.

use super::syntax::{BlockKind, Cv, split_commas, trim_ws};
use super::tokenizer::Token;
use crate::html::atoms::{Atom, FxHashMap, Interner};
use crate::html::dom::{Document, NONE, NodeData, NodeId, Namespace};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Combinator {
    Descendant,
    Child,
    NextSibling,
    SubsequentSibling,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AttrOp {
    Exists,
    Equals,
    Includes,
    DashMatch,
    Prefix,
    Suffix,
    Substring,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CaseSens {
    /// Default: sensitive, except for the HTML attributes with case-insensitive values.
    Default,
    Insensitive,
    Sensitive,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AttrSel {
    pub name: Atom,
    /// Lower-cased name, used for elements in the HTML namespace.
    pub lower: Atom,
    pub op: AttrOp,
    pub value: String,
    pub case: CaseSens,
}

/// An+B.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Nth {
    pub a: i32,
    pub b: i32,
}

impl Nth {
    pub fn matches(self, index: i64) -> bool {
        // index = a*n + b for some n >= 0
        let (a, b) = (self.a as i64, self.b as i64);
        if a == 0 {
            return index == b;
        }
        let diff = index - b;
        diff % a == 0 && diff / a >= 0
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PseudoElement {
    Before,
    After,
    Marker,
    FirstLine,
    FirstLetter,
    Selection,
    Placeholder,
    Backdrop,
    FileSelectorButton,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PseudoClass {
    Root,
    Empty,
    FirstChild,
    LastChild,
    OnlyChild,
    FirstOfType,
    LastOfType,
    OnlyOfType,
    NthChild(Nth, Option<Vec<Selector>>),
    NthLastChild(Nth, Option<Vec<Selector>>),
    NthOfType(Nth),
    NthLastOfType(Nth),
    Not(Vec<Selector>),
    Is(Vec<Selector>),
    Where(Vec<Selector>),
    Link,
    Checked,
    Disabled,
    Enabled,
    Lang(Vec<String>),
    Dir(bool), // true = rtl
    Defined,
    /// Valid but never matches in a static, non-interactive renderer.
    Never,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Simple {
    Type { name: Atom, lower: Atom },
    Universal,
    Id(Atom),
    Class(Atom),
    Attr(AttrSel),
    Pseudo(PseudoClass),
    /// `|name` / `ns|name` with a namespace that elements of an HTML document never have.
    NeverMatches,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Compound {
    pub simples: Vec<Simple>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Selector {
    pub compounds: Vec<Compound>,
    /// `combinators[i]` joins `compounds[i]` and `compounds[i + 1]`.
    pub combinators: Vec<Combinator>,
    pub specificity: u32,
    pub pseudo_element: Option<PseudoElement>,
    /// Bloom filter keys of the ancestor compounds (see [`Bloom`]).
    pub ancestor_keys: Vec<u32>,
    /// For `X Y` with `X` a single id, class or type selector: the key `X` needs among the ancestors.
    pub fast_key: Option<u32>,
}

pub fn pack_specificity(a: u32, b: u32, c: u32) -> u32 {
    (a.min(255) << 16) | (b.min(255) << 8) | c.min(255)
}

fn unpack(s: u32) -> (u32, u32, u32) {
    (s >> 16, (s >> 8) & 255, s & 255)
}

fn add_spec(x: u32, y: u32) -> u32 {
    let (a1, b1, c1) = unpack(x);
    let (a2, b2, c2) = unpack(y);
    pack_specificity(a1 + a2, b1 + b2, c1 + c2)
}

// -----------------------------------------------------------------------------------------------
// Parsing

pub struct ParseCtx<'a> {
    pub atoms: &'a mut Interner,
    /// Quirks mode: class and id names match ASCII case-insensitively.
    pub quirks: bool,
    /// Selector lists currently open: 0 at the start, 1 while the top-level list is parsed.
    pub depth: usize,
}

/// Maximum nesting of selector lists inside `:is()`, `:where()`, `:not()` and `:nth-child(... of S)`; a
/// selector that nests deeper is invalid. Keeps parsing and matching recursion shallow (docs/decisions.md).
pub const MAX_SELECTOR_NESTING: usize = 32;
/// Maximum compounds of a complex selector inside a functional pseudo-class. Matching recurses once per
/// compound, so this bounds the stack use of nested lists (top-level selectors are limited in `stylesheet`).
const MAX_NESTED_COMPOUNDS: usize = 16;

/// Parse a selector list. A non-forgiving list is invalid (`None`) if any selector is; a forgiving
/// list (`:is()`, `:where()`) drops invalid selectors. A list nested deeper than [`MAX_SELECTOR_NESTING`]
/// is invalid.
pub fn parse_selector_list(cvs: &[Cv], ctx: &mut ParseCtx<'_>, forgiving: bool, allow_pseudo_element: bool) -> Option<Vec<Selector>> {
    if ctx.depth > MAX_SELECTOR_NESTING {
        return None;
    }
    ctx.depth += 1;
    let r = parse_selector_list_inner(cvs, ctx, forgiving, allow_pseudo_element);
    ctx.depth -= 1;
    r
}

fn parse_selector_list_inner(cvs: &[Cv], ctx: &mut ParseCtx<'_>, forgiving: bool, allow_pseudo_element: bool) -> Option<Vec<Selector>> {
    let mut out = Vec::new();
    let trimmed = trim_ws(cvs);
    if trimmed.is_empty() {
        return if forgiving { Some(out) } else { None };
    }
    for part in split_commas(trimmed) {
        match parse_complex(part, ctx, allow_pseudo_element) {
            Some(s) => out.push(s),
            None if forgiving => {}
            None => return None,
        }
    }
    Some(out)
}

fn parse_complex(cvs: &[Cv], ctx: &mut ParseCtx<'_>, allow_pseudo_element: bool) -> Option<Selector> {
    let cvs = trim_ws(cvs);
    if cvs.is_empty() {
        return None;
    }
    let mut compounds: Vec<Compound> = Vec::new();
    let mut combinators: Vec<Combinator> = Vec::new();
    let mut pseudo_element = None;
    let mut spec = 0u32;
    let mut i = 0usize;
    loop {
        if pseudo_element.is_some() {
            // Nothing may follow a pseudo-element's compound.
            return None;
        }
        let (compound, next, pe, cspec) = parse_compound(cvs, i, ctx, allow_pseudo_element)?;
        compounds.push(compound);
        pseudo_element = pe;
        spec = add_spec(spec, cspec);
        let mut j = next;
        let mut had_ws = false;
        while cvs.get(j).is_some_and(Cv::is_ws) {
            had_ws = true;
            j += 1;
        }
        if j >= cvs.len() {
            break;
        }
        let comb = match cvs.get(j) {
            Some(c) if c.is_delim('>') => Some(Combinator::Child),
            Some(c) if c.is_delim('+') => Some(Combinator::NextSibling),
            Some(c) if c.is_delim('~') => Some(Combinator::SubsequentSibling),
            _ => None,
        };
        match comb {
            Some(c) => {
                j += 1;
                while cvs.get(j).is_some_and(Cv::is_ws) {
                    j += 1;
                }
                if j >= cvs.len() {
                    return None;
                }
                combinators.push(c);
            }
            None if had_ws => combinators.push(Combinator::Descendant),
            None => return None,
        }
        i = j;
    }
    if ctx.depth > 1 && compounds.len() > MAX_NESTED_COMPOUNDS {
        return None;
    }
    let ancestor_keys = ancestor_keys(&compounds, &combinators);
    let fast_key = match (compounds.as_slice(), combinators.as_slice()) {
        ([left, _], [Combinator::Descendant]) => match left.simples.as_slice() {
            [Simple::Id(a)] => Some(bloom_key(KIND_ID, *a)),
            [Simple::Class(a)] => Some(bloom_key(KIND_CLASS, *a)),
            [Simple::Type { name, lower }] if name == lower => Some(bloom_key(KIND_TAG, *lower)),
            _ => None,
        },
        _ => None,
    };
    Some(Selector { compounds, combinators, specificity: spec, pseudo_element, ancestor_keys, fast_key })
}

/// Parse one compound selector starting at `start`. Returns (compound, next index, pseudo-element, specificity).
fn parse_compound(
    cvs: &[Cv],
    start: usize,
    ctx: &mut ParseCtx<'_>,
    allow_pe: bool,
) -> Option<(Compound, usize, Option<PseudoElement>, u32)> {
    let mut i = start;
    let mut simples: Vec<Simple> = Vec::new();
    let mut spec = 0u32;
    let mut pseudo_element = None;
    // Optional type / universal selector with optional namespace prefix.
    let is_tq = |c: Option<&Cv>| matches!(c, Some(Cv::Tok(Token::Ident(_)))) || c.is_some_and(|c| c.is_delim('*'));
    let mut never = false;
    if is_tq(cvs.get(i)) || cvs.get(i).is_some_and(|c| c.is_delim('|')) {
        // Namespace prefix?
        if cvs.get(i).is_some_and(|c| c.is_delim('|')) && is_tq(cvs.get(i + 1)) {
            // `|name`: elements without a namespace: HTML elements are not in that set.
            never = true;
            i += 1;
        } else if is_tq(cvs.get(i)) && cvs.get(i + 1).is_some_and(|c| c.is_delim('|')) && is_tq(cvs.get(i + 2)) {
            // `*|name` is fine; `prefix|name` has an undeclared prefix (no @namespace support): invalid.
            if cvs.get(i).is_some_and(|c| c.is_delim('*')) {
                i += 2;
            } else {
                return None;
            }
        }
        match cvs.get(i) {
            Some(Cv::Tok(Token::Ident(name))) => {
                let lower = ctx.atoms.intern(&name.to_ascii_lowercase());
                let n = ctx.atoms.intern(name);
                simples.push(Simple::Type { name: n, lower });
                spec = add_spec(spec, pack_specificity(0, 0, 1));
                i += 1;
            }
            Some(c) if c.is_delim('*') => {
                simples.push(Simple::Universal);
                i += 1;
            }
            _ => return None,
        }
        if never {
            simples.push(Simple::NeverMatches);
        }
    }
    while let Some(cv) = cvs.get(i) {
        match cv {
            Cv::Tok(Token::Hash { value, is_id }) => {
                if !*is_id {
                    return None;
                }
                let v = if ctx.quirks { value.to_ascii_lowercase() } else { value.clone() };
                simples.push(Simple::Id(ctx.atoms.intern(&v)));
                spec = add_spec(spec, pack_specificity(1, 0, 0));
                i += 1;
            }
            c if c.is_delim('.') => {
                let Some(Cv::Tok(Token::Ident(name))) = cvs.get(i + 1) else { return None };
                let v = if ctx.quirks { name.to_ascii_lowercase() } else { name.clone() };
                simples.push(Simple::Class(ctx.atoms.intern(&v)));
                spec = add_spec(spec, pack_specificity(0, 1, 0));
                i += 1 + 1;
            }
            Cv::Block(BlockKind::Square, inner) => {
                simples.push(Simple::Attr(parse_attr(inner, ctx)?));
                spec = add_spec(spec, pack_specificity(0, 1, 0));
                i += 1;
            }
            Cv::Tok(Token::Colon) => {
                // Pseudo-element (`::x`) or pseudo-class (`:x`).
                if matches!(cvs.get(i + 1), Some(Cv::Tok(Token::Colon))) {
                    let Some(Cv::Tok(Token::Ident(name))) = cvs.get(i + 2) else { return None };
                    let pe = pseudo_element_by_name(name)?;
                    if !allow_pe || pseudo_element.is_some() {
                        return None;
                    }
                    pseudo_element = Some(pe);
                    spec = add_spec(spec, pack_specificity(0, 0, 1));
                    i += 3;
                    continue;
                }
                match cvs.get(i + 1) {
                    Some(Cv::Tok(Token::Ident(name))) => {
                        let lname = name.to_ascii_lowercase();
                        // Legacy single-colon pseudo-elements.
                        if matches!(lname.as_str(), "before" | "after" | "first-line" | "first-letter") {
                            let pe = pseudo_element_by_name(&lname)?;
                            if !allow_pe || pseudo_element.is_some() {
                                return None;
                            }
                            pseudo_element = Some(pe);
                            spec = add_spec(spec, pack_specificity(0, 0, 1));
                        } else {
                            let pc = simple_pseudo_class(&lname)?;
                            simples.push(Simple::Pseudo(pc));
                            spec = add_spec(spec, pack_specificity(0, 1, 0));
                        }
                        i += 2;
                    }
                    Some(Cv::Func(name, args)) => {
                        let (pc, s) = functional_pseudo_class(&name.to_ascii_lowercase(), args, ctx)?;
                        simples.push(Simple::Pseudo(pc));
                        spec = add_spec(spec, s);
                        i += 2;
                    }
                    _ => return None,
                }
            }
            _ => break,
        }
    }
    if simples.is_empty() && pseudo_element.is_none() {
        return None;
    }
    // Only user-action pseudo-classes may follow a pseudo-element; we simply forbid anything else.
    Some((Compound { simples }, i, pseudo_element, spec))
}

fn pseudo_element_by_name(name: &str) -> Option<PseudoElement> {
    Some(match name.to_ascii_lowercase().as_str() {
        "before" => PseudoElement::Before,
        "after" => PseudoElement::After,
        "marker" => PseudoElement::Marker,
        "first-line" => PseudoElement::FirstLine,
        "first-letter" => PseudoElement::FirstLetter,
        "selection" => PseudoElement::Selection,
        "placeholder" => PseudoElement::Placeholder,
        "backdrop" => PseudoElement::Backdrop,
        "file-selector-button" => PseudoElement::FileSelectorButton,
        _ => return None,
    })
}

fn simple_pseudo_class(name: &str) -> Option<PseudoClass> {
    Some(match name {
        "root" | "scope" => PseudoClass::Root,
        "empty" => PseudoClass::Empty,
        "first-child" => PseudoClass::FirstChild,
        "last-child" => PseudoClass::LastChild,
        "only-child" => PseudoClass::OnlyChild,
        "first-of-type" => PseudoClass::FirstOfType,
        "last-of-type" => PseudoClass::LastOfType,
        "only-of-type" => PseudoClass::OnlyOfType,
        "link" | "any-link" => PseudoClass::Link,
        "checked" => PseudoClass::Checked,
        "disabled" => PseudoClass::Disabled,
        "enabled" => PseudoClass::Enabled,
        "defined" => PseudoClass::Defined,
        // User-state and form-state classes a static renderer never matches.
        "hover" | "active" | "focus" | "focus-visible" | "focus-within" | "visited" | "target" | "target-within"
        | "local-link" | "current" | "past" | "future" | "fullscreen" | "modal" | "popover-open" | "paused"
        | "playing" | "autofill" | "-webkit-autofill" | "user-valid" | "user-invalid" | "host" | "in-range"
        | "out-of-range" | "valid" | "invalid" | "indeterminate" | "placeholder-shown" | "read-write" | "read-only"
        | "default" | "required" | "optional" | "blank" | "open" | "closed" | "picture-in-picture" | "seeking"
        | "buffering" | "stalled" | "muted" | "volume-locked" | "popover-open-" | "target-current" => PseudoClass::Never,
        _ => return None,
    })
}

fn functional_pseudo_class(name: &str, args: &[Cv], ctx: &mut ParseCtx<'_>) -> Option<(PseudoClass, u32)> {
    match name {
        "not" => {
            let list = parse_selector_list(args, ctx, false, false)?;
            let s = max_spec(&list);
            Some((PseudoClass::Not(list), s))
        }
        "is" | "matches" | "-webkit-any" | "-moz-any" => {
            let list = parse_selector_list(args, ctx, true, false)?;
            let s = max_spec(&list);
            Some((PseudoClass::Is(list), s))
        }
        "where" => {
            let list = parse_selector_list(args, ctx, true, false)?;
            Some((PseudoClass::Where(list), 0))
        }
        "nth-child" | "nth-last-child" => {
            // An+B [of <selector-list>]
            let args = trim_ws(args);
            let of_pos = args.iter().position(|c| c.is_ident("of"));
            let (nth_part, of) = match of_pos {
                Some(p) => (args.get(..p)?, Some(args.get(p + 1..)?)),
                None => (args, None),
            };
            let nth = parse_nth(nth_part)?;
            let of_list = match of {
                Some(o) => Some(parse_selector_list(o, ctx, false, false)?),
                None => None,
            };
            let s = pack_specificity(0, 1, 0);
            let s = of_list.as_ref().map_or(s, |l| add_spec(s, max_spec(l)));
            let pc = if name == "nth-child" { PseudoClass::NthChild(nth, of_list) } else { PseudoClass::NthLastChild(nth, of_list) };
            Some((pc, s))
        }
        "nth-of-type" => Some((PseudoClass::NthOfType(parse_nth(trim_ws(args))?), pack_specificity(0, 1, 0))),
        "nth-last-of-type" => Some((PseudoClass::NthLastOfType(parse_nth(trim_ws(args))?), pack_specificity(0, 1, 0))),
        "lang" => {
            let mut langs = Vec::new();
            for part in split_commas(trim_ws(args)) {
                let p = trim_ws(part);
                match p {
                    [Cv::Tok(Token::Ident(s))] | [Cv::Tok(Token::Str(s))] => langs.push(s.clone()),
                    _ => return None,
                }
            }
            if langs.is_empty() {
                return None;
            }
            Some((PseudoClass::Lang(langs), pack_specificity(0, 1, 0)))
        }
        "dir" => match trim_ws(args) {
            [c] if c.is_ident("ltr") => Some((PseudoClass::Dir(false), pack_specificity(0, 1, 0))),
            [c] if c.is_ident("rtl") => Some((PseudoClass::Dir(true), pack_specificity(0, 1, 0))),
            _ => None,
        },
        _ => None,
    }
}

fn max_spec(list: &[Selector]) -> u32 {
    list.iter().map(|s| s.specificity).max().unwrap_or(0)
}

/// The An+B microsyntax (css-syntax-3 6).
pub fn parse_nth(cvs: &[Cv]) -> Option<Nth> {
    let toks: Vec<&Cv> = trim_ws(cvs).iter().collect();
    // Remove white space but remember it where it matters: signs must be separated per the grammar.
    let mut items: Vec<(&Cv, bool)> = Vec::new(); // (token, preceded by white space)
    let mut ws = false;
    for c in &toks {
        if c.is_ws() {
            ws = true;
        } else {
            items.push((c, ws));
            ws = false;
        }
    }
    let (first, _) = *items.first()?;
    let int_of = |v: f64| -> i32 { v.clamp(-1.0e9, 1.0e9) as i32 };
    // Returns (a, b-so-far, index of the next item, needs_signless_b, needs_b_after_dash)
    let mut idx = 1;
    let a: i32;
    let mut b: Option<i32>;
    let mut need_b: bool;
    match first {
        Cv::Tok(Token::Ident(s)) => {
            let l = s.to_ascii_lowercase();
            if l == "odd" {
                return (items.len() == 1).then_some(Nth { a: 2, b: 1 });
            }
            if l == "even" {
                return (items.len() == 1).then_some(Nth { a: 2, b: 0 });
            }
            let (sign, rest) = if let Some(r) = l.strip_prefix('-') { (-1, r.to_string()) } else { (1, l.clone()) };
            if rest == "n" {
                a = sign;
                b = None;
                need_b = false;
            } else if rest == "n-" {
                a = sign;
                b = None;
                need_b = true; // "n-" followed by a signless integer: b = -int
            } else {
                let d = rest.strip_prefix("n-")?;
                if d.is_empty() || !d.chars().all(|c| c.is_ascii_digit()) {
                    return None;
                }
                a = sign;
                b = Some(-(d.parse::<i32>().ok()?));
                need_b = false;
            }
        }
        Cv::Tok(Token::Delim('+')) => {
            // `+n`, `+n-`, `+n-5`: the ident must follow immediately.
            let (Cv::Tok(Token::Ident(s)), spaced) = *items.get(1)? else { return None };
            if spaced {
                return None;
            }
            let l = s.to_ascii_lowercase();
            idx = 2;
            if l == "n" {
                a = 1;
                b = None;
                need_b = false;
            } else if l == "n-" {
                a = 1;
                b = None;
                need_b = true;
            } else {
                let d = l.strip_prefix("n-")?;
                if d.is_empty() || !d.chars().all(|c| c.is_ascii_digit()) {
                    return None;
                }
                a = 1;
                b = Some(-(d.parse::<i32>().ok()?));
                need_b = false;
            }
        }
        Cv::Tok(Token::Number { value, is_int: true, .. }) => {
            return (items.len() == 1).then(|| Nth { a: 0, b: int_of(*value) });
        }
        Cv::Tok(Token::Dimension { value, is_int: true, unit, .. }) => {
            let l = unit.to_ascii_lowercase();
            a = int_of(*value);
            if l == "n" {
                b = None;
                need_b = false;
            } else if l == "n-" {
                b = None;
                need_b = true;
            } else {
                let d = l.strip_prefix("n-")?;
                if d.is_empty() || !d.chars().all(|c| c.is_ascii_digit()) {
                    return None;
                }
                b = Some(-(d.parse::<i32>().ok()?));
                need_b = false;
            }
        }
        _ => return None,
    }
    if need_b {
        // "n-" followed by a signless integer.
        let (Cv::Tok(Token::Number { value, is_int: true, signed: false }), _) = *items.get(idx)? else { return None };
        b = Some(-int_of(*value));
        idx += 1;
        need_b = false;
    }
    let _ = need_b;
    if b.is_none() && idx < items.len() {
        match *items.get(idx)? {
            (Cv::Tok(Token::Number { value, is_int: true, signed: true }), _) => {
                b = Some(int_of(*value));
                idx += 1;
            }
            (op, _) if op.is_delim('+') || op.is_delim('-') => {
                let neg = op.is_delim('-');
                let (Cv::Tok(Token::Number { value, is_int: true, signed: false }), _) = *items.get(idx + 1)? else { return None };
                b = Some(if neg { -int_of(*value) } else { int_of(*value) });
                idx += 2;
            }
            _ => return None,
        }
    }
    if idx != items.len() {
        return None;
    }
    Some(Nth { a, b: b.unwrap_or(0) })
}

fn parse_attr(inner: &[Cv], ctx: &mut ParseCtx<'_>) -> Option<AttrSel> {
    let items = trim_ws(inner);
    let mut i;
    // Optional namespace prefix: `*|name` is fine, `|name` and `prefix|name` are not supported.
    let name = match (items.first(), items.get(1), items.get(2)) {
        (Some(c), Some(bar), Some(Cv::Tok(Token::Ident(n)))) if c.is_delim('*') && bar.is_delim('|') => {
            i = 3;
            n.clone()
        }
        (Some(Cv::Tok(Token::Ident(n))), _, _) => {
            i = 1;
            n.clone()
        }
        _ => return None,
    };
    let skip_ws = |i: &mut usize| {
        while items.get(*i).is_some_and(Cv::is_ws) {
            *i += 1;
        }
    };
    skip_ws(&mut i);
    let lower = ctx.atoms.intern(&name.to_ascii_lowercase());
    let nm = ctx.atoms.intern(&name);
    if i >= items.len() {
        return Some(AttrSel { name: nm, lower, op: AttrOp::Exists, value: String::new(), case: CaseSens::Default });
    }
    let op = match items.get(i)? {
        c if c.is_delim('=') => {
            i += 1;
            AttrOp::Equals
        }
        c => {
            let op = if c.is_delim('~') {
                AttrOp::Includes
            } else if c.is_delim('|') {
                AttrOp::DashMatch
            } else if c.is_delim('^') {
                AttrOp::Prefix
            } else if c.is_delim('$') {
                AttrOp::Suffix
            } else if c.is_delim('*') {
                AttrOp::Substring
            } else {
                return None;
            };
            if !items.get(i + 1)?.is_delim('=') {
                return None;
            }
            i += 2;
            op
        }
    };
    skip_ws(&mut i);
    let value = match items.get(i)? {
        Cv::Tok(Token::Ident(v)) | Cv::Tok(Token::Str(v)) => v.clone(),
        _ => return None,
    };
    i += 1;
    skip_ws(&mut i);
    let mut case = CaseSens::Default;
    if let Some(flag) = items.get(i) {
        if flag.is_ident("i") {
            case = CaseSens::Insensitive;
        } else if flag.is_ident("s") {
            case = CaseSens::Sensitive;
        } else {
            return None;
        }
        i += 1;
        skip_ws(&mut i);
    }
    if i != items.len() {
        return None;
    }
    Some(AttrSel { name: nm, lower, op, value, case })
}

// -----------------------------------------------------------------------------------------------
// Bloom filter for ancestors

/// A counting Bloom filter over (tag, id, class) keys of the open ancestors during a tree walk.
pub struct Bloom {
    counts: Vec<u8>,
}

pub const KIND_TAG: u32 = 1;
pub const KIND_ID: u32 = 2;
pub const KIND_CLASS: u32 = 3;

pub fn bloom_key(kind: u32, atom: Atom) -> u32 {
    (atom.0.wrapping_mul(4).wrapping_add(kind)).wrapping_mul(0x9E37_79B1)
}

impl Bloom {
    const SIZE: usize = 4096;

    pub fn new() -> Bloom {
        Bloom { counts: vec![0; Self::SIZE] }
    }

    fn slots(key: u32) -> (usize, usize) {
        ((key >> 8) as usize % Self::SIZE, (key >> 20 ^ key.rotate_left(7)) as usize % Self::SIZE)
    }

    pub fn add(&mut self, key: u32) {
        let (a, b) = Self::slots(key);
        for i in [a, b] {
            if let Some(c) = self.counts.get_mut(i) {
                *c = c.saturating_add(1);
            }
        }
    }

    pub fn remove(&mut self, key: u32) {
        let (a, b) = Self::slots(key);
        for i in [a, b] {
            if let Some(c) = self.counts.get_mut(i) {
                *c = c.saturating_sub(1);
            }
        }
    }

    pub fn may_contain(&self, key: u32) -> bool {
        let (a, b) = Self::slots(key);
        self.counts.get(a).copied().unwrap_or(1) > 0 && self.counts.get(b).copied().unwrap_or(1) > 0
    }
}

impl Default for Bloom {
    fn default() -> Self {
        Self::new()
    }
}

/// What is known about the open ancestors during a tree walk: a Bloom filter for a quick "no", and exact
/// counts of their (tag, id, class) keys.
pub struct AncestorFilter {
    bloom: Bloom,
    counts: FxHashMap<u32, u32>,
}

impl AncestorFilter {
    pub fn new() -> AncestorFilter {
        AncestorFilter { bloom: Bloom::new(), counts: FxHashMap::default() }
    }
    pub fn add(&mut self, key: u32) {
        self.bloom.add(key);
        *self.counts.entry(key).or_insert(0) += 1;
    }
    pub fn remove(&mut self, key: u32) {
        self.bloom.remove(key);
        if let Some(c) = self.counts.get_mut(&key) {
            *c -= 1;
            if *c == 0 {
                self.counts.remove(&key);
            }
        }
    }
    pub fn may_contain(&self, key: u32) -> bool {
        self.bloom.may_contain(key)
    }
    pub fn contains(&self, key: u32) -> bool {
        self.counts.contains_key(&key)
    }
}

impl Default for AncestorFilter {
    fn default() -> Self {
        Self::new()
    }
}

/// Keys the ancestors must provide: from the compounds joined to the rightmost one by
/// descendant/child combinators (at most 4 keys).
fn ancestor_keys(compounds: &[Compound], combs: &[Combinator]) -> Vec<u32> {
    let mut keys = Vec::new();
    let n = compounds.len();
    for idx in (0..n.saturating_sub(1)).rev() {
        match combs.get(idx) {
            Some(Combinator::Descendant | Combinator::Child) => {}
            _ => break,
        }
        // Bloom keys only describe the *ancestors*, i.e. every compound left of a descendant/child link.
        let Some(c) = compounds.get(idx) else { break };
        for s in &c.simples {
            match s {
                Simple::Id(a) => keys.push(bloom_key(KIND_ID, *a)),
                Simple::Class(a) => keys.push(bloom_key(KIND_CLASS, *a)),
                Simple::Type { lower, .. } => keys.push(bloom_key(KIND_TAG, *lower)),
                _ => {}
            }
        }
        if keys.len() >= 4 {
            break;
        }
    }
    keys.truncate(4);
    keys
}

// -----------------------------------------------------------------------------------------------
// Matching

/// HTML attributes whose values are ASCII case-insensitive in selectors (HTML 15.2 "Case-insensitive attributes").
const CI_ATTRS: &[&str] = &[
    "accept", "accept-charset", "align", "alink", "axis", "bgcolor", "charset", "checked", "clear", "codetype", "color",
    "compact", "declare", "defer", "dir", "direction", "disabled", "enctype", "face", "frame", "hreflang", "http-equiv",
    "lang", "language", "link", "media", "method", "multiple", "nohref", "noresize", "noshade", "nowrap", "readonly",
    "rel", "rev", "rules", "scope", "scrolling", "selected", "shape", "target", "text", "type", "valign", "valuetype",
    "vlink",
];

/// Work limit for one selector match (hostile selectors / documents).
const MATCH_BUDGET: u32 = 20_000;

const NO_ATOM: Atom = Atom(u32::MAX);

/// Matches selectors against elements of one document. Owns caches that make repeated matching cheap.
pub struct Matcher<'a> {
    pub doc: &'a Document,
    pub quirks: bool,
    /// `keys[node]` index into `arena` (u32::MAX = not computed). Arena layout: id atom, class count, classes...
    key_index: Vec<u32>,
    arena: Vec<Atom>,
    /// Sibling positions: [index, last index, type index, last type index] per node (0 = not computed).
    pos: Vec<[u32; 4]>,
    pos_done: Vec<bool>,
    budget: u32,
}

impl<'a> Matcher<'a> {
    pub fn new(doc: &'a Document) -> Matcher<'a> {
        Matcher {
            doc,
            quirks: doc.quirks == crate::html::dom::QuirksMode::Quirks,
            key_index: vec![u32::MAX; doc.nodes.len()],
            arena: Vec::new(),
            pos: Vec::new(),
            pos_done: Vec::new(),
            budget: MATCH_BUDGET,
        }
    }

    fn element_keys_index(&mut self, el: NodeId) -> usize {
        if let Some(&i) = self.key_index.get(el as usize)
            && i != u32::MAX
        {
            return i as usize;
        }
        let start = self.arena.len();
        let doc = self.doc;
        // Layout in the arena: id atom (or NO_ATOM), number of classes, the classes.
        self.arena.push(NO_ATOM);
        self.arena.push(Atom(0));
        let mut count = 0u32;
        if let Some(e) = doc.element(el) {
            for a in &e.attrs {
                if a.ns != Namespace::None {
                    continue;
                }
                if a.name == crate::html::atoms::a::ID && !a.value.is_empty() {
                    let found = if self.quirks { doc.atoms.lookup(&a.value.to_ascii_lowercase()) } else { doc.atoms.lookup(&a.value) };
                    if let Some(slot) = self.arena.get_mut(start) {
                        *slot = found.unwrap_or(NO_ATOM);
                    }
                } else if a.name == crate::html::atoms::a::CLASS {
                    for tok in a.value.split([' ', '\t', '\n', '\x0c', '\r']).filter(|t| !t.is_empty()) {
                        let found = if self.quirks { doc.atoms.lookup(&tok.to_ascii_lowercase()) } else { doc.atoms.lookup(tok) };
                        if let Some(c) = found
                            && !self.arena.get(start + 2..).unwrap_or(&[]).contains(&c)
                        {
                            self.arena.push(c);
                            count += 1;
                        }
                    }
                }
            }
        }
        if let Some(slot) = self.arena.get_mut(start + 1) {
            *slot = Atom(count);
        }
        if let Some(slot) = self.key_index.get_mut(el as usize) {
            *slot = start as u32;
        }
        start
    }

    /// The element's id atom (None if absent or never mentioned by any stylesheet).
    pub fn id_of(&mut self, el: NodeId) -> Option<Atom> {
        let i = self.element_keys_index(el);
        self.arena.get(i).copied().filter(|a| *a != NO_ATOM)
    }

    /// The element's class atoms that the interner knows.
    pub fn classes_of(&mut self, el: NodeId) -> &[Atom] {
        let i = self.element_keys_index(el);
        let n = self.arena.get(i + 1).map_or(0, |a| a.0 as usize);
        self.arena.get(i + 2..i + 2 + n).unwrap_or(&[])
    }

    /// Number of class atoms of the element that the interner knows.
    pub fn class_count(&mut self, el: NodeId) -> usize {
        let i = self.element_keys_index(el);
        self.arena.get(i + 1).map_or(0, |a| a.0 as usize)
    }

    /// The `k`-th class atom of the element.
    pub fn class_at(&mut self, el: NodeId, k: usize) -> Option<Atom> {
        let i = self.element_keys_index(el);
        self.arena.get(i + 2 + k).copied()
    }

    /// Number of attributes (without namespace) of the element.
    pub fn attr_count(&self, el: NodeId) -> usize {
        self.doc.element(el).map_or(0, |e| e.attrs.len())
    }

    /// The name of the `k`-th attribute of the element (`None` for namespaced attributes).
    pub fn attr_name_at(&self, el: NodeId, k: usize) -> Option<Atom> {
        let a = self.doc.element(el)?.attrs.get(k)?;
        if a.ns == Namespace::None { Some(a.name) } else { Some(Atom(u32::MAX)) }
    }

    /// Append the Bloom keys of an element (tag, id, classes) to `out`.
    pub fn push_bloom_keys(&mut self, el: NodeId, out: &mut Vec<u32>) {
        if let Some(e) = self.doc.element(el) {
            out.push(bloom_key(KIND_TAG, e.name));
        }
        if let Some(id) = self.id_of(el) {
            out.push(bloom_key(KIND_ID, id));
        }
        for k in 0..self.class_count(el) {
            if let Some(c) = self.class_at(el, k) {
                out.push(bloom_key(KIND_CLASS, c));
            }
        }
    }

    /// Bloom keys of an element (tag, id, classes), to add to / remove from the ancestor filter.
    pub fn bloom_keys_of(&mut self, el: NodeId) -> Vec<u32> {
        let mut v = Vec::new();
        if let Some(e) = self.doc.element(el) {
            v.push(bloom_key(KIND_TAG, e.name));
        }
        if let Some(id) = self.id_of(el) {
            v.push(bloom_key(KIND_ID, id));
        }
        let cls: Vec<Atom> = self.classes_of(el).to_vec();
        v.extend(cls.into_iter().map(|c| bloom_key(KIND_CLASS, c)));
        v
    }

    fn parent_element(&self, el: NodeId) -> Option<NodeId> {
        let p = self.doc.parent(el);
        if p != NONE && self.doc.element(p).is_some() { Some(p) } else { None }
    }

    fn prev_element(&self, el: NodeId) -> Option<NodeId> {
        let mut s = self.doc.prev_sibling(el);
        while s != NONE {
            if self.doc.element(s).is_some() {
                return Some(s);
            }
            s = self.doc.prev_sibling(s);
        }
        None
    }

    /// Does `sel` match `el`? `ancestors` (if given) describes `el`'s ancestors.
    pub fn matches(&mut self, sel: &Selector, el: NodeId, ancestors: Option<&AncestorFilter>) -> bool {
        if sel.pseudo_element.is_some() {
            return false;
        }
        if let Some(b) = ancestors {
            if sel.ancestor_keys.iter().any(|&k| !b.may_contain(k)) {
                return false;
            }
            if let Some(k) = sel.fast_key {
                // `X Y`: the ancestors are known exactly, no need to climb.
                return b.contains(k) && sel.compounds.last().is_some_and(|c| self.match_compound(c, el));
            }
        }
        self.budget = MATCH_BUDGET;
        let last = sel.compounds.len().saturating_sub(1);
        self.match_from(sel, last, el)
    }

    /// Match a selector ignoring its pseudo-element (used for `:not()` etc. arguments, which have none).
    pub fn matches_any(&mut self, list: &[Selector], el: NodeId) -> bool {
        let saved = self.budget;
        let r = list.iter().any(|s| {
            self.budget = MATCH_BUDGET;
            let last = s.compounds.len().saturating_sub(1);
            self.match_from(s, last, el)
        });
        self.budget = saved;
        r
    }

    fn match_from(&mut self, sel: &Selector, idx: usize, el: NodeId) -> bool {
        if self.budget == 0 {
            return false;
        }
        self.budget -= 1;
        let Some(compound) = sel.compounds.get(idx) else { return false };
        if !self.match_compound(compound, el) {
            return false;
        }
        if idx == 0 {
            return true;
        }
        match sel.combinators.get(idx - 1) {
            Some(Combinator::Child) => match self.parent_element(el) {
                Some(p) => self.match_from(sel, idx - 1, p),
                None => false,
            },
            Some(Combinator::Descendant) => {
                let mut cur = self.parent_element(el);
                while let Some(p) = cur {
                    if self.match_from(sel, idx - 1, p) {
                        return true;
                    }
                    if self.budget == 0 {
                        return false;
                    }
                    cur = self.parent_element(p);
                }
                false
            }
            Some(Combinator::NextSibling) => match self.prev_element(el) {
                Some(s) => self.match_from(sel, idx - 1, s),
                None => false,
            },
            Some(Combinator::SubsequentSibling) => {
                let mut cur = self.prev_element(el);
                while let Some(s) = cur {
                    if self.match_from(sel, idx - 1, s) {
                        return true;
                    }
                    if self.budget == 0 {
                        return false;
                    }
                    cur = self.prev_element(s);
                }
                false
            }
            None => false,
        }
    }

    fn match_compound(&mut self, c: &Compound, el: NodeId) -> bool {
        c.simples.iter().all(|s| self.match_simple(s, el))
    }

    fn match_simple(&mut self, s: &Simple, el: NodeId) -> bool {
        let doc = self.doc;
        let Some(e) = doc.element(el) else { return false };
        match s {
            Simple::Universal => true,
            Simple::NeverMatches => false,
            Simple::Type { name, lower } => {
                if e.ns == Namespace::Html {
                    e.name == *lower
                } else {
                    e.name == *name
                }
            }
            Simple::Id(a) => self.id_of(el) == Some(*a),
            Simple::Class(a) => self.classes_of(el).contains(a),
            Simple::Attr(attr) => self.match_attr(attr, el),
            Simple::Pseudo(p) => self.match_pseudo(p, el),
        }
    }

    fn match_attr(&self, a: &AttrSel, el: NodeId) -> bool {
        let doc = self.doc;
        let Some(e) = doc.element(el) else { return false };
        let is_html = e.ns == Namespace::Html;
        let name = if is_html { a.lower } else { a.name };
        let Some(attr) = e.attrs.iter().find(|x| x.ns == Namespace::None && x.name == name) else { return false };
        if a.op == AttrOp::Exists {
            return true;
        }
        let ci = match a.case {
            CaseSens::Insensitive => true,
            CaseSens::Sensitive => false,
            CaseSens::Default => is_html && CI_ATTRS.contains(&doc.atoms.get(a.lower)),
        };
        let (hay, needle) = (attr.value.as_str(), a.value.as_str());
        let eq = |x: &str, y: &str| if ci { x.eq_ignore_ascii_case(y) } else { x == y };
        let starts = |h: &str, n: &str| h.len() >= n.len() && h.get(..n.len()).is_some_and(|p| eq(p, n));
        let ends = |h: &str, n: &str| h.len() >= n.len() && h.get(h.len() - n.len()..).is_some_and(|p| eq(p, n));
        match a.op {
            AttrOp::Exists => true,
            AttrOp::Equals => eq(hay, needle),
            AttrOp::Includes => {
                !needle.is_empty()
                    && !needle.contains(|c: char| c.is_ascii_whitespace())
                    && hay.split(|c: char| c.is_ascii_whitespace()).any(|w| eq(w, needle))
            }
            AttrOp::DashMatch => eq(hay, needle) || (starts(hay, needle) && hay.as_bytes().get(needle.len()) == Some(&b'-')),
            AttrOp::Prefix => !needle.is_empty() && starts(hay, needle),
            AttrOp::Suffix => !needle.is_empty() && ends(hay, needle),
            AttrOp::Substring => {
                if needle.is_empty() {
                    return false;
                }
                if ci {
                    hay.to_ascii_lowercase().contains(&needle.to_ascii_lowercase())
                } else {
                    hay.contains(needle)
                }
            }
        }
    }

    /// Compute sibling positions for all element children of `parent`.
    fn ensure_positions(&mut self, parent: NodeId) {
        if self.pos.len() < self.doc.nodes.len() {
            self.pos.resize(self.doc.nodes.len(), [0; 4]);
            self.pos_done.resize(self.doc.nodes.len(), false);
        }
        if self.pos_done.get(parent as usize).copied().unwrap_or(true) {
            return;
        }
        if let Some(d) = self.pos_done.get_mut(parent as usize) {
            *d = true;
        }
        let doc = self.doc;
        let kids: Vec<NodeId> = doc.children(parent).filter(|&c| doc.element(c).is_some()).collect();
        let mut type_counts: FxHashMap<(u8, u32), u32> = FxHashMap::default();
        let total = kids.len() as u32;
        let key_of = |c: NodeId| doc.element(c).map_or((0u8, 0u32), |e| (e.ns as u8, e.name.0));
        let mut type_totals: FxHashMap<(u8, u32), u32> = FxHashMap::default();
        for &c in &kids {
            *type_totals.entry(key_of(c)).or_insert(0) += 1;
        }
        for (i, &c) in kids.iter().enumerate() {
            let k = key_of(c);
            let t = type_counts.entry(k).or_insert(0);
            *t += 1;
            let tt = type_totals.get(&k).copied().unwrap_or(1);
            if let Some(slot) = self.pos.get_mut(c as usize) {
                *slot = [i as u32 + 1, total - i as u32, *t, tt - *t + 1];
            }
        }
    }

    fn positions(&mut self, el: NodeId) -> Option<[u32; 4]> {
        let parent = self.doc.parent(el);
        if parent == NONE {
            return None;
        }
        self.ensure_positions(parent);
        self.pos.get(el as usize).copied().filter(|p| p[0] != 0)
    }

    fn match_pseudo(&mut self, p: &PseudoClass, el: NodeId) -> bool {
        let doc = self.doc;
        let Some(e) = doc.element(el) else { return false };
        use crate::html::atoms::a;
        let html = e.ns == Namespace::Html;
        match p {
            PseudoClass::Root => doc.parent(el) == crate::html::dom::DOCUMENT,
            PseudoClass::Empty => doc.children(el).all(|c| match doc.node(c).map(|n| &n.data) {
                Some(NodeData::Element(_)) => false,
                Some(NodeData::Text(t)) => t.is_empty(),
                _ => true,
            }),
            PseudoClass::FirstChild => self.positions(el).is_some_and(|p| p[0] == 1),
            PseudoClass::LastChild => self.positions(el).is_some_and(|p| p[1] == 1),
            PseudoClass::OnlyChild => self.positions(el).is_some_and(|p| p[0] == 1 && p[1] == 1),
            PseudoClass::FirstOfType => self.positions(el).is_some_and(|p| p[2] == 1),
            PseudoClass::LastOfType => self.positions(el).is_some_and(|p| p[3] == 1),
            PseudoClass::OnlyOfType => self.positions(el).is_some_and(|p| p[2] == 1 && p[3] == 1),
            PseudoClass::NthChild(nth, None) => self.positions(el).is_some_and(|p| nth.matches(p[0] as i64)),
            PseudoClass::NthLastChild(nth, None) => self.positions(el).is_some_and(|p| nth.matches(p[1] as i64)),
            PseudoClass::NthOfType(nth) => self.positions(el).is_some_and(|p| nth.matches(p[2] as i64)),
            PseudoClass::NthLastOfType(nth) => self.positions(el).is_some_and(|p| nth.matches(p[3] as i64)),
            PseudoClass::NthChild(nth, Some(of)) => {
                if !self.matches_any(of, el) {
                    return false;
                }
                let mut idx = 1i64;
                let mut s = self.prev_element(el);
                while let Some(x) = s {
                    if self.matches_any(of, x) {
                        idx += 1;
                    }
                    s = self.prev_element(x);
                }
                nth.matches(idx)
            }
            PseudoClass::NthLastChild(nth, Some(of)) => {
                if !self.matches_any(of, el) {
                    return false;
                }
                let mut idx = 1i64;
                let mut s = doc.next_sibling(el);
                while s != NONE {
                    if doc.element(s).is_some() && self.matches_any(of, s) {
                        idx += 1;
                    }
                    s = doc.next_sibling(s);
                }
                nth.matches(idx)
            }
            PseudoClass::Not(list) => !self.matches_any(list, el),
            PseudoClass::Is(list) | PseudoClass::Where(list) => self.matches_any(list, el),
            PseudoClass::Link => html && matches!(e.name, a::A | a::AREA) && doc.attr(el, a::HREF).is_some(),
            PseudoClass::Checked => {
                html && ((e.name == a::INPUT
                    && doc.attr(el, a::TYPE).is_some_and(|t| t.eq_ignore_ascii_case("checkbox") || t.eq_ignore_ascii_case("radio"))
                    && has_attr_named(doc, el, "checked"))
                    || (e.name == a::OPTION && has_attr_named(doc, el, "selected")))
            }
            PseudoClass::Disabled => html && is_form_control(e.name) && is_disabled(doc, el),
            PseudoClass::Enabled => html && is_form_control(e.name) && !is_disabled(doc, el),
            PseudoClass::Lang(ranges) => {
                let Some(lang) = language_of(doc, el) else { return false };
                ranges.iter().any(|r| lang_matches(&lang, r))
            }
            PseudoClass::Dir(rtl) => (direction_of(doc, el) == *rtl) && html,
            PseudoClass::Defined => !doc.atoms.get(e.name).contains('-'),
            PseudoClass::Never => false,
        }
    }
}

fn has_attr_named(doc: &Document, el: NodeId, name: &str) -> bool {
    doc.element(el).is_some_and(|e| e.attrs.iter().any(|a| a.ns == Namespace::None && doc.atoms.get(a.name) == name))
}

fn is_form_control(name: Atom) -> bool {
    use crate::html::atoms::a;
    matches!(name, a::BUTTON | a::INPUT | a::SELECT | a::TEXTAREA | a::OPTGROUP | a::OPTION | a::FIELDSET)
}

fn is_disabled(doc: &Document, el: NodeId) -> bool {
    use crate::html::atoms::a;
    if has_attr_named(doc, el, "disabled") {
        return true;
    }
    // Inside a disabled fieldset (but not inside its first legend).
    let name = doc.element(el).map(|e| e.name);
    if matches!(name, Some(a::BUTTON | a::INPUT | a::SELECT | a::TEXTAREA)) {
        let mut cur = doc.parent(el);
        let mut child = el;
        while cur != NONE {
            if doc.is_html_element(cur, a::FIELDSET) && has_attr_named(doc, cur, "disabled") {
                let first_legend = doc.children(cur).find(|&c| doc.is_html_element(c, a::LEGEND));
                if first_legend != Some(child) {
                    return true;
                }
            }
            child = cur;
            cur = doc.parent(cur);
        }
    }
    false
}

/// Language of an element: the nearest `lang` attribute (HTML 3.2.6.2).
pub fn language_of(doc: &Document, el: NodeId) -> Option<String> {
    let mut cur = el;
    while cur != NONE {
        if let Some(e) = doc.element(cur)
            && let Some(a) = e.attrs.iter().find(|a| a.ns == Namespace::None && doc.atoms.get(a.name) == "lang")
        {
            return if a.value.is_empty() { None } else { Some(a.value.clone()) };
        }
        cur = doc.parent(cur);
    }
    None
}

/// Extended filtering, basic form (RFC 4647 3.3.2) as used by `:lang()`.
fn lang_matches(lang: &str, range: &str) -> bool {
    let l = lang.to_ascii_lowercase();
    let r = range.to_ascii_lowercase();
    if r == "*" {
        return !l.is_empty();
    }
    l == r || (l.starts_with(&r) && l.as_bytes().get(r.len()) == Some(&b'-'))
}

/// `true` if the element's directionality is rtl (nearest `dir` attribute; default ltr).
fn direction_of(doc: &Document, el: NodeId) -> bool {
    let mut cur = el;
    while cur != NONE {
        if let Some(e) = doc.element(cur)
            && let Some(a) = e.attrs.iter().find(|a| a.ns == Namespace::None && doc.atoms.get(a.name) == "dir")
        {
            if a.value.eq_ignore_ascii_case("rtl") {
                return true;
            }
            if a.value.eq_ignore_ascii_case("ltr") {
                return false;
            }
        }
        cur = doc.parent(cur);
    }
    false
}
