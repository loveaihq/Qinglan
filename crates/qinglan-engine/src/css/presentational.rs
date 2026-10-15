//! Presentational hints: the legacy HTML attributes that map to CSS (HTML 15.3 and 15.4.3):
//! `bgcolor`, `text`, `link`, `align`, `valign`, `width`, `height`, `border`, `cellpadding`, `hspace`,
//! `vspace`, `<font color face size>`, `<hr>` attributes and `<body>` margins.
//!
//! The hints are returned as a declaration block that takes part in the cascade as the weakest
//! author-level declarations (specificity 0, before every author rule).

use std::rc::Rc;

use super::color::{Color, ColorValue, parse_legacy_color};
use super::types::*;
use super::values::Lp;
use crate::html::atoms::{FxHashMap, a};
use crate::html::dom::{Document, NONE, Namespace, NodeId};

pub struct HintBuilder {
    /// By attribute atom: does the attribute take part in some presentational hint?
    relevant_attr: Vec<bool>,
    cache: FxHashMap<String, Option<Rc<DeclBlock>>>,
    next_id: u32,
    body_link: Option<Color>,
    quirks: bool,
}

type Hints = Vec<(PropId, Value)>;

fn lp(v: f32) -> Value {
    Value::Dim(Dim::Lp(Lp::px(v)))
}

fn color(c: Color) -> Value {
    Value::Color(ColorValue::Color(c))
}

/// "Rules for parsing non-negative integers" (HTML 2.3.4.2): leading white space, optional `+`, digits.
fn parse_nonneg_int(s: &str) -> Option<u32> {
    let t = s.trim_start_matches(['\t', '\n', '\x0c', '\r', ' ']);
    let t = t.strip_prefix('+').unwrap_or(t);
    let digits: String = t.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        return None;
    }
    Some(digits.parse::<u64>().unwrap_or(u64::MAX).min(u32::MAX as u64) as u32)
}

/// "Rules for parsing dimension values" (HTML 2.3.5.3): a length (px) or a percentage.
fn parse_dimension(s: &str) -> Option<Lp> {
    let t = s.trim_start_matches(['\t', '\n', '\x0c', '\r', ' ']);
    let mut digits = String::new();
    let mut chars = t.chars().peekable();
    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() {
            digits.push(c);
            chars.next();
        } else {
            break;
        }
    }
    if digits.is_empty() {
        return None;
    }
    let mut frac = String::new();
    if chars.peek() == Some(&'.') {
        chars.next();
        while let Some(&c) = chars.peek() {
            if c.is_ascii_digit() {
                frac.push(c);
                chars.next();
            } else {
                break;
            }
        }
    }
    let text = if frac.is_empty() { digits } else { format!("{digits}.{frac}") };
    let v: f32 = text.parse().ok()?;
    if chars.peek() == Some(&'%') { Some(Lp::Pct(v)) } else { Some(Lp::px(v)) }
}

fn dim_value(l: Lp) -> Value {
    Value::Dim(Dim::Lp(l))
}

impl HintBuilder {
    pub fn new(doc: &Document) -> HintBuilder {
        let mut body_link = None;
        let root = doc.document_element();
        if root != NONE {
            for c in doc.children(root) {
                if doc.is_html_element(c, a::BODY) {
                    body_link = attr(doc, c, "link").and_then(|v| parse_legacy_color(&v));
                }
            }
        }
        // `href` only matters for the `link` colour of `<body link>`.
        let mut relevant_attr = vec![false; doc.atoms.len()];
        for name in [
            "bgcolor", "text", "marginwidth", "marginheight", "leftmargin", "rightmargin", "topmargin", "bottommargin", "color", "face", "size",
            "bordercolor", "border", "width", "height", "align", "valign", "nowrap", "hspace", "vspace", "frameborder", "noshade", "type",
        ]
        .into_iter()
        .chain(body_link.map(|_| "href"))
        {
            if let Some(at) = doc.atoms.lookup(name)
                && let Some(slot) = relevant_attr.get_mut(at.0 as usize)
            {
                *slot = true;
            }
        }
        HintBuilder {
            relevant_attr,
            cache: FxHashMap::default(),
            next_id: 0x8000_0000,
            body_link,
            quirks: doc.quirks == crate::html::dom::QuirksMode::Quirks,
        }
    }

