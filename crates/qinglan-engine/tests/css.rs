//! CSS behaviour tests: cascade order, `var()`, `calc()`, `@media`, `@supports`, colors, error recovery,
//! selectors and presentational hints.
#![allow(clippy::indexing_slicing, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use qinglan_engine::css::{NoResources, StyleOptions, style_document};
use qinglan_engine::html::dom::{Document, NodeId};
use qinglan_engine::html::treebuilder::parse_document;

struct Page {
    doc: Document,
    tree: qinglan_engine::css::cascade::StyleTree,
}

fn page_w(html: &str, width: f32) -> Page {
    let mut doc = parse_document(html.to_string());
    let out = style_document(&mut doc, None, &mut NoResources, &StyleOptions { viewport_w: width, viewport_h: 800.0 });
    Page { doc, tree: out.tree }
}

fn page(html: &str) -> Page {
    page_w(html, 1280.0)
}

impl Page {
    fn by_id(&self, id: &str) -> NodeId {
        (0..self.doc.nodes.len() as NodeId)
            .find(|&n| self.doc.element(n).is_some() && self.doc.attr(n, qinglan_engine::html::atoms::a::ID) == Some(id))
            .unwrap_or_else(|| panic!("no element #{id}"))
    }
    fn get(&self, id: &str, prop: &str) -> String {
        let n = self.by_id(id);
        let style = self.tree.get(n).expect("style");
        style
            .properties()
            .into_iter()
            .find(|(name, _)| *name == prop)
            .unwrap_or_else(|| panic!("no property {prop}"))
            .1
    }
}

const DOC: &str = "<!DOCTYPE html><html><head><style>";

fn with_css(css: &str, body: &str) -> String {
    format!("{DOC}{css}</style></head><body>{body}</body></html>")
}

#[test]
fn origin_and_importance() {
    // Author normal beats UA normal.
    let p = page(&with_css("p { margin-top: 0; display: inline }", "<p id=p>x</p>"));
    assert_eq!(p.get("p", "margin-top"), "0px");
    assert_eq!(p.get("p", "display"), "inline");
    // UA important beats everything of the author.
    let p = page(&with_css("input { display: block !important }", "<input id=h type=hidden style='display:block !important'>"));
    assert_eq!(p.get("h", "display"), "none");
    // Author important beats inline normal; inline important beats author important.
    let p = page(&with_css("#a { color: red !important } #b { color: red !important }", "<p id=a style='color:blue'>x</p><p id=b style='color:blue !important'>y</p>"));
    assert_eq!(p.get("a", "color"), "#ff0000");
    assert_eq!(p.get("b", "color"), "#0000ff");
}

#[test]
fn specificity_and_order() {
    let css = "#a { color: red } .b { color: blue } p { color: green } .b { color: yellow }";
    let p = page(&with_css(css, "<p id=a class=b>x</p><p id=c class=b>y</p>"));
    assert_eq!(p.get("a", "color"), "#ff0000"); // id beats class
    assert_eq!(p.get("c", "color"), "#ffff00"); // later class rule wins
    // :where() has no specificity, :is() takes the most specific argument.
    let css = ".x { color: red } :where(#id) { color: blue } p:is(.x, #zz) { color: green }";
    let p = page(&with_css(css, "<p id=id class=x>x</p>"));
    assert_eq!(p.get("id", "color"), "#008000"); // p:is(.x,#zz) = (0,1,1) beats .x (0,1,0)
    let css = ".x { color: red } :where(#id) { color: blue }";
    let p = page(&with_css(css, "<p id=id class=x>x</p>"));
    assert_eq!(p.get("id", "color"), "#ff0000");
}

