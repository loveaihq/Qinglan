//! WHATWG URL Standard: URL records, the basic URL parser, host parsing and serialization.
//!
//! Spec: <https://url.spec.whatwg.org/> (section numbers below refer to it).
//! Deviations (see docs/decisions.md): the IDNA step of the domain parser (3.3) only
//! lower-cases, folds a few characters and applies Punycode instead of full UTS #46;
//! the query encoding is always UTF-8.

use std::fmt::Write as _;

/// A URL record (4.1). Hosts, paths and so on are already in their serialized-ready form.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Url {
    pub scheme: String,
    pub username: String,
    pub password: String,
    pub host: Option<Host>,
    pub port: Option<u16>,
    pub path: UrlPath,
    pub query: Option<String>,
    pub fragment: Option<String>,
}

/// Host (3.1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Host {
    Domain(String),
    Ipv4(u32),
    Ipv6([u16; 8]),
    Opaque(String),
    Empty,
}

/// URL path (4.1): a list of segments, or one opaque string.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UrlPath {
    Opaque(String),
    Segments(Vec<String>),
}

/// The only error the parser reports: "failure" (validation errors are not collected).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UrlError;

// ---------------------------------------------------------------------------------------------
// Percent-encoding (1.3, 1.2 "percent-encode sets")

#[derive(Clone, Copy, PartialEq, Eq)]
enum EncodeSet {
    C0Control,
    Fragment,
    Query,
    SpecialQuery,
    Path,
    Userinfo,
}

fn in_set(set: EncodeSet, c: char) -> bool {
    let cp = c as u32;
    // C0 control percent-encode set: C0 controls and everything above U+007E.
    if cp <= 0x1F || cp > 0x7E {
        return true;
    }
    let fragment = matches!(c, ' ' | '"' | '<' | '>' | '`');
    let query = matches!(c, ' ' | '"' | '#' | '<' | '>');
    let path = query || matches!(c, '?' | '^' | '`' | '{' | '}');
    match set {
        EncodeSet::C0Control => false,
        EncodeSet::Fragment => fragment,
        EncodeSet::Query => query,
        EncodeSet::SpecialQuery => query || c == '\'',
        EncodeSet::Path => path,
        EncodeSet::Userinfo => {
            path || matches!(c, '/' | ':' | ';' | '=' | '@' | '[' | '\\' | ']' | '|')
        }
    }
}

fn percent_encode_char(c: char, set: EncodeSet, out: &mut String) {
    if in_set(set, c) {
        let mut buf = [0u8; 4];
        for b in c.encode_utf8(&mut buf).bytes() {
            let _ = write!(out, "%{b:02X}");
        }
    } else {
        out.push(c);
    }
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Percent-decode (1.3).
pub fn percent_decode(input: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len());
    let mut i = 0;
    while let Some(&b) = input.get(i) {
        if b == b'%'
            && let (Some(h), Some(l)) = (
                input.get(i + 1).and_then(|&x| hex_val(x)),
                input.get(i + 2).and_then(|&x| hex_val(x)),
            )
        {
            out.push(h * 16 + l);
            i += 3;
            continue;
        }
        out.push(b);
        i += 1;
    }
    out
}

// ---------------------------------------------------------------------------------------------
// Schemes

fn default_port(scheme: &str) -> Option<u16> {
    match scheme {
        "ftp" => Some(21),
        "http" | "ws" => Some(80),
        "https" | "wss" => Some(443),
        _ => None,
    }
}

fn is_special_scheme(s: &str) -> bool {
    matches!(s, "ftp" | "file" | "http" | "https" | "ws" | "wss")
}

// ---------------------------------------------------------------------------------------------
// Hosts (3.5, 3.6)

fn is_forbidden_host_code_point(c: char) -> bool {
    matches!(
        c,
        '\0' | '\t' | '\n' | '\r' | ' ' | '#' | '/' | ':' | '<' | '>' | '?' | '@' | '[' | '\\' | ']' | '^' | '|'
    )
}

fn is_forbidden_domain_code_point(c: char) -> bool {
    is_forbidden_host_code_point(c) || (c as u32) <= 0x1F || c == '%' || c == '\u{7F}'
}

/// Host parser (3.5).
fn parse_host(input: &str, is_opaque: bool) -> Result<Host, UrlError> {
    if let Some(rest) = input.strip_prefix('[') {
        let inner = rest.strip_suffix(']').ok_or(UrlError)?;
        return parse_ipv6(inner).map(Host::Ipv6);
    }
    if is_opaque {
        return parse_opaque_host(input);
    }
    if input.is_empty() {
        return Err(UrlError);
    }
    let decoded = percent_decode(input.as_bytes());
    let domain = String::from_utf8_lossy(&decoded).into_owned();
    let ascii_domain = domain_to_ascii(&domain)?;
    if ends_in_number(&ascii_domain) {
        return parse_ipv4(&ascii_domain).map(Host::Ipv4);
    }
    Ok(Host::Domain(ascii_domain))
}