    /// The hint declarations of `el`, if it has any.
    pub fn hints_for(&mut self, doc: &Document, el: NodeId) -> Option<Rc<DeclBlock>> {
        let e = doc.element(el)?;
        let is_cell = e.ns == Namespace::Html && matches!(e.name, a::TD | a::TH);
        if e.ns != Namespace::Html || (e.attrs.is_empty() && !is_cell) {
            return None;
        }
        // Cheap rejection: most elements carry no presentational attribute at all.
        let relevant = e.attrs.iter().any(|x| x.ns == Namespace::None && self.relevant_attr.get(x.name.0 as usize).copied().unwrap_or(false));
        if !relevant && !is_cell {
            return None;
        }
        let name = doc.atoms.get(e.name);
        // Cheap rejection: only these elements have hints.
        if !matches!(
            name,
            "body" | "font" | "table" | "td" | "th" | "tr" | "thead" | "tbody" | "tfoot" | "col" | "colgroup" | "div" | "center" | "p"
                | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "img" | "embed" | "object" | "iframe" | "input" | "hr" | "a" | "area"
                | "caption" | "ul" | "ol" | "li" | "dl" | "dd" | "dt" | "pre" | "br" | "marquee" | "applet" | "video"
        ) {
            return None;
        }
        let mut hints: Hints = Vec::new();
        let mut key = String::from(name);
        let mut used = |k: &str, v: &str| {
            key.push('|');
            key.push_str(k);
            key.push('=');
            key.push_str(v);
        };
        let at = |n: &str| attr(doc, el, n);
        // Text alignment via `align`.
        let align_text = |v: &str| -> Option<TextAlign> {
            match v.to_ascii_lowercase().as_str() {
                "center" | "middle" => Some(TextAlign::Center),
                "left" => Some(TextAlign::Left),
                "right" => Some(TextAlign::Right),
                "justify" => Some(TextAlign::Justify),
                _ => None,
            }
        };
        match name {
            "body" => {
                if let Some(v) = at("bgcolor") {
                    used("bgcolor", &v);
                    if let Some(c) = parse_legacy_color(&v) {
                        hints.push((PropId::BackgroundColor, color(c)));
                    }
                }
                if let Some(v) = at("text") {
                    used("text", &v);
                    if let Some(c) = parse_legacy_color(&v) {
                        hints.push((PropId::Color, color(c)));
                    }
                }
                for (names, props) in [
                    (["marginwidth", "leftmargin"], [PropId::MarginLeft, PropId::MarginRight]),
                    (["marginheight", "topmargin"], [PropId::MarginTop, PropId::MarginBottom]),
                ] {
                    if let Some((n, v)) = names.iter().find_map(|n| at(n).map(|v| (*n, v))) {
                        used(n, &v);
                        let px = parse_nonneg_int(&v).map_or(8.0, |x| x as f32);
                        for p in props {
                            hints.push((p, lp(px)));
                        }
                    }
                }
                for (n, p) in [("rightmargin", PropId::MarginRight), ("bottommargin", PropId::MarginBottom)] {
                    if let Some(v) = at(n) {
                        used(n, &v);
                        hints.push((p, lp(parse_nonneg_int(&v).map_or(8.0, |x| x as f32))));
                    }
                }
            }
            "font" => {
                if let Some(v) = at("color") {
                    used("color", &v);
                    if let Some(c) = parse_legacy_color(&v) {
                        hints.push((PropId::Color, color(c)));
                    }
                }
                if let Some(v) = at("face") {
                    used("face", &v);
                    let fam: Vec<FontFamily> = v
                        .split(',')
                        .map(|f| f.trim().trim_matches(|c| c == '"' || c == '\''))
                        .filter(|f| !f.is_empty())
                        .map(|f| match GenericFamily::from_keyword(f) {
                            Some(g) => FontFamily::Generic(g),
                            None => FontFamily::Named(Rc::from(f)),
                        })
                        .collect();
                    if !fam.is_empty() {
                        hints.push((PropId::FontFamily, Value::FontFamily(Rc::new(fam))));
                    }
                }
                if let Some(v) = at("size") {
                    used("size", &v);
                    if let Some(k) = legacy_font_size(&v) {
                        hints.push((PropId::FontSize, Value::FontSize(FontSizeSpec::Absolute(k))));
                    }
                }
            }
            "table" => {
                if let Some(v) = at("bgcolor") {
                    used("bgcolor", &v);
                    if let Some(c) = parse_legacy_color(&v) {
                        hints.push((PropId::BackgroundColor, color(c)));
                    }
                }
                if let Some(v) = at("bordercolor") {
                    used("bordercolor", &v);
                    if let Some(c) = parse_legacy_color(&v) {
                        for p in [PropId::BorderTopColor, PropId::BorderRightColor, PropId::BorderBottomColor, PropId::BorderLeftColor] {
                            hints.push((p, color(c)));
                        }
                    }
                }
                if let Some(v) = at("border") {
                    used("border", &v);
                    let w = parse_nonneg_int(&v).unwrap_or(1);
                    let style = if w > 0 { BorderLineStyle::Outset } else { BorderLineStyle::None };
                    for p in [PropId::BorderTopWidth, PropId::BorderRightWidth, PropId::BorderBottomWidth, PropId::BorderLeftWidth] {
                        hints.push((p, Value::BorderWidth(BorderWidthSpec::Lp(Lp::px(w as f32)))));
                    }
                    for p in [PropId::BorderTopStyle, PropId::BorderRightStyle, PropId::BorderBottomStyle, PropId::BorderLeftStyle] {
                        hints.push((p, Value::BorderStyle(style)));
                    }
                }
                if let Some(v) = at("width") {
                    used("width", &v);
                    if let Some(d) = parse_dimension(&v).filter(|d| !matches!(d, Lp::Len(x, _) | Lp::Pct(x) if *x == 0.0)) {
                        hints.push((PropId::Width, dim_value(d)));
                    }
                }
                if let Some(v) = at("height") {
                    used("height", &v);
                    if let Some(d) = parse_dimension(&v) {
                        hints.push((PropId::Height, dim_value(d)));
                    }
                }
                if let Some(v) = at("align") {
                    used("align", &v);
                    match v.to_ascii_lowercase().as_str() {
                        "left" => hints.push((PropId::Float, Value::Float(Float::Left))),
                        "right" => hints.push((PropId::Float, Value::Float(Float::Right))),
                        "center" | "middle" => {
                            hints.push((PropId::MarginLeft, Value::Dim(Dim::Auto)));
                            hints.push((PropId::MarginRight, Value::Dim(Dim::Auto)));
                        }
                        _ => {}
                    }
                }
            }
            "td" | "th" | "tr" | "thead" | "tbody" | "tfoot" | "col" | "colgroup" | "caption" => {
                if matches!(name, "td" | "th" | "tr" | "thead" | "tbody" | "tfoot")
                    && let Some(v) = at("bgcolor")
                {
                    used("bgcolor", &v);
                    if let Some(c) = parse_legacy_color(&v) {
                        hints.push((PropId::BackgroundColor, color(c)));
                    }
                }
                if matches!(name, "td" | "th" | "col" | "colgroup")
                    && let Some(v) = at("width")
                {
                    used("width", &v);
                    if let Some(d) = parse_dimension(&v).filter(|d| !matches!(d, Lp::Len(x, _) | Lp::Pct(x) if *x == 0.0)) {
                        hints.push((PropId::Width, dim_value(d)));
                    }
                }
                if matches!(name, "td" | "th" | "tr" | "thead" | "tbody" | "tfoot")
                    && let Some(v) = at("height")
                {
                    used("height", &v);
                    if let Some(d) = parse_dimension(&v).filter(|d| name == "tr" || !matches!(d, Lp::Len(x, _) | Lp::Pct(x) if *x == 0.0)) {
                        hints.push((PropId::Height, dim_value(d)));
                    }
                }
                if let Some(v) = at("align") {
                    used("align", &v);
                    if let Some(t) = align_text(&v) {
                        hints.push((PropId::TextAlign, Value::TextAlign(t)));
                    }
                }
                if let Some(v) = at("valign") {
                    used("valign", &v);
                    let va = match v.to_ascii_lowercase().as_str() {
                        "top" => Some(VerticalAlign::Top),
                        "middle" => Some(VerticalAlign::Middle),
                        "bottom" => Some(VerticalAlign::Bottom),
                        "baseline" => Some(VerticalAlign::Baseline),
                        _ => None,
                    };
                    if let Some(va) = va {
                        hints.push((PropId::VerticalAlign, Value::VerticalAlign(va)));
                    }
                }
                if matches!(name, "td" | "th") {
                    if at("nowrap").is_some() {
                        // In quirks mode a nowrap cell with a length width wraps normally.
                        let wraps = self.quirks && at("width").is_some_and(|w| matches!(parse_dimension(&w), Some(Lp::Len(..))));
                        used("nowrap", if wraps { "wrap" } else { "1" });
                        if !wraps {
                            hints.push((PropId::TextWrapMode, Value::TextWrapMode(TextWrapMode::Nowrap)));
                        }
                    }
                    if let Some(table) = nearest_table(doc, el) {
                        // cellpadding of the table.
                        if let Some(v) = attr(doc, table, "cellpadding") {
                            used("cellpadding", &v);
                            if let Some(px) = parse_nonneg_int(&v) {
                                for p in [PropId::PaddingTop, PropId::PaddingRight, PropId::PaddingBottom, PropId::PaddingLeft] {
                                    hints.push((p, lp(px as f32)));
                                }
                            }
                        }
                        // `table[border] > ... > td`: 1px inset, only if the border is not zero (HTML 15.3.8).
                        if let Some(v) = attr(doc, table, "border")
                            && parse_nonneg_int(&v) != Some(0)
                        {
                            used("tborder", "1");
                            for p in [PropId::BorderTopWidth, PropId::BorderRightWidth, PropId::BorderBottomWidth, PropId::BorderLeftWidth] {
                                hints.push((p, Value::BorderWidth(BorderWidthSpec::Lp(Lp::px(1.0)))));
                            }
                            for p in [PropId::BorderTopStyle, PropId::BorderRightStyle, PropId::BorderBottomStyle, PropId::BorderLeftStyle] {
                                hints.push((p, Value::BorderStyle(BorderLineStyle::Inset)));
                            }
                        }
                    }
                }
            }
            "div" | "center" | "p" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                if let Some(v) = at("align") {
                    used("align", &v);
                    if let Some(t) = align_text(&v) {
                        hints.push((PropId::TextAlign, Value::TextAlign(t)));
                    }
                }
            }
            "img" | "embed" | "object" | "iframe" | "input" | "video" | "applet" | "marquee" => {
                let is_image_input = name != "input" || at("type").is_some_and(|t| t.eq_ignore_ascii_case("image"));
                if is_image_input {
                    if let Some(v) = at("width") {
                        used("width", &v);
                        if let Some(d) = parse_dimension(&v) {
                            hints.push((PropId::Width, dim_value(d)));
                        }
                    }
                    if let Some(v) = at("height") {
                        used("height", &v);
                        if let Some(d) = parse_dimension(&v) {
                            hints.push((PropId::Height, dim_value(d)));
                        }
                    }
                    if let Some(v) = at("align") {
                        used("align", &v);
                        match v.to_ascii_lowercase().as_str() {
                            "left" => {
                                hints.push((PropId::Float, Value::Float(Float::Left)));
                            }
                            "right" => {
                                hints.push((PropId::Float, Value::Float(Float::Right)));
                            }
                            "top" => hints.push((PropId::VerticalAlign, Value::VerticalAlign(VerticalAlign::Top))),
                            "baseline" => hints.push((PropId::VerticalAlign, Value::VerticalAlign(VerticalAlign::Baseline))),
                            "texttop" => hints.push((PropId::VerticalAlign, Value::VerticalAlign(VerticalAlign::TextTop))),
                            "absmiddle" | "abscenter" | "center" | "middle" => {
                                hints.push((PropId::VerticalAlign, Value::VerticalAlign(VerticalAlign::Middle)))
                            }
                            "bottom" => hints.push((PropId::VerticalAlign, Value::VerticalAlign(VerticalAlign::Bottom))),
                            _ => {}
                        }
                    }
                    if let Some(v) = at("hspace") {
                        used("hspace", &v);
                        if let Some(px) = parse_nonneg_int(&v) {
                            hints.push((PropId::MarginLeft, lp(px as f32)));
                            hints.push((PropId::MarginRight, lp(px as f32)));
                        }
                    }
                    if let Some(v) = at("vspace") {
                        used("vspace", &v);
                        if let Some(px) = parse_nonneg_int(&v) {
                            hints.push((PropId::MarginTop, lp(px as f32)));
                            hints.push((PropId::MarginBottom, lp(px as f32)));
                        }
                    }
                    if matches!(name, "img" | "object" | "input")
                        && let Some(v) = at("border")
                    {
                        used("border", &v);
                        if let Some(px) = parse_nonneg_int(&v).filter(|p| *p > 0) {
                            for p in [PropId::BorderTopWidth, PropId::BorderRightWidth, PropId::BorderBottomWidth, PropId::BorderLeftWidth] {
                                hints.push((p, Value::BorderWidth(BorderWidthSpec::Lp(Lp::px(px as f32)))));
                            }
                            for p in [PropId::BorderTopStyle, PropId::BorderRightStyle, PropId::BorderBottomStyle, PropId::BorderLeftStyle] {
                                hints.push((p, Value::BorderStyle(BorderLineStyle::Solid)));
                            }
                        }
                    }
                }
                if name == "iframe"
                    && let Some(v) = at("frameborder")
                {
                    used("frameborder", &v);
                    if parse_nonneg_int(&v).is_none_or(|x| x == 0) {
                        for p in [PropId::BorderTopWidth, PropId::BorderRightWidth, PropId::BorderBottomWidth, PropId::BorderLeftWidth] {
                            hints.push((p, Value::BorderWidth(BorderWidthSpec::Lp(Lp::px(0.0)))));
                        }
                    }
                }
            }
            "hr" => {
                if let Some(v) = at("align") {
                    used("align", &v);
                    match v.to_ascii_lowercase().as_str() {
                        "left" => {
                            hints.push((PropId::MarginLeft, lp(0.0)));
                            hints.push((PropId::MarginRight, Value::Dim(Dim::Auto)));
                        }
                        "right" => {
                            hints.push((PropId::MarginLeft, Value::Dim(Dim::Auto)));
                            hints.push((PropId::MarginRight, lp(0.0)));
                        }
                        "center" => {
                            hints.push((PropId::MarginLeft, Value::Dim(Dim::Auto)));
                            hints.push((PropId::MarginRight, Value::Dim(Dim::Auto)));
                        }
                        _ => {}
                    }
                }
                let has_color = at("color");
                let has_noshade = at("noshade").is_some();
                if let Some(c) = &has_color {
                    used("color", c);
                }
                if has_noshade {
                    used("noshade", "1");
                }
                if has_color.is_some() || has_noshade {
                    for p in [PropId::BorderTopStyle, PropId::BorderRightStyle, PropId::BorderBottomStyle, PropId::BorderLeftStyle] {
                        hints.push((p, Value::BorderStyle(BorderLineStyle::Solid)));
                    }
                }
                if let Some(c) = has_color.as_deref().and_then(parse_legacy_color) {
                    for p in [PropId::BorderTopColor, PropId::BorderRightColor, PropId::BorderBottomColor, PropId::BorderLeftColor] {
                        hints.push((p, color(c)));
                    }
                    hints.push((PropId::BackgroundColor, color(c)));
                }
                if let Some(v) = at("size") {
                    used("size", &v);
                    if let Some(sz) = parse_nonneg_int(&v) {
                        if has_color.is_some() || has_noshade {
                            let w = Lp::px(sz as f32 / 2.0);
                            for p in [PropId::BorderTopWidth, PropId::BorderRightWidth, PropId::BorderBottomWidth, PropId::BorderLeftWidth] {
                                hints.push((p, Value::BorderWidth(BorderWidthSpec::Lp(w.clone()))));
                            }
                        } else if sz == 1 {
                            hints.push((PropId::BorderBottomWidth, Value::BorderWidth(BorderWidthSpec::Lp(Lp::px(0.0)))));
                        } else if sz > 1 {
                            hints.push((PropId::Height, lp(sz.saturating_sub(2) as f32)));
                        }
                    }
                }
                if let Some(v) = at("width") {
                    used("width", &v);
                    if let Some(d) = parse_dimension(&v) {
                        hints.push((PropId::Width, dim_value(d)));
                    }
                }
            }
            "a" | "area" => {
                if attr(doc, el, "href").is_some()
                    && let Some(c) = self.body_link
                {
                    used("link", &c.to_css());
                    hints.push((PropId::Color, color(c)));
                }
            }
            _ => {}
        }
        if hints.is_empty() {
            return None;
        }
        if let Some(found) = self.cache.get(&key) {
            return found.clone();
        }
        let decls: Vec<Declaration> =
            hints.into_iter().map(|(p, v)| Declaration { prop: DeclProp::Known(p), value: Specified::Value(v), important: false }).collect();
        self.next_id += 1;
        let block = Rc::new(DeclBlock { decls, id: self.next_id });
        self.cache.insert(key, Some(block.clone()));
        Some(block)
    }
}

