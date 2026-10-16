//! The DOM: nodes live in one array and refer to each other by `u32` index.
//! Tag names and attribute names are interned [`Atom`]s.

use super::atoms::{Atom, Interner, a};

pub type NodeId = u32;
/// "No node" in parent/child/sibling links.
pub const NONE: NodeId = u32::MAX;
/// The document node is always node 0.
pub const DOCUMENT: NodeId = 0;

/// Node count limit per page (L1 task 5): content after it is dropped.
pub const MAX_NODES: usize = 500_000;
/// Open-element depth limit (L1 task 5): deeper elements are attached to the 512th level.
pub const MAX_DEPTH: usize = 512;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Namespace {
    /// Attribute without a namespace.
    None,
    Html,
    MathMl,
    Svg,
    XLink,
    Xml,
    Xmlns,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attribute {
    pub ns: Namespace,
    /// Prefix for adjusted foreign attributes (`xlink` in `xlink:href`), `a::EMPTY` otherwise.
    pub prefix: Atom,
    pub name: Atom,
    pub value: String,
}

/// An attribute as the document keeps it: the value is a range of [`Document::attr_pool`] (one string for all the
/// values of the page instead of a heap block each; read it with [`Document::value`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StoredAttr {
    pub ns: Namespace,
    /// Prefix for adjusted foreign attributes (`xlink` in `xlink:href`), `a::EMPTY` otherwise.
    pub prefix: Atom,
    pub name: Atom,
    off: u32,
    len: u32,
}

#[derive(Clone, Debug)]
pub struct ElementData {
    pub name: Atom,
    pub ns: Namespace,
    pub attrs: Vec<StoredAttr>,
    /// For `<template>` (HTML): the template contents document fragment.
    pub template_contents: NodeId,
}

#[derive(Clone, Debug)]
pub struct DoctypeData {
    pub name: String,
    pub public_id: String,
    pub system_id: String,
}

#[derive(Clone, Debug)]
pub enum NodeData {
    Document,
    /// Document fragment (template contents, fragment parsing root).
    Fragment,
    Doctype(Box<DoctypeData>),
    Element(ElementData),
    Text(String),
    Comment(String),
    ProcessingInstruction { target: String, data: String },
}

#[derive(Clone, Debug)]
pub struct Node {
    pub parent: NodeId,
    pub first_child: NodeId,
    pub last_child: NodeId,
    pub prev_sibling: NodeId,
    pub next_sibling: NodeId,
    pub data: NodeData,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum QuirksMode {
    NoQuirks,
    LimitedQuirks,
    Quirks,
}

pub struct Document {
    pub nodes: Vec<Node>,
    pub atoms: Interner,
    pub quirks: QuirksMode,
    /// True when the node limit was hit and the rest of the page was dropped ("页面太大，只显示了前面部分").
    pub too_large: bool,
    /// The values of all the attributes, one after the other (see [`StoredAttr`]).
    pub attr_pool: String,
}

impl Document {
    pub fn new(atoms: Interner) -> Document {
        let mut d = Document { nodes: Vec::new(), atoms, quirks: QuirksMode::NoQuirks, too_large: false, attr_pool: String::new() };
        d.nodes.push(Self::blank(NodeData::Document));
        d
    }

    fn blank(data: NodeData) -> Node {
        Node { parent: NONE, first_child: NONE, last_child: NONE, prev_sibling: NONE, next_sibling: NONE, data }
    }

    /// Allocate a detached node.
    pub fn alloc(&mut self, data: NodeData) -> NodeId {
        let id = self.nodes.len() as NodeId;
        self.nodes.push(Self::blank(data));
        id
    }

    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(id as usize)
    }

