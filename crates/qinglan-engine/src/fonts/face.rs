//! `@font-face` (css-fonts-4 section 4): reading the rule, and the font matching algorithm (section 5.2) that orders
//! the faces of one family for a request. Pure data: no loading, no platform.
//!
//! The rule is read from the text of its block, not from component values: `unicode-range` (`U+0025-00FF`) does not
//! survive the CSS tokenizer (it reads `U+1E00` as a number).

use std::time::Duration;

use crate::platform::FontSlant;
use crate::url::Url;

/// More faces than this in one page are ignored.
pub const MAX_FACES: usize = 256;
/// Most `unicode-range` ranges kept per face.
const MAX_RANGES: usize = 512;
/// Most `src` entries kept per face.
const MAX_SRC: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FaceStyle {
    Normal,
    Italic,
    /// `oblique` with its angle range in degrees.
    Oblique(f32, f32),
}

/// `font-display` (css-fonts-4 4.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Display {
    Auto,
    Block,
    Swap,
    Fallback,
    Optional,
}

impl Display {
    /// How long text in the face is invisible while it loads. `auto` is `block` (as in the browsers).
    pub fn block_period(self) -> Duration {
        match self {
            Display::Auto | Display::Block => Duration::from_secs(3),
            Display::Swap => Duration::ZERO,
            Display::Fallback | Display::Optional => Duration::from_millis(100),
        }
    }

    /// How long after the block period the face is still used if it arrives (`None`: for ever).
    pub fn swap_period(self) -> Option<Duration> {
        match self {
            Display::Auto | Display::Block | Display::Swap => None,
            Display::Fallback => Some(Duration::from_secs(3)),
            Display::Optional => Some(Duration::ZERO),
        }
    }
}

/// What the matching and the platform need to know of a face.
#[derive(Clone, Debug, PartialEq)]
pub struct FaceDesc {
    /// The family name, lower-cased (family names compare case-insensitively).
    pub family: String,
    pub weight: (f32, f32),
    pub stretch: (f32, f32),
    pub style: FaceStyle,
    /// `unicode-range`; empty means all of Unicode.
    pub ranges: Vec<(u32, u32)>,
}

impl FaceDesc {
    pub fn covers(&self, cp: u32) -> bool {
        self.ranges.is_empty() || self.ranges.iter().any(|&(a, b)| (a..=b).contains(&cp))
    }
}

#[derive(Clone, Debug)]
pub enum Src {
    Url(Url),
    Local(String),
}

#[derive(Clone, Debug)]
pub struct FontFaceRule {
    pub desc: FaceDesc,
    pub src: Vec<Src>,
    pub display: Display,
}

/// Remove `/* */` comments (outside strings).
fn strip_comments(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars().peekable();
    let mut quote: Option<char> = None;
    while let Some(c) = it.next() {
        match quote {
            Some(q) => {
                out.push(c);
                if c == '\\' {
                    if let Some(n) = it.next() {
                        out.push(n);
                    }
                } else if c == q {
                    quote = None;
                }
            }
            None => {
                if c == '/' && it.peek() == Some(&'*') {
                    it.next();
                    let mut prev = ' ';
                    for d in it.by_ref() {
                        if prev == '*' && d == '/' {
                            break;
                        }
                        prev = d;
                    }
                    out.push(' ');
                } else {
                    if c == '"' || c == '\'' {
                        quote = Some(c);
                    }
                    out.push(c);
                }
            }
        }
    }
    out
}

/// Split at `sep` outside strings and brackets.
fn split_top(s: &str, sep: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let (mut depth, mut start, mut quote) = (0i32, 0usize, None::<char>);
    let mut skip = false;
    for (i, c) in s.char_indices() {
        if skip {
            skip = false;
            continue;
        }
        match quote {
            Some(q) => {
                if c == '\\' {
                    skip = true;
                } else if c == q {
                    quote = None;
                }
            }
            None => match c {
                '"' | '\'' => quote = Some(c),
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' => depth -= 1,
                c if c == sep && depth <= 0 => {
                    parts.push(s.get(start..i).unwrap_or(""));
                    start = i + c.len_utf8();
                }
                _ => {}
            },
        }
    }
    parts.push(s.get(start..).unwrap_or(""));
    parts
}

