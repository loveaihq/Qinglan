//! First-screen-first (L1 task 17, engine part): a layout that stops once normal flow reaches a given y
//! gives, for every box above that line, exactly what the full layout gives.
//!
//! For each box that starts above the stop line (and is not out of flow): x, y and width are the same. For each box
//! that also ends above it: height, clipping, its lines, fragments and list marker are the same too.
//! Boxes the stop line cuts through are shorter in the partial layout (their later children are missing).
#![allow(clippy::indexing_slicing, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use qinglan_engine::css::{NoResources, StyleOptions, style_document_with};
use qinglan_engine::encoding::decode_document;
use qinglan_engine::html::treebuilder::parse_document;
use qinglan_engine::layout::boxtree::{BoxId, BoxKind, NO_BOX};
use qinglan_engine::layout::units::LayoutUnit as LU;
use qinglan_engine::layout::{LayoutOptions, LayoutResult, layout_document};
use qinglan_engine::net::blocking::{self, OpenResult};
use qinglan_engine::net::loader::Loader;
use qinglan_engine::net::replay::ReplayTransport;
use qinglan_engine::platform::{BasicPlatform, Platform, TestPlatform};
use qinglan_engine::url::Url;

struct Counts {
    top: usize,
    complete: usize,
}

fn has_out(res: &LayoutResult, id: BoxId) -> bool {
    matches!(res.tree.get(id).kind, BoxKind::Block | BoxKind::InlineBlock | BoxKind::Replaced)
}

fn out_of_flow(res: &LayoutResult, id: BoxId) -> bool {
    let mut cur = id;
    while cur != NO_BOX {
        if res.tree.get(cur).oof {
            return true;
        }
        cur = res.tree.get(cur).parent;
    }
    false
}

#[track_caller]
fn compare(name: &str, full: &LayoutResult, part: &LayoutResult, stop: f32) -> Counts {
    assert_eq!(full.tree.boxes.len(), part.tree.boxes.len(), "{name}: box trees differ");
    let stop = LU::from_f32(stop);
    let mut n = Counts { top: 0, complete: 0 };
    for id in 0..full.tree.boxes.len() as BoxId {
        if !has_out(full, id) || out_of_flow(full, id) {
            continue;
        }
        let (fo, po) = (full.box_out(id), part.box_out(id));
        let (fp, pp) = (full.page_pos(id), part.page_pos(id));
        if fp.1 >= stop {
            continue;
        }
        n.top += 1;
        assert_eq!((fp.0, fp.1, fo.w), (pp.0, pp.1, po.w), "{name}: box {id} moved or changed width");
        if fp.1 + fo.h > stop {
            continue;
        }
        n.complete += 1;
        assert_eq!((fo.h, fo.clip), (po.h, po.clip), "{name}: box {id} height or clip");
        assert_eq!(fo.lines.1 - fo.lines.0, po.lines.1 - po.lines.0, "{name}: box {id} line count");
        assert_eq!(fo.marker > 0, po.marker > 0, "{name}: box {id} marker");
        for k in 0..fo.lines.1 - fo.lines.0 {
            let (fl, pl) = (full.lines[(fo.lines.0 + k) as usize], part.lines[(po.lines.0 + k) as usize]);
            assert_eq!((fl.x, fl.y, fl.w, fl.h, fl.baseline, fl.forced), (pl.x, pl.y, pl.w, pl.h, pl.baseline, pl.forced), "{name}: box {id} line {k}");
            assert_eq!(fl.frags.1 - fl.frags.0, pl.frags.1 - pl.frags.0, "{name}: box {id} line {k} fragments");
            for j in 0..fl.frags.1 - fl.frags.0 {
                let (ff, pf) = (full.fragments[(fl.frags.0 + j) as usize], part.fragments[(pl.frags.0 + j) as usize]);
                assert_eq!((ff.box_id, ff.x, ff.y, ff.w, ff.h), (pf.box_id, pf.x, pf.y, pf.w, pf.h), "{name}: box {id} line {k} fragment {j}");
            }
        }
    }
    n
}

fn pages_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/pages")
}

fn lay(src: &str, vw: f32, vh: f32, stop: Option<f32>) -> LayoutResult {
    let platform = TestPlatform;
    let decoded = decode_document(src.as_bytes(), None, &BasicPlatform);
    let mut doc = parse_document(decoded.text);
    let sopts = StyleOptions { viewport_w: vw, viewport_h: vh, ..StyleOptions::default() };
    let styled = style_document_with(&mut doc, None, &mut NoResources, &sopts, Some(&platform as &dyn Platform));
    layout_document(&doc, &styled.tree, &platform, &LayoutOptions { viewport_w: vw, viewport_h: vh, stop_y: stop, ..LayoutOptions::default() })
}