/// The attribute `name` of an HTML element (no namespace).
fn attr(doc: &Document, el: NodeId, name: &str) -> Option<String> {
    let e = doc.element(el)?;
    e.attrs.iter().find(|x| x.ns == Namespace::None && doc.atoms.get(x.name) == name).map(|x| x.value.clone())
}

fn nearest_table(doc: &Document, el: NodeId) -> Option<NodeId> {
    let mut cur = doc.parent(el);
    let mut hops = 0;
    while cur != NONE && hops < 8 {
        if doc.is_html_element(cur, a::TABLE) {
            return Some(cur);
        }
        cur = doc.parent(cur);
        hops += 1;
    }
    None
}

/// "Rules for parsing a legacy font size" (HTML 15.3.4): the absolute-size keyword index.
fn legacy_font_size(input: &str) -> Option<u8> {
    let t = input.trim_start_matches(['\t', '\n', '\x0c', '\r', ' ']);
    let (mode, rest) = match t.chars().next()? {
        '+' => (1, t.get(1..)?),
        '-' => (-1, t.get(1..)?),
        _ => (0, t),
    };
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        return None;
    }
    let mut value: i64 = digits.parse::<i64>().unwrap_or(i64::MAX / 4);
    if mode == 1 {
        value += 3;
    } else if mode == -1 {
        value = 3 - value;
    }
    let value = value.clamp(1, 7);
    // 1 -> x-small (index 1) ... 7 -> xxx-large (index 7).
    Some(value as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn font_sizes() {
        assert_eq!(legacy_font_size("3"), Some(3));
        assert_eq!(legacy_font_size("+2"), Some(5));
        assert_eq!(legacy_font_size("-2"), Some(1));
        assert_eq!(legacy_font_size("9"), Some(7));
        assert_eq!(legacy_font_size("x"), None);
    }

    #[test]
    fn dimensions() {
        assert_eq!(parse_dimension("50%"), Some(Lp::Pct(50.0)));
        assert_eq!(parse_dimension(" 120px"), Some(Lp::px(120.0)));
        assert_eq!(parse_dimension("12.5"), Some(Lp::px(12.5)));
        assert_eq!(parse_dimension("auto"), None);
    }
}
