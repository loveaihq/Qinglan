//! html5lib-tests tokenizer conformance (tests/html5lib/tokenizer/*.test, MIT).
//! Parse errors are not reported by the engine, so only the token stream is compared.
#![allow(clippy::indexing_slicing, clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod common;

use common::{Json, parse_json};
use qinglan_engine::html::atoms::Interner;
use qinglan_engine::html::tokenizer::{State, Token, Tokenizer};

/// Undo the "double escaping" of the test files. `None` if the text would need a lone surrogate.
fn unescape(s: &str) -> Option<String> {
    let mut out = String::new();
    let cs: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < cs.len() {
        if cs[i] == '\\' && cs.get(i + 1) == Some(&'u') && i + 6 <= cs.len() {
            let hex: String = cs[i + 2..i + 6].iter().collect();
            if let Ok(v) = u32::from_str_radix(&hex, 16) {
                i += 6;
                if (0xD800..0xDC00).contains(&v) {
                    // need a following low surrogate
                    if cs.get(i) == Some(&'\\') && cs.get(i + 1) == Some(&'u') && i + 6 <= cs.len() {
                        let hex2: String = cs[i + 2..i + 6].iter().collect();
                        if let Ok(lo) = u32::from_str_radix(&hex2, 16)
                            && (0xDC00..0xE000).contains(&lo)
                        {
                            out.push(char::from_u32(0x10000 + ((v - 0xD800) << 10) + (lo - 0xDC00))?);
                            i += 6;
                            continue;
                        }
                    }
                    return None;
                }
                out.push(char::from_u32(v)?); // None for a lone low surrogate
                continue;
            }
        }
        out.push(cs[i]);
        i += 1;
    }
    Some(out)
}

fn state_from_name(n: &str) -> State {
    match n {
        "Data state" => State::Data,
        "PLAINTEXT state" => State::PlainText,
        "RCDATA state" => State::RcData,
        "RAWTEXT state" => State::RawText,
        "Script data state" => State::ScriptData,
        "CDATA section state" => State::CdataSection,
        other => panic!("unknown state {other}"),
    }
}

fn render_actual(tokens: &[Token], atoms: &Interner) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for t in tokens {
        match t {
            Token::Text(s) => {
                if let Some(last) = out.last_mut()
                    && last.starts_with("Character:")
                {
                    last.push_str(s);
                } else {
                    out.push(format!("Character:{s}"));
                }
            }
            Token::Comment(s) => out.push(format!("Comment:{s}")),
            Token::ProcessingInstruction { target, data } => out.push(format!("PI:{target}|{data}")),
            Token::Doctype(d) => out.push(format!(
                "DOCTYPE:{:?}|{:?}|{:?}|{}",
                d.name,
                d.public_id,
                d.system_id,
                !d.force_quirks
            )),
            Token::StartTag(t) => {
                let mut attrs: Vec<(String, String)> =
                    t.attrs.iter().map(|a| (atoms.get(a.name).to_string(), t.attrs.value(a).to_string())).collect();
                attrs.sort();
                out.push(format!("StartTag:{}|{attrs:?}|{}", atoms.get(t.name), t.self_closing));
            }
            Token::EndTag(t) => out.push(format!("EndTag:{}", atoms.get(t.name))),
            Token::Eof => {}
        }
    }
    out
}

fn opt_str(j: &Json, un: &dyn Fn(&str) -> Option<String>) -> Option<Option<String>> {
    match j {
        Json::Null => Some(None),
        Json::Str(s) => Some(Some(un(s)?)),
        _ => None,
    }
}

