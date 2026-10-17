//! Inline formatting: white-space processing (css-text-3 4.1), shaping, line breaking (css-text-3 5),
//! line boxes (CSS2 10.8, css-inline-3), `vertical-align`, inline box edges, atomic inlines,
//! `text-align` (incl. `justify`, css-text-3 7), `text-indent`, spacing, `text-transform` and
//! decoration lines.
//!
//! One block container whose children are inline-level is one *paragraph*:
//!
//! 1. `collect` walks the inline boxes and turns the content into *items*: text (one per text box,
//!    split at forced breaks, white space collapsed -- Phase I, 4.1.1 and 4.1.3), forced breaks, the
//!    start and end edges of inline boxes that have horizontal padding/border/margin, atomic inlines
//!    and out-of-flow placeholders.
//! 2. Each item is cut into *units*, one per glyph cluster for text: the smallest piece a line may be
//!    cut into. (`prepare`; the result is `Prepared`, which does not depend on the width.)
//! 3. The platform reports the UAX #14 break opportunities of the whole paragraph text; `opp_before`
//!    adds `word-break` and `white-space` on top, `break_line` applies `overflow-wrap`.
//! 4. Lines are filled greedily; each line is trimmed (Phase II, 4.1.2), aligned, justified, its
//!    boxes are aligned vertically and its fragments are emitted (`lay_lines`).
//!
//! `measure_inline` gives the min-content and max-content widths of a paragraph from the same units.

use std::collections::HashMap;
use std::rc::Rc;

use super::block::{Flow, border_px, lp_has_percent, pad_px, resolve_margin};
use super::boxtree::{BoxId, BoxKind, Control, NO_BOX};
use super::units::LayoutUnit as LU;
use super::{DecorKind, Decoration, FragKind, Fragment, Layouter, Line, Pending, ResolvedFont};
use crate::css::color::Color;
use crate::css::computed::{ComputedStyle, LineHeight, slant_of};
use crate::css::types::{
    ListStylePosition, OverflowWrap, Position, TabSize, TextAlign, TextDecorationLine, TextDecorationStyle, TextTransform, TextWrapMode, VerticalAlign,
    WhiteSpaceCollapse, WordBreak,
};
use crate::html::dom::NodeData;
use crate::platform::FontQuery;
use crate::textclass::{is_cjk_break, is_closing_punct, is_east_asian_wide_non_hangul, is_opening_punct, is_space_like};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ItemKind {
    Text,
    Forced,
    Atomic,
    /// The start / end edge (margin + border + padding) of an inline box.
    Start,
    End,
    /// An out-of-flow placeholder.
    Oof,
    /// A float's place in the text.
    Float,
}

#[derive(Clone, Copy)]
struct DecorSpec {
    kind: DecorKind,
    style: TextDecorationStyle,
    color: Color,
    thickness: Option<f32>,
}

/// A run of text of one text box, a forced line break, an edge, an atomic inline or a placeholder.
struct Item {
    kind: ItemKind,
    /// The text after white-space collapsing; empty for everything but text.
    text: String,
    /// The text box, `<br>`, inline box, atomic inline or placeholder.
    box_id: BoxId,
    /// The inline box (or block) the item sits in; its chain of ancestors up to the paragraph's
    /// block are the inline boxes that appear on the lines of this item.
    owner: BoxId,
    style: Rc<ComputedStyle>,
    /// Spaces are collapsible (`white-space-collapse: collapse | preserve-breaks`).
    collapsible: bool,
    wrap: bool,
    break_spaces: bool,
    word_break: WordBreak,
    /// `overflow-wrap: anywhere | break-word` or `word-break: break-word`.
    emergency: bool,
    /// The text of an inside list marker.
    marker: bool,
    decor: Vec<DecorSpec>,
    /// Width of an edge item.
    edge: LU,
}

fn make_item(kind: ItemKind, box_id: BoxId, owner: BoxId, style: &Rc<ComputedStyle>, text: String) -> Item {
    let t = &style.text;
    Item {
        kind,
        text,
        box_id,
        owner,
        style: style.clone(),
        collapsible: matches!(t.white_space_collapse, WhiteSpaceCollapse::Collapse | WhiteSpaceCollapse::PreserveBreaks),
        wrap: t.text_wrap_mode == TextWrapMode::Wrap,
        break_spaces: t.white_space_collapse == WhiteSpaceCollapse::BreakSpaces,
        word_break: t.word_break,
        emergency: matches!(t.overflow_wrap, OverflowWrap::Anywhere | OverflowWrap::BreakWord) || t.word_break == WordBreak::BreakWord,
        marker: false,
        decor: Vec::new(),
        edge: LU::ZERO,
    }
}

/// State of white-space collapsing across the whole paragraph (Phase I looks across inline boundaries).
#[derive(Default)]
struct Collapser {
    /// The last character that is not a collapsible space (for the segment break rule).
    last_char: Option<char>,
    /// The last thing written was a collapsible space.
    last_space_collapsible: bool,
    /// A collapsible segment break was seen; whether it becomes a space is decided by the next character.
    pending_break: bool,
    /// Right after a preserved line feed in `pre-line`: following collapsible spaces are removed.
    skip_spaces: bool,
    /// The previous character was a letter or digit (for `text-transform: capitalize`).
    cap_prev: bool,
}

impl Collapser {
    fn reset_line(&mut self) {
        let cap = self.cap_prev;
        *self = Collapser { cap_prev: cap, ..Collapser::default() };
    }

    /// "Any sequence of collapsible spaces and tabs immediately preceding ... a segment break is
    /// removed" (4.1.1): take back the space just written.
    fn drop_trailing_space(&mut self, items: &mut [Item], cur: &mut String) {
        if !self.last_space_collapsible {
            return;
        }
        if cur.ends_with(' ') {
            cur.pop();
        } else if let Some(it) = items.iter_mut().rev().find(|i| i.kind == ItemKind::Text && !i.text.is_empty())
            && it.text.ends_with(' ')
        {
            it.text.pop();
        }
        self.last_space_collapsible = false;
    }

    /// Segment break transformation (4.1.3 and css-text-4 4.1.3): a collapsible line feed becomes a
    /// space, except between two East Asian wide characters (Chinese and Japanese text has no spaces
    /// between words) or next to a zero width space, where it is removed.
    fn break_needs_space(&self, next: char) -> bool {
        let Some(prev) = self.last_char else { return false };
        if prev == '\u{200B}' || next == '\u{200B}' {
            return false;
        }
        !(is_east_asian_wide_non_hangul(prev) && is_east_asian_wide_non_hangul(next))
    }
}

/// `text-transform` (css-text-3 2.1). `prev_letter` carries the word state of `capitalize`.
fn transform_text(text: &str, mode: TextTransform, prev_letter: &mut bool) -> String {
    match mode {
        TextTransform::None => text.to_string(),
        TextTransform::Uppercase => text.to_uppercase(),
        TextTransform::Lowercase => text.to_lowercase(),
        TextTransform::Capitalize => {
            let mut out = String::with_capacity(text.len());
            for ch in text.chars() {
                let word = ch.is_alphanumeric();
                if word && !*prev_letter {
                    out.extend(ch.to_uppercase());
                } else {
                    out.push(ch);
                }
                *prev_letter = word;
            }
            out
        }
        // ASCII to fullwidth forms, the space to the ideographic space (css-text-3 2.1 "full-width").
        TextTransform::FullWidth => text
            .chars()
            .map(|c| match c {
                ' ' => '\u{3000}',
                '\u{21}'..='\u{7E}' => char::from_u32(c as u32 + 0xFEE0).unwrap_or(c),
                c => c,
            })
            .collect(),
        // Small kana to their full-size forms.
        TextTransform::FullSizeKana => text
            .chars()
            .map(|c| match c {
                'ぁ' | 'ぃ' | 'ぅ' | 'ぇ' | 'ぉ' | 'っ' | 'ゃ' | 'ゅ' | 'ょ' | 'ゎ' => char::from_u32(c as u32 + 1).unwrap_or(c),
                'ァ' | 'ィ' | 'ゥ' | 'ェ' | 'ォ' | 'ッ' | 'ャ' | 'ュ' | 'ョ' | 'ヮ' => char::from_u32(c as u32 + 1).unwrap_or(c),
                'ヵ' => 'カ',
                'ヶ' => 'ケ',
                c => c,
            })
            .collect(),
    }
}

/// The context `add_text` needs about the text box it is reading.
struct TextCx<'a> {
    bid: BoxId,
    owner: BoxId,
    style: &'a Rc<ComputedStyle>,
    decor: &'a [DecorSpec],
    marker: bool,
}

fn push_text(st: &mut Collapser, items: &mut Vec<Item>, cx: &TextCx<'_>, cur: &mut String) {
    if cur.is_empty() {
        return;
    }
    let raw = std::mem::take(cur);
    let text = if cx.style.text.text_transform == TextTransform::None {
        raw
    } else {
        transform_text(&raw, cx.style.text.text_transform, &mut st.cap_prev)
    };
    let mut it = make_item(ItemKind::Text, cx.bid, cx.owner, cx.style, text);
    it.marker = cx.marker;
    it.decor = cx.decor.to_vec();
    items.push(it);
}

fn push_forced(items: &mut Vec<Item>, cx: &TextCx<'_>) {
    items.push(make_item(ItemKind::Forced, cx.bid, cx.owner, cx.style, String::new()));
}

