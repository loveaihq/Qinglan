//! What the window asks a finished layout (L1 task 17): which link is under the pointer, where an element is on
//! the page (`#fragment`, scroll anchoring) and what the document is called. Pure functions over the DOM and the
//! layout; nothing here draws or loads.

use crate::css::types::Position;
use crate::html::atoms::{FxHashMap, a};
use crate::html::dom::{DOCUMENT, Document, NONE, NodeId};
use crate::layout::{BoxId, BoxKind, FragKind, Fragment, LayoutResult, NO_BOX, Positioned};
use crate::url::percent_decode;

/// Longest document title kept (characters).
pub const MAX_TITLE_CHARS: usize = 300;

/// Absolute positions of the boxes of one layout, built once per layout ([`PageIndex::new`], linear in the number of
/// boxes) so that hit tests and position queries do not have to walk the ancestors of every box.
pub struct PageIndex {
    /// Page coordinates (CSS px) of the border box of every box; for boxes of `position: fixed` content, viewport
    /// coordinates at scroll position 0.
    origin: Vec<(f32, f32)>,
    fixed: Vec<bool>,
    /// The blocks that own lines.
    text_blocks: Vec<BoxId>,
    /// Where each stacking context's own flow content (and each `z-index: auto` positioned box) comes in the paint
    /// order (CSS2 appendix E.2): a larger number is painted later, so it is on top.
    ranks: FxHashMap<BoxId, u32>,
}

/// Number the paint order of the stacking context `ctx` and what it contains.
fn rank_context(layout: &LayoutResult, ctx: BoxId, next: &mut u32, ranks: &mut FxHashMap<BoxId, u32>, depth: u32) {
    if depth > 300 {
        return;
    }
    let list = layout.stacked_in(ctx);
    let visit = |p: &Positioned, next: &mut u32, ranks: &mut FxHashMap<BoxId, u32>| {
        if p.auto {
            *next += 1;
            ranks.insert(p.box_id, *next);
        } else {
            rank_context(layout, p.box_id, next, ranks, depth + 1);
        }
    };
    for p in list.iter().filter(|p| p.z_index < 0) {
        visit(p, next, ranks);
    }
    *next += 1;
    ranks.insert(ctx, *next);
    for p in list.iter().filter(|p| p.z_index >= 0) {
        visit(p, next, ranks);
    }
}

/// A fragment's rectangle in page (or, when `fixed`, viewport) coordinates.
struct FragRect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    fixed: bool,
}

impl PageIndex {
    pub fn new(layout: &LayoutResult) -> PageIndex {
        let n = layout.tree.boxes.len();
        let mut origin = vec![(0.0f32, 0.0f32); n];
        let mut fixed = vec![false; n];
        let mut done = vec![false; n];
        let mut chain: Vec<usize> = Vec::new();
        for start in 0..n {
            if done.get(start) != Some(&false) {
                continue;
            }
            // Climb to an ancestor that is done (or the root), then fill in downwards.
            chain.clear();
            let mut cur = start as BoxId;
            while cur != NO_BOX && chain.len() < 100_000 {
                let i = cur as usize;
                if done.get(i) != Some(&false) {
                    break;
                }
                chain.push(i);
                cur = layout.tree.layout_parent(cur);
            }
            let (mut base, mut base_fixed) = match origin.get(cur as usize) {
                Some(&o) if cur != NO_BOX && done.get(cur as usize) == Some(&true) => (o, fixed.get(cur as usize).copied().unwrap_or(false)),
                _ => ((0.0, 0.0), false),
            };
            for &i in chain.iter().rev() {
                let o = layout.box_out(i as BoxId);
                base = (base.0 + o.x.to_f32(), base.1 + o.y.to_f32());
                base_fixed |= layout.tree.get(i as BoxId).style.boxs.position == Position::Fixed;
                if let (Some(slot), Some(f), Some(d)) = (origin.get_mut(i), fixed.get_mut(i), done.get_mut(i)) {
                    *slot = base;
                    *f = base_fixed;
                    *d = true;
                }
            }
        }
        let text_blocks = (0..n as BoxId).filter(|&b| layout.box_out(b).lines.1 > layout.box_out(b).lines.0).collect();
        let mut ranks = FxHashMap::default();
        rank_context(layout, layout.tree.root, &mut 0, &mut ranks, 0);
        PageIndex { origin, fixed, text_blocks, ranks }
    }

