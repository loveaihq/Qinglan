//! CSS: parsing, selectors, the cascade and computed style (layer 1: step 1a).
//!
//! Entry point: [`style_document`], which finds the style sheets of a parsed document
//! (`<style>`, `<link rel=stylesheet>`, `@import`, `style=""`, presentational attributes), loads
//! them through a [`ResourceLoader`] and computes a style for every element.

pub mod cascade;
pub mod color;
mod color_names;
pub mod computed;
pub mod import_scan;
pub mod media;
pub mod parse_prop;
pub mod presentational;
pub mod selector;
pub mod stylesheet;
pub mod syntax;
pub mod tokenizer;
pub mod types;
pub mod values;

use std::rc::Rc;
use std::time::{Duration, Instant};

use cascade::{ImportResolver, Origin, RuleSet, StyleInput, StyleTree, compute_styles};
use computed::Env;
use media::{MediaEnv, MediaQueryList};
use stylesheet::{ImportRule, SheetCtx, Stylesheet, parse_stylesheet, parse_style_attribute};
use types::DeclBlock;

use crate::html::atoms::{FxHashMap, Interner, a};
use crate::platform::Platform;
use crate::html::dom::{Document, NONE, Namespace, NodeId, QuirksMode};
use crate::url::Url;

/// Style sheets per page (L1 task sheet): `<style>`, `<link>` and `@import` together.
pub const MAX_SHEETS: usize = 64;

/// Loads the text of a style sheet.
pub trait ResourceLoader {
    /// The decoded text of the style sheet at `url`, or `None` if it cannot be had.
    fn load_stylesheet(&mut self, url: &Url) -> Option<String>;
}

/// A loader that finds nothing (documents without external style sheets).
pub struct NoResources;

impl ResourceLoader for NoResources {
    fn load_stylesheet(&mut self, _url: &Url) -> Option<String> {
        None
    }
}

#[derive(Clone, Copy, Debug)]
pub struct StyleOptions {
    pub viewport_w: f32,
    pub viewport_h: f32,
    /// Measure where the time goes (see [`StyleTimings`]); off by default because it reads the clock per element.
    pub profile: bool,
}

impl Default for StyleOptions {
    fn default() -> Self {
        StyleOptions { viewport_w: 1280.0, viewport_h: 1000.0, profile: false }
    }
}

/// What styling did.
pub struct StyleOutput {
    pub tree: StyleTree,
    /// Style sheets parsed (inline, linked and imported), not counting the built-in one.
    pub sheets: usize,
    pub author_rules: usize,
    /// Style sheets that could not be loaded.
    pub missing: Vec<String>,
    /// Time per phase; the durations are zero unless `StyleOptions::profile` was set.
    pub timings: StyleTimings,
}

/// Time spent in the phases of [`style_document`] outside the per-element cascade (which has its own
/// breakdown in `StyleTree::timings`).
#[derive(Clone, Debug, Default)]
pub struct StyleTimings {
    /// `ResourceLoader` calls for linked sheets.
    pub load_sheets: Duration,
    /// Tokenizing and parsing the style sheets (built-in one included).
    pub parse_sheets: Duration,
    /// Media queries, `@layer`, selector bucketing: `RuleSet::add_sheet`.
    pub index_rules: Duration,
    /// Parsing `style=""` attributes.
    pub style_attrs: Duration,
    /// `compute_styles` as a whole.
    pub cascade: Duration,
}

enum SourceKind {
    Inline(String),
    Link(Url),
}

struct Source {
    kind: SourceKind,
    media: String,
}

/// `<base href>` (HTML 4.2.3 "document base URL").
pub fn base_url(doc: &Document, doc_url: Option<&Url>) -> Option<Url> {
    for n in 0..doc.nodes.len() as NodeId {
        if doc.is_html_element(n, a::BASE)
            && let Some(href) = doc.attr(n, a::HREF)
        {
            return match doc_url {
                Some(u) => u.join(href).ok().or_else(|| Some(u.clone())),
                None => Url::parse(href, None).ok(),
            };
        }
    }
    doc_url.cloned()
}

