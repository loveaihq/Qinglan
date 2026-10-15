//! CSS colors: css-color-4. Parsing of the color syntaxes layer 1 supports and conversion to sRGB.
//!
//! Supported: 148 named colors, `#rgb[a]` / `#rrggbb[aa]`, `rgb()`/`rgba()`, `hsl()`/`hsla()`, `hwb()`
//! (comma and space syntaxes), `lab()`, `lch()`, `oklab()`, `oklch()`, `currentcolor`, `transparent` and
//! the system colors. `color-mix()`, `color()`, `light-dark()` and relative colors are invalid
//! (decisions.md). Out-of-gamut Lab/LCH/OKLab/OKLCH colors are clipped to sRGB instead of gamut-mapped.

use super::color_names::NAMED_COLORS;
use super::syntax::{Cv, split_commas, trim_ws};
use super::tokenizer::Token;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const BLACK: Color = Color { r: 0, g: 0, b: 0, a: 255 };
    pub const WHITE: Color = Color { r: 255, g: 255, b: 255, a: 255 };
    pub const TRANSPARENT: Color = Color { r: 0, g: 0, b: 0, a: 0 };

    pub const fn rgb(r: u8, g: u8, b: u8) -> Color {
        Color { r, g, b, a: 255 }
    }

    pub fn from_hex(h: u32) -> Color {
        Color::rgb((h >> 16) as u8, (h >> 8) as u8, h as u8)
    }

    /// Serialize like browsers do: `#rrggbb` or `rgba(r, g, b, a)`.
    pub fn to_css(self) -> String {
        if self.a == 255 {
            format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
        } else {
            format!("rgba({}, {}, {}, {:.3})", self.r, self.g, self.b, self.a as f32 / 255.0)
        }
    }
}

/// A color property value: a color or the `currentcolor` keyword (resolved when used).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ColorValue {
    Color(Color),
    CurrentColor,
}

impl ColorValue {
    pub fn to_css(self) -> String {
        match self {
            ColorValue::Color(c) => c.to_css(),
            ColorValue::CurrentColor => "currentcolor".to_string(),
        }
    }
    pub fn resolve(self, current: Color) -> Color {
        match self {
            ColorValue::Color(c) => c,
            ColorValue::CurrentColor => current,
        }
    }
}

/// System colors (css-color-4 6.2), light color scheme values.
fn system_color(name: &str) -> Option<Color> {
    let h = match name {
        "accentcolor" | "selecteditem" | "highlight" => 0x0078D7,
        "accentcolortext" | "selecteditemtext" | "highlighttext" => 0xFFFFFF,
        "activetext" => 0xFF0000,
        "buttonborder" => 0x767676,
        "buttonface" | "threedface" | "menu" => 0xEFEFEF,
        "buttontext" | "canvastext" | "captiontext" | "fieldtext" | "infotext" | "menutext" | "windowtext" | "marktext" => 0x000000,
        "canvas" | "field" | "window" | "threedhighlight" | "buttonhighlight" => 0xFFFFFF,
        "graytext" => 0x808080,
        "linktext" => 0x0000EE,
        "mark" => 0xFFFF00,
        "visitedtext" => 0x551A8B,
        "activeborder" => 0xB4B4B4,
        "activecaption" => 0x99B4D1,
        "appworkspace" => 0xABABAB,
        "background" => 0x5A8BB5,
        "buttonshadow" | "threedshadow" => 0xA0A0A0,
        "inactiveborder" => 0xF4F7FC,
        "inactivecaption" => 0xBFCDDB,
        "inactivecaptiontext" => 0x434E54,
        "infobackground" => 0xFFFFE1,
        "scrollbar" => 0xC8C8C8,
        "threeddarkshadow" => 0x696969,
        "threedlightshadow" => 0xE3E3E3,
        "windowframe" => 0x646464,
        _ => return None,
    };
    Some(Color::from_hex(h))
}

pub fn named_color(name: &str) -> Option<Color> {
    let lower = name.to_ascii_lowercase();
    if lower == "transparent" {
        return Some(Color::TRANSPARENT);
    }
    if let Ok(i) = NAMED_COLORS.binary_search_by(|(n, _)| (*n).cmp(lower.as_str()))
        && let Some(&(_, h)) = NAMED_COLORS.get(i)
    {
        return Some(Color::from_hex(h));
    }
    system_color(&lower)
}

