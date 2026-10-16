//! The box tree (L1 task 9): boxes made from the DOM and the computed styles.
//!
//! * Boxes follow `display` (css-display-3 2). `display: none` makes none; `display: contents` makes
//!   none for the element itself and its children take its place.
//! * Block-level boxes that sit among inline-level siblings put the inline ones into anonymous block
//!   boxes (CSS2 9.2.1.1); an inline box that contains a block-level box is split around it.
//! * Whitespace-only text between blocks makes no box (CSS2 9.2.2.1; css-text-3 4.1.1 note).
//! * Fallbacks for what layer 1 does not lay out: `flex`, `grid`, `table*` become `block`; their
//!   `inline-` forms become atomic inlines (`inline-block`); `float` is ignored (the element stays in
//!   the normal flow).
//! * `inline-block` and its fallbacks are atomic inline-level boxes ([`BoxKind::InlineBlock`]); `img`,
//!   `iframe`, `video`, `audio`, `canvas`, `object`, `embed` and some form controls are
//!   [`BoxKind::Replaced`] leaves; text fields, text areas, selects and buttons are `InlineBlock`
//!   (or `Block`) containers whose text is synthesised from the attributes (L1 task 10, form controls).
//! * `position: absolute / fixed` boxes are out of flow: they stay in their parent's child list as
//!   inline-level placeholders (so that the static position can be found) and never split an inline box.
//! * List items get their ordinal here (HTML 4.4.8: `<ol start reversed>`, `<li value>`).

use std::rc::Rc;

use super::image::{ImageRequest, ImageSizes, ImageState, SelectEnv, select_source};
use crate::css::cascade::StyleTree;
use crate::css::color::Color;
use crate::css::computed::{BoxStyle, ComputedStyle};
use crate::css::types::{Dim, Display, Position, TextWrapMode, WhiteSpaceCollapse};
use crate::css::values::Lp;
use crate::html::atoms::{Atom, FxHashMap, a};
use crate::html::dom::{Document, NONE, Namespace, NodeData, NodeId};

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
    /// The text of one DOM text node (or synthesised text: alt text, input values).
    Text,
    /// A `<br>`: a forced line break.
    LineBreak,
    /// An atomic inline-level box with block content (`inline-block`, text fields, buttons ...).
    InlineBlock,
    /// A replaced element: image, frame, plug-in placeholder, check box ...
    Replaced,
}

/// What a replaced box shows.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ReplacedKind {
    /// `<img>` or `<input type=image>`: the image of `LayoutBox::image`.
    Image,
    /// A grey placeholder with the element name written in it (`iframe`, `video` ...).
    Placeholder(&'static str),
    /// A check box or radio button (13x13).
    Check,
    /// A control drawn as a fixed-size box (`meter`, `progress`, `input type=range` ...).
    Fixed(f32, f32, &'static str),
}

/// Form controls whose intrinsic size does not come from their text.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Control {
    None,
    /// A one-line text field of `size` characters.
    TextInput(u32),
    /// A text area of `cols` x `rows` characters.
    TextArea(u32, u32),
    /// A drop-down list: one line, showing the selected option.
    Select,
}

/// The chosen source of an image (`srcset`/`sizes` already applied).
#[derive(Clone, Debug, PartialEq)]
pub struct ImageSource {
    pub src: String,
    pub density: f32,
}

/// Which part of an inline element a box is, when a block-level descendant split it in pieces
/// (CSS2 9.2.1.1). Padding, border and margin of the element belong to the first piece's start
/// and the last piece's end.
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
    /// The computed `display` when a fallback replaced it (flex, grid, table parts ...).
    pub fallback: Option<Display>,
    pub split: SplitPart,
    /// `position: absolute | fixed`: out of flow.
    pub oof: bool,
    /// Document order of the element (pre-order); pieces of a split element share it.
    pub order: u32,
    /// What only a few boxes have (replaced elements, controls, made-up text, list items), out of line so the
    /// common box stays small. Read it through `replaced()`, `control()`, `synth()`, `ordinal()` and `image()`.
    pub rare: Option<Box<Rare>>,
    /// Content is clipped to the padding box regardless of `overflow` (text fields).
    pub force_clip: bool,
}