fn has_token(value: &str, token: &str) -> bool {
    value.split(|c: char| c.is_ascii_whitespace()).any(|t| t.eq_ignore_ascii_case(token))
}

/// The node after `n` in tree order (NONE at the end).
fn next_in_tree(doc: &Document, n: NodeId) -> NodeId {
    let fc = doc.first_child(n);
    if fc != NONE {
        return fc;
    }
    let mut cur = n;
    loop {
        let s = doc.next_sibling(cur);
        if s != NONE {
            return s;
        }
        cur = doc.parent(cur);
        if cur == NONE || cur == crate::html::dom::DOCUMENT {
            return NONE;
        }
    }
}

/// Find the style sheets in tree order.
fn collect_sources(doc: &Document, base: Option<&Url>) -> Vec<Source> {
    let mut out = Vec::new();
    // Tree-order walk by sibling/parent links; template contents are not part of the document tree.
    let mut cur = doc.first_child(crate::html::dom::DOCUMENT);
    while cur != NONE {
        let n = cur;
        cur = next_in_tree(doc, n);
        let Some(e) = doc.element(n) else { continue };
        if e.ns != Namespace::Html {
            continue;
        }
        let attr = |name: &str| e.attrs.iter().find(|x| x.ns == Namespace::None && doc.atoms.get(x.name) == name).map(|x| x.value.as_str());
        if e.name == a::STYLE {
            if attr("type").is_some_and(|t| !t.is_empty() && !t.trim().eq_ignore_ascii_case("text/css")) {
                continue;
            }
            let mut text = String::new();
            for c in doc.children(n) {
                if let Some(crate::html::dom::NodeData::Text(t)) = doc.node(c).map(|x| &x.data) {
                    text.push_str(t);
                }
            }
            out.push(Source { kind: SourceKind::Inline(text), media: attr("media").unwrap_or("").to_string() });
        } else if e.name == a::LINK {
            let Some(rel) = attr("rel") else { continue };
            if !has_token(rel, "stylesheet") || has_token(rel, "alternate") || attr("disabled").is_some() {
                continue;
            }
            if attr("type").is_some_and(|t| !t.is_empty() && !t.trim().eq_ignore_ascii_case("text/css")) {
                continue;
            }
            let Some(href) = attr("href").map(str::trim).filter(|h| !h.is_empty()) else { continue };
            let url = match base {
                Some(b) => b.join(href).ok(),
                None => Url::parse(href, None).ok(),
            };
            if let Some(url) = url {
                out.push(Source { kind: SourceKind::Link(url), media: attr("media").unwrap_or("").to_string() });
            }
        }
    }
    out
}

struct Resolver<'a> {
    atoms: &'a mut Interner,
    loader: &'a mut dyn ResourceLoader,
    quirks: bool,
    var_id: &'a mut u32,
    block_id: &'a mut u32,
    sheets: &'a mut usize,
    missing: &'a mut Vec<String>,
    /// URLs of the sheets being processed (the chain of imports), for relative URLs and cycle detection.
    chain: Vec<Url>,
}

impl ImportResolver for Resolver<'_> {
    fn resolve(&mut self, rule: &ImportRule, _depth: usize) -> Option<Rc<Stylesheet>> {
        if *self.sheets >= MAX_SHEETS {
            return None;
        }
        let url = match self.chain.last() {
            Some(b) => b.join(&rule.url).ok()?,
            None => Url::parse(&rule.url, None).ok()?,
        };
        if self.chain.iter().any(|u| u.href() == url.href()) {
            return None; // import cycle
        }
        let Some(text) = self.loader.load_stylesheet(&url) else {
            self.missing.push(url.href());
            return None;
        };
        *self.sheets += 1;
        let mut ctx = SheetCtx { atoms: self.atoms, quirks: self.quirks, var_id: self.var_id, block_id: self.block_id };
        let sheet = parse_stylesheet(&text, &mut ctx);
        self.chain.push(url);
        Some(Rc::new(sheet))
    }

    fn leave(&mut self) {
        self.chain.pop();
    }
}