fn add_text(st: &mut Collapser, items: &mut Vec<Item>, cx: &TextCx<'_>, text: &str) {
    let collapse = cx.style.text.white_space_collapse;
    let spaces_collapsible = matches!(collapse, WhiteSpaceCollapse::Collapse | WhiteSpaceCollapse::PreserveBreaks);
    let mut cur = String::new();
    for ch in text.chars() {
        // Carriage returns are spaces in every respect (4).
        let mut ch = if ch == '\r' { ' ' } else { ch };
        if ch == '\n' {
            match collapse {
                WhiteSpaceCollapse::Collapse => {
                    st.drop_trailing_space(items, &mut cur);
                    st.pending_break = true;
                    continue;
                }
                WhiteSpaceCollapse::PreserveBreaks => {
                    st.drop_trailing_space(items, &mut cur);
                    push_text(st, items, cx, &mut cur);
                    push_forced(items, cx);
                    st.reset_line();
                    st.skip_spaces = true;
                    continue;
                }
                WhiteSpaceCollapse::Preserve | WhiteSpaceCollapse::BreakSpaces => {
                    push_text(st, items, cx, &mut cur);
                    push_forced(items, cx);
                    st.reset_line();
                    continue;
                }
                // `preserve-spaces`: the line feed is a (preserved) space.
                WhiteSpaceCollapse::PreserveSpaces => ch = ' ',
            }
        }
        if spaces_collapsible && (ch == ' ' || ch == '\t') {
            if st.pending_break || st.skip_spaces || st.last_space_collapsible {
                continue;
            }
            cur.push(' ');
            st.last_space_collapsible = true;
            continue;
        }
        if st.pending_break {
            if st.break_needs_space(ch) {
                cur.push(' ');
            }
            st.pending_break = false;
        }
        cur.push(ch);
        st.last_char = Some(ch);
        st.last_space_collapsible = false;
        st.skip_spaces = false;
    }
    push_text(st, items, cx, &mut cur);
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum UnitKind {
    Normal,
    Tab,
    Forced,
    Atomic,
    /// An inline box edge.
    Edge,
    /// An out-of-flow placeholder.
    Oof,
    /// A float's place in the text (zero width; its outer widths are in `atoms`).
    Float,
}

/// The most bytes of one text item the platform shapes in a single call (see `Layouter::shape_item`).
const SHAPE_CHUNK: usize = 64 * 1024;

/// Whether `c` attaches to the character before it (combining marks, variation selectors, joiners, medial and final
/// Hangul jamo, kana voicing marks): a piece must not start with one.
fn joins_previous(c: char) -> bool {
    matches!(u32::from(c),
        0x0300..=0x036F | 0x0483..=0x0489 | 0x0591..=0x05C7 | 0x0610..=0x061A | 0x064B..=0x065F | 0x0670
        | 0x06D6..=0x06ED | 0x0900..=0x0903 | 0x093A..=0x094F | 0x0E31 | 0x0E34..=0x0E3A | 0x0E47..=0x0E4E
        | 0x1160..=0x11FF | 0x1AB0..=0x1AFF | 0x1DC0..=0x1DFF | 0x200B..=0x200F | 0x20D0..=0x20FF
        | 0x3099..=0x309A | 0xA960..=0xA97C | 0xD7B0..=0xD7FF | 0xFE00..=0xFE0F | 0xFE20..=0xFE2F | 0xE0100..=0xE01EF)
}

/// End of the piece of `text` that starts at byte `from` and goes to the platform in one call: the end of the text,
/// or, for a longer text, about `SHAPE_CHUNK` bytes on, preferably right after a space or line feed within the last
/// 4 KiB, else at the last place where the next character does not attach to the one before (and that one is not a
/// zero width joiner). If there is none within 4 KiB, the cut stays at the limit. Always greater than `from`.
fn shape_cut(text: &str, from: usize) -> usize {
    let limit = from.saturating_add(SHAPE_CHUNK);
    if limit >= text.len() {
        return text.len();
    }
    let limit = text.floor_char_boundary(limit).max(from + 1);
    let lo = text.floor_char_boundary(limit.saturating_sub(4096).max(from + 1));
    let ok = |p: usize| {
        text.get(p..).and_then(|r| r.chars().next()).is_some_and(|c| !joins_previous(c)) && text.get(..p).and_then(|l| l.chars().next_back()) != Some('\u{200D}')
    };
    let mut p = limit;
    while p > lo {
        if matches!(text.as_bytes().get(p - 1), Some(b' ' | b'\n')) && ok(p) {
            return p;
        }
        p -= 1;
    }
    p = limit;
    while p > lo {
        if text.is_char_boundary(p) && ok(p) {
            return p;
        }
        p -= 1;
    }
    limit
}

/// One glyph cluster (or edge, atomic inline, placeholder): the smallest piece a line can be cut into.
struct Unit {
    kind: UnitKind,
    item: u32,
    /// Byte range in the item's text (for an atomic inline: `start` is its index in `Prepared::atoms`).
    start: u32,
    end: u32,
    /// Advance (for tabs: the tab size; for atomic inlines: the margin box width).
    adv: LU,
    /// Range in the layouter's glyph array.
    glyphs: (u32, u32),
    first: char,
    last: char,
    collapsible_space: bool,
    hang_space: bool,
    edge_start: bool,
    /// The platform allows a line break before this unit (UAX #14).
    brk: bool,
}

/// What an atomic inline needs besides its width: min-content width (measuring), height above /
/// below its baseline (margin box), left and top margin, `position: relative` offset.
#[derive(Clone, Copy, Default)]
struct AtomInfo {
    min: LU,
    a: LU,
    d: LU,
    ml: LU,
    mt: LU,
    rel: (LU, LU),
}

impl Unit {
    fn new(kind: UnitKind, item: u32) -> Unit {
        Unit {
            kind,
            item,
            start: 0,
            end: 0,
            adv: LU::ZERO,
            glyphs: (0, 0),
            first: '\u{FFFC}',
            last: '\u{FFFC}',
            collapsible_space: false,
            hang_space: false,
            edge_start: false,
            brk: false,
        }
    }
}

/// A paragraph ready to be broken into lines: everything that does not depend on the width.
pub(crate) struct Prepared {
    items: Vec<Item>,
    units: Vec<Unit>,
    atoms: Vec<AtomInfo>,
    /// The text of each item (parallel to `items`).
    texts: Vec<String>,
    /// For each unit: the nearest earlier unit that is not an edge or placeholder.
    prev_content: Vec<u32>,
    /// Inline box edges used percentages, so the widths depend on the containing block width.
    pct_edges: bool,
    cbw: LU,
}

impl Prepared {
    /// May a line end between unit `j - 1` and unit `j`? Platform opportunities (UAX #14) plus
    /// `white-space`, `word-break` (css-text-3 5.1, 5.2, 4.1.1). Edges and placeholders between two
    /// units do not matter.
    fn opp_before(&self, j: usize) -> bool {
        let Some(u) = self.units.get(j) else { return false };
        if matches!(u.kind, UnitKind::Forced | UnitKind::Edge | UnitKind::Oof | UnitKind::Float) {
            return false;
        }
        let Some(pj) = self.prev_content.get(j).copied().filter(|&p| p != u32::MAX) else { return false };
        let Some(p) = self.units.get(pj as usize) else { return false };
        if p.kind == UnitKind::Forced {
            return false;
        }
        let (Some(iu), Some(ip)) = (self.items.get(u.item as usize), self.items.get(p.item as usize)) else { return false };
        let same = u.item == p.item;
        // The wrapping style of the nearest common ancestor decides; approximated by "either side wraps".
        let wrap_ok = if same { iu.wrap } else { iu.wrap || ip.wrap };
        if !wrap_ok {
            return false;
        }
        // An atomic inline allows a break on both sides.
        if u.kind == UnitKind::Atomic || p.kind == UnitKind::Atomic {
            return true;
        }
        let mut ok = u.brk;
        match iu.word_break {
            WordBreak::BreakAll => {
                if !ok && !is_space_like(p.last) && !is_space_like(u.first) && !is_closing_punct(u.first) && !is_opening_punct(p.last) && p.last != '\u{00A0}' && u.first != '\u{00A0}' && p.last != '\u{200B}' {
                    ok = true;
                }
            }
            WordBreak::KeepAll => {
                if ok && is_cjk_break(p.last) && is_cjk_break(u.first) && !is_closing_punct(u.first) && !is_opening_punct(p.last) {
                    ok = false;
                }
            }
            WordBreak::Normal | WordBreak::BreakWord => {}
        }
        // `break-spaces`: a break after every space and tab.
        if ip.break_spaces && matches!(p.last, ' ' | '\t') {
            ok = true;
        }
        ok
    }

    /// Width of a unit when it starts at `x` in its line (tabs depend on the position, 4.1.2).
    fn width_at(&self, u: &Unit, x: LU) -> LU {
        if u.kind != UnitKind::Tab {
            return u.adv;
        }
        let ts = u.adv;
        if ts <= LU::ZERO {
            return LU::ZERO;
        }
        let rest = LU::from_raw(ts.raw() as i64 - (x.raw() as i64).rem_euclid(ts.raw() as i64));
        // "If this distance is less than 0.5ch, then the subsequent tab stop is used instead."
        if rest.raw() < ts.raw() / 16 { rest + ts } else { rest }
    }

    /// Where the line starting at unit `s` ends: `(end, forced)`; the line holds units `s..end`.
    /// Greedy filling: break at the last opportunity that fits; if nothing fits, at the first one
    /// after the overflow (or inside the word when `overflow-wrap` allows). `x0` is where the line's
    /// content starts (text-indent), which tab stops count from.
    fn break_line(&self, s: usize, avail: LU, x0: LU) -> (usize, bool) {
        let mut best: Option<usize> = None;
        let mut overflowed = false;
        let (mut w_all, mut w_fit) = (LU::ZERO, LU::ZERO);
        let mut j = s;
        while let Some(u) = self.units.get(j) {
            if u.kind == UnitKind::Forced {
                return (j, true);
            }
            if j > s && self.opp_before(j) {
                if overflowed {
                    return (j, false);
                }
                best = Some(j);
            }
            w_all += self.width_at(u, x0 + w_all);
            // Collapsible spaces are removed and hanging spaces hang: neither counts for fitting.
            if !(u.collapsible_space || u.hang_space) {
                w_fit = w_all;
            }
            if w_fit > avail {
                if let Some(b) = best {
                    return (b, false);
                }
                let emergency = self.items.get(u.item as usize).is_some_and(|i| i.emergency);
                if emergency && j > s {
                    return (j, false);
                }
                overflowed = true;
            }
            j += 1;
        }
        (self.units.len(), false)
    }
}

enum Align {
    Left,
    Center,
    Right,
}

fn align_of(a: TextAlign) -> Align {
    // `start`/`end` are left/right (layer 1 has no right-to-left layout); `justify` starts at the left.
    match a {
        TextAlign::End | TextAlign::Right => Align::Right,
        TextAlign::Center => Align::Center,
        _ => Align::Left,
    }
}

/// What `prefetch_text` collects for one style: the distinct non-ASCII characters and ASCII words, in order of first appearance.
struct PrefetchGroup<'t> {
    style: Rc<ComputedStyle>,
    seen_chars: HashMap<char, ()>,
    chars: String,
    seen_words: HashMap<&'t str, ()>,
    words: Vec<&'t str>,
}