fn render_expected(output: &[Json], un: &dyn Fn(&str) -> Option<String>) -> Option<Vec<String>> {
    let mut out: Vec<String> = Vec::new();
    for tok in output {
        let a = tok.as_arr()?;
        let kind = a.first()?.as_str()?;
        match kind {
            "Character" => {
                let s = un(a.get(1)?.as_str()?)?;
                if let Some(last) = out.last_mut()
                    && last.starts_with("Character:")
                {
                    last.push_str(&s);
                } else {
                    out.push(format!("Character:{s}"));
                }
            }
            "Comment" => out.push(format!("Comment:{}", un(a.get(1)?.as_str()?)?)),
            "ProcessingInstruction" => {
                out.push(format!("PI:{}|{}", un(a.get(1)?.as_str()?)?, un(a.get(2)?.as_str()?)?));
            }
            "DOCTYPE" => {
                let name = opt_str(a.get(1)?, un)?;
                let public = opt_str(a.get(2)?, un)?;
                let system = opt_str(a.get(3)?, un)?;
                let ok = a.get(4)?.as_bool()?;
                out.push(format!("DOCTYPE:{name:?}|{public:?}|{system:?}|{ok}"));
            }
            "StartTag" => {
                let name = un(a.get(1)?.as_str()?)?;
                let mut attrs: Vec<(String, String)> = Vec::new();
                if let Some(Json::Obj(kv)) = a.get(2) {
                    for (k, v) in kv {
                        attrs.push((un(k)?, un(v.as_str()?)?));
                    }
                }
                attrs.sort();
                let sc = a.get(3).and_then(Json::as_bool).unwrap_or(false);
                out.push(format!("StartTag:{name}|{attrs:?}|{sc}"));
            }
            "EndTag" => out.push(format!("EndTag:{}", un(a.get(1)?.as_str()?)?)),
            _ => return None,
        }
    }
    Some(out)
}

#[test]
fn html5lib_tokenizer() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/html5lib/tokenizer");
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "test"))
        .collect();
    files.sort();
    let (mut total, mut passed, mut skipped) = (0, 0, 0);
    let mut failures: Vec<String> = Vec::new();
    let mut skips: Vec<String> = Vec::new();
    for f in files {
        let name = f.file_name().unwrap().to_string_lossy().to_string();
        // xmlViolation.test checks the XML infoset coercion, which an HTML parser for browsers does not do.
        if name == "xmlViolation.test" {
            continue;
        }
        let data = parse_json(&std::fs::read_to_string(&f).unwrap()).unwrap();
        let Some(tests) = data.get("tests").and_then(Json::as_arr) else { continue };
        for t in tests {
            let desc = t.get("description").and_then(Json::as_str).unwrap_or("?").to_string();
            let double = t.get("doubleEscaped").and_then(Json::as_bool).unwrap_or(false);
            let un = |s: &str| -> Option<String> { if double { unescape(s) } else { Some(s.to_string()) } };
            let states: Vec<&str> = match t.get("initialStates").and_then(Json::as_arr) {
                Some(v) => v.iter().filter_map(Json::as_str).collect(),
                None => vec!["Data state"],
            };
            let input = t.get("input").and_then(Json::as_str).unwrap();
            let last = t.get("lastStartTag").and_then(Json::as_str);
            let (Some(input), Some(expected)) = (
                un(input),
                t.get("output").and_then(Json::as_arr).and_then(|o| render_expected(o, &un)),
            ) else {
                skipped += states.len();
                skips.push(format!("{name}: {desc} (needs a lone surrogate, not representable in a Rust string)"));
                continue;
            };
            for st in states {
                total += 1;
                let mut atoms = Interner::new();
                let mut tk = Tokenizer::new(input.clone());
                tk.set_state(state_from_name(st));
                if let Some(l) = last {
                    tk.set_last_start_tag(l);
                }
                let mut toks = Vec::new();
                loop {
                    let tok = tk.next_token(&mut atoms);
                    let done = tok == Token::Eof;
                    toks.push(tok);
                    if done {
                        break;
                    }
                }
                let actual = render_actual(&toks, &atoms);
                if actual == expected {
                    passed += 1;
                } else {
                    failures.push(format!("{name}: {desc} [{st}] input={input:?}\n  want {expected:?}\n  got  {actual:?}"));
                }
            }
        }
    }
    println!("html5lib tokenizer: {passed}/{total} passed, {skipped} skipped, {} failed", failures.len());
    for s in &skips {
        println!("SKIP {s}");
    }
    for f in failures.iter().take(40) {
        println!("FAIL {f}");
    }
    assert!(failures.is_empty(), "{} tokenizer failures", failures.len());
}
