//! HTML tree construction: WHATWG HTML 13.2.6 "Tree construction" (and 13.4 fragment parsing).
//!
//! Scripting is off: `<noscript>` content is parsed as HTML, scripts never run. Declarative shadow
//! DOM (`shadowrootmode`) and template content patching are not implemented (decisions.md).
//! Limits: [`MAX_NODES`] nodes per page (the rest of the page is dropped, `Document::too_large`) and
//! open elements are not nested deeper than [`MAX_DEPTH`] (deeper elements become children of the
//! element at that depth).

use super::atoms::{Atom, Interner, a};
use super::dom::{
    Attrs, Document, ElementData, StoredAttr, MAX_DEPTH, MAX_NODES, NONE, NodeData, NodeId, Namespace, QuirksMode, DOCUMENT,
};
use super::foreign_data::{SVG_ATTRS, SVG_TAGS};
use super::quirks_data as q;
use super::tokenizer::{DoctypeToken, State, TagToken, Token, Tokenizer};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Mode {
    Initial,
    BeforeHtml,
    BeforeHead,
    InHead,
    InHeadNoscript,
    AfterHead,
    InBody,
    Text,
    InTable,
    InTableText,
    InCaption,
    InColumnGroup,
    InTableBody,
    InRow,
    InCell,
    InTemplate,
    AfterBody,
    InFrameset,
    AfterFrameset,
    AfterAfterBody,
    AfterAfterFrameset,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Scope {
    Default,
    ListItem,
    Button,
    Table,
}

/// An entry of the list of active formatting elements (13.2.4.3).
///
/// The spec keeps the start tag token with each entry; here the entry has the element and its name, and the
/// attributes are read from the element when a token is needed (the element's attributes never change:
/// only `html` and `body` get attributes merged in, and they are not formatting elements). That saves
/// copying the attributes of every `<a>`, `<b>`, `<i>` ...
#[derive(Clone)]
enum Afe {
    Marker,
    /// Position holder used only inside the adoption agency algorithm.
    Bookmark,
    Entry { node: NodeId, name: Atom },
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Chunk {
    Whitespace,
    Nul,
    Other,
}

fn is_ws_char(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\x0c' | '\r' | ' ')
}

fn chunk_kind(c: char) -> Chunk {
    if c == '\0' {
        Chunk::Nul
    } else if is_ws_char(c) {
        Chunk::Whitespace
    } else {
        Chunk::Other
    }
}

fn is_ws_str(s: &str) -> bool {
    s.bytes().all(|b| matches!(b, b'\t' | b'\n' | 0x0c | b'\r' | b' '))
}

pub struct TreeBuilder {
    pub doc: Document,
    tok: Tokenizer,
    mode: Mode,
    orig_mode: Mode,
    tmpl_modes: Vec<Mode>,
    /// Set when the end of a template asks for the EOF token to be processed again in the new mode.
    reprocess_eof: bool,
    open: Vec<NodeId>,
    afe: Vec<Afe>,
    head: NodeId,
    form: NodeId,
    frameset_ok: bool,
    foster: bool,
    pending: Vec<String>,
    /// Fragment case: the context element (13.4), else NONE.
    ctx: NodeId,
    skip_lf: bool,
    done: bool,
}

/// Parse a complete document (13.2).
pub fn parse_document(input: String) -> Document {
    let mut tb = TreeBuilder::new(input);
    tb.run();
    tb.doc.nodes.shrink_to_fit();
    tb.doc
}

/// Parse a fragment (13.4) in the context of an element `context_name` in namespace `ns`.
/// Returns the document and the node whose children are the parsed fragment.
pub fn parse_fragment(input: String, context_name: &str, ns: Namespace) -> (Document, NodeId) {
    let mut tb = TreeBuilder::new(input);
    let name = tb.doc.atoms.intern(context_name);
    let ctx = tb.doc.alloc(NodeData::Element(ElementData { name, ns, attrs: Vec::new(), template_contents: NONE }));
    if ns == Namespace::Html && name == a::TEMPLATE {
        let c = tb.doc.alloc(NodeData::Fragment);
        if let Some(e) = tb.doc.element_mut(ctx) {
            e.template_contents = c;
        }
    }
    tb.ctx = ctx;
    if ns == Namespace::Html {
        let state = match name {
            a::TITLE | a::TEXTAREA => Some(State::RcData),
            a::STYLE | a::XMP | a::IFRAME | a::NOEMBED | a::NOFRAMES => Some(State::RawText),
            a::SCRIPT => Some(State::ScriptData),
            a::PLAINTEXT => Some(State::PlainText),
            _ => None,
        };
        if let Some(s) = state {
            tb.tok.set_state(s);
        }
    }
    let root = tb.doc.alloc(NodeData::Element(ElementData {
        name: a::HTML,
        ns: Namespace::Html,
        attrs: Vec::new(),
        template_contents: NONE,
    }));
    tb.doc.append_child(DOCUMENT, root);
    tb.open.push(root);
    if ns == Namespace::Html && name == a::TEMPLATE {
        tb.tmpl_modes.push(Mode::InTemplate);
    }
    tb.reset_insertion_mode();
    // Form element pointer: the context element itself, if it is a form.
    if ns == Namespace::Html && name == a::FORM {
        tb.form = ctx;
    }
    tb.run();
    (tb.doc, root)
}

impl TreeBuilder {
    fn new(input: String) -> TreeBuilder {
        let mut doc = Document::new(Interner::new());
        // Rough guess of the node count (about one node per 40 bytes of markup).
        doc.nodes.reserve((input.len() / 40).min(MAX_NODES));
        TreeBuilder {
            doc,
            tok: Tokenizer::new(input),
            mode: Mode::Initial,
            orig_mode: Mode::Initial,
            tmpl_modes: Vec::new(),
            reprocess_eof: false,
            open: Vec::new(),
            afe: Vec::new(),
            head: NONE,
            form: NONE,
            frameset_ok: true,
            foster: false,
            pending: Vec::new(),
            ctx: NONE,
            skip_lf: false,
            done: false,
        }
    }

    fn run(&mut self) {
        loop {
            self.tok.allow_cdata = {
                let adj = self.adjusted_current();
                self.doc.element(adj).is_some_and(|e| e.ns != Namespace::Html)
            };
            let t = self.tok.next_token(&mut self.doc.atoms);
            self.dispatch(t);
            if self.done {
                break;
            }
            if self.doc.nodes.len() >= MAX_NODES {
                self.doc.too_large = true;
                break;
            }
        }
    }

    // ------------------------------------------------------------------------------------------
    // Dispatcher (13.2.6)

    fn dispatch(&mut self, tok: Token) {
        let skip = std::mem::take(&mut self.skip_lf);
        match tok {
            Token::Text(mut s) => {
                if skip && s.starts_with('\n') {
                    s.remove(0);
                    if s.is_empty() {
                        return;
                    }
                }
                // The tree builder treats characters one by one; runs of characters that behave alike are
                // handed over together. Always split off leading white space and NUL characters; split
                // all white space runs only in the frameset modes, which keep white space but drop text.
                let fine = matches!(self.mode, Mode::InFrameset | Mode::AfterFrameset | Mode::AfterAfterFrameset);
                let lead = s.len() - s.trim_start_matches(is_ws_char).len();
                if !fine && !s.contains('\0') && (lead == 0 || lead == s.len() || matches!(self.mode, Mode::InBody)) {
                    self.dispatch_one(Token::Text(s));
                    return;
                }
                let mut pieces: Vec<&str> = Vec::new();
                let mut rest: &str = &s;
                if lead > 0 {
                    pieces.push(s.get(..lead).unwrap_or(""));
                    rest = s.get(lead..).unwrap_or("");
                }
                let mut start = 0usize;
                let mut prev: Option<Chunk> = None;
                for (i, c) in rest.char_indices() {
                    let k = if fine { chunk_kind(c) } else if c == '\0' { Chunk::Nul } else { Chunk::Other };
                    if let Some(p) = prev
                        && p != k
                    {
                        pieces.push(rest.get(start..i).unwrap_or(""));
                        start = i;
                    }
                    prev = Some(k);
                }
                if start < rest.len() {
                    pieces.push(rest.get(start..).unwrap_or(""));
                }
                let owned: Vec<String> = pieces.into_iter().map(str::to_string).collect();
                for c in owned {
                    if self.done {
                        break;
                    }
                    self.dispatch_one(Token::Text(c));
                }
            }
            other => self.dispatch_one(other),
        }
    }

    fn dispatch_one(&mut self, tok: Token) {
        if self.use_foreign(&tok) {
            self.foreign(tok);
        } else if matches!(tok, Token::Eof) {
            self.step(self.mode, Token::Eof);
            // "Reprocess the token" after the end of a template (see `m_in_template`), once per nested template.
            while std::mem::take(&mut self.reprocess_eof) && !self.done {
                self.step(self.mode, Token::Eof);
            }
        } else {
            self.step(self.mode, tok);
        }
    }

    fn use_foreign(&self, tok: &Token) -> bool {
        if self.open.is_empty() || matches!(tok, Token::Eof) {
            return false;
        }
        let adj = self.adjusted_current();
        let Some(e) = self.doc.element(adj) else { return false };
        if e.ns == Namespace::Html {
            return false;
        }
        let is_start = matches!(tok, Token::StartTag(_));
        let is_text = matches!(tok, Token::Text(_));
        if self.is_mathml_text_ip(adj) {
            if let Token::StartTag(t) = tok
                && t.name != a::MGLYPH
                && t.name != a::MALIGNMARK
            {
                return false;
            }
            if is_text {
                return false;
            }
        }
        if e.ns == Namespace::MathMl
            && e.name == a::ANNOTATION_XML
            && let Token::StartTag(t) = tok
            && t.name == a::SVG
        {
            return false;
        }
        if self.is_html_ip(adj) && (is_start || is_text) {
            return false;
        }
        true
    }

    fn is_mathml_text_ip(&self, id: NodeId) -> bool {
        self.doc
            .element(id)
            .is_some_and(|e| e.ns == Namespace::MathMl && matches!(e.name, a::MI | a::MO | a::MN | a::MS | a::MTEXT))
    }

    fn is_html_ip(&self, id: NodeId) -> bool {
        let Some(e) = self.doc.element(id) else { return false };
        match e.ns {
            Namespace::MathMl if e.name == a::ANNOTATION_XML => e.attrs.iter().any(|at| {
                at.ns == Namespace::None
                    && at.name == a::ENCODING
                    && (self.doc.value(at).eq_ignore_ascii_case("text/html") || self.doc.value(at).eq_ignore_ascii_case("application/xhtml+xml"))
            }),
            Namespace::Svg => matches!(e.name, a::FOREIGNOBJECT | a::DESC | a::TITLE),
            _ => false,
        }
    }

    fn step(&mut self, mode: Mode, tok: Token) {
        match mode {
            Mode::Initial => self.m_initial(tok),
            Mode::BeforeHtml => self.m_before_html(tok),
            Mode::BeforeHead => self.m_before_head(tok),
            Mode::InHead => self.m_in_head(tok),
            Mode::InHeadNoscript => self.m_in_head_noscript(tok),
            Mode::AfterHead => self.m_after_head(tok),
            Mode::InBody => self.m_in_body(tok),
            Mode::Text => self.m_text(tok),
            Mode::InTable => self.m_in_table(tok),
            Mode::InTableText => self.m_in_table_text(tok),
            Mode::InCaption => self.m_in_caption(tok),
            Mode::InColumnGroup => self.m_in_column_group(tok),
            Mode::InTableBody => self.m_in_table_body(tok),
            Mode::InRow => self.m_in_row(tok),
            Mode::InCell => self.m_in_cell(tok),
            Mode::InTemplate => self.m_in_template(tok),
            Mode::AfterBody => self.m_after_body(tok),
            Mode::InFrameset => self.m_in_frameset(tok),
            Mode::AfterFrameset => self.m_after_frameset(tok),
            Mode::AfterAfterBody => self.m_after_after_body(tok),
            Mode::AfterAfterFrameset => self.m_after_after_frameset(tok),
        }
    }

    // ------------------------------------------------------------------------------------------
    // Stack and node helpers

    fn cur(&self) -> NodeId {
        self.open.last().copied().unwrap_or(NONE)
    }

    fn adjusted_current(&self) -> NodeId {
        if self.ctx != NONE && self.open.len() == 1 { self.ctx } else { self.cur() }
    }

    fn is_html_el(&self, id: NodeId, name: Atom) -> bool {
        self.doc.is_html_element(id, name)
    }

    fn cur_is(&self, name: Atom) -> bool {
        self.is_html_el(self.cur(), name)
    }

    fn html_name(&self, id: NodeId) -> Option<Atom> {
        self.doc.element(id).filter(|e| e.ns == Namespace::Html).map(|e| e.name)
    }

    fn is_special(&self, id: NodeId) -> bool {
        let Some(e) = self.doc.element(id) else { return false };
        match e.ns {
            Namespace::Html => matches!(
                e.name,
                a::ADDRESS | a::APPLET | a::AREA | a::ARTICLE | a::ASIDE | a::BASE | a::BASEFONT | a::BGSOUND
                    | a::BLOCKQUOTE | a::BODY | a::BR | a::BUTTON | a::CAPTION | a::CENTER | a::COL | a::COLGROUP
                    | a::DD | a::DETAILS | a::DIR | a::DIV | a::DL | a::DT | a::EMBED | a::FIELDSET | a::FIGCAPTION
                    | a::FIGURE | a::FOOTER | a::FORM | a::FRAME | a::FRAMESET | a::H1 | a::H2 | a::H3 | a::H4
                    | a::H5 | a::H6 | a::HEAD | a::HEADER | a::HGROUP | a::HR | a::HTML | a::IFRAME | a::IMG
                    | a::INPUT | a::KEYGEN | a::LI | a::LINK | a::LISTING | a::MAIN | a::MARQUEE | a::MENU
                    | a::META | a::NAV | a::NOEMBED | a::NOFRAMES | a::NOSCRIPT | a::OBJECT | a::OL | a::P
                    | a::PARAM | a::PLAINTEXT | a::PRE | a::SCRIPT | a::SEARCH | a::SECTION | a::SELECT
                    | a::SOURCE | a::STYLE | a::SUMMARY | a::TABLE | a::TBODY | a::TD | a::TEMPLATE
                    | a::TEXTAREA | a::TFOOT | a::TH | a::THEAD | a::TITLE | a::TR | a::TRACK | a::UL | a::WBR
                    | a::XMP
            ),
            Namespace::MathMl => matches!(e.name, a::MI | a::MO | a::MN | a::MS | a::MTEXT | a::ANNOTATION_XML),
            Namespace::Svg => matches!(e.name, a::FOREIGNOBJECT | a::DESC | a::TITLE),
            _ => false,
        }
    }

    fn is_scope_boundary(&self, id: NodeId, scope: Scope) -> bool {
        let Some(e) = self.doc.element(id) else { return false };
        if scope == Scope::Table {
            return e.ns == Namespace::Html && matches!(e.name, a::HTML | a::TABLE | a::TEMPLATE);
        }
        match e.ns {
            Namespace::Html => {
                let base = matches!(
                    e.name,
                    a::APPLET | a::CAPTION | a::HTML | a::TABLE | a::TD | a::TH | a::MARQUEE | a::OBJECT | a::SELECT | a::TEMPLATE
                );
                base || match scope {
                    Scope::ListItem => matches!(e.name, a::OL | a::UL),
                    Scope::Button => e.name == a::BUTTON,
                    _ => false,
                }
            }
            Namespace::MathMl => matches!(e.name, a::MI | a::MO | a::MN | a::MS | a::MTEXT | a::ANNOTATION_XML),
            Namespace::Svg => matches!(e.name, a::FOREIGNOBJECT | a::DESC | a::TITLE),
            _ => false,
        }
    }

    /// "Has an element in <scope>" for an HTML element whose name satisfies `pred`.
    fn has_in_scope_by(&self, pred: impl Fn(Atom) -> bool, scope: Scope) -> bool {
        for &n in self.open.iter().rev() {
            if let Some(name) = self.html_name(n)
                && pred(name)
            {
                return true;
            }
            if self.is_scope_boundary(n, scope) {
                return false;
            }
        }
        false
    }

    fn has_in_scope(&self, name: Atom, scope: Scope) -> bool {
        self.has_in_scope_by(|n| n == name, scope)
    }

    fn has_node_in_scope(&self, node: NodeId) -> bool {
        for &n in self.open.iter().rev() {
            if n == node {
                return true;
            }
            if self.is_scope_boundary(n, Scope::Default) {
                return false;
            }
        }
        false
    }

    fn is_heading(name: Atom) -> bool {
        matches!(name, a::H1 | a::H2 | a::H3 | a::H4 | a::H5 | a::H6)
    }

    fn pop_until_html(&mut self, pred: impl Fn(Atom) -> bool) {
        while let Some(n) = self.open.pop() {
            if self.html_name(n).is_some_and(&pred) {
                break;
            }
        }
    }

    fn pop_until_name(&mut self, name: Atom) {
        self.pop_until_html(|n| n == name);
    }

    fn generate_implied_end_tags(&mut self, except: Option<Atom>) {
        while let Some(name) = self.html_name(self.cur()) {
            if matches!(name, a::DD | a::DT | a::LI | a::OPTGROUP | a::OPTION | a::P | a::RB | a::RP | a::RT | a::RTC)
                && Some(name) != except
            {
                self.open.pop();
            } else {
                break;
            }
        }
    }

    fn generate_all_implied_end_tags_thoroughly(&mut self) {
        while let Some(name) = self.html_name(self.cur()) {
            if matches!(
                name,
                a::CAPTION | a::COLGROUP | a::DD | a::DT | a::LI | a::OPTGROUP | a::OPTION | a::P | a::RB | a::RP | a::RT
                    | a::RTC | a::TBODY | a::TD | a::TFOOT | a::TH | a::THEAD | a::TR
            ) {
                self.open.pop();
            } else {
                break;
            }
        }
    }

    fn close_p_element(&mut self) {
        self.generate_implied_end_tags(Some(a::P));
        self.pop_until_name(a::P);
    }

    fn close_p_if_in_button_scope(&mut self) {
        if self.has_in_scope(a::P, Scope::Button) {
            self.close_p_element();
        }
    }

    fn has_template_on_stack(&self) -> bool {
        self.open.iter().any(|&n| self.is_html_el(n, a::TEMPLATE))
    }

    fn parsing_template_contents(&self) -> bool {
        self.has_template_on_stack() || (self.ctx != NONE && self.is_html_el(self.ctx, a::TEMPLATE))
    }

    // ------------------------------------------------------------------------------------------
    // Creating and inserting nodes (13.2.6.1)

    fn template_contents_of(&self, id: NodeId) -> NodeId {
        match self.doc.element(id) {
            Some(e) if e.ns == Namespace::Html && e.name == a::TEMPLATE => e.template_contents,
            _ => NONE,
        }
    }

    /// "Appropriate place for inserting a node" (13.2.6.1). Returns (parent, reference child).
    fn appropriate_place(&self, override_target: Option<NodeId>) -> (NodeId, NodeId) {
        let mut target = override_target.unwrap_or_else(|| self.cur());
        let mut before = NONE;
        let is_table_like = self
            .doc
            .element(target)
            .is_some_and(|e| e.ns == Namespace::Html && matches!(e.name, a::TABLE | a::TBODY | a::TFOOT | a::THEAD | a::TR));
        if self.foster && is_table_like {
            let last_template = self.open.iter().rposition(|&n| self.is_html_el(n, a::TEMPLATE));
            let last_table = self.open.iter().rposition(|&n| self.is_html_el(n, a::TABLE));
            // "The last template or table element in the stack": whichever is lower in the stack.
            let lt = match (last_template, last_table) {
                (None, None) => None,
                (Some(t), None) => Some(t),
                (None, Some(t)) => Some(t),
                (Some(x), Some(y)) => Some(x.max(y)),
            };
            match lt {
                None => return (self.open.first().copied().unwrap_or(DOCUMENT), NONE),
                Some(i) => {
                    let el = self.open.get(i).copied().unwrap_or(NONE);
                    if self.is_html_el(el, a::TEMPLATE) {
                        target = el;
                    } else if self.doc.parent(el) != NONE {
                        target = self.doc.parent(el);
                        before = el;
                    } else {
                        target = if i > 0 { self.open.get(i - 1).copied().unwrap_or(NONE) } else { NONE };
                    }
                }
            }
        }
        // Inside a template the content goes to its template contents.
        let tc = self.template_contents_of(target);
        if tc != NONE {
            return (tc, NONE);
        }
        (target, before)
    }

    fn can_alloc(&self) -> bool {
        self.doc.nodes.len() < MAX_NODES + 1024
    }

    fn insert_node(&mut self, loc: (NodeId, NodeId), node: NodeId) {
        self.doc.insert_before(loc.0, node, loc.1);
    }

    /// "Create an element for a token" (13.2.6.1) from already converted attributes.
    fn create_element(&mut self, name: Atom, ns: Namespace, attrs: Attrs) -> NodeId {
        let is_template = ns == Namespace::Html && name == a::TEMPLATE;
        let contents = if is_template { self.doc.alloc(NodeData::Fragment) } else { NONE };
        let attrs = self.doc.store_attrs(attrs);
        self.doc.alloc(NodeData::Element(ElementData { name, ns, attrs, template_contents: contents }))
    }

    /// "Insert a foreign element" without pushing: create and insert at the appropriate place.
    fn insert_element_node(&mut self, name: Atom, ns: Namespace, attrs: Attrs) -> NodeId {
        let el = self.create_element(name, ns, attrs);
        let loc = self.appropriate_place(None);
        self.insert_node(loc, el);
        el
    }

    /// "Insert a foreign element" / "insert an HTML element": create, insert, push on the stack of open
    /// elements (unless the depth limit is reached: then the element is attached but not pushed).
    fn insert_element(&mut self, name: Atom, ns: Namespace, attrs: Attrs) -> (NodeId, bool) {
        let el = self.insert_element_node(name, ns, attrs);
        if self.open.len() < MAX_DEPTH {
            self.open.push(el);
            (el, true)
        } else {
            (el, false)
        }
    }

    fn insert_html_element(&mut self, t: TagToken) -> NodeId {
        self.insert_element(t.name, Namespace::Html, t.attrs).0
    }

    /// "Insert an HTML element ... immediately pop the current node": the element never goes on the stack.
    fn insert_void_html_element(&mut self, t: TagToken) -> NodeId {
        self.insert_element_node(t.name, Namespace::Html, t.attrs)
    }

    fn insert_html_element_by_name(&mut self, name: Atom) -> NodeId {
        self.insert_element(name, Namespace::Html, Attrs::default()).0
    }

    fn insert_char(&mut self, s: &str) {
        let (parent, before) = self.appropriate_place(None);
        if matches!(self.doc.node(parent).map(|n| &n.data), Some(NodeData::Document) | None) {
            return;
        }
        let prev = if before != NONE { self.doc.prev_sibling(before) } else { self.doc.last_child(parent) };
        if let Some(node) = self.doc.node_mut(prev)
            && let NodeData::Text(t) = &mut node.data
        {
            t.push_str(s);
            return;
        }
        if !self.can_alloc() {
            return;
        }
        let n = self.doc.alloc(NodeData::Text(s.to_string()));
        self.insert_node((parent, before), n);
    }

    /// Like `insert_char`, for an owned string: a new text node takes it over instead of copying it.
    fn insert_char_owned(&mut self, s: String) {
        let (parent, before) = self.appropriate_place(None);
        if matches!(self.doc.node(parent).map(|n| &n.data), Some(NodeData::Document) | None) {
            return;
        }
        let prev = if before != NONE { self.doc.prev_sibling(before) } else { self.doc.last_child(parent) };
        if let Some(node) = self.doc.node_mut(prev)
            && let NodeData::Text(t) = &mut node.data
        {
            t.push_str(&s);
            return;
        }
        if !self.can_alloc() {
            return;
        }
        let n = self.doc.alloc(NodeData::Text(s));
        self.insert_node((parent, before), n);
    }

    /// "Insert a comment" / "insert a processing instruction"; `target` overrides the location (always appended).
    fn insert_misc(&mut self, data: NodeData, target: Option<NodeId>) {
        let loc = match target {
            Some(t) => {
                let tc = self.template_contents_of(t);
                (if tc != NONE { tc } else { t }, NONE)
            }
            None => self.appropriate_place(None),
        };
        if !self.can_alloc() {
            return;
        }
        let n = self.doc.alloc(data);
        self.insert_node(loc, n);
    }

    fn insert_comment(&mut self, data: String, target: Option<NodeId>) {
        self.insert_misc(NodeData::Comment(data), target);
    }

    fn insert_pi(&mut self, target_name: String, data: String, target: Option<NodeId>) {
        self.insert_misc(NodeData::ProcessingInstruction { target: target_name, data }, target);
    }

    /// Comment and processing-instruction tokens handled the same way in most modes.
    fn insert_comment_like(&mut self, tok: Token, target: Option<NodeId>) {
        match tok {
            Token::Comment(d) => self.insert_comment(d, target),
            Token::ProcessingInstruction { target: t, data } => self.insert_pi(t, data, target),
            _ => {}
        }
    }

    // ------------------------------------------------------------------------------------------
    // Active formatting elements (13.2.4.3)

    fn afe_contains_node(&self, node: NodeId) -> bool {
        self.afe.iter().any(|e| matches!(e, Afe::Entry { node: n, .. } if *n == node))
    }

    fn afe_position(&self, node: NodeId) -> Option<usize> {
        self.afe.iter().position(|e| matches!(e, Afe::Entry { node: n, .. } if *n == node))
    }

    /// The start tag the element `node` (an entry of the list of active formatting elements) was created for.
    fn afe_token(&self, node: NodeId, name: Atom) -> TagToken {
        let attrs = self.doc.element(node).map(|e| self.doc.token_attrs(&e.attrs)).unwrap_or_default();
        TagToken { name, attrs, self_closing: false }
    }

    fn push_afe(&mut self, node: NodeId, name: Atom) {
        // Noah's Ark clause: at most three identical entries after the last marker.
        let mut same = 0usize;
        let mut earliest = None;
        let new_attrs: &[StoredAttr] = self.doc.element(node).map_or(&[], |e| e.attrs.as_slice());
        for (i, e) in self.afe.iter().enumerate().rev() {
            match e {
                Afe::Marker => break,
                Afe::Bookmark => {}
                Afe::Entry { node: n, name: nm } => {
                    if *nm == name && Self::same_attrs(&self.doc, self.doc.element(*n).map_or(&[], |e| e.attrs.as_slice()), new_attrs) {
                        same += 1;
                        earliest = Some(i);
                    }
                }
            }
        }
        if same >= 3
            && let Some(i) = earliest
        {
            self.afe.remove(i);
        }
        self.afe.push(Afe::Entry { node, name });
    }

    fn same_attrs(doc: &Document, x: &[StoredAttr], y: &[StoredAttr]) -> bool {
        x.len() == y.len() && x.iter().all(|p| y.iter().any(|q| q.name == p.name && doc.value(q) == doc.value(p)))
    }

    fn reconstruct_afe(&mut self) {
        let Some(last) = self.afe.last() else { return };
        match last {
            Afe::Marker => return,
            Afe::Entry { node, .. } if self.open.contains(node) => return,
            _ => {}
        }
        let mut i = self.afe.len() - 1;
        // Rewind.
        loop {
            if i == 0 {
                break;
            }
            i -= 1;
            let stop = match self.afe.get(i) {
                Some(Afe::Marker) => true,
                Some(Afe::Entry { node, .. }) => self.open.contains(node),
                _ => false,
            };
            if stop {
                i += 1; // Advance
                break;
            }
        }
        // Create.
        while i < self.afe.len() {
            let Some(Afe::Entry { node: old, name }) = self.afe.get(i).cloned() else {
                i += 1;
                continue;
            };
            let token = self.afe_token(old, name);
            let (node, pushed) = self.insert_element(token.name, Namespace::Html, token.attrs);
            if pushed {
                if let Some(slot) = self.afe.get_mut(i) {
                    *slot = Afe::Entry { node, name };
                }
            } else {
                // Depth limit: the clone is not on the stack; drop the entry.
                self.afe.remove(i);
                continue;
            }
            i += 1;
        }
    }

    fn clear_afe_to_marker(&mut self) {
        while let Some(e) = self.afe.pop() {
            if matches!(e, Afe::Marker) {
                break;
            }
        }
    }

    // ------------------------------------------------------------------------------------------
    // Adoption agency algorithm (13.2.6.4.7)

    fn adoption_agency(&mut self, subject: Atom) {
        let cur = self.cur();
        if self.is_html_el(cur, subject) && !self.afe_contains_node(cur) {
            self.open.pop();
            return;
        }
        for _outer in 0..8 {
            // Formatting element: last entry after the last marker with this tag name.
            let mut fe_idx = None;
            for (i, e) in self.afe.iter().enumerate().rev() {
                match e {
                    Afe::Marker => break,
                    Afe::Entry { name, .. } if *name == subject => {
                        fe_idx = Some(i);
                        break;
                    }
                    _ => {}
                }
            }
            let Some(fe_idx) = fe_idx else {
                self.any_other_end_tag(subject);
                return;
            };
            let (fe, fe_name) = match self.afe.get(fe_idx) {
                Some(Afe::Entry { node, name }) => (*node, *name),
                _ => return,
            };
            let fe_token = self.afe_token(fe, fe_name);
            let Some(fe_pos) = self.open.iter().position(|&n| n == fe) else {
                self.afe.remove(fe_idx);
                return;
            };
            if !self.has_node_in_scope(fe) {
                return;
            }
            // Furthest block: topmost special element below the formatting element in the stack.
            let fb_pos = (fe_pos + 1..self.open.len()).find(|&i| self.open.get(i).is_some_and(|&n| self.is_special(n)));
            let Some(fb_pos) = fb_pos else {
                self.open.truncate(fe_pos);
                if let Some(i) = self.afe_position(fe) {
                    self.afe.remove(i);
                }
                return;
            };
            let furthest = self.open.get(fb_pos).copied().unwrap_or(NONE);
            let common_ancestor = if fe_pos > 0 { self.open.get(fe_pos - 1).copied().unwrap_or(NONE) } else { NONE };
            // Bookmark: a placeholder in the list right after the formatting element.
            self.afe.insert(fe_idx + 1, Afe::Bookmark);
            let mut node;
            let mut last_node = furthest;
            let mut node_pos = fb_pos;
            let mut inner = 0;
            loop {
                inner += 1;
                if node_pos == 0 {
                    break;
                }
                node_pos -= 1;
                node = self.open.get(node_pos).copied().unwrap_or(NONE);
                if node == fe {
                    break;
                }
                if inner > 3
                    && let Some(i) = self.afe_position(node)
                {
                    self.afe.remove(i);
                }
                let Some(node_afe_idx) = self.afe_position(node) else {
                    self.open.remove(node_pos);
                    continue;
                };
                // Replace the node with a fresh element for its token.
                let token = match self.afe.get(node_afe_idx) {
                    Some(Afe::Entry { node: n, name }) => self.afe_token(*n, *name),
                    _ => break,
                };
                let new_el = self.create_element(token.name, Namespace::Html, token.attrs);
                if let Some(slot) = self.afe.get_mut(node_afe_idx) {
                    *slot = Afe::Entry { node: new_el, name: token.name };
                }
                if let Some(slot) = self.open.get_mut(node_pos) {
                    *slot = new_el;
                }
                node = new_el;
                if last_node == furthest {
                    // Move the bookmark to be immediately after the new node.
                    if let Some(bm) = self.afe.iter().position(|e| matches!(e, Afe::Bookmark)) {
                        self.afe.remove(bm);
                    }
                    let at = self.afe_position(new_el).map_or(self.afe.len(), |p| p + 1);
                    self.afe.insert(at, Afe::Bookmark);
                }
                self.doc.detach(last_node);
                self.doc.append_child(node, last_node);
                last_node = node;
            }
            // Insert last_node at the appropriate place with the common ancestor as override target.
            self.doc.detach(last_node);
            let loc = self.appropriate_place(Some(common_ancestor));
            self.insert_node(loc, last_node);
            // New element for the formatting element's token, adopting the furthest block's children.
            let new_el = self.create_element(fe_token.name, Namespace::Html, fe_token.attrs.clone());
            self.doc.move_children(furthest, new_el);
            let tc = self.template_contents_of(furthest);
            self.doc.append_child(if tc != NONE { tc } else { furthest }, new_el);
            // Lists: replace the formatting element's entry by the bookmark's position.
            if let Some(i) = self.afe_position(fe) {
                self.afe.remove(i);
            }
            let bm = self.afe.iter().position(|e| matches!(e, Afe::Bookmark));
            match bm {
                Some(b) => {
                    if let Some(slot) = self.afe.get_mut(b) {
                        *slot = Afe::Entry { node: new_el, name: fe_name };
                    }
                }
                None => self.afe.push(Afe::Entry { node: new_el, name: fe_name }),
            }
            if let Some(i) = self.open.iter().position(|&n| n == fe) {
                self.open.remove(i);
            }
            let fb_now = self.open.iter().position(|&n| n == furthest).map_or(self.open.len(), |p| p + 1);
            self.open.insert(fb_now, new_el);
        }
        // Remove a leftover bookmark, if any.
        self.afe.retain(|e| !matches!(e, Afe::Bookmark));
    }

    /// "Any other end tag" of the "in body" insertion mode.
    fn any_other_end_tag(&mut self, name: Atom) {
        let mut i = self.open.len();
        while i > 0 {
            i -= 1;
            let node = self.open.get(i).copied().unwrap_or(NONE);
            if self.is_html_el(node, name) {
                self.generate_implied_end_tags(Some(name));
                self.open.truncate(i);
                return;
            }
            if self.is_special(node) {
                return;
            }
        }
    }

    // ------------------------------------------------------------------------------------------
    // Insertion mode management

    fn reset_insertion_mode(&mut self) {
        let mut last = false;
        let mut i = self.open.len();
        while i > 0 {
            i -= 1;
            let mut node = self.open.get(i).copied().unwrap_or(NONE);
            if i == 0 {
                last = true;
                if self.ctx != NONE {
                    node = self.ctx;
                }
            }
            let Some(name) = self.html_name(node) else {
                if last {
                    self.mode = Mode::InBody;
                    return;
                }
                continue;
            };
            let m = match name {
                a::TD | a::TH if !last => Some(Mode::InCell),
                a::TR => Some(Mode::InRow),
                a::TBODY | a::THEAD | a::TFOOT => Some(Mode::InTableBody),
                a::CAPTION => Some(Mode::InCaption),
                a::COLGROUP => Some(Mode::InColumnGroup),
                a::TABLE => Some(Mode::InTable),
                a::TEMPLATE => Some(self.tmpl_modes.last().copied().unwrap_or(Mode::InTemplate)),
                a::HEAD if !last => Some(Mode::InHead),
                a::BODY => Some(Mode::InBody),
                a::FRAMESET => Some(Mode::InFrameset),
                a::HTML => Some(if self.head == NONE { Mode::BeforeHead } else { Mode::AfterHead }),
                _ => None,
            };
            if let Some(m) = m {
                self.mode = m;
                return;
            }
            if last {
                self.mode = Mode::InBody;
                return;
            }
        }
        self.mode = Mode::InBody;
    }

    fn clear_stack_to(&mut self, stop: impl Fn(Atom) -> bool) {
        while let Some(&n) = self.open.last() {
            if self.html_name(n).is_some_and(&stop) {
                break;
            }
            // Never pop the root element (fragment case).
            if self.open.len() <= 1 {
                break;
            }
            self.open.pop();
        }
    }

    fn clear_to_table_context(&mut self) {
        self.clear_stack_to(|n| matches!(n, a::TABLE | a::TEMPLATE | a::HTML));
    }

    fn clear_to_table_body_context(&mut self) {
        self.clear_stack_to(|n| matches!(n, a::TBODY | a::TFOOT | a::THEAD | a::TEMPLATE | a::HTML));
    }

    fn clear_to_table_row_context(&mut self) {
        self.clear_stack_to(|n| matches!(n, a::TR | a::TEMPLATE | a::HTML));
    }

    fn generic_text_element(&mut self, t: TagToken, state: State) {
        self.insert_html_element(t);
        self.tok.set_state(state);
        self.orig_mode = self.mode;
        self.mode = Mode::Text;
    }

    fn stop(&mut self) {
        self.done = true;
    }

    // ------------------------------------------------------------------------------------------
    // Insertion modes

    fn m_initial(&mut self, tok: Token) {
        match tok {
            Token::Text(ref s) if is_ws_str(s) => {}
            Token::Comment(_) | Token::ProcessingInstruction { .. } => self.insert_comment_like(tok, Some(DOCUMENT)),
            Token::Doctype(d) => {
                let name = d.name.clone().unwrap_or_default();
                let node = self.doc.alloc(NodeData::Doctype(Box::new(super::dom::DoctypeData {
                    name,
                    public_id: d.public_id.clone().unwrap_or_default(),
                    system_id: d.system_id.clone().unwrap_or_default(),
                })));
                self.doc.append_child(DOCUMENT, node);
                self.doc.quirks = quirks_mode_for(&d);
                self.mode = Mode::BeforeHtml;
            }
            other => {
                self.doc.quirks = QuirksMode::Quirks;
                self.mode = Mode::BeforeHtml;
                self.step(Mode::BeforeHtml, other);
            }
        }
    }

    fn m_before_html(&mut self, tok: Token) {
        match tok {
            Token::Doctype(_) => {}
            Token::Comment(_) | Token::ProcessingInstruction { .. } => self.insert_comment_like(tok, Some(DOCUMENT)),
            Token::Text(ref s) if is_ws_str(s) => {}
            Token::StartTag(t) if t.name == a::HTML => {
                let el = self.create_element(a::HTML, Namespace::Html, t.attrs);
                self.doc.append_child(DOCUMENT, el);
                self.open.push(el);
                self.mode = Mode::BeforeHead;
            }
            Token::EndTag(ref t) if !matches!(t.name, a::HEAD | a::BODY | a::HTML | a::BR) => {}
            other => {
                let el = self.create_element(a::HTML, Namespace::Html, Attrs::default());
                self.doc.append_child(DOCUMENT, el);
                self.open.push(el);
                self.mode = Mode::BeforeHead;
                self.step(Mode::BeforeHead, other);
            }
        }
    }

    fn m_before_head(&mut self, tok: Token) {
        match tok {
            Token::Text(ref s) if is_ws_str(s) => {}
            Token::Comment(_) | Token::ProcessingInstruction { .. } => self.insert_comment_like(tok, None),
            Token::Doctype(_) => {}
            Token::StartTag(t) if t.name == a::HTML => self.step(Mode::InBody, Token::StartTag(t)),
            Token::StartTag(t) if t.name == a::HEAD => {
                self.head = self.insert_html_element(t);
                self.mode = Mode::InHead;
            }
            Token::EndTag(ref t) if !matches!(t.name, a::HEAD | a::BODY | a::HTML | a::BR) => {}
            other => {
                self.head = self.insert_html_element_by_name(a::HEAD);
                self.mode = Mode::InHead;
                self.step(Mode::InHead, other);
            }
        }
    }

    fn m_in_head(&mut self, tok: Token) {
        match tok {
            Token::Text(ref s) if is_ws_str(s) => self.insert_char(s),
            Token::Comment(_) | Token::ProcessingInstruction { .. } => self.insert_comment_like(tok, None),
            Token::Doctype(_) => {}
            Token::StartTag(t) => match t.name {
                a::HTML => self.step(Mode::InBody, Token::StartTag(t)),
                a::BASE | a::BASEFONT | a::BGSOUND | a::LINK | a::META => {
                    self.insert_void_html_element(t);
                }
                a::TITLE => self.generic_text_element(t, State::RcData),
                a::NOSCRIPT => {
                    // Scripting is disabled: parse the content as HTML.
                    self.insert_html_element(t);
                    self.mode = Mode::InHeadNoscript;
                }
                a::NOFRAMES | a::STYLE => self.generic_text_element(t, State::RawText),
                a::SCRIPT => {
                    self.insert_html_element(t);
                    self.tok.set_state(State::ScriptData);
                    self.orig_mode = self.mode;
                    self.mode = Mode::Text;
                }
                a::TEMPLATE => {
                    self.afe.push(Afe::Marker);
                    self.frameset_ok = false;
                    self.mode = Mode::InTemplate;
                    self.tmpl_modes.push(Mode::InTemplate);
                    self.insert_html_element(t);
                }
                a::HEAD => {}
                _ => self.in_head_anything_else(Token::StartTag(t)),
            },
            Token::EndTag(t) => match t.name {
                a::HEAD => {
                    self.open.pop();
                    self.mode = Mode::AfterHead;
                }
                a::BODY | a::HTML | a::BR => self.in_head_anything_else(Token::EndTag(t)),
                a::TEMPLATE => {
                    if !self.has_template_on_stack() {
                        return;
                    }
                    self.generate_all_implied_end_tags_thoroughly();
                    self.pop_until_name(a::TEMPLATE);
                    self.clear_afe_to_marker();
                    self.tmpl_modes.pop();
                    self.reset_insertion_mode();
                }
                _ => {}
            },
            other => self.in_head_anything_else(other),
        }
    }

    fn in_head_anything_else(&mut self, tok: Token) {
        self.open.pop();
        self.mode = Mode::AfterHead;
        self.step(Mode::AfterHead, tok);
    }

    fn m_in_head_noscript(&mut self, tok: Token) {
        match tok {
            Token::Doctype(_) => {}
            Token::StartTag(t) if t.name == a::HTML => self.step(Mode::InBody, Token::StartTag(t)),
            Token::EndTag(ref t) if t.name == a::NOSCRIPT => {
                self.open.pop();
                self.mode = Mode::InHead;
            }
            Token::Text(ref s) if is_ws_str(s) => self.step(Mode::InHead, tok),
            Token::Comment(_) | Token::ProcessingInstruction { .. } => self.step(Mode::InHead, tok),
            Token::StartTag(ref t) if matches!(t.name, a::BASEFONT | a::BGSOUND | a::LINK | a::META | a::NOFRAMES | a::STYLE) => {
                self.step(Mode::InHead, tok)
            }
            Token::StartTag(ref t) if matches!(t.name, a::HEAD | a::NOSCRIPT) => {}
            Token::EndTag(ref t) if t.name != a::BR => {}
            other => {
                self.open.pop();
                self.mode = Mode::InHead;
                self.step(Mode::InHead, other);
            }
        }
    }

    fn m_after_head(&mut self, tok: Token) {
        match tok {
            Token::Text(ref s) if is_ws_str(s) => self.insert_char(s),
            Token::Comment(_) | Token::ProcessingInstruction { .. } => self.insert_comment_like(tok, None),
            Token::Doctype(_) => {}
            Token::StartTag(t) => match t.name {
                a::HTML => self.step(Mode::InBody, Token::StartTag(t)),
                a::BODY => {
                    self.insert_html_element(t);
                    self.frameset_ok = false;
                    self.mode = Mode::InBody;
                }
                a::FRAMESET => {
                    self.insert_html_element(t);
                    self.mode = Mode::InFrameset;
                }
                a::BASE | a::BASEFONT | a::BGSOUND | a::LINK | a::META | a::NOFRAMES | a::SCRIPT | a::STYLE | a::TEMPLATE | a::TITLE => {
                    let head = self.head;
                    self.open.push(head);
                    self.step(Mode::InHead, Token::StartTag(t));
                    if let Some(i) = self.open.iter().rposition(|&n| n == head) {
                        self.open.remove(i);
                    }
                }
                a::HEAD => {}
                _ => self.after_head_anything_else(Token::StartTag(t)),
            },
            Token::EndTag(t) => match t.name {
                a::TEMPLATE => self.step(Mode::InHead, Token::EndTag(t)),
                a::BODY | a::HTML | a::BR => self.after_head_anything_else(Token::EndTag(t)),
                _ => {}
            },
            other => self.after_head_anything_else(other),
        }
    }

    fn after_head_anything_else(&mut self, tok: Token) {
        self.insert_html_element_by_name(a::BODY);
        self.frameset_ok = true;
        self.mode = Mode::InBody;
        self.step(Mode::InBody, tok);
    }

    // ----- in body -----

    fn m_in_body(&mut self, tok: Token) {
        match tok {
            Token::Text(s) => {
                if s.bytes().all(|b| b == 0) {
                    return;
                }
                self.reconstruct_afe();
                let ws = is_ws_str(&s);
                self.insert_char_owned(s);
                if !ws {
                    self.frameset_ok = false;
                }
            }
            Token::Comment(_) | Token::ProcessingInstruction { .. } => self.insert_comment_like(tok, None),
            Token::Doctype(_) => {}
            Token::StartTag(t) => self.in_body_start(t),
            Token::EndTag(t) => self.in_body_end(t),
            Token::Eof => {
                if !self.tmpl_modes.is_empty() {
                    self.step(Mode::InTemplate, Token::Eof);
                } else {
                    self.stop();
                }
            }
        }
    }

    fn in_body_start(&mut self, mut t: TagToken) {
        match t.name {
            a::HTML => {
                if self.has_template_on_stack() {
                    return;
                }
                if let Some(&root) = self.open.first() {
                    self.merge_attrs(root, t.attrs);
                }
            }
            a::BASE | a::BASEFONT | a::BGSOUND | a::LINK | a::META | a::NOFRAMES | a::SCRIPT | a::STYLE | a::TEMPLATE | a::TITLE => {
                self.step(Mode::InHead, Token::StartTag(t))
            }
            a::BODY => {
                let second = self.open.get(1).copied().unwrap_or(NONE);
                if self.open.len() == 1 || !self.is_html_el(second, a::BODY) || self.has_template_on_stack() {
                    return;
                }
                self.frameset_ok = false;
                self.merge_attrs(second, t.attrs);
            }
            a::FRAMESET => {
                let second = self.open.get(1).copied().unwrap_or(NONE);
                if self.open.len() == 1 || !self.is_html_el(second, a::BODY) || !self.frameset_ok {
                    return;
                }
                self.doc.detach(second);
                self.open.truncate(1);
                self.insert_html_element(t);
                self.mode = Mode::InFrameset;
            }
            a::ADDRESS | a::ARTICLE | a::ASIDE | a::BLOCKQUOTE | a::CENTER | a::DETAILS | a::DIALOG | a::DIR | a::DIV | a::DL
            | a::FIELDSET | a::FIGCAPTION | a::FIGURE | a::FOOTER | a::HEADER | a::HGROUP | a::MAIN | a::MENU | a::NAV | a::OL
            | a::P | a::SEARCH | a::SECTION | a::SUMMARY | a::UL => {
                self.close_p_if_in_button_scope();
                self.insert_html_element(t);
            }
            a::H1 | a::H2 | a::H3 | a::H4 | a::H5 | a::H6 => {
                self.close_p_if_in_button_scope();
                if self.html_name(self.cur()).is_some_and(Self::is_heading) {
                    self.open.pop();
                }
                self.insert_html_element(t);
            }
            a::PRE | a::LISTING => {
                self.close_p_if_in_button_scope();
                self.insert_html_element(t);
                self.skip_lf = true;
                self.frameset_ok = false;
            }
            a::FORM => {
                if self.form != NONE && !self.parsing_template_contents() {
                    return;
                }
                self.close_p_if_in_button_scope();
                let el = self.insert_html_element(t);
                if !self.parsing_template_contents() {
                    self.form = el;
                }
            }
            a::LI => {
                self.frameset_ok = false;
                let mut i = self.open.len();
                while i > 0 {
                    i -= 1;
                    let node = self.open.get(i).copied().unwrap_or(NONE);
                    if self.is_html_el(node, a::LI) {
                        self.generate_implied_end_tags(Some(a::LI));
                        self.pop_until_name(a::LI);
                        break;
                    }
                    if self.is_special(node) && !matches!(self.html_name(node), Some(a::ADDRESS | a::DIV | a::P)) {
                        break;
                    }
                }
                self.close_p_if_in_button_scope();
                self.insert_html_element(t);
            }
            a::DD | a::DT => {
                self.frameset_ok = false;
                let mut i = self.open.len();
                while i > 0 {
                    i -= 1;
                    let node = self.open.get(i).copied().unwrap_or(NONE);
                    if let Some(n) = self.html_name(node).filter(|n| matches!(*n, a::DD | a::DT)) {
                        self.generate_implied_end_tags(Some(n));
                        self.pop_until_name(n);
                        break;
                    }
                    if self.is_special(node) && !matches!(self.html_name(node), Some(a::ADDRESS | a::DIV | a::P)) {
                        break;
                    }
                }
                self.close_p_if_in_button_scope();
                self.insert_html_element(t);
            }
            a::PLAINTEXT => {
                self.close_p_if_in_button_scope();
                self.insert_html_element(t);
                self.tok.set_state(State::PlainText);
            }
            a::BUTTON => {
                if self.has_in_scope(a::BUTTON, Scope::Default) {
                    self.generate_implied_end_tags(None);
                    self.pop_until_name(a::BUTTON);
                }
                self.reconstruct_afe();
                self.insert_html_element(t);
                self.frameset_ok = false;
            }
            a::A => {
                // An `a` element in the list after the last marker: close it first.
                let existing = self
                    .afe
                    .iter()
                    .rev()
                    .take_while(|e| !matches!(e, Afe::Marker))
                    .find_map(|e| match e {
                        Afe::Entry { node, name } if *name == a::A => Some(*node),
                        _ => None,
                    });
                if let Some(node) = existing {
                    self.adoption_agency(a::A);
                    if let Some(i) = self.afe_position(node) {
                        self.afe.remove(i);
                    }
                    if let Some(i) = self.open.iter().position(|&n| n == node) {
                        self.open.remove(i);
                    }
                }
                self.reconstruct_afe();
                self.insert_formatting(t);
            }
            a::B | a::BIG | a::CODE | a::EM | a::FONT | a::I | a::S | a::SMALL | a::STRIKE | a::STRONG | a::TT | a::U => {
                self.reconstruct_afe();
                self.insert_formatting(t);
            }
            a::NOBR => {
                self.reconstruct_afe();
                if self.has_in_scope(a::NOBR, Scope::Default) {
                    self.adoption_agency(a::NOBR);
                    self.reconstruct_afe();
                }
                self.insert_formatting(t);
            }
            a::APPLET | a::MARQUEE | a::OBJECT => {
                self.reconstruct_afe();
                self.insert_html_element(t);
                self.afe.push(Afe::Marker);
                self.frameset_ok = false;
            }
            a::TABLE => {
                if self.doc.quirks != QuirksMode::Quirks {
                    self.close_p_if_in_button_scope();
                }
                self.insert_html_element(t);
                self.frameset_ok = false;
                self.mode = Mode::InTable;
            }
            a::AREA | a::BR | a::EMBED | a::IMG | a::KEYGEN | a::WBR => {
                self.reconstruct_afe();
                self.insert_void_html_element(t);
                self.frameset_ok = false;
            }
            a::INPUT => {
                if self.ctx != NONE && self.is_html_el(self.ctx, a::SELECT) {
                    return;
                }
                if self.has_in_scope(a::SELECT, Scope::Default) {
                    self.pop_until_name(a::SELECT);
                }
                self.reconstruct_afe();
                let hidden = t.attrs.iter().any(|at| at.name == a::TYPE && t.attrs.value(at).eq_ignore_ascii_case("hidden"));
                self.insert_void_html_element(t);
                if !hidden {
                    self.frameset_ok = false;
                }
            }
            a::PARAM | a::SOURCE | a::TRACK => {
                self.insert_void_html_element(t);
            }
            a::HR => {
                self.close_p_if_in_button_scope();
                if self.has_in_scope(a::SELECT, Scope::Default) {
                    self.generate_implied_end_tags(None);
                }
                self.insert_void_html_element(t);
                self.frameset_ok = false;
            }
            a::IMAGE => {
                t.name = a::IMG;
                self.in_body_start(t);
            }
            a::TEXTAREA => {
                self.insert_html_element(t);
                self.skip_lf = true;
                self.tok.set_state(State::RcData);
                self.orig_mode = self.mode;
                self.frameset_ok = false;
                self.mode = Mode::Text;
            }
            a::XMP => {
                self.close_p_if_in_button_scope();
                self.reconstruct_afe();
                self.frameset_ok = false;
                self.generic_text_element(t, State::RawText);
            }
            a::IFRAME => {
                self.frameset_ok = false;
                self.generic_text_element(t, State::RawText);
            }
            a::NOEMBED => self.generic_text_element(t, State::RawText),
            a::SELECT => {
                if self.ctx != NONE && self.is_html_el(self.ctx, a::SELECT) {
                    return;
                }
                if self.has_in_scope(a::SELECT, Scope::Default) {
                    self.pop_until_name(a::SELECT);
                    return;
                }
                self.reconstruct_afe();
                self.insert_html_element(t);
                self.frameset_ok = false;
            }
            a::OPTION => {
                if self.has_in_scope(a::SELECT, Scope::Default) {
                    self.generate_implied_end_tags(Some(a::OPTGROUP));
                } else if self.cur_is(a::OPTION) {
                    self.open.pop();
                }
                self.reconstruct_afe();
                self.insert_html_element(t);
            }
            a::OPTGROUP => {
                if self.has_in_scope(a::SELECT, Scope::Default) {
                    self.generate_implied_end_tags(None);
                } else if self.cur_is(a::OPTION) {
                    self.open.pop();
                }
                self.reconstruct_afe();
                self.insert_html_element(t);
            }
            a::RB | a::RTC => {
                if self.has_in_scope(a::RUBY, Scope::Default) {
                    self.generate_implied_end_tags(None);
                }
                self.insert_html_element(t);
            }
            a::RP | a::RT => {
                if self.has_in_scope(a::RUBY, Scope::Default) {
                    self.generate_implied_end_tags(Some(a::RTC));
                }
                self.insert_html_element(t);
            }
            a::MATH => {
                self.reconstruct_afe();
                let attrs = self.adjust_attrs(t.attrs, Namespace::MathMl);
                let (_, pushed) = self.insert_element(a::MATH, Namespace::MathMl, attrs);
                if t.self_closing && pushed {
                    self.open.pop();
                }
            }
            a::SVG => {
                self.reconstruct_afe();
                let attrs = self.adjust_attrs(t.attrs, Namespace::Svg);
                let (_, pushed) = self.insert_element(a::SVG, Namespace::Svg, attrs);
                if t.self_closing && pushed {
                    self.open.pop();
                }
            }
            a::CAPTION | a::COL | a::COLGROUP | a::FRAME | a::HEAD | a::TBODY | a::TD | a::TFOOT | a::TH | a::THEAD | a::TR => {}
            _ => {
                self.reconstruct_afe();
                self.insert_html_element(t);
            }
        }
    }

    fn insert_formatting(&mut self, t: TagToken) {
        let name = t.name;
        let (node, pushed) = self.insert_element(name, Namespace::Html, t.attrs);
        if pushed {
            self.push_afe(node, name);
        }
    }

    fn merge_attrs(&mut self, target: NodeId, attrs: Attrs) {
        let Some(e) = self.doc.element(target) else { return };
        let mut new = attrs;
        new.retain(|at| !e.attrs.iter().any(|x| x.ns == Namespace::None && x.name == at.name));
        let stored = self.doc.store_attrs(new);
        if let Some(e) = self.doc.element_mut(target) {
            e.attrs.extend(stored);
        }
    }

    fn in_body_end(&mut self, t: TagToken) {
        match t.name {
            a::TEMPLATE => self.step(Mode::InHead, Token::EndTag(t)),
            a::BODY => {
                if !self.has_in_scope(a::BODY, Scope::Default) {
                    return;
                }
                self.mode = Mode::AfterBody;
            }
            a::HTML => {
                if !self.has_in_scope(a::BODY, Scope::Default) {
                    return;
                }
                self.mode = Mode::AfterBody;
                self.step(Mode::AfterBody, Token::EndTag(t));
            }
            a::ADDRESS | a::ARTICLE | a::ASIDE | a::BLOCKQUOTE | a::BUTTON | a::CENTER | a::DETAILS | a::DIALOG | a::DIR | a::DIV
            | a::DL | a::FIELDSET | a::FIGCAPTION | a::FIGURE | a::FOOTER | a::HEADER | a::HGROUP | a::LISTING | a::MAIN
            | a::MENU | a::NAV | a::OL | a::PRE | a::SEARCH | a::SECTION | a::SELECT | a::SUMMARY | a::UL => {
                if !self.has_in_scope(t.name, Scope::Default) {
                    return;
                }
                self.generate_implied_end_tags(None);
                self.pop_until_name(t.name);
            }
            a::FORM => {
                if !self.parsing_template_contents() {
                    let node = self.form;
                    self.form = NONE;
                    if node == NONE || !self.has_node_in_scope(node) {
                        return;
                    }
                    self.generate_implied_end_tags(None);
                    if let Some(i) = self.open.iter().position(|&n| n == node) {
                        self.open.remove(i);
                    }
                } else {
                    if !self.has_in_scope(a::FORM, Scope::Default) {
                        return;
                    }
                    self.generate_implied_end_tags(None);
                    self.pop_until_name(a::FORM);
                }
            }
            a::P => {
                if !self.has_in_scope(a::P, Scope::Button) {
                    self.insert_html_element_by_name(a::P);
                }
                self.close_p_element();
            }
            a::LI => {
                if !self.has_in_scope(a::LI, Scope::ListItem) {
                    return;
                }
                self.generate_implied_end_tags(Some(a::LI));
                self.pop_until_name(a::LI);
            }
            a::DD | a::DT => {
                if !self.has_in_scope(t.name, Scope::Default) {
                    return;
                }
                self.generate_implied_end_tags(Some(t.name));
                self.pop_until_name(t.name);
            }
            a::H1 | a::H2 | a::H3 | a::H4 | a::H5 | a::H6 => {
                if !self.has_in_scope_by(Self::is_heading, Scope::Default) {
                    return;
                }
                self.generate_implied_end_tags(None);
                self.pop_until_html(Self::is_heading);
            }
            a::A | a::B | a::BIG | a::CODE | a::EM | a::FONT | a::I | a::NOBR | a::S | a::SMALL | a::STRIKE | a::STRONG | a::TT
            | a::U => self.adoption_agency(t.name),
            a::APPLET | a::MARQUEE | a::OBJECT => {
                if !self.has_in_scope(t.name, Scope::Default) {
                    return;
                }
                self.generate_implied_end_tags(None);
                self.pop_until_name(t.name);
                self.clear_afe_to_marker();
            }
            a::BR => {
                // Act as if this was a "br" start tag token with no attributes.
                self.reconstruct_afe();
                self.insert_element_node(a::BR, Namespace::Html, Attrs::default());
                self.frameset_ok = false;
            }
            _ => self.any_other_end_tag(t.name),
        }
    }

    /// Adjust MathML/SVG attribute names and foreign attributes (13.2.6.1, "adjust ... attributes").
    fn adjust_attrs(&mut self, attrs: Attrs, ns: Namespace) -> Attrs {
        let mut out = Attrs::default();
        for r in attrs.iter() {
            let mut name = r.name;
            let text = self.doc.atoms.get(name).to_string();
            match ns {
                Namespace::MathMl if text == "definitionurl" => name = self.doc.atoms.intern("definitionURL"),
                Namespace::Svg => {
                    if let Ok(i) = SVG_ATTRS.binary_search_by(|(k, _)| (*k).cmp(text.as_str()))
                        && let Some(&(_, fixed)) = SVG_ATTRS.get(i)
                    {
                        name = self.doc.atoms.intern(fixed);
                    }
                }
                _ => {}
            }
            let (ans, prefix, local) = match text.as_str() {
                "xlink:actuate" | "xlink:arcrole" | "xlink:href" | "xlink:role" | "xlink:show" | "xlink:title" | "xlink:type" => {
                    (Namespace::XLink, "xlink", text.get(6..).unwrap_or(""))
                }
                "xml:lang" | "xml:space" => (Namespace::Xml, "xml", text.get(4..).unwrap_or("")),
                "xmlns" => (Namespace::Xmlns, "", "xmlns"),
                "xmlns:xlink" => (Namespace::Xmlns, "xmlns", "xlink"),
                _ => (Namespace::None, "", ""),
            };
            if ans != Namespace::None {
                let p = self.doc.atoms.intern(prefix);
                let l = self.doc.atoms.intern(local);
                out.push(ans, p, l, attrs.value(r));
            } else {
                out.push(Namespace::None, a::EMPTY, name, attrs.value(r));
            }
        }
        out
    }

    // ----- text -----

    fn m_text(&mut self, tok: Token) {
        match tok {
            Token::Text(s) => self.insert_char(&s),
            Token::Eof => {
                self.open.pop();
                self.mode = self.orig_mode;
                self.step(self.mode, Token::Eof);
            }
            Token::EndTag(_) => {
                self.open.pop();
                self.mode = self.orig_mode;
            }
            _ => {}
        }
    }

    // ----- tables -----

    fn m_in_table(&mut self, tok: Token) {
        match tok {
            Token::Text(_) if self.html_name(self.cur()).is_some_and(|n| matches!(n, a::TABLE | a::TBODY | a::TEMPLATE | a::TFOOT | a::THEAD | a::TR)) => {
                self.pending.clear();
                self.orig_mode = self.mode;
                self.mode = Mode::InTableText;
                self.step(Mode::InTableText, tok);
            }
            Token::Comment(_) | Token::ProcessingInstruction { .. } => self.insert_comment_like(tok, None),
            Token::Doctype(_) => {}
            Token::StartTag(t) => match t.name {
                a::CAPTION => {
                    self.clear_to_table_context();
                    self.afe.push(Afe::Marker);
                    self.insert_html_element(t);
                    self.mode = Mode::InCaption;
                }
                a::COLGROUP => {
                    self.clear_to_table_context();
                    self.insert_html_element(t);
                    self.mode = Mode::InColumnGroup;
                }
                a::COL => {
                    self.clear_to_table_context();
                    self.insert_html_element_by_name(a::COLGROUP);
                    self.mode = Mode::InColumnGroup;
                    self.step(Mode::InColumnGroup, Token::StartTag(t));
                }
                a::TBODY | a::TFOOT | a::THEAD => {
                    self.clear_to_table_context();
                    self.insert_html_element(t);
                    self.mode = Mode::InTableBody;
                }
                a::TD | a::TH | a::TR => {
                    self.clear_to_table_context();
                    self.insert_html_element_by_name(a::TBODY);
                    self.mode = Mode::InTableBody;
                    self.step(Mode::InTableBody, Token::StartTag(t));
                }
                a::TABLE => {
                    if !self.has_in_scope(a::TABLE, Scope::Table) {
                        return;
                    }
                    self.pop_until_name(a::TABLE);
                    self.reset_insertion_mode();
                    self.step(self.mode, Token::StartTag(t));
                }
                a::STYLE | a::SCRIPT | a::TEMPLATE => self.step(Mode::InHead, Token::StartTag(t)),
                a::INPUT => {
                    let hidden = t.attrs.iter().any(|at| at.name == a::TYPE && t.attrs.value(at).eq_ignore_ascii_case("hidden"));
                    if hidden {
                        self.insert_void_html_element(t);
                    } else {
                        self.in_table_anything_else(Token::StartTag(t));
                    }
                }
                a::FORM => {
                    if self.form != NONE && !self.parsing_template_contents() {
                        return;
                    }
                    let el = self.insert_element_node(a::FORM, Namespace::Html, t.attrs);
                    if !self.parsing_template_contents() {
                        self.form = el;
                    }
                }
                _ => self.in_table_anything_else(Token::StartTag(t)),
            },
            Token::EndTag(t) => match t.name {
                a::TABLE => {
                    if !self.has_in_scope(a::TABLE, Scope::Table) {
                        return;
                    }
                    self.pop_until_name(a::TABLE);
                    self.reset_insertion_mode();
                }
                a::BODY | a::CAPTION | a::COL | a::COLGROUP | a::HTML | a::TBODY | a::TD | a::TFOOT | a::TH | a::THEAD | a::TR => {}
                a::TEMPLATE => self.step(Mode::InHead, Token::EndTag(t)),
                _ => self.in_table_anything_else(Token::EndTag(t)),
            },
            Token::Eof => self.step(Mode::InBody, Token::Eof),
            other => self.in_table_anything_else(other),
        }
    }

    fn in_table_anything_else(&mut self, tok: Token) {
        self.foster = true;
        self.step(Mode::InBody, tok);
        self.foster = false;
    }

    fn m_in_table_text(&mut self, tok: Token) {
        match tok {
            Token::Text(s) => {
                if s.chars().all(|c| c == '\0') {
                    return;
                }
                self.pending.push(s);
            }
            other => {
                let pending = std::mem::take(&mut self.pending);
                if pending.iter().any(|p| !is_ws_str(p)) {
                    // Reprocess with the "anything else" rules of "in table": foster parenting.
                    self.foster = true;
                    for p in pending {
                        self.step(Mode::InBody, Token::Text(p));
                    }
                    self.foster = false;
                } else {
                    for p in pending {
                        self.insert_char(&p);
                    }
                }
                self.mode = self.orig_mode;
                self.step(self.mode, other);
            }
        }
    }

    fn m_in_caption(&mut self, tok: Token) {
        match tok {
            Token::EndTag(ref t) if t.name == a::CAPTION => {
                if !self.has_in_scope(a::CAPTION, Scope::Table) {
                    return;
                }
                self.generate_implied_end_tags(None);
                self.pop_until_name(a::CAPTION);
                self.clear_afe_to_marker();
                self.mode = Mode::InTable;
            }
            Token::StartTag(ref t)
                if matches!(t.name, a::CAPTION | a::COL | a::COLGROUP | a::TBODY | a::TD | a::TFOOT | a::TH | a::THEAD | a::TR) =>
            {
                self.caption_close_and_reprocess(tok)
            }
            Token::EndTag(ref t) if t.name == a::TABLE => self.caption_close_and_reprocess(tok),
            Token::EndTag(ref t) if matches!(t.name, a::BODY | a::COL | a::COLGROUP | a::HTML | a::TBODY | a::TD | a::TFOOT | a::TH | a::THEAD | a::TR) => {}
            other => self.step(Mode::InBody, other),
        }
    }

    fn caption_close_and_reprocess(&mut self, tok: Token) {
        if !self.has_in_scope(a::CAPTION, Scope::Table) {
            return;
        }
        self.generate_implied_end_tags(None);
        self.pop_until_name(a::CAPTION);
        self.clear_afe_to_marker();
        self.mode = Mode::InTable;
        self.step(Mode::InTable, tok);
    }

    fn m_in_column_group(&mut self, tok: Token) {
        match tok {
            Token::Text(ref s) if is_ws_str(s) => self.insert_char(s),
            Token::Comment(_) | Token::ProcessingInstruction { .. } => self.insert_comment_like(tok, None),
            Token::Doctype(_) => {}
            Token::StartTag(t) => match t.name {
                a::HTML => self.step(Mode::InBody, Token::StartTag(t)),
                a::COL => {
                    self.insert_void_html_element(t);
                }
                a::TEMPLATE => self.step(Mode::InHead, Token::StartTag(t)),
                _ => self.column_group_anything_else(Token::StartTag(t)),
            },
            Token::EndTag(t) => match t.name {
                a::COLGROUP => {
                    if !self.cur_is(a::COLGROUP) {
                        return;
                    }
                    self.open.pop();
                    self.mode = Mode::InTable;
                }
                a::COL => {}
                a::TEMPLATE => self.step(Mode::InHead, Token::EndTag(t)),
                _ => self.column_group_anything_else(Token::EndTag(t)),
            },
            Token::Eof => self.step(Mode::InBody, Token::Eof),
            other => self.column_group_anything_else(other),
        }
    }

    fn column_group_anything_else(&mut self, tok: Token) {
        if !self.cur_is(a::COLGROUP) {
            return;
        }
        self.open.pop();
        self.mode = Mode::InTable;
        self.step(Mode::InTable, tok);
    }

    fn m_in_table_body(&mut self, tok: Token) {
        match tok {
            Token::StartTag(t) => match t.name {
                a::TR => {
                    self.clear_to_table_body_context();
                    self.insert_html_element(t);
                    self.mode = Mode::InRow;
                }
                a::TH | a::TD => {
                    self.clear_to_table_body_context();
                    self.insert_html_element_by_name(a::TR);
                    self.mode = Mode::InRow;
                    self.step(Mode::InRow, Token::StartTag(t));
                }
                a::CAPTION | a::COL | a::COLGROUP | a::TBODY | a::TFOOT | a::THEAD => self.table_body_close_and_reprocess(Token::StartTag(t)),
                _ => self.step(Mode::InTable, Token::StartTag(t)),
            },
            Token::EndTag(t) => match t.name {
                a::TBODY | a::TFOOT | a::THEAD => {
                    if !self.has_in_scope(t.name, Scope::Table) {
                        return;
                    }
                    self.clear_to_table_body_context();
                    self.open.pop();
                    self.mode = Mode::InTable;
                }
                a::TABLE => self.table_body_close_and_reprocess(Token::EndTag(t)),
                a::BODY | a::CAPTION | a::COL | a::COLGROUP | a::HTML | a::TD | a::TH | a::TR => {}
                _ => self.step(Mode::InTable, Token::EndTag(t)),
            },
            other => self.step(Mode::InTable, other),
        }
    }

    fn table_body_close_and_reprocess(&mut self, tok: Token) {
        if !self.has_in_scope_by(|n| matches!(n, a::TBODY | a::THEAD | a::TFOOT), Scope::Table) {
            return;
        }
        self.clear_to_table_body_context();
        self.open.pop();
        self.mode = Mode::InTable;
        self.step(Mode::InTable, tok);
    }

    fn m_in_row(&mut self, tok: Token) {
        match tok {
            Token::StartTag(t) => match t.name {
                a::TH | a::TD => {
                    self.clear_to_table_row_context();
                    self.insert_html_element(t);
                    self.mode = Mode::InCell;
                    self.afe.push(Afe::Marker);
                }
                a::CAPTION | a::COL | a::COLGROUP | a::TBODY | a::TFOOT | a::THEAD | a::TR => self.row_close_and_reprocess(Token::StartTag(t)),
                _ => self.step(Mode::InTable, Token::StartTag(t)),
            },
            Token::EndTag(t) => match t.name {
                a::TR => {
                    if !self.has_in_scope(a::TR, Scope::Table) {
                        return;
                    }
                    self.clear_to_table_row_context();
                    self.open.pop();
                    self.mode = Mode::InTableBody;
                }
                a::TABLE => self.row_close_and_reprocess(Token::EndTag(t)),
                a::TBODY | a::TFOOT | a::THEAD => {
                    if !self.has_in_scope(t.name, Scope::Table) {
                        return;
                    }
                    self.row_close_and_reprocess(Token::EndTag(t));
                }
                a::BODY | a::CAPTION | a::COL | a::COLGROUP | a::HTML | a::TD | a::TH => {}
                _ => self.step(Mode::InTable, Token::EndTag(t)),
            },
            other => self.step(Mode::InTable, other),
        }
    }

    fn row_close_and_reprocess(&mut self, tok: Token) {
        if !self.has_in_scope(a::TR, Scope::Table) {
            return;
        }
        self.clear_to_table_row_context();
        self.open.pop();
        self.mode = Mode::InTableBody;
        self.step(Mode::InTableBody, tok);
    }

    fn close_cell(&mut self) {
        self.generate_implied_end_tags(None);
        self.pop_until_html(|n| matches!(n, a::TD | a::TH));
        self.clear_afe_to_marker();
        self.mode = Mode::InRow;
    }

    fn m_in_cell(&mut self, tok: Token) {
        match tok {
            Token::EndTag(t) => match t.name {
                a::TD | a::TH => {
                    if !self.has_in_scope(t.name, Scope::Table) {
                        return;
                    }
                    self.generate_implied_end_tags(None);
                    self.pop_until_name(t.name);
                    self.clear_afe_to_marker();
                    self.mode = Mode::InRow;
                }
                a::BODY | a::CAPTION | a::COL | a::COLGROUP | a::HTML => {}
                a::TABLE | a::TBODY | a::TFOOT | a::THEAD | a::TR => {
                    if !self.has_in_scope(t.name, Scope::Table) {
                        return;
                    }
                    self.close_cell();
                    self.step(Mode::InRow, Token::EndTag(t));
                }
                _ => self.step(Mode::InBody, Token::EndTag(t)),
            },
            Token::StartTag(t)
                if matches!(t.name, a::CAPTION | a::COL | a::COLGROUP | a::TBODY | a::TD | a::TFOOT | a::TH | a::THEAD | a::TR) =>
            {
                if !self.has_in_scope_by(|n| matches!(n, a::TD | a::TH), Scope::Table) {
                    return;
                }
                self.close_cell();
                self.step(Mode::InRow, Token::StartTag(t));
            }
            other => self.step(Mode::InBody, other),
        }
    }

    fn m_in_template(&mut self, tok: Token) {
        match tok {
            Token::Text(_) | Token::Comment(_) | Token::ProcessingInstruction { .. } | Token::Doctype(_) => self.step(Mode::InBody, tok),
            Token::StartTag(t) => match t.name {
                a::BASE | a::BASEFONT | a::BGSOUND | a::LINK | a::META | a::NOFRAMES | a::SCRIPT | a::STYLE | a::TEMPLATE | a::TITLE => {
                    self.step(Mode::InHead, Token::StartTag(t))
                }
                a::CAPTION | a::COLGROUP | a::TBODY | a::TFOOT | a::THEAD => self.template_switch(Mode::InTable, Token::StartTag(t)),
                a::COL => self.template_switch(Mode::InColumnGroup, Token::StartTag(t)),
                a::TR => self.template_switch(Mode::InTableBody, Token::StartTag(t)),
                a::TD | a::TH => self.template_switch(Mode::InRow, Token::StartTag(t)),
                _ => self.template_switch(Mode::InBody, Token::StartTag(t)),
            },
            Token::EndTag(t) => {
                if t.name == a::TEMPLATE {
                    self.step(Mode::InHead, Token::EndTag(t));
                }
            }
            Token::Eof => {
                // HTML 13.2.6.4.18: pop to the innermost template, reset the insertion mode, reprocess the
                // token. The reprocessing is left to `dispatch_one`, which loops: a recursive call used
                // several stack frames per nested template (600 nested templates overflowed a debug stack).
                if !self.has_template_on_stack() {
                    self.stop();
                    return;
                }
                self.pop_until_name(a::TEMPLATE);
                self.clear_afe_to_marker();
                self.tmpl_modes.pop();
                self.reset_insertion_mode();
                self.reprocess_eof = true;
            }
        }
    }

    fn template_switch(&mut self, mode: Mode, tok: Token) {
        self.tmpl_modes.pop();
        self.tmpl_modes.push(mode);
        self.mode = mode;
        self.step(mode, tok);
    }

    // ----- after body, frameset -----

    fn m_after_body(&mut self, tok: Token) {
        match tok {
            Token::Text(ref s) if is_ws_str(s) => self.step(Mode::InBody, tok),
            Token::Comment(_) | Token::ProcessingInstruction { .. } => {
                let html = self.open.first().copied();
                self.insert_comment_like(tok, html)
            }
            Token::Doctype(_) => {}
            Token::StartTag(t) if t.name == a::HTML => self.step(Mode::InBody, Token::StartTag(t)),
            Token::EndTag(t) if t.name == a::HTML => {
                if self.ctx != NONE {
                    return;
                }
                self.mode = Mode::AfterAfterBody;
            }
            Token::Eof => self.stop(),
            other => {
                self.mode = Mode::InBody;
                self.step(Mode::InBody, other);
            }
        }
    }

    fn m_in_frameset(&mut self, tok: Token) {
        match tok {
            Token::Text(ref s) if is_ws_str(s) => self.insert_char(s),
            Token::Comment(_) | Token::ProcessingInstruction { .. } => self.insert_comment_like(tok, None),
            Token::Doctype(_) => {}
            Token::StartTag(t) => match t.name {
                a::HTML => self.step(Mode::InBody, Token::StartTag(t)),
                a::FRAMESET => {
                    self.insert_html_element(t);
                }
                a::FRAME => {
                    self.insert_void_html_element(t);
                }
                a::NOFRAMES => self.step(Mode::InHead, Token::StartTag(t)),
                _ => {}
            },
            Token::EndTag(t) if t.name == a::FRAMESET => {
                if self.open.len() <= 1 {
                    return;
                }
                self.open.pop();
                if self.ctx == NONE && !self.cur_is(a::FRAMESET) {
                    self.mode = Mode::AfterFrameset;
                }
            }
            Token::Eof => self.stop(),
            _ => {}
        }
    }

    fn m_after_frameset(&mut self, tok: Token) {
        match tok {
            Token::Text(ref s) if is_ws_str(s) => self.insert_char(s),
            Token::Comment(_) | Token::ProcessingInstruction { .. } => self.insert_comment_like(tok, None),
            Token::Doctype(_) => {}
            Token::StartTag(t) => match t.name {
                a::HTML => self.step(Mode::InBody, Token::StartTag(t)),
                a::NOFRAMES => self.step(Mode::InHead, Token::StartTag(t)),
                _ => {}
            },
            Token::EndTag(t) if t.name == a::HTML => self.mode = Mode::AfterAfterFrameset,
            Token::Eof => self.stop(),
            _ => {}
        }
    }

    fn m_after_after_body(&mut self, tok: Token) {
        match tok {
            Token::Comment(_) | Token::ProcessingInstruction { .. } => self.insert_comment_like(tok, Some(DOCUMENT)),
            Token::Doctype(_) => self.step(Mode::InBody, tok),
            Token::Text(ref s) if is_ws_str(s) => self.step(Mode::InBody, tok),
            Token::StartTag(t) if t.name == a::HTML => self.step(Mode::InBody, Token::StartTag(t)),
            Token::Eof => self.stop(),
            other => {
                self.mode = Mode::InBody;
                self.step(Mode::InBody, other);
            }
        }
    }

    fn m_after_after_frameset(&mut self, tok: Token) {
        match tok {
            Token::Comment(_) | Token::ProcessingInstruction { .. } => self.insert_comment_like(tok, Some(DOCUMENT)),
            Token::Doctype(_) => self.step(Mode::InBody, tok),
            Token::Text(ref s) if is_ws_str(s) => self.step(Mode::InBody, tok),
            Token::StartTag(t) if t.name == a::HTML => self.step(Mode::InBody, Token::StartTag(t)),
            Token::Eof => self.stop(),
            Token::StartTag(t) if t.name == a::NOFRAMES => self.step(Mode::InHead, Token::StartTag(t)),
            _ => {}
        }
    }

    // ----- foreign content (13.2.6.5) -----

    fn foreign(&mut self, tok: Token) {
        match tok {
            Token::Text(s) => {
                if s.chars().all(|c| c == '\0') {
                    let repl: String = s.chars().map(|_| '\u{FFFD}').collect();
                    self.insert_char(&repl);
                    return;
                }
                self.insert_char(&s);
                if !is_ws_str(&s) {
                    self.frameset_ok = false;
                }
            }
            Token::Comment(_) | Token::ProcessingInstruction { .. } => self.insert_comment_like(tok, None),
            Token::Doctype(_) | Token::Eof => {}
            Token::StartTag(t) => {
                let breakout = matches!(
                    t.name,
                    a::B | a::BIG | a::BLOCKQUOTE | a::BODY | a::BR | a::CENTER | a::CODE | a::DD | a::DIV | a::DL | a::DT | a::EM
                        | a::EMBED | a::H1 | a::H2 | a::H3 | a::H4 | a::H5 | a::H6 | a::HEAD | a::HR | a::I | a::IMG | a::LI
                        | a::LISTING | a::MENU | a::META | a::NOBR | a::OL | a::P | a::PRE | a::RUBY | a::S | a::SMALL
                        | a::STRONG | a::STRIKE | a::SUB | a::SUP | a::TABLE | a::TT | a::U | a::UL | a::VAR
                ) || t.name == a::SPAN
                    || (t.name == a::FONT && t.attrs.iter().any(|at| matches!(at.name, a::COLOR | a::FACE | a::SIZE)));
                if breakout {
                    self.foreign_breakout(Token::StartTag(t));
                    return;
                }
                let adj = self.adjusted_current();
                let ns = self.doc.element(adj).map_or(Namespace::Html, |e| e.ns);
                let mut name = t.name;
                if ns == Namespace::Svg {
                    let text = self.doc.atoms.get(name).to_string();
                    if let Ok(i) = SVG_TAGS.binary_search_by(|(k, _)| (*k).cmp(text.as_str()))
                        && let Some(&(_, fixed)) = SVG_TAGS.get(i)
                    {
                        name = self.doc.atoms.intern(fixed);
                    }
                }
                let attrs = self.adjust_attrs(t.attrs, ns);
                let (_, pushed) = self.insert_element(name, ns, attrs);
                if t.self_closing {
                    if ns == Namespace::Svg && name == a::SCRIPT {
                        // Acknowledge the flag and act as for a "script" end tag: pop the element.
                        if pushed {
                            self.open.pop();
                        }
                    } else if pushed {
                        self.open.pop();
                    }
                }
            }
            Token::EndTag(t) => {
                if matches!(t.name, a::BR | a::P) {
                    self.foreign_breakout(Token::EndTag(t));
                    return;
                }
                let cur = self.cur();
                if self.doc.element(cur).is_some_and(|e| e.ns == Namespace::Svg && e.name == a::SCRIPT) && t.name == a::SCRIPT {
                    self.open.pop();
                    return;
                }
                let mut i = self.open.len();
                if i == 0 {
                    return;
                }
                i -= 1;
                let tag = self.doc.atoms.get(t.name).to_string();
                let lower = |s: &Self, id: NodeId| s.doc.element(id).map(|e| s.doc.atoms.get(e.name).to_ascii_lowercase());
                // (parse error if the current node's name does not match)
                loop {
                    let node = self.open.get(i).copied().unwrap_or(NONE);
                    if i == 0 {
                        return;
                    }
                    if lower(self, node).as_deref() == Some(tag.as_str()) {
                        self.open.truncate(i);
                        return;
                    }
                    i -= 1;
                    let next = self.open.get(i).copied().unwrap_or(NONE);
                    if self.doc.element(next).is_some_and(|e| e.ns != Namespace::Html) {
                        continue;
                    }
                    break;
                }
                self.step(self.mode, Token::EndTag(t));
            }
        }
    }

    fn foreign_breakout(&mut self, tok: Token) {
        while let Some(&n) = self.open.last() {
            let keep = self.is_mathml_text_ip(n)
                || self.is_html_ip(n)
                || self.doc.element(n).is_some_and(|e| e.ns == Namespace::Html);
            if keep || self.open.len() <= 1 {
                break;
            }
            self.open.pop();
        }
        self.step(self.mode, tok);
    }
}

/// Quirks mode for a DOCTYPE token (13.2.6.4.1).
fn quirks_mode_for(d: &DoctypeToken) -> QuirksMode {
    if d.force_quirks || d.name.as_deref() != Some("html") {
        return QuirksMode::Quirks;
    }
    let public = d.public_id.as_deref().map(str::to_ascii_lowercase);
    let system = d.system_id.as_deref().map(str::to_ascii_lowercase);
    let starts = |list: &[&str]| public.as_deref().is_some_and(|p| list.iter().any(|x| p.starts_with(&x.to_ascii_lowercase())));
    let public_is = |list: &[&str]| public.as_deref().is_some_and(|p| list.iter().any(|x| p == x.to_ascii_lowercase()));
    let system_is = |list: &[&str]| system.as_deref().is_some_and(|s| list.iter().any(|x| s == x.to_ascii_lowercase()));
    let system_missing = system.as_deref().is_none_or(str::is_empty);
    if public_is(q::QUIRKS_PUBLIC_EXACT)
        || system_is(q::QUIRKS_SYSTEM_EXACT)
        || starts(q::QUIRKS_PUBLIC_PREFIX)
        || (system_missing && starts(q::QUIRKS_PUBLIC_PREFIX_IF_SYSTEM_MISSING))
    {
        return QuirksMode::Quirks;
    }
    if starts(q::LIMITED_PUBLIC_PREFIX) || (!system_missing && starts(q::LIMITED_PUBLIC_PREFIX_IF_SYSTEM_PRESENT)) {
        return QuirksMode::LimitedQuirks;
    }
    QuirksMode::NoQuirks
}
