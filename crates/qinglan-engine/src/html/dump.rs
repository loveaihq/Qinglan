//! DOM dump in the html5lib tree-construction test format (used by `qinglan-cli dom` and the tests).

use super::dom::{Attribute, Document, NodeData, NodeId, Namespace, NONE};

/// Dump the children of `root` (the document node, or a fragment root) in html5lib format.
pub fn dump_children(doc: &Document, root: NodeId) -> String {
    let mut out = String::new();
    // Iterative walk: (node, depth).
    let mut stack: Vec<(NodeId, usize)> = Vec::new();
    let kids: Vec<NodeId> = doc.children(root).collect();
    for k in kids.into_iter().rev() {
        stack.push((k, 0));
    }
    while let Some((id, depth)) = stack.pop() {
        let Some(node) = doc.node(id) else { continue };
        let indent = "  ".repeat(depth);
        match &node.data {
            NodeData::Doctype(d) => {
                out.push_str(&format!("| {indent}<!DOCTYPE {}", d.name));
                if !d.public_id.is_empty() || !d.system_id.is_empty() {
                    out.push_str(&format!(" \"{}\" \"{}\"", d.public_id, d.system_id));
                }
                out.push_str(">\n");
            }
            NodeData::Comment(c) => out.push_str(&format!("| {indent}<!-- {c} -->\n")),
            NodeData::ProcessingInstruction { target, data } => {
                out.push_str(&format!("| {indent}<?{target} {data}>\n"));
            }
            NodeData::Text(t) => out.push_str(&format!("| {indent}\"{t}\"\n")),
            NodeData::Element(e) => {
                let name = doc.atoms.get(e.name);
                let prefix = match e.ns {
                    Namespace::Svg => "svg ",
                    Namespace::MathMl => "math ",
                    _ => "",
                };
                out.push_str(&format!("| {indent}<{prefix}{name}>\n"));
                let mut attrs: Vec<(String, &Attribute)> = e.attrs.iter().map(|a| (attr_display_name(doc, a), a)).collect();
                attrs.sort_by(|x, y| x.0.cmp(&y.0));
                for (n, a) in attrs {
                    out.push_str(&format!("| {indent}  {n}=\"{}\"\n", a.value));
                }
                // Children (pushed in reverse so they pop in order). A template's contents are shown under "content".
                if e.template_contents != NONE {
                    let tc = e.template_contents;
                    out.push_str(&format!("| {indent}  content\n"));
                    let kids: Vec<NodeId> = doc.children(tc).collect();
                    for k in kids.into_iter().rev() {
                        stack.push((k, depth + 2));
                    }
                }
            }
            NodeData::Document | NodeData::Fragment => {}
        }
        if let NodeData::Element(e) = &node.data
            && e.template_contents == NONE
        {
            let kids: Vec<NodeId> = doc.children(id).collect();
            for k in kids.into_iter().rev() {
                stack.push((k, depth + 1));
            }
        }
    }
    out
}

fn attr_display_name(doc: &Document, a: &Attribute) -> String {
    let local = doc.atoms.get(a.name);
    match a.ns {
        Namespace::XLink | Namespace::Xml | Namespace::Xmlns => {
            let prefix = doc.atoms.get(a.prefix);
            if prefix.is_empty() { local.to_string() } else { format!("{prefix} {local}") }
        }
        _ => local.to_string(),
    }
}