/// A string literal or a bare name at the start of `s`; returns it and the rest.
fn name_or_string(s: &str) -> Option<(String, &str)> {
    let s = s.trim_start();
    let first = s.chars().next()?;
    if first == '"' || first == '\'' {
        let mut out = String::new();
        let mut it = s.char_indices().skip(1);
        while let Some((i, c)) = it.next() {
            if c == first {
                return Some((out, s.get(i + 1..)?));
            }
            if c == '\\' {
                if let Some((_, n)) = it.next() {
                    out.push(n);
                }
            } else {
                out.push(c);
            }
        }
        return None;
    }
    let end = s.find(|c: char| c.is_whitespace() || c == ',' || c == ')').unwrap_or(s.len());
    Some((s.get(..end)?.to_string(), s.get(end..)?))
}

/// The arguments of the function call `name(...)` at the start of `s` (any case) and what follows it.
fn call<'a>(s: &'a str, name: &str) -> Option<(&'a str, &'a str)> {
    let s = s.trim_start();
    let head = s.get(..name.len() + 1)?;
    if !head.get(..name.len())?.eq_ignore_ascii_case(name) || !head.ends_with('(') {
        return None;
    }
    let body = s.get(name.len() + 1..)?;
    let (mut depth, mut quote, mut skip) = (1i32, None::<char>, false);
    for (i, c) in body.char_indices() {
        if skip {
            skip = false;
            continue;
        }
        match quote {
            Some(q) => {
                if c == '\\' {
                    skip = true;
                } else if c == q {
                    quote = None;
                }
            }
            None => match c {
                '"' | '\'' => quote = Some(c),
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some((body.get(..i)?, body.get(i + 1..)?));
                    }
                }
                _ => {}
            },
        }
    }
    None
}

fn supported_format(f: &str) -> bool {
    matches!(f.to_ascii_lowercase().as_str(), "woff2" | "woff" | "truetype" | "opentype" | "woff2-variations" | "woff-variations" | "truetype-variations" | "opentype-variations")
}

fn supported_tech(t: &str) -> bool {
    matches!(t.to_ascii_lowercase().as_str(), "variations" | "features-opentype" | "color-colrv0" | "color-colrv1" | "palettes")
}

fn parse_src(value: &str, base: Option<&Url>) -> Vec<Src> {
    let mut out = Vec::new();
    for item in split_top(value, ',') {
        if out.len() >= MAX_SRC {
            break;
        }
        let item = item.trim();
        if let Some((inner, _)) = call(item, "local") {
            if let Some((name, _)) = name_or_string(inner) {
                out.push(Src::Local(name));
            }
            continue;
        }
        let Some((inner, mut rest)) = call(item, "url") else { continue };
        let Some((raw, _)) = name_or_string(inner.trim()) else { continue };
        let url = match base {
            Some(b) => b.join(&raw).ok(),
            None => Url::parse(&raw, None).ok(),
        };
        let mut usable = true;
        loop {
            if let Some((args, r)) = call(rest, "format") {
                usable &= split_top(args, ',').iter().any(|f| name_or_string(f).is_some_and(|(n, _)| supported_format(&n)));
                rest = r;
            } else if let Some((args, r)) = call(rest, "tech") {
                usable &= split_top(args, ',').iter().all(|f| name_or_string(f).is_some_and(|(n, _)| supported_tech(&n)));
                rest = r;
            } else {
                break;
            }
        }
        if usable && rest.trim().is_empty() && let Some(url) = url {
            out.push(Src::Url(url));
        }
    }
    out
}

