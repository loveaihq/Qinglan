//! Text dump of computed styles (used by `qinglan-cli style` and the tests).

use super::cascade::StyleTree;
use super::computed::ComputedStyle;
use crate::html::dom::{DOCUMENT, Document, NodeId};

/// For each element in tree order: its tag (with id and classes) and the computed properties that differ
/// from the initial values, one per line.
pub fn dump_styles(doc: &Document, tree: &StyleTree) -> String {
    let initial = ComputedStyle::initial().properties();
    let mut out = String::new();
    let mut stack: Vec<(NodeId, usize)> = vec![(DOCUMENT, 0)];
    while let Some((n, depth)) = stack.pop() {
        let kids: Vec<NodeId> = doc.children(n).collect();
        let child_depth = if n == DOCUMENT { 0 } else { depth + 1 };
        for k in kids.into_iter().rev() {
            if !doc.is_template(n) {
                stack.push((k, child_depth));
            }
        }
        let Some(e) = doc.element(n) else { continue };
        let indent = "  ".repeat(depth);
        out.push_str(&indent);
        out.push_str(doc.atoms.get(e.name));
        if let Some(id) = doc.attr(n, crate::html::atoms::a::ID) {
            out.push('#');
            out.push_str(id);
        }
        if let Some(c) = doc.attr(n, crate::html::atoms::a::CLASS) {
            for cls in c.split_ascii_whitespace() {
                out.push('.');
                out.push_str(cls);
            }
        }
        out.push('\n');
        if let Some(style) = tree.get(n) {
            for ((name, value), (_, init)) in style.properties().into_iter().zip(initial.iter()) {
                if &value != init {
                    out.push_str(&format!("{indent}  {name}: {value}\n"));
                }
            }
        }
    }
    out
}