/// Opaque-host parser (3.5).
fn parse_opaque_host(input: &str) -> Result<Host, UrlError> {
    if input.chars().any(is_forbidden_host_code_point) {
        return Err(UrlError);
    }
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        percent_encode_char(c, EncodeSet::C0Control, &mut out);
    }
    Ok(Host::Opaque(out))
}

/// Domain parser (3.3) with beStrict = false. Simplified IDNA: see the module comment.
fn domain_to_ascii(domain: &str) -> Result<String, UrlError> {
    let result = if domain.is_ascii() {
        domain.to_ascii_lowercase()
    } else {
        let mut mapped = String::with_capacity(domain.len());
        for c in domain.chars() {
            match c {
                // UTS #46 maps these to U+002E.
                '\u{3002}' | '\u{FF0E}' | '\u{FF61}' => mapped.push('.'),
                // Disallowed by UTS #46: U+FFFD (the spec notes the domain parser fails on it) and noncharacters.
                '\u{FFFD}' | '\u{FDD0}'..='\u{FDEF}' => return Err(UrlError),
                _ if (c as u32) & 0xFFFE == 0xFFFE => return Err(UrlError),
                // No-break space and ideographic space map to U+0020, a forbidden domain code point.
                '\u{A0}' | '\u{3000}' => mapped.push(' '),
                // UTS #46 "ignored" code points.
                '\u{AD}' | '\u{200B}' | '\u{2060}' | '\u{FEFF}' => {}
                // Fullwidth ASCII variants map to their ASCII counterparts (compatibility mapping).
                '\u{FF01}'..='\u{FF5E}' => {
                    if let Some(a) = char::from_u32(c as u32 - 0xFEE0) {
                        mapped.extend(a.to_lowercase());
                    }
                }
                _ => mapped.extend(c.to_lowercase()),
            }
        }
        let mut out = String::with_capacity(mapped.len() + 8);
        for (i, label) in mapped.split('.').enumerate() {
            if i > 0 {
                out.push('.');
            }
            if label.is_ascii() {
                out.push_str(label);
            } else {
                out.push_str("xn--");
                out.push_str(&punycode_encode(label).ok_or(UrlError)?);
            }
        }
        out
    };
    if result.is_empty() || result.chars().any(is_forbidden_domain_code_point) {
        return Err(UrlError);
    }
    Ok(result)
}

/// Punycode encoder, RFC 3492 section 6.3.
fn punycode_encode(input: &str) -> Option<String> {
    const BASE: u32 = 36;
    const TMIN: u32 = 1;
    const TMAX: u32 = 26;
    const SKEW: u32 = 38;
    const DAMP: u32 = 700;
    fn digit(d: u32) -> char {
        // 0..25 -> a..z, 26..35 -> 0..9
        if d < 26 {
            (b'a' + d as u8) as char
        } else {
            (b'0' + (d - 26) as u8) as char
        }
    }
    fn adapt(mut delta: u32, num_points: u32, first: bool) -> u32 {
        delta = if first { delta / DAMP } else { delta / 2 };
        delta += delta / num_points;
        let mut k = 0;
        while delta > ((BASE - TMIN) * TMAX) / 2 {
            delta /= BASE - TMIN;
            k += BASE;
        }
        k + (BASE - TMIN + 1) * delta / (delta + SKEW)
    }
    let chars: Vec<u32> = input.chars().map(|c| c as u32).collect();
    let mut out: String = input.chars().filter(char::is_ascii).collect();
    let b = out.len() as u32;
    let mut h = b;
    if b > 0 {
        out.push('-');
    }
    let mut n: u32 = 128;
    let mut delta: u32 = 0;
    let mut bias: u32 = 72;
    while (h as usize) < chars.len() {
        let m = chars.iter().copied().filter(|&c| c >= n).min()?;
        delta = delta.checked_add((m - n).checked_mul(h + 1)?)?;
        n = m;
        for &c in &chars {
            if c < n {
                delta = delta.checked_add(1)?;
            }
            if c == n {
                let mut q = delta;
                let mut k = BASE;
                loop {
                    let t = if k <= bias {
                        TMIN
                    } else if k >= bias + TMAX {
                        TMAX
                    } else {
                        k - bias
                    };
                    if q < t {
                        break;
                    }
                    out.push(digit(t + (q - t) % (BASE - t)));
                    q = (q - t) / (BASE - t);
                    k += BASE;
                }
                out.push(digit(q));
                bias = adapt(delta, h + 1, h == b);
                delta = 0;
                h += 1;
            }
        }
        delta = delta.checked_add(1)?;
        n += 1;
    }
    Some(out)
}