fn clamp_u8(v: f64) -> u8 {
    if v.is_nan() { 0 } else { v.round().clamp(0.0, 255.0) as u8 }
}

fn alpha_u8(a: f64) -> u8 {
    clamp_u8(a.clamp(0.0, 1.0) * 255.0)
}

fn parse_hex(h: &str) -> Option<Color> {
    let d: Vec<u32> = h.chars().map(|c| c.to_digit(16)).collect::<Option<Vec<_>>>()?;
    let g = |i: usize| d.get(i).copied().unwrap_or(0);
    match d.len() {
        3 => Some(Color { r: (g(0) * 17) as u8, g: (g(1) * 17) as u8, b: (g(2) * 17) as u8, a: 255 }),
        4 => Some(Color { r: (g(0) * 17) as u8, g: (g(1) * 17) as u8, b: (g(2) * 17) as u8, a: (g(3) * 17) as u8 }),
        6 => Some(Color { r: (g(0) * 16 + g(1)) as u8, g: (g(2) * 16 + g(3)) as u8, b: (g(4) * 16 + g(5)) as u8, a: 255 }),
        8 => Some(Color {
            r: (g(0) * 16 + g(1)) as u8,
            g: (g(2) * 16 + g(3)) as u8,
            b: (g(4) * 16 + g(5)) as u8,
            a: (g(6) * 16 + g(7)) as u8,
        }),
        _ => None,
    }
}

/// One channel argument.
#[derive(Clone, Copy, Debug)]
enum Arg {
    None,
    Number(f64),
    Percent(f64),
    /// Angle already converted to degrees.
    Angle(f64),
}

fn parse_arg(c: &Cv) -> Option<Arg> {
    match c {
        Cv::Tok(Token::Number { value, .. }) => Some(Arg::Number(*value)),
        Cv::Tok(Token::Percentage { value }) => Some(Arg::Percent(*value)),
        Cv::Tok(Token::Dimension { value, unit, .. }) => {
            let deg = match unit.to_ascii_lowercase().as_str() {
                "deg" => *value,
                "grad" => *value * 0.9,
                "rad" => value.to_degrees(),
                "turn" => *value * 360.0,
                _ => return None,
            };
            Some(Arg::Angle(deg))
        }
        Cv::Tok(Token::Ident(i)) if i.eq_ignore_ascii_case("none") => Some(Arg::None),
        Cv::Func(name, items) if matches!(name.to_ascii_lowercase().as_str(), "calc") => {
            // calc() with plain numbers / percentages only.
            super::values::eval_calc_number_or_percent(items).map(|r| match r {
                (v, true) => Arg::Percent(v),
                (v, false) => Arg::Number(v),
            })
        }
        _ => None,
    }
}

fn hue_deg(a: Arg) -> Option<f64> {
    match a {
        Arg::None => Some(0.0),
        Arg::Number(v) | Arg::Angle(v) => Some(v),
        Arg::Percent(_) => None,
    }
}

/// Split the arguments of a color function: either `a, b, c[, d]` (legacy) or `a b c [/ d]`.
/// Returns (channel args, alpha arg, used commas).
fn color_args(items: &[Cv]) -> Option<(Vec<Arg>, Option<Arg>, bool)> {
    let items = trim_ws(items);
    if items.iter().any(Cv::is_comma) {
        let parts = split_commas(items);
        if parts.len() != 3 && parts.len() != 4 {
            return None;
        }
        let mut args = Vec::new();
        for p in &parts {
            let p = trim_ws(p);
            if p.len() != 1 {
                return None;
            }
            args.push(parse_arg(p.first()?)?);
        }
        let alpha = if args.len() == 4 { args.pop() } else { None };
        return Some((args, alpha, true));
    }
    // Space separated with optional "/ alpha".
    let mut chans: Vec<Arg> = Vec::new();
    let mut alpha = None;
    let mut seen_slash = false;
    for c in items {
        if c.is_ws() {
            continue;
        }
        if c.is_delim('/') {
            if seen_slash || chans.len() != 3 {
                return None;
            }
            seen_slash = true;
            continue;
        }
        let a = parse_arg(c)?;
        if seen_slash {
            if alpha.is_some() {
                return None;
            }
            alpha = Some(a);
        } else {
            chans.push(a);
        }
    }
    if chans.len() != 3 || (seen_slash && alpha.is_none()) {
        return None;
    }
    Some((chans, alpha, false))
}