fn parse_unicode_range(value: &str) -> Option<Vec<(u32, u32)>> {
    let mut out = Vec::new();
    for item in split_top(value, ',') {
        let item = item.trim();
        let body = item.get(2..).filter(|_| item.get(..2).is_some_and(|h| h.eq_ignore_ascii_case("u+")))?;
        let (lo, hi) = if let Some((a, b)) = body.split_once('-') {
            (u32::from_str_radix(a, 16).ok().filter(|_| (1..=6).contains(&a.len()))?, u32::from_str_radix(b, 16).ok().filter(|_| (1..=6).contains(&b.len()))?)
        } else if body.contains('?') {
            if body.len() > 6 || body.trim_end_matches('?').chars().any(|c| !c.is_ascii_hexdigit()) {
                return None;
            }
            let a = body.replace('?', "0");
            let b = body.replace('?', "F");
            (u32::from_str_radix(&a, 16).ok()?, u32::from_str_radix(&b, 16).ok()?)
        } else {
            let v = u32::from_str_radix(body, 16).ok().filter(|_| (1..=6).contains(&body.len()))?;
            (v, v)
        };
        if lo > hi || lo > 0x10FFFF {
            return None;
        }
        if out.len() < MAX_RANGES {
            out.push((lo, hi.min(0x10FFFF)));
        }
    }
    (!out.is_empty()).then_some(out)
}

fn numbers(value: &str, keyword: impl Fn(&str) -> Option<f32>, lo: f32, hi: f32, percent: bool) -> Option<(f32, f32)> {
    let mut v = Vec::new();
    for w in value.split_whitespace() {
        let n = match keyword(&w.to_ascii_lowercase()) {
            Some(n) => n,
            None => {
                let t = if percent { w.strip_suffix('%')? } else { w };
                t.parse::<f32>().ok()?
            }
        };
        if !n.is_finite() || n < lo || n > hi {
            return None;
        }
        v.push(n);
    }
    match v.as_slice() {
        [a] => Some((*a, *a)),
        [a, b] => Some((a.min(*b), a.max(*b))),
        _ => None,
    }
}

