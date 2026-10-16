//! Text dump of a layout (used by `qinglan-cli layout` and the snapshot tests).
//!
//! ```text
//! viewport 800x600
//! canvas #ffffff (from body)
//! block html (0,0 800x40)
//!   block body (8,8 784x24) [clip]
//!     line (8,8 784x16) baseline=21
//!       text "hello" (8,8 80x16)
//!       br (88,8 0x16)
//! ```
//!
//! Every box is one line: the kind (`block`, `inline-block`, `replaced`), the element (`tag#id.class`)
//! or `(anonymous)`, then `(x,y WxH)` of the border box in CSS px, in page coordinates. `[display:...]`
//! marks a box that came from a fallback (flex, grid, table parts ...), `[clip]` a box with
//! `overflow: hidden/clip`, `[relative]` / `[absolute]` / `[fixed]` a positioned box, `[image]`,
//! `[placeholder:iframe]` ... what a replaced box shows. An outside list marker is a `marker "1. "`
//! line right under its item. Lines show their box and the page y of the baseline; text fragments show
//! their content area (font ascent + descent high), followed by their decoration lines
//! (`underline (x,y WxT)`); `inline span (x,y WxH)` is the border box of an inline element on that line;
//! `br` and `newline` mark a forced break (a `<br>` element, a preserved line feed). Atomic inlines
//! (`inline-block`, images) appear as a nested box line inside the line they sit in. Out-of-flow boxes
//! are listed after the normal children of the block they sit in. Numbers print without trailing
//! zeros; the smallest unit is 1/64 px = 0.015625.