fn alpha_value(a: Option<Arg>) -> Option<f64> {
    match a {
        None => Some(1.0),
        Some(Arg::None) => Some(0.0),
        Some(Arg::Number(v)) => Some(v),
        Some(Arg::Percent(p)) => Some(p / 100.0),
        Some(Arg::Angle(_)) => None,
    }
}

fn parse_rgb(items: &[Cv]) -> Option<Color> {
    let (args, alpha, legacy) = color_args(items)?;
    let a = alpha_value(alpha)?;
    let all_pct = args.iter().all(|a| matches!(a, Arg::Percent(_)));
    let all_num = args.iter().all(|a| matches!(a, Arg::Number(_)));
    if legacy && !(all_pct || all_num) {
        return None;
    }
    let mut out = [0u8; 3];
    for (i, arg) in args.iter().enumerate() {
        let v = match arg {
            Arg::None => 0.0,
            Arg::Number(v) => *v,
            Arg::Percent(p) => p * 255.0 / 100.0,
            Arg::Angle(_) => return None,
        };
        *out.get_mut(i)? = clamp_u8(v);
    }
    Some(Color { r: out[0], g: out[1], b: out[2], a: alpha_u8(a) })
}

fn hsl_to_rgb(h: f64, s: f64, l: f64) -> (f64, f64, f64) {
    let h = h.rem_euclid(360.0);
    let f = |n: f64| {
        let k = (n + h / 30.0) % 12.0;
        let a = s * l.min(1.0 - l);
        l - a * (k - 3.0).min(9.0 - k).clamp(-1.0, 1.0)
    };
    (f(0.0), f(8.0), f(4.0))
}

fn parse_hsl(items: &[Cv]) -> Option<Color> {
    let (args, alpha, legacy) = color_args(items)?;
    let a = alpha_value(alpha)?;
    let h = hue_deg(*args.first()?)?;
    let pct = |arg: Arg, legacy: bool| -> Option<f64> {
        match arg {
            Arg::None => Some(0.0),
            Arg::Percent(p) => Some(p),
            Arg::Number(n) if !legacy => Some(n),
            _ => None,
        }
    };
    let s = pct(*args.get(1)?, legacy)?.clamp(0.0, 100.0) / 100.0;
    let l = pct(*args.get(2)?, legacy)?.clamp(0.0, 100.0) / 100.0;
    let (r, g, b) = hsl_to_rgb(h, s, l);
    Some(Color { r: clamp_u8(r * 255.0), g: clamp_u8(g * 255.0), b: clamp_u8(b * 255.0), a: alpha_u8(a) })
}

fn parse_hwb(items: &[Cv]) -> Option<Color> {
    let (args, alpha, legacy) = color_args(items)?;
    if legacy {
        return None;
    }
    let a = alpha_value(alpha)?;
    let h = hue_deg(*args.first()?)?;
    let pct = |arg: Arg| -> Option<f64> {
        match arg {
            Arg::None => Some(0.0),
            Arg::Percent(p) => Some(p),
            Arg::Number(n) => Some(n),
            Arg::Angle(_) => None,
        }
    };
    let w = pct(*args.get(1)?)?.clamp(0.0, 100.0) / 100.0;
    let bl = pct(*args.get(2)?)?.clamp(0.0, 100.0) / 100.0;
    if w + bl >= 1.0 {
        let gray = w / (w + bl);
        let g = clamp_u8(gray * 255.0);
        return Some(Color { r: g, g, b: g, a: alpha_u8(a) });
    }
    let (r, g, b) = hsl_to_rgb(h, 1.0, 0.5);
    let mix = |c: f64| c * (1.0 - w - bl) + w;
    Some(Color { r: clamp_u8(mix(r) * 255.0), g: clamp_u8(mix(g) * 255.0), b: clamp_u8(mix(b) * 255.0), a: alpha_u8(a) })
}

fn mat3(m: &[[f64; 3]; 3], v: [f64; 3]) -> [f64; 3] {
    let row = |i: usize| m.get(i).map_or(0.0, |r| r[0] * v[0] + r[1] * v[1] + r[2] * v[2]);
    [row(0), row(1), row(2)]
}

