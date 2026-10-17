//! CSS syntax: component values and the generic rule / declaration structure
//! (css-syntax-3 section 5 "Parsing"). Errors are recovered the way the standard says:
//! unterminated blocks and functions close at the end of the input, bad declarations are skipped.

use super::tokenizer::{Token, tokenize};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockKind {
    Paren,
    Square,
    Curly,
}

/// A component value (css-syntax-3 5.4.7).
#[derive(Clone, Debug, PartialEq)]
pub enum Cv {
    Tok(Token),
    Func(String, Vec<Cv>),
    Block(BlockKind, Vec<Cv>),
}

impl Cv {
    pub fn is_ws(&self) -> bool {
        matches!(self, Cv::Tok(Token::Whitespace))
    }
    pub fn ident(&self) -> Option<&str> {
        match self {
            Cv::Tok(Token::Ident(s)) => Some(s),
            _ => None,
        }
    }
    pub fn is_ident(&self, name: &str) -> bool {
        self.ident().is_some_and(|s| s.eq_ignore_ascii_case(name))
    }
    pub fn is_delim(&self, c: char) -> bool {
        matches!(self, Cv::Tok(Token::Delim(d)) if *d == c)
    }
    pub fn is_comma(&self) -> bool {
        matches!(self, Cv::Tok(Token::Comma))
    }
}

/// Maximum nesting of blocks and functions; deeper input is flattened (hostile input guard).
const MAX_NESTING: usize = 512;

/// Maximum nesting of `@media` and `@supports` conditions (parentheses and functions, including
/// `selector()` in `@supports`); deeper conditions are invalid (media query: `not all`; supports: false).
pub const MAX_CONDITION_NESTING: usize = 32;

/// True if blocks and functions in `cvs` nest deeper than `limit` levels. It stops descending at the limit,
/// so its own recursion is at most `limit` deep whatever the input.
pub fn nests_deeper_than(cvs: &[Cv], limit: usize) -> bool {
    cvs.iter().any(|c| match c {
        Cv::Func(_, items) | Cv::Block(_, items) => limit == 0 || nests_deeper_than(items, limit.saturating_sub(1)),
        Cv::Tok(_) => false,
    })
}

/// Build the component value tree for a token list.
pub fn component_values(tokens: Vec<Token>) -> Vec<Cv> {
    // Iterative construction with an explicit stack.
    enum Open {
        Func(String),
        Block(BlockKind),
    }
    let mut stack: Vec<(Open, Vec<Cv>)> = Vec::new();
    let mut cur: Vec<Cv> = Vec::new();
    for t in tokens {
        let opener = match &t {
            Token::Function(n) => Some(Open::Func(n.clone())),
            Token::OpenParen => Some(Open::Block(BlockKind::Paren)),
            Token::OpenSquare => Some(Open::Block(BlockKind::Square)),
            Token::OpenCurly => Some(Open::Block(BlockKind::Curly)),
            _ => None,
        };
        if let Some(o) = opener {
            if stack.len() < MAX_NESTING {
                stack.push((o, std::mem::take(&mut cur)));
            }
            continue;
        }
        let closes = match &t {
            Token::CloseParen => Some(false),
            Token::CloseSquare | Token::CloseCurly => Some(true),
            _ => None,
        };
        if let Some(_square_or_curly) = closes {
            // A closing token ends the innermost open construct if it is of the matching kind.
            let matches_top = matches!(
                (stack.last().map(|(o, _)| o), &t),
                (Some(Open::Func(_)), Token::CloseParen)
                    | (Some(Open::Block(BlockKind::Paren)), Token::CloseParen)
                    | (Some(Open::Block(BlockKind::Square)), Token::CloseSquare)
                    | (Some(Open::Block(BlockKind::Curly)), Token::CloseCurly)
            );
            if matches_top {
                if let Some((o, parent)) = stack.pop() {
                    let items = std::mem::replace(&mut cur, parent);
                    cur.push(match o {
                        Open::Func(n) => Cv::Func(n, items),
                        Open::Block(k) => Cv::Block(k, items),
                    });
                }
            } else {
                // Stray closer: kept as a token so that callers can see it (it makes declarations invalid).
                cur.push(Cv::Tok(t));
            }
            continue;
        }
        cur.push(Cv::Tok(t));
    }
    // EOF closes everything.
    while let Some((o, parent)) = stack.pop() {
        let items = std::mem::replace(&mut cur, parent);
        cur.push(match o {
            Open::Func(n) => Cv::Func(n, items),
            Open::Block(k) => Cv::Block(k, items),
        });
    }
    cur
}