use super::units::{LayoutUnit as LU, fmt_lu};
use super::boxtree::{BoxKind, ReplacedKind};
use super::{BoxId, DecorKind, FragKind, LayoutResult};
use crate::css::types::Position;
use crate::html::atoms::{FxHashMap, a};
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
    if let Some(c) = &res.canvas {
        let b = res.tree.get(c.from);
        let who = if b.node == res.tree.get(res.tree.root).node { "root" } else { "body" };
        out.push_str(&format!("canvas {} (from {who})\n", c.color.to_css()));
    }
    let mut oof: FxHashMap<BoxId, Vec<BoxId>> = FxHashMap::default();
    for &(owner, id) in &res.oof {
        oof.entry(owner).or_default().push(id);
    }
    dump_box(doc, res, &oof, res.tree.root, LU::ZERO, LU::ZERO, 0, &mut out);
    for (title, list) in [("paint-below", &res.below), ("paint-above", &res.above)] {
        for p in list {
            let b = res.tree.get(p.box_id);
            let name = if b.anonymous { "(anonymous)".to_string() } else { element_label(doc, b.node) };
            out.push_str(&format!("{title} {name} {} z={}
", p.position.keyword(), p.z_index));
        }
    }
    if res.incomplete {
        out.push_str("incomplete
");
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn dump_box(doc: &Document, res: &LayoutResult, oof: &FxHashMap<BoxId, Vec<BoxId>>, id: BoxId, ox: LU, oy: LU, depth: usize, out: &mut String) {
    // The parts are in functions of their own so that this frame, which the recursion over the box
    // tree goes through, stays small.
    let b = res.tree.get(id);
    let o = res.box_out(id);
    let (ax, ay) = (ox + o.x, oy + o.y);
    dump_head(doc, res, id, (ax, ay), depth, out);
    dump_lines(doc, res, oof, id, (ax, ay), depth, out);
    if !b.inline_children {
        for &c in &b.children {
            if !res.tree.get(c).oof {
                dump_box(doc, res, oof, c, ax, ay, depth + 1, out);
            }
        }
    } else if res.has_floats {
        // The floats of the inline content, after its lines.
        let mut floats = Vec::new();
        super::floats::floats_in(&res.tree, id, &mut floats);
        for c in floats {
            dump_box(doc, res, oof, c, ax, ay, depth + 1, out);
        }
    }
    if let Some(list) = oof.get(&id) {
        for &c in list {
            dump_box(doc, res, oof, c, ax, ay, depth + 1, out);
        }
    }
}

fn dump_head(doc: &Document, res: &LayoutResult, id: BoxId, pos: (LU, LU), depth: usize, out: &mut String) {
    let b = res.tree.get(id);
    let o = res.box_out(id);
    let (ax, ay) = pos;
    let indent = "  ".repeat(depth);
    let name = if b.anonymous { "(anonymous)".to_string() } else { element_label(doc, b.node) };
    let kind = match b.kind {
        BoxKind::InlineBlock => "inline-block",
        BoxKind::Replaced => "replaced",
        _ => "block",
    };
    out.push_str(&format!("{indent}{kind} {name} {}", rect(ax, ay, o.w, o.h)));
    if let Some(d) = b.fallback {
        out.push_str(&format!(" [display:{}]", d.keyword()));
    }
    match b.style.boxs.position {
        Position::Relative => out.push_str(" [relative]"),
        Position::Absolute => out.push_str(" [absolute]"),
        Position::Fixed => out.push_str(" [fixed]"),
        _ => {}
    }
    if super::floats::is_float(b) {
        out.push_str(&format!(" [float:{}]", b.style.boxs.float.keyword()));
    }
    if o.clip {
        out.push_str(" [clip]");
    }
    match b.replaced() {
        Some(ReplacedKind::Image) => match b.image() {
            Some(i) => out.push_str(&format!(" [image src={} d={}]", i.src, i.density)),
            None => out.push_str(" [image]"),
        },
        Some(ReplacedKind::Placeholder(k)) => out.push_str(&format!(" [placeholder:{k}]")),
        Some(ReplacedKind::Check) => out.push_str(" [check]"),
        Some(ReplacedKind::Fixed(_, _, k)) => out.push_str(&format!(" [control:{k}]")),
        None => {}
    }
    out.push('\n');
    if o.marker > 0
        && let Some(m) = res.markers.get(o.marker as usize - 1)
        && let FragKind::Marker { text, start, end, .. } = m.kind
    {
        let t = res.texts.get(text as usize).and_then(|t| t.get(start as usize..end as usize)).unwrap_or("");
        out.push_str(&format!("{indent}  marker \"{}\" {}\n", escape(t), rect(ax + m.x, ay + m.y, m.w, m.h)));
    }
}

#[allow(clippy::too_many_arguments)]
fn dump_lines(doc: &Document, res: &LayoutResult, oof: &FxHashMap<BoxId, Vec<BoxId>>, id: BoxId, pos: (LU, LU), depth: usize, out: &mut String) {
    let o = res.box_out(id);
    let (ax, ay) = pos;
    let indent = "  ".repeat(depth);
    for li in o.lines.0..o.lines.1 {
        let Some(l) = res.lines.get(li as usize) else { continue };
        out.push_str(&format!("{indent}  line {} baseline={}\n", rect(ax + l.x, ay + l.y, l.w, l.h), fmt_lu(ay + l.y + l.baseline)));
        for fi in l.frags.0..l.frags.1 {
            let Some(f) = res.fragments.get(fi as usize) else { continue };
            let r = rect(ax + f.x, ay + f.y, f.w, f.h);
            match f.kind {
                FragKind::Text { text, start, end, decor, .. } => {
                    let t = res.texts.get(text as usize).and_then(|t| t.get(start as usize..end as usize)).unwrap_or("");
                    out.push_str(&format!("{indent}    text \"{}\" {r}\n", escape(t)));
                    for di in decor.0..decor.1 {
                        let Some(d) = res.decorations.get(di as usize) else { continue };
                        let k = match d.kind {
                            DecorKind::Underline => "underline",
                            DecorKind::Overline => "overline",
                            DecorKind::LineThrough => "line-through",
                        };
                        out.push_str(&format!("{indent}      {k} {} {} {}\n", rect(ax + d.x, ay + d.y, d.w, d.thickness), d.style.keyword(), d.color.to_css()));
                    }
                }
                FragKind::Marker { text, start, end, .. } => {
                    let t = res.texts.get(text as usize).and_then(|t| t.get(start as usize..end as usize)).unwrap_or("");
                    out.push_str(&format!("{indent}    marker \"{}\" {r}\n", escape(t)));
                }
                // A `<br>` element, or a preserved line feed in the text.
                FragKind::Br => {
                    let what = if res.tree.get(f.box_id).kind == BoxKind::LineBreak { "br" } else { "newline" };
                    out.push_str(&format!("{indent}    {what} {r}\n"));
                }
                FragKind::InlineBox { first, last } => {
                    let ends = match (first, last) {
                        (true, true) => " [first last]",
                        (true, false) => " [first]",
                        (false, true) => " [last]",
                        (false, false) => "",
                    };
                    out.push_str(&format!("{indent}    inline {} {r}{ends}\n", element_label(doc, res.tree.get(f.box_id).node)));
                }
                FragKind::Atomic => dump_box(doc, res, oof, f.box_id, ax, ay, depth + 2, out),
            }
        }
    }
}