fn parse_style(value: &str) -> Option<FaceStyle> {
    let mut w = value.split_whitespace();
    match w.next()?.to_ascii_lowercase().as_str() {
        "normal" => w.next().is_none().then_some(FaceStyle::Normal),
        "italic" => w.next().is_none().then_some(FaceStyle::Italic),
        "oblique" => {
            let mut a = Vec::new();
            for x in w {
                let t = x.to_ascii_lowercase();
                let deg: f32 = if let Some(d) = t.strip_suffix("deg") { d.parse().ok()? } else if t == "0" { 0.0 } else { return None };
                if !(-90.0..=90.0).contains(&deg) {
                    return None;
                }
                a.push(deg);
            }
            match a.as_slice() {
                [] => Some(FaceStyle::Oblique(14.0, 14.0)),
                [x] => Some(FaceStyle::Oblique(*x, *x)),
                [x, y] => Some(FaceStyle::Oblique(x.min(*y), x.max(*y))),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Read the block of an `@font-face` rule. `None` when the rule is invalid (no family or no usable source).
/// `base` is the address of the style sheet, for relative `url()`s.
pub fn parse_font_face(block: &str, base: Option<&Url>) -> Option<FontFaceRule> {
    let text = strip_comments(block);
    let mut family: Option<String> = None;
    let mut src: Vec<Src> = Vec::new();
    let mut desc = FaceDesc { family: String::new(), weight: (400.0, 400.0), stretch: (100.0, 100.0), style: FaceStyle::Normal, ranges: Vec::new() };
    let mut display = Display::Auto;
    for d in split_top(&text, ';') {
        let Some((name, value)) = d.split_once(':') else { continue };
        let (name, value) = (name.trim().to_ascii_lowercase(), value.trim().trim_end_matches("!important").trim());
        match name.as_str() {
            "font-family" => {
                let parts: Vec<&str> = value.split_whitespace().collect();
                let n = match name_or_string(value) {
                    Some((s, rest)) if value.starts_with(['"', '\'']) && rest.trim().is_empty() => s,
                    _ if !parts.is_empty() && !value.contains([',', '"', '\'']) => parts.join(" "),
                    _ => continue,
                };
                if matches!(n.to_ascii_lowercase().as_str(), "serif" | "sans-serif" | "monospace" | "cursive" | "fantasy" | "system-ui" | "inherit" | "initial" | "unset") && !value.starts_with(['"', '\'']) {
                    continue;
                }
                family = Some(n);
            }
            "src" => src = parse_src(value, base),
            "unicode-range" => {
                if let Some(r) = parse_unicode_range(value) {
                    desc.ranges = r;
                }
            }
            "font-weight" => {
                let kw = |w: &str| match w {
                    "normal" => Some(400.0),
                    "bold" => Some(700.0),
                    _ => None,
                };
                if let Some(w) = numbers(value, kw, 1.0, 1000.0, false) {
                    desc.weight = w;
                }
            }
            "font-stretch" | "font-width" => {
                let kw = |w: &str| {
                    Some(match w {
                        "ultra-condensed" => 50.0,
                        "extra-condensed" => 62.5,
                        "condensed" => 75.0,
                        "semi-condensed" => 87.5,
                        "normal" => 100.0,
                        "semi-expanded" => 112.5,
                        "expanded" => 125.0,
                        "extra-expanded" => 150.0,
                        "ultra-expanded" => 200.0,
                        _ => return None,
                    })
                };
                if let Some(w) = numbers(value, kw, 0.0, 1000.0, true) {
                    desc.stretch = w;
                }
            }
            "font-style" => {
                if let Some(s) = parse_style(value) {
                    desc.style = s;
                }
            }
            "font-display" => {
                display = match value.to_ascii_lowercase().as_str() {
                    "auto" => Display::Auto,
                    "block" => Display::Block,
                    "swap" => Display::Swap,
                    "fallback" => Display::Fallback,
                    "optional" => Display::Optional,
                    _ => display,
                };
            }
            _ => {}
        }
    }
    let family = family?;
    if src.is_empty() {
        return None;
    }
    desc.family = family.to_lowercase();
    Some(FontFaceRule { desc, src, display })
}

/// The indices of `faces` (all of one family) in the order the matching algorithm of css-fonts-4 5.2 would try them
/// for a request of this weight and slant (stretch is always `normal`): the best first. Ties go to the face declared
/// last. Which of them draws a character is then a matter of `unicode-range`: the first that covers it.
pub fn rank_faces(faces: &[&FaceDesc], weight: f32, slant: FontSlant) -> Vec<usize> {
    let w = if weight.is_finite() { weight } else { 400.0 };
    let key = |f: &FaceDesc| -> (u8, f32, u8, u8, f32) {
        // font-stretch: the desired width is 100%; closer first, narrower before wider.
        let (slo, shi) = f.stretch;
        let stretch = if slo <= 100.0 && 100.0 <= shi {
            (0u8, 0.0)
        } else if shi < 100.0 {
            (1, 100.0 - shi)
        } else {
            (2, slo - 100.0)
        };
        // font-style.
        let style_rank = match (slant, f.style) {
            (FontSlant::Upright, FaceStyle::Normal) | (FontSlant::Italic, FaceStyle::Italic) | (FontSlant::Oblique, FaceStyle::Oblique(..)) => 0,
            (FontSlant::Upright, FaceStyle::Oblique(..)) | (FontSlant::Italic, FaceStyle::Oblique(..)) | (FontSlant::Oblique, FaceStyle::Italic) => 1,
            (FontSlant::Upright, FaceStyle::Italic) | (FontSlant::Italic, FaceStyle::Normal) | (FontSlant::Oblique, FaceStyle::Normal) => 2,
        };
        // font-weight.
        let (lo, hi) = f.weight;
        let weight_key = if lo <= w && w <= hi {
            (0u8, 0.0)
        } else if w < 400.0 {
            if hi < w { (1, w - hi) } else { (2, lo - w) }
        } else if w > 500.0 {
            if lo > w { (1, lo - w) } else { (2, w - hi) }
        } else if lo > w && lo <= 500.0 {
            (1, lo - w)
        } else if hi < w {
            (2, w - hi)
        } else {
            (3, lo - w)
        };
        (stretch.0, stretch.1, style_rank, weight_key.0, weight_key.1)
    };
    let mut order: Vec<usize> = (0..faces.len()).collect();
    let keys: Vec<(u8, f32, u8, u8, f32)> = faces.iter().map(|f| key(f)).collect();
    order.sort_by(|&a, &b| {
        let (ka, kb) = (keys.get(a).copied().unwrap_or((9, 0.0, 9, 9, 0.0)), keys.get(b).copied().unwrap_or((9, 0.0, 9, 9, 0.0)));
        ka.0.cmp(&kb.0)
            .then(ka.1.total_cmp(&kb.1))
            .then(ka.2.cmp(&kb.2))
            .then(ka.3.cmp(&kb.3))
            .then(ka.4.total_cmp(&kb.4))
            .then(b.cmp(&a))
    });
    order
}

/// Which 256-code-point blocks of Unicode the text of a page uses: a face whose `unicode-range` touches none of
/// them is not worth downloading.
#[derive(Clone)]
pub struct Blocks(Vec<u64>);

impl Default for Blocks {
    fn default() -> Self {
        Blocks(vec![0; 0x1100 / 64])
    }
}

impl Blocks {
    pub fn add(&mut self, c: char) {
        let b = (c as usize) >> 8;
        if let Some(w) = self.0.get_mut(b / 64) {
            *w |= 1 << (b % 64);
        }
    }

    pub fn touches(&self, ranges: &[(u32, u32)]) -> bool {
        if ranges.is_empty() {
            return self.0.iter().any(|&w| w != 0);
        }
        ranges.iter().any(|&(a, b)| ((a >> 8)..=(b >> 8)).any(|blk| self.0.get(blk as usize / 64).is_some_and(|w| w & (1 << (blk % 64)) != 0)))
    }
}

/// What a page asks of its fonts: the family list, weight and slant of some text.
pub struct Need<'a> {
    pub families: &'a [crate::css::types::FontFamily],
    pub weight: f32,
    pub slant: FontSlant,
}

/// The faces (indices into `rules`) worth downloading for these needs: for each request the faces its family list
/// would use, best first, leaving out faces that only repeat the range of one already taken.
pub fn faces_to_fetch(rules: &[FontFaceRule], needs: &[Need<'_>], blocks: &Blocks) -> Vec<usize> {
    let mut chosen = vec![false; rules.len()];
    for need in needs {
        for fam in need.families {
            let crate::css::types::FontFamily::Named(n) = fam else { continue };
            let key = n.to_lowercase();
            let idx: Vec<usize> = rules.iter().enumerate().filter(|(_, r)| r.desc.family == key).map(|(i, _)| i).collect();
            if idx.is_empty() {
                continue;
            }
            let descs: Vec<&FaceDesc> = idx.iter().filter_map(|&i| rules.get(i).map(|r| &r.desc)).collect();
            let mut taken: Vec<&[(u32, u32)]> = Vec::new();
            for r in rank_faces(&descs, need.weight, need.slant) {
                let (Some(&gi), Some(d)) = (idx.get(r), descs.get(r)) else { continue };
                if !blocks.touches(&d.ranges) {
                    continue;
                }
                if taken.iter().any(|t| t.is_empty() || *t == d.ranges.as_slice()) {
                    continue;
                }
                taken.push(&d.ranges);
                if let Some(c) = chosen.get_mut(gi) {
                    *c = true;
                }
            }
        }
    }
    chosen.iter().enumerate().filter(|(_, c)| **c).map(|(i, _)| i).collect()
}
