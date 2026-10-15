//! Text dump of a layout (used by `qinglan-cli layout` and the snapshot tests).
//!
//! ```text
//! viewport 800x600
//! block html (0,0 800x40)
//!   block body (8,8 784x24) [clip]
//!     line (8,8 784x16) baseline=21
//!       text "hello" (8,8 80x16)
//!       br (88,8 0x16)
//! ```
//!
//! Every box is one line: the kind, the element (`tag#id.class`) or `(anonymous)`, then
//! `(x,y WxH)` of the border box in CSS px, in page coordinates. `[display:...]` marks a box that
//! came from a fallback (flex, grid, table parts, inline-block ...), `[clip]` a box with
//! `overflow: hidden/clip`. Lines show their box and the page y of the baseline; text fragments show
//! their content area (font ascent + descent high); `br` and `newline` mark a forced
//! break (a `<br>` element, a preserved line feed). Numbers print without trailing zeros; the
//! smallest unit is 1/64 px = 0.015625.

use super::units::{LayoutUnit as LU, fmt_lu};
use super::{BoxId, BoxKind, FragKind, LayoutResult};
use crate::html::atoms::a;
use crate::html::dom::{Document, NodeId};

fn element_label(doc: &Document, n: NodeId) -> String {
    let Some(e) = doc.element(n) else { return String::new() };
    let mut s = doc.atoms.get(e.name).to_string();
    if let Some(id) = doc.attr(n, a::ID) {
        s.push('#');
        s.push_str(id);
    }
    if let Some(c) = doc.attr(n, a::CLASS) {
        for cls in c.split_ascii_whitespace() {
            s.push('.');
            s.push_str(cls);
        }
    }
    s
}

fn escape(t: &str) -> String {
    let mut s = String::new();
    for c in t.chars() {
        match c {
            '"' => s.push_str("\\\""),
            '\\' => s.push_str("\\\\"),
            '\n' => s.push_str("\\n"),
            '\t' => s.push_str("\\t"),
            c if c.is_control() => s.push_str(&format!("\\u{{{:x}}}", c as u32)),
            c => s.push(c),
        }
    }
    s
}

fn rect(x: LU, y: LU, w: LU, h: LU) -> String {
    format!("({},{} {}x{})", fmt_lu(x), fmt_lu(y), fmt_lu(w), fmt_lu(h))
}

pub fn dump_layout(doc: &Document, res: &LayoutResult) -> String {
    let mut out = String::new();
    out.push_str(&format!("viewport {}x{}\n", fmt_lu(res.viewport.0), fmt_lu(res.viewport.1)));
    dump_box(doc, res, res.tree.root, LU::ZERO, LU::ZERO, 0, &mut out);
    out
}

fn dump_box(doc: &Document, res: &LayoutResult, id: BoxId, ox: LU, oy: LU, depth: usize, out: &mut String) {
    let b = res.tree.get(id);
    let o = res.box_out(id);
    let (ax, ay) = (ox + o.x, oy + o.y);
    let indent = "  ".repeat(depth);
    let name = if b.anonymous { "(anonymous)".to_string() } else { element_label(doc, b.node) };
    out.push_str(&format!("{indent}block {name} {}", rect(ax, ay, o.w, o.h)));
    if let Some(d) = b.fallback {
        out.push_str(&format!(" [display:{}]", d.keyword()));
    }
    if o.clip {
        out.push_str(" [clip]");
    }
    out.push('\n');
    for li in o.lines.0..o.lines.1 {
        let Some(l) = res.lines.get(li as usize) else { continue };
        out.push_str(&format!("{indent}  line {} baseline={}\n", rect(ax + l.x, ay + l.y, l.w, l.h), fmt_lu(ay + l.y + l.baseline)));
        for fi in l.frags.0..l.frags.1 {
            let Some(f) = res.fragments.get(fi as usize) else { continue };
            let r = rect(ax + f.x, ay + f.y, f.w, f.h);
            match f.kind {
                FragKind::Text { text, start, end, .. } => {
                    let t = res.texts.get(text as usize).and_then(|t| t.get(start as usize..end as usize)).unwrap_or("");
                    out.push_str(&format!("{indent}    text \"{}\" {r}\n", escape(t)));
                }
                // A `<br>` element, or a preserved line feed in the text.
                FragKind::Br => {
                    let what = if res.tree.get(f.box_id).kind == BoxKind::LineBreak { "br" } else { "newline" };
                    out.push_str(&format!("{indent}    {what} {r}\n"));
                }
            }
        }
    }
    for &c in &b.children {
        if res.tree.get(c).kind == BoxKind::Block {
            dump_box(doc, res, c, ax, ay, depth + 1, out);
        }
    }
}