    /// The paint rank of the box a fragment belongs to: that of the nearest ancestor with one.
    fn rank_of(&self, layout: &LayoutResult, b: BoxId) -> u32 {
        let mut cur = b;
        for _ in 0..100_000 {
            if cur == NO_BOX {
                break;
            }
            if let Some(&r) = self.ranks.get(&cur) {
                return r;
            }
            cur = layout.tree.get(cur).parent;
        }
        0
    }

    fn origin_of(&self, b: BoxId) -> (f32, f32) {
        self.origin.get(b as usize).copied().unwrap_or((0.0, 0.0))
    }

    fn is_fixed(&self, b: BoxId) -> bool {
        self.fixed.get(b as usize).copied().unwrap_or(false)
    }

    fn frag_rect(&self, block: BoxId, f: &Fragment) -> FragRect {
        let (ox, oy) = self.origin_of(block);
        FragRect { x: ox + f.x.to_f32(), y: oy + f.y.to_f32(), w: f.w.to_f32(), h: f.h.to_f32(), fixed: self.is_fixed(block) }
    }

    /// Every fragment of the page with its rectangle: `visit(block, fragment, rect)`.
    fn for_each_fragment(&self, layout: &LayoutResult, mut visit: impl FnMut(&Fragment, FragRect)) {
        for &b in &self.text_blocks {
            let (la, lb) = layout.box_out(b).lines;
            for line in layout.lines.get(la as usize..lb as usize).unwrap_or(&[]) {
                for f in layout.fragments.get(line.frags.0 as usize..line.frags.1 as usize).unwrap_or(&[]) {
                    visit(f, self.frag_rect(b, f));
                }
            }
        }
    }

    /// The `<a href>` element under the point (`x`, `page_y`) in page coordinates (`view_y` is the same point in
    /// viewport coordinates, for `position: fixed` content), if any: text and images inside a link. Clipping by
    /// `overflow` is not considered.
    pub fn link_at(&self, doc: &Document, layout: &LayoutResult, x: f32, page_y: f32, view_y: f32) -> Option<NodeId> {
        // Later boxes are painted later, so they are on top: the hit with the highest paint rank wins, the last one
        // among equals (tree order is paint order inside a stacking context).
        let mut hit: Option<(u32, NodeId)> = None;
        self.for_each_fragment(layout, |f, r| {
            if !matches!(f.kind, FragKind::Text { .. } | FragKind::Atomic) {
                return;
            }
            let y = if r.fixed { view_y } else { page_y };
            if x >= r.x && x < r.x + r.w && y >= r.y && y < r.y + r.h
                && let Some(link) = enclosing_link(doc, layout.tree.get(f.box_id).node)
            {
                let rank = self.rank_of(layout, f.box_id);
                if hit.is_none_or(|(best, _)| rank >= best) {
                    hit = Some((rank, link));
                }
            }
        });
        hit.map(|h| h.1)
    }

    /// The page y (CSS px) where `node` starts: its box, or its first fragment (an inline element or text), or the
    /// block an empty inline element sits in. `None` when it has no box at all (`display: none`).
    pub fn node_y(&self, layout: &LayoutResult, node: NodeId) -> Option<f32> {
        let mut best: Option<f32> = None;
        let mut any_box: Option<BoxId> = None;
        let note = |best: &mut Option<f32>, y: f32| *best = Some(best.map_or(y, |b| b.min(y)));
        for (i, b) in layout.tree.boxes.iter().enumerate() {
            if b.node != node || self.is_fixed(i as BoxId) {
                continue;
            }
            any_box = any_box.or(Some(i as BoxId));
            if matches!(b.kind, BoxKind::Block | BoxKind::InlineBlock | BoxKind::Replaced) {
                note(&mut best, self.origin_of(i as BoxId).1);
            }
        }
        any_box?;
        self.for_each_fragment(layout, |f, r| {
            if !r.fixed && layout.tree.get(f.box_id).node == node {
                note(&mut best, r.y);
            }
        });
        if best.is_none() {
            // An empty inline element: the block that holds it is as close as we get.
            let p = layout.tree.layout_parent(any_box?);
            if p != NO_BOX {
                best = Some(self.origin_of(p).1);
            }
        }
        best
    }

