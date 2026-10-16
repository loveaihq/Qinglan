//! Step 2a-1: the WPT reftests that passed when `tests/wpt/passing.txt` was made still pass (`qinglan-cli wpt --check`).
#![allow(clippy::indexing_slicing, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::process::Command;

#[test]
fn the_listed_wpt_reftests_still_pass() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/wpt");
    let out = Command::new(env!("CARGO_BIN_EXE_qinglan-cli")).args(["wpt", "--check", "--root"]).arg(&root).output().unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{text}{}", String::from_utf8_lossy(&out.stderr));
    assert!(text.contains("listed tests pass"), "{text}");
}
