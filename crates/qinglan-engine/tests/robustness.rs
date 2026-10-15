//! Hostile input: every page must produce a result (or a clean limit) and must not panic or hang.
//! Each case has a 5 second budget (the budget of the L1 task sheet for malicious pages).
#![allow(clippy::indexing_slicing, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::cell::RefCell;
use std::time::{Duration, Instant};

use qinglan_engine::css::{ResourceLoader, StyleOptions, style_document};
use qinglan_engine::html::dom::{Document, MAX_DEPTH, MAX_NODES, NONE, NodeId};
use qinglan_engine::html::treebuilder::parse_document;
use qinglan_engine::url::Url;

// Time budgets are only asserted in release builds; debug speed varies too much
// with machine load to be a meaningful check.
const BUDGET: Duration = Duration::from_secs(5);

struct MapLoader {
    sheets: Vec<(String, String)>,
    loads: RefCell<usize>,
}

impl ResourceLoader for MapLoader {
    fn load_stylesheet(&mut self, url: &Url) -> Option<String> {
        *self.loads.borrow_mut() += 1;
        let path = url.pathname();
        self.sheets.iter().find(|(p, _)| *p == path).map(|(_, t)| t.clone())
    }
}

fn depth_of(doc: &Document) -> usize {
    let mut max = 0usize;
    let mut stack: Vec<(NodeId, usize)> = vec![(0, 0)];
    while let Some((n, d)) = stack.pop() {
        max = max.max(d);
        let mut c = doc.first_child(n);
        while c != NONE {
            stack.push((c, d + 1));
            c = doc.next_sibling(c);
        }
    }
    max
}

fn style(doc: &mut Document) -> usize {
    let out = style_document(doc, None, &mut qinglan_engine::css::NoResources, &StyleOptions::default());
    out.tree.styles.iter().filter(|s| s.is_some()).count()
}

#[test]
fn deep_nesting_is_flattened() {
    let t = Instant::now();
    for tag in ["div", "span", "b", "table", "ul"] {
        let html = format!("{}x", format!("<{tag}>").repeat(100_000));
        let mut doc = parse_document(html);
        assert!(depth_of(&doc) <= MAX_DEPTH + 8, "{tag}: depth {}", depth_of(&doc));
        style(&mut doc);
    }
    assert!(cfg!(debug_assertions) || t.elapsed() < BUDGET * 3, "took {:?}", t.elapsed());
}

#[test]
fn mismatched_formatting_does_not_blow_up() {
    let t = Instant::now();
    let html = format!("{}x{}", "<b><i><div>".repeat(20_000), "</b></i></div>".repeat(20_000));
    let mut doc = parse_document(html);
    style(&mut doc);
    assert!(cfg!(debug_assertions) || t.elapsed() < BUDGET, "took {:?}", t.elapsed());
}

#[test]
fn node_limit() {
    let t = Instant::now();
    let html = "<p>x".repeat(MAX_NODES);
    let doc = parse_document(html);
    assert!(doc.too_large);
    assert!(doc.nodes.len() < MAX_NODES + 5000);
    assert!(cfg!(debug_assertions) || t.elapsed() < BUDGET, "took {:?}", t.elapsed());
}

#[test]
fn many_attributes_and_huge_text() {
    let t = Instant::now();
    let mut html = String::from("<div");
    for i in 0..100_000 {
        html.push_str(&format!(" a{i}=1"));
    }
    html.push('>');
    html.push_str(&"a".repeat(10_000_000));
    let mut doc = parse_document(html);
    style(&mut doc);
    assert!(cfg!(debug_assertions) || t.elapsed() < BUDGET, "took {:?}", t.elapsed());
}

#[test]
fn nul_and_bad_bytes() {
    let mut bytes = b"<p>a\0b</p><svg>\0<![CDATA[\0]]></svg><style>p{color:\0red}</style>\xff\xfe<p>".to_vec();
    bytes.extend_from_slice(b"\xC4\xE3\xBA\xC3");
    let d = qinglan_engine::encoding::decode_document(&bytes, None, &qinglan_engine::platform::BasicPlatform);
    let mut doc = parse_document(d.text);
    style(&mut doc);
}

#[test]
fn huge_stylesheets() {
    let t = Instant::now();
    // 100k lines of rules.
    let mut css = String::new();
    for i in 0..100_000 {
        css.push_str(&format!(".c{i} > a:not(.x):nth-child(2n+1) {{ margin: {i}px }}\n"));
    }
    // A very long selector and a very deep calc().
    css.push_str(&format!("{} {{ color: red }}\n", vec!["a"; 100_000].join(" ")));
    css.push_str(&format!("{} {{ color: red }}\n", vec!["a"; 100_000].join(" > ")));
    css.push_str(&format!("p {{ width: {}1px{} }}\n", "calc(".repeat(10_000), ")".repeat(10_000)));
    css.push_str("p { font-size: 1e9px; margin-top: 1e30px; margin-bottom: -1e30px; width: calc(1e308px * 10) ; line-height: 1e9 }\n");
    let html = format!("<style>{css}</style><p class=c5>x</p><a>y</a>");
    let mut doc = parse_document(html);
    let n = style(&mut doc);
    assert!(n >= 3);
    assert!(cfg!(debug_assertions) || t.elapsed() < BUDGET * 2, "took {:?}", t.elapsed());
}