/// The padding box of one piece of a relatively positioned inline element, in the owner block's coordinates.
#[derive(Clone, Copy)]
struct PadRect {
    l: LU,
    t: LU,
    r: LU,
    b: LU,
}

/// Where the containing block of an out-of-flow box in a positioned inline element comes from.
enum InlineCb {
    /// The element's own pieces (known once all lines are done).
    Span(BoxId),
    /// A zero-width box at the placeholder (the element has no piece on this line).
    At(PadRect),
}

/// The innermost `position: relative` inline element between `owner` and `block` (CSS2 10.1 item 4: such
/// an element is the containing block of its absolutely positioned descendants).
fn relative_inline_ancestor(tree: &super::boxtree::BoxTree, owner: BoxId, block: BoxId) -> Option<BoxId> {
    let mut b = owner;
    for _ in 0..300 {
        if b == block || b == NO_BOX {
            return None;
        }
        let bx = tree.get(b);
        if bx.kind == BoxKind::Inline && bx.style.boxs.position == Position::Relative {
            return Some(b);
        }
        b = bx.parent;
    }
    None
}

struct Group {
    item: u32,
    a: usize,
    b: usize,
    x: LU,
    w: LU,
}

/// A box taking part in the vertical alignment of one line.
struct Ent {
    id: BoxId,
    parent: Option<usize>,
    atomic: bool,
    a: LU,
    d: LU,
    /// Raised by this much relative to the parent's baseline (`vertical-align`).
    shift: LU,
    /// Baseline raised by this much relative to the root inline box's baseline.
    off: LU,
    /// 0: aligned to the baseline, 1: `top`, 2: `bottom`.
    ta: u8,
    under_ta: bool,
    first: LU,
    last: LU,
    has_start: bool,
    has_end: bool,
    rel: (LU, LU),
    font: Option<Rc<ResolvedFont>>,
}

/// For every box: does it (or an ancestor that passes decorations down to it) declare a decoration
/// line? Parents have higher ids than their children, so one pass from the end sees parents first.
pub(super) fn decorated_boxes(tree: &super::boxtree::BoxTree) -> Vec<bool> {
    let mut v = vec![false; tree.boxes.len()];
    for i in (0..tree.boxes.len()).rev() {
        let Some(bx) = tree.boxes.get(i) else { continue };
        let own = bx.style.misc.text_decoration_line.0 & !TextDecorationLine::BLINK != 0;
        let stops = bx.oof || matches!(bx.kind, BoxKind::InlineBlock | BoxKind::Replaced);
        let from_parent = !stops && bx.parent != NO_BOX && v.get(bx.parent as usize).copied().unwrap_or(false);
        if let Some(d) = v.get_mut(i) {
            *d = own || from_parent;
        }
    }
    v
}

/// Does an inline element's box need a fragment of its own (background, border or padding to paint)?
fn paints_box(s: &ComputedStyle) -> bool {
    s.misc.background_color.resolve(s.text.color).a != 0 || s.border.width.iter().any(|w| *w > 0.0) || s.boxs.padding.iter().any(|p| p.as_px() != Some(0.0))
}

/// Blink's offsets for `sub` and `super`: a fifth and a third of the font size, plus a pixel.
fn sub_super_offset(size: f32, is_super: bool) -> LU {
    LU::from_f32(if is_super { size / 3.0 + 1.0 } else { size / 5.0 + 1.0 })
}

