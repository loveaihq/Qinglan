//! Style sheet structure: rules, at-rules (`@media`, `@import`, `@layer`, `@supports`) and the
//! parsing of whole style sheets, `<style>` contents and `style=""` attributes
//! (css-syntax-3, css-cascade-5, css-conditional-3, mediaqueries-4).
//!
//! `@font-face`, `@keyframes`, `@container`, `@page`, `@namespace` and other at-rules are ignored in
//! layer 1. CSS nesting is not supported (nested rules are dropped).

use std::rc::Rc;

use super::media::{MediaQueryList, Tri};
use super::parse_prop::{ParseEnv, parse_declaration};
use super::selector::{ParseCtx, Selector, parse_selector_list};
use super::syntax::{BlockKind, Cv, MAX_CONDITION_NESTING, RawRule, nests_deeper_than, parse_cvs, parse_declaration_list, parse_rule_list, split_commas, trim_ws};
use super::tokenizer::Token;
use super::types::{DeclBlock, DeclProp, Declaration, PropId};
use crate::html::atoms::Interner;

/// Nesting limit for `@media` / `@supports` / `@layer` blocks.
const MAX_RULE_NESTING: usize = 32;
/// A selector with more compounds than this is dropped.
const MAX_COMPOUNDS: usize = 128;

pub struct SheetCtx<'a> {
    pub atoms: &'a mut Interner,
    pub quirks: bool,
    /// Counters that make declaration blocks and `var()` declarations identifiable.
    pub var_id: &'a mut u32,
    pub block_id: &'a mut u32,
}

pub struct StyleRule {
    /// Shared with the rule index (`Rc`), so indexing a rule does not copy its selector.
    pub selectors: Vec<Rc<Selector>>,
    pub block: Rc<DeclBlock>,
}

pub struct ImportRule {
    pub url: String,
    pub media: MediaQueryList,
    /// `layer` / `layer(name)`: `Some(None)` is an anonymous layer.
    pub layer: Option<Option<Vec<String>>>,
}

pub enum Rule {
    Style(StyleRule),
    Media(MediaQueryList, Vec<Rule>),
    /// `@layer name { ... }` (`None`: anonymous).
    Layer(Option<Vec<String>>, Vec<Rule>),
    /// `@layer a, b.c;`
    LayerStatement(Vec<Vec<String>>),
    Import(ImportRule),
}

#[derive(Default)]
pub struct Stylesheet {
    pub rules: Vec<Rule>,
}

pub fn parse_stylesheet(text: &str, ctx: &mut SheetCtx<'_>) -> Stylesheet {
    let raw = parse_rule_list(parse_cvs(text), true);
    Stylesheet { rules: convert_rules(raw, ctx, true, 0) }
}

fn convert_rules(raw: Vec<RawRule>, ctx: &mut SheetCtx<'_>, top_level: bool, depth: usize) -> Vec<Rule> {
    let mut out = Vec::new();
    // @import is only valid before everything but @charset and @layer statements.
    let mut imports_allowed = top_level;
    for r in raw {
        match r.at_name.as_deref() {
            None => {
                imports_allowed = false;
                if let Some(rule) = style_rule(r, ctx) {
                    out.push(Rule::Style(rule));
                }
            }
            Some("charset") => {}
            Some("import") => {
                if imports_allowed && r.block.is_none() && let Some(i) = parse_import(&r.prelude, ctx) {
                    out.push(Rule::Import(i));
                }
            }
            Some("layer") => match r.block {
                None => {
                    let names = parse_layer_names(&r.prelude);
                    if let Some(n) = names {
                        out.push(Rule::LayerStatement(n));
                    }
                }
                Some(body) => {
                    if depth >= MAX_RULE_NESTING {
                        continue;
                    }
                    let name = if trim_ws(&r.prelude).is_empty() {
                        None
                    } else {
                        match parse_layer_names(&r.prelude) {
                            Some(mut v) if v.len() == 1 => v.pop(),
                            _ => continue,
                        }
                    };
                    let inner = convert_rules(parse_rule_list(body, false), ctx, false, depth + 1);
                    imports_allowed = false;
                    out.push(Rule::Layer(name, inner));
                }
            },
            Some("media") => {
                imports_allowed = false;
                let Some(body) = r.block else { continue };
                if depth >= MAX_RULE_NESTING {
                    continue;
                }
                let q = MediaQueryList::parse(&r.prelude);
                let inner = convert_rules(parse_rule_list(body, false), ctx, false, depth + 1);
                out.push(Rule::Media(q, inner));
            }
            Some("supports") => {
                imports_allowed = false;
                let Some(body) = r.block else { continue };
                if depth >= MAX_RULE_NESTING {
                    continue;
                }
                if eval_supports(&r.prelude, ctx) == Tri::True {
                    out.extend(convert_rules(parse_rule_list(body, false), ctx, false, depth + 1));
                }
            }
            Some(_) => {
                // @font-face, @keyframes, @container, @page, @namespace, ...: ignored.
                imports_allowed = false;
            }
        }
    }
    out
}

