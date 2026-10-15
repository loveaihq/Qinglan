//! The cascade: rule indexing, matching, ordering (origin, importance, `@layer`, specificity, order),
//! `var()` substitution and the computation of styles for a whole document
//! (css-cascade-5, css-variables-1).

use std::borrow::Cow;
use std::rc::Rc;

use super::computed::{
    ComputedStyle, CustomProps, Declared, Env, compute_border, compute_box, compute_font, compute_misc, compute_text,
};
use super::media::MediaEnv;
use super::parse_prop::{ParseEnv, parse_declaration};
use super::selector::{AncestorFilter, Combinator, Matcher, PseudoClass, Selector, Simple};
use super::stylesheet::{ImportRule, Rule, Stylesheet};
use super::syntax::{Cv, trim_ws};
use super::tokenizer::Token;
use super::types::*;
use super::values::LenCtx;
use crate::html::atoms::{Atom, FxHashMap, a};
use crate::html::dom::{Document, NONE, NodeId};

/// Rule count limit per page.
pub const MAX_RULES: usize = 300_000;
/// `@import` nesting limit.
pub const MAX_IMPORT_DEPTH: usize = 4;
/// Longest value a `var()` substitution may produce (L1 task sheet: 64 KB).
pub const MAX_VAR_VALUE: usize = 64 * 1024;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Origin {
    UserAgent,
    Author,
}

struct IndexedRule {
    selector: Selector,
    block: Rc<DeclBlock>,
    origin: Origin,
    layer_node: u32,
    order: u32,
}

struct LayerNode {
    name: Option<String>,
    children: Vec<u32>,
}

/// Cascade layers in order of first appearance; node 0 is the implicit outermost layer (unlayered rules).
struct LayerTree {
    nodes: Vec<LayerNode>,
}

impl LayerTree {
    fn new() -> LayerTree {
        LayerTree { nodes: vec![LayerNode { name: None, children: Vec::new() }] }
    }

    fn child(&mut self, parent: u32, name: Option<&str>) -> u32 {
        if let Some(n) = name
            && let Some(&c) = self
                .nodes
                .get(parent as usize)
                .and_then(|p| p.children.iter().find(|&&c| self.nodes.get(c as usize).is_some_and(|x| x.name.as_deref() == Some(n))))
        {
            return c;
        }
        let id = self.nodes.len() as u32;
        self.nodes.push(LayerNode { name: name.map(str::to_string), children: Vec::new() });
        if let Some(p) = self.nodes.get_mut(parent as usize) {
            p.children.push(id);
        }
        id
    }

    fn path(&mut self, base: u32, path: &[String]) -> u32 {
        let mut cur = base;
        for seg in path {
            cur = self.child(cur, Some(seg));
        }
        cur
    }

    /// Ordinals: post-order, so a layer's own rules rank above its sub-layers; the root (unlayered) is last.
    fn ordinals(&self) -> Vec<u32> {
        let mut ord = vec![0u32; self.nodes.len()];
        let mut counter = 1u32;
        // Iterative post-order.
        let mut stack: Vec<(u32, bool)> = vec![(0, false)];
        while let Some((n, visited)) = stack.pop() {
            if visited {
                if let Some(o) = ord.get_mut(n as usize) {
                    *o = counter;
                }
                counter += 1;
            } else {
                stack.push((n, true));
                if let Some(node) = self.nodes.get(n as usize) {
                    for &c in node.children.iter().rev() {
                        stack.push((c, false));
                    }
                }
            }
        }
        ord
    }
}

/// Resolves `@import` to a parsed style sheet (loading, depth/cycle/count limits are up to the resolver).
pub trait ImportResolver {
    /// Load and parse the imported sheet. When it returns `Some`, `leave` is called after the sheet's rules were added.
    fn resolve(&mut self, rule: &ImportRule, depth: usize) -> Option<Rc<Stylesheet>>;
    fn leave(&mut self);
}

/// Rules indexed by the most selective part of their rightmost compound.
#[derive(Default)]
struct Buckets {
    by_id: FxHashMap<Atom, Vec<u32>>,
    by_class: FxHashMap<Atom, Vec<u32>>,
    by_tag: FxHashMap<Atom, Vec<u32>>,
    /// Rules whose rightmost compound needs a given attribute.
    by_attr: FxHashMap<Atom, Vec<u32>>,
    /// Rules for the root element only.
    root_rules: Vec<u32>,
    universal: Vec<u32>,
}

/// An indexed set of rules. Rules that depend on the siblings or children of an element
/// (sibling combinators, `:first-child`, `:empty`, ...) are indexed apart: the matches of all the other
/// rules only depend on the element itself and its ancestors, so they can be cached.
pub struct RuleSet {
    rules: Vec<IndexedRule>,
    normal: Buckets,
    sensitive: Buckets,
    /// Attribute names tested by some selector.
    used_attrs: Vec<Atom>,
    layers: LayerTree,
    layer_ord: Vec<u32>,
    next_order: u32,
}