/// Parse text into component values.
pub fn parse_cvs(text: &str) -> Vec<Cv> {
    component_values(tokenize(text))
}

/// Below this many bytes of style sheet text in all, tokenizing on several threads costs more than it saves.
const PARALLEL_MIN_BYTES: usize = 256 * 1024;
/// At most this many worker threads for [`parse_cvs_many`].
const PARALLEL_MAX_THREADS: usize = 4;
/// Stack of a worker: the main thread has 8 MB and the parser may recurse as deep as the nesting limits allow.
const WORKER_STACK: usize = 8 << 20;

/// [`parse_cvs`] for several texts, in order. Large sets of sheets are tokenized on up to four threads (the
/// biggest first): a page with 3 MB of CSS spends a third of its style time here, and the texts are independent.
/// The result is the same as calling `parse_cvs` on each.
pub fn parse_cvs_many(texts: &[&str]) -> Vec<Vec<Cv>> {
    let total: usize = texts.iter().map(|t| t.len()).sum();
    let threads = std::thread::available_parallelism().map_or(1, usize::from).min(PARALLEL_MAX_THREADS).min(texts.len());
    if threads < 2 || total < PARALLEL_MIN_BYTES {
        return texts.iter().map(|t| parse_cvs(t)).collect();
    }
    let mut order: Vec<usize> = (0..texts.len()).collect();
    crate::sorting::sort_by_u64_key(&mut order, |&i| !(texts.get(i).map_or(0, |t| t.len()) as u64));
    let next = std::sync::atomic::AtomicUsize::new(0);
    let slots: Vec<std::sync::Mutex<Option<Vec<Cv>>>> = texts.iter().map(|_| std::sync::Mutex::new(None)).collect();
    let work = || {
        while let Some(&i) = order.get(next.fetch_add(1, std::sync::atomic::Ordering::Relaxed)) {
            let (Some(text), Some(slot)) = (texts.get(i), slots.get(i)) else { break };
            let parsed = parse_cvs(text);
            *slot.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = Some(parsed);
        }
    };
    std::thread::scope(|s| {
        for _ in 1..threads {
            // A worker that cannot start just leaves its share to the others (and to this thread).
            let _ = std::thread::Builder::new().stack_size(WORKER_STACK).spawn_scoped(s, work);
        }
        work();
    });
    // A text whose worker panicked is parsed again here: the panic then happens on the caller's thread, as without threads.
    slots.into_iter().zip(texts).map(|(m, t)| m.into_inner().unwrap_or_else(std::sync::PoisonError::into_inner).unwrap_or_else(|| parse_cvs(t))).collect()
}

/// A generic rule: an at-rule or a qualified rule.
#[derive(Clone, Debug)]
pub struct RawRule {
    /// `Some(name)` for an at-rule (lower-cased), `None` for a qualified rule.
    pub at_name: Option<String>,
    pub prelude: Vec<Cv>,
    /// The `{}` block contents, if any.
    pub block: Option<Vec<Cv>>,
}

/// "Consume a list of rules" over component values. `top_level` makes `<!--` and `-->` skippable.
pub fn parse_rule_list(cvs: Vec<Cv>, top_level: bool) -> Vec<RawRule> {
    let mut rules = Vec::new();
    let mut it = cvs.into_iter().peekable();
    while let Some(first) = it.next() {
        match first {
            Cv::Tok(Token::Whitespace) => {}
            Cv::Tok(Token::Cdo | Token::Cdc) if top_level => {}
            Cv::Tok(Token::AtKeyword(name)) => {
                let mut prelude = Vec::new();
                let mut block = None;
                for next in it.by_ref() {
                    match next {
                        Cv::Tok(Token::Semicolon) => break,
                        Cv::Block(BlockKind::Curly, items) => {
                            block = Some(items);
                            break;
                        }
                        other => prelude.push(other),
                    }
                }
                rules.push(RawRule { at_name: Some(name.to_ascii_lowercase()), prelude, block });
            }
            other => {
                // Qualified rule: prelude up to the first {} block; no block means the rule is dropped.
                let mut prelude = vec![other];
                let mut block = None;
                for next in it.by_ref() {
                    match next {
                        Cv::Block(BlockKind::Curly, items) => {
                            block = Some(items);
                            break;
                        }
                        n => prelude.push(n),
                    }
                }
                if block.is_some() {
                    rules.push(RawRule { at_name: None, prelude, block });
                }
            }
        }
    }
    rules
}