fn style_rule(r: RawRule, ctx: &mut SheetCtx<'_>) -> Option<StyleRule> {
    let body = r.block?;
    let selectors = {
        let mut pc = ParseCtx { atoms: ctx.atoms, quirks: ctx.quirks, depth: 0 };
        parse_selector_list(&r.prelude, &mut pc, false, true)?
    };
    let selectors: Vec<Rc<Selector>> = selectors.into_iter().filter(|s| s.compounds.len() <= MAX_COMPOUNDS).map(Rc::new).collect();
    if selectors.is_empty() {
        return None;
    }
    let block = parse_declaration_block(body, ctx);
    if block.decls.is_empty() {
        return None;
    }
    Some(StyleRule { selectors, block })
}

/// Parse the contents of a declaration block into declarations (invalid ones are dropped).
pub fn parse_declaration_block(cvs: Vec<Cv>, ctx: &mut SheetCtx<'_>) -> Rc<DeclBlock> {
    let env = ParseEnv { quirks: ctx.quirks };
    let mut decls = Vec::new();
    for d in parse_declaration_list(cvs) {
        if d.name.starts_with("--") {
            let name = ctx.atoms.intern(&d.name);
            decls.push(Declaration {
                prop: DeclProp::Custom(name, Rc::new(trim_ws(&d.value).to_vec())),
                value: super::types::Specified::Unset,
                important: d.important,
            });
            continue;
        }
        let Some(parsed) = parse_declaration(&d.name, &d.value, &env, ctx.var_id) else { continue };
        for (prop, value) in parsed {
            decls.push(Declaration { prop: DeclProp::Known(prop), value, important: d.important });
        }
    }
    *ctx.block_id += 1;
    Rc::new(DeclBlock { decls, id: *ctx.block_id })
}

/// The `style` attribute of an element.
pub fn parse_style_attribute(text: &str, ctx: &mut SheetCtx<'_>) -> Rc<DeclBlock> {
    parse_declaration_block(parse_cvs(text), ctx)
}

fn parse_layer_names(prelude: &[Cv]) -> Option<Vec<Vec<String>>> {
    let mut out = Vec::new();
    for part in split_commas(trim_ws(prelude)) {
        let mut path = Vec::new();
        let mut expect_ident = true;
        for c in trim_ws(part) {
            if expect_ident {
                path.push(c.ident()?.to_string());
                expect_ident = false;
            } else if c.is_delim('.') {
                expect_ident = true;
            } else {
                return None;
            }
        }
        if path.is_empty() || expect_ident {
            return None;
        }
        out.push(path);
    }
    if out.is_empty() { None } else { Some(out) }
}

fn parse_import(prelude: &[Cv], ctx: &mut SheetCtx<'_>) -> Option<ImportRule> {
    let items: Vec<&Cv> = prelude.iter().filter(|c| !c.is_ws()).collect();
    let url = match items.first()? {
        Cv::Tok(Token::Url(u)) => u.clone(),
        Cv::Tok(Token::Str(s)) => s.clone(),
        Cv::Func(n, inner) if n.eq_ignore_ascii_case("url") => match trim_ws(inner) {
            [Cv::Tok(Token::Str(s))] => s.clone(),
            _ => return None,
        },
        _ => return None,
    };
    let mut i = 1usize;
    let mut layer = None;
    match items.get(i) {
        Some(c) if c.is_ident("layer") => {
            layer = Some(None);
            i += 1;
        }
        Some(Cv::Func(n, inner)) if n.eq_ignore_ascii_case("layer") => {
            let mut names = parse_layer_names(inner)?;
            if names.len() != 1 {
                return None;
            }
            layer = Some(names.pop());
            i += 1;
        }
        _ => {}
    }
    if let Some(Cv::Func(n, inner)) = items.get(i)
        && n.eq_ignore_ascii_case("supports")
    {
        // supports(<supports-condition> | <declaration>)
        let ok = match eval_supports(inner, ctx) {
            Tri::True => true,
            _ => supports_declaration(inner, ctx),
        };
        if !ok {
            return None;
        }
        i += 1;
    }
    let rest: Vec<Cv> = items.iter().skip(i).map(|c| (*c).clone()).collect();
    let media = MediaQueryList::parse(&rest);
    Some(ImportRule { url, media, layer })
}