    pub fn node_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        self.nodes.get_mut(id as usize)
    }

    pub fn element(&self, id: NodeId) -> Option<&ElementData> {
        match &self.node(id)?.data {
            NodeData::Element(e) => Some(e),
            _ => None,
        }
    }

    pub fn element_mut(&mut self, id: NodeId) -> Option<&mut ElementData> {
        match &mut self.node_mut(id)?.data {
            NodeData::Element(e) => Some(e),
            _ => None,
        }
    }

    pub fn parent(&self, id: NodeId) -> NodeId {
        self.node(id).map_or(NONE, |n| n.parent)
    }
    pub fn first_child(&self, id: NodeId) -> NodeId {
        self.node(id).map_or(NONE, |n| n.first_child)
    }
    pub fn last_child(&self, id: NodeId) -> NodeId {
        self.node(id).map_or(NONE, |n| n.last_child)
    }
    pub fn next_sibling(&self, id: NodeId) -> NodeId {
        self.node(id).map_or(NONE, |n| n.next_sibling)
    }
    pub fn prev_sibling(&self, id: NodeId) -> NodeId {
        self.node(id).map_or(NONE, |n| n.prev_sibling)
    }

    pub fn children(&self, id: NodeId) -> Children<'_> {
        Children { doc: self, next: self.first_child(id) }
    }

    /// Append `child` (detached) as the last child of `parent`.
    pub fn append_child(&mut self, parent: NodeId, child: NodeId) {
        let last = self.last_child(parent);
        if let Some(c) = self.node_mut(child) {
            c.parent = parent;
            c.prev_sibling = last;
            c.next_sibling = NONE;
        }
        if let Some(l) = self.node_mut(last) {
            l.next_sibling = child;
        }
        if let Some(p) = self.node_mut(parent) {
            if p.first_child == NONE {
                p.first_child = child;
            }
            p.last_child = child;
        }
    }

    /// Insert `child` (detached) before `reference`, a child of `parent`.
    pub fn insert_before(&mut self, parent: NodeId, child: NodeId, reference: NodeId) {
        if reference == NONE {
            return self.append_child(parent, child);
        }
        let prev = self.prev_sibling(reference);
        if let Some(c) = self.node_mut(child) {
            c.parent = parent;
            c.prev_sibling = prev;
            c.next_sibling = reference;
        }
        if let Some(r) = self.node_mut(reference) {
            r.prev_sibling = child;
        }
        if let Some(p) = self.node_mut(prev) {
            p.next_sibling = child;
        } else if let Some(p) = self.node_mut(parent) {
            p.first_child = child;
        }
    }

    /// Detach `id` from its parent (the node and its subtree stay in the array).
    pub fn detach(&mut self, id: NodeId) {
        let (parent, prev, next) = match self.node(id) {
            Some(n) => (n.parent, n.prev_sibling, n.next_sibling),
            None => return,
        };
        if parent == NONE {
            return;
        }
        if let Some(p) = self.node_mut(prev) {
            p.next_sibling = next;
        } else if let Some(p) = self.node_mut(parent) {
            p.first_child = next;
        }
        if let Some(n) = self.node_mut(next) {
            n.prev_sibling = prev;
        } else if let Some(p) = self.node_mut(parent) {
            p.last_child = prev;
        }
        if let Some(n) = self.node_mut(id) {
            n.parent = NONE;
            n.prev_sibling = NONE;
            n.next_sibling = NONE;
        }
    }

    /// Move all children of `from` to the end of `to`.
    pub fn move_children(&mut self, from: NodeId, to: NodeId) {
        let mut c = self.first_child(from);
        while c != NONE {
            let next = self.next_sibling(c);
            self.detach(c);
            self.append_child(to, c);
            c = next;
        }
    }

    pub fn is_html_element(&self, id: NodeId, name: Atom) -> bool {
        self.element(id).is_some_and(|e| e.ns == Namespace::Html && e.name == name)
    }

    pub fn attr(&self, id: NodeId, name: Atom) -> Option<&str> {
        self.element(id)?.attrs.iter().find(|a| a.ns == Namespace::None && a.name == name).map(|a| self.value(a))
    }

    /// The value of a stored attribute.
    pub fn value(&self, a: &StoredAttr) -> &str {
        let (off, len) = (a.off as usize, a.len as usize);
        self.attr_pool.get(off..off + len).unwrap_or("")
    }

    /// Put the attributes' values in the pool. (A page whose values would not fit in the pool's 32-bit offsets keeps
    /// the attributes with empty values.)
    pub fn store_attrs(&mut self, attrs: Vec<Attribute>) -> Vec<StoredAttr> {
        let mut out = Vec::with_capacity(attrs.len());
        for at in attrs {
            let off = self.attr_pool.len();
            let fits = u32::try_from(off + at.value.len()).is_ok();
            if fits {
                self.attr_pool.push_str(&at.value);
            }
            let len = if fits { at.value.len() as u32 } else { 0 };
            out.push(StoredAttr { ns: at.ns, prefix: at.prefix, name: at.name, off: off.min(u32::MAX as usize) as u32, len });
        }
        out
    }

    /// The stored attribute as a token's attribute (a copy of the value).
    pub fn token_attr(&self, a: &StoredAttr) -> Attribute {
        Attribute { ns: a.ns, prefix: a.prefix, name: a.name, value: self.value(a).to_string() }
    }

    /// The `<html>` element, if any.
    pub fn document_element(&self) -> NodeId {
        self.children(DOCUMENT).find(|&c| self.element(c).is_some()).unwrap_or(NONE)
    }

    /// Plain text content of a node (descendant text, in order).
    pub fn text_content(&self, id: NodeId) -> String {
        let mut out = String::new();
        let mut stack = vec![id];
        while let Some(n) = stack.pop() {
            match self.node(n).map(|n| &n.data) {
                Some(NodeData::Text(t)) => out.push_str(t),
                Some(_) => {
                    let kids: Vec<NodeId> = self.children(n).collect();
                    stack.extend(kids.into_iter().rev());
                }
                None => {}
            }
        }
        out
    }

    pub fn local_name(&self, id: NodeId) -> &str {
        self.element(id).map_or("", |e| self.atoms.get(e.name))
    }

    pub fn is_template(&self, id: NodeId) -> bool {
        self.is_html_element(id, a::TEMPLATE)
    }
}

pub struct Children<'a> {
    doc: &'a Document,
    next: NodeId,
}

impl Iterator for Children<'_> {
    type Item = NodeId;
    fn next(&mut self) -> Option<NodeId> {
        if self.next == NONE {
            return None;
        }
        let cur = self.next;
        self.next = self.doc.next_sibling(cur);
        Some(cur)
    }
}
