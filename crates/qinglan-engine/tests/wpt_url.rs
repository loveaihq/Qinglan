//! WHATWG URL conformance: web-platform-tests `urltestdata.json` (BSD-3-Clause, tests/wpt-url/).
#![allow(clippy::indexing_slicing, clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod common;

use common::{Json, parse_json};
use qinglan_engine::url::Url;

#[test]
fn wpt_urltestdata() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/wpt-url/urltestdata.json");
    let text = std::fs::read_to_string(path).expect("read urltestdata.json");
    let data = parse_json(&text).expect("parse json");
    let mut total = 0;
    let mut failed: Vec<String> = Vec::new();
    for case in data.as_arr().unwrap() {
        let Json::Obj(_) = case else { continue }; // comment strings
        total += 1;
        let input = case.get("input").and_then(Json::as_str).unwrap();
        let base_s = case.get("base").and_then(Json::as_str);
        let expect_failure = case.get("failure").and_then(Json::as_bool).unwrap_or(false);
        let base = match base_s {
            Some(b) => match Url::parse(b, None) {
                Ok(u) => Some(u),
                Err(_) => {
                    // A base that fails to parse: the case expects failure.
                    if !expect_failure {
                        failed.push(format!("{input:?} base {b:?}: base failed to parse"));
                    }
                    continue;
                }
            },
            None => None,
        };
        let got = Url::parse(input, base.as_ref());
        if expect_failure {
            if let Ok(u) = got {
                failed.push(format!("{input:?} base {base_s:?}: expected failure, got {}", u.href()));
            }
            continue;
        }
        let u = match got {
            Ok(u) => u,
            Err(_) => {
                failed.push(format!("{input:?} base {base_s:?}: unexpected failure"));
                continue;
            }
        };
        let want = |k: &str| case.get(k).and_then(Json::as_str).unwrap_or("").to_string();
        let checks = [
            ("href", u.href()),
            ("origin", u.origin()),
            ("protocol", u.protocol()),
            ("username", u.username.clone()),
            ("password", u.password.clone()),
            ("host", u.host_with_port()),
            ("hostname", u.hostname()),
            ("port", u.port_string()),
            ("pathname", u.pathname()),
            ("search", u.search()),
            ("hash", u.hash()),
        ];
        for (k, got) in checks {
            // `origin` is only present for some cases.
            if k == "origin" && case.get("origin").is_none() {
                continue;
            }
            if got != want(k) {
                failed.push(format!("{input:?} base {base_s:?}: {k}: got {got:?}, want {:?}", want(k)));
                break;
            }
        }
    }
    let passed = total - failed.len();
    println!("WPT URL: {passed}/{total} passed ({:.1}%)", 100.0 * passed as f64 / total as f64);
    if let Ok(out) = std::env::var("WPT_URL_FAILURES") {
        let _ = std::fs::write(out, failed.join("\n"));
    }
    // Regression guard: raise this as the parser improves.
    assert!(passed * 100 >= total * 85, "{} failures, first: {:#?}", failed.len(), &failed[..failed.len().min(10)]);
}