/// The parts of a [`LayoutBox`] that most boxes do not have.
#[derive(Clone, Debug)]
pub struct Rare {
    pub replaced: Option<ReplacedKind>,
    pub control: Control,
    /// Text made up by layout (alt text, field values) instead of coming from a DOM text node.
    pub synth: Option<Rc<str>>,
    /// The number of a list item (shown in its marker).
    pub ordinal: Option<i32>,
    pub image: Option<ImageSource>,
}

impl Default for Rare {
    fn default() -> Self {
        Rare { replaced: None, control: Control::None, synth: None, ordinal: None, image: None }
    }
}

impl LayoutBox {
    fn new(kind: BoxKind, node: NodeId, style: Rc<ComputedStyle>, anonymous: bool) -> LayoutBox {
        LayoutBox {
            kind,
            node,
            style,
            parent: NO_BOX,
            children: Vec::new(),
            anonymous,
            inline_children: false,
            fallback: None,
            split: SplitPart::default(),
            oof: false,
            order: 0,
            rare: None,
            force_clip: false,
        }
    }

    pub fn replaced(&self) -> Option<ReplacedKind> {
        self.rare.as_ref().and_then(|r| r.replaced)
    }

    pub fn control(&self) -> Control {
        self.rare.as_ref().map_or(Control::None, |r| r.control)
    }

    pub fn synth(&self) -> Option<&Rc<str>> {
        self.rare.as_ref().and_then(|r| r.synth.as_ref())
    }

    pub fn ordinal(&self) -> Option<i32> {
        self.rare.as_ref().and_then(|r| r.ordinal)
    }

    pub fn image(&self) -> Option<&ImageSource> {
        self.rare.as_ref().and_then(|r| r.image.as_ref())
    }

    fn rare_mut(&mut self) -> &mut Rare {
        self.rare.get_or_insert_with(Box::default)
    }

    /// Is this box an atomic inline-level box (laid out as a unit inside a line)?
    pub fn is_atomic_inline(&self) -> bool {
        match self.kind {
            BoxKind::InlineBlock => true,
            BoxKind::Replaced => self.style.boxs.display != Display::Block && !matches!(classify(self.style.boxs.display), Gen::Block(_)),
            _ => false,
        }
    }
}

pub struct BoxTree {
    pub boxes: Vec<LayoutBox>,
    pub root: BoxId,
    /// The sources of the images of the page, in document order.
    pub images: Vec<ImageRequest>,
    dummy: LayoutBox,
}

impl BoxTree {
    /// The box with this id. Out-of-range ids give an empty dummy block (never happens for ids the
    /// builder made; this keeps the engine free of panics on any input).
    pub fn get(&self, id: BoxId) -> &LayoutBox {
        self.boxes.get(id as usize).unwrap_or(&self.dummy)
    }

    /// The box whose coordinates `id`'s are relative to: the nearest ancestor that is not an inline box.
    pub fn layout_parent(&self, id: BoxId) -> BoxId {
        let mut p = self.get(id).parent;
        while p != NO_BOX && self.get(p).kind == BoxKind::Inline {
            p = self.get(p).parent;
        }
        p
    }
}

/// What the builder needs besides the DOM and the styles.
pub struct BuildEnv<'a> {
    pub viewport_w: f32,
    pub viewport_h: f32,
    pub dpr: f32,
    pub images: &'a ImageSizes,
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
    Atomic(Option<Display>),
}