const UA_CSS: &str = include_str!("ua.css");
const UA_QUIRKS_CSS: &str = include_str!("ua_quirks.css");

/// Compute the style of every element of `doc`.
///
/// `doc_url` is the address of the page (for relative style sheet addresses); `loader` supplies the
/// text of external style sheets.
pub fn style_document(doc: &mut Document, doc_url: Option<&Url>, loader: &mut dyn ResourceLoader, opts: &StyleOptions) -> StyleOutput {
    style_document_with(doc, doc_url, loader, opts, None)
}

/// Like [`style_document`], with a platform that supplies font metrics for `ex` and `ch`
/// (css-values-4 6.1). Without one both are half an em.
pub fn style_document_with(
    doc: &mut Document,
    doc_url: Option<&Url>,
    loader: &mut dyn ResourceLoader,
    opts: &StyleOptions,
    platform: Option<&dyn Platform>,
) -> StyleOutput {
    let quirks = doc.quirks == QuirksMode::Quirks;
    let base = base_url(doc, doc_url);
    let sources = collect_sources(doc, base.as_ref());
    let media_env = MediaEnv::screen(opts.viewport_w, opts.viewport_h);
    let mut var_id = 0u32;
    let mut block_id = 0u32;
    let mut timings = StyleTimings::default();
    let mut clock = Clock::new(opts.profile);

    // Built-in style sheet.
    let mut ua = RuleSet::new();
    {
        let mut ctx = SheetCtx { atoms: &mut doc.atoms, quirks: false, var_id: &mut var_id, block_id: &mut block_id };
        let sheet = parse_stylesheet(UA_CSS, &mut ctx);
        clock.lap(&mut timings.parse_sheets);
        ua.add_sheet(&sheet, Origin::UserAgent, &media_env, &mut NoImports);
        clock.lap(&mut timings.index_rules);
        if quirks {
            let q = parse_stylesheet(UA_QUIRKS_CSS, &mut ctx);
            clock.lap(&mut timings.parse_sheets);
            ua.add_sheet(&q, Origin::UserAgent, &media_env, &mut NoImports);
            clock.lap(&mut timings.index_rules);
        }
    }

    // Author style sheets, in document order.
    let mut author = RuleSet::new();
    let mut sheets = 0usize;
    let mut missing = Vec::new();
    for src in sources {
        if sheets >= MAX_SHEETS {
            break;
        }
        let mq = {
            let cvs = syntax::parse_cvs(&src.media);
            MediaQueryList::parse(&cvs)
        };
        if !mq.matches(&media_env) {
            continue;
        }
        clock.reset();
        let (text, url) = match src.kind {
            SourceKind::Inline(t) => (t, None),
            SourceKind::Link(u) => match loader.load_stylesheet(&u) {
                Some(t) => (t, Some(u)),
                None => {
                    missing.push(u.href());
                    continue;
                }
            },
        };
        clock.lap(&mut timings.load_sheets);
        sheets += 1;
        let sheet = {
            let mut ctx = SheetCtx { atoms: &mut doc.atoms, quirks, var_id: &mut var_id, block_id: &mut block_id };
            parse_stylesheet(&text, &mut ctx)
        };
        clock.lap(&mut timings.parse_sheets);
        let chain = vec![url.or_else(|| base.clone()).unwrap_or_else(|| Url::parse("about:blank", None).unwrap_or_else(|_| empty_url()))];
        let mut resolver = Resolver {
            atoms: &mut doc.atoms,
            loader: &mut *loader,
            quirks,
            var_id: &mut var_id,
            block_id: &mut block_id,
            sheets: &mut sheets,
            missing: &mut missing,
            chain,
        };
        author.add_sheet(&sheet, Origin::Author, &media_env, &mut resolver);
        // `@import`ed sheets are loaded and parsed inside `add_sheet`; their time lands here, with the indexing.
        clock.lap(&mut timings.index_rules);
    }

    // `style` attributes (memoized by text).
    clock.reset();
    let mut attr_texts: Vec<(NodeId, String)> = Vec::new();
    for n in 0..doc.nodes.len() as NodeId {
        if let Some(e) = doc.element(n)
            && e.ns == Namespace::Html
            && let Some(v) = e.attrs.iter().find(|x| x.ns == Namespace::None && x.name == a::STYLE).map(|x| x.value.clone())
        {
            attr_texts.push((n, v));
        }
    }
    let mut style_attrs: Vec<Option<Rc<DeclBlock>>> = vec![None; doc.nodes.len()];
    let mut memo: FxHashMap<String, Rc<DeclBlock>> = FxHashMap::default();
    for (n, text) in attr_texts {
        let block = match memo.get(&text) {
            Some(b) => b.clone(),
            None => {
                let mut ctx = SheetCtx { atoms: &mut doc.atoms, quirks, var_id: &mut var_id, block_id: &mut block_id };
                let b = parse_style_attribute(&text, &mut ctx);
                memo.insert(text, b.clone());
                b
            }
        };
        if !block.decls.is_empty()
            && let Some(slot) = style_attrs.get_mut(n as usize)
        {
            *slot = Some(block);
        }
    }

    clock.lap(&mut timings.style_attrs);
    let mut hints = presentational::HintBuilder::new(doc);
    let author_rules = author.len();
    let tree = {
        let mut hint_fn = |d: &Document, el: NodeId| hints.hints_for(d, el);
        compute_styles(StyleInput {
            doc,
            ua: &ua,
            author: &author,
            style_attrs: &style_attrs,
            env: Env::new(opts.viewport_w, opts.viewport_h, platform),
            hints: &mut hint_fn,
            profile: opts.profile,
        })
    };
    clock.lap(&mut timings.cascade);
    StyleOutput { tree, sheets, author_rules, missing, timings }
}