fn gamma_srgb(v: f64) -> f64 {
    let sign = if v < 0.0 { -1.0 } else { 1.0 };
    let a = v.abs();
    if a > 0.0031308 { sign * (1.055 * a.powf(1.0 / 2.4) - 0.055) } else { 12.92 * v }
}

fn xyz_d65_to_srgb(xyz: [f64; 3], alpha: f64) -> Color {
    const M: [[f64; 3]; 3] = [
        [12831.0 / 3959.0, -329.0 / 214.0, -1974.0 / 3959.0],
        [-851781.0 / 878810.0, 1648619.0 / 878810.0, 36519.0 / 878810.0],
        [705.0 / 12673.0, -2585.0 / 12673.0, 705.0 / 667.0],
    ];
    let lin = mat3(&M, xyz);
    let c = |v: f64| clamp_u8(gamma_srgb(v) * 255.0);
    Color { r: c(lin[0]), g: c(lin[1]), b: c(lin[2]), a: alpha_u8(alpha) }
}

fn lab_to_srgb(l: f64, a: f64, b: f64, alpha: f64) -> Color {
    // css-color-4 sample code: Lab -> D50 XYZ -> D65 XYZ -> sRGB.
    const KAPPA: f64 = 24389.0 / 27.0;
    const EPS: f64 = 216.0 / 24389.0;
    const D50: [f64; 3] = [0.3457 / 0.3585, 1.0, (1.0 - 0.3457 - 0.3585) / 0.3585];
    let fy = (l + 16.0) / 116.0;
    let fx = a / 500.0 + fy;
    let fz = fy - b / 200.0;
    let cube = |f: f64| if f * f * f > EPS { f * f * f } else { (116.0 * f - 16.0) / KAPPA };
    let x = cube(fx) * D50[0];
    let y = (if l > KAPPA * EPS { ((l + 16.0) / 116.0).powi(3) } else { l / KAPPA }) * D50[1];
    let z = cube(fz) * D50[2];
    const D50_TO_D65: [[f64; 3]; 3] = [
        [0.955473421488075, -0.02309845494876471, 0.06325924320057072],
        [-0.0283697093338637, 1.0099953980813041, 0.021041441191917323],
        [0.012314014864481998, -0.020507649298898964, 1.330365926242124],
    ];
    xyz_d65_to_srgb(mat3(&D50_TO_D65, [x, y, z]), alpha)
}

fn oklab_to_srgb(l: f64, a: f64, b: f64, alpha: f64) -> Color {
    const OKLAB_TO_LMS: [[f64; 3]; 3] = [
        [1.0, 0.3963377773761749, 0.2158037573099136],
        [1.0, -0.1055613458156586, -0.0638541728258133],
        [1.0, -0.0894841775298119, -1.2914855480194092],
    ];
    const LMS_TO_XYZ: [[f64; 3]; 3] = [
        [1.2268798758459243, -0.5578149944602171, 0.2813910456659647],
        [-0.0405757452148008, 1.112286803280317, -0.0717110580655164],
        [-0.0763729366746601, -0.4214933324022432, 1.5869240198367816],
    ];
    let lms = mat3(&OKLAB_TO_LMS, [l, a, b]);
    let cubed = [lms[0].powi(3), lms[1].powi(3), lms[2].powi(3)];
    xyz_d65_to_srgb(mat3(&LMS_TO_XYZ, cubed), alpha)
}

/// `lab()`, `lch()`, `oklab()`, `oklch()`.
fn parse_lab_like(items: &[Cv], polar: bool, ok: bool) -> Option<Color> {
    let (args, alpha, legacy) = color_args(items)?;
    if legacy {
        return None;
    }
    let a = alpha_value(alpha)?;
    // Percent reference values: lightness 100 (Lab) / 1 (OK); a, b: 125 / 0.4; chroma: 150 / 0.4.
    let l_ref = if ok { 1.0 } else { 100.0 };
    let ab_ref = if ok { 0.4 } else { 125.0 };
    let c_ref = if ok { 0.4 } else { 150.0 };
    let scalar = |arg: Arg, pct_ref: f64| -> Option<f64> {
        match arg {
            Arg::None => Some(0.0),
            Arg::Number(v) => Some(v),
            Arg::Percent(p) => Some(p / 100.0 * pct_ref),
            Arg::Angle(_) => None,
        }
    };
    let l = scalar(*args.first()?, l_ref)?;
    let (x, y) = if polar {
        let c = scalar(*args.get(1)?, c_ref)?.max(0.0);
        let h = hue_deg(*args.get(2)?)?;
        (c * h.to_radians().cos(), c * h.to_radians().sin())
    } else {
        (scalar(*args.get(1)?, ab_ref)?, scalar(*args.get(2)?, ab_ref)?)
    };
    let l = if ok { l.clamp(0.0, 1.0) } else { l.clamp(0.0, 100.0) };
    Some(if ok { oklab_to_srgb(l, x, y, a) } else { lab_to_srgb(l, x, y, a) })
}