#[test]
fn layers() {
    // Unlayered normal rules beat layered ones, whatever the order.
    let css = "p { color: blue } @layer a { p { color: red } }";
    assert_eq!(page(&with_css(css, "<p id=p>x</p>")).get("p", "color"), "#0000ff");
    // Layer order from the statement; later layer wins for normal declarations.
    let css = "@layer b, a; @layer a { p { color: red } } @layer b { p { color: blue } }";
    assert_eq!(page(&with_css(css, "<p id=p>x</p>")).get("p", "color"), "#ff0000");
    // Important declarations: the earlier layer wins, and layered beats unlayered.
    let css = "@layer a, b; @layer a { p { color: red !important } } @layer b { p { color: blue !important } } p { color: green !important }";
    assert_eq!(page(&with_css(css, "<p id=p>x</p>")).get("p", "color"), "#ff0000");
    // Nested layers: a.c is below a's own rules, and a (declared first) is below b.
    let css = "@layer a { @layer c { p { color: red } } p { color: blue } } @layer b { p { color: green } }";
    assert_eq!(page(&with_css(css, "<p id=p>x</p>")).get("p", "color"), "#008000");
    let css = "@layer a { @layer c { p { color: red } } p { color: blue } }";
    assert_eq!(page(&with_css(css, "<p id=p>x</p>")).get("p", "color"), "#0000ff");
    // Specificity does not matter across layers.
    let css = "@layer a { #p { color: red } } @layer b { p { color: blue } }";
    assert_eq!(page(&with_css(css, "<p id=p>x</p>")).get("p", "color"), "#0000ff");
    // revert-layer.
    let css = "@layer a { p { color: red } } @layer b { p { color: revert-layer } }";
    assert_eq!(page(&with_css(css, "<p id=p>x</p>")).get("p", "color"), "#ff0000");
}

#[test]
fn css_wide_keywords() {
    let css = "div { color: red; margin-top: 7px } span { color: inherit; margin-top: inherit } .u { color: unset; margin-top: unset } .i { color: initial } p { color: revert }";
    let p = page(&with_css(css, "<div id=d><span id=s>a</span><b class=u id=u>b</b><b class=i id=i>c</b></div><p id=p>x</p>"));
    assert_eq!(p.get("s", "color"), "#ff0000");
    assert_eq!(p.get("s", "margin-top"), "7px");
    assert_eq!(p.get("u", "color"), "#ff0000"); // unset on an inherited property inherits
    assert_eq!(p.get("u", "margin-top"), "0px"); // unset on a reset property is initial
    assert_eq!(p.get("i", "color"), "#000000");
    assert_eq!(p.get("p", "color"), "#000000"); // revert to the UA style (black)
}

#[test]
fn custom_properties() {
    let css = ":root { --c: #123456; --w: 10px; --a: var(--b); --b: var(--a); --d: var(--missing, 3px) } \
               p { color: var(--c); width: var(--w); height: var(--nope, 5px); margin-top: var(--a, 9px); padding-top: var(--d) }";
    let p = page(&with_css(css, "<p id=p>x</p>"));
    assert_eq!(p.get("p", "color"), "#123456");
    assert_eq!(p.get("p", "width"), "10px");
    assert_eq!(p.get("p", "height"), "5px"); // fallback
    assert_eq!(p.get("p", "margin-top"), "9px"); // --a is in a cycle: invalid, so the fallback is used
    assert_eq!(p.get("p", "padding-top"), "3px"); // fallback inside a custom property
    // A property with an unresolvable var() and no fallback is unset (here: inherited / initial).
    let css = "div { color: red } p { color: var(--nope); margin-top: var(--nope) }";
    let p = page(&with_css(css, "<div><p id=p>x</p></div>"));
    assert_eq!(p.get("p", "color"), "#ff0000");
    assert_eq!(p.get("p", "margin-top"), "0px");
    // var() in a shorthand, inheritance of custom properties, override on a child.
    let css = "div { --m: 4px 8px } p { margin: var(--m) } span { --m: 1px }";
    let p = page(&with_css(css, "<div><p id=p>x</p><span id=s><p id=q>y</p></span></div>"));
    assert_eq!(p.get("p", "margin-top"), "4px");
    assert_eq!(p.get("p", "margin-right"), "8px");
    assert_eq!(p.get("q", "margin-left"), "1px");
    // A custom property whose value is too long becomes invalid.
    let big = "a".repeat(70_000);
    let css = format!("div {{ --big: {big} }} p {{ --x: var(--big); font-family: var(--x, serif); }}");
    let p = page(&with_css(&css, "<div><p id=p>x</p></div>"));
    assert_eq!(p.get("p", "font-family"), "serif");
    // Exponential expansion is cut off by the length limit, not by hanging.
    let mut css = String::from("div { --a0: xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx; ");
    for i in 1..30 {
        css.push_str(&format!("--a{i}: var(--a{}) var(--a{}); ", i - 1, i - 1));
    }
    css.push_str("} p { font-family: var(--a29, serif) }");
    let p = page(&with_css(&css, "<div><p id=p>x</p></div>"));
    assert_eq!(p.get("p", "font-family"), "serif");
}

