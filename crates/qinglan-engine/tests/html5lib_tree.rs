//! html5lib-tests tree-construction conformance (tests/html5lib/tree-construction/*.dat, MIT).
//! Scripting is off: tests marked `#script-on` are skipped. Parse errors are not compared.
#![allow(clippy::indexing_slicing, clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::type_complexity)]

use qinglan_engine::html::dom::Namespace;
use qinglan_engine::html::dump::dump_children;
use qinglan_engine::html::treebuilder::{parse_document, parse_fragment};

struct Case {
    file: String,
    index: usize,
    data: String,
    fragment: Option<String>,
    script_on: bool,
    expected: String,
}

fn parse_dat(file: &str, text: &str) -> Vec<Case> {
    let mut cases = Vec::new();
    let mut cur: Option<(Vec<String>, Vec<String>, Vec<String>, Option<String>, bool)> = None; // data, errors, doc, frag, script_on
    let mut section = "";
    let lines: Vec<&str> = text.split('\n').collect();
    let flush = |cur: &mut Option<(Vec<String>, Vec<String>, Vec<String>, Option<String>, bool)>, cases: &mut Vec<Case>| {
        if let Some((data, _e, mut doc, frag, script_on)) = cur.take() {
            while doc.last().is_some_and(|l| l.is_empty()) {
                doc.pop();
            }
            let index = cases.len() + 1;
            cases.push(Case {
                file: file.to_string(),
                index,
                data: data.join("\n"),
                fragment: frag,
                script_on,
                expected: doc.join("\n"),
            });
        }
    };
    for (i, line) in lines.iter().enumerate() {
        let prev_blank = i == 0 || lines[i - 1].is_empty();
        if *line == "#data" && (cur.is_none() || (section == "document" && prev_blank)) {
            flush(&mut cur, &mut cases);
            cur = Some((Vec::new(), Vec::new(), Vec::new(), None, false));
            section = "data";
            continue;
        }
        let Some(c) = cur.as_mut() else { continue };
        if section != "document" || line.starts_with('#') && matches!(*line, "#errors" | "#new-errors" | "#document") {
            match *line {
                "#errors" => {
                    section = "errors";
                    continue;
                }
                "#new-errors" => {
                    section = "new-errors";
                    continue;
                }
                "#document-fragment" => {
                    section = "fragment";
                    continue;
                }
                "#script-on" => {
                    c.4 = true;
                    section = "flag";
                    continue;
                }
                "#script-off" => {
                    section = "flag";
                    continue;
                }
                "#document" => {
                    section = "document";
                    continue;
                }
                _ => {}
            }
        }
        match section {
            "data" => c.0.push(line.to_string()),
            "errors" | "new-errors" => c.1.push(line.to_string()),
            "fragment" => c.3 = Some(line.to_string()),
            "document" => c.2.push(line.to_string()),
            _ => {}
        }
    }
    flush(&mut cur, &mut cases);
    cases
}

fn run_case(c: &Case) -> String {
    match &c.fragment {
        None => {
            let doc = parse_document(c.data.clone());
            dump_children(&doc, 0)
        }
        Some(ctx) => {
            let (ns, name) = match ctx.split_once(' ') {
                Some(("svg", n)) => (Namespace::Svg, n),
                Some(("math", n)) => (Namespace::MathMl, n),
                _ => (Namespace::Html, ctx.as_str()),
            };
            let (doc, root) = parse_fragment(c.data.clone(), name, ns);
            dump_children(&doc, root)
        }
    }
}

/// Known failures, explained in docs/decisions.md ("html5lib 建树测试没过的用例"):
/// processing instructions (the vendored tests predate the spec change) and `<selectedcontent>` (DOM step, not parsing).
const KNOWN_FAILURES: &[(&str, usize)] = &[
    ("html5test-com.dat", 12),
    ("tests1.dat", 40),
    ("tests1.dat", 44),
    ("tests1.dat", 47),
    ("webkit02.dat", 45),
    ("webkit02.dat", 46),
    ("webkit02.dat", 47),
    ("webkit02.dat", 48),
];

#[test]
fn html5lib_tree_construction() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/html5lib/tree-construction");
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "dat"))
        .collect();
    files.sort();
    let (mut total, mut passed, mut skipped) = (0usize, 0usize, 0usize);
    let mut failures: Vec<String> = Vec::new();
    let mut unexpected: Vec<String> = Vec::new();
    for f in files {
        let name = f.file_name().unwrap().to_string_lossy().to_string();
        let text = std::fs::read_to_string(&f).unwrap();
        for c in parse_dat(&name, &text) {
            if c.script_on {
                skipped += 1;
                continue;
            }
            total += 1;
            let got = run_case(&c);
            let got = got.trim_end_matches('\n').to_string();
            if got == c.expected {
                passed += 1;
            } else {
                if !KNOWN_FAILURES.contains(&(c.file.as_str(), c.index)) {
                    unexpected.push(format!("{}#{}", c.file, c.index));
                }
                failures.push(format!(
                    "{}#{} data={:?} fragment={:?}\n--- expected\n{}\n--- got\n{}\n",
                    c.file, c.index, c.data, c.fragment, c.expected, got
                ));
            }
        }
    }
    println!(
        "html5lib tree construction: {passed}/{total} passed ({:.2}%), {skipped} skipped (script-on), {} failed",
        100.0 * passed as f64 / total as f64,
        failures.len()
    );
    if let Ok(path) = std::env::var("TREE_FAILURES") {
        let _ = std::fs::write(path, failures.join("\n=========\n"));
    }
    assert!(unexpected.is_empty(), "new failures: {unexpected:?}");
    assert!(passed * 100 >= total * 98, "{} failures", failures.len());
}