/// Parse a color from a single component value (after trimming white space).
pub fn parse_color_cv(cv: &Cv) -> Option<ColorValue> {
    match cv {
        Cv::Tok(Token::Ident(name)) => {
            if name.eq_ignore_ascii_case("currentcolor") {
                Some(ColorValue::CurrentColor)
            } else {
                named_color(name).map(ColorValue::Color)
            }
        }
        Cv::Tok(Token::Hash { value, .. }) => parse_hex(value).map(ColorValue::Color),
        Cv::Func(name, items) => {
            let c = match name.to_ascii_lowercase().as_str() {
                "rgb" | "rgba" => parse_rgb(items),
                "hsl" | "hsla" => parse_hsl(items),
                "hwb" => parse_hwb(items),
                "lab" => parse_lab_like(items, false, false),
                "lch" => parse_lab_like(items, true, false),
                "oklab" => parse_lab_like(items, false, true),
                "oklch" => parse_lab_like(items, true, true),
                _ => None,
            };
            c.map(ColorValue::Color)
        }
        _ => None,
    }
}

/// Parse a whole value as a color.
pub fn parse_color(cvs: &[Cv]) -> Option<ColorValue> {
    let cvs = trim_ws(cvs);
    if cvs.len() != 1 {
        return None;
    }
    parse_color_cv(cvs.first()?)
}