#[test]
fn import_limits() {
    let t = Instant::now();
    // A loop (a imports b imports a), a chain deeper than the limit and a thousand imports.
    let many: String = (0..1000).map(|i| format!("@import 'n{i}.css';")).collect();
    let loader_sheets = vec![
        ("/a.css".to_string(), "@import 'b.css'; p { color: red }".to_string()),
        ("/b.css".to_string(), "@import 'a.css'; p { color: blue }".to_string()),
        ("/many.css".to_string(), many),
        ("/d0.css".to_string(), "@import 'd1.css';".to_string()),
        ("/d1.css".to_string(), "@import 'd2.css';".to_string()),
        ("/d2.css".to_string(), "@import 'd3.css';".to_string()),
        ("/d3.css".to_string(), "@import 'd4.css';".to_string()),
        ("/d4.css".to_string(), "@import 'd5.css'; p { margin-top: 99px }".to_string()),
        ("/d5.css".to_string(), "p { margin-top: 77px }".to_string()),
    ];
    let mut loader = MapLoader { sheets: loader_sheets, loads: RefCell::new(0) };
    let mut doc = parse_document(
        "<link rel=stylesheet href=a.css><link rel=stylesheet href=many.css><link rel=stylesheet href=d0.css><p id=p>x</p>".to_string(),
    );
    let base = Url::parse("http://example.com/", None).unwrap();
    let out = style_document(&mut doc, Some(&base), &mut loader, &StyleOptions::default());
    // 64 sheets at most (L1 task sheet), and the import chain stops at depth 4.
    assert!(out.sheets <= 64, "{} sheets", out.sheets);
    assert!(*loader.loads.borrow() <= 80 + 1000, "{} loads", loader.loads.borrow());
    assert!(cfg!(debug_assertions) || t.elapsed() < BUDGET);
}

// -------------------------------------------------------------------------------------------------
// Stack depth. These tests run on a 1 MB stack, the stack of the main thread of the CLI on Windows, so
// what passes here also passes in the debug build of `qinglan-cli` and on the 2 MB test-thread stack.

fn on_small_stack(f: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new().stack_size(1 << 20).spawn(f).unwrap().join().unwrap();
}

/// `open` repeated n times, the core, then one ")" per "(" in `open`.
fn nest(open: &str, core: &str, n: usize) -> String {
    format!("{}{core}{}", open.repeat(n), ")".repeat(open.matches('(').count() * n))
}

fn styled_page(css: &str, inline: &str) {
    let html = format!("<style>{css}</style><div><p style=\"{inline}\">x</p><span>y</span></div>");
    let mut doc = parse_document(html);
    style(&mut doc);
}

#[test]
fn hostile_selector_nesting_10000() {
    on_small_stack(|| {
        let t = Instant::now();
        let n = 10_000;
        let mut sheets: Vec<String> = Vec::new();
        for open in [":is(", ":where(", ":not(", ":matches(", ":-webkit-any(", ":nth-child(1 of ", ":nth-last-child(2n+1 of ", ":is(:not(", ":not(:where("] {
            sheets.push(format!("p{} {{ color: red }}", nest(open, "p", n)));
            sheets.push(format!("p{} {{ color: red }}", open.repeat(n)));
        }
        for comb in [" ", " > ", " + ", " ~ "] {
            let chain = format!("p{comb}").repeat(n);
            sheets.push(format!("{chain}p {{ color: red }}"));
            sheets.push(format!("p:is({chain}p) {{ color: red }}"));
            sheets.push(format!("p:not({chain}p, a) {{ color: red }}"));
            sheets.push(format!("p:nth-child(1 of {chain}p) {{ color: red }}"));
        }
        let list = "p,".repeat(n);
        sheets.push(format!("{list}p {{ color: red }}"));
        sheets.push(format!("p:is({list}p) {{ color: red }}"));
        for css in &sheets {
            styled_page(css, "color: blue");
        }
        assert!(cfg!(debug_assertions) || t.elapsed() < BUDGET * 2, "took {:?}", t.elapsed());
    });
}

