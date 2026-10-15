//! Hostile input: every page must produce a result (or a clean limit) and must not panic or hang.
//! Each case has a 5 second budget (the budget of the L1 task sheet for malicious pages).
#![allow(clippy::indexing_slicing, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::cell::RefCell;
use std::time::{Duration, Instant};

use qinglan_engine::css::{ResourceLoader, StyleOptions, style_document};
use qinglan_engine::html::dom::{Document, MAX_DEPTH, MAX_NODES, NONE, NodeId};
use qinglan_engine::html::treebuilder::parse_document;
use qinglan_engine::url::Url;

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
    assert!(t.elapsed() < BUDGET * 3, "took {:?}", t.elapsed());
}

#[test]
fn mismatched_formatting_does_not_blow_up() {
    let t = Instant::now();
    let html = format!("{}x{}", "<b><i><div>".repeat(20_000), "</b></i></div>".repeat(20_000));
    let mut doc = parse_document(html);
    style(&mut doc);
    assert!(t.elapsed() < BUDGET, "took {:?}", t.elapsed());
}

#[test]
fn node_limit() {
    let t = Instant::now();
    let html = "<p>x".repeat(MAX_NODES);
    let doc = parse_document(html);
    assert!(doc.too_large);
    assert!(doc.nodes.len() < MAX_NODES + 5000);
    assert!(t.elapsed() < BUDGET, "took {:?}", t.elapsed());
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
    assert!(t.elapsed() < BUDGET, "took {:?}", t.elapsed());
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
    assert!(t.elapsed() < BUDGET * 2, "took {:?}", t.elapsed());
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
    assert!(t.elapsed() < BUDGET);
}