impl Layouter<'_> {
    /// Metrics of the primary font of `style`, cached per font description.
    pub(super) fn font_for(&mut self, style: &ComputedStyle) -> Rc<ResolvedFont> {
        let key = (Rc::as_ptr(&style.font) as usize, style.text.lang.as_ref().map_or(0, |l| Rc::as_ptr(l) as *const u8 as usize));
        if let Some(r) = self.fonts.get(&key) {
            return r.clone();
        }
        let f = &style.font;
        let q = FontQuery { families: &f.family, weight: f.weight, slant: slant_of(f.style), lang: style.text.lang.as_deref() };
        let m = self.platform.resolve_font(&q).map(|id| self.platform.font_metrics(id, f.size)).unwrap_or_default();
        // Ascent, descent and line gap are whole px (Blink rounds them the same way).
        let asc = LU::from_f32(m.ascent).round_px().max0();
        let desc = LU::from_f32(m.descent).round_px().max0();
        let gap = LU::from_f32(m.line_gap).round_px().max0();
        let lh = match f.line_height {
            LineHeight::Normal => asc + desc + gap,
            LineHeight::Number(n) => LU::from_f32(n * f.size),
            LineHeight::Px(v) => LU::from_f32(v),
        }
        .max0();
        // Half-leading (css-inline-3 4.2): the part above is rounded down to whole px (like Blink).
        let diff = lh - (asc + desc);
        let lead_top = diff.half().floor_px();
        let r = Rc::new(ResolvedFont {
            size: f.size,
            asc,
            desc,
            lead_top,
            lead_bottom: diff - lead_top,
            x_height: LU::from_f32(m.x_height.unwrap_or(0.5 * f.size)),
            zero_adv: LU::from_f32(m.zero_advance.unwrap_or(0.5 * f.size)),
            underline_offset: m.underline_offset,
            underline_thickness: m.underline_thickness,
            strike_offset: m.strikeout_offset,
            strike_thickness: m.strikeout_thickness,
        });
        self.fonts.insert(key, r.clone());
        r
    }

    /// Decoration lines that apply to text in `owner` (css-text-decor-3 2: the lines of the box and its
    /// ancestors up to an atomic inline or out-of-flow box propagate to the text).
    fn decor_specs(&self, owner: BoxId) -> Vec<DecorSpec> {
        let tree = self.tree;
        let mut v = Vec::new();
        let mut b = owner;
        for _ in 0..300 {
            if b == NO_BOX {
                break;
            }
            let bx = tree.get(b);
            let m = &bx.style.misc;
            if m.text_decoration_line.0 & !TextDecorationLine::BLINK != 0 {
                let color = m.text_decoration_color.resolve(bx.style.text.color);
                let thickness = m.text_decoration_thickness.as_ref().map(|l| l.resolve(bx.style.font.size));
                for (flag, kind) in [(TextDecorationLine::UNDERLINE, DecorKind::Underline), (TextDecorationLine::OVERLINE, DecorKind::Overline), (TextDecorationLine::LINE_THROUGH, DecorKind::LineThrough)] {
                    if m.text_decoration_line.has(flag) && !v.iter().any(|s: &DecorSpec| s.kind == kind && s.color == color) {
                        v.push(DecorSpec { kind, style: m.text_decoration_style, color, thickness });
                    }
                }
            }
            if bx.oof || matches!(bx.kind, BoxKind::InlineBlock | BoxKind::Replaced) {
                break;
            }
            b = bx.parent;
        }
        v
    }

    /// Start and end edge of an inline box: margin + border + padding on each side (only the first /
    /// last piece of a split element has them). The flag says whether a percentage was involved.
    fn edge_widths(&self, bid: BoxId, cbw: LU) -> (LU, LU, bool) {
        let bx = self.tree.get(bid);
        if !super::boxtree::has_horizontal_edges(&bx.style) {
            return (LU::ZERO, LU::ZERO, false);
        }
        let bs = &bx.style.boxs;
        let bd = &bx.style.border;
        let m = |i: usize| bs.margin.get(i).and_then(|d| resolve_margin(d, cbw)).unwrap_or(LU::ZERO);
        let p = |i: usize| bs.padding.get(i).map_or(LU::ZERO, |l| pad_px(l, cbw));
        let b = |i: usize| border_px(bd.width.get(i).copied().unwrap_or(0.0));
        let pct = [1usize, 3].iter().any(|&i| bs.padding.get(i).is_some_and(lp_has_percent) || matches!(bs.margin.get(i), Some(crate::css::types::Dim::Lp(l)) if lp_has_percent(l)));
        let start = if !bx.split.is_split || bx.split.first { m(3) + b(3) + p(3) } else { LU::ZERO };
        let end = if !bx.split.is_split || bx.split.last { m(1) + b(1) + p(1) } else { LU::ZERO };
        (start, end, pct)
    }

    fn collect(&self, bid: BoxId, st: &mut Collapser, items: &mut Vec<Item>, pct: &mut bool, cbw: LU) {
        let b = self.tree.get(bid);
        if b.oof {
            items.push(make_item(ItemKind::Oof, bid, b.parent, &b.style, String::new()));
            return;
        }
        if super::floats::is_float(b) {
            items.push(make_item(ItemKind::Float, bid, b.parent, &b.style, String::new()));
            return;
        }
        match b.kind {
            BoxKind::Text => {
                let decor = if self.decorated.get(b.parent as usize).copied().unwrap_or(false) { self.decor_specs(b.parent) } else { Vec::new() };
                let cx = TextCx { bid, owner: b.parent, style: &b.style, decor: &decor, marker: false };
                if let Some(s) = b.synth() {
                    add_text(st, items, &cx, s);
                } else if let Some(NodeData::Text(t)) = self.doc.node(b.node).map(|n| &n.data) {
                    add_text(st, items, &cx, t);
                }
            }
            BoxKind::LineBreak => {
                st.reset_line();
                items.push(make_item(ItemKind::Forced, bid, bid, &b.style, String::new()));
            }
            BoxKind::Inline => {
                let (start, end, p) = self.edge_widths(bid, cbw);
                *pct |= p;
                if start != LU::ZERO {
                    let mut it = make_item(ItemKind::Start, bid, bid, &b.style, String::new());
                    it.edge = start;
                    items.push(it);
                }
                for &c in &b.children {
                    self.collect(c, st, items, pct, cbw);
                }
                if end != LU::ZERO {
                    let mut it = make_item(ItemKind::End, bid, bid, &b.style, String::new());
                    it.edge = end;
                    items.push(it);
                }
            }
            BoxKind::InlineBlock | BoxKind::Replaced => {
                // An atomic inline acts like a non-space character for white-space collapsing; a
                // pending segment break before it becomes a space.
                if st.pending_break {
                    if st.last_char.is_some() {
                        items.push(make_item(ItemKind::Text, bid, b.parent, &b.style, " ".to_string()));
                    }
                    st.pending_break = false;
                }
                st.last_char = Some('\u{FFFC}');
                st.last_space_collapsible = false;
                st.skip_spaces = false;
                items.push(make_item(ItemKind::Atomic, bid, b.parent, &b.style, String::new()));
            }
            BoxKind::Block => {}
        }
    }

    /// Turn the inline content of `block` into units (the part of layout that does not depend on the
    /// width of the line, except for the percentages of inline box edges).
    fn prepare(&mut self, block: BoxId, cbw: LU) -> Prepared {
        let tree = self.tree;
        let bx = tree.get(block);
        let mut items: Vec<Item> = Vec::new();
        let mut st = Collapser::default();
        let mut pct = false;
        // An inside list marker is the first inline of the item (css-lists-3 3.1).
        if bx.ordinal().is_some()
            && bx.style.text.list_style_position == ListStylePosition::Inside
            && let Some(text) = super::lists::marker_string(bx)
        {
            let mut it = make_item(ItemKind::Text, block, block, &bx.style, text);
            it.collapsible = false;
            it.wrap = false;
            it.marker = true;
            items.push(it);
        }
        for &c in &bx.children {
            self.collect(c, &mut st, &mut items, &mut pct, cbw);
        }

        let mut units: Vec<Unit> = Vec::new();
        let mut base: Vec<usize> = Vec::with_capacity(items.len());
        let mut atoms: Vec<AtomInfo> = Vec::new();
        let mut texts: Vec<String> = Vec::with_capacity(items.len());
        let mut para_text = String::new();
        let mut work = 0u64;
        for (ii, item) in items.iter_mut().enumerate() {
            base.push(para_text.len());
            let mut text = std::mem::take(&mut item.text);
            if item.kind == ItemKind::Text {
                // The page's text allowance (see `MAX_SHAPED_CHARS`): the rest is dropped.
                let n = text.chars().count() as u64;
                if n > self.text_left {
                    let cut = text.char_indices().nth(self.text_left as usize).map_or(text.len(), |(i, _)| i);
                    text.truncate(cut);
                    self.truncated = true;
                }
                // The text stays in the result: do not keep the room the white-space pass grew it by.
                if text.len() > SHAPE_CHUNK && text.capacity() > text.len() + text.len() / 8 {
                    text.shrink_to_fit();
                }
                self.text_left = self.text_left.saturating_sub(n);
            }
            match item.kind {
                ItemKind::Forced => {
                    let mut u = Unit::new(UnitKind::Forced, ii as u32);
                    u.first = '\n';
                    u.last = '\n';
                    units.push(u);
                }
                ItemKind::Atomic => {
                    let mut u = Unit::new(UnitKind::Atomic, ii as u32);
                    u.start = atoms.len() as u32;
                    atoms.push(AtomInfo::default());
                    units.push(u);
                }
                ItemKind::Start | ItemKind::End => {
                    let mut u = Unit::new(UnitKind::Edge, ii as u32);
                    u.adv = item.edge;
                    u.edge_start = item.kind == ItemKind::Start;
                    units.push(u);
                }
                ItemKind::Oof => units.push(Unit::new(UnitKind::Oof, ii as u32)),
                ItemKind::Float => {
                    let mut u = Unit::new(UnitKind::Float, ii as u32);
                    u.start = atoms.len() as u32;
                    atoms.push(AtomInfo::default());
                    units.push(u);
                }
                ItemKind::Text => {
                    para_text.push_str(&text);
                    work += text.len() as u64;
                    if !text.is_empty() {
                        self.shape_item(item, ii as u32, &text, &mut units);
                    }
                }
            }
            texts.push(text);
        }
        self.charge(work + items.len() as u64);
        let lang = bx.style.text.lang.as_deref();
        let breaks = self.platform.line_breaks(&para_text, lang);
        // Mark the units a platform break precedes (both lists are in text order).
        let mut bi = 0usize;
        for u in units.iter_mut().filter(|u| matches!(u.kind, UnitKind::Normal | UnitKind::Tab)) {
            let off = base.get(u.item as usize).copied().unwrap_or(0) + u.start as usize;
            while breaks.get(bi).is_some_and(|&b| b < off) {
                bi += 1;
            }
            u.brk = breaks.get(bi) == Some(&off);
        }
        let mut prev_content = Vec::with_capacity(units.len());
        let mut last = u32::MAX;
        for (j, u) in units.iter().enumerate() {
            prev_content.push(last);
            if !matches!(u.kind, UnitKind::Edge | UnitKind::Oof | UnitKind::Float) {
                last = j as u32;
            }
        }
        Prepared { items, units, atoms, texts, prev_content, pct_edges: pct, cbw }
    }

    /// Before layout: tell the platform which non-ASCII characters each style is going to shape, so
    /// that it can do them in bulk (`Platform::prefetch`). At most the page's text allowance is scanned.
    pub(super) fn prefetch_text(&mut self) {
        let tree = self.tree;
        let mut index: HashMap<(usize, usize), usize> = HashMap::new();
        // Per style: the distinct non-ASCII characters in order of first appearance.
        let mut groups: Vec<PrefetchGroup<'_>> = Vec::new();
        let mut left = self.text_left;
        if let Some(stop) = self.stop_at {
            // First-screen-first: only the text that can be on the first screens is worth shaping in bulk now
            // (about one byte per 60 px2 of the area down to the stop line); the rest is shaped when it is laid out.
            let area = f64::from(stop.to_f32()) * f64::from(self.viewport.0.to_f32());
            left = left.min((area / 60.0) as u64 + 4096);
        }
        for b in &tree.boxes {
            if b.kind != BoxKind::Text || left == 0 {
                continue;
            }
            let text: &str = match (b.synth(), self.doc.node(b.node).map(|n| &n.data)) {
                (Some(s), _) => s,
                (None, Some(NodeData::Text(t))) => t,
                _ => continue,
            };
            let key = (Rc::as_ptr(&b.style.font) as usize, b.style.text.lang.as_ref().map_or(0, |l| Rc::as_ptr(l) as *const u8 as usize));
            let gi = *index.entry(key).or_insert_with(|| {
                groups.push(PrefetchGroup { style: b.style.clone(), seen_chars: HashMap::new(), chars: String::new(), seen_words: HashMap::new(), words: Vec::new() });
                groups.len() - 1
            });
            let Some(g) = groups.get_mut(gi) else { continue };
            let text = text.get(..text.floor_char_boundary(left as usize)).unwrap_or(text);
            for c in text.chars() {
                if !c.is_ascii() && g.seen_chars.insert(c, ()).is_none() {
                    g.chars.push(c);
                }
            }
            // ASCII words: runs of printable characters between spaces and non-ASCII characters.
            let bytes = text.as_bytes();
            let mut i = 0usize;
            while i < bytes.len() {
                if !bytes.get(i).is_some_and(|b| (0x21..0x7F).contains(b)) {
                    i += 1;
                    continue;
                }
                let mut j = i;
                while bytes.get(j).is_some_and(|b| (0x21..0x7F).contains(b)) {
                    j += 1;
                }
                if let Some(w) = text.get(i..j)
                    && j - i <= 128
                    && g.seen_words.insert(w, ()).is_none()
                {
                    g.words.push(w);
                }
                i = j;
            }
            left = left.saturating_sub(text.len() as u64);
        }
        for PrefetchGroup { style, chars, words, .. } in groups {
            if chars.is_empty() && words.is_empty() {
                continue;
            }
            let f = &style.font;
            let q = FontQuery { families: &f.family, weight: f.weight, slant: slant_of(f.style), lang: style.text.lang.as_deref() };
            self.platform.prefetch(&chars, &words, &q);
        }
    }

    /// Shape a text item and cut it into units (one per glyph cluster). A long item goes to the platform in pieces of
    /// about `SHAPE_CHUNK` bytes (cut where nothing can join across, see `shape_cut`), so that the platform's temporary
    /// arrays and the pieces' glyph arrays are the size of a piece, not of a megabyte of text; the arrays that keep the
    /// result are reserved once.
    fn shape_item(&mut self, item: &Item, ii: u32, text: &str, units: &mut Vec<Unit>) {
        if text.len() > SHAPE_CHUNK {
            // A little more than one glyph per character (ligature-free text can still come out at 1.01).
            let n = text.chars().count();
            self.glyphs.reserve(n + n / 16);
            units.reserve(n);
        }
        let mut tab_size: Option<LU> = None;
        let mut a = 0usize;
        while a < text.len() {
            let b = shape_cut(text, a);
            self.shape_piece(item, ii, text, (a, b), &mut tab_size, units);
            a = b;
        }
    }

    /// Shape `text[range]` and append its glyphs and units; offsets in both are relative to the whole item text.
    fn shape_piece(&mut self, item: &Item, ii: u32, text: &str, range: (usize, usize), tab_size: &mut Option<LU>, units: &mut Vec<Unit>) {
        let Some(piece) = text.get(range.0..range.1) else { return };
        let rf = self.font_for(&item.style);
        let f = &item.style.font;
        let q = FontQuery { families: &f.family, weight: f.weight, slant: slant_of(f.style), lang: item.style.text.lang.as_deref() };
        let mut glyphs = self.platform.shape(piece, &q, rf.size);
        if range.0 > 0 {
            let off = range.0 as u32;
            for g in &mut glyphs {
                g.cluster_start = g.cluster_start.saturating_add(off);
                g.cluster_end = g.cluster_end.saturating_add(off);
            }
        }
        let (ls, ws) = (f.letter_spacing, f.word_spacing);
        // Letter- and word-spacing are added to the advance of the last glyph of a cluster, so the
        // glyph array alone says where to draw (css-text-3 8.1, 8.2).
        if ls != 0.0 || ws != 0.0 {
            for i in 0..glyphs.len() {
                let Some(g) = glyphs.get(i).copied() else { break };
                let cluster_ends = glyphs.get(i + 1).is_none_or(|n| n.cluster_start != g.cluster_start || n.cluster_end != g.cluster_end);
                if cluster_ends {
                    let ch = text.get(g.cluster_start as usize..g.cluster_end as usize).and_then(|s| s.chars().next());
                    let sep = matches!(ch, Some(' ' | '\u{00A0}'));
                    if let Some(gm) = glyphs.get_mut(i) {
                        gm.advance += ls + if sep { ws } else { 0.0 };
                    }
                }
            }
        }
        let gbase = self.glyphs.len() as u32;
        self.glyphs.extend(glyphs.iter().map(super::PlacedGlyph::from));
        let mut i = 0usize;
        while let Some(g0) = glyphs.get(i) {
            let (cs, ce) = (g0.cluster_start, g0.cluster_end);
            let mut adv = g0.advance;
            let mut j = i + 1;
            while let Some(g) = glyphs.get(j) {
                if g.cluster_start != cs || g.cluster_end != ce {
                    break;
                }
                adv += g.advance;
                j += 1;
            }
            let s = text.get(cs as usize..ce as usize).unwrap_or("");
            let first = s.chars().next().unwrap_or('\u{FFFD}');
            let last = s.chars().next_back().unwrap_or('\u{FFFD}');
            let single_space = s == " ";
            let is_tab = s == "\t" && !item.collapsible;
            let mut u = Unit::new(UnitKind::Normal, ii);
            let mut width = LU::from_f32(adv);
            if is_tab {
                u.kind = UnitKind::Tab;
                // tab-size (css-text-3 7.3): a multiple of the space advance (with spacing), or a length.
                let ts = *tab_size.get_or_insert_with(|| match &item.style.text.tab_size {
                    TabSize::Spaces(n) => {
                        let sp = self.platform.shape(" ", &q, rf.size).first().map_or(0.0, |g| g.advance);
                        LU::from_f32((sp + ls + ws) * n)
                    }
                    TabSize::Len(l) => LU::from_f32(l.resolve(0.0)),
                });
                width = ts;
            }
            u.start = cs;
            u.end = ce;
            u.adv = width;
            u.glyphs = (gbase + i as u32, gbase + j as u32);
            u.first = first;
            u.last = last;
            u.collapsible_space = item.collapsible && single_space;
            // Preserved spaces at the end of a wrapped line hang (4.1.2); `break-spaces` ones do not.
            u.hang_space = !item.collapsible && item.wrap && !item.break_spaces && (single_space || is_tab);
            units.push(u);
            i = j;
        }
    }

    /// Min-content and max-content width of the paragraph in `block` (css-sizing-3 4.1): the longest
    /// unbreakable run, and the widest line without wrapping.
    pub(crate) fn measure_inline(&mut self, block: BoxId) -> (LU, LU) {
        if self.depth >= super::MAX_PARAGRAPH_DEPTH {
            return (LU::ZERO, LU::ZERO);
        }
        self.depth += 1;
        let r = self.measure_inline_inner(block);
        self.depth -= 1;
        r
    }

    fn measure_inline_inner(&mut self, block: BoxId) -> (LU, LU) {
        let mut prep = match self.paras.remove(&block) {
            Some(p) => p,
            None => self.prepare(block, LU::ZERO),
        };
        let tree = self.tree;
        for k in 0..prep.units.len() {
            let Some(u) = prep.units.get(k) else { break };
            if !matches!(u.kind, UnitKind::Atomic | UnitKind::Float) {
                continue;
            }
            let Some(bid) = prep.items.get(u.item as usize).map(|i| i.box_id) else { continue };
            let (mn, mx) = self.outer_widths(bid);
            let ai = prep.units.get(k).map_or(0, |u| u.start as usize);
            if let Some(at) = prep.atoms.get_mut(ai) {
                at.min = mn;
            }
            if let Some(u) = prep.units.get_mut(k) {
                u.adv = mx;
            }
        }
        let indent = {
            let l = &tree.get(block).style.text.text_indent;
            if lp_has_percent(l) { LU::ZERO } else { LU::from_f32(l.resolve(0.0)) }
        };
        let (mut min, mut max) = (LU::ZERO, LU::ZERO);
        let (mut run_min, mut seg) = (LU::ZERO, indent);
        let mut seg_started = false;
        for j in 0..prep.units.len() {
            let Some(u) = prep.units.get(j) else { break };
            if u.kind == UnitKind::Forced {
                min = min.max(run_min);
                max = max.max(seg);
                (run_min, seg, seg_started) = (LU::ZERO, LU::ZERO, false);
                continue;
            }
            if u.kind == UnitKind::Float {
                // A float is as wide as it is wherever it goes: its min-content width counts alone, its max-content
                // width joins the line.
                min = min.max(prep.atoms.get(u.start as usize).map_or(LU::ZERO, |a| a.min));
                seg += u.adv;
                continue;
            }
            if prep.opp_before(j) {
                min = min.max(run_min);
                run_min = LU::ZERO;
            }
            let skip = u.collapsible_space && !seg_started;
            if !skip {
                seg_started = true;
                seg += prep.width_at(u, seg);
                if !(u.collapsible_space || u.hang_space) {
                    run_min += if u.kind == UnitKind::Atomic { prep.atoms.get(u.start as usize).map_or(u.adv, |a| a.min) } else { u.adv };
                }
            }
        }
        // Trailing collapsible spaces of the last segment do not count.
        let trailing = prep.units.iter().rev().take_while(|u| u.collapsible_space).fold(LU::ZERO, |a, u| a + u.adv);
        min = min.max(run_min);
        max = max.max(seg - trailing);
        self.paras.insert(block, prep);
        (min, max)
    }

    /// Lay out one atomic inline: the margin box width, its height above / below the baseline, the
    /// left and top margin and the `position: relative` offset.
    pub(super) fn layout_atomic(&mut self, id: BoxId, cbw: LU) -> (LU, LU, LU, LU, LU, (LU, LU)) {
        let tree = self.tree;
        let bx = tree.get(id);
        let bs = &bx.style.boxs;
        let m = |i: usize| bs.margin.get(i).and_then(|d| resolve_margin(d, cbw)).unwrap_or(LU::ZERO);
        let (mt, mr, mb, ml) = (m(0), m(1), m(2), m(3));
        let cw = if bx.kind == BoxKind::Replaced { self.replaced_size(id, cbw, None).0 } else { self.atomic_width(id, cbw) };
        // Atomic inlines are laid out whole: their position is not known yet, so there is no stop line inside.
        self.no_stop += 1;
        let r = self.layout_block(id, cbw, None, Flow { forced_w: Some(cw), ..Flow::default() });
        self.no_stop -= 1;
        let o = self.out.get(id as usize).copied().unwrap_or_default();
        // CSS2 10.8.1: the baseline of an inline-block is that of its last line box, unless it is
        // clipped or has none; replaced elements sit on the bottom margin edge.
        // (A text field keeps its text's baseline even though its content is clipped; a text area does not.)
        let clipped = o.clip && !bx.force_clip;
        let table = matches!(bx.fallback, Some(crate::css::types::Display::InlineTable));
        let baseline = if bx.kind == BoxKind::Replaced || clipped || matches!(bx.control(), Control::TextArea(..)) {
            None
        } else if table {
            // The baseline of an inline-table is that of its first row (CSS2 17.5.1).
            self.first_baseline(id)
        } else {
            self.last_baseline(id)
        };
        let outer_h = mt + o.h + mb;
        let a = baseline.map_or(outer_h, |b| mt + b);
        (ml + o.w + mr, a, outer_h - a, ml, mt, r.rel)
    }

    /// Lay out the inline content of `block` into lines. Positions are relative to the block's border
    /// box: the content box starts at `(left, top)` and is `content_w` wide. Returns the height of the lines.
    pub(super) fn layout_inline(&mut self, block: BoxId, content_w: LU, left: LU, top: LU) -> LU {
        if self.depth >= super::MAX_PARAGRAPH_DEPTH {
            return LU::ZERO;
        }
        self.depth += 1;
        let prep = match self.paras.remove(&block) {
            Some(p) if !p.pct_edges || p.cbw == content_w => p,
            _ => self.prepare(block, content_w),
        };
        let (h, inline_oof) = self.lay_lines(block, prep, content_w, left, top);
        self.depth -= 1;
        // Out-of-flow boxes whose containing block is a positioned inline element of this paragraph.
        for (p, rect) in inline_oof {
            // The containing block of a cut paragraph is not final (first-screen-first).
            if !self.cut.contains(&block) {
                self.layout_oof_in_rect(p, rect);
            }
        }
        h
    }

    /// Returns the height of the lines, and the out-of-flow boxes that sit in a `position: relative`
    /// inline element with the padding box of that element (x, y, w, h in this block's coordinates).
    fn lay_lines(&mut self, block: BoxId, mut prep: Prepared, content_w: LU, left: LU, top: LU) -> (LU, Vec<(Pending, [LU; 4])>) {
        let tree = self.tree;
        let bx = tree.get(block);
        let text_base = self.texts.len() as u32;
        self.texts.append(&mut prep.texts);

        // Floats are laid out first too (a float is a formatting context of its own: where it ends up does not change it, and
        // its lines must not come between those of this block); the loop below only places them.
        let mut fdims: Vec<(usize, super::floats::FloatDims)> = Vec::new();
        // Atomic inlines are laid out first: their widths decide where lines break.
        for k in 0..prep.units.len() {
            if self.exhausted {
                break;
            }
            let Some(u) = prep.units.get(k) else { break };
            if u.kind == UnitKind::Float {
                // Measured for the shrink-to-fit width of the container; in a line it takes no room (it is placed beside it).
                let fid = prep.items.get(u.item as usize).map(|i| i.box_id);
                if let Some(u) = prep.units.get_mut(k) {
                    u.adv = LU::ZERO;
                }
                if let Some(fid) = fid {
                    let d = self.measure_float(fid, content_w);
                    fdims.push((k, d));
                }
                continue;
            }
            if u.kind != UnitKind::Atomic {
                continue;
            }
            let Some(bid) = prep.items.get(u.item as usize).map(|i| i.box_id) else { continue };
            let (w, a, d, ml, mt, rel) = self.layout_atomic(bid, content_w);
            let ai = prep.units.get(k).map_or(0, |u| u.start as usize);
            if let Some(at) = prep.atoms.get_mut(ai) {
                *at = AtomInfo { min: at.min, a, d, ml, mt, rel };
            }
            if let Some(u) = prep.units.get_mut(k) {
                u.adv = w;
            }
        }

        let strut_font = self.font_for(&bx.style);
        let strut_asc = strut_font.asc + strut_font.lead_top;
        let strut_desc = strut_font.desc + strut_font.lead_bottom;
        let text_align = bx.style.text.text_align;
        let align = align_of(text_align);
        let indent = LU::from_f32(bx.style.text.text_indent.resolve(content_w.to_f32()));
        let first_line = self.lines.len() as u32;
        let cb_for_oof = |this: &Self, fixed: bool| if fixed { None } else { this.cb_stack.last().copied() };

        let mut y = LU::ZERO;
        let mut s = 0usize;
        let mut emitted = 0usize;
        // Floats in this paragraph: the units already placed, and those that wait for the end of their line.
        let has_floats = prep.units.iter().any(|u| u.kind == UnitKind::Float);
        let mut placed: Vec<usize> = Vec::new();
        let mut deferred: Vec<(BoxId, super::floats::FloatDims)> = Vec::new();
        let strut_h = strut_asc + strut_desc;
        let mut groups: Vec<Group> = Vec::new();
        let mut widths: Vec<LU> = Vec::new();
        let mut ents: Vec<Ent> = Vec::new();
        let n = prep.units.len();
        // First and last padding box of every relatively positioned inline element, and the boxes waiting for them.
        let mut spans: HashMap<BoxId, (PadRect, PadRect)> = HashMap::new();
        let mut inline_oof: Vec<(Pending, InlineCb)> = Vec::new();
        // Index of the last unit that is real content: "is there more content after this line" is then a
        // comparison, not a scan to the end of the paragraph on every line (blank lines in `<pre>`).
        let last_content = prep.units.iter().rposition(|u| matches!(u.kind, UnitKind::Normal | UnitKind::Tab | UnitKind::Atomic) && !u.collapsible_space);
        loop {
            // Collapsible spaces at the start of a line are removed (4.1.2).
            while prep.units.get(s).is_some_and(|u| u.collapsible_space) {
                s += 1;
            }
            if s >= n || self.exhausted {
                break;
            }
            if self.past_stop(self.origin_y + top + y) {
                self.cut.push(block);
                break;
            }
            let line_indent = if emitted == 0 { indent } else { LU::ZERO };
            // Floats (CSS2 9.5, 9.4.2): the ones that open the line are placed first; the line is as wide as the room
            // they and the earlier ones leave at its height. A float inside the line goes at its top when it fits next
            // to the text before it (the line is broken again), else below the line. A line whose first piece does not
            // fit beside the floats moves down to where they end.
            let (mut line_left, mut line_w) = (LU::ZERO, content_w);
            let (mut end, mut forced);
            if has_floats || !self.fs.list.is_empty() {
                let mut rounds = 0;
                // The height of the line is not known before it is laid out: the strut's, or an atomic inline's if the
                // line has one (the room the floats leave is looked up for that height).
                let mut band_h = strut_h;
                loop {
                    rounds += 1;
                    if has_floats {
                        let mut k = s;
                        while let Some(u) = prep.units.get(k) {
                            match u.kind {
                                UnitKind::Float if !placed.contains(&k) => {
                                    if let Some(fid) = prep.items.get(u.item as usize).map(|i| i.box_id)
                                        && let Some(&(_, d)) = fdims.iter().find(|(i, _)| *i == k)
                                    {
                                        self.put_float(fid, d, content_w, left, top + y);
                                    }
                                    placed.push(k);
                                }
                                UnitKind::Float | UnitKind::Edge | UnitKind::Oof => {}
                                _ if u.collapsible_space => {}
                                _ => break,
                            }
                            k += 1;
                        }
                    }
                    (line_left, line_w) = self.line_band(left, content_w, top + y, band_h);
                    (end, forced) = prep.break_line(s, line_w - line_indent, line_indent);
                    if end <= s && !forced {
                        end = s + 1;
                    }
                    if rounds > 200 {
                        break;
                    }
                    let tall = (s..end)
                        .filter_map(|k| prep.units.get(k).filter(|u| u.kind == UnitKind::Atomic))
                        .filter_map(|u| prep.atoms.get(u.start as usize))
                        .fold(band_h, |m, at| m.max(at.a + at.d));
                    if tall > band_h {
                        band_h = tall;
                        continue;
                    }
                    if has_floats
                        && let Some(f) = (s..end).find(|k| prep.units.get(*k).is_some_and(|u| u.kind == UnitKind::Float) && !placed.contains(k))
                    {
                        let before = (s..f).filter_map(|k| prep.units.get(k)).filter(|u| !u.collapsible_space).fold(LU::ZERO, |a, u| a + u.adv);
                        if let Some(fid) = prep.units.get(f).and_then(|u| prep.items.get(u.item as usize)).map(|i| i.box_id)
                            && let Some(&(_, d)) = fdims.iter().find(|(i, _)| *i == f)
                        {
                            placed.push(f);
                            // Floats do not reorder (9.5.1 rule 5): one that waits for the next line holds back the later ones.
                            if before + d.w <= line_w - line_indent && deferred.is_empty() {
                                self.put_float(fid, d, content_w, left, top + y);
                                continue;
                            }
                            deferred.push((fid, d));
                        }
                        continue;
                    }
                    if line_w < content_w {
                        let used = (s..end).filter_map(|k| prep.units.get(k)).filter(|u| !(u.collapsible_space || u.hang_space)).fold(LU::ZERO, |a, u| a + u.adv);
                        if used > line_w - line_indent
                            && let Some(nb) = self.line_next_bottom(top + y, band_h)
                            && nb > top + y
                        {
                            y = nb - top;
                            continue;
                        }
                    }
                    break;
                }
            } else {
                (end, forced) = prep.break_line(s, content_w - line_indent, line_indent);
                if end <= s && !forced {
                    end = s + 1;
                }
            }
            if !forced {
                // Start edges before the break belong to the next line.
                while end > s + 1 && prep.units.get(end - 1).is_some_and(|u| u.kind == UnitKind::Edge && u.edge_start) {
                    end -= 1;
                }
            }
            let next = if forced { end + 1 } else { end };
            self.charge((end - s) as u64 + 4);
            // ... and at the end of a line.
            let mut e = end;
            while e > s && prep.units.get(e - 1).is_some_and(|u| u.collapsible_space) {
                e -= 1;
            }
            if e == s && !forced {
                s = next;
                continue;
            }
            let content = forced || prep.units.get(s..e).unwrap_or(&[]).iter().any(|u| matches!(u.kind, UnitKind::Normal | UnitKind::Tab | UnitKind::Atomic | UnitKind::Edge));
            if !content {
                // Only placeholders: no line box, but they have a static position.
                for u in prep.units.get(s..e).unwrap_or(&[]) {
                    if let Some(it) = prep.items.get(u.item as usize)
                        && it.kind == ItemKind::Oof
                    {
                        let fixed = tree.get(it.box_id).style.boxs.position == Position::Fixed;
                        let cb = cb_for_oof(self, fixed);
                        let (sx, sy) = (left + line_indent, top + y);
                        let pend = Pending { id: it.box_id, owner: block, sx, sy, cb };
                        match if fixed { None } else { relative_inline_ancestor(tree, it.owner, block) } {
                            Some(span) => {
                                // No line box: the element is empty here, a zero-width box on the strut's baseline.
                                let rf = self.font_for(&tree.get(span).style);
                                let t = sy + strut_asc - rf.asc;
                                inline_oof.push((pend, InlineCb::At(PadRect { l: sx, t, r: sx, b: t + rf.asc + rf.desc })));
                            }
                            None => self.pending.push(pend),
                        }
                    }
                }
                s = next;
                continue;
            }

            // Measure.
            groups.clear();
            widths.clear();
            let mut x = line_indent;
            for k in s..e {
                let Some(u) = prep.units.get(k) else { break };
                let w = prep.width_at(u, x);
                widths.push(w);
                x += w;
            }
            // Hanging spaces do not count for alignment, except that ones before a forced break
            // hang only if they would overflow (4.1.2).
            let mut hang = LU::ZERO;
            let mut hang_n = 0usize;
            let mut k = e;
            while k > s && prep.units.get(k - 1).is_some_and(|u| u.hang_space) {
                hang += widths.get(k - 1 - s).copied().unwrap_or(LU::ZERO);
                hang_n += 1;
                k -= 1;
            }
            if forced && x <= line_w {
                hang = LU::ZERO;
                hang_n = 0;
            }
            let mut free = line_w - (x - hang);

            // Justification (css-text-3 7.3, 8.4): the line is stretched at spaces (Latin) and between
            // ideographs (CJK); the last line and lines ended by a break are left alone.
            let justify_all = text_align == TextAlign::JustifyAll;
            let wraps = prep.items.get(prep.units.get(s).map_or(0, |u| u.item as usize)).is_none_or(|i| i.wrap);
            let more = last_content.is_some_and(|l| l >= next);
            if (justify_all || (text_align == TextAlign::Justify && !forced && more)) && wraps && free > LU::ZERO {
                let stop = e - hang_n;
                let mut opps: Vec<usize> = Vec::new();
                for k in s..stop {
                    let Some(u) = prep.units.get(k) else { break };
                    if u.kind != UnitKind::Normal {
                        continue;
                    }
                    let nxt = prep.units.get(k + 1..stop).unwrap_or(&[]).iter().find(|m| matches!(m.kind, UnitKind::Normal | UnitKind::Tab | UnitKind::Atomic));
                    let Some(nu) = nxt else { continue };
                    let sep = matches!(u.last, ' ' | '\u{00A0}');
                    let cjk = nu.kind == UnitKind::Normal && is_cjk_break(u.last) && is_cjk_break(nu.first) && !is_closing_punct(nu.first) && !is_opening_punct(u.last);
                    if sep || cjk {
                        opps.push(k);
                    }
                }
                if !opps.is_empty() {
                    let total = free.raw() as i64;
                    let cnt = opps.len() as i64;
                    for (i, &k) in opps.iter().enumerate() {
                        let add = LU::from_raw(total / cnt + i64::from((i as i64) < total % cnt));
                        if let Some(w) = widths.get_mut(k - s) {
                            *w += add;
                        }
                        // The glyph array carries the extra room too (the last glyph of the cluster).
                        if let Some(gi) = prep.units.get(k).map(|u| u.glyphs).filter(|g| g.1 > g.0).map(|g| g.1 as usize - 1)
                            && let Some(g) = self.glyphs.get_mut(gi)
                        {
                            g.advance += add.to_f32();
                        }
                    }
                    free = LU::ZERO;
                }
            }
            let offset = match align {
                Align::Left => LU::ZERO,
                Align::Center if free > LU::ZERO => free.half(),
                Align::Right if free > LU::ZERO => free,
                _ => LU::ZERO,
            };

            // Positions of the groups (consecutive units of one item).
            let mut gx = line_indent;
            for k in s..e {
                let Some(u) = prep.units.get(k) else { break };
                let w = widths.get(k - s).copied().unwrap_or(LU::ZERO);
                match groups.last_mut() {
                    Some(g) if g.item == u.item => {
                        g.b = k + 1;
                        g.w += w;
                    }
                    _ => groups.push(Group { item: u.item, a: k, b: k + 1, x: gx, w }),
                }
                gx += w;
            }
            if forced && let Some(fu) = prep.units.get(end) {
                groups.push(Group { item: fu.item, a: end, b: end + 1, x: gx, w: LU::ZERO });
            }

            // Vertical alignment (CSS2 10.8.1): the boxes on this line and their offsets.
            self.line_no += 1;
            let ln = self.line_no;
            ents.clear();
            let mut scratch = std::mem::take(&mut self.scratch);
            for g in &groups {
                let Some(it) = prep.items.get(g.item as usize) else { continue };
                if matches!(it.kind, ItemKind::Oof | ItemKind::Float) {
                    continue;
                }
                scratch.clear();
                let mut b = it.owner;
                for _ in 0..300 {
                    if b == block || b == NO_BOX {
                        break;
                    }
                    scratch.push(b);
                    b = tree.get(b).parent;
                }
                let mut parent: Option<usize> = None;
                for &cb in scratch.iter().rev() {
                    let slot = self.stamp.get(cb as usize).copied().unwrap_or((0, 0));
                    let idx = if slot.0 == ln {
                        slot.1 as usize
                    } else {
                        let cbx = tree.get(cb);
                        let rf = self.font_for(&cbx.style);
                        let (pa, pd, pxh, psz, pfont) = match parent.and_then(|p| ents.get(p)).and_then(|p| p.font.clone()) {
                            Some(pf) => (pf.asc + pf.lead_top, pf.desc + pf.lead_bottom, pf.x_height, pf.size, pf),
                            None => (strut_asc, strut_desc, strut_font.x_height, strut_font.size, strut_font.clone()),
                        };
                        let (a, d) = (rf.asc + rf.lead_top, rf.desc + rf.lead_bottom);
                        let (shift, ta) = vshift(&cbx.style.misc.vertical_align, a, d, &pfont, (pa, pd, pxh, psz));
                        let rel = relative_of(cbx, parent.and_then(|p| ents.get(p)).map(|p| p.rel), content_w);
                        ents.push(Ent {
                            id: cb,
                            parent,
                            atomic: false,
                            a,
                            d,
                            shift,
                            off: LU::ZERO,
                            ta,
                            under_ta: parent.and_then(|p| ents.get(p)).is_some_and(|p| p.ta != 0 || p.under_ta),
                            first: g.x,
                            last: g.x + g.w,
                            has_start: false,
                            has_end: false,
                            rel,
                            font: Some(rf),
                        });
                        if let Some(sl) = self.stamp.get_mut(cb as usize) {
                            *sl = (ln, (ents.len() - 1) as u32);
                        }
                        ents.len() - 1
                    };
                    if let Some(en) = ents.get_mut(idx) {
                        en.first = en.first.min(g.x);
                        en.last = en.last.max(g.x + g.w);
                    }
                    parent = Some(idx);
                }
                match it.kind {
                    ItemKind::Start | ItemKind::End => {
                        if let Some(en) = parent.and_then(|p| ents.get_mut(p)) {
                            if it.kind == ItemKind::Start {
                                en.has_start = true;
                            } else {
                                en.has_end = true;
                            }
                        }
                    }
                    ItemKind::Atomic => {
                        let Some(u) = prep.units.get(g.a) else { continue };
                        let at = prep.atoms.get(u.start as usize).copied().unwrap_or_default();
                        let abx = tree.get(it.box_id);
                        let (pa, pd, pxh, psz, pfont) = match parent.and_then(|p| ents.get(p)).and_then(|p| p.font.clone()) {
                            Some(pf) => (pf.asc + pf.lead_top, pf.desc + pf.lead_bottom, pf.x_height, pf.size, pf),
                            None => (strut_asc, strut_desc, strut_font.x_height, strut_font.size, strut_font.clone()),
                        };
                        let (shift, ta) = vshift(&abx.style.misc.vertical_align, at.a, at.d, &pfont, (pa, pd, pxh, psz));
                        let prel = parent.and_then(|p| ents.get(p)).map_or((LU::ZERO, LU::ZERO), |p| p.rel);
                        ents.push(Ent {
                            id: it.box_id,
                            parent,
                            atomic: true,
                            a: at.a,
                            d: at.d,
                            shift,
                            off: LU::ZERO,
                            ta,
                            under_ta: false,
                            first: g.x,
                            last: g.x + g.w,
                            has_start: false,
                            has_end: false,
                            rel: (prel.0 + at.rel.0, prel.1 + at.rel.1),
                            font: None,
                        });
                    }
                    _ => {}
                }
            }
            self.scratch = scratch;

            // Offsets of the baseline-aligned boxes, then the line's extent (strut included).
            let (mut above, mut below) = (strut_asc, strut_desc);
            for i in 0..ents.len() {
                let Some(en) = ents.get(i) else { break };
                if en.ta != 0 || en.under_ta {
                    continue;
                }
                let po = en.parent.and_then(|p| ents.get(p)).map_or(LU::ZERO, |p| p.off);
                let off = po + en.shift;
                above = above.max(off + en.a);
                below = below.max(en.d - off);
                if let Some(en) = ents.get_mut(i) {
                    en.off = off;
                }
            }
            // `top` / `bottom` boxes make the line at least as tall as themselves.
            let h0 = above + below;
            let (mut extra_top, mut extra_bottom) = (LU::ZERO, LU::ZERO);
            for en in ents.iter().filter(|e| e.ta != 0) {
                let h = en.a + en.d;
                if h > h0 {
                    if en.ta == 1 {
                        extra_bottom = extra_bottom.max(h - h0);
                    } else {
                        extra_top = extra_top.max(h - h0);
                    }
                }
            }
            let (above, below) = (above + extra_top, below + extra_bottom);
            let line_h = above + below;
            for i in 0..ents.len() {
                let Some(en) = ents.get(i) else { break };
                if en.ta == 0 && !en.under_ta {
                    continue;
                }
                let off = if en.ta == 1 {
                    above - en.a
                } else if en.ta == 2 {
                    en.d - below
                } else {
                    en.parent.and_then(|p| ents.get(p)).map_or(LU::ZERO, |p| p.off) + en.shift
                };
                if let Some(en) = ents.get_mut(i) {
                    en.off = off;
                }
            }

            // Fragments.
            let frag_start = self.fragments.len() as u32;
            let line_top = top + y;
            let base_y = line_top + above;
            let ox = left + line_left + offset;
            for en in ents.iter().filter(|e| !e.atomic) {
                let cbx = tree.get(en.id);
                let positioned = cbx.style.boxs.position == Position::Relative;
                let paints = paints_box(&cbx.style);
                if !paints && !positioned {
                    continue;
                }
                let Some(rf) = en.font.as_ref() else { continue };
                let m = |i: usize| cbx.style.boxs.margin.get(i).and_then(|d| resolve_margin(d, content_w)).unwrap_or(LU::ZERO);
                let p = |i: usize| cbx.style.boxs.padding.get(i).map_or(LU::ZERO, |l| pad_px(l, content_w));
                let b = |i: usize| border_px(cbx.style.border.width.get(i).copied().unwrap_or(0.0));
                let x0 = en.first + if en.has_start { m(3) } else { LU::ZERO };
                let x1 = en.last - if en.has_end { m(1) } else { LU::ZERO };
                let content_top = base_y - en.off - rf.asc;
                let (pt, pb, bt, bb) = (p(0), p(2), b(0), b(2));
                if positioned {
                    // Padding box of this piece (CSS2 10.1 item 4): the border only sits at the ends of the element.
                    let piece = PadRect {
                        l: ox + x0 + en.rel.0 + if en.has_start { b(3) } else { LU::ZERO },
                        t: content_top - pt + en.rel.1,
                        r: ox + x1 + en.rel.0 - if en.has_end { b(1) } else { LU::ZERO },
                        b: content_top + rf.asc + rf.desc + pb + en.rel.1,
                    };
                    spans.entry(en.id).and_modify(|e| e.1 = piece).or_insert((piece, piece));
                }
                if !paints {
                    continue;
                }
                self.fragments.push(Fragment {
                    box_id: en.id,
                    kind: FragKind::InlineBox { first: en.has_start, last: en.has_end },
                    x: ox + x0 + en.rel.0,
                    y: content_top - pt - bt + en.rel.1,
                    w: (x1 - x0).max0(),
                    h: rf.asc + rf.desc + pt + pb + bt + bb,
                });
            }
            for g in &groups {
                let (Some(it), Some(ua)) = (prep.items.get(g.item as usize), prep.units.get(g.a)) else { continue };
                // The entry of the item's owner gives the baseline offset and relative shift.
                let (off, rel) = if it.owner == block || it.owner == NO_BOX {
                    (LU::ZERO, (LU::ZERO, LU::ZERO))
                } else {
                    match self.stamp.get(it.owner as usize).copied() {
                        Some((l, i)) if l == ln => ents.get(i as usize).map_or((LU::ZERO, (LU::ZERO, LU::ZERO)), |e| (e.off, e.rel)),
                        _ => (LU::ZERO, (LU::ZERO, LU::ZERO)),
                    }
                };
                match it.kind {
                    ItemKind::Text => {
                        let Some(ub) = g.b.checked_sub(1).and_then(|k| prep.units.get(k)) else { continue };
                        let rf = self.font_for(&it.style);
                        let baseline_y = base_y - off + rel.1;
                        let fx = ox + g.x + rel.0;
                        let tid = text_base + g.item;
                        let glyphs = (ua.glyphs.0, ub.glyphs.1);
                        let kind = if it.marker {
                            FragKind::Marker { text: tid, start: ua.start, end: ub.end, glyphs, asc: rf.asc }
                        } else {
                            let d0 = self.decorations.len() as u32;
                            for sp in &it.decor {
                                self.push_decoration(sp, &rf, fx, baseline_y, g.w);
                            }
                            FragKind::Text { text: tid, start: ua.start, end: ub.end, glyphs, decor: (d0, self.decorations.len() as u32), asc: rf.asc }
                        };
                        self.fragments.push(Fragment { box_id: it.box_id, kind, x: fx, y: baseline_y - rf.asc, w: g.w, h: rf.asc + rf.desc });
                    }
                    ItemKind::Forced => {
                        let rf = self.font_for(&it.style);
                        self.fragments.push(Fragment { box_id: it.box_id, kind: FragKind::Br, x: ox + g.x + rel.0, y: base_y - off + rel.1 - rf.asc, w: LU::ZERO, h: rf.asc + rf.desc });
                    }
                    ItemKind::Atomic => {
                        let Some(en) = ents.iter().find(|e| e.atomic && e.id == it.box_id) else { continue };
                        let at = prep.atoms.get(ua.start as usize).copied().unwrap_or_default();
                        let ax = ox + g.x + at.ml + en.rel.0;
                        let ay = base_y - en.off - at.a + at.mt + en.rel.1;
                        let o = self.out.get(it.box_id as usize).copied().unwrap_or_default();
                        if let Some(o) = self.out.get_mut(it.box_id as usize) {
                            o.x = ax;
                            o.y = ay;
                        }
                        self.fragments.push(Fragment { box_id: it.box_id, kind: FragKind::Atomic, x: ax, y: ay, w: o.w, h: o.h });
                    }
                    ItemKind::Oof => {
                        let fixed = tree.get(it.box_id).style.boxs.position == Position::Fixed;
                        let cb = cb_for_oof(self, fixed);
                        let pend = Pending { id: it.box_id, owner: block, sx: ox + g.x, sy: line_top, cb };
                        match if fixed { None } else { relative_inline_ancestor(tree, it.owner, block) } {
                            Some(span) if self.stamp.get(span as usize).is_some_and(|sl| sl.0 == ln) => inline_oof.push((pend, InlineCb::Span(span))),
                            Some(span) => {
                                // The element has no piece on this line (it is empty): a zero-width box at the placeholder.
                                let rf = self.font_for(&tree.get(span).style);
                                let (x, t) = (ox + g.x, base_y - rf.asc);
                                inline_oof.push((pend, InlineCb::At(PadRect { l: x, t, r: x, b: t + rf.asc + rf.desc })));
                            }
                            None => self.pending.push(pend),
                        }
                    }
                    ItemKind::Start | ItemKind::End | ItemKind::Float => {}
                }
            }
            self.lines.push(Line {
                x: left + line_left,
                y: line_top,
                w: line_w,
                h: line_h,
                baseline: above,
                frags: (frag_start, self.fragments.len() as u32),
                forced,
            });
            y += line_h;
            emitted += 1;
            s = next;
            for (fid, d) in deferred.drain(..) {
                self.put_float(fid, d, content_w, left, top + y);
            }
        }

        let last_line = self.lines.len() as u32;
        if let Some(o) = self.out.get_mut(block as usize) {
            o.lines = (first_line, last_line);
        }
        // The containing block of a box in a positioned inline element runs from the top left padding corner
        // of the element's first piece to the bottom right one of its last piece (CSS2 10.1 item 4; with
        // left-to-right text, the only direction layer 1 lays out).
        let rects = inline_oof
            .into_iter()
            .map(|(p, cb)| {
                let (f, l) = match cb {
                    InlineCb::At(r) => (r, r),
                    InlineCb::Span(id) => {
                        let at = PadRect { l: p.sx, t: p.sy, r: p.sx, b: p.sy };
                        spans.get(&id).copied().unwrap_or((at, at))
                    }
                };
                (p, [f.l, f.t, (l.r - f.l).max0(), (l.b - f.t).max0()])
            })
            .collect();
        (y, rects)
    }

    /// The decoration line `sp` for a text fragment at `x` with the baseline at `baseline_y`, `w` wide
    /// (css-text-decor-3 2.3: positions from the font's metrics; thickness at least one pixel).
    fn push_decoration(&mut self, sp: &DecorSpec, rf: &ResolvedFont, x: LU, baseline_y: LU, w: LU) {
        let default = if sp.kind == DecorKind::LineThrough { rf.strike_thickness } else { rf.underline_thickness };
        let thickness = LU::from_f32(sp.thickness.unwrap_or(default)).round_px().max(LU::px(1));
        let y = match sp.kind {
            DecorKind::Underline => baseline_y + LU::from_f32(rf.underline_offset).round_px(),
            DecorKind::Overline => baseline_y - rf.asc,
            DecorKind::LineThrough => baseline_y - LU::from_f32(rf.strike_offset).round_px(),
        };
        self.decorations.push(Decoration { kind: sp.kind, style: sp.style, color: sp.color, x, y, w, thickness });
    }
}