// -----------------------------------------------------------------------------------------------
// @supports

fn supports_declaration(inner: &[Cv], ctx: &mut SheetCtx<'_>) -> bool {
    let decls = parse_declaration_list(inner.to_vec());
    let [d] = decls.as_slice() else { return false };
    let env = ParseEnv { quirks: false };
    let mut tmp = 0u32;
    if d.name.starts_with("--") {
        return true;
    }
    let _ = &ctx.atoms;
    parse_declaration(&d.name, &d.value, &env, &mut tmp).is_some()
}

/// Evaluate a `<supports-condition>`. A condition nested deeper than `MAX_CONDITION_NESTING` is false.
pub fn eval_supports(cvs: &[Cv], ctx: &mut SheetCtx<'_>) -> Tri {
    if nests_deeper_than(cvs, MAX_CONDITION_NESTING) {
        return Tri::False;
    }
    eval_supports_inner(cvs, ctx)
}

fn eval_supports_inner(cvs: &[Cv], ctx: &mut SheetCtx<'_>) -> Tri {
    let items: Vec<&Cv> = cvs.iter().filter(|c| !c.is_ws()).collect();
    if items.is_empty() {
        return Tri::False;
    }
    if items.first().is_some_and(|c| c.is_ident("not")) {
        return match items.as_slice() {
            [_, one] => eval_in_parens(one, ctx).not_tri(),
            _ => Tri::False,
        };
    }
    let mut result: Option<Tri> = None;
    let mut op: Option<&str> = None;
    let mut i = 0usize;
    loop {
        let Some(item) = items.get(i) else { return Tri::False };
        let v = eval_in_parens(item, ctx);
        result = Some(match (result, op) {
            (None, _) => v,
            (Some(a), Some("and")) => and_tri(a, v),
            (Some(a), Some("or")) => or_tri(a, v),
            _ => return Tri::False,
        });
        i += 1;
        match items.get(i) {
            None => break,
            Some(c) if c.is_ident("and") => {
                if op == Some("or") {
                    return Tri::False;
                }
                op = Some("and");
            }
            Some(c) if c.is_ident("or") => {
                if op == Some("and") {
                    return Tri::False;
                }
                op = Some("or");
            }
            _ => return Tri::False,
        }
        i += 1;
    }
    result.unwrap_or(Tri::False)
}

trait NotTri {
    fn not_tri(self) -> Tri;
}
impl NotTri for Tri {
    fn not_tri(self) -> Tri {
        match self {
            Tri::True => Tri::False,
            Tri::False => Tri::True,
            Tri::Unknown => Tri::Unknown,
        }
    }
}

fn and_tri(a: Tri, b: Tri) -> Tri {
    match (a, b) {
        (Tri::False, _) | (_, Tri::False) => Tri::False,
        (Tri::True, Tri::True) => Tri::True,
        _ => Tri::Unknown,
    }
}

fn or_tri(a: Tri, b: Tri) -> Tri {
    match (a, b) {
        (Tri::True, _) | (_, Tri::True) => Tri::True,
        (Tri::False, Tri::False) => Tri::False,
        _ => Tri::Unknown,
    }
}