    /// Scroll anchoring (L1 task 17): the node of the first text or image on the page that is at least partly
    /// below `scroll_y`, and how far its start is below `scroll_y`. After a relayout, scrolling to
    /// `node_y(node) - offset` keeps it where it was.
    pub fn anchor(&self, layout: &LayoutResult, scroll_y: f32) -> Option<(NodeId, f32)> {
        let mut best: Option<(f32, NodeId)> = None;
        self.for_each_fragment(layout, |f, r| {
            if r.fixed || !matches!(f.kind, FragKind::Text { .. } | FragKind::Atomic) || r.y + r.h <= scroll_y {
                return;
            }
            let node = layout.tree.get(f.box_id).node;
            if node == NONE || best.is_some_and(|(y, _)| y <= r.y) {
                return;
            }
            best = Some((r.y, node));
        });
        let (_, node) = best?;
        let y = self.node_y(layout, node)?;
        Some((node, y - scroll_y))
    }
}

/// The `<a>` element with an `href` that `node` is in (or is).
fn enclosing_link(doc: &Document, node: NodeId) -> Option<NodeId> {
    let mut cur = node;
    for _ in 0..1024 {
        if cur == NONE || cur == DOCUMENT {
            return None;
        }
        if doc.is_html_element(cur, a::A) && doc.attr(cur, a::HREF).is_some() {
            return Some(cur);
        }
        cur = doc.parent(cur);
    }
    None
}

/// The address a link element points at (not resolved) and whether it asks for a new window (`target="_blank"`).
pub fn link_target(doc: &Document, link: NodeId) -> Option<(String, bool)> {
    let href = doc.attr(link, a::HREF)?.trim().to_string();
    let blank = doc.atoms.lookup("target").and_then(|t| doc.attr(link, t)).is_some_and(|v| v.trim().eq_ignore_ascii_case("_blank"));
    Some((href, blank))
}

/// The element a `#fragment` indicates (HTML 7.4.2.3.3): the first element whose `id` is the fragment, else the first
/// `<a>` whose `name` is. The fragment is tried as written and percent-decoded. `None` for an empty fragment or when
/// nothing matches (`#top` is the caller's: it means the top of the page).
pub fn fragment_target(doc: &Document, fragment: &str) -> Option<NodeId> {
    if fragment.is_empty() {
        return None;
    }
    let decoded = String::from_utf8_lossy(&percent_decode(fragment.as_bytes())).into_owned();
    let (mut by_id, mut by_id_decoded, mut by_name, mut by_name_decoded) = (None, None, None, None);
    for n in tree_order(doc) {
        let Some(e) = doc.element(n) else { continue };
        if by_id.is_none()
            && let Some(v) = doc.attr(n, a::ID)
        {
            if v == fragment {
                by_id = Some(n);
                break;
            }
            if by_id_decoded.is_none() && v == decoded {
                by_id_decoded = Some(n);
            }
        }
        if e.name == a::A && (by_name.is_none() || by_name_decoded.is_none())
            && let Some(v) = doc.attr(n, a::NAME)
        {
            if v == fragment && by_name.is_none() {
                by_name = Some(n);
            } else if v == decoded && by_name_decoded.is_none() {
                by_name_decoded = Some(n);
            }
        }
    }
    by_id.or(by_id_decoded).or(by_name).or(by_name_decoded)
}

/// Every node in tree order (depth first, parents before children).
fn tree_order(doc: &Document) -> Vec<NodeId> {
    let mut out = Vec::new();
    let mut stack = vec![DOCUMENT];
    while let Some(n) = stack.pop() {
        out.push(n);
        let kids: Vec<NodeId> = doc.children(n).collect();
        stack.extend(kids.into_iter().rev());
    }
    out
}

/// The document title (HTML 4.2.2): the text of the first HTML `<title>`, white space collapsed, at most
/// [`MAX_TITLE_CHARS`] characters. Empty when there is none.
pub fn document_title(doc: &Document) -> String {
    for n in tree_order(doc) {
        if doc.is_html_element(n, a::TITLE) {
            let text = doc.text_content(n);
            let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
            return collapsed.chars().take(MAX_TITLE_CHARS).collect();
        }
    }
    String::new()
}
