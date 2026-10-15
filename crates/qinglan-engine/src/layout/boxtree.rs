//! The box tree (L1 task 9): boxes made from the DOM and the computed styles.
//!
//! * Boxes follow `display` (css-display-3 2). `display: none` makes none; `display: contents` makes
//!   none for the element itself and its children take its place.
//! * Block-level boxes that sit among inline-level siblings put the inline ones into anonymous block
//!   boxes (CSS2 9.2.1.1); an inline box that contains a block-level box is split around it.
//! * Whitespace-only text between blocks makes no box (CSS2 9.2.2.1; css-text-3 4.1.1 note).
//! * Fallbacks for what layer 1 does not lay out: `flex`, `grid`, `table*` become `block`; their
//!   `inline-` forms become `inline-block`, which in step 1b-1 is laid out as a block (1b-2 adds
//!   real atomic inlines); `float` is ignored (the element stays in the normal flow).

use std::rc::Rc;

use crate::css::cascade::StyleTree;
use crate::css::computed::{BoxStyle, ComputedStyle};
use crate::css::types::{Display, WhiteSpaceCollapse};
use crate::html::atoms::a;
use crate::html::dom::{Document, Namespace, NodeData, NodeId, NONE};

pub type BoxId = u32;
pub const NO_BOX: BoxId = u32::MAX;

/// Deeper than this many nested elements, an element makes no box of its own (it is treated like
/// `display: contents`), which keeps the recursion of box building and layout bounded.
pub const MAX_BOX_DEPTH: usize = 256;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BoxKind {
    /// A block container that is block-level (also anonymous blocks).
    Block,
    /// A non-atomic inline box (`<span>`), or one part of it when a block split it.
    Inline,
    /// The text of one DOM text node.
    Text,
    /// A `<br>`: a forced line break.
    LineBreak,
}

/// Which part of an inline element a box is, when a block-level descendant split it in pieces
/// (CSS2 9.2.1.1). Padding, border and margin of the element belong to the first piece's start
/// and the last piece's end (used by 1b-2).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct SplitPart {
    pub is_split: bool,
    pub first: bool,
    pub last: bool,
}

pub struct LayoutBox {
    pub kind: BoxKind,
    /// The DOM node (element or text); `NONE` for anonymous boxes.
    pub node: NodeId,
    /// Text boxes carry the style of their parent element.
    pub style: Rc<ComputedStyle>,
    pub parent: BoxId,
    pub children: Vec<BoxId>,
    pub anonymous: bool,
    /// For a block: its children are inline-level and form one inline formatting context.
    /// Otherwise all children are blocks.
    pub inline_children: bool,
    /// The computed `display` when a fallback replaced it (flex, grid, table parts, inline-block ...).
    pub fallback: Option<Display>,
    pub split: SplitPart,
}

pub struct BoxTree {
    pub boxes: Vec<LayoutBox>,
    pub root: BoxId,
    dummy: LayoutBox,
}

impl BoxTree {
    /// The box with this id. Out-of-range ids give an empty dummy block (never happens for ids the
    /// builder made; this keeps the engine free of panics on any input).
    pub fn get(&self, id: BoxId) -> &LayoutBox {
        self.boxes.get(id as usize).unwrap_or(&self.dummy)
    }
}

enum Out {
    Inline(BoxId),
    Block(BoxId),
}

enum Gen {
    Nothing,
    Contents,
    Block(Option<Display>),
    Inline(Option<Display>),
}

fn classify(d: Display) -> Gen {
    match d {
        Display::None => Gen::Nothing,
        Display::Contents => Gen::Contents,
        Display::Block | Display::ListItem | Display::FlowRoot => Gen::Block(None),
        Display::Inline => Gen::Inline(None),
        // Atomic inlines are blockified in 1b-1 (see the module comment).
        Display::InlineBlock => Gen::Block(Some(d)),
        Display::Table
        | Display::Flex
        | Display::Grid
        | Display::InlineTable
        | Display::InlineFlex
        | Display::InlineGrid
        | Display::TableRowGroup
        | Display::TableHeaderGroup
        | Display::TableFooterGroup
        | Display::TableRow
        | Display::TableColumnGroup
        | Display::TableColumn
        | Display::TableCell
        | Display::TableCaption => Gen::Block(Some(d)),
        Display::Ruby | Display::RubyText => Gen::Inline(Some(d)),
    }
}

struct Builder<'a> {
    doc: &'a Document,
    styles: &'a StyleTree,
    boxes: Vec<LayoutBox>,
    init: Rc<ComputedStyle>,
}