/// A raw declaration.
#[derive(Clone, Debug)]
pub struct RawDecl {
    pub name: String,
    pub value: Vec<Cv>,
    pub important: bool,
}

/// "Consume a list of declarations" over component values. Nested rules and at-rules are skipped
/// (CSS nesting is not supported in layer 1).
pub fn parse_declaration_list(cvs: Vec<Cv>) -> Vec<RawDecl> {
    #[derive(PartialEq)]
    enum Mode {
        Normal,
        /// `--custom: ...` values may contain `{}` blocks.
        Custom,
        /// Inside an at-rule: skip to its `;` or `{}` block.
        Skip,
    }
    let mut out = Vec::new();
    let mut segment: Vec<Cv> = Vec::new();
    let mut mode = Mode::Normal;
    for cv in cvs {
        match (&mode, &cv) {
            (Mode::Skip, Cv::Tok(Token::Semicolon) | Cv::Block(BlockKind::Curly, _)) => {
                mode = Mode::Normal;
                segment.clear();
            }
            (Mode::Skip, _) => {}
            (_, Cv::Tok(Token::Semicolon)) => {
                if let Some(d) = declaration_from(std::mem::take(&mut segment)) {
                    out.push(d);
                }
                mode = Mode::Normal;
            }
            (Mode::Normal, Cv::Block(BlockKind::Curly, _)) => {
                // A nested rule (or garbage): drop it together with its prelude.
                segment.clear();
            }
            _ => {
                if !cv.is_ws() && segment.iter().all(Cv::is_ws) {
                    if matches!(&cv, Cv::Tok(Token::AtKeyword(_))) {
                        mode = Mode::Skip;
                        segment.clear();
                        continue;
                    }
                    if matches!(&cv, Cv::Tok(Token::Ident(n)) if n.starts_with("--")) {
                        mode = Mode::Custom;
                    }
                }
                segment.push(cv);
            }
        }
    }
    if mode != Mode::Skip
        && let Some(d) = declaration_from(segment)
    {
        out.push(d);
    }
    out
}

fn declaration_from(mut seg: Vec<Cv>) -> Option<RawDecl> {
    // Trim white space.
    while seg.last().is_some_and(Cv::is_ws) {
        seg.pop();
    }
    let start = seg.iter().position(|c| !c.is_ws())?;
    let seg: Vec<Cv> = seg.into_iter().skip(start).collect();
    let mut it = seg.into_iter();
    let name = match it.next()? {
        Cv::Tok(Token::Ident(n)) => n,
        _ => return None,
    };
    let mut rest: Vec<Cv> = it.collect();
    let first = rest.iter().position(|c| !c.is_ws())?;
    if !matches!(rest.get(first), Some(Cv::Tok(Token::Colon))) {
        return None;
    }
    rest.drain(..=first);
    // Leading white space of the value.
    let lead = rest.iter().position(|c| !c.is_ws()).unwrap_or(rest.len());
    rest.drain(..lead);
    // `!important`.
    let mut important = false;
    let n = rest.len();
    if n >= 2 {
        let last_nonws = |v: &[Cv], end: usize| (0..end).rev().find(|&i| v.get(i).is_some_and(|c| !c.is_ws()));
        if let Some(i_imp) = last_nonws(&rest, n)
            && rest.get(i_imp).is_some_and(|c| c.is_ident("important"))
            && let Some(i_bang) = last_nonws(&rest, i_imp)
            && rest.get(i_bang).is_some_and(|c| c.is_delim('!'))
        {
            rest.truncate(i_bang);
            important = true;
        }
    }
    while rest.last().is_some_and(Cv::is_ws) {
        rest.pop();
    }
    let name = if name.starts_with("--") { name } else { name.to_ascii_lowercase() };
    Some(RawDecl { name, value: rest, important })
}

/// Split component values at top-level commas.
pub fn split_commas(cvs: &[Cv]) -> Vec<&[Cv]> {
    let mut out = Vec::new();
    let mut start = 0;
    for (i, c) in cvs.iter().enumerate() {
        if c.is_comma() {
            out.push(cvs.get(start..i).unwrap_or(&[]));
            start = i + 1;
        }
    }
    out.push(cvs.get(start..).unwrap_or(&[]));
    out
}