#[test]
fn calc_and_units() {
    let css = "p { width: calc(100% - 10px); margin-top: calc(1em + 2px); height: min(10px, 5vw); padding-top: clamp(1px, 50%, 3px); padding-left: calc(2 * (3px + 1px)) }";
    let p = page(&with_css(css, "<p id=p>x</p>"));
    assert_eq!(p.get("p", "width"), "calc(100% + (-1 * 10px))");
    assert_eq!(p.get("p", "margin-top"), "18px");
    assert_eq!(p.get("p", "height"), "10px"); // 5vw = 64px; min = 10px
    assert_eq!(p.get("p", "padding-left"), "8px");
    // Invalid calc() is dropped; the earlier declaration stays.
    let p = page(&with_css("p { margin-top: 3px; margin-top: calc(1px+2px) }", "<p id=p>x</p>"));
    assert_eq!(p.get("p", "margin-top"), "3px");
    // Units.
    let css = "p { margin-top: 1in; margin-left: 12pt; margin-right: 1rem; margin-bottom: 10vmax; padding-top: 2ex }";
    let p = page(&with_css(css, "<p id=p>x</p>"));
    assert_eq!(p.get("p", "margin-top"), "96px");
    assert_eq!(p.get("p", "margin-left"), "16px");
    assert_eq!(p.get("p", "margin-right"), "16px");
    assert_eq!(p.get("p", "margin-bottom"), "128px");
    assert_eq!(p.get("p", "padding-top"), "16px"); // ex = 0.5em
}

#[test]
fn media_queries_at_three_widths() {
    let css = "p { color: black } \
               @media (max-width: 500px) { p { color: red } } \
               @media (min-width: 501px) and (max-width: 900px) { p { color: blue } } \
               @media (min-width: 1000px) { p { color: green } } \
               @media print { p { color: yellow } } \
               @media (width >= 1000px) and (hover: hover) and (pointer: fine) { p { margin-top: 1px } } \
               @media (prefers-color-scheme: dark) { p { margin-top: 2px } } \
               @media (prefers-reduced-motion: reduce) { p { margin-top: 3px } }";
    for (w, color, mt) in [(360.0, "#ff0000", "16px"), (768.0, "#0000ff", "16px"), (1280.0, "#008000", "1px")] {
        let p = page_w(&with_css(css, "<p id=p>x</p>"), w);
        assert_eq!(p.get("p", "color"), color, "width {w}");
        assert_eq!(p.get("p", "margin-top"), mt, "width {w}");
    }
    // Media attribute of <style> and <link>-like sources.
    let html = "<style media='(max-width: 500px)'>p{color:red}</style><style media='print'>p{color:blue}</style><p id=p>x</p>";
    assert_eq!(page_w(html, 360.0).get("p", "color"), "#ff0000");
    assert_eq!(page_w(html, 1280.0).get("p", "color"), "#000000");
}