/// Ends-in-a-number checker (3.5).
fn ends_in_number(input: &str) -> bool {
    let mut parts: Vec<&str> = input.split('.').collect();
    if parts.last() == Some(&"") {
        if parts.len() == 1 {
            return false;
        }
        parts.pop();
    }
    let Some(last) = parts.last() else { return false };
    if !last.is_empty() && last.bytes().all(|b| b.is_ascii_digit()) {
        return true;
    }
    parse_ipv4_number(last).is_some()
}

/// IPv4 number parser (3.5). Returns the number (None on failure).
fn parse_ipv4_number(input: &str) -> Option<u64> {
    if input.is_empty() {
        return None;
    }
    let mut s = input;
    let mut radix = 10;
    if s.len() >= 2 && (s.starts_with("0x") || s.starts_with("0X")) {
        s = s.get(2..)?;
        radix = 16;
    } else if s.len() >= 2 && s.starts_with('0') {
        s = s.get(1..)?;
        radix = 8;
    }
    if s.is_empty() {
        return Some(0);
    }
    let mut value: u64 = 0;
    for b in s.bytes() {
        let d = hex_val(b)? as u32;
        if d >= radix {
            return None;
        }
        // Saturate: anything this large is out of range anyway.
        value = value.saturating_mul(radix as u64).saturating_add(d as u64);
    }
    Some(value)
}

/// IPv4 parser (3.5).
fn parse_ipv4(input: &str) -> Result<u32, UrlError> {
    let mut parts: Vec<&str> = input.split('.').collect();
    if parts.last() == Some(&"") && parts.len() > 1 {
        parts.pop();
    }
    if parts.len() > 4 {
        return Err(UrlError);
    }
    let mut numbers = Vec::with_capacity(4);
    for p in &parts {
        numbers.push(parse_ipv4_number(p).ok_or(UrlError)?);
    }
    let (last, init) = numbers.split_last().ok_or(UrlError)?;
    if init.iter().any(|&n| n > 255) {
        return Err(UrlError);
    }
    if *last >= 256u64.pow(5 - numbers.len() as u32) {
        return Err(UrlError);
    }
    let mut ipv4 = *last;
    for (counter, &n) in init.iter().enumerate() {
        ipv4 += n * 256u64.pow(3 - counter as u32);
    }
    u32::try_from(ipv4).map_err(|_| UrlError)
}

/// IPv6 parser (3.5).
fn parse_ipv6(input: &str) -> Result<[u16; 8], UrlError> {
    let c: Vec<char> = input.chars().collect();
    let mut address = [0u16; 8];
    let mut piece_index = 0usize;
    let mut compress: Option<usize> = None;
    let mut p = 0usize;
    let at = |i: usize| c.get(i).copied();
    if at(p) == Some(':') {
        if at(p + 1) != Some(':') {
            return Err(UrlError);
        }
        p += 2;
        piece_index += 1;
        compress = Some(piece_index);
    }
    while let Some(ch) = at(p) {
        if piece_index == 8 {
            return Err(UrlError);
        }
        if ch == ':' {
            if compress.is_some() {
                return Err(UrlError);
            }
            p += 1;
            piece_index += 1;
            compress = Some(piece_index);
            continue;
        }
        let mut value: u32 = 0;
        let mut length = 0;
        while length < 4 {
            match at(p).and_then(|x| x.to_digit(16)) {
                Some(d) => {
                    value = value * 16 + d;
                    p += 1;
                    length += 1;
                }
                None => break,
            }
        }
        if at(p) == Some('.') {
            if length == 0 {
                return Err(UrlError);
            }
            p -= length;
            if piece_index > 6 {
                return Err(UrlError);
            }
            let mut numbers_seen = 0;
            while let Some(ch) = at(p) {
                let mut ipv4_piece: Option<u32> = None;
                if numbers_seen > 0 {
                    if ch == '.' && numbers_seen < 4 {
                        p += 1;
                    } else {
                        return Err(UrlError);
                    }
                }
                if !at(p).is_some_and(|x| x.is_ascii_digit()) {
                    return Err(UrlError);
                }
                while let Some(d) = at(p).and_then(|x| x.to_digit(10)) {
                    match ipv4_piece {
                        None => ipv4_piece = Some(d),
                        Some(0) => return Err(UrlError),
                        Some(v) => ipv4_piece = Some(v * 10 + d),
                    }
                    if ipv4_piece.unwrap_or(0) > 255 {
                        return Err(UrlError);
                    }
                    p += 1;
                }
                let slot = address.get_mut(piece_index).ok_or(UrlError)?;
                *slot = slot.wrapping_mul(0x100).wrapping_add(ipv4_piece.unwrap_or(0) as u16);
                numbers_seen += 1;
                if numbers_seen == 2 || numbers_seen == 4 {
                    piece_index += 1;
                }
            }
            if numbers_seen != 4 {
                return Err(UrlError);
            }
            break;
        } else if at(p) == Some(':') {
            p += 1;
            if at(p).is_none() {
                return Err(UrlError);
            }
        } else if at(p).is_some() {
            return Err(UrlError);
        }
        *address.get_mut(piece_index).ok_or(UrlError)? = value as u16;
        piece_index += 1;
    }
    if let Some(compress) = compress {
        let mut swaps = piece_index - compress;
        piece_index = 7;
        while piece_index != 0 && swaps > 0 {
            let other = compress + swaps - 1;
            address.swap(piece_index, other);
            piece_index -= 1;
            swaps -= 1;
        }
    } else if piece_index != 8 {
        return Err(UrlError);
    }
    Ok(address)
}