impl Default for RuleSet {
    fn default() -> Self {
        Self::new()
    }
}

impl RuleSet {
    pub fn new() -> RuleSet {
        RuleSet {
            rules: Vec::new(),
            normal: Buckets::default(),
            sensitive: Buckets::default(),
            used_attrs: Vec::new(),
            layers: LayerTree::new(),
            layer_ord: vec![1],
            next_order: 0,
        }
    }

    /// Attribute names whose values some selector tests (they are part of an element's matching signature).
    pub fn used_attrs(&self) -> &[Atom] {
        &self.used_attrs
    }

    pub fn len(&self) -> usize {
        self.rules.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Add the rules of a style sheet that applies in `env`. `resolver` loads `@import`ed sheets.
    pub fn add_sheet(&mut self, sheet: &Stylesheet, origin: Origin, env: &MediaEnv, resolver: &mut dyn ImportResolver) {
        self.add_rules(&sheet.rules, origin, env, 0, 0, resolver);
        self.layer_ord = self.layers.ordinals();
    }

    fn add_rules(&mut self, rules: &[Rule], origin: Origin, env: &MediaEnv, layer: u32, depth: usize, resolver: &mut dyn ImportResolver) {
        for r in rules {
            if self.rules.len() >= MAX_RULES {
                return;
            }
            match r {
                Rule::Style(sr) => {
                    for sel in &sr.selectors {
                        self.push(sel, &sr.block, origin, layer);
                    }
                }
                Rule::Media(q, inner) => {
                    if q.matches(env) {
                        self.add_rules(inner, origin, env, layer, depth, resolver);
                    }
                }
                Rule::Layer(name, inner) => {
                    let node = match name {
                        Some(path) => self.layers.path(layer, path),
                        None => self.layers.child(layer, None),
                    };
                    self.add_rules(inner, origin, env, node, depth, resolver);
                }
                Rule::LayerStatement(names) => {
                    for p in names {
                        self.layers.path(layer, p);
                    }
                }
                Rule::Import(imp) => {
                    if depth >= MAX_IMPORT_DEPTH || !imp.media.matches(env) {
                        continue;
                    }
                    let node = match &imp.layer {
                        None => layer,
                        Some(Some(path)) => self.layers.path(layer, path),
                        Some(None) => self.layers.child(layer, None),
                    };
                    if let Some(sheet) = resolver.resolve(imp, depth + 1) {
                        self.add_rules(&sheet.rules, origin, env, node, depth + 1, resolver);
                        resolver.leave();
                    }
                }
            }
        }
    }

    fn push(&mut self, sel: &Selector, block: &Rc<DeclBlock>, origin: Origin, layer: u32) {
        // Selectors with a pseudo-element never match an element's own style.
        if sel.pseudo_element.is_some() {
            return;
        }
        let idx = self.rules.len() as u32;
        let order = self.next_order;
        self.next_order += 1;
        self.rules.push(IndexedRule { selector: sel.clone(), block: block.clone(), origin, layer_node: layer, order });
        collect_used_attrs(sel, &mut self.used_attrs);
        if selector_sensitive(sel) {
            self.sensitive.add(idx, sel);
        } else {
            self.normal.add(idx, sel);
        }
    }

    /// Indices of the rules that match `el`: those that only depend on the element and its ancestors.
    fn matching_plain(&self, matcher: &mut Matcher<'_>, el: NodeId, name: Atom, is_root: bool, bloom: &AncestorFilter, out: &mut Vec<u32>) {
        self.normal.gather(&self.rules, matcher, el, name, is_root, bloom, out);
    }

    /// Indices of the matching rules that depend on siblings or children.
    fn matching_sensitive(&self, matcher: &mut Matcher<'_>, el: NodeId, name: Atom, is_root: bool, bloom: &AncestorFilter, out: &mut Vec<u32>) {
        self.sensitive.gather(&self.rules, matcher, el, name, is_root, bloom, out);
    }
}

impl Buckets {
    fn add(&mut self, idx: u32, sel: &Selector) {
        // Bucket by the most selective part of the rightmost compound.
        let Some(last) = sel.compounds.last() else { return };
        let mut id = None;
        let mut class = None;
        let mut tags: Vec<Atom> = Vec::new();
        let mut attr: Vec<Atom> = Vec::new();
        let mut is_root = false;
        for s in &last.simples {
            match s {
                Simple::Id(a) if id.is_none() => id = Some(*a),
                Simple::Class(a) if class.is_none() => class = Some(*a),
                Simple::Type { name, lower } => {
                    tags.push(*lower);
                    if name != lower {
                        tags.push(*name);
                    }
                }
                Simple::Attr(at) if attr.is_empty() => {
                    attr.push(at.lower);
                    if at.name != at.lower {
                        attr.push(at.name);
                    }
                }
                Simple::Pseudo(PseudoClass::Root) => is_root = true,
                Simple::Pseudo(PseudoClass::Link) if tags.is_empty() => {
                    // :link matches only a and area.
                    tags.push(a::A);
                    tags.push(a::AREA);
                }
                Simple::Pseudo(PseudoClass::Is(list)) if tags.is_empty() => {
                    // :is(x, y, z) with a plain type in each alternative: bucket under those types.
                    let mut found = Vec::new();
                    for alt in list {
                        let types: Vec<&Simple> = alt
                            .compounds
                            .last()
                            .map(|c| c.simples.iter().filter(|s| matches!(s, Simple::Type { .. })).collect())
                            .unwrap_or_default();
                        match types.as_slice() {
                            [Simple::Type { name, lower }] => {
                                found.push(*lower);
                                if name != lower {
                                    found.push(*name);
                                }
                            }
                            _ => {
                                found.clear();
                                break;
                            }
                        }
                    }
                    if !found.is_empty() {
                        tags = found;
                    }
                }
                _ => {}
            }
        }
        if let Some(i) = id {
            self.by_id.entry(i).or_default().push(idx);
        } else if let Some(c) = class {
            self.by_class.entry(c).or_default().push(idx);
        } else if !tags.is_empty() {
            for t in tags {
                self.by_tag.entry(t).or_default().push(idx);
            }
        } else if is_root {
            self.root_rules.push(idx);
        } else if !attr.is_empty() {
            for t in attr {
                self.by_attr.entry(t).or_default().push(idx);
            }
        } else {
            self.universal.push(idx);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn gather(&self, rules: &[IndexedRule], matcher: &mut Matcher<'_>, el: NodeId, name: Atom, is_root: bool, bloom: &AncestorFilter, out: &mut Vec<u32>) {
        let start = out.len();
        let test = |idx: u32, matcher: &mut Matcher<'_>, out: &mut Vec<u32>| {
            if let Some(r) = rules.get(idx as usize)
                && matcher.matches(&r.selector, el, Some(bloom))
            {
                out.push(idx);
            }
        };
        if let Some(v) = self.by_tag.get(&name) {
            for &i in v {
                test(i, matcher, out);
            }
        }
        if let Some(id) = matcher.id_of(el)
            && let Some(v) = self.by_id.get(&id)
        {
            for &i in v {
                test(i, matcher, out);
            }
        }
        if !self.by_class.is_empty() {
            for k in 0..matcher.class_count(el) {
                let Some(c) = matcher.class_at(el, k) else { break };
                if let Some(v) = self.by_class.get(&c) {
                    for &i in v {
                        test(i, matcher, out);
                    }
                }
            }
        }
        if !self.by_attr.is_empty() {
            for k in 0..matcher.attr_count(el) {
                let Some(n) = matcher.attr_name_at(el, k) else { break };
                if let Some(v) = self.by_attr.get(&n) {
                    for &i in v {
                        test(i, matcher, out);
                    }
                }
            }
        }
        if is_root {
            for &i in &self.root_rules {
                test(i, matcher, out);
            }
        }
        for &i in &self.universal {
            test(i, matcher, out);
        }
        // A rule can be reached through two buckets (type names of SVG elements): remove duplicates.
        if let Some(slice) = out.get_mut(start..) {
            slice.sort_unstable();
        }
        let tail: Vec<u32> = out.drain(start..).collect();
        let mut last = None;
        for i in tail {
            if last != Some(i) {
                out.push(i);
                last = Some(i);
            }
        }
    }
}

/// Does the selector depend on the siblings or children of the element?
fn selector_sensitive(sel: &Selector) -> bool {
    sel.combinators.iter().any(|c| matches!(c, Combinator::NextSibling | Combinator::SubsequentSibling))
        || sel.compounds.iter().any(|c| c.simples.iter().any(simple_sensitive))
}

fn simple_sensitive(s: &Simple) -> bool {
    match s {
        Simple::Pseudo(p) => match p {
            PseudoClass::FirstChild
            | PseudoClass::LastChild
            | PseudoClass::OnlyChild
            | PseudoClass::FirstOfType
            | PseudoClass::LastOfType
            | PseudoClass::OnlyOfType
            | PseudoClass::NthChild(..)
            | PseudoClass::NthLastChild(..)
            | PseudoClass::NthOfType(_)
            | PseudoClass::NthLastOfType(_)
            | PseudoClass::Empty
            | PseudoClass::Disabled
            | PseudoClass::Enabled => true,
            PseudoClass::Not(l) | PseudoClass::Is(l) | PseudoClass::Where(l) => l.iter().any(selector_sensitive),
            _ => false,
        },
        _ => false,
    }
}

/// Attribute names a selector reads (including inside `:not()` and `:is()`).
fn collect_used_attrs(sel: &Selector, out: &mut Vec<Atom>) {
    for c in &sel.compounds {
        for s in &c.simples {
            match s {
                Simple::Attr(a) => {
                    for n in [a.name, a.lower] {
                        if !out.contains(&n) {
                            out.push(n);
                        }
                    }
                }
                Simple::Pseudo(PseudoClass::Not(l) | PseudoClass::Is(l) | PseudoClass::Where(l)) => {
                    for x in l {
                        collect_used_attrs(x, out);
                    }
                }
                _ => {}
            }
        }
    }
}

// -----------------------------------------------------------------------------------------------
// Cascade entries

#[derive(Clone)]
struct Entry {
    /// (rank, layer key, specificity, source order): higher wins.
    key: (u8, u32, u32, u32),
    block: Rc<DeclBlock>,
    important: bool,
    origin: Origin,
}

const RANK_UA_NORMAL: u8 = 0;
const RANK_AUTHOR_NORMAL: u8 = 1;
const RANK_AUTHOR_IMPORTANT: u8 = 2;
const RANK_UA_IMPORTANT: u8 = 3;

/// Everything the style computation needs.
pub struct StyleInput<'a> {
    pub doc: &'a Document,
    pub ua: &'a RuleSet,
    pub author: &'a RuleSet,
    /// Parsed `style` attributes by node id.
    pub style_attrs: &'a [Option<Rc<DeclBlock>>],
    pub env: Env,
    /// Presentational hints per element (HTML 15.3), built by the caller on demand.
    pub hints: &'a mut dyn FnMut(&Document, NodeId) -> Option<Rc<DeclBlock>>,
}

/// The result of styling a document: a style per element node.
pub struct StyleTree {
    pub styles: Vec<Option<Rc<ComputedStyle>>>,
}

impl StyleTree {
    pub fn get(&self, id: NodeId) -> Option<&Rc<ComputedStyle>> {
        self.styles.get(id as usize).and_then(Option::as_ref)
    }
}

type ShareKey = (usize, u64);

/// Parsed results of `var()` declarations, by (declaration id, custom property map).
type VarCache = FxHashMap<(u32, usize), Option<Rc<Vec<(PropId, Specified)>>>>;

struct ShareEntry {
    parent: usize,
    ids: Vec<(u32, bool)>,
    lang: Option<Rc<str>>,
    style: Rc<ComputedStyle>,
}

/// The rules (UA and author) that matched an element and only depend on the element and its ancestors.
struct PlainMatches {
    ua: Vec<u32>,
    author: Vec<u32>,
}

/// Cache entries kept at most (memory guard for huge documents).
const MAX_MATCH_CACHE: usize = 200_000;

fn mix(h: u64, v: u64) -> u64 {
    (h.rotate_left(5) ^ v).wrapping_mul(0x517c_c1b7_2722_0a95)
}

pub fn compute_styles(input: StyleInput<'_>) -> StyleTree {
    let StyleInput { doc, ua, author, style_attrs, env, hints } = input;
    let n = doc.nodes.len();
    let mut styles: Vec<Option<Rc<ComputedStyle>>> = vec![None; n];
    let mut matcher = Matcher::new(doc);
    let mut ancestors = AncestorFilter::new();
    let init = Rc::new(ComputedStyle::initial());
    let mut share: FxHashMap<ShareKey, Vec<ShareEntry>> = FxHashMap::default();
    let mut var_cache: VarCache = FxHashMap::default();
    let mut root_font_size = 16.0f32;
    let mut matched: Vec<u32> = Vec::new();
    let mut entries: Vec<Entry> = Vec::new();
    // Attribute names that selectors test: their values are part of an element's matching signature.
    let mut used_attrs: Vec<Atom> = ua.used_attrs().to_vec();
    for a in author.used_attrs() {
        if !used_attrs.contains(a) {
            used_attrs.push(*a);
        }
    }
    for implicit in ["href", "type", "checked", "selected", "disabled", "lang", "dir"] {
        if let Some(at) = doc.atoms.lookup(implicit)
            && !used_attrs.contains(&at)
        {
            used_attrs.push(at);
        }
    }
    // Matches of the rules that do not look at siblings, by (ancestor context, element signature).
    let mut match_cache: FxHashMap<(u64, u64), Rc<PlainMatches>> = FxHashMap::default();
    // Depth-first walk over the elements without recursion: one frame per open element.
    struct Frame {
        style: Rc<ComputedStyle>,
        /// The next child to visit (NONE when done).
        next: NodeId,
        /// Start of this element's Bloom keys in `key_stack`.
        keys_start: usize,
        /// Hash of the signatures of this element and all its ancestors.
        ctx: u64,
    }
    let mut frames: Vec<Frame> = Vec::new();
    let mut key_stack: Vec<u32> = Vec::new();
    // The Document node acts as the first frame (it has the initial style as its "style").
    frames.push(Frame { style: init.clone(), next: doc.first_child(crate::html::dom::DOCUMENT), keys_start: 0, ctx: 0 });
    while let Some(top) = frames.last_mut() {
        // Next element child of the top frame.
        let mut el = top.next;
        while el != NONE && doc.element(el).is_none() {
            el = doc.next_sibling(el);
        }
        if el == NONE {
            if let Some(f) = frames.pop() {
                for k in key_stack.drain(f.keys_start..) {
                    ancestors.remove(k);
                }
            }
            continue;
        }
        top.next = doc.next_sibling(el);
        let parent_style = top.style.clone();
        let parent_ctx = top.ctx;
        let Some(elem) = doc.element(el) else { continue };
        let is_root = frames.len() == 1;
        let name = elem.name;
        entries.clear();

        // The signature of the element: everything the rules that ignore siblings can look at.
        let mut sig = mix(0, elem.ns as u64 + 1);
        sig = mix(sig, name.0 as u64);
        sig = mix(sig, is_root as u64);
        if let Some(id) = matcher.id_of(el) {
            sig = mix(sig, 0x1_0000_0000 | id.0 as u64);
        }
        for k in 0..matcher.class_count(el) {
            if let Some(c) = matcher.class_at(el, k) {
                sig = mix(sig, 0x2_0000_0000 | c.0 as u64);
            }
        }
        for at in &elem.attrs {
            if at.ns == crate::html::dom::Namespace::None && used_attrs.contains(&at.name) {
                sig = mix(sig, 0x3_0000_0000 | at.name.0 as u64);
                for b in at.value.bytes() {
                    sig = mix(sig, b as u64);
                }
            }
        }
        let key = (parent_ctx, sig);
        let plain = match match_cache.get(&key) {
            Some(p) => p.clone(),
            None => {
                matched.clear();
                ua.matching_plain(&mut matcher, el, name, is_root, &ancestors, &mut matched);
                let ua_list = matched.clone();
                matched.clear();
                author.matching_plain(&mut matcher, el, name, is_root, &ancestors, &mut matched);
                let p = Rc::new(PlainMatches { ua: ua_list, author: matched.clone() });
                if match_cache.len() < MAX_MATCH_CACHE {
                    match_cache.insert(key, p.clone());
                }
                p
            }
        };
        for &i in &plain.ua {
            if let Some(r) = ua.rules.get(i as usize) {
                push_entries(&mut entries, r, ua);
            }
        }
        for &i in &plain.author {
            if let Some(r) = author.rules.get(i as usize) {
                push_entries(&mut entries, r, author);
            }
        }
        // Rules that look at siblings or children are matched for every element.
        matched.clear();
        ua.matching_sensitive(&mut matcher, el, name, is_root, &ancestors, &mut matched);
        for &i in &matched {
            if let Some(r) = ua.rules.get(i as usize) {
                push_entries(&mut entries, r, ua);
            }
        }
        matched.clear();
        author.matching_sensitive(&mut matcher, el, name, is_root, &ancestors, &mut matched);
        for &i in &matched {
            if let Some(r) = author.rules.get(i as usize) {
                push_entries(&mut entries, r, author);
            }
        }
        if let Some(h) = hints(doc, el) {
            entries.push(Entry { key: (RANK_AUTHOR_NORMAL, 0, 0, 0), block: h, important: false, origin: Origin::Author });
        }
        if let Some(Some(sa)) = style_attrs.get(el as usize) {
            entries.push(Entry {
                key: (RANK_AUTHOR_NORMAL, u32::MAX, u32::MAX, u32::MAX),
                block: sa.clone(),
                important: false,
                origin: Origin::Author,
            });
            entries.push(Entry {
                key: (RANK_AUTHOR_IMPORTANT, 0, u32::MAX, u32::MAX),
                block: sa.clone(),
                important: true,
                origin: Origin::Author,
            });
        }
        entries.sort_by_key(|e| e.key);
        let lang = doc.attr(el, a::LANG).filter(|l| !l.is_empty()).map(Rc::<str>::from);
        // Style sharing: same parent style, same declarations in the same order, same language.
        let parent_ptr = Rc::as_ptr(&parent_style) as usize;
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for e in &entries {
            h = (h ^ (((e.block.id as u64) << 1) | e.important as u64)).wrapping_mul(0x100_0000_01b3);
        }
        if let Some(l) = &lang {
            for b in l.bytes() {
                h = (h ^ b as u64).wrapping_mul(0x100_0000_01b3);
            }
        }
        let share_key = (parent_ptr, h);
        let mut hit = None;
        if let Some(list) = share.get(&share_key) {
            for s in list {
                if s.parent == parent_ptr
                    && s.lang == lang
                    && s.ids.len() == entries.len()
                    && s.ids.iter().zip(entries.iter()).all(|(x, e)| x.0 == e.block.id && x.1 == e.important)
                {
                    hit = Some(s.style.clone());
                    break;
                }
            }
        }
        let style = match hit {
            Some(s) => s,
            None => {
                let s = Rc::new(style_for(
                    doc,
                    &entries,
                    &parent_style,
                    &init,
                    &env,
                    root_font_size,
                    lang.as_deref(),
                    is_root,
                    &mut var_cache,
                ));
                share.entry(share_key).or_default().push(ShareEntry {
                    parent: parent_ptr,
                    ids: entries.iter().map(|e| (e.block.id, e.important)).collect(),
                    lang: lang.clone(),
                    style: s.clone(),
                });
                s
            }
        };
        if is_root {
            root_font_size = style.font.size;
        }
        if let Some(slot) = styles.get_mut(el as usize) {
            *slot = Some(style.clone());
        }
        // Descend (template contents are a separate tree and are not styled).
        let keys_start = key_stack.len();
        matcher.push_bloom_keys(el, &mut key_stack);
        for &k in key_stack.get(keys_start..).unwrap_or(&[]) {
            ancestors.add(k);
        }
        let first = if doc.is_template(el) { NONE } else { doc.first_child(el) };
        frames.push(Frame { style, next: first, keys_start, ctx: mix(parent_ctx, sig) });
    }
    StyleTree { styles }
}

fn push_entries(entries: &mut Vec<Entry>, r: &IndexedRule, set: &RuleSet) {
    let layer_ord = set.layer_ord.get(r.layer_node as usize).copied().unwrap_or(1);
    let root_ord = set.layer_ord.first().copied().unwrap_or(1);
    let _ = root_ord;
    // Importance flags of the block: avoid creating entries that carry no declarations.
    let has_normal = r.block.decls.iter().any(|d| !d.important);
    let has_important = r.block.decls.iter().any(|d| d.important);
    let top = set.layer_ord.first().copied().unwrap_or(1); // ordinal of the unlayered "layer"
    match r.origin {
        Origin::UserAgent => {
            if has_normal {
                entries.push(Entry {
                    key: (RANK_UA_NORMAL, 0, r.selector.specificity, r.order),
                    block: r.block.clone(),
                    important: false,
                    origin: r.origin,
                });
            }
            if has_important {
                entries.push(Entry {
                    key: (RANK_UA_IMPORTANT, 0, r.selector.specificity, r.order),
                    block: r.block.clone(),
                    important: true,
                    origin: r.origin,
                });
            }
        }
        Origin::Author => {
            if has_normal {
                entries.push(Entry {
                    key: (RANK_AUTHOR_NORMAL, layer_ord, r.selector.specificity, r.order),
                    block: r.block.clone(),
                    important: false,
                    origin: r.origin,
                });
            }
            if has_important {
                // Important declarations: earlier layers win, unlayered ones are weakest.
                entries.push(Entry {
                    key: (RANK_AUTHOR_IMPORTANT, top.saturating_sub(layer_ord), r.selector.specificity, r.order),
                    block: r.block.clone(),
                    important: true,
                    origin: r.origin,
                });
            }
        }
    }
}

// -----------------------------------------------------------------------------------------------
// Computing one element's style

#[allow(clippy::too_many_arguments)]
fn style_for(
    doc: &Document,
    entries: &[Entry],
    parent: &Rc<ComputedStyle>,
    init: &Rc<ComputedStyle>,
    env: &Env,
    root_font_size: f32,
    lang: Option<&str>,
    is_root: bool,
    var_cache: &mut VarCache,
) -> ComputedStyle {
    let mut declared = Declared::new();
    let mut custom_decls: Vec<(Atom, Rc<Vec<Cv>>)> = Vec::new();
    for (i, e) in entries.iter().enumerate() {
        for d in &e.block.decls {
            if d.important != e.important {
                continue;
            }
            match &d.prop {
                DeclProp::Known(p) => match &d.value {
                    Specified::Revert | Specified::RevertLayer => {
                        let replacement = revert_value(entries, i, *p, d.value == Specified::Revert, e.origin);
                        match replacement {
                            Some(v) => declared.set(*p, Cow::Owned(v)),
                            None => declared.set(*p, Cow::Owned(Specified::Unset)),
                        }
                    }
                    v => declared.set(*p, Cow::Borrowed(v)),
                },
                DeclProp::Custom(name, toks) => {
                    custom_decls.push((*name, toks.clone()));
                }
            }
        }
    }
    // Custom properties first: var() in other properties reads the final map.
    let custom = resolve_custom_props(doc, &custom_decls, &parent.custom);
    // Resolve the winners that contain var().
    let custom_ptr = Rc::as_ptr(&custom) as usize;
    for p in ALL_PROPS {
        if declared.mask & p.bit() == 0 {
            continue;
        }
        let Some(Specified::Var(vd)) = declared.get(p) else { continue };
        let vd = vd.clone();
        let resolved = var_cache
            .entry((vd.id, custom_ptr))
            .or_insert_with(|| resolve_var_decl(doc, &vd, &custom))
            .clone();
        let found = resolved.and_then(|list| list.iter().find(|(q, _)| *q == p).map(|(_, s)| s.clone()));
        // Invalid at computed-value time: behaves as `unset`.
        declared.set(p, Cow::Owned(found.unwrap_or(Specified::Unset)));
    }

    let parent_ctx = LenCtx { font_size: parent.font.size, root_font_size, viewport_w: env.viewport_w, viewport_h: env.viewport_h };
    let font = compute_font(&parent.font, &declared, env, root_font_size);
    let own = LenCtx { font_size: font.size, ..parent_ctx };
    let _ = is_root;
    let boxs = compute_box(&parent.boxs, &init.boxs, &declared, &own, is_root);
    let border = compute_border(&parent.border, &init.border, &declared, &own);
    let text = compute_text(&parent.text, &declared, &own, lang);
    let misc = compute_misc(&parent.misc, &init.misc, &declared, &own);
    ComputedStyle { boxs, border, font, text, misc, custom }
}

/// The value `revert` / `revert-layer` rolls back to for property `p`, looking at `entries[..i]`.
fn revert_value(entries: &[Entry], i: usize, p: PropId, whole_origin: bool, origin: Origin) -> Option<Specified> {
    let here = entries.get(i)?;
    let candidates = entries.get(..i)?;
    for e in candidates.iter().rev() {
        let eligible = if whole_origin {
            // `revert`: only the user agent origin is below the author origin.
            origin == Origin::Author && e.origin == Origin::UserAgent
        } else {
            // `revert-layer`: anything that sorts below the current layer.
            (e.key.0, e.key.1) != (here.key.0, here.key.1)
        };
        if !eligible {
            continue;
        }
        for d in e.block.decls.iter().rev() {
            if d.important == e.important && d.prop == DeclProp::Known(p) {
                return match &d.value {
                    Specified::Revert | Specified::RevertLayer => None,
                    v => Some(v.clone()),
                };
            }
        }
    }
    None
}

// -----------------------------------------------------------------------------------------------
// var()

fn size_of_cv(c: &Cv) -> usize {
    match c {
        Cv::Tok(t) => match t {
            Token::Ident(s) | Token::Function(s) | Token::AtKeyword(s) | Token::Str(s) | Token::Url(s) => s.len() + 1,
            Token::Hash { value, .. } => value.len() + 1,
            Token::Dimension { unit, .. } => unit.len() + 8,
            _ => 4,
        },
        Cv::Func(n, items) => n.len() + 2 + items.iter().map(size_of_cv).sum::<usize>(),
        Cv::Block(_, items) => 2 + items.iter().map(size_of_cv).sum::<usize>(),
    }
}

/// Substitute `var()` references in `tokens`. `lookup` gives the value of a custom property.
/// `None` if a reference has no value and no fallback, or the result is too long.
fn substitute(
    doc: &Document,
    tokens: &[Cv],
    lookup: &mut dyn FnMut(Atom) -> Option<Rc<Vec<Cv>>>,
    budget: &mut usize,
    depth: usize,
) -> Option<Vec<Cv>> {
    if depth > 32 {
        return None;
    }
    let mut out = Vec::with_capacity(tokens.len());
    for cv in tokens {
        match cv {
            Cv::Func(name, args) if name.eq_ignore_ascii_case("var") || name.eq_ignore_ascii_case("env") => {
                // var( --name [, fallback]? )
                let first = args.iter().position(|c| !c.is_ws())?;
                let Some(Cv::Tok(Token::Ident(prop))) = args.get(first) else { return None };
                if !prop.starts_with("--") && name.eq_ignore_ascii_case("var") {
                    return None;
                }
                let rest = args.get(first + 1..).unwrap_or(&[]);
                let fallback: Option<&[Cv]> = match rest.iter().position(Cv::is_comma) {
                    Some(c) => rest.get(c + 1..),
                    None => None,
                };
                let value = if name.eq_ignore_ascii_case("var") { doc.atoms.lookup(prop).and_then(&mut *lookup) } else { None };
                match value {
                    Some(v) => {
                        let sz: usize = v.iter().map(size_of_cv).sum();
                        if sz > *budget {
                            return None;
                        }
                        *budget -= sz;
                        // The value itself may not contain var() any more (it was substituted when it was resolved).
                        out.extend(v.iter().cloned());
                    }
                    None => {
                        let fb = fallback?;
                        out.extend(substitute(doc, trim_ws(fb), lookup, budget, depth + 1)?);
                    }
                }
            }
            Cv::Func(name, args) => {
                out.push(Cv::Func(name.clone(), substitute(doc, args, lookup, budget, depth + 1)?));
            }
            Cv::Block(k, items) => out.push(Cv::Block(*k, substitute(doc, items, lookup, budget, depth + 1)?)),
            other => {
                let sz = size_of_cv(other);
                if sz > *budget {
                    return None;
                }
                *budget -= sz;
                out.push(other.clone());
            }
        }
    }
    Some(out)
}

fn resolve_var_decl(doc: &Document, vd: &VarDecl, custom: &Rc<CustomProps>) -> Option<Rc<Vec<(PropId, Specified)>>> {
    let mut budget = MAX_VAR_VALUE;
    let toks = substitute(doc, &vd.tokens, &mut |n| custom.get(&n).cloned(), &mut budget, 0)?;
    let mut id = 0u32;
    let parsed = parse_declaration(&vd.prop, &toks, &ParseEnv { quirks: doc.quirks == crate::html::dom::QuirksMode::Quirks }, &mut id)?;
    Some(Rc::new(parsed))
}

#[derive(Clone)]
enum CState {
    Resolving,
    Done(Option<Rc<Vec<Cv>>>),
}

fn is_css_wide(tokens: &[Cv]) -> Option<&'static str> {
    let t = trim_ws(tokens);
    if let [Cv::Tok(Token::Ident(i))] = t {
        return match i.to_ascii_lowercase().as_str() {
            "inherit" => Some("inherit"),
            "initial" => Some("initial"),
            "unset" => Some("unset"),
            "revert" | "revert-layer" => Some("unset"),
            _ => None,
        };
    }
    None
}