fn classify(d: Display) -> Gen {
    match d {
        Display::None => Gen::Nothing,
        Display::Contents => Gen::Contents,
        Display::Block | Display::ListItem | Display::FlowRoot => Gen::Block(None),
        Display::Inline => Gen::Inline(None),
        Display::InlineBlock => Gen::Atomic(None),
        Display::InlineTable | Display::InlineFlex | Display::InlineGrid => Gen::Atomic(Some(d)),
        Display::Table
        | Display::Flex
        | Display::Grid
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

/// What an element becomes beyond the plain box of its `display`.
enum Special {
    Plain,
    Replaced(ReplacedKind),
    Image(Option<ImageSource>),
    /// An image that is not there yet and has no size: its alt text, as an inline box.
    Alt(String),
    /// A form control with a box of its own: the text it shows and whether that is a placeholder.
    Container(Control, Option<(String, bool)>),
}

struct ListCounter {
    next: i32,
    step: i32,
}

struct Builder<'a> {
    doc: &'a Document,
    styles: &'a StyleTree,
    env: &'a BuildEnv<'a>,
    boxes: Vec<LayoutBox>,
    init: Rc<ComputedStyle>,
    next_order: u32,
    counters: FxHashMap<NodeId, ListCounter>,
    images: Vec<ImageRequest>,
    video: Option<Atom>,
    audio: Option<Atom>,
    canvas: Option<Atom>,
    meter: Option<Atom>,
    progress: Option<Atom>,
}

/// Build the box tree of a styled document (no image sizes known).
pub fn build_box_tree(doc: &Document, styles: &StyleTree) -> BoxTree {
    let sizes = ImageSizes::new();
    build_box_tree_with(doc, styles, &BuildEnv { viewport_w: 1280.0, viewport_h: 1000.0, dpr: 1.0, images: &sizes })
}

pub fn build_box_tree_with(doc: &Document, styles: &StyleTree, env: &BuildEnv<'_>) -> BoxTree {
    let init = Rc::new(ComputedStyle::initial());
    let mut b = Builder {
        doc,
        styles,
        env,
        boxes: Vec::new(),
        init: init.clone(),
        next_order: 0,
        counters: FxHashMap::default(),
        images: Vec::new(),
        video: doc.atoms.lookup("video"),
        audio: doc.atoms.lookup("audio"),
        canvas: doc.atoms.lookup("canvas"),
        meter: doc.atoms.lookup("meter"),
        progress: doc.atoms.lookup("progress"),
    };
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
    let dummy = LayoutBox::new(BoxKind::Block, NONE, init, true);
    BoxTree { boxes: b.boxes, root, images: b.images, dummy }
}

/// HTML 2.3.4.? "rules for parsing integers" (leading white space, sign, digits; the rest is ignored).
pub(crate) fn parse_html_int(s: &str) -> Option<i32> {
    let t = s.trim_start_matches(|c: char| c.is_ascii_whitespace());
    let (neg, rest) = match t.as_bytes().first() {
        Some(b'-') => (true, t.get(1..)?),
        Some(b'+') => (false, t.get(1..)?),
        _ => (false, t),
    };
    let digits: String = rest.chars().take_while(char::is_ascii_digit).take(12).collect();
    if digits.is_empty() {
        return None;
    }
    let v: i64 = digits.parse().ok()?;
    let v = if neg { -v } else { v };
    Some(v.clamp(i32::MIN as i64, i32::MAX as i64) as i32)
}

fn collapse_ws(s: &str) -> String {
    s.split_ascii_whitespace().collect::<Vec<_>>().join(" ")
}

impl Builder<'_> {
    fn new_box(&mut self, kind: BoxKind, node: NodeId, style: Rc<ComputedStyle>, anonymous: bool) -> BoxId {
        let id = self.boxes.len() as BoxId;
        self.boxes.push(LayoutBox::new(kind, node, style, anonymous));
        id
    }

    fn box_mut(&mut self, id: BoxId) -> Option<&mut LayoutBox> {
        self.boxes.get_mut(id as usize)
    }

    fn attr(&self, n: NodeId, name: &str) -> Option<&str> {
        self.doc.atoms.lookup(name).and_then(|a| self.doc.attr(n, a))
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

    /// A text box with text layout made up. `single_line` keeps the text on one line, as in a text field.
    fn synth_text(&mut self, text: &str, base: &Rc<ComputedStyle>, gray: bool, single_line: bool) -> Out {
        let mut t = (*base.text).clone();
        if single_line {
            t.white_space_collapse = WhiteSpaceCollapse::Preserve;
            t.text_wrap_mode = TextWrapMode::Nowrap;
        }
        if gray {
            t.color = Color::rgb(0x75, 0x75, 0x75);
        }
        let style = Rc::new(ComputedStyle { text: Rc::new(t), ..(**base).clone() });
        let id = self.new_box(BoxKind::Text, NONE, style, true);
        if let Some(b) = self.box_mut(id) {
            b.rare_mut().synth = Some(Rc::from(text));
        }
        Out::Inline(id)
    }

    fn next_ordinal(&mut self, n: NodeId) -> i32 {
        let doc = self.doc;
        let parent = doc.parent(n);
        let is_li = doc.is_html_element(n, a::LI);
        let ctr = self.counters.entry(parent).or_insert_with(|| {
            let ol = doc.is_html_element(parent, a::OL);
            let at = |name: &str| doc.atoms.lookup(name).and_then(|a| doc.attr(parent, a));
            let reversed = ol && at("reversed").is_some();
            let step = if reversed { -1 } else { 1 };
            let start = if ol { at("start").and_then(parse_html_int) } else { None };
            let start = start.unwrap_or_else(|| {
                if reversed { doc.children(parent).filter(|&c| doc.is_html_element(c, a::LI)).count().min(i32::MAX as usize) as i32 } else { 1 }
            });
            ListCounter { next: start, step }
        });
        let value = if is_li { doc.atoms.lookup("value").and_then(|a| doc.attr(n, a)).and_then(parse_html_int) } else { None };
        let ord = value.unwrap_or(ctr.next);
        ctr.next = ord.saturating_add(ctr.step);
        ord
    }

    /// Decide what an element is besides its `display`: replaced element, control, image with alt text.
    fn special(&mut self, n: NodeId, name: Atom, style: &Rc<ComputedStyle>) -> Special {
        let doc = self.doc;
        let is = |o: Option<Atom>| o == Some(name);
        if name == a::IMG {
            return self.image_special(n, style);
        }
        if name == a::IFRAME {
            return Special::Replaced(ReplacedKind::Placeholder("iframe"));
        }
        if name == a::EMBED {
            return Special::Replaced(ReplacedKind::Placeholder("embed"));
        }
        if name == a::OBJECT {
            return Special::Replaced(ReplacedKind::Placeholder("object"));
        }
        if is(self.video) {
            return Special::Replaced(ReplacedKind::Placeholder("video"));
        }
        if is(self.audio) {
            return Special::Replaced(ReplacedKind::Placeholder("audio"));
        }
        if is(self.canvas) {
            return Special::Replaced(ReplacedKind::Placeholder("canvas"));
        }
        if is(self.meter) {
            return Special::Replaced(ReplacedKind::Fixed(80.0, 16.0, "meter"));
        }
        if is(self.progress) {
            return Special::Replaced(ReplacedKind::Fixed(160.0, 16.0, "progress"));
        }
        if name == a::TEXTAREA {
            let dim = |k: &str, d: u32| self.attr(n, k).and_then(parse_html_int).filter(|v| *v > 0).map_or(d, |v| (v as u32).min(10_000));
            return Special::Container(Control::TextArea(dim("cols", 20), dim("rows", 2)), None);
        }
        if name == a::SELECT {
            // The text of the selected option, else of the first one (options inside `optgroup` too).
            let mut stack: Vec<NodeId> = doc.children(n).collect();
            stack.reverse();
            let (mut selected, mut first): (Option<String>, Option<String>) = (None, None);
            while let Some(c) = stack.pop() {
                if doc.is_html_element(c, a::OPTION) {
                    let t = collapse_ws(&doc.text_content(c));
                    if self.attr(c, "selected").is_some() {
                        selected = Some(t);
                        break;
                    }
                    if first.is_none() {
                        first = Some(t);
                    }
                } else if doc.is_html_element(c, a::OPTGROUP) {
                    let mut kids: Vec<NodeId> = doc.children(c).collect();
                    kids.reverse();
                    stack.extend(kids);
                }
            }
            return Special::Container(Control::Select, Some((selected.or(first).unwrap_or_default(), false)));
        }
        if name == a::INPUT {
            let ty = self.attr(n, "type").map(str::to_ascii_lowercase).unwrap_or_default();
            let value = self.attr(n, "value").map(str::to_string);
            return match ty.as_str() {
                "checkbox" | "radio" => Special::Replaced(ReplacedKind::Check),
                "image" => self.image_special(n, style),
                "range" => Special::Replaced(ReplacedKind::Fixed(129.0, 16.0, "range")),
                "color" => Special::Replaced(ReplacedKind::Fixed(50.0, 27.0, "color")),
                "submit" => Special::Container(Control::None, Some((value.unwrap_or_else(|| "Submit".into()), false))),
                "reset" => Special::Container(Control::None, Some((value.unwrap_or_else(|| "Reset".into()), false))),
                "button" => Special::Container(Control::None, Some((value.unwrap_or_default(), false))),
                "file" => Special::Container(Control::None, Some(("Choose File  No file chosen".into(), false))),
                _ => {
                    let size = self.attr(n, "size").and_then(parse_html_int).filter(|v| *v > 0).map_or(20, |v| (v as u32).min(10_000));
                    let text = match value.filter(|v| !v.is_empty()) {
                        Some(v) if ty == "password" => Some(("\u{2022}".repeat(v.chars().count().min(10_000)), false)),
                        Some(v) => Some((v, false)),
                        None => self.attr(n, "placeholder").filter(|p| !p.is_empty()).map(|p| (p.to_string(), true)),
                    };
                    Special::Container(Control::TextInput(size), text)
                }
            };
        }
        Special::Plain
    }

    fn image_special(&mut self, n: NodeId, style: &Rc<ComputedStyle>) -> Special {
        let senv = SelectEnv { viewport_w: self.env.viewport_w, viewport_h: self.env.viewport_h, dpr: self.env.dpr };
        let source = select_source(self.doc, n, &senv).map(|(src, density)| ImageSource { src, density });
        if let Some(s) = &source {
            self.images.push(ImageRequest { node: n, src: s.src.clone(), density: s.density });
        }
        let loaded = source.is_some() && matches!(self.env.images.get(n), Some(ImageState::Loaded { .. }));
        if loaded {
            return Special::Image(source);
        }
        let sized = |d: &Dim| matches!(d, Dim::Lp(_));
        let has_dims = sized(&style.boxs.width) || sized(&style.boxs.height);
        let alt = self.attr(n, "alt").map(str::trim).filter(|t| !t.is_empty());
        match alt {
            Some(t) if !has_dims => Special::Alt(t.to_string()),
            _ => Special::Image(source),
        }
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
                let order = self.next_order;
                self.next_order = self.next_order.wrapping_add(1);
                let display = style.boxs.display;
                let mut kind = classify(display);
                if depth >= MAX_BOX_DEPTH && !matches!(kind, Gen::Nothing) {
                    kind = Gen::Contents;
                }
                let oof = matches!(style.boxs.position, Position::Absolute | Position::Fixed);
                // A float is a placeholder in the inline content around it, like an out-of-flow box (floats.rs).
                let floated = !oof && style.boxs.float != crate::css::types::Float::None;
                let special = if matches!(kind, Gen::Nothing | Gen::Contents) { Special::Plain } else { self.special(n, e.name, &style) };
                match special {
                    Special::Replaced(rk) => {
                        let id = self.make_replaced(n, style, rk, None, order);
                        outs.push(self.level_out(id, &kind, oof || floated));
                        return;
                    }
                    Special::Image(src) => {
                        let id = self.make_replaced(n, style, ReplacedKind::Image, src, order);
                        outs.push(self.level_out(id, &kind, oof || floated));
                        return;
                    }
                    Special::Alt(text) => {
                        let child = self.synth_text(&text, &style, false, false);
                        let id = self.new_box(BoxKind::Inline, n, style, false);
                        if let Out::Inline(c) = child {
                            self.adopt(id, vec![c]);
                        }
                        self.set_order(id, order, false);
                        outs.push(Out::Inline(id));
                        return;
                    }
                    Special::Container(ctl, text) => {
                        let child_outs = match (&text, ctl) {
                            (Some((t, gray)), _) => {
                                if t.is_empty() {
                                    Vec::new()
                                } else {
                                    let single = !matches!(ctl, Control::None);
                                    vec![self.synth_text(t, &style, *gray, single)]
                                }
                            }
                            (None, Control::TextArea(..)) => self.gen_children(n, &style, depth),
                            _ => Vec::new(),
                        };
                        let block_level = matches!(kind, Gen::Block(_));
                        let bk = if block_level { BoxKind::Block } else { BoxKind::InlineBlock };
                        let fallback = match kind {
                            Gen::Block(f) | Gen::Atomic(f) | Gen::Inline(f) => f,
                            _ => None,
                        };
                        let id = self.new_box(bk, n, style, false);
                        if let Some(b) = self.box_mut(id) {
                            if ctl != Control::None {
                                b.rare_mut().control = ctl;
                            }
                            b.force_clip = matches!(ctl, Control::TextInput(_) | Control::Select);
                            b.fallback = fallback;
                        }
                        self.set_order(id, order, oof);
                        self.finish_block(id, child_outs);
                        outs.push(if oof || floated || !block_level { Out::Inline(id) } else { Out::Block(id) });
                        return;
                    }
                    Special::Plain => {}
                }
                match kind {
                    Gen::Nothing => {}
                    Gen::Contents => {
                        for c in doc.children(n) {
                            self.gen_node(c, &style, depth + 1, outs);
                        }
                    }
                    Gen::Block(fallback) | Gen::Atomic(fallback) => {
                        let block_level = matches!(kind, Gen::Block(_));
                        let ordinal = (display == Display::ListItem).then(|| self.next_ordinal(n));
                        let child_outs = self.gen_children(n, &style, depth);
                        let bk = if block_level { BoxKind::Block } else { BoxKind::InlineBlock };
                        let id = self.new_box(bk, n, style, false);
                        if let Some(b) = self.box_mut(id) {
                            b.fallback = fallback;
                            if ordinal.is_some() {
                                b.rare_mut().ordinal = ordinal;
                            }
                        }
                        self.set_order(id, order, oof);
                        self.finish_block(id, child_outs);
                        outs.push(if oof || floated || !block_level { Out::Inline(id) } else { Out::Block(id) });
                    }
                    Gen::Inline(fallback) => {
                        if e.name == a::BR {
                            let id = self.new_box(BoxKind::LineBreak, n, style, false);
                            self.set_order(id, order, false);
                            outs.push(Out::Inline(id));
                            return;
                        }
                        let child_outs = self.gen_children(n, &style, depth);
                        let first = self.boxes.len();
                        self.finish_inline(n, style, fallback, child_outs, outs);
                        for b in self.boxes.iter_mut().skip(first) {
                            if b.node == n {
                                b.order = order;
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn set_order(&mut self, id: BoxId, order: u32, oof: bool) {
        if let Some(b) = self.box_mut(id) {
            b.order = order;
            b.oof = oof;
        }
    }

    fn make_replaced(&mut self, n: NodeId, style: Rc<ComputedStyle>, rk: ReplacedKind, image: Option<ImageSource>, order: u32) -> BoxId {
        let oof = matches!(style.boxs.position, Position::Absolute | Position::Fixed);
        let id = self.new_box(BoxKind::Replaced, n, style, false);
        if let Some(b) = self.box_mut(id) {
            let r = b.rare_mut();
            r.replaced = Some(rk);
            r.image = image;
        }
        self.set_order(id, order, oof);
        id
    }

    /// A replaced box is block-level when its `display` is (and it is in flow).
    fn level_out(&self, id: BoxId, kind: &Gen, oof: bool) -> Out {
        if !oof && matches!(kind, Gen::Block(_)) { Out::Block(id) } else { Out::Inline(id) }
    }

    /// An inline element: one box, or several when block-level boxes inside split it.
    fn finish_inline(&mut self, n: NodeId, style: Rc<ComputedStyle>, fallback: Option<Display>, child_outs: Vec<Out>, outs: &mut Vec<Out>) {
        // Out-of-flow boxes are inline-level placeholders: only in-flow blocks split an inline box.
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
        // Split around the block-level children. Pieces without children are dropped.
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

    /// Boxes that make nothing visible and no line: collapsible whitespace and empty inline boxes
    /// (an inline box with horizontal padding, border or margin is not ignorable).
    fn ignorable(&self, id: BoxId) -> bool {
        let Some(b) = self.boxes.get(id as usize) else { return true };
        match b.kind {
            BoxKind::Text => {
                let collapse = b.style.text.white_space_collapse;
                if let Some(s) = b.synth() {
                    return is_collapsible_whitespace(s, collapse);
                }
                self.doc.node(b.node).is_some_and(|n| match &n.data {
                    NodeData::Text(t) => is_collapsible_whitespace(t, collapse),
                    _ => false,
                })
            }
            BoxKind::Inline => !has_horizontal_edges(&b.style) && b.children.iter().all(|&c| self.ignorable(c)),
            BoxKind::LineBreak | BoxKind::Block | BoxKind::InlineBlock | BoxKind::Replaced => false,
        }
    }
}

/// Does an inline element have horizontal padding, border or margin (so that it takes room even when empty)?
pub(super) fn has_horizontal_edges(s: &ComputedStyle) -> bool {
    let margin = |i: usize| s.boxs.margin.get(i).is_some_and(|d| !matches!(d, Dim::Lp(l) if l.as_px() == Some(0.0)));
    let pad = |i: usize| s.boxs.padding.get(i).is_some_and(|l: &Lp| l.as_px() != Some(0.0));
    let border = |i: usize| s.border.width.get(i).is_some_and(|w| *w > 0.0);
    margin(1) || margin(3) || pad(1) || pad(3) || border(1) || border(3)
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