fn eval_in_parens(cv: &Cv, ctx: &mut SheetCtx<'_>) -> Tri {
    match cv {
        Cv::Block(BlockKind::Paren, inner) => {
            let non_ws: Vec<&Cv> = inner.iter().filter(|c| !c.is_ws()).collect();
            // `( <supports-condition> )`
            if non_ws.first().is_some_and(|c| c.is_ident("not") || matches!(c, Cv::Block(BlockKind::Paren, _)) || matches!(c, Cv::Func(..))) {
                return eval_supports_inner(inner, ctx);
            }
            // `( <declaration> )`
            if supports_declaration(inner, ctx) { Tri::True } else { Tri::False }
        }
        Cv::Func(n, inner) if n.eq_ignore_ascii_case("selector") => {
            let mut pc = ParseCtx { atoms: ctx.atoms, quirks: ctx.quirks, depth: 0 };
            let single = !inner.iter().any(Cv::is_comma);
            match parse_selector_list(inner, &mut pc, false, true) {
                Some(l) if single && l.len() == 1 => Tri::True,
                _ => Tri::False,
            }
        }
        // font-tech(), font-format(), and unknown functions are "general enclosed": unknown.
        Cv::Func(..) => Tri::Unknown,
        _ => Tri::False,
    }
}

#[allow(dead_code)]
fn _keep(_: PropId) {}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> (Stylesheet, Interner) {
        let mut atoms = Interner::new();
        let (mut v, mut b) = (0, 0);
        let mut ctx = SheetCtx { atoms: &mut atoms, quirks: false, var_id: &mut v, block_id: &mut b };
        let s = parse_stylesheet(text, &mut ctx);
        (s, atoms)
    }

    #[test]
    fn rules_and_atrules() {
        let (s, _) = parse(
            "@charset 'utf-8'; @import url(a.css) screen; a, b > c { color: red; margin: 1px }\n\
             @media (min-width: 600px) { p { x: y; width: 1px } } @font-face { font-family: x }\n\
             @layer base, theme; @layer base { div { color: blue } }\n\
             @supports (display: grid) { i { color: green } } @supports (bogus: 1) { u { color: green } }",
        );
        let kinds: Vec<&str> = s
            .rules
            .iter()
            .map(|r| match r {
                Rule::Style(_) => "style",
                Rule::Media(..) => "media",
                Rule::Layer(..) => "layer",
                Rule::LayerStatement(_) => "layer-stmt",
                Rule::Import(_) => "import",
            })
            .collect();
        assert_eq!(kinds, ["import", "style", "media", "layer-stmt", "layer", "style"]);
    }

    #[test]
    fn invalid_selector_drops_rule() {
        let (s, _) = parse("a:foo { color: red } b { color: red } c:has(d) { color: red } e::before { color: red }");
        assert_eq!(s.rules.len(), 2); // b and e::before
    }

    #[test]
    fn nesting_caps() {
        // n nested levels: `open` repeated n times, the core, then one `)` per `(` in `open`.
        let nest = |open: &str, core: &str, n: usize| format!("{}{core}{}", open.repeat(n), ")".repeat(open.matches('(').count() * n));
        // Selectors: 32 nested :not() levels are fine, 33 make the selector (and the rule) invalid.
        for (n, rules) in [(32, 1), (33, 0)] {
            let (s, _) = parse(&format!("p{} {{ color: red }}", nest(":not(", "q", n)));
            assert_eq!(s.rules.len(), rules, "selector depth {n}");
        }
        // @supports: 32 nested parentheses are fine, 33 are false.
        for (n, rules) in [(32, 1), (33, 0)] {
            let (s, _) = parse(&format!("@supports {} {{ p {{ color: red }} }}", nest("(", "display: block", n)));
            assert_eq!(s.rules.len(), rules, "supports depth {n}");
        }
        // Compounds in a nested list: 16 are fine, 17 are invalid.
        for (n, rules) in [(16, 1), (17, 0)] {
            let (s, _) = parse(&format!("p:not({}) {{ color: red }}", vec!["q"; n].join(" ")));
            assert_eq!(s.rules.len(), rules, "nested compounds {n}");
        }
    }

    #[test]
    fn late_import_is_ignored() {
        let (s, _) = parse("a { color: red } @import 'x.css';");
        assert_eq!(s.rules.len(), 1);
    }

    #[test]
    fn declaration_recovery() {
        let (s, _) = parse("a { color: red; width: -1px; color: ; margin: 1px 2px }");
        let Some(Rule::Style(r)) = s.rules.first() else { panic!("no rule") };
        // color:red + margin (4 longhands); the invalid ones are gone
        assert_eq!(r.block.decls.len(), 5);
    }
}
