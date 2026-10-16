//! The memory breakdown of `--bench` and `qinglan-cli bench`: what each structure holds, found by dropping it and
//! seeing how much the counting allocator's live bytes shrink (see `sys/alloc.rs`), plus a split of the DOM's own
//! parts. Only meaningful after [`crate::count_heap`].

use qinglan_engine::html::dom::{Document, Node, NodeData, StoredAttr};

use crate::heap_live;

/// Rows of `(name, bytes)` in the order they were taken.
#[derive(Default)]
pub struct Breakdown {
    rows: Vec<(String, usize)>,
}

impl Breakdown {
    /// Run `f` (it drops or clears something) and record how much the live heap shrank.
    pub fn freed(&mut self, name: &str, f: impl FnOnce()) {
        let (before, blocks) = (heap_live(), crate::heap_blocks());
        f();
        self.rows.push((name.to_string(), before.saturating_sub(heap_live())));
        self.rows.push((format!("{name}-blocks"), blocks.saturating_sub(crate::heap_blocks())));
    }

    /// Record a number measured some other way.
    pub fn note(&mut self, name: &str, bytes: usize) {
        self.rows.push((name.to_string(), bytes));
    }

    /// `mem-<name>: <bytes>` lines.
    pub fn lines(&self) -> String {
        let mut s = String::new();
        for (k, v) in &self.rows {
            s.push_str(&format!("mem-{k}: {v}\n"));
        }
        s
    }
}

/// The DOM's parts, as capacities: `(node array, text and comment strings, attribute arrays and values)`.
pub fn dom_parts(doc: &Document) -> (usize, usize, usize) {
    let nodes = doc.nodes.capacity() * std::mem::size_of::<Node>();
    let (mut text, mut attrs) = (0, 0);
    for n in &doc.nodes {
        match &n.data {
            NodeData::Text(t) | NodeData::Comment(t) => text += t.capacity(),
            NodeData::Element(e) => attrs += e.attrs.capacity() * std::mem::size_of::<StoredAttr>(),
            _ => {}
        }
    }
    (nodes, text, attrs + doc.attr_pool.capacity())
}

/// `bytes` as megabytes with one decimal.
pub fn mb(bytes: u64) -> String {
    format!("{:.1} MB", bytes as f64 / 1_048_576.0)
}

/// The arrays of a layout result: `(name, number of elements, bytes held by the array and what hangs off it)`.
pub fn layout_parts(l: &qinglan_engine::layout::LayoutResult) -> Vec<(&'static str, usize, usize)> {
    use std::mem::size_of;
    fn arr<T>(v: &Vec<T>) -> (usize, usize) {
        (v.len(), v.capacity() * size_of::<T>())
    }
    let mut rows = Vec::new();
    let (n, b) = arr(&l.tree.boxes);
    let kids: usize = l.tree.boxes.iter().map(|x| x.children.capacity() * size_of::<u32>()).sum();
    rows.push(("box-tree-boxes", n, b));
    rows.push(("box-tree-children-arrays", l.tree.boxes.len(), kids));
    let (n, b) = arr(&l.boxes);
    rows.push(("box-positions", n, b));
    let (n, b) = arr(&l.lines);
    rows.push(("lines", n, b));
    let (n, b) = arr(&l.fragments);
    rows.push(("fragments", n, b));
    let (n, b) = arr(&l.glyphs);
    rows.push(("glyphs", n, b));
    let (n, b) = arr(&l.texts);
    rows.push(("texts", n, b + l.texts.iter().map(String::capacity).sum::<usize>()));
    let (n, b) = arr(&l.decorations);
    rows.push(("decorations", n, b));
    let (n, b) = arr(&l.images);
    rows.push(("images", n, b + arr(&l.tree.images).1));
    rows
}
