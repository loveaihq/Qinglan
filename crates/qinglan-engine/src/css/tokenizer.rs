//! CSS tokenizer: css-syntax-3 section 4 "Tokenization".

#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    Ident(String),
    Function(String),
    AtKeyword(String),
    Hash { value: String, is_id: bool },
    Str(String),
    BadStr,
    Url(String),
    BadUrl,
    Delim(char),
    Number { value: f64, is_int: bool, signed: bool },
    Percentage { value: f64 },
    Dimension { value: f64, is_int: bool, signed: bool, unit: String },
    Whitespace,
    Cdo,
    Cdc,
    Colon,
    Semicolon,
    Comma,
    OpenSquare,
    CloseSquare,
    OpenParen,
    CloseParen,
    OpenCurly,
    CloseCurly,
}

/// The tokenizer works on the bytes of the text (UTF-8). Every byte of a non-ASCII character is a name character
/// (css-syntax-3 4.2 "ident-start code point"), so only the places that build text need to care about characters.
pub(super) struct Lexer<'a> {
    src: &'a str,
    b: &'a [u8],
    pos: usize,
    /// Skimming: tokens are found exactly as in normal mode (so their boundaries are the same) but their text is not
    /// kept, which makes it allocation free. Used to find the end of a block that is only tokenized if it is needed.
    pub(super) skim: bool,
}

fn is_name_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_' || !c.is_ascii()
}

fn is_name(c: char) -> bool {
    is_name_start(c) || c.is_ascii_digit() || c == '-'
}

fn is_newline(c: char) -> bool {
    c == '\n'
}

fn is_ws(c: char) -> bool {
    matches!(c, '\n' | '\t' | ' ')
}

fn is_non_printable(c: char) -> bool {
    matches!(c, '\0'..='\u{8}' | '\u{B}' | '\u{E}'..='\u{1F}' | '\u{7F}')
}

fn text_of(v: Vec<u8>) -> String {
    String::from_utf8(v).unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned())
}

fn push_char(v: &mut Vec<u8>, c: char) {
    let mut buf = [0u8; 4];
    v.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
}