/// The element's custom property map: the parent's, overridden by its own declarations (var() substituted,
/// cycles and over-long values made invalid).
fn resolve_custom_props(doc: &Document, decls: &[(Atom, Rc<Vec<Cv>>)], parent: &Rc<CustomProps>) -> Rc<CustomProps> {
    if decls.is_empty() {
        return parent.clone();
    }
    let mut pending: FxHashMap<Atom, Rc<Vec<Cv>>> = FxHashMap::default();
    let mut order: Vec<Atom> = Vec::new();
    for (n, t) in decls {
        if pending.insert(*n, t.clone()).is_none() {
            order.push(*n);
        }
    }
    let mut state: FxHashMap<Atom, CState> = FxHashMap::default();
    let mut invalid: Vec<Atom> = Vec::new();
    let mut stack: Vec<Atom> = Vec::new();
    for n in &order {
        resolve_one(doc, *n, &pending, parent, &mut state, &mut invalid, &mut stack);
    }
    let mut map: CustomProps = (**parent).clone();
    for n in &order {
        match state.get(n) {
            Some(CState::Done(Some(v))) => {
                map.insert(*n, v.clone());
            }
            _ => {
                map.remove(n);
            }
        }
    }
    Rc::new(map)
}

fn resolve_one(
    doc: &Document,
    name: Atom,
    pending: &FxHashMap<Atom, Rc<Vec<Cv>>>,
    parent: &Rc<CustomProps>,
    state: &mut FxHashMap<Atom, CState>,
    invalid: &mut Vec<Atom>,
    stack: &mut Vec<Atom>,
) -> Option<Rc<Vec<Cv>>> {
    match state.get(&name) {
        Some(CState::Done(v)) => return v.clone(),
        Some(CState::Resolving) => {
            // A cycle: every property on the stack from `name` upwards is invalid.
            if let Some(pos) = stack.iter().position(|n| *n == name) {
                invalid.extend(stack.iter().skip(pos).copied());
            }
            return None;
        }
        None => {}
    }
    let Some(tokens) = pending.get(&name) else { return parent.get(&name).cloned() };
    if stack.len() > 64 {
        state.insert(name, CState::Done(None));
        return None;
    }
    state.insert(name, CState::Resolving);
    stack.push(name);
    let result: Option<Rc<Vec<Cv>>> = match is_css_wide(tokens) {
        Some("inherit") | Some("unset") => parent.get(&name).cloned(),
        Some(_) => None,
        None => {
            if !super::parse_prop::contains_var(tokens) {
                Some(Rc::new(trim_ws(tokens).to_vec()))
            } else {
                let mut budget = MAX_VAR_VALUE;
                let r = substitute(
                    doc,
                    tokens,
                    &mut |n| {
                        if pending.contains_key(&n) {
                            resolve_one(doc, n, pending, parent, state, invalid, stack)
                        } else {
                            parent.get(&n).cloned()
                        }
                    },
                    &mut budget,
                    0,
                );
                r.map(|v| Rc::new(trim_ws(&v).to_vec()))
            }
        }
    };
    stack.pop();
    let result = if invalid.contains(&name) { None } else { result };
    state.insert(name, CState::Done(result.clone()));
    result
}