/// Trim leading and trailing white space.
pub fn trim_ws(cvs: &[Cv]) -> &[Cv] {
    let start = cvs.iter().position(|c| !c.is_ws()).unwrap_or(cvs.len());
    let end = cvs.iter().rposition(|c| !c.is_ws()).map_or(start, |e| e + 1);
    cvs.get(start..end).unwrap_or(&[])
}

/// Serialize component values back to text (used for `@supports` and debugging).
pub fn serialize_cvs(cvs: &[Cv]) -> String {
    let mut s = String::new();
    for c in cvs {
        serialize_cv(c, &mut s);
    }
    s
}

pub fn serialize_cv(c: &Cv, s: &mut String) {
    match c {
        Cv::Tok(t) => serialize_token(t, s),
        Cv::Func(n, items) => {
            s.push_str(n);
            s.push('(');
            for i in items {
                serialize_cv(i, s);
            }
            s.push(')');
        }
        Cv::Block(k, items) => {
            let (o, cl) = match k {
                BlockKind::Paren => ('(', ')'),
                BlockKind::Square => ('[', ']'),
                BlockKind::Curly => ('{', '}'),
            };
            s.push(o);
            for i in items {
                serialize_cv(i, s);
            }
            s.push(cl);
        }
    }
}

pub fn serialize_token(t: &Token, s: &mut String) {
    use std::fmt::Write as _;
    match t {
        Token::Ident(n) => s.push_str(n),
        Token::Function(n) => {
            s.push_str(n);
            s.push('(');
        }
        Token::AtKeyword(n) => {
            s.push('@');
            s.push_str(n);
        }
        Token::Hash { value, .. } => {
            s.push('#');
            s.push_str(value);
        }
        Token::Str(v) => {
            s.push('"');
            s.push_str(&v.replace('\\', "\\\\").replace('"', "\\\""));
            s.push('"');
        }
        Token::BadStr => s.push_str("\"\n"),
        Token::Url(u) => {
            let _ = write!(s, "url({u})");
        }
        Token::BadUrl => s.push_str("url(!)"),
        Token::Delim(c) => s.push(*c),
        Token::Number { value, .. } => {
            let _ = write!(s, "{value}");
        }
        Token::Percentage { value } => {
            let _ = write!(s, "{value}%");
        }
        Token::Dimension { value, unit, .. } => {
            let _ = write!(s, "{value}{unit}");
        }
        Token::Whitespace => s.push(' '),
        Token::Cdo => s.push_str("<!--"),
        Token::Cdc => s.push_str("-->"),
        Token::Colon => s.push(':'),
        Token::Semicolon => s.push(';'),
        Token::Comma => s.push(','),
        Token::OpenSquare => s.push('['),
        Token::CloseSquare => s.push(']'),
        Token::OpenParen => s.push('('),
        Token::CloseParen => s.push(')'),
        Token::OpenCurly => s.push('{'),
        Token::CloseCurly => s.push('}'),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules_and_recovery() {
        let rules = parse_rule_list(parse_cvs("a{color:red} @media screen{b{c:d}} @import 'x'; e f"), true);
        assert_eq!(rules.len(), 3); // the last qualified rule has no block and is dropped
        assert_eq!(rules[1].at_name.as_deref(), Some("media"));
        assert_eq!(rules[2].at_name.as_deref(), Some("import"));
    }

    #[test]
    fn declarations() {
        let cv = parse_cvs("color: red; margin : 1px 2px !important; bad; --x: {a:b}; width:10px");
        let d = parse_declaration_list(cv);
        let names: Vec<&str> = d.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(names, ["color", "margin", "--x", "width"]);
        assert!(d[1].important);
    }

    #[test]
    fn nested_rule_is_dropped() {
        let d = parse_declaration_list(parse_cvs("color:red; .a:hover { x:y } width:1px"));
        let names: Vec<&str> = d.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(names, ["color", "width"]);
    }

    #[test]
    fn unclosed_blocks_close_at_eof() {
        let rules = parse_rule_list(parse_cvs("a { color: red"), true);
        assert_eq!(rules.len(), 1);
        assert!(rules[0].block.is_some());
    }
}