impl<'a> Lexer<'a> {
    /// `text` must be preprocessed already (see [`normalize`]).
    pub(super) fn new(text: &'a str) -> Lexer<'a> {
        Lexer { src: text, b: text.as_bytes(), pos: 0, skim: false }
    }

    /// The byte offset in the text of what was consumed so far.
    pub(super) fn pos(&self) -> usize {
        self.pos
    }

    /// Go back (or on) to an offset that `pos` gave.
    pub(super) fn set_pos(&mut self, pos: usize) {
        self.pos = pos;
    }

    /// The code point at `pos + off`; any byte of a non-ASCII character reads as U+0080 (they are all alike here).
    fn at(&self, off: usize) -> Option<char> {
        self.b.get(self.pos + off).map(|&x| if x < 0x80 { x as char } else { '\u{80}' })
    }

    /// "Check if two code points are a valid escape" starting at offset `off`.
    fn valid_escape_at(&self, off: usize) -> bool {
        self.at(off) == Some('\\') && self.at(off + 1).is_some_and(|c| !is_newline(c))
    }

    /// "Check if three code points would start an ident sequence" at offset `off`.
    fn would_start_ident_at(&self, off: usize) -> bool {
        match self.at(off) {
            Some('-') => {
                self.at(off + 1).is_some_and(|c| is_name_start(c) || c == '-') || self.valid_escape_at(off + 1)
            }
            Some(c) if is_name_start(c) => true,
            Some('\\') => self.valid_escape_at(off),
            _ => false,
        }
    }

    /// "Check if three code points would start a number" at offset `off`.
    fn would_start_number_at(&self, off: usize) -> bool {
        match self.at(off) {
            Some('+' | '-') => match self.at(off + 1) {
                Some(c) if c.is_ascii_digit() => true,
                Some('.') => self.at(off + 2).is_some_and(|c| c.is_ascii_digit()),
                _ => false,
            },
            Some('.') => self.at(off + 1).is_some_and(|c| c.is_ascii_digit()),
            Some(c) => c.is_ascii_digit(),
            None => false,
        }
    }

    fn consume_comments(&mut self) {
        while self.at(0) == Some('/') && self.at(1) == Some('*') {
            self.pos += 2;
            loop {
                match self.at(0) {
                    None => return,
                    Some('*') if self.at(1) == Some('/') => {
                        self.pos += 2;
                        break;
                    }
                    _ => self.pos += 1,
                }
            }
        }
    }

    /// "Consume an escaped code point" (the backslash is already consumed).
    fn consume_escape(&mut self) -> char {
        match self.at(0) {
            None => '\u{FFFD}',
            Some(c) if c.is_ascii_hexdigit() => {
                let mut v: u32 = 0;
                let mut n = 0;
                while n < 6 {
                    match self.at(0).and_then(|c| c.to_digit(16)) {
                        Some(d) => {
                            v = v * 16 + d;
                            self.pos += 1;
                            n += 1;
                        }
                        None => break,
                    }
                }
                if self.at(0).is_some_and(is_ws) {
                    self.pos += 1;
                }
                if v == 0 || v > 0x10FFFF || (0xD800..=0xDFFF).contains(&v) {
                    '\u{FFFD}'
                } else {
                    char::from_u32(v).unwrap_or('\u{FFFD}')
                }
            }
            Some(_) => {
                // Any other code point is itself; a non-ASCII one is several bytes (`pos` is on a boundary here).
                let c = self.src.get(self.pos..).and_then(|r| r.chars().next()).unwrap_or('\u{FFFD}');
                self.pos += c.len_utf8();
                c
            }
        }
    }

    /// "Consume an ident sequence".
    fn consume_name(&mut self) -> String {
        // Fast path: a run of name characters, no escape: one exactly sized string.
        let start = self.pos;
        let mut end = start;
        while self.b.get(end).is_some_and(|&x| is_name(if x < 0x80 { x as char } else { '\u{80}' })) {
            end += 1;
        }
        if self.b.get(end) != Some(&b'\\') {
            self.pos = end;
            if self.skim {
                return String::new();
            }
            return String::from_utf8_lossy(self.b.get(start..end).unwrap_or(&[])).into_owned();
        }
        let mut s: Vec<u8> = Vec::with_capacity(16);
        loop {
            match self.at(0) {
                Some(c) if is_name(c) => {
                    if let Some(&x) = self.b.get(self.pos) {
                        s.push(x);
                    }
                    self.pos += 1;
                }
                Some('\\') if self.valid_escape_at(0) => {
                    self.pos += 1;
                    let c = self.consume_escape();
                    push_char(&mut s, c);
                }
                _ => return text_of(s),
            }
        }
    }

    /// "Consume a number": returns (value, is_integer, has explicit sign).
    fn consume_number(&mut self) -> (f64, bool, bool) {
        // Fast path: an unsigned integer of at most 9 digits that is not followed by `.` or `e`.
        let mut n = 0usize;
        let mut v: u64 = 0;
        while let Some(d) = self.at(n).and_then(|c| c.to_digit(10)) {
            v = v * 10 + d as u64;
            n += 1;
            if n > 9 {
                break;
            }
        }
        let exp = matches!(self.at(n), Some('e' | 'E'))
            && (self.at(n + 1).is_some_and(|c| c.is_ascii_digit())
                || (matches!(self.at(n + 1), Some('+' | '-')) && self.at(n + 2).is_some_and(|c| c.is_ascii_digit())));
        let frac = self.at(n) == Some('.') && self.at(n + 1).is_some_and(|c| c.is_ascii_digit());
        if n > 0 && n <= 9 && !exp && !frac {
            self.pos += n;
            return (v as f64, true, false);
        }
        let mut repr = String::new();
        let mut is_int = true;
        let mut signed = false;
        if let Some(c @ ('+' | '-')) = self.at(0) {
            repr.push(c);
            signed = true;
            self.pos += 1;
        }
        while let Some(c) = self.at(0).filter(char::is_ascii_digit) {
            repr.push(c);
            self.pos += 1;
        }
        if self.at(0) == Some('.') && self.at(1).is_some_and(|c| c.is_ascii_digit()) {
            repr.push('.');
            self.pos += 1;
            is_int = false;
            while let Some(c) = self.at(0).filter(char::is_ascii_digit) {
                repr.push(c);
                self.pos += 1;
            }
        }
        if matches!(self.at(0), Some('e' | 'E')) {
            let sign = matches!(self.at(1), Some('+' | '-'));
            let digit_off = if sign { 2 } else { 1 };
            if self.at(digit_off).is_some_and(|c| c.is_ascii_digit()) {
                repr.push('e');
                self.pos += 1;
                if sign && let Some(c) = self.at(0) {
                    repr.push(c);
                    self.pos += 1;
                }
                is_int = false;
                while let Some(c) = self.at(0).filter(char::is_ascii_digit) {
                    repr.push(c);
                    self.pos += 1;
                }
            }
        }
        let value: f64 = repr.parse().unwrap_or(0.0);
        let value = if value.is_finite() { value } else if value > 0.0 { f64::MAX } else { f64::MIN };
        (value, is_int, signed)
    }

    fn consume_numeric(&mut self) -> Token {
        let (value, is_int, signed) = self.consume_number();
        if self.would_start_ident_at(0) {
            let unit = self.consume_name();
            Token::Dimension { value, is_int, signed, unit }
        } else if self.at(0) == Some('%') {
            self.pos += 1;
            Token::Percentage { value }
        } else {
            Token::Number { value, is_int, signed }
        }
    }

    fn consume_string(&mut self, ending: char) -> Token {
        let mut s: Vec<u8> = Vec::new();
        loop {
            match self.at(0) {
                None => return Token::Str(text_of(s)),
                Some(c) if c == ending => {
                    self.pos += 1;
                    return Token::Str(text_of(s));
                }
                Some('\n') => return Token::BadStr, // not consumed
                Some('\\') => {
                    self.pos += 1;
                    match self.at(0) {
                        None => {}
                        Some('\n') => self.pos += 1,
                        Some(_) => {
                            let c = self.consume_escape();
                            if !self.skim {
                                push_char(&mut s, c);
                            }
                        }
                    }
                }
                Some(_) => {
                    if !self.skim
                        && let Some(&x) = self.b.get(self.pos)
                    {
                        s.push(x);
                    }
                    self.pos += 1;
                }
            }
        }
    }

    fn consume_bad_url_remnants(&mut self) {
        loop {
            match self.at(0) {
                None => return,
                Some(')') => {
                    self.pos += 1;
                    return;
                }
                Some('\\') if self.valid_escape_at(0) => {
                    self.pos += 1;
                    self.consume_escape();
                }
                _ => self.pos += 1,
            }
        }
    }

    /// "Consume a url token" (the `url(` is consumed and the next is not a quote).
    fn consume_url(&mut self) -> Token {
        while self.at(0).is_some_and(is_ws) {
            self.pos += 1;
        }
        let mut s: Vec<u8> = Vec::new();
        loop {
            match self.at(0) {
                None => return Token::Url(text_of(s)),
                Some(')') => {
                    self.pos += 1;
                    return Token::Url(text_of(s));
                }
                Some(c) if is_ws(c) => {
                    while self.at(0).is_some_and(is_ws) {
                        self.pos += 1;
                    }
                    return match self.at(0) {
                        None => Token::Url(text_of(s)),
                        Some(')') => {
                            self.pos += 1;
                            Token::Url(text_of(s))
                        }
                        _ => {
                            self.consume_bad_url_remnants();
                            Token::BadUrl
                        }
                    };
                }
                Some('"' | '\'' | '(') => {
                    self.consume_bad_url_remnants();
                    return Token::BadUrl;
                }
                Some(c) if is_non_printable(c) => {
                    self.consume_bad_url_remnants();
                    return Token::BadUrl;
                }
                Some('\\') => {
                    if self.valid_escape_at(0) {
                        self.pos += 1;
                        let c = self.consume_escape();
                        if !self.skim {
                            push_char(&mut s, c);
                        }
                    } else {
                        self.consume_bad_url_remnants();
                        return Token::BadUrl;
                    }
                }
                Some(_) => {
                    if !self.skim
                        && let Some(&x) = self.b.get(self.pos)
                    {
                        s.push(x);
                    }
                    self.pos += 1;
                }
            }
        }
    }

    /// "Consume an ident-like token".
    fn consume_ident_like(&mut self) -> Token {
        let start = self.pos;
        let name = self.consume_name();
        if self.at(0) == Some('(') {
            // Skimming leaves a name without escapes empty (a name is never empty otherwise); the text is still there.
            let is_url = if name.is_empty() { self.b.get(start..self.pos).is_some_and(|n| n.eq_ignore_ascii_case(b"url")) } else { name.eq_ignore_ascii_case("url") };
            self.pos += 1;
            if is_url {
                // If the next one or two non-whitespace characters are quotes, it is a function token.
                let mut off = 0;
                while self.at(off).is_some_and(is_ws) {
                    off += 1;
                }
                if matches!(self.at(off), Some('"' | '\'')) {
                    return Token::Function(name);
                }
                return self.consume_url();
            }
            return Token::Function(name);
        }
        Token::Ident(name)
    }

    pub(super) fn next_token(&mut self) -> Option<Token> {
        self.consume_comments();
        let c = self.at(0)?;
        let t = match c {
            c if is_ws(c) => {
                while self.at(0).is_some_and(is_ws) {
                    self.pos += 1;
                }
                Token::Whitespace
            }
            '"' | '\'' => {
                self.pos += 1;
                self.consume_string(c)
            }
            '#' => {
                self.pos += 1;
                if self.at(0).is_some_and(is_name) || self.valid_escape_at(0) {
                    let is_id = self.would_start_ident_at(0);
                    let value = self.consume_name();
                    Token::Hash { value, is_id }
                } else {
                    Token::Delim('#')
                }
            }
            '(' => {
                self.pos += 1;
                Token::OpenParen
            }
            ')' => {
                self.pos += 1;
                Token::CloseParen
            }
            '+' => {
                if self.would_start_number_at(0) {
                    self.consume_numeric()
                } else {
                    self.pos += 1;
                    Token::Delim('+')
                }
            }
            ',' => {
                self.pos += 1;
                Token::Comma
            }
            '-' => {
                if self.would_start_number_at(0) {
                    self.consume_numeric()
                } else if self.at(1) == Some('-') && self.at(2) == Some('>') {
                    self.pos += 3;
                    Token::Cdc
                } else if self.would_start_ident_at(0) {
                    self.consume_ident_like()
                } else {
                    self.pos += 1;
                    Token::Delim('-')
                }
            }
            '.' => {
                if self.would_start_number_at(0) {
                    self.consume_numeric()
                } else {
                    self.pos += 1;
                    Token::Delim('.')
                }
            }
            ':' => {
                self.pos += 1;
                Token::Colon
            }
            ';' => {
                self.pos += 1;
                Token::Semicolon
            }
            '<' => {
                if self.at(1) == Some('!') && self.at(2) == Some('-') && self.at(3) == Some('-') {
                    self.pos += 4;
                    Token::Cdo
                } else {
                    self.pos += 1;
                    Token::Delim('<')
                }
            }
            '@' => {
                self.pos += 1;
                if self.would_start_ident_at(0) {
                    Token::AtKeyword(self.consume_name())
                } else {
                    Token::Delim('@')
                }
            }
            '[' => {
                self.pos += 1;
                Token::OpenSquare
            }
            '\\' => {
                if self.valid_escape_at(0) {
                    self.consume_ident_like()
                } else {
                    self.pos += 1;
                    Token::Delim('\\')
                }
            }
            ']' => {
                self.pos += 1;
                Token::CloseSquare
            }
            '{' => {
                self.pos += 1;
                Token::OpenCurly
            }
            '}' => {
                self.pos += 1;
                Token::CloseCurly
            }
            c if c.is_ascii_digit() => self.consume_numeric(),
            c if is_name_start(c) => self.consume_ident_like(),
            c => {
                self.pos += 1;
                Token::Delim(c)
            }
        };
        Some(t)
    }
}

/// Input preprocessing (css-syntax-3 3.3): CR, FF and CRLF become LF, NUL becomes U+FFFD. Text without them is
/// used as it is.
pub fn normalize(input: &str) -> std::borrow::Cow<'_, str> {
    if !input.bytes().any(|b| matches!(b, b'\r' | 0x0c | 0)) {
        return std::borrow::Cow::Borrowed(input);
    }
    let mut out = String::with_capacity(input.len());
    let mut it = input.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '\r' => {
                if it.peek() == Some(&'\n') {
                    it.next();
                }
                out.push('\n');
            }
            '\x0c' => out.push('\n'),
            '\0' => out.push('\u{FFFD}'),
            c => out.push(c),
        }
    }
    std::borrow::Cow::Owned(out)
}

