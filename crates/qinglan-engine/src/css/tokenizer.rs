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

struct Lexer {
    chars: Vec<char>,
    pos: usize,
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

impl Lexer {
    fn at(&self, off: usize) -> Option<char> {
        self.chars.get(self.pos + off).copied()
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
            Some(c) => {
                self.pos += 1;
                c
            }
        }
    }

    /// "Consume an ident sequence".
    fn consume_name(&mut self) -> String {
        let mut s = String::with_capacity(16);
        loop {
            match self.at(0) {
                Some(c) if is_name(c) => {
                    s.push(c);
                    self.pos += 1;
                }
                Some('\\') if self.valid_escape_at(0) => {
                    self.pos += 1;
                    s.push(self.consume_escape());
                }
                _ => return s,
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
        let mut s = String::new();
        loop {
            match self.at(0) {
                None => return Token::Str(s),
                Some(c) if c == ending => {
                    self.pos += 1;
                    return Token::Str(s);
                }
                Some('\n') => return Token::BadStr, // not consumed
                Some('\\') => {
                    self.pos += 1;
                    match self.at(0) {
                        None => {}
                        Some('\n') => self.pos += 1,
                        Some(_) => s.push(self.consume_escape()),
                    }
                }
                Some(c) => {
                    s.push(c);
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
        let mut s = String::new();
        loop {
            match self.at(0) {
                None => return Token::Url(s),
                Some(')') => {
                    self.pos += 1;
                    return Token::Url(s);
                }
                Some(c) if is_ws(c) => {
                    while self.at(0).is_some_and(is_ws) {
                        self.pos += 1;
                    }
                    return match self.at(0) {
                        None => Token::Url(s),
                        Some(')') => {
                            self.pos += 1;
                            Token::Url(s)
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
                        s.push(self.consume_escape());
                    } else {
                        self.consume_bad_url_remnants();
                        return Token::BadUrl;
                    }
                }
                Some(c) => {
                    s.push(c);
                    self.pos += 1;
                }
            }
        }
    }

    /// "Consume an ident-like token".
    fn consume_ident_like(&mut self) -> Token {
        let name = self.consume_name();
        if self.at(0) == Some('(') {
            self.pos += 1;
            if name.eq_ignore_ascii_case("url") {
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

    fn next_token(&mut self) -> Option<Token> {
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

/// Tokenize a style sheet or a declaration list. Input preprocessing (css-syntax-3 3.3): CR, FF and
/// CRLF become LF, NUL becomes U+FFFD.
pub fn tokenize(input: &str) -> Vec<Token> {
    let mut chars: Vec<char> = Vec::with_capacity(input.len());
    let mut it = input.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '\r' => {
                if it.peek() == Some(&'\n') {
                    it.next();
                }
                chars.push('\n');
            }
            '\x0c' => chars.push('\n'),
            '\0' => chars.push('\u{FFFD}'),
            c => chars.push(c),
        }
    }
    let mut lx = Lexer { chars, pos: 0 };
    let mut out = Vec::new();
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
}