#[test]
fn supports() {
    let css = "@supports (display: grid) { p { color: red } } \
               @supports (bogus-prop: 1) { p { color: blue } } \
               @supports not (bogus-prop: 1) { p { margin-top: 5px } } \
               @supports (display: grid) and (color: red) { p { margin-left: 5px } } \
               @supports (bogus: 1) or (color: red) { p { margin-right: 5px } } \
               @supports selector(a > b) { p { margin-bottom: 5px } } \
               @supports selector(a:has(b)) { p { padding-top: 5px } } \
               @supports (--custom: x) { p { padding-left: 5px } }";
    let p = page(&with_css(css, "<p id=p>x</p>"));
    assert_eq!(p.get("p", "color"), "#ff0000");
    assert_eq!(p.get("p", "margin-top"), "5px");
    assert_eq!(p.get("p", "margin-left"), "5px");
    assert_eq!(p.get("p", "margin-right"), "5px");
    assert_eq!(p.get("p", "margin-bottom"), "5px");
    assert_eq!(p.get("p", "padding-top"), "0px"); // :has() is not supported
    assert_eq!(p.get("p", "padding-left"), "5px");
}

#[test]
fn colors() {
    let css = "#a { color: rgb(255 128 0 / 50%) } #b { color: hsl(120deg 100% 25%) } #c { color: hwb(0 0% 0%) } \
               #d { color: lab(100% 0 0) } #e { color: oklch(0.628 0.2577 29.23) } #f { color: #0f08 } \
               #g { color: RebeccaPurple } #h { color: transparent } #i { color: currentColor } #j { color: color-mix(in srgb, red, blue) } \
               #k { color: lch(54.29% 106.84 40.85) } #l { color: oklab(62.8% 0.2249 0.1258) }";
    let ids = "abcdefghijkl";
    let body: String = ids.chars().map(|c| format!("<p id={c}>x</p>")).collect();
    let p = page(&with_css(css, &body));
    assert_eq!(p.get("a", "color"), "rgba(255, 128, 0, 0.502)");
    assert_eq!(p.get("b", "color"), "#008000");
    assert_eq!(p.get("c", "color"), "#ff0000");
    assert_eq!(p.get("d", "color"), "#ffffff");
    assert_eq!(p.get("e", "color"), "#ff0000");
    assert_eq!(p.get("f", "color"), "rgba(0, 255, 0, 0.533)");
    assert_eq!(p.get("g", "color"), "#663399");
    assert_eq!(p.get("h", "color"), "rgba(0, 0, 0, 0.000)");
    assert_eq!(p.get("i", "color"), "#000000");
    assert_eq!(p.get("j", "color"), "#000000"); // invalid: color-mix() is not supported
    assert_eq!(p.get("k", "color"), "#ff0000");
    assert_eq!(p.get("l", "color"), "#ff0000");
}

#[test]
fn error_recovery() {
    // Bad declarations are dropped; earlier valid ones stay; later ones apply.
    let css = "p { color: red; color: bogus; margin-top: 1px 2px 3px 4px 5px; margin-left: 7px; ; width: ; height: 9px }";
    let p = page(&with_css(css, "<p id=p>x</p>"));
    assert_eq!(p.get("p", "color"), "#ff0000");
    assert_eq!(p.get("p", "margin-top"), "16px"); // the invalid declaration is dropped: the UA margin stays
    assert_eq!(p.get("p", "margin-left"), "7px");
    assert_eq!(p.get("p", "width"), "auto");
    assert_eq!(p.get("p", "height"), "9px");
    // An invalid selector drops the rule; the following rules still work.
    let css = "p:unknown-thing { color: red } p::-webkit-foo { color: red } p, q:has(a) { color: red } p { margin-top: 3px } b { color: blue";
    let p = page(&with_css(css, "<p id=p>x</p><b id=b>y</b>"));
    assert_eq!(p.get("p", "color"), "#000000");
    assert_eq!(p.get("p", "margin-top"), "3px");
    assert_eq!(p.get("b", "color"), "#0000ff"); // unterminated block closes at EOF
    // Unknown at-rules, bad strings and garbage between rules.
    // The stray `}` becomes part of the next rule's prelude, which makes that rule's selector invalid.
    let css = "@font-face { font-family: x; src: url(a) } @keyframes k { from { color: red } } } p { color: green } u { color: green } @bogus x y; \
               b { content: 'unterminated\n; color: red } i { color: blue }";
    let p = page(&with_css(css, "<p id=p>x</p><u id=u>w</u><b id=b>y</b><i id=i>z</i>"));
    assert_eq!(p.get("p", "color"), "#000000");
    assert_eq!(p.get("u", "color"), "#008000");
    assert_eq!(p.get("i", "color"), "#0000ff");
    // !important with odd spacing and case.
    let p = page(&with_css("p { color : red ! IMPORTANT } p { color: blue }", "<p id=p>x</p>"));
    assert_eq!(p.get("p", "color"), "#ff0000");
}