#[test]
fn test_pages_above_the_stop_line_are_identical() {
    let mut pages = 0;
    let mut cut_pages = 0;
    let (mut top, mut complete) = (0, 0);
    for entry in std::fs::read_dir(pages_dir()).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("html") {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let src = std::fs::read_to_string(&path).unwrap();
        let full = lay(&src, 800.0, 600.0, None);
        let height = full.box_out(full.tree.root).h.to_f32();
        for frac in [0.5f32, 0.25, 0.75] {
            let stop = (height * frac).max(1.0);
            let part = lay(&src, 800.0, 600.0, Some(stop));
            let c = compare(&name, &full, &part, stop);
            top += c.top;
            complete += c.complete;
            if part.partial {
                cut_pages += 1;
            }
        }
        pages += 1;
    }
    println!("first-screen: {pages} test pages, {cut_pages} partial layouts, {top} boxes above the line, {complete} fully above it");
    assert!(pages >= 25 && cut_pages >= 20 && complete >= 300, "{pages} pages, {cut_pages} cut, {complete} complete");
}

#[test]
fn a_long_page_is_cut_and_a_short_one_is_not() {
    let mut s = String::from("<body>");
    for i in 0..1000 {
        s.push_str(&format!("<p>paragraph {i} of the long page</p>"));
    }
    let long = lay(&s, 400.0, 300.0, Some(600.0));
    assert!(long.partial);
    let full = lay(&s, 400.0, 300.0, None);
    assert!(!full.partial);
    assert!(long.work * 10 < full.work, "{} of {} work units", long.work, full.work);
    let short = lay("<body><p>one</p><p>two</p>", 400.0, 300.0, Some(600.0));
    assert!(!short.partial, "a page shorter than the stop line is complete");
}

#[test]
fn a_cut_containing_block_waits_for_the_full_layout() {
    // The relative box is cut by the stop line, so the absolute box that depends on its size is skipped.
    let mut s = String::from("<body><div style='position:relative'><div id=a style='position:absolute;bottom:0;left:0;width:10px;height:10px;background:red'></div>");
    for i in 0..500 {
        s.push_str(&format!("<p>row {i}</p>"));
    }
    s.push_str("</div></body>");
    let part = lay(&s, 400.0, 300.0, Some(500.0));
    let full = lay(&s, 400.0, 300.0, None);
    assert!(part.partial);
    let abs_id = (0..full.tree.boxes.len() as BoxId).find(|&i| full.tree.get(i).oof).unwrap();
    assert!(full.box_out(abs_id).h > LU::ZERO);
    assert_eq!(part.box_out(abs_id).h, LU::ZERO, "not laid out in the partial pass");
    assert!(full.box_out(abs_id).y > LU::px(1000), "bottom:0 of the whole relative box: y = {:?}", full.box_out(abs_id).y);
}

/// The Chinese Wikipedia article at 1920x1080 with the stop line one screen below the viewport
/// (needs `tests/corpus/local/zhwiki-rec`, otherwise it is skipped).
#[test]
fn zhwiki_above_the_stop_line_is_identical() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/corpus/local/zhwiki-rec");
    let Ok(transport) = ReplayTransport::open(&dir) else {
        println!("first-screen: tests/corpus/local/zhwiki-rec not found, skipped");
        return;
    };
    let platform = TestPlatform;
    let url = Url::parse("https://zh.wikipedia.org/wiki/%E4%B8%AD%E5%8D%8E%E4%BA%BA%E6%B0%91%E5%85%B1%E5%92%8C%E5%9B%BD", None).unwrap();
    let mut loader = Loader::without_wake(Arc::new(transport));
    let OpenResult::Page(mut opened) = blocking::open(&mut loader, &url, &platform, Duration::from_secs(30)) else { panic!("cannot open the recording") };
    let sopts = StyleOptions { viewport_w: 1920.0, viewport_h: 1080.0, ..StyleOptions::default() };
    let mut res = blocking::resources_for(&opened);
    res.start_sheets(&opened.doc, &mut loader, &sopts);
    assert!(blocking::load_sheets(&mut loader, &mut res, &platform, Duration::from_secs(30)));
    let doc_url = opened.url.clone();
    let out = style_document_with(&mut opened.doc, Some(&doc_url), &mut res, &sopts, Some(&platform as &dyn Platform));
    let opts = LayoutOptions { viewport_w: 1920.0, viewport_h: 1080.0, ..LayoutOptions::default() };
    let full = layout_document(&opened.doc, &out.tree, &platform, &opts);
    let stop = 2160.0;
    let part = layout_document(&opened.doc, &out.tree, &platform, &LayoutOptions { stop_y: Some(stop), ..opts });
    assert!(part.partial);
    let c = compare("zhwiki", &full, &part, stop);
    println!("first-screen: zhwiki: {} of {} work units, {} of {} lines, {} boxes above the line ({} fully)", part.work, full.work, part.lines.len(), full.lines.len(), c.top, c.complete);
    assert!(c.complete > 100, "{} boxes", c.complete);
    assert!(part.work * 3 < full.work);
}