/// Build the box tree of a styled document.
pub fn build_box_tree(doc: &Document, styles: &StyleTree) -> BoxTree {
    let init = Rc::new(ComputedStyle::initial());
    let mut b = Builder { doc, styles, boxes: Vec::new(), init: init.clone() };
    let root_el = doc.document_element();
    let mut outs = Vec::new();
    if root_el != NONE {
        b.gen_node(root_el, &init, 0, &mut outs);
    }
    let root = match outs.as_slice() {
        [Out::Block(id)] => *id,
        _ => {
            let id = b.new_box(BoxKind::Block, NONE, b.anon_style(&init), true);
            b.finish_block(id, outs);
            id
        }
    };
    let dummy = LayoutBox {
        kind: BoxKind::Block,
        node: NONE,
        style: init,
        parent: NO_BOX,
        children: Vec::new(),
        anonymous: true,
        inline_children: false,
        fallback: None,
        split: SplitPart::default(),
    };
    BoxTree { boxes: b.boxes, root, dummy }
}

impl Builder<'_> {
    fn new_box(&mut self, kind: BoxKind, node: NodeId, style: Rc<ComputedStyle>, anonymous: bool) -> BoxId {
        let id = self.boxes.len() as BoxId;
        self.boxes.push(LayoutBox {
            kind,
            node,
            style,
            parent: NO_BOX,
            children: Vec::new(),
            anonymous,
            inline_children: false,
            fallback: None,
            split: SplitPart::default(),
        });
        id
    }

    fn box_mut(&mut self, id: BoxId) -> Option<&mut LayoutBox> {
        self.boxes.get_mut(id as usize)
    }

    /// The style of an anonymous block inside a box with style `p`: inherited properties come from
    /// the parent, everything else is initial (CSS2 9.2.1.1), `display: block`.
    fn anon_style(&self, p: &Rc<ComputedStyle>) -> Rc<ComputedStyle> {
        let mut bs = BoxStyle::initial();
        bs.display = Display::Block;
        Rc::new(ComputedStyle {
            boxs: Rc::new(bs),
            border: self.init.border.clone(),
            font: p.font.clone(),
            text: p.text.clone(),
            misc: self.init.misc.clone(),
            custom: p.custom.clone(),
        })
    }

    fn gen_children(&mut self, n: NodeId, style: &Rc<ComputedStyle>, depth: usize) -> Vec<Out> {
        let doc = self.doc;
        let mut outs = Vec::new();
        for c in doc.children(n) {
            self.gen_node(c, style, depth + 1, &mut outs);
        }
        outs
    }

    /// Make the boxes of DOM node `n`. `parent_style` is the style of the nearest element ancestor
    /// (what text inherits).
    fn gen_node(&mut self, n: NodeId, parent_style: &Rc<ComputedStyle>, depth: usize, outs: &mut Vec<Out>) {
        let doc = self.doc;
        let Some(node) = doc.node(n) else { return };
        match &node.data {
            NodeData::Text(t) => {
                if !t.is_empty() {
                    let id = self.new_box(BoxKind::Text, n, parent_style.clone(), false);
                    outs.push(Out::Inline(id));
                }
            }
            NodeData::Element(e) => {
                // SVG and MathML are not shown in layer 1.
                if e.ns != Namespace::Html {
                    return;
                }
                let Some(style) = self.styles.get(n).cloned() else { return };
                let display = style.boxs.display;
                let mut kind = classify(display);
                if depth >= MAX_BOX_DEPTH && !matches!(kind, Gen::Nothing) {
                    kind = Gen::Contents;
                }
                match kind {
                    Gen::Nothing => {}
                    Gen::Contents => {
                        for c in doc.children(n) {
                            self.gen_node(c, &style, depth + 1, outs);
                        }
                    }
                    Gen::Block(fallback) => {
                        let child_outs = self.gen_children(n, &style, depth);
                        let id = self.new_box(BoxKind::Block, n, style, false);
                        if let Some(b) = self.box_mut(id) {
                            b.fallback = fallback;
                        }
                        self.finish_block(id, child_outs);
                        outs.push(Out::Block(id));
                    }
                    Gen::Inline(fallback) => {
                        if e.name == a::BR {
                            let id = self.new_box(BoxKind::LineBreak, n, style, false);
                            outs.push(Out::Inline(id));
                            return;
                        }
                        let child_outs = self.gen_children(n, &style, depth);
                        self.finish_inline(n, style, fallback, child_outs, outs);
                    }
                }
            }
            _ => {}
        }
    }

    /// An inline element: one box, or several when block-level boxes inside split it.
    fn finish_inline(&mut self, n: NodeId, style: Rc<ComputedStyle>, fallback: Option<Display>, child_outs: Vec<Out>, outs: &mut Vec<Out>) {
        let has_block = child_outs.iter().any(|o| matches!(o, Out::Block(_)));
        if !has_block {
            let kids: Vec<BoxId> = child_outs
                .iter()
                .filter_map(|o| match o {
                    Out::Inline(b) => Some(*b),
                    Out::Block(_) => None,
                })
                .collect();
            let id = self.new_box(BoxKind::Inline, n, style, false);
            self.adopt(id, kids);
            if let Some(b) = self.box_mut(id) {
                b.fallback = fallback;
            }
            outs.push(Out::Inline(id));
            return;
        }
        // Split around the block-level children. Pieces without children are dropped; with
        // padding, border or margin on the element 1b-2 will keep them (`piece_needed`).
        let mut cur: Vec<BoxId> = Vec::new();
        let mut seen_block = false;
        let mut pieces: Vec<Out> = Vec::new();
        let mut piece_first: Vec<bool> = Vec::new();
        for o in child_outs {
            match o {
                Out::Inline(b) => cur.push(b),
                Out::Block(b) => {
                    if !cur.is_empty() {
                        let id = self.new_box(BoxKind::Inline, n, style.clone(), false);
                        self.adopt(id, std::mem::take(&mut cur));
                        pieces.push(Out::Inline(id));
                        piece_first.push(!seen_block);
                    }
                    seen_block = true;
                    pieces.push(Out::Block(b));
                    piece_first.push(false);
                }
            }
        }
        if !cur.is_empty() {
            let id = self.new_box(BoxKind::Inline, n, style, false);
            self.adopt(id, cur);
            pieces.push(Out::Inline(id));
            piece_first.push(false);
        }
        let last_inline = pieces.iter().rposition(|o| matches!(o, Out::Inline(_)));
        let last_block = pieces.iter().rposition(|o| matches!(o, Out::Block(_)));
        for (i, (p, first)) in pieces.iter().zip(piece_first).enumerate() {
            if let Out::Inline(id) = p {
                let last = last_inline == Some(i) && last_block.is_none_or(|lb| lb < i);
                if let Some(b) = self.box_mut(*id) {
                    b.fallback = fallback;
                    b.split = SplitPart { is_split: true, first, last };
                }
            }
        }
        outs.extend(pieces);
    }

    fn adopt(&mut self, parent: BoxId, kids: Vec<BoxId>) {
        for &k in &kids {
            if let Some(b) = self.box_mut(k) {
                b.parent = parent;
            }
        }
        if let Some(b) = self.box_mut(parent) {
            b.children = kids;
        }
    }

    /// Set the children of a block from the boxes its content made.
    fn finish_block(&mut self, id: BoxId, outs: Vec<Out>) {
        let has_block = outs.iter().any(|o| matches!(o, Out::Block(_)));
        if !has_block {
            let kids: Vec<BoxId> = outs
                .into_iter()
                .filter_map(|o| match o {
                    Out::Inline(b) => Some(b),
                    Out::Block(_) => None,
                })
                .collect();
            let inline = !kids.is_empty();
            self.adopt(id, kids);
            if let Some(b) = self.box_mut(id) {
                b.inline_children = inline;
            }
            return;
        }
        let style = self.boxes.get(id as usize).map(|b| b.style.clone()).unwrap_or_else(|| self.init.clone());
        let mut kids: Vec<BoxId> = Vec::new();
        let mut run: Vec<BoxId> = Vec::new();
        for o in outs {
            match o {
                Out::Inline(b) => run.push(b),
                Out::Block(b) => {
                    self.flush_run(&mut run, &mut kids, &style);
                    kids.push(b);
                }
            }
        }
        self.flush_run(&mut run, &mut kids, &style);
        self.adopt(id, kids);
    }

    /// Wrap a run of inline-level boxes in an anonymous block, unless it would be empty.
    fn flush_run(&mut self, run: &mut Vec<BoxId>, kids: &mut Vec<BoxId>, container: &Rc<ComputedStyle>) {
        if run.is_empty() {
            return;
        }
        let items = std::mem::take(run);
        if items.iter().all(|&b| self.ignorable(b)) {
            return;
        }
        let style = self.anon_style(container);
        let id = self.new_box(BoxKind::Block, NONE, style, true);
        self.adopt(id, items);
        if let Some(b) = self.box_mut(id) {
            b.inline_children = true;
        }
        kids.push(id);
    }

    /// Boxes that make nothing visible and no line: collapsible whitespace and empty inline boxes.
    /// (1b-2: an inline box with horizontal padding, border or margin is not ignorable.)
    fn ignorable(&self, id: BoxId) -> bool {
        let Some(b) = self.boxes.get(id as usize) else { return true };
        match b.kind {
            BoxKind::Text => self.doc.node(b.node).is_some_and(|n| match &n.data {
                NodeData::Text(t) => is_collapsible_whitespace(t, b.style.text.white_space_collapse),
                _ => false,
            }),
            BoxKind::Inline => b.children.iter().all(|&c| self.ignorable(c)),
            BoxKind::LineBreak | BoxKind::Block => false,
        }
    }
}

/// Document white space (space, tab, line feed, form feed, carriage return) that white-space
/// processing removes entirely when it sits between blocks.
pub fn is_collapsible_whitespace(t: &str, collapse: WhiteSpaceCollapse) -> bool {
    let ws = t.chars().all(|c| matches!(c, ' ' | '\t' | '\n' | '\r' | '\u{0C}'));
    ws && match collapse {
        WhiteSpaceCollapse::Collapse => true,
        WhiteSpaceCollapse::PreserveBreaks => !t.contains('\n'),
        _ => false,
    }
}