/// `position: relative` offset of an inline box, added to its parent's (CSS2 9.3.2).
fn relative_of(bx: &super::boxtree::LayoutBox, parent: Option<(LU, LU)>, cb_w: LU) -> (LU, LU) {
    let (px, py) = parent.unwrap_or((LU::ZERO, LU::ZERO));
    let bs = &bx.style.boxs;
    if bs.position != Position::Relative {
        return (px, py);
    }
    let get = |d: &crate::css::types::Dim, pct: bool| match d {
        crate::css::types::Dim::Lp(l) if !lp_has_percent(l) => Some(LU::from_f32(l.resolve(0.0))),
        crate::css::types::Dim::Lp(l) if pct => Some(LU::from_f32(l.resolve(cb_w.to_f32()))),
        _ => None,
    };
    let [top, right, bottom, left] = &bs.inset;
    let dx = get(left, true).or_else(|| get(right, true).map(|v| -v)).unwrap_or(LU::ZERO);
    let dy = get(top, false).or_else(|| get(bottom, false).map(|v| -v)).unwrap_or(LU::ZERO);
    (px + dx, py + dy)
}

/// `vertical-align` (CSS2 10.8.1): how much the baseline of a box with height `a` above and `d` below
/// its baseline is raised relative to the parent's baseline; and 1 / 2 for `top` / `bottom`.
/// `parent` is (ascent incl. leading, descent incl. leading, x-height, font size) of the parent.
fn vshift(va: &VerticalAlign, a: LU, d: LU, pf: &ResolvedFont, parent: (LU, LU, LU, f32)) -> (LU, u8) {
    let (_, _, xh, size) = parent;
    match va {
        VerticalAlign::Baseline => (LU::ZERO, 0),
        VerticalAlign::Sub => (-sub_super_offset(size, false), 0),
        VerticalAlign::Super => (sub_super_offset(size, true), 0),
        // The top of the box at the top of the parent's content area.
        VerticalAlign::TextTop => (pf.asc - a, 0),
        // The bottom of the box at the bottom of the parent's content area.
        VerticalAlign::TextBottom => (d - pf.desc, 0),
        // The box's midpoint at the parent's baseline plus half its x-height.
        VerticalAlign::Middle => (xh.half() - (a - d).half(), 0),
        VerticalAlign::Top => (LU::ZERO, 1),
        VerticalAlign::Bottom => (LU::ZERO, 2),
        // A length raises the box; a percentage is of the box's own line-height.
        VerticalAlign::Lp(l) => (LU::from_f32(l.resolve((a + d).to_f32())), 0),
    }
}