#[test]
fn selectors() {
    let html = "<ul id=u><li id=l1 class='a b'>1</li><li id=l2 data-x='foo-bar'>2</li><li id=l3 lang='zh-CN'>3</li><li id=l4 data-x=BAR>4</li></ul><p id=p1></p><p id=p2> </p><input id=i1 type=checkbox checked><a id=a1 href=x>l</a><a id=a2>n</a>";
    let css = "li:first-child { margin-top: 1px } li:last-child { margin-top: 2px } li:nth-child(2) { margin-top: 3px } \
               li:nth-last-child(-n+2) { margin-left: 4px } li:not(.a):not([data-x]) { margin-right: 5px } \
               [data-x|=foo] { padding-top: 6px } [data-x=bar i] { padding-top: 7px } li[class~=b] { padding-left: 8px } \
               li:lang(zh) { padding-right: 9px } p:empty { margin-bottom: 10px } input:checked { margin-top: 11px } \
               a:link { margin-top: 12px } a:not(:link) { margin-top: 13px } ul > li + li { margin-bottom: 14px } \
               ul li ~ li:last-of-type { padding-bottom: 15px } :root { margin-top: 16px }";
    let p = page(&with_css(css, html));
    assert_eq!(p.get("l1", "margin-top"), "1px");
    assert_eq!(p.get("l4", "margin-top"), "2px");
    assert_eq!(p.get("l2", "margin-top"), "3px");
    assert_eq!(p.get("l3", "margin-left"), "4px");
    assert_eq!(p.get("l4", "margin-left"), "4px");
    assert_eq!(p.get("l1", "margin-left"), "0px");
    assert_eq!(p.get("l3", "margin-right"), "5px");
    assert_eq!(p.get("l1", "margin-right"), "0px");
    assert_eq!(p.get("l2", "padding-top"), "6px");
    assert_eq!(p.get("l4", "padding-top"), "7px");
    assert_eq!(p.get("l1", "padding-left"), "8px");
    assert_eq!(p.get("l3", "padding-right"), "9px");
    assert_eq!(p.get("p1", "margin-bottom"), "10px");
    assert_eq!(p.get("p2", "margin-bottom"), "16px"); // a space is not empty: only the UA margin applies
    assert_eq!(p.get("i1", "margin-top"), "11px");
    assert_eq!(p.get("a1", "margin-top"), "12px");
    assert_eq!(p.get("a2", "margin-top"), "13px");
    assert_eq!(p.get("l2", "margin-bottom"), "14px");
    assert_eq!(p.get("l1", "margin-bottom"), "0px");
    assert_eq!(p.get("l4", "padding-bottom"), "15px");
}