#[test]
fn hostile_condition_nesting_10000() {
    on_small_stack(|| {
        let t = Instant::now();
        let n = 10_000;
        let mut sheets: Vec<String> = Vec::new();
        for head in ["@media ", "@media screen and ", "@media not ", "@supports ", "@supports not "] {
            for open in ["(", "not (", "(not (", "(("] {
                sheets.push(format!("{head}{} {{ p {{ color: red }} }}", nest(open, "min-width: 1px", n)));
                sheets.push(format!("{head}{} {{ p {{ color: red }} }}", open.repeat(n)));
            }
            sheets.push(format!("{head}{} {{ p {{ color: red }} }}", nest("selector(", "p", n)));
            sheets.push(format!("{head}{} {{ p {{ color: red }} }}", nest("(selector(", ":is(p)", n)));
            sheets.push(format!("{head}selector({}) {{ p {{ color: red }} }}", nest(":is(", "p", n)));
        }
        sheets.push(format!("@import url(a.css) supports({});", nest("(", "display: block", n)));
        sheets.push(format!("@import url(a.css) supports({});", nest("not (", "display: block", n)));
        sheets.push(format!("@import url(a.css) {};", nest("(", "min-width: 1px", n)));
        for css in &sheets {
            styled_page(css, "color: blue");
        }
        assert!(cfg!(debug_assertions) || t.elapsed() < BUDGET * 2, "took {:?}", t.elapsed());
    });
}

#[test]
fn hostile_block_nesting_10000() {
    on_small_stack(|| {
        let t = Instant::now();
        let n = 10_000;
        for head in ["@media screen {", "@layer a {", "@supports (display: block) {", "@media screen { @layer a {", "{", "p {", "[", "foo(", "(", "@media ", ";{", "@unknown {"] {
            styled_page(&format!("{}p {{ color: red }}", head.repeat(n)), "color: blue");
            styled_page(&format!("{}p {{ color: red }}{}", head.repeat(n), "}".repeat(n)), "color: blue");
        }
        assert!(cfg!(debug_assertions) || t.elapsed() < BUDGET * 2, "took {:?}", t.elapsed());
    });
}

#[test]
fn hostile_value_nesting_10000() {
    on_small_stack(|| {
        let t = Instant::now();
        let n = 10_000;
        let values: Vec<String> = vec![
            nest("calc(", "1px", n),
            format!("calc({})", nest("(", "1px", n)),
            format!("calc({}1px)", "1 * ".repeat(n)),
            format!("calc({}1px)", "2 / ".repeat(n)),
            format!("calc({}1px)", "1px + ".repeat(n)),
            format!("calc({}1px)", "1px - ".repeat(n)),
            format!("calc({}", "(".repeat(n)),
            nest("min(1px, ", "2px", n),
            nest("max(", "2px", n),
            nest("clamp(1px, ", "2px", n),
            nest("calc(1px + ", "2px", n),
            nest("calc(1 * ", "2px", n),
            nest("var(--a, ", "1px", n),
            nest("var(--a, var(--b, ", "1px", n),
            nest("var(", "--a", n),
            nest("env(a, ", "1px", n),
            nest("rgb(", "1", n),
            nest("hsl(", "1", n),
            nest("color-mix(in srgb, ", "red", n),
            nest("foo(", "1", n),
            nest("url(", "a", n),
            "(".repeat(n),
            "[".repeat(n),
            "{".repeat(n),
            format!("{}{}", "(".repeat(n), ")".repeat(n)),
        ];
        for prop in ["width", "color", "--x"] {
            for v in &values {
                styled_page(&format!("p {{ {prop}: {v}; --y: {v} }} span {{ width: var(--x) }}"), "color: blue");
            }
        }
        for v in &values {
            styled_page("", &format!("width: {v}").replace('"', ""));
        }
        // Custom properties that refer to each other in a long chain, and in a cycle.
        let chain: String = (0..n).map(|i| format!("--p{i}: var(--p{}); ", i + 1)).collect();
        styled_page(&format!("p {{ {chain} width: var(--p0) }}"), "color: blue");
        styled_page(&format!("p {{ {chain} --p{n}: var(--p0); width: var(--p0) }}"), "color: blue");
        assert!(cfg!(debug_assertions) || t.elapsed() < BUDGET * 2, "took {:?}", t.elapsed());
    });
}

#[test]
fn deep_selector_matching_fits_the_stack() {
    on_small_stack(|| {
        let t = Instant::now();
        // 32 nested :is() levels; every level is a chain of 16 compounds whose leftmost compound is the next
        // level, and 490 nested divs let the match go all the way down.
        let mut sel = String::from("div");
        for _ in 0..32 {
            sel = format!(":is({sel}{})", " div".repeat(15));
        }
        let html = format!("<style>{sel} div div div p {{ color: red }}</style>{}<p>x</p>", "<div>".repeat(490));
        let mut doc = parse_document(html);
        style(&mut doc);
        assert!(cfg!(debug_assertions) || t.elapsed() < BUDGET, "took {:?}", t.elapsed());
    });
}

#[test]
fn nested_templates_do_not_overflow() {
    on_small_stack(|| {
        for unit in ["<template>", "<template><div>", "<template><table><tr><td>"] {
            for n in [600, 5000] {
                let mut doc = parse_document(unit.repeat(n));
                let dump = qinglan_engine::html::dump::dump_children(&doc, 0);
                assert!(dump.contains("content"));
                style(&mut doc);
            }
        }
    });
}