impl Host {
    /// Host serializer (3.6).
    pub fn serialize(&self) -> String {
        match self {
            Host::Domain(s) | Host::Opaque(s) => s.clone(),
            Host::Empty => String::new(),
            Host::Ipv4(a) => {
                format!("{}.{}.{}.{}", a >> 24, (a >> 16) & 255, (a >> 8) & 255, a & 255)
            }
            Host::Ipv6(pieces) => {
                // Find the first longest run of zero pieces with length > 1.
                let (mut best_start, mut best_len) = (None, 1usize);
                let mut i = 0;
                while i < 8 {
                    if pieces.get(i) == Some(&0) {
                        let start = i;
                        while pieces.get(i) == Some(&0) {
                            i += 1;
                        }
                        if i - start > best_len {
                            best_start = Some(start);
                            best_len = i - start;
                        }
                    } else {
                        i += 1;
                    }
                }
                let mut out = String::from("[");
                let mut ignore0 = false;
                for (idx, &piece) in pieces.iter().enumerate() {
                    if ignore0 && piece == 0 {
                        continue;
                    }
                    ignore0 = false;
                    if best_start == Some(idx) {
                        out.push_str(if idx == 0 { "::" } else { ":" });
                        ignore0 = true;
                        continue;
                    }
                    let _ = write!(out, "{piece:x}");
                    if idx != 7 {
                        out.push(':');
                    }
                }
                out.push(']');
                out
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Paths

fn is_ascii_alpha(c: char) -> bool {
    c.is_ascii_alphabetic()
}

/// Windows drive letter (4.2): two code points, an ASCII alpha and ':' or '|'.
fn is_windows_drive_letter(s: &str) -> bool {
    let mut it = s.chars();
    matches!((it.next(), it.next(), it.next()), (Some(a), Some(':' | '|'), None) if is_ascii_alpha(a))
}

fn is_normalized_windows_drive_letter(s: &str) -> bool {
    let mut it = s.chars();
    matches!((it.next(), it.next(), it.next()), (Some(a), Some(':'), None) if is_ascii_alpha(a))
}

/// "starts with a Windows drive letter" (4.2).
fn starts_with_windows_drive_letter(s: &[char]) -> bool {
    match (s.first(), s.get(1)) {
        (Some(&a), Some(&(':' | '|'))) if is_ascii_alpha(a) => {
            s.len() == 2 || matches!(s.get(2), Some('/' | '\\' | '?' | '#'))
        }
        _ => false,
    }
}

fn is_single_dot(s: &str) -> bool {
    s == "." || s.eq_ignore_ascii_case("%2e")
}

fn is_double_dot(s: &str) -> bool {
    s == ".." || s.eq_ignore_ascii_case(".%2e") || s.eq_ignore_ascii_case("%2e.") || s.eq_ignore_ascii_case("%2e%2e")
}

impl Url {
    fn empty() -> Url {
        Url {
            scheme: String::new(),
            username: String::new(),
            password: String::new(),
            host: None,
            port: None,
            path: UrlPath::Segments(Vec::new()),
            query: None,
            fragment: None,
        }
    }

    pub fn is_special(&self) -> bool {
        is_special_scheme(&self.scheme)
    }

    fn has_opaque_path(&self) -> bool {
        matches!(self.path, UrlPath::Opaque(_))
    }

    fn includes_credentials(&self) -> bool {
        !self.username.is_empty() || !self.password.is_empty()
    }

    fn segments_mut(&mut self) -> Option<&mut Vec<String>> {
        match &mut self.path {
            UrlPath::Segments(v) => Some(v),
            UrlPath::Opaque(_) => None,
        }
    }

    /// "Shorten a URL's path" (4.2).
    fn shorten_path(&mut self) {
        let is_file = self.scheme == "file";
        if let Some(path) = self.segments_mut() {
            if is_file && path.len() == 1 && path.first().is_some_and(|s| is_normalized_windows_drive_letter(s)) {
                return;
            }
            path.pop();
        }
    }

    /// Parse `input` against an optional base URL: the URL parser (4.4) with UTF-8 encoding.
    pub fn parse(input: &str, base: Option<&Url>) -> Result<Url, UrlError> {
        basic_parse(input, base)
    }

    /// Resolve a (possibly relative) reference against this URL.
    pub fn join(&self, reference: &str) -> Result<Url, UrlError> {
        basic_parse(reference, Some(self))
    }

    // --- URL serializing (4.5) and API getters (6.1) ---

    pub fn href(&self) -> String {
        self.serialize(false)
    }

    /// URL serializer (4.5).
    pub fn serialize(&self, exclude_fragment: bool) -> String {
        let mut out = String::with_capacity(64);
        out.push_str(&self.scheme);
        out.push(':');
        if let Some(host) = &self.host {
            out.push_str("//");
            if self.includes_credentials() {
                out.push_str(&self.username);
                if !self.password.is_empty() {
                    out.push(':');
                    out.push_str(&self.password);
                }
                out.push('@');
            }
            out.push_str(&host.serialize());
            if let Some(port) = self.port {
                let _ = write!(out, ":{port}");
            }
        }
        if self.host.is_none()
            && let UrlPath::Segments(segs) = &self.path
            && segs.len() > 1
            && segs.first().is_some_and(String::is_empty)
        {
            out.push_str("/.");
        }
        out.push_str(&self.pathname());
        if let Some(q) = &self.query {
            out.push('?');
            out.push_str(q);
        }
        if !exclude_fragment && let Some(f) = &self.fragment {
            out.push('#');
            out.push_str(f);
        }
        out
    }

    pub fn protocol(&self) -> String {
        format!("{}:", self.scheme)
    }

    pub fn hostname(&self) -> String {
        self.host.as_ref().map(Host::serialize).unwrap_or_default()
    }

    /// The URL API's `host`: hostname plus ":port" when there is a port.
    pub fn host_with_port(&self) -> String {
        match (&self.host, self.port) {
            (None, _) => String::new(),
            (Some(h), None) => h.serialize(),
            (Some(h), Some(p)) => format!("{}:{p}", h.serialize()),
        }
    }

    pub fn port_string(&self) -> String {
        self.port.map(|p| p.to_string()).unwrap_or_default()
    }

    /// URL path serializer (4.5).
    pub fn pathname(&self) -> String {
        match &self.path {
            UrlPath::Opaque(s) => s.clone(),
            UrlPath::Segments(segs) => {
                let mut out = String::new();
                for s in segs {
                    out.push('/');
                    out.push_str(s);
                }
                out
            }
        }
    }

    pub fn search(&self) -> String {
        match &self.query {
            Some(q) if !q.is_empty() => format!("?{q}"),
            _ => String::new(),
        }
    }

    pub fn hash(&self) -> String {
        match &self.fragment {
            Some(f) if !f.is_empty() => format!("#{f}"),
            _ => String::new(),
        }
    }

    /// Serialization of the URL's origin (4.7). Opaque origins serialize as "null".
    pub fn origin(&self) -> String {
        match self.scheme.as_str() {
            "blob" => {
                if let UrlPath::Opaque(p) = &self.path
                    && let Ok(inner) = Url::parse(p, None)
                    && matches!(inner.scheme.as_str(), "http" | "https")
                {
                    return inner.origin();
                }
                "null".to_string()
            }
            "ftp" | "http" | "https" | "ws" | "wss" => {
                format!("{}://{}", self.scheme, self.host_with_port())
            }
            _ => "null".to_string(),
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The basic URL parser (4.4)

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum State {
    SchemeStart,
    Scheme,
    NoScheme,
    SpecialRelativeOrAuthority,
    PathOrAuthority,
    Relative,
    RelativeSlash,
    SpecialAuthoritySlashes,
    SpecialAuthorityIgnoreSlashes,
    Authority,
    Host,
    Port,
    File,
    FileSlash,
    FileHost,
    PathStart,
    Path,
    OpaquePath,
    Query,
    Fragment,
}

fn basic_parse(input: &str, base: Option<&Url>) -> Result<Url, UrlError> {
    let mut url = Url::empty();
    // Strip leading/trailing C0 control or space, then remove ASCII tab and newline.
    let trimmed = input.trim_matches(|c: char| (c as u32) <= 0x20);
    let chars: Vec<char> = trimmed.chars().filter(|&c| !matches!(c, '\t' | '\n' | '\r')).collect();
    let len = chars.len() as isize;

    let mut state = State::SchemeStart;
    let mut buffer = String::new();
    let mut at_sign_seen = false;
    let mut inside_brackets = false;
    let mut password_token_seen = false;
    let mut pointer: isize = 0;

    loop {
        let c: Option<char> = if pointer >= 0 { chars.get(pointer as usize).copied() } else { None };
        let remaining_start = (pointer + 1).max(0) as usize;
        let remaining = chars.get(remaining_start..).unwrap_or(&[]);
        let remaining_starts_with_slash = remaining.first() == Some(&'/');

        match state {
            State::SchemeStart => {
                if let Some(ch) = c.filter(|&ch| is_ascii_alpha(ch)) {
                    buffer.push(ch.to_ascii_lowercase());
                    state = State::Scheme;
                } else {
                    state = State::NoScheme;
                    pointer -= 1;
                }
            }
            State::Scheme => {
                if let Some(ch) = c.filter(|&ch| ch.is_ascii_alphanumeric() || matches!(ch, '+' | '-' | '.')) {
                    buffer.push(ch.to_ascii_lowercase());
                } else if c == Some(':') {
                    url.scheme = std::mem::take(&mut buffer);
                    if url.scheme == "file" {
                        state = State::File;
                    } else if url.is_special() && base.is_some_and(|b| b.scheme == url.scheme) {
                        state = State::SpecialRelativeOrAuthority;
                    } else if url.is_special() {
                        state = State::SpecialAuthoritySlashes;
                    } else if remaining_starts_with_slash {
                        state = State::PathOrAuthority;
                        pointer += 1;
                    } else {
                        url.path = UrlPath::Opaque(String::new());
                        state = State::OpaquePath;
                    }
                } else {
                    buffer.clear();
                    state = State::NoScheme;
                    pointer = -1; // start over from the first code point
                }
            }
            State::NoScheme => {
                let base = base.ok_or(UrlError)?;
                if base.has_opaque_path() {
                    if c != Some('#') {
                        return Err(UrlError);
                    }
                    url.scheme = base.scheme.clone();
                    url.path = base.path.clone();
                    url.query = base.query.clone();
                    url.fragment = Some(String::new());
                    state = State::Fragment;
                } else if base.scheme != "file" {
                    state = State::Relative;
                    pointer -= 1;
                } else {
                    state = State::File;
                    pointer -= 1;
                }
            }
            State::SpecialRelativeOrAuthority => {
                if c == Some('/') && remaining_starts_with_slash {
                    state = State::SpecialAuthorityIgnoreSlashes;
                    pointer += 1;
                } else {
                    state = State::Relative;
                    pointer -= 1;
                }
            }
            State::PathOrAuthority => {
                if c == Some('/') {
                    state = State::Authority;
                } else {
                    state = State::Path;
                    pointer -= 1;
                }
            }
            State::Relative => {
                let base = base.ok_or(UrlError)?;
                url.scheme = base.scheme.clone();
                if c == Some('/') || (url.is_special() && c == Some('\\')) {
                    state = State::RelativeSlash;
                } else {
                    url.username = base.username.clone();
                    url.password = base.password.clone();
                    url.host = base.host.clone();
                    url.port = base.port;
                    url.path = base.path.clone();
                    url.query = base.query.clone();
                    if c == Some('?') {
                        url.query = Some(String::new());
                        state = State::Query;
                    } else if c == Some('#') {
                        url.fragment = Some(String::new());
                        state = State::Fragment;
                    } else if c.is_some() {
                        url.query = None;
                        url.shorten_path();
                        state = State::Path;
                        pointer -= 1;
                    }
                }
            }
            State::RelativeSlash => {
                if url.is_special() && matches!(c, Some('/' | '\\')) {
                    state = State::SpecialAuthorityIgnoreSlashes;
                } else if c == Some('/') {
                    state = State::Authority;
                } else {
                    let base = base.ok_or(UrlError)?;
                    url.username = base.username.clone();
                    url.password = base.password.clone();
                    url.host = base.host.clone();
                    url.port = base.port;
                    state = State::Path;
                    pointer -= 1;
                }
            }
            State::SpecialAuthoritySlashes => {
                if c == Some('/') && remaining_starts_with_slash {
                    state = State::SpecialAuthorityIgnoreSlashes;
                    pointer += 1;
                } else {
                    state = State::SpecialAuthorityIgnoreSlashes;
                    pointer -= 1;
                }
            }
            State::SpecialAuthorityIgnoreSlashes => {
                if !matches!(c, Some('/' | '\\')) {
                    state = State::Authority;
                    pointer -= 1;
                }
            }
            State::Authority => {
                if c == Some('@') {
                    if at_sign_seen {
                        buffer.insert_str(0, "%40");
                    }
                    at_sign_seen = true;
                    for cp in buffer.chars() {
                        if cp == ':' && !password_token_seen {
                            password_token_seen = true;
                            continue;
                        }
                        if password_token_seen {
                            percent_encode_char(cp, EncodeSet::Userinfo, &mut url.password);
                        } else {
                            percent_encode_char(cp, EncodeSet::Userinfo, &mut url.username);
                        }
                    }
                    buffer.clear();
                } else if matches!(c, None | Some('/' | '?' | '#')) || (url.is_special() && c == Some('\\')) {
                    if at_sign_seen && buffer.is_empty() {
                        return Err(UrlError);
                    }
                    pointer -= buffer.chars().count() as isize + 1;
                    buffer.clear();
                    state = State::Host;
                } else if let Some(ch) = c {
                    buffer.push(ch);
                }
            }
            State::Host => {
                if c == Some(':') && !inside_brackets {
                    if buffer.is_empty() {
                        return Err(UrlError);
                    }
                    let host = parse_host(&buffer, !url.is_special())?;
                    url.host = Some(host);
                    buffer.clear();
                    state = State::Port;
                } else if matches!(c, None | Some('/' | '?' | '#')) || (url.is_special() && c == Some('\\')) {
                    pointer -= 1;
                    if url.is_special() && buffer.is_empty() {
                        return Err(UrlError);
                    }
                    let host = if buffer.is_empty() { Host::Empty } else { parse_host(&buffer, !url.is_special())? };
                    url.host = Some(host);
                    buffer.clear();
                    state = State::PathStart;
                } else if let Some(ch) = c {
                    if ch == '[' {
                        inside_brackets = true;
                    }
                    if ch == ']' {
                        inside_brackets = false;
                    }
                    buffer.push(ch);
                }
            }
            State::Port => {
                if let Some(ch) = c.filter(char::is_ascii_digit) {
                    buffer.push(ch);
                } else if matches!(c, None | Some('/' | '?' | '#')) || (url.is_special() && c == Some('\\')) {
                    if !buffer.is_empty() {
                        let port: u32 = buffer.parse().map_err(|_| UrlError).and_then(|p: u32| {
                            if p > 65535 { Err(UrlError) } else { Ok(p) }
                        })?;
                        let port = port as u16;
                        url.port = if default_port(&url.scheme) == Some(port) { None } else { Some(port) };
                        buffer.clear();
                    }
                    state = State::PathStart;
                    pointer -= 1;
                } else {
                    return Err(UrlError);
                }
            }
            State::File => {
                url.scheme = "file".to_string();
                url.host = Some(Host::Empty);
                if matches!(c, Some('/' | '\\')) {
                    state = State::FileSlash;
                } else if let Some(base) = base.filter(|b| b.scheme == "file") {
                    url.host = base.host.clone();
                    url.path = base.path.clone();
                    url.query = base.query.clone();
                    if c == Some('?') {
                        url.query = Some(String::new());
                        state = State::Query;
                    } else if c == Some('#') {
                        url.fragment = Some(String::new());
                        state = State::Fragment;
                    } else if c.is_some() {
                        url.query = None;
                        let rest = chars.get(pointer.max(0) as usize..).unwrap_or(&[]);
                        if !starts_with_windows_drive_letter(rest) {
                            url.shorten_path();
                        } else {
                            url.path = UrlPath::Segments(Vec::new());
                        }
                        state = State::Path;
                        pointer -= 1;
                    }
                } else {
                    state = State::Path;
                    pointer -= 1;
                }
            }
            State::FileSlash => {
                if matches!(c, Some('/' | '\\')) {
                    state = State::FileHost;
                } else {
                    if let Some(base) = base.filter(|b| b.scheme == "file") {
                        url.host = base.host.clone();
                        let rest = chars.get(pointer.max(0) as usize..).unwrap_or(&[]);
                        if !starts_with_windows_drive_letter(rest)
                            && let UrlPath::Segments(segs) = &base.path
                            && let Some(first) = segs.first()
                            && is_normalized_windows_drive_letter(first)
                        {
                            let first = first.clone();
                            if let Some(p) = url.segments_mut() {
                                p.push(first);
                            }
                        }
                    }
                    state = State::Path;
                    pointer -= 1;
                }
            }
            State::FileHost => {
                if matches!(c, None | Some('/' | '\\' | '?' | '#')) {
                    pointer -= 1;
                    if is_windows_drive_letter(&buffer) {
                        // Windows drive letter quirk: buffer is reused by the path state.
                        state = State::Path;
                    } else if buffer.is_empty() {
                        url.host = Some(Host::Empty);
                        state = State::PathStart;
                    } else {
                        let mut host = parse_host(&buffer, !url.is_special())?;
                        if host == Host::Domain("localhost".to_string()) {
                            host = Host::Empty;
                        }
                        url.host = Some(host);
                        buffer.clear();
                        state = State::PathStart;
                    }
                } else if let Some(ch) = c {
                    buffer.push(ch);
                }
            }
            State::PathStart => {
                if url.is_special() {
                    state = State::Path;
                    if !matches!(c, Some('/' | '\\')) {
                        pointer -= 1;
                    }
                } else if c == Some('?') {
                    url.query = Some(String::new());
                    state = State::Query;
                } else if c == Some('#') {
                    url.fragment = Some(String::new());
                    state = State::Fragment;
                } else if c.is_some() {
                    state = State::Path;
                    if c != Some('/') {
                        pointer -= 1;
                    }
                }
            }
            State::Path => {
                let end_of_segment = matches!(c, None | Some('/' | '?' | '#')) || (url.is_special() && c == Some('\\'));
                if end_of_segment {
                    let slash = c == Some('/') || (url.is_special() && c == Some('\\'));
                    if is_double_dot(&buffer) {
                        url.shorten_path();
                        if !slash && let Some(p) = url.segments_mut() {
                            p.push(String::new());
                        }
                    } else if is_single_dot(&buffer) {
                        if !slash && let Some(p) = url.segments_mut() {
                            p.push(String::new());
                        }
                    } else {
                        let is_file = url.scheme == "file";
                        if let Some(p) = url.segments_mut() {
                            if is_file && p.is_empty() && is_windows_drive_letter(&buffer) {
                                // Replace the second code point with ':'.
                                let mut it = buffer.chars();
                                if let Some(first) = it.next() {
                                    buffer = format!("{first}:");
                                }
                            }
                            p.push(std::mem::take(&mut buffer));
                        }
                    }
                    buffer.clear();
                    if c == Some('?') {
                        url.query = Some(String::new());
                        state = State::Query;
                    } else if c == Some('#') {
                        url.fragment = Some(String::new());
                        state = State::Fragment;
                    }
                } else if let Some(ch) = c {
                    percent_encode_char(ch, EncodeSet::Path, &mut buffer);
                }
            }
            State::OpaquePath => {
                if c == Some('?') {
                    url.query = Some(String::new());
                    state = State::Query;
                } else if c == Some('#') {
                    url.fragment = Some(String::new());
                    state = State::Fragment;
                } else if let Some(ch) = c
                    && let UrlPath::Opaque(p) = &mut url.path
                {
                    if ch == ' ' && matches!(remaining.first(), Some('?' | '#')) {
                        p.push_str("%20");
                    } else {
                        percent_encode_char(ch, EncodeSet::C0Control, p);
                    }
                }
            }
            State::Query => {
                if c.is_none() || c == Some('#') {
                    let set = if url.is_special() { EncodeSet::SpecialQuery } else { EncodeSet::Query };
                    let q = url.query.get_or_insert_with(String::new);
                    for ch in buffer.chars() {
                        percent_encode_char(ch, set, q);
                    }
                    buffer.clear();
                    if c == Some('#') {
                        url.fragment = Some(String::new());
                        state = State::Fragment;
                    }
                } else if let Some(ch) = c {
                    buffer.push(ch);
                }
            }
            State::Fragment => {
                if let Some(ch) = c {
                    let f = url.fragment.get_or_insert_with(String::new);
                    percent_encode_char(ch, EncodeSet::Fragment, f);
                }
            }
        }

        if pointer >= len {
            break;
        }
        pointer += 1;
    }
    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> Url {
        Url::parse(s, None).unwrap()
    }

    #[test]
    fn simple() {
        assert_eq!(p("HTTP://Example.COM:80/a/./b/../c?x=1#f").href(), "http://example.com/a/c?x=1#f");
    }

    #[test]
    fn relative() {
        let base = p("http://a/b/c/d;p?q");
        assert_eq!(base.join("../g").unwrap().href(), "http://a/b/g");
        assert_eq!(base.join("//x/y").unwrap().href(), "http://x/y");
        assert_eq!(base.join("?y").unwrap().href(), "http://a/b/c/d;p?y");
        assert_eq!(base.join("#s").unwrap().href(), "http://a/b/c/d;p?q#s");
    }

    #[test]
    fn idna_chinese() {
        assert_eq!(p("http://中文.com/").href(), "http://xn--fiq228c.com/");
        assert_eq!(p("http://例え.jp/").hostname(), "xn--r8jz45g.jp");
    }

    #[test]
    fn ip_hosts() {
        assert_eq!(p("http://0x7f.1/").hostname(), "127.0.0.1");
        assert_eq!(p("http://[::1]:8080/").host_with_port(), "[::1]:8080");
        assert_eq!(p("http://[2001:db8:0:0:1:0:0:1]/").hostname(), "[2001:db8::1:0:0:1]");
    }

    #[test]
    fn failures() {
        assert!(Url::parse("http://", None).is_err());
        assert!(Url::parse("/rel", None).is_err());
        assert!(Url::parse("http://a:99999/", None).is_err());
    }

    #[test]
    fn opaque_and_file() {
        assert_eq!(p("mailto:a@b.c?s=1").pathname(), "a@b.c");
        assert_eq!(p("file:///C|/x").href(), "file:///C:/x");
        assert_eq!(p("data:text/plain,hi there").href(), "data:text/plain,hi there");
    }
}