#[test]
fn inheritance_and_lang() {
    let html = "<div id=d lang='zh-Hans' style='font-size: 20px; line-height: 1.5; letter-spacing: .1em'><p id=p>x</p><p id=q lang=ja>y</p></div>";
    let p = page(&with_css("", html));
    assert_eq!(p.get("p", "font-size"), "20px");
    assert_eq!(p.get("p", "line-height"), "1.5");
    assert_eq!(p.get("p", "letter-spacing"), "2px");
    let st = p.tree.get(p.by_id("p")).unwrap();
    assert_eq!(st.text.lang.as_deref(), Some("zh-Hans"));
    let st = p.tree.get(p.by_id("q")).unwrap();
    assert_eq!(st.text.lang.as_deref(), Some("ja"));
    // Percentage and em line-heights are inherited as lengths; numbers as numbers.
    let p = page(&with_css("div { font-size: 10px; line-height: 150% } p { font-size: 40px }", "<div><p id=p>x</p></div>"));
    assert_eq!(p.get("p", "line-height"), "15px");
}

#[test]
fn presentational_hints_are_weak() {
    let html = "<body bgcolor='#ff0000' text='#00ff00'><table id=t border=2 cellpadding=5 width=50% bgcolor=blue><tr><td id=td align=center>x</table>\
                <p id=p align=right>y</p><font id=f size=+1 face='Arial, sans-serif' color=#abc>z</font><hr id=h size=4 noshade>";
    let p = page(&with_css("body { background-color: white }", html));
    // Author CSS beats the attribute.
    assert_eq!(p.get("t", "border-top-width"), "2px");
    assert_eq!(p.get("t", "border-top-style"), "outset");
    assert_eq!(p.get("t", "width"), "50%");
    assert_eq!(p.get("t", "background-color"), "#0000ff");
    assert_eq!(p.get("td", "padding-top"), "5px");
    assert_eq!(p.get("td", "text-align"), "center");
    assert_eq!(p.get("td", "border-top-style"), "inset");
    assert_eq!(p.get("p", "text-align"), "right");
    assert_eq!(p.get("f", "font-size"), "18px");
    assert_eq!(p.get("f", "color"), "#aabbcc");
    assert_eq!(p.get("f", "font-family"), "\"Arial\", sans-serif");
    assert_eq!(p.get("h", "border-top-width"), "2px");
    // border="0" gives no cell borders.
    let p = page(&with_css("", "<table border=0><tr><td id=a>x</table><table border><tr><td id=b>y</table>"));
    assert_eq!(p.get("a", "border-top-style"), "none");
    assert_eq!(p.get("b", "border-top-style"), "inset");
}

#[test]
fn quirks_mode_unitless_lengths_and_inheritance() {
    // No doctype: quirks mode: unitless lengths are px, tables do not inherit the font size.
    let html = "<html><head><style>p { width: 100; margin-top: 5 } div { font-size: 30px } </style></head><body><div><table id=t><tr><td>x</td></tr></table><p id=p>y</p></div></body></html>";
    let p = page(html);
    assert_eq!(p.get("p", "width"), "100px");
    assert_eq!(p.get("p", "margin-top"), "5px");
    assert_eq!(p.get("t", "font-size"), "16px");
    // Standards mode: no such quirks.
    let p = page(&html.replacen("<html>", "<!DOCTYPE html><html>", 1));
    assert_eq!(p.get("p", "width"), "auto");
    assert_eq!(p.get("t", "font-size"), "30px");
}

#[test]
fn font_shorthand_and_monospace_size() {
    let css = "p { font: italic bold 14px/20px 'Helvetica Neue', Arial, sans-serif } pre { }";
    let p = page(&with_css(css, "<p id=p>x</p><pre id=r>y</pre><div style='font-family: monospace'><span id=m>z</span></div>"));
    assert_eq!(p.get("p", "font-style"), "italic");
    assert_eq!(p.get("p", "font-weight"), "700");
    assert_eq!(p.get("p", "font-size"), "14px");
    assert_eq!(p.get("p", "line-height"), "20px");
    assert_eq!(p.get("p", "font-family"), "\"Helvetica Neue\", \"Arial\", sans-serif");
    // The default size of a monospace-only family is 13px.
    assert_eq!(p.get("r", "font-size"), "13px");
    assert_eq!(p.get("m", "font-size"), "13px");
}