/// HTML legacy color parsing ("rules for parsing a legacy colour value", HTML 2.3.5) for the
/// `bgcolor`, `text`, `color` ... presentational attributes.
pub fn parse_legacy_color(input: &str) -> Option<Color> {
    let s = input.trim_matches(|c| matches!(c, '\t' | '\n' | '\x0c' | '\r' | ' '));
    if s.is_empty() {
        return None;
    }
    if s.eq_ignore_ascii_case("transparent") {
        return None;
    }
    if let Some(c) = NAMED_COLORS
        .binary_search_by(|(n, _)| (*n).cmp(s.to_ascii_lowercase().as_str()))
        .ok()
        .and_then(|i| NAMED_COLORS.get(i))
    {
        return Some(Color::from_hex(c.1));
    }
    // #rgb
    let b = s.as_bytes();
    if b.len() == 4 && b.first() == Some(&b'#') && b.iter().skip(1).all(u8::is_ascii_hexdigit) {
        let d = |i: usize| b.get(i).and_then(|&c| (c as char).to_digit(16)).unwrap_or(0) as u8;
        return Some(Color::rgb(d(1) * 17, d(2) * 17, d(3) * 17));
    }
    // Otherwise the legacy algorithm: replace non-BMP chars with "00", truncate to 128, strip '#',
    // replace non-hex with '0', pad to a multiple of 3, split in three, truncate to 8, strip leading zeros
    // beyond 2 digits, take the first two.
    let mut chars: Vec<char> = Vec::with_capacity(s.len().min(256));
    for c in s.chars() {
        if (c as u32) > 0xFFFF {
            // HTML 2.3.5 step 6: a code point above U+FFFF becomes the two characters "00" (before the 128 limit).
            chars.extend(['0', '0']);
        } else {
            chars.push(c);
        }
    }
    chars.truncate(128);
    if chars.first() == Some(&'#') {
        chars.remove(0);
    }
    let mut v: Vec<char> = chars.iter().map(|&c| if c.is_ascii_hexdigit() { c } else { '0' }).collect();
    while v.is_empty() || !v.len().is_multiple_of(3) {
        v.push('0');
    }
    let len = v.len() / 3;
    let mut comps: Vec<Vec<char>> = v.chunks(len).map(<[char]>::to_vec).collect();
    if len > 8 {
        for c in &mut comps {
            *c = c.iter().skip(len - 8).copied().collect();
        }
    }
    let len = comps.first().map_or(0, Vec::len);
    // Strip leading zeros while all components start with '0' and length > 2.
    let mut skip = 0usize;
    while len - skip > 2 && comps.iter().all(|c| c.get(skip) == Some(&'0')) {
        skip += 1;
    }
    let val = |c: &Vec<char>| -> u8 {
        let hex: String = c.iter().skip(skip).take(2).collect();
        u8::from_str_radix(&hex, 16).unwrap_or(0)
    };
    Some(Color::rgb(val(comps.first()?), val(comps.get(1)?), val(comps.get(2)?)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::css::syntax::parse_cvs;

    #[test]
    fn legacy_color_non_bmp_becomes_two_zeros() {
        // "1" + U+1F600 + "2" -> "1002" -> padded "100200" -> #100200 (HTML 2.3.5 step 6).
        assert_eq!(parse_legacy_color("1\u{1F600}2"), Some(Color::rgb(0x10, 0x02, 0x00)));
    }

    fn p(s: &str) -> Option<Color> {
        match parse_color(&parse_cvs(s))? {
            ColorValue::Color(c) => Some(c),
            ColorValue::CurrentColor => None,
        }
    }

    #[test]
    fn syntaxes() {
        assert_eq!(p("red"), Some(Color::rgb(255, 0, 0)));
        assert_eq!(p("#f00"), Some(Color::rgb(255, 0, 0)));
        assert_eq!(p("#FF000080"), Some(Color { r: 255, g: 0, b: 0, a: 128 }));
        assert_eq!(p("rgb(255, 0, 0)"), Some(Color::rgb(255, 0, 0)));
        assert_eq!(p("rgb(100% 0% 0% / 50%)"), Some(Color { r: 255, g: 0, b: 0, a: 128 }));
        assert_eq!(p("rgba(0,0,0,.5)"), Some(Color { r: 0, g: 0, b: 0, a: 128 }));
        assert_eq!(p("rgb(255 0 0, 1)"), None);
        assert_eq!(p("rgb(10%, 20, 30)"), None); // legacy syntax cannot mix
        assert_eq!(p("hsl(120, 100%, 50%)"), Some(Color::rgb(0, 255, 0)));
        assert_eq!(p("hsl(0.5turn 100% 50%)"), Some(Color::rgb(0, 255, 255)));
        assert_eq!(p("hwb(0 0% 0%)"), Some(Color::rgb(255, 0, 0)));
        assert_eq!(p("hwb(0 100% 100%)"), Some(Color::rgb(128, 128, 128)));
        assert_eq!(p("color-mix(in srgb, red, blue)"), None);
        assert_eq!(p("transparent"), Some(Color::TRANSPARENT));
        assert_eq!(p("CanvasText"), Some(Color::BLACK));
    }

    #[test]
    fn lab_conversions() {
        // Reference values (css-color-4 examples / Chrome): lab(50% 40 59.5) ~ rgb(193, 98, 0) region; check white and black.
        assert_eq!(p("lab(100% 0 0)"), Some(Color::WHITE));
        assert_eq!(p("lab(0% 0 0)"), Some(Color::BLACK));
        assert_eq!(p("oklab(100% 0 0)"), Some(Color::WHITE));
        assert_eq!(p("oklab(0 0 0)"), Some(Color::BLACK));
        assert_eq!(p("oklch(62.8% 0.2577 29.23)"), Some(Color::rgb(255, 0, 0)));
        assert_eq!(p("lch(54.29% 106.84 40.85)"), Some(Color::rgb(255, 0, 0)));
        assert_eq!(p("lab(54.29% 80.8 69.89)"), Some(Color::rgb(255, 0, 0)));
    }

    #[test]
    fn legacy_colors() {
        assert_eq!(parse_legacy_color("#ff8800"), Some(Color::rgb(255, 136, 0)));
        assert_eq!(parse_legacy_color("ff8800"), Some(Color::rgb(255, 136, 0)));
        assert_eq!(parse_legacy_color("red"), Some(Color::rgb(255, 0, 0)));
        assert_eq!(parse_legacy_color("#abc"), Some(Color::rgb(170, 187, 204)));
        assert_eq!(parse_legacy_color(""), None);
    }
}
