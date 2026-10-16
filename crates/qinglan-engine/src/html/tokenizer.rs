//! HTML tokenizer: WHATWG HTML 13.2.5 "Tokenization". State names follow the spec's numbering
//! (13.2.5.1 data state, 13.2.5.2 RCDATA state, ...). Parse errors are not reported.
//!
//! Text is delivered in runs ([`Token::Text`]) instead of one token per code point; this is
//! equivalent for the tree builder, which only looks at the characters' class.
//! Limits: at most [`MAX_ATTRIBUTES`] attributes are kept per tag.

use std::collections::VecDeque;

use super::atoms::{Atom, Interner};
use super::entities;

/// Attributes kept per tag; the rest are dropped (keeps the duplicate check cheap on hostile input).
pub const MAX_ATTRIBUTES: usize = 2048;

pub use super::dom::Attribute;
use super::dom::Namespace;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TagToken {
    pub name: Atom,
    pub attrs: Vec<Attribute>,
    pub self_closing: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DoctypeToken {
    pub name: Option<String>,
    pub public_id: Option<String>,
    pub system_id: Option<String>,
    pub force_quirks: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Token {
    Doctype(DoctypeToken),
    StartTag(TagToken),
    EndTag(TagToken),
    Comment(String),
    /// `<?target data?>` (13.2.5.72 - 13.2.5.76).
    ProcessingInstruction { target: String, data: String },
    /// One or more character tokens.
    Text(String),
    Eof,
}

/// Tokenizer states (HTML 13.2.5.1 - 13.2.5.80). The tree builder may only switch to the first six.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {
    Data,
    RcData,
    RawText,
    ScriptData,
    PlainText,
    CdataSection,
    TagOpen,
    EndTagOpen,
    TagName,
    RcDataLessThan,
    RcDataEndTagOpen,
    RcDataEndTagName,
    RawTextLessThan,
    RawTextEndTagOpen,
    RawTextEndTagName,
    ScriptDataLessThan,
    ScriptDataEndTagOpen,
    ScriptDataEndTagName,
    ScriptDataEscapeStart,
    ScriptDataEscapeStartDash,
    ScriptDataEscaped,
    ScriptDataEscapedDash,
    ScriptDataEscapedDashDash,
    ScriptDataEscapedLessThan,
    ScriptDataEscapedEndTagOpen,
    ScriptDataEscapedEndTagName,
    ScriptDataDoubleEscapeStart,
    ScriptDataDoubleEscaped,
    ScriptDataDoubleEscapedDash,
    ScriptDataDoubleEscapedDashDash,
    ScriptDataDoubleEscapedLessThan,
    ScriptDataDoubleEscapeEnd,
    BeforeAttributeName,
    AttributeName,
    AfterAttributeName,
    BeforeAttributeValue,
    AttributeValueDoubleQuoted,
    AttributeValueSingleQuoted,
    AttributeValueUnquoted,
    AfterAttributeValueQuoted,
    SelfClosingStartTag,
    BogusComment,
    MarkupDeclarationOpen,
    CommentStart,
    CommentStartDash,
    Comment,
    CommentLessThanSign,
    CommentLessThanSignBang,
    CommentLessThanSignBangDash,
    CommentLessThanSignBangDashDash,
    CommentEndDash,
    CommentEnd,
    CommentEndBang,
    Doctype,
    BeforeDoctypeName,
    DoctypeName,
    AfterDoctypeName,
    AfterDoctypePublicKeyword,
    BeforeDoctypePublicIdentifier,
    DoctypePublicIdentifierDoubleQuoted,
    DoctypePublicIdentifierSingleQuoted,
    AfterDoctypePublicIdentifier,
    BetweenDoctypePublicAndSystemIdentifiers,
    AfterDoctypeSystemKeyword,
    BeforeDoctypeSystemIdentifier,
    DoctypeSystemIdentifierDoubleQuoted,
    DoctypeSystemIdentifierSingleQuoted,
    AfterDoctypeSystemIdentifier,
    BogusDoctype,
    CdataSectionBracket,
    CdataSectionEnd,
    ProcessingInstructionOpen,
    ProcessingInstructionTarget,
    AfterProcessingInstructionTarget,
    ProcessingInstructionData,
    ProcessingInstructionQuestionable,
}

pub struct Tokenizer {
    input: String,
    pos: usize,
    state: State,
    text: String,
    queue: VecDeque<Token>,
    finished: bool,
    // current tag token
    tag_is_end: bool,
    tag_name: String,
    tag_attrs: Vec<Attribute>,
    tag_self_closing: bool,
    attr_name: String,
    attr_value: String,
    attr_active: bool,
    last_start_tag: String,
    temp: String,
    comment: String,
    pi_target: String,
    pi_data: String,
    doctype: DoctypeToken,
    /// Set by the tree builder before each token: is the adjusted current node a foreign (non-HTML) element?
    /// Decides whether `<![CDATA[` starts a CDATA section (13.2.5.42).
    pub allow_cdata: bool,
}

#[allow(clippy::indexing_slicing)] // const fn: `u8` indexes a 256-entry table, `i` is bounded by the loop
const fn stop_table(stops: &[u8]) -> [bool; 256] {
    let mut t = [false; 256];
    let mut i = 0;
    while i < stops.len() {
        t[stops[i] as usize] = true;
        i += 1;
    }
    t
}

static STOP_DATA: [bool; 256] = stop_table(b"&<\0");
static STOP_RAW: [bool; 256] = stop_table(b"<\0");
static STOP_PLAIN: [bool; 256] = stop_table(b"\0");
static STOP_CDATA: [bool; 256] = stop_table(b"]");
static STOP_DQ: [bool; 256] = stop_table(b"\"&\0");
static STOP_SQ: [bool; 256] = stop_table(b"'&\0");

fn is_ws(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\x0c' | ' ')
}

impl Tokenizer {
    pub fn new(mut input: String) -> Tokenizer {
        // 13.2.3.5 Preprocessing the input stream: CRLF and lone CR become LF.
        if input.contains('\r') {
            input = input.replace("\r\n", "\n").replace('\r', "\n");
        }
        Tokenizer {
            input,
            pos: 0,
            state: State::Data,
            text: String::new(),
            queue: VecDeque::new(),
            finished: false,
            tag_is_end: false,
            tag_name: String::new(),
            tag_attrs: Vec::new(),
            tag_self_closing: false,
            attr_name: String::new(),
            attr_value: String::new(),
            attr_active: false,
            last_start_tag: String::new(),
            temp: String::new(),
            comment: String::new(),
            pi_target: String::new(),
            pi_data: String::new(),
            doctype: DoctypeToken::default(),
            allow_cdata: false,
        }
    }

    pub fn set_state(&mut self, state: State) {
        self.state = state;
    }

    pub fn set_last_start_tag(&mut self, name: &str) {
        self.last_start_tag = name.to_string();
    }

    /// The next token. After `Token::Eof` it keeps returning `Eof`.
    pub fn next_token(&mut self, atoms: &mut Interner) -> Token {
        loop {
            if let Some(t) = self.queue.pop_front() {
                return t;
            }
            if self.finished {
                return Token::Eof;
            }
            self.step(atoms);
        }
    }

    // ----- input access -----

    fn peek(&self) -> Option<char> {
        let b = *self.input.as_bytes().get(self.pos)?;
        if b < 0x80 { Some(b as char) } else { self.input.get(self.pos..)?.chars().next() }
    }

    /// Consume the current character (if any) and return it.
    fn next_char(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    /// "Reconsume": step back over the character `next_char` returned.
    fn back(&mut self, c: Option<char>) {
        if let Some(c) = c {
            self.pos -= c.len_utf8();
        }
    }

    fn rest(&self) -> &[u8] {
        self.input.as_bytes().get(self.pos..).unwrap_or(&[])
    }

    /// Length of the run of bytes at the current position for which `stops` is false.
    fn run_len(&self, stops: &[bool; 256]) -> usize {
        let rest = self.rest();
        rest.iter().position(|&b| stops.get(b as usize).copied().unwrap_or(true)).unwrap_or(rest.len())
    }

    // ----- emitting -----

    fn flush_text(&mut self) {
        if !self.text.is_empty() {
            // Copy to an exactly sized string and keep the buffer: a taken buffer would regrow from nothing
            // for the next run of text (several reallocations each).
            let t = self.text.clone();
            self.text.clear();
            self.queue.push_back(Token::Text(t));
        }
    }

    fn emit_char(&mut self, c: char) {
        self.text.push(c);
    }

    fn emit_str(&mut self, s: &str) {
        self.text.push_str(s);
    }

    fn emit_eof(&mut self) {
        self.flush_text();
        self.queue.push_back(Token::Eof);
        self.finished = true;
    }

    fn emit_comment(&mut self) {
        self.flush_text();
        let c = std::mem::take(&mut self.comment);
        self.queue.push_back(Token::Comment(c));
    }

    fn emit_pi(&mut self) {
        self.flush_text();
        let target = std::mem::take(&mut self.pi_target);
        let data = std::mem::take(&mut self.pi_data);
        self.queue.push_back(Token::ProcessingInstruction { target, data });
    }

    /// "Convert the temporary buffer to a comment" (13.2.5): the comment data is "?" plus the buffer.
    fn temp_to_comment(&mut self) {
        self.comment.clear();
        self.comment.push('?');
        let t = std::mem::take(&mut self.temp);
        self.comment.push_str(&t);
    }

    fn emit_doctype(&mut self) {
        self.flush_text();
        let d = std::mem::take(&mut self.doctype);
        self.queue.push_back(Token::Doctype(d));
    }

    fn new_tag(&mut self, end: bool) {
        self.tag_is_end = end;
        self.tag_name.clear();
        self.tag_attrs.clear();
        self.tag_self_closing = false;
        self.attr_active = false;
    }

    fn finish_attr(&mut self, atoms: &mut Interner) {
        if !self.attr_active {
            return;
        }
        self.attr_active = false;
        let name = atoms.intern(&self.attr_name);
        // Duplicate attribute names: the first one wins (13.2.5.33). Too many: drop the rest.
        if self.tag_attrs.len() >= MAX_ATTRIBUTES || self.tag_attrs.iter().any(|a| a.name == name) {
            self.attr_value.clear();
            return;
        }
        let value = self.attr_value.clone(); // exactly sized, the buffer is kept (see `flush_text`)
        self.attr_value.clear();
        self.tag_attrs.push(Attribute { ns: Namespace::None, prefix: super::atoms::a::EMPTY, name, value });
    }

    fn start_attr(&mut self, atoms: &mut Interner) {
        self.finish_attr(atoms);
        self.attr_active = true;
        self.attr_name.clear();
        self.attr_value.clear();
    }

    fn emit_tag(&mut self, atoms: &mut Interner) {
        self.finish_attr(atoms);
        self.flush_text();
        let name = atoms.intern(&self.tag_name);
        if self.tag_is_end {
            self.tag_attrs.clear();
            self.queue.push_back(Token::EndTag(TagToken { name, attrs: Vec::new(), self_closing: false }));
        } else {
            self.last_start_tag.clear();
            self.last_start_tag.push_str(&self.tag_name);
            // Exactly sized: a pushed-to Vec would keep room for four attributes in every element, for the life of the page.
            let mut attrs: Vec<Attribute> = Vec::with_capacity(self.tag_attrs.len());
            attrs.append(&mut self.tag_attrs);
            self.queue.push_back(Token::StartTag(TagToken { name, attrs, self_closing: self.tag_self_closing }));
        }
    }

    fn appropriate_end_tag(&self) -> bool {
        self.tag_is_end && self.tag_name == self.last_start_tag
    }

    fn attr_value_push(&mut self, c: char) {
        self.attr_value.push(c);
    }

    // ----- character references (13.2.5.72 - 13.2.5.80) -----

    fn emit_ref_text(&mut self, in_attr: bool, s: &str) {
        if in_attr {
            self.attr_value.push_str(s);
        } else {
            self.text.push_str(s);
        }
    }

    /// Called right after a `&` was consumed. Appends the decoded text (or the literal text) to the
    /// current attribute value (`in_attr`) or to the text run; leaves `pos` after what was consumed.
    fn char_ref(&mut self, in_attr: bool) {
        let rest = self.rest();
        match rest.first() {
            Some(b) if b.is_ascii_alphanumeric() => {
                if let Some((len, cps)) = entities::longest_match(rest) {
                    let ends_semi = rest.get(len.wrapping_sub(1)) == Some(&b';');
                    let next = rest.get(len).copied();
                    if in_attr && !ends_semi && next.is_some_and(|n| n == b'=' || n.is_ascii_alphanumeric()) {
                        // Historical rule in attributes: not decoded.
                        let name = self.input.get(self.pos..self.pos + len).unwrap_or("").to_string();
                        self.emit_ref_text(true, "&");
                        self.emit_ref_text(true, &name);
                    } else {
                        let mut s = String::new();
                        for cp in cps {
                            if cp != 0 {
                                s.push(char::from_u32(cp).unwrap_or('\u{FFFD}'));
                            }
                        }
                        self.emit_ref_text(in_attr, &s);
                    }
                    self.pos += len;
                } else {
                    self.emit_ref_text(in_attr, "&");
                }
            }
            Some(b'#') => self.numeric_ref(in_attr),
            _ => self.emit_ref_text(in_attr, "&"),
        }
    }

    /// 13.2.5.75 - 13.2.5.80 numeric character reference states.
    fn numeric_ref(&mut self, in_attr: bool) {
        // self.pos is at '#'.
        let bytes = self.input.as_bytes();
        let mut p = self.pos + 1;
        let hex = matches!(bytes.get(p), Some(b'x' | b'X'));
        let prefix_len = if hex { 2 } else { 1 };
        if hex {
            p += 1;
        }
        let start = p;
        let mut code: u32 = 0;
        while let Some(&b) = bytes.get(p) {
            let d = match b {
                b'0'..=b'9' => (b - b'0') as u32,
                b'a'..=b'f' if hex => (b - b'a' + 10) as u32,
                b'A'..=b'F' if hex => (b - b'A' + 10) as u32,
                _ => break,
            };
            code = code.saturating_mul(if hex { 16 } else { 10 }).saturating_add(d).min(0x11_0000);
            p += 1;
        }
        if p == start {
            // Absence of digits: the "&#" / "&#x" is literal text.
            let lit = self.input.get(self.pos - 1..self.pos + prefix_len).unwrap_or("&#").to_string();
            self.emit_ref_text(in_attr, &lit);
            self.pos += prefix_len;
            return;
        }
        if bytes.get(p) == Some(&b';') {
            p += 1;
        }
        self.pos = p;
        let c = match code {
            0 | 0x11_0000.. | 0xD800..=0xDFFF => '\u{FFFD}',
            0x80..=0x9F => entities::c1_replacement(code).unwrap_or_else(|| char::from_u32(code).unwrap_or('\u{FFFD}')),
            _ => char::from_u32(code).unwrap_or('\u{FFFD}'),
        };
        let mut buf = [0u8; 4];
        self.emit_ref_text(in_attr, c.encode_utf8(&mut buf));
    }

    // ----- the state machine -----

    /// Run the state machine until a token is queued (or the input ends).
    fn step(&mut self, atoms: &mut Interner) {
        loop {
            self.step_once(atoms);
            if !self.queue.is_empty() || self.finished {
                return;
            }
        }
    }

    /// Run one step of the state machine. May queue tokens (or none).
    #[inline(always)]
    fn step_once(&mut self, atoms: &mut Interner) {
        use State as S;
        match self.state {
            // 13.2.5.1
            S::Data => {
                let n = self.run_len(&STOP_DATA);
                if n > 0 {
                    if let Some(s) = self.input.get(self.pos..self.pos + n) {
                        self.text.push_str(s);
                    }
                    self.pos += n;
                }
                match self.next_char() {
                    Some('&') => {
                        self.char_ref(false);
                    }
                    Some('<') => self.state = S::TagOpen,
                    Some(c) => self.emit_char(c), // U+0000: parse error, emitted as is
                    None => self.emit_eof(),
                }
            }
            // 13.2.5.2
            S::RcData => {
                let n = self.run_len(&STOP_DATA);
                if n > 0 {
                    if let Some(s) = self.input.get(self.pos..self.pos + n) {
                        self.text.push_str(s);
                    }
                    self.pos += n;
                }
                match self.next_char() {
                    Some('&') => {
                        self.char_ref(false);
                    }
                    Some('<') => self.state = S::RcDataLessThan,
                    Some('\0') => self.emit_char('\u{FFFD}'),
                    Some(c) => self.emit_char(c),
                    None => self.emit_eof(),
                }
            }
            // 13.2.5.3
            S::RawText => self.raw_like(S::RawTextLessThan),
            // 13.2.5.4
            S::ScriptData => self.raw_like(S::ScriptDataLessThan),
            // 13.2.5.5
            S::PlainText => {
                let n = self.run_len(&STOP_PLAIN);
                if n > 0 {
                    if let Some(s) = self.input.get(self.pos..self.pos + n) {
                        self.text.push_str(s);
                    }
                    self.pos += n;
                }
                match self.next_char() {
                    Some('\0') => self.emit_char('\u{FFFD}'),
                    Some(c) => self.emit_char(c),
                    None => self.emit_eof(),
                }
            }
            // 13.2.5.6
            S::TagOpen => {
                let c = self.next_char();
                match c {
                    Some('!') => self.state = S::MarkupDeclarationOpen,
                    Some('/') => self.state = S::EndTagOpen,
                    Some(ch) if ch.is_ascii_alphabetic() => {
                        self.new_tag(false);
                        self.back(c);
                        self.state = S::TagName;
                    }
                    Some('?') => {
                        self.temp.clear();
                        self.state = S::ProcessingInstructionOpen;
                    }
                    None => {
                        self.emit_char('<');
                        self.emit_eof();
                    }
                    Some(_) => {
                        self.emit_char('<');
                        self.back(c);
                        self.state = S::Data;
                    }
                }
            }
            // 13.2.5.7
            S::EndTagOpen => {
                let c = self.next_char();
                match c {
                    Some(ch) if ch.is_ascii_alphabetic() => {
                        self.new_tag(true);
                        self.back(c);
                        self.state = S::TagName;
                    }
                    Some('>') => self.state = S::Data,
                    None => {
                        self.emit_str("</");
                        self.emit_eof();
                    }
                    Some(_) => {
                        self.comment.clear();
                        self.back(c);
                        self.state = S::BogusComment;
                    }
                }
            }
            // 13.2.5.8
            S::TagName => {
                // Fast path: a run of plain ASCII name characters.
                let bytes = self.input.as_bytes().get(self.pos..).unwrap_or(&[]);
                let mut n = 0;
                while let Some(&b) = bytes.get(n) {
                    if b.is_ascii() && !matches!(b, b'\t' | b'\n' | 0x0c | b' ' | b'/' | b'>' | 0) {
                        self.tag_name.push(b.to_ascii_lowercase() as char);
                        n += 1;
                    } else {
                        break;
                    }
                }
                self.pos += n;
                let c = self.next_char();
                match c {
                    Some(ch) if is_ws(ch) => self.state = S::BeforeAttributeName,
                    Some('/') => self.state = S::SelfClosingStartTag,
                    Some('>') => {
                        self.state = S::Data;
                        self.emit_tag(atoms);
                    }
                    Some('\0') => self.tag_name.push('\u{FFFD}'),
                    Some(ch) => self.tag_name.push(ch.to_ascii_lowercase()),
                    None => self.emit_eof(),
                }
            }
            // 13.2.5.9 - 13.2.5.11
            S::RcDataLessThan => self.lt_state(S::RcDataEndTagOpen, S::RcData, true),
            S::RcDataEndTagOpen => self.end_tag_open(S::RcDataEndTagName, S::RcData),
            S::RcDataEndTagName => self.end_tag_name(S::RcData, atoms),
            // 13.2.5.12 - 13.2.5.14
            S::RawTextLessThan => self.lt_state(S::RawTextEndTagOpen, S::RawText, true),
            S::RawTextEndTagOpen => self.end_tag_open(S::RawTextEndTagName, S::RawText),
            S::RawTextEndTagName => self.end_tag_name(S::RawText, atoms),
            // 13.2.5.15
            S::ScriptDataLessThan => {
                let c = self.next_char();
                match c {
                    Some('/') => {
                        self.temp.clear();
                        self.state = S::ScriptDataEndTagOpen;
                    }
                    Some('!') => {
                        self.state = S::ScriptDataEscapeStart;
                        self.emit_str("<!");
                    }
                    _ => {
                        self.emit_char('<');
                        self.back(c);
                        self.state = S::ScriptData;
                    }
                }
            }
            S::ScriptDataEndTagOpen => self.end_tag_open(S::ScriptDataEndTagName, S::ScriptData),
            S::ScriptDataEndTagName => self.end_tag_name(S::ScriptData, atoms),
            // 13.2.5.18
            S::ScriptDataEscapeStart => {
                let c = self.next_char();
                if c == Some('-') {
                    self.state = S::ScriptDataEscapeStartDash;
                    self.emit_char('-');
                } else {
                    self.back(c);
                    self.state = S::ScriptData;
                }
            }
            S::ScriptDataEscapeStartDash => {
                let c = self.next_char();
                if c == Some('-') {
                    self.state = S::ScriptDataEscapedDashDash;
                    self.emit_char('-');
                } else {
                    self.back(c);
                    self.state = S::ScriptData;
                }
            }
            // 13.2.5.20
            S::ScriptDataEscaped => match self.next_char() {
                Some('-') => {
                    self.state = S::ScriptDataEscapedDash;
                    self.emit_char('-');
                }
                Some('<') => self.state = S::ScriptDataEscapedLessThan,
                Some('\0') => self.emit_char('\u{FFFD}'),
                Some(c) => self.emit_char(c),
                None => self.emit_eof(),
            },
            S::ScriptDataEscapedDash => match self.next_char() {
                Some('-') => {
                    self.state = S::ScriptDataEscapedDashDash;
                    self.emit_char('-');
                }
                Some('<') => self.state = S::ScriptDataEscapedLessThan,
                Some('\0') => {
                    self.state = S::ScriptDataEscaped;
                    self.emit_char('\u{FFFD}');
                }
                Some(c) => {
                    self.state = S::ScriptDataEscaped;
                    self.emit_char(c);
                }
                None => self.emit_eof(),
            },
            S::ScriptDataEscapedDashDash => match self.next_char() {
                Some('-') => self.emit_char('-'),
                Some('<') => self.state = S::ScriptDataEscapedLessThan,
                Some('>') => {
                    self.state = S::ScriptData;
                    self.emit_char('>');
                }
                Some('\0') => {
                    self.state = S::ScriptDataEscaped;
                    self.emit_char('\u{FFFD}');
                }
                Some(c) => {
                    self.state = S::ScriptDataEscaped;
                    self.emit_char(c);
                }
                None => self.emit_eof(),
            },
            // 13.2.5.23
            S::ScriptDataEscapedLessThan => {
                let c = self.next_char();
                match c {
                    Some('/') => {
                        self.temp.clear();
                        self.state = S::ScriptDataEscapedEndTagOpen;
                    }
                    Some(ch) if ch.is_ascii_alphabetic() => {
                        self.temp.clear();
                        self.emit_char('<');
                        self.back(c);
                        self.state = S::ScriptDataDoubleEscapeStart;
                    }
                    _ => {
                        self.emit_char('<');
                        self.back(c);
                        self.state = S::ScriptDataEscaped;
                    }
                }
            }
            S::ScriptDataEscapedEndTagOpen => self.end_tag_open(S::ScriptDataEscapedEndTagName, S::ScriptDataEscaped),
            S::ScriptDataEscapedEndTagName => self.end_tag_name(S::ScriptDataEscaped, atoms),
            // 13.2.5.26
            S::ScriptDataDoubleEscapeStart => {
                let c = self.next_char();
                match c {
                    Some(ch) if is_ws(ch) || ch == '/' || ch == '>' => {
                        self.state = if self.temp == "script" { S::ScriptDataDoubleEscaped } else { S::ScriptDataEscaped };
                        self.emit_char(ch);
                    }
                    Some(ch) if ch.is_ascii_alphabetic() => {
                        self.temp.push(ch.to_ascii_lowercase());
                        self.emit_char(ch);
                    }
                    _ => {
                        self.back(c);
                        self.state = S::ScriptDataEscaped;
                    }
                }
            }
            // 13.2.5.27
            S::ScriptDataDoubleEscaped => match self.next_char() {
                Some('-') => {
                    self.state = S::ScriptDataDoubleEscapedDash;
                    self.emit_char('-');
                }
                Some('<') => {
                    self.state = S::ScriptDataDoubleEscapedLessThan;
                    self.emit_char('<');
                }
                Some('\0') => self.emit_char('\u{FFFD}'),
                Some(c) => self.emit_char(c),
                None => self.emit_eof(),
            },
            S::ScriptDataDoubleEscapedDash => match self.next_char() {
                Some('-') => {
                    self.state = S::ScriptDataDoubleEscapedDashDash;
                    self.emit_char('-');
                }
                Some('<') => {
                    self.state = S::ScriptDataDoubleEscapedLessThan;
                    self.emit_char('<');
                }
                Some('\0') => {
                    self.state = S::ScriptDataDoubleEscaped;
                    self.emit_char('\u{FFFD}');
                }
                Some(c) => {
                    self.state = S::ScriptDataDoubleEscaped;
                    self.emit_char(c);
                }
                None => self.emit_eof(),
            },
            S::ScriptDataDoubleEscapedDashDash => match self.next_char() {
                Some('-') => self.emit_char('-'),
                Some('<') => {
                    self.state = S::ScriptDataDoubleEscapedLessThan;
                    self.emit_char('<');
                }
                Some('>') => {
                    self.state = S::ScriptData;
                    self.emit_char('>');
                }
                Some('\0') => {
                    self.state = S::ScriptDataDoubleEscaped;
                    self.emit_char('\u{FFFD}');
                }
                Some(c) => {
                    self.state = S::ScriptDataDoubleEscaped;
                    self.emit_char(c);
                }
                None => self.emit_eof(),
            },
            S::ScriptDataDoubleEscapedLessThan => {
                let c = self.next_char();
                if c == Some('/') {
                    self.temp.clear();
                    self.state = S::ScriptDataDoubleEscapeEnd;
                    self.emit_char('/');
                } else {
                    self.back(c);
                    self.state = S::ScriptDataDoubleEscaped;
                }
            }
            S::ScriptDataDoubleEscapeEnd => {
                let c = self.next_char();
                match c {
                    Some(ch) if is_ws(ch) || ch == '/' || ch == '>' => {
                        self.state = if self.temp == "script" { S::ScriptDataEscaped } else { S::ScriptDataDoubleEscaped };
                        self.emit_char(ch);
                    }
                    Some(ch) if ch.is_ascii_alphabetic() => {
                        self.temp.push(ch.to_ascii_lowercase());
                        self.emit_char(ch);
                    }
                    _ => {
                        self.back(c);
                        self.state = S::ScriptDataDoubleEscaped;
                    }
                }
            }
            // 13.2.5.32
            S::BeforeAttributeName => {
                while matches!(self.input.as_bytes().get(self.pos), Some(b'\t' | b'\n' | 0x0c | b' ')) {
                    self.pos += 1;
                }
                let c = self.next_char();
                match c {
                    Some(ch) if is_ws(ch) => {}
                    Some('/') | Some('>') | None => {
                        self.back(c);
                        self.state = S::AfterAttributeName;
                    }
                    Some('=') => {
                        self.start_attr(atoms);
                        self.attr_name.push('=');
                        self.state = S::AttributeName;
                    }
                    Some(_) => {
                        self.start_attr(atoms);
                        self.back(c);
                        self.state = S::AttributeName;
                    }
                }
            }
            // 13.2.5.33
            S::AttributeName => {
                let bytes = self.input.as_bytes().get(self.pos..).unwrap_or(&[]);
                let mut n = 0;
                while let Some(&b) = bytes.get(n) {
                    if b.is_ascii() && !matches!(b, b'\t' | b'\n' | 0x0c | b' ' | b'/' | b'>' | b'=' | 0) {
                        self.attr_name.push(b.to_ascii_lowercase() as char);
                        n += 1;
                    } else {
                        break;
                    }
                }
                self.pos += n;
                let c = self.next_char();
                match c {
                    Some(ch) if is_ws(ch) => {
                        self.back(c);
                        self.state = S::AfterAttributeName;
                    }
                    Some('/') | Some('>') | None => {
                        self.back(c);
                        self.state = S::AfterAttributeName;
                    }
                    Some('=') => self.state = S::BeforeAttributeValue,
                    Some('\0') => self.attr_name.push('\u{FFFD}'),
                    Some(ch) => self.attr_name.push(ch.to_ascii_lowercase()),
                }
            }
            // 13.2.5.34
            S::AfterAttributeName => {
                let c = self.next_char();
                match c {
                    Some(ch) if is_ws(ch) => {}
                    Some('/') => self.state = S::SelfClosingStartTag,
                    Some('=') => self.state = S::BeforeAttributeValue,
                    Some('>') => {
                        self.state = S::Data;
                        self.emit_tag(atoms);
                    }
                    None => self.emit_eof(),
                    Some(_) => {
                        self.start_attr(atoms);
                        self.back(c);
                        self.state = S::AttributeName;
                    }
                }
            }
            // 13.2.5.35
            S::BeforeAttributeValue => {
                let c = self.next_char();
                match c {
                    Some(ch) if is_ws(ch) => {}
                    Some('"') => self.state = S::AttributeValueDoubleQuoted,
                    Some('\'') => self.state = S::AttributeValueSingleQuoted,
                    Some('>') => {
                        self.state = S::Data;
                        self.emit_tag(atoms);
                    }
                    _ => {
                        self.back(c);
                        self.state = S::AttributeValueUnquoted;
                    }
                }
            }
            // 13.2.5.36 / 37
            S::AttributeValueDoubleQuoted => self.quoted_value(b'"'),
            S::AttributeValueSingleQuoted => self.quoted_value(b'\''),
            // 13.2.5.38
            S::AttributeValueUnquoted => {
                let c = self.next_char();
                match c {
                    Some(ch) if is_ws(ch) => self.state = S::BeforeAttributeName,
                    Some('&') => {
                        self.char_ref(true);
                    }
                    Some('>') => {
                        self.state = S::Data;
                        self.emit_tag(atoms);
                    }
                    Some('\0') => self.attr_value_push('\u{FFFD}'),
                    Some(ch) => self.attr_value_push(ch),
                    None => self.emit_eof(),
                }
            }
            // 13.2.5.39
            S::AfterAttributeValueQuoted => {
                let c = self.next_char();
                match c {
                    Some(ch) if is_ws(ch) => self.state = S::BeforeAttributeName,
                    Some('/') => self.state = S::SelfClosingStartTag,
                    Some('>') => {
                        self.state = S::Data;
                        self.emit_tag(atoms);
                    }
                    None => self.emit_eof(),
                    Some(_) => {
                        self.back(c);
                        self.state = S::BeforeAttributeName;
                    }
                }
            }
            // 13.2.5.40
            S::SelfClosingStartTag => {
                let c = self.next_char();
                match c {
                    Some('>') => {
                        self.tag_self_closing = true;
                        self.state = S::Data;
                        self.emit_tag(atoms);
                    }
                    None => self.emit_eof(),
                    Some(_) => {
                        self.back(c);
                        self.state = S::BeforeAttributeName;
                    }
                }
            }
            // 13.2.5.41
            S::BogusComment => match self.next_char() {
                Some('>') => {
                    self.state = S::Data;
                    self.emit_comment();
                }
                Some('\0') => self.comment.push('\u{FFFD}'),
                Some(c) => self.comment.push(c),
                None => {
                    self.emit_comment();
                    self.emit_eof();
                }
            },
            // 13.2.5.42
            S::MarkupDeclarationOpen => {
                let rest = self.rest();
                if rest.starts_with(b"--") {
                    self.pos += 2;
                    self.comment.clear();
                    self.state = S::CommentStart;
                } else if rest.get(..7).is_some_and(|w| w.eq_ignore_ascii_case(b"DOCTYPE")) {
                    self.pos += 7;
                    self.state = S::Doctype;
                } else if rest.starts_with(b"[CDATA[") {
                    self.pos += 7;
                    if self.allow_cdata {
                        self.state = S::CdataSection;
                    } else {
                        self.comment.clear();
                        self.comment.push_str("[CDATA[");
                        self.state = S::BogusComment;
                    }
                } else {
                    self.comment.clear();
                    self.state = S::BogusComment;
                }
            }
            // 13.2.5.43
            S::CommentStart => {
                let c = self.next_char();
                match c {
                    Some('-') => self.state = S::CommentStartDash,
                    Some('>') => {
                        self.state = S::Data;
                        self.emit_comment();
                    }
                    _ => {
                        self.back(c);
                        self.state = S::Comment;
                    }
                }
            }
            S::CommentStartDash => {
                let c = self.next_char();
                match c {
                    Some('-') => self.state = S::CommentEnd,
                    Some('>') => {
                        self.state = S::Data;
                        self.emit_comment();
                    }
                    None => {
                        self.emit_comment();
                        self.emit_eof();
                    }
                    Some(_) => {
                        self.comment.push('-');
                        self.back(c);
                        self.state = S::Comment;
                    }
                }
            }
            // 13.2.5.45
            S::Comment => match self.next_char() {
                Some('<') => {
                    self.comment.push('<');
                    self.state = S::CommentLessThanSign;
                }
                Some('-') => self.state = S::CommentEndDash,
                Some('\0') => self.comment.push('\u{FFFD}'),
                Some(c) => self.comment.push(c),
                None => {
                    self.emit_comment();
                    self.emit_eof();
                }
            },
            S::CommentLessThanSign => {
                let c = self.next_char();
                match c {
                    Some('!') => {
                        self.comment.push('!');
                        self.state = S::CommentLessThanSignBang;
                    }
                    Some('<') => self.comment.push('<'),
                    _ => {
                        self.back(c);
                        self.state = S::Comment;
                    }
                }
            }
            S::CommentLessThanSignBang => {
                let c = self.next_char();
                if c == Some('-') {
                    self.state = S::CommentLessThanSignBangDash;
                } else {
                    self.back(c);
                    self.state = S::Comment;
                }
            }
            S::CommentLessThanSignBangDash => {
                let c = self.next_char();
                if c == Some('-') {
                    self.state = S::CommentLessThanSignBangDashDash;
                } else {
                    self.back(c);
                    self.state = S::CommentEndDash;
                }
            }
            S::CommentLessThanSignBangDashDash => {
                let c = self.next_char();
                self.back(c);
                self.state = S::CommentEnd;
            }
            // 13.2.5.50
            S::CommentEndDash => {
                let c = self.next_char();
                match c {
                    Some('-') => self.state = S::CommentEnd,
                    None => {
                        self.emit_comment();
                        self.emit_eof();
                    }
                    Some(_) => {
                        self.comment.push('-');
                        self.back(c);
                        self.state = S::Comment;
                    }
                }
            }
            S::CommentEnd => {
                let c = self.next_char();
                match c {
                    Some('>') => {
                        self.state = S::Data;
                        self.emit_comment();
                    }
                    Some('!') => self.state = S::CommentEndBang,
                    Some('-') => self.comment.push('-'),
                    None => {
                        self.emit_comment();
                        self.emit_eof();
                    }
                    Some(_) => {
                        self.comment.push_str("--");
                        self.back(c);
                        self.state = S::Comment;
                    }
                }
            }
            S::CommentEndBang => {
                let c = self.next_char();
                match c {
                    Some('-') => {
                        self.comment.push_str("--!");
                        self.state = S::CommentEndDash;
                    }
                    Some('>') => {
                        self.state = S::Data;
                        self.emit_comment();
                    }
                    None => {
                        self.emit_comment();
                        self.emit_eof();
                    }
                    Some(_) => {
                        self.comment.push_str("--!");
                        self.back(c);
                        self.state = S::Comment;
                    }
                }
            }
            // 13.2.5.53 DOCTYPE
            S::Doctype => {
                let c = self.next_char();
                match c {
                    Some(ch) if is_ws(ch) => self.state = S::BeforeDoctypeName,
                    None => {
                        self.doctype = DoctypeToken { force_quirks: true, ..Default::default() };
                        self.emit_doctype();
                        self.emit_eof();
                    }
                    Some(_) => {
                        // Includes '>': the missing-name case is handled by BeforeDoctypeName.
                        self.back(c);
                        self.state = S::BeforeDoctypeName;
                    }
                }
            }
            S::BeforeDoctypeName => {
                let c = self.next_char();
                match c {
                    Some(ch) if is_ws(ch) => {}
                    Some('\0') => {
                        self.doctype = DoctypeToken { name: Some("\u{FFFD}".to_string()), ..Default::default() };
                        self.state = S::DoctypeName;
                    }
                    Some('>') => {
                        self.doctype = DoctypeToken { force_quirks: true, ..Default::default() };
                        self.state = S::Data;
                        self.emit_doctype();
                    }
                    None => {
                        self.doctype = DoctypeToken { force_quirks: true, ..Default::default() };
                        self.emit_doctype();
                        self.emit_eof();
                    }
                    Some(ch) => {
                        self.doctype = DoctypeToken { name: Some(ch.to_ascii_lowercase().to_string()), ..Default::default() };
                        self.state = S::DoctypeName;
                    }
                }
            }
            S::DoctypeName => {
                let c = self.next_char();
                match c {
                    Some(ch) if is_ws(ch) => self.state = S::AfterDoctypeName,
                    Some('>') => {
                        self.state = S::Data;
                        self.emit_doctype();
                    }
                    Some('\0') => self.doctype.name.get_or_insert_with(String::new).push('\u{FFFD}'),
                    Some(ch) => self.doctype.name.get_or_insert_with(String::new).push(ch.to_ascii_lowercase()),
                    None => self.doctype_eof(),
                }
            }
            S::AfterDoctypeName => {
                let c = self.next_char();
                match c {
                    Some(ch) if is_ws(ch) => {}
                    Some('>') => {
                        self.state = S::Data;
                        self.emit_doctype();
                    }
                    None => self.doctype_eof(),
                    Some(_) => {
                        self.back(c);
                        let rest = self.rest();
                        if rest.get(..6).is_some_and(|w| w.eq_ignore_ascii_case(b"PUBLIC")) {
                            self.pos += 6;
                            self.state = S::AfterDoctypePublicKeyword;
                        } else if rest.get(..6).is_some_and(|w| w.eq_ignore_ascii_case(b"SYSTEM")) {
                            self.pos += 6;
                            self.state = S::AfterDoctypeSystemKeyword;
                        } else {
                            self.doctype.force_quirks = true;
                            self.state = S::BogusDoctype;
                        }
                    }
                }
            }
            S::AfterDoctypePublicKeyword => {
                let c = self.next_char();
                match c {
                    Some(ch) if is_ws(ch) => self.state = S::BeforeDoctypePublicIdentifier,
                    Some('"') => {
                        self.doctype.public_id = Some(String::new());
                        self.state = S::DoctypePublicIdentifierDoubleQuoted;
                    }
                    Some('\'') => {
                        self.doctype.public_id = Some(String::new());
                        self.state = S::DoctypePublicIdentifierSingleQuoted;
                    }
                    Some('>') => self.doctype_abrupt(),
                    None => self.doctype_eof(),
                    Some(_) => {
                        self.doctype.force_quirks = true;
                        self.back(c);
                        self.state = S::BogusDoctype;
                    }
                }
            }
            S::BeforeDoctypePublicIdentifier => {
                let c = self.next_char();
                match c {
                    Some(ch) if is_ws(ch) => {}
                    Some('"') => {
                        self.doctype.public_id = Some(String::new());
                        self.state = S::DoctypePublicIdentifierDoubleQuoted;
                    }
                    Some('\'') => {
                        self.doctype.public_id = Some(String::new());
                        self.state = S::DoctypePublicIdentifierSingleQuoted;
                    }
                    Some('>') => self.doctype_abrupt(),
                    None => self.doctype_eof(),
                    Some(_) => {
                        self.doctype.force_quirks = true;
                        self.back(c);
                        self.state = S::BogusDoctype;
                    }
                }
            }
            S::DoctypePublicIdentifierDoubleQuoted => self.doctype_id(b'"', true),
            S::DoctypePublicIdentifierSingleQuoted => self.doctype_id(b'\'', true),
            S::AfterDoctypePublicIdentifier => {
                let c = self.next_char();
                match c {
                    Some(ch) if is_ws(ch) => self.state = S::BetweenDoctypePublicAndSystemIdentifiers,
                    Some('>') => {
                        self.state = S::Data;
                        self.emit_doctype();
                    }
                    Some('"') => {
                        self.doctype.system_id = Some(String::new());
                        self.state = S::DoctypeSystemIdentifierDoubleQuoted;
                    }
                    Some('\'') => {
                        self.doctype.system_id = Some(String::new());
                        self.state = S::DoctypeSystemIdentifierSingleQuoted;
                    }
                    None => self.doctype_eof(),
                    Some(_) => {
                        self.doctype.force_quirks = true;
                        self.back(c);
                        self.state = S::BogusDoctype;
                    }
                }
            }
            S::BetweenDoctypePublicAndSystemIdentifiers => {
                let c = self.next_char();
                match c {
                    Some(ch) if is_ws(ch) => {}
                    Some('>') => {
                        self.state = S::Data;
                        self.emit_doctype();
                    }
                    Some('"') => {
                        self.doctype.system_id = Some(String::new());
                        self.state = S::DoctypeSystemIdentifierDoubleQuoted;
                    }
                    Some('\'') => {
                        self.doctype.system_id = Some(String::new());
                        self.state = S::DoctypeSystemIdentifierSingleQuoted;
                    }
                    None => self.doctype_eof(),
                    Some(_) => {
                        self.doctype.force_quirks = true;
                        self.back(c);
                        self.state = S::BogusDoctype;
                    }
                }
            }
            S::AfterDoctypeSystemKeyword => {
                let c = self.next_char();
                match c {
                    Some(ch) if is_ws(ch) => self.state = S::BeforeDoctypeSystemIdentifier,
                    Some('"') => {
                        self.doctype.system_id = Some(String::new());
                        self.state = S::DoctypeSystemIdentifierDoubleQuoted;
                    }
                    Some('\'') => {
                        self.doctype.system_id = Some(String::new());
                        self.state = S::DoctypeSystemIdentifierSingleQuoted;
                    }
                    Some('>') => self.doctype_abrupt(),
                    None => self.doctype_eof(),
                    Some(_) => {
                        self.doctype.force_quirks = true;
                        self.back(c);
                        self.state = S::BogusDoctype;
                    }
                }
            }
            S::BeforeDoctypeSystemIdentifier => {
                let c = self.next_char();
                match c {
                    Some(ch) if is_ws(ch) => {}
                    Some('"') => {
                        self.doctype.system_id = Some(String::new());
                        self.state = S::DoctypeSystemIdentifierDoubleQuoted;
                    }
                    Some('\'') => {
                        self.doctype.system_id = Some(String::new());
                        self.state = S::DoctypeSystemIdentifierSingleQuoted;
                    }
                    Some('>') => self.doctype_abrupt(),
                    None => self.doctype_eof(),
                    Some(_) => {
                        self.doctype.force_quirks = true;
                        self.back(c);
                        self.state = S::BogusDoctype;
                    }
                }
            }
            S::DoctypeSystemIdentifierDoubleQuoted => self.doctype_id(b'"', false),
            S::DoctypeSystemIdentifierSingleQuoted => self.doctype_id(b'\'', false),
            S::AfterDoctypeSystemIdentifier => {
                let c = self.next_char();
                match c {
                    Some(ch) if is_ws(ch) => {}
                    Some('>') => {
                        self.state = S::Data;
                        self.emit_doctype();
                    }
                    None => self.doctype_eof(),
                    Some(_) => {
                        // unexpected-character-after-doctype-system-identifier: does not set force-quirks.
                        self.back(c);
                        self.state = S::BogusDoctype;
                    }
                }
            }
            S::BogusDoctype => match self.next_char() {
                Some('>') => {
                    self.state = S::Data;
                    self.emit_doctype();
                }
                Some(_) => {}
                None => {
                    self.emit_doctype();
                    self.emit_eof();
                }
            },
            // 13.2.5.72
            S::ProcessingInstructionOpen => {
                let c = self.next_char();
                match c {
                    Some(ch) if ch.is_ascii_alphabetic() || ch == '_' => {
                        self.back(c);
                        self.state = S::ProcessingInstructionTarget;
                    }
                    None => self.emit_eof(),
                    Some(_) => {
                        self.temp_to_comment();
                        self.back(c);
                        self.state = S::BogusComment;
                    }
                }
            }
            S::ProcessingInstructionTarget => {
                let c = self.next_char();
                match c {
                    Some(ch) if is_ws(ch) || ch == '?' || ch == '>' => {
                        if self.temp.eq_ignore_ascii_case("xml") || self.temp.eq_ignore_ascii_case("xml-stylesheet") {
                            self.temp_to_comment();
                            self.back(c);
                            self.state = S::BogusComment;
                        } else {
                            self.pi_target = std::mem::take(&mut self.temp);
                            self.pi_data.clear();
                            self.back(c);
                            self.state = S::AfterProcessingInstructionTarget;
                        }
                    }
                    Some(ch) if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' => self.temp.push(ch),
                    None => self.emit_eof(),
                    Some(_) => {
                        self.temp_to_comment();
                        self.back(c);
                        self.state = S::BogusComment;
                    }
                }
            }
            S::AfterProcessingInstructionTarget => {
                let c = self.next_char();
                match c {
                    Some(ch) if is_ws(ch) => {}
                    _ => {
                        self.back(c);
                        self.state = S::ProcessingInstructionData;
                    }
                }
            }
            S::ProcessingInstructionData => match self.next_char() {
                Some('?') => self.state = S::ProcessingInstructionQuestionable,
                Some('>') => {
                    self.state = S::Data;
                    self.emit_pi();
                }
                Some(c) => self.pi_data.push(c),
                None => self.emit_eof(),
            },
            S::ProcessingInstructionQuestionable => {
                let c = self.next_char();
                match c {
                    Some('>') => {
                        self.state = S::Data;
                        self.emit_pi();
                    }
                    None => self.emit_eof(),
                    Some(_) => {
                        self.pi_data.push('?');
                        self.back(c);
                        self.state = S::ProcessingInstructionData;
                    }
                }
            }
            // 13.2.5.69
            S::CdataSection => {
                let n = self.run_len(&STOP_CDATA);
                if n > 0 {
                    if let Some(s) = self.input.get(self.pos..self.pos + n) {
                        self.text.push_str(s);
                    }
                    self.pos += n;
                }
                match self.next_char() {
                    Some(']') => self.state = S::CdataSectionBracket,
                    Some(c) => self.emit_char(c),
                    None => self.emit_eof(),
                }
            }
            S::CdataSectionBracket => {
                let c = self.next_char();
                if c == Some(']') {
                    self.state = S::CdataSectionEnd;
                } else {
                    self.emit_char(']');
                    self.back(c);
                    self.state = S::CdataSection;
                }
            }
            S::CdataSectionEnd => {
                let c = self.next_char();
                match c {
                    Some(']') => self.emit_char(']'),
                    Some('>') => self.state = S::Data,
                    _ => {
                        self.emit_str("]]");
                        self.back(c);
                        self.state = S::CdataSection;
                    }
                }
            }
        }
    }

    /// RAWTEXT / script data states: only `<` and NUL are special.
    fn raw_like(&mut self, lt_state: State) {
        let n = self.run_len(&STOP_RAW);
        if n > 0 {
            if let Some(s) = self.input.get(self.pos..self.pos + n) {
                self.text.push_str(s);
            }
            self.pos += n;
        }
        match self.next_char() {
            Some('<') => self.state = lt_state,
            Some('\0') => self.emit_char('\u{FFFD}'),
            Some(c) => self.emit_char(c),
            None => self.emit_eof(),
        }
    }

    /// RCDATA/RAWTEXT "less-than sign" states (13.2.5.9, 13.2.5.12).
    fn lt_state(&mut self, end_open: State, fallback: State, _rcdata_like: bool) {
        let c = self.next_char();
        if c == Some('/') {
            self.temp.clear();
            self.state = end_open;
        } else {
            self.emit_char('<');
            self.back(c);
            self.state = fallback;
        }
    }

    /// "... end tag open state" for RCDATA, RAWTEXT, script data and script data escaped.
    fn end_tag_open(&mut self, name_state: State, fallback: State) {
        let c = self.next_char();
        match c {
            Some(ch) if ch.is_ascii_alphabetic() => {
                self.new_tag(true);
                self.back(c);
                self.state = name_state;
            }
            _ => {
                self.emit_str("</");
                self.back(c);
                self.state = fallback;
            }
        }
    }

    /// "... end tag name state" for RCDATA, RAWTEXT, script data and script data escaped.
    fn end_tag_name(&mut self, fallback: State, atoms: &mut Interner) {
        let c = self.next_char();
        let appropriate = self.appropriate_end_tag();
        match c {
            Some(ch) if is_ws(ch) && appropriate => self.state = State::BeforeAttributeName,
            Some('/') if appropriate => self.state = State::SelfClosingStartTag,
            Some('>') if appropriate => {
                self.state = State::Data;
                self.emit_tag(atoms);
            }
            Some(ch) if ch.is_ascii_alphabetic() => {
                self.tag_name.push(ch.to_ascii_lowercase());
                self.temp.push(ch);
            }
            _ => {
                self.emit_str("</");
                let t = std::mem::take(&mut self.temp);
                self.emit_str(&t);
                self.back(c);
                self.state = fallback;
            }
        }
    }

    fn quoted_value(&mut self, quote: u8) {
        let n = self.run_len(if quote == b'"' { &STOP_DQ } else { &STOP_SQ });
        if n > 0 {
            if let Some(s) = self.input.get(self.pos..self.pos + n) {
                self.attr_value.push_str(s);
            }
            self.pos += n;
        }
        match self.next_char() {
            Some(c) if c as u32 == quote as u32 => self.state = State::AfterAttributeValueQuoted,
            Some('&') => self.char_ref(true),
            Some('\0') => self.attr_value_push('\u{FFFD}'),
            Some(c) => self.attr_value_push(c),
            None => self.emit_eof(),
        }
    }

    fn doctype_eof(&mut self) {
        self.doctype.force_quirks = true;
        self.emit_doctype();
        self.emit_eof();
    }

    /// `>` where an identifier was expected: force-quirks, back to data, emit.
    fn doctype_abrupt(&mut self) {
        self.doctype.force_quirks = true;
        self.state = State::Data;
        self.emit_doctype();
    }

    fn doctype_id(&mut self, quote: u8, public: bool) {
        let c = self.next_char();
        let target = if public { &mut self.doctype.public_id } else { &mut self.doctype.system_id };
        match c {
            Some(ch) if ch as u32 == quote as u32 => {
                self.state = if public { State::AfterDoctypePublicIdentifier } else { State::AfterDoctypeSystemIdentifier };
            }
            Some('\0') => target.get_or_insert_with(String::new).push('\u{FFFD}'),
            Some('>') => self.doctype_abrupt(),
            Some(ch) => target.get_or_insert_with(String::new).push(ch),
            None => self.doctype_eof(),
        }
    }
}