/// A stopwatch that does nothing when profiling is off.
struct Clock(Option<Instant>);

impl Clock {
    fn new(on: bool) -> Clock {
        Clock(on.then(Instant::now))
    }

    fn lap(&mut self, acc: &mut Duration) {
        if let Some(t) = self.0 {
            let now = Instant::now();
            *acc += now - t;
            self.0 = Some(now);
        }
    }

    fn reset(&mut self) {
        if self.0.is_some() {
            self.0 = Some(Instant::now());
        }
    }
}

struct NoImports;

impl ImportResolver for NoImports {
    fn resolve(&mut self, _rule: &ImportRule, _depth: usize) -> Option<Rc<Stylesheet>> {
        None
    }
    fn leave(&mut self) {}
}

fn empty_url() -> Url {
    Url {
        scheme: "about".to_string(),
        username: String::new(),
        password: String::new(),
        host: None,
        port: None,
        path: crate::url::UrlPath::Opaque("blank".to_string()),
        query: None,
        fragment: None,
    }
}

#[allow(dead_code)]
fn _keep(_: NodeId) -> NodeId {
    NONE
}
pub mod dump;

pub use import_scan::scan_imports;

/// The addresses of the `<link rel=stylesheet>` sheets that apply at the viewport in `opts` (their
/// `media` matches), in document order and resolved against the document base URL: what the network
/// code fetches before [`style_document`] runs.
pub fn linked_stylesheets(doc: &Document, doc_url: Option<&Url>, opts: &StyleOptions) -> Vec<Url> {
    let base = base_url(doc, doc_url);
    let media_env = MediaEnv::screen(opts.viewport_w, opts.viewport_h);
    collect_sources(doc, base.as_ref())
        .into_iter()
        .filter_map(|s| match s.kind {
            SourceKind::Link(u) => {
                let mq = MediaQueryList::parse(&syntax::parse_cvs(&s.media));
                mq.matches(&media_env).then_some(u)
            }
            SourceKind::Inline(_) => None,
        })
        .take(MAX_SHEETS)
        .collect()
}