/// Tokenize a style sheet or a declaration list.
pub fn tokenize(input: &str) -> Vec<Token> {
    tokenize_normalized(&normalize(input))
}

/// [`tokenize`] for text that [`normalize`] has been applied to.
pub fn tokenize_normalized(text: &str) -> Vec<Token> {
    let mut lx = Lexer::new(text);
    let mut out = Vec::with_capacity(text.len() / 6 + 16);
    while let Some(t) = lx.next_token() {
        out.push(t);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basics() {
        let t = tokenize("a:hover { color: #fff; margin: -1.5px 2em 10% }");
        assert_eq!(t.first(), Some(&Token::Ident("a".into())));
        assert!(t.contains(&Token::Hash { value: "fff".into(), is_id: true }));
        assert!(t.contains(&Token::Dimension { value: -1.5, is_int: false, signed: true, unit: "px".into() }));
        assert!(t.contains(&Token::Percentage { value: 10.0 }));
    }

    #[test]
    fn escapes_and_strings() {
        assert_eq!(tokenize("\\31 23"), vec![Token::Ident("123".into())]);
        assert_eq!(tokenize("'a\\\nb'"), vec![Token::Str("ab".into())]);
        assert_eq!(tokenize("'a\nb'")[0], Token::BadStr);
    }

    #[test]
    fn urls_and_functions() {
        assert_eq!(tokenize("url( a.png )"), vec![Token::Url("a.png".into())]);
        assert_eq!(tokenize("url('a')")[0], Token::Function("url".into()));
        assert_eq!(tokenize("url(a b)"), vec![Token::BadUrl]);
        assert_eq!(tokenize("rgb(")[0], Token::Function("rgb".into()));
    }

    #[test]
    fn numbers() {
        assert_eq!(tokenize("1e3"), vec![Token::Number { value: 1000.0, is_int: false, signed: false }]);
        assert_eq!(tokenize("+5"), vec![Token::Number { value: 5.0, is_int: true, signed: true }]);
        assert_eq!(tokenize("2n+1").len(), 2);
        assert_eq!(tokenize("<!-- -->"), vec![Token::Cdo, Token::Whitespace, Token::Cdc]);
    }

    #[test]
    fn non_ascii_text_and_input_preprocessing() {
        assert_eq!(tokenize(".类名é{}")[1], Token::Ident("类名é".into()));
        assert_eq!(tokenize("'日本\\語x'"), vec![Token::Str("日本語x".into())]);
        assert_eq!(tokenize("\\é"), vec![Token::Ident("é".into())]);
        assert_eq!(tokenize("url(\\é.png)"), vec![Token::Url("é.png".into())]);
        assert_eq!(
            tokenize("a\r\nb\rc\x0cd"),
            vec![Token::Ident("a".into()), Token::Whitespace, Token::Ident("b".into()), Token::Whitespace, Token::Ident("c".into()), Token::Whitespace, Token::Ident("d".into())]
        );
        assert_eq!(tokenize("a\0b"), vec![Token::Ident("a\u{FFFD}b".into())]);
    }

    #[test]
    fn skimming_finds_the_same_token_boundaries() {
        let text = "a{b:url( x\\)y ) \\75rl(\"z\") 'q\\'}' é\\{ /* } */ 1.5e3px 0x}\\}} [a=\"}\"] url(\"}\" c{d}} u\\rl(}) end";
        let mut full = Lexer::new(text);
        let mut skim = Lexer::new(text);
        skim.skim = true;
        loop {
            let (a, b) = (full.next_token(), skim.next_token());
            assert_eq!(a.is_some(), b.is_some());
            assert_eq!(full.pos(), skim.pos());
            let (Some(a), Some(b)) = (a, b) else { break };
            assert_eq!(std::mem::discriminant(&a), std::mem::discriminant(&b));
        }
    }
}
