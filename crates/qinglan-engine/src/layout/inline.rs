//! Inline formatting: white-space processing (css-text-3 4.1), shaping, line breaking (css-text-3 5),
//! line boxes (CSS2 10.8, css-inline-3) and `text-align`.
//!
//! One block container whose children are inline-level is one *paragraph*:
//!
//! 1. `collect` walks the inline boxes and turns the text into *items* (one per text box, split at
//!    forced breaks) with white space collapsed (Phase I, 4.1.1 and 4.1.3).
//! 2. Each item is shaped by the platform and cut into *units*, one per glyph cluster: the smallest
//!    piece a line may be cut into.
//! 3. The platform reports the UAX #14 break opportunities of the whole paragraph text; `opp_before`
//!    adds `word-break` and `white-space` on top, `break_line` applies `overflow-wrap`.
//! 4. Lines are filled greedily; each line is trimmed (Phase II, 4.1.2), aligned and measured.
//!
//! Not yet (1b-2): `vertical-align` other than `baseline`, inline padding/border/margin, atomic
//! inlines, `text-indent`, `justify`, letter- and word-spacing, text transforms, decorations.

use std::rc::Rc;

use super::boxtree::{BoxId, BoxKind, NO_BOX};
use super::units::LayoutUnit as LU;
use super::{FragKind, Fragment, Layouter, Line, ResolvedFont};
use crate::css::computed::{ComputedStyle, LineHeight, slant_of};
use crate::css::types::{OverflowWrap, TextAlign, TextWrapMode, WhiteSpaceCollapse, WordBreak};
use crate::html::dom::NodeData;
use crate::platform::FontQuery;
use crate::textclass::{is_cjk_break, is_closing_punct, is_east_asian_wide_non_hangul, is_opening_punct, is_space_like};

/// A run of text of one text box, or a forced line break.
struct Item {
    forced: bool,
    /// The text after white-space collapsing; empty for forced breaks.
    text: String,
    /// The text box (or the `<br>` box).
    box_id: BoxId,
    /// The inline box (or block) the text sits in; its chain of ancestors up to the paragraph's
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
}

fn make_item(box_id: BoxId, owner: BoxId, style: &Rc<ComputedStyle>, text: String, forced: bool) -> Item {
    let t = &style.text;
    Item {
        forced,
        text,
        box_id,
        owner,
        style: style.clone(),
        collapsible: matches!(t.white_space_collapse, WhiteSpaceCollapse::Collapse | WhiteSpaceCollapse::PreserveBreaks),
        wrap: t.text_wrap_mode == TextWrapMode::Wrap,
        break_spaces: t.white_space_collapse == WhiteSpaceCollapse::BreakSpaces,
        word_break: t.word_break,
        emergency: matches!(t.overflow_wrap, OverflowWrap::Anywhere | OverflowWrap::BreakWord) || t.word_break == WordBreak::BreakWord,
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
}

impl Collapser {
    fn reset_line(&mut self) {
        *self = Collapser::default();
    }

    /// "Any sequence of collapsible spaces and tabs immediately preceding ... a segment break is
    /// removed" (4.1.1): take back the space just written.
    fn drop_trailing_space(&mut self, items: &mut [Item], cur: &mut String) {
        if !self.last_space_collapsible {
            return;
        }
        if cur.ends_with(' ') {
            cur.pop();
        } else if let Some(it) = items.iter_mut().rev().find(|i| !i.forced && !i.text.is_empty())
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

fn add_text(st: &mut Collapser, items: &mut Vec<Item>, bid: BoxId, owner: BoxId, style: &Rc<ComputedStyle>, text: &str) {
    let collapse = style.text.white_space_collapse;
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
                    if !cur.is_empty() {
                        items.push(make_item(bid, owner, style, std::mem::take(&mut cur), false));
                    }
                    items.push(make_item(bid, owner, style, String::new(), true));
                    st.reset_line();
                    st.skip_spaces = true;
                    continue;
                }
                WhiteSpaceCollapse::Preserve | WhiteSpaceCollapse::BreakSpaces => {
                    if !cur.is_empty() {
                        items.push(make_item(bid, owner, style, std::mem::take(&mut cur), false));
                    }
                    items.push(make_item(bid, owner, style, String::new(), true));
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
    if !cur.is_empty() {
        items.push(make_item(bid, owner, style, cur, false));
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum UnitKind {
    Normal,
    Tab,
    Forced,
}

/// One glyph cluster: the smallest piece of text a line can be cut into.
struct Unit {
    kind: UnitKind,
    item: u32,
    /// Byte range in the item's text.
    start: u32,
    end: u32,
    /// Advance (for tabs: the tab size).
    adv: LU,
    /// Range in the layouter's glyph array.
    glyphs: (u32, u32),
    first: char,
    last: char,
    collapsible_space: bool,
    hang_space: bool,
}

struct Para<'p> {
    items: &'p [Item],
    units: Vec<Unit>,
    /// Offset of each item's text in the paragraph text.
    base: Vec<usize>,
    breaks: Vec<usize>,
}

impl Para<'_> {
    /// May a line end between unit `j - 1` and unit `j`? Platform opportunities (UAX #14) plus
    /// `white-space`, `word-break` (css-text-3 5.1, 5.2, 4.1.1).
    fn opp_before(&self, j: usize) -> bool {
        let (Some(u), Some(p)) = (self.units.get(j), j.checked_sub(1).and_then(|k| self.units.get(k))) else { return false };
        if u.kind == UnitKind::Forced || p.kind == UnitKind::Forced {
            return false;
        }
        let (Some(iu), Some(ip)) = (self.items.get(u.item as usize), self.items.get(p.item as usize)) else { return false };
        let same = u.item == p.item;
        // The wrapping style of the nearest common ancestor decides; approximated by "either side wraps".
        let wrap_ok = if same { iu.wrap } else { iu.wrap || ip.wrap };
        if !wrap_ok {
            return false;
        }
        let off = self.base.get(u.item as usize).copied().unwrap_or(0) + u.start as usize;
        let mut ok = self.breaks.binary_search(&off).is_ok();
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
    /// after the overflow (or inside the word when `overflow-wrap` allows).
    fn break_line(&self, s: usize, avail: LU) -> (usize, bool) {
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
            w_all += self.width_at(u, w_all);
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
    // `start`/`end` are left/right (layer 1 has no right-to-left layout); `justify` is 1b-2.
    match a {
        TextAlign::End | TextAlign::Right => Align::Right,
        TextAlign::Center => Align::Center,
        _ => Align::Left,
    }
}

struct Group {
    item: u32,
    a: usize,
    b: usize,
    x: LU,
    w: LU,
}

impl Layouter<'_> {
    /// Metrics of the primary font of `style`, cached per font description.
    pub(super) fn font_for(&mut self, style: &ComputedStyle) -> Rc<ResolvedFont> {
        let key = (Rc::as_ptr(&style.font) as usize, style.text.lang.clone());
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
        let r = Rc::new(ResolvedFont { size: f.size, asc, desc, lead_top, lead_bottom: diff - lead_top });
        self.fonts.insert(key, r.clone());
        r
    }

    /// An inline box that has content on the current line adds its own height (CSS2 10.8.1) -- and so
    /// do all inline boxes around it up to the paragraph's block.
    fn contribute(&mut self, mut b: BoxId, block: BoxId, asc: &mut LU, desc: &mut LU) {
        let tree = self.tree;
        while b != block && b != NO_BOX {
            let line_no = self.line_no;
            let Some(slot) = self.stamp.get_mut(b as usize) else { break };
            if *slot == line_no {
                break;
            }
            *slot = line_no;
            let bx = tree.get(b);
            let rf = self.font_for(&bx.style);
            *asc = (*asc).max(rf.asc + rf.lead_top);
            *desc = (*desc).max(rf.desc + rf.lead_bottom);
            b = bx.parent;
        }
    }

    fn collect(&self, bid: BoxId, st: &mut Collapser, items: &mut Vec<Item>) {
        let b = self.tree.get(bid);
        match b.kind {
            BoxKind::Text => {
                if let Some(NodeData::Text(t)) = self.doc.node(b.node).map(|n| &n.data) {
                    add_text(st, items, bid, b.parent, &b.style, t);
                }
            }
            BoxKind::LineBreak => {
                st.reset_line();
                items.push(make_item(bid, bid, &b.style, String::new(), true));
            }
            BoxKind::Inline => {
                for &c in &b.children {
                    self.collect(c, st, items);
                }
            }
            BoxKind::Block => {}
        }
    }

    /// Lay out the inline content of `block` into lines. Positions are relative to the block's border
    /// box: the content box starts at `(left, top)` and is `content_w` wide. Returns the height of the lines.
    pub(super) fn layout_inline(&mut self, block: BoxId, content_w: LU, left: LU, top: LU) -> LU {
        let tree = self.tree;
        let bx = tree.get(block);
        let mut items: Vec<Item> = Vec::new();
        let mut st = Collapser::default();
        for &c in &bx.children {
            self.collect(c, &mut st, &mut items);
        }

        // Shape every item and cut it into units.
        let mut units: Vec<Unit> = Vec::new();
        let mut base: Vec<usize> = Vec::with_capacity(items.len());
        let mut para_text = String::new();
        for (ii, item) in items.iter().enumerate() {
            base.push(para_text.len());
            if item.forced {
                units.push(Unit {
                    kind: UnitKind::Forced,
                    item: ii as u32,
                    start: 0,
                    end: 0,
                    adv: LU::ZERO,
                    glyphs: (0, 0),
                    first: '\n',
                    last: '\n',
                    collapsible_space: false,
                    hang_space: false,
                });
                continue;
            }
            para_text.push_str(&item.text);
            if item.text.is_empty() {
                continue;
            }
            let rf = self.font_for(&item.style);
            let f = &item.style.font;
            let q = FontQuery { families: &f.family, weight: f.weight, slant: slant_of(f.style), lang: item.style.text.lang.as_deref() };
            let glyphs = self.platform.shape(&item.text, &q, rf.size);
            let gbase = self.glyphs.len() as u32;
            self.glyphs.extend_from_slice(&glyphs);
            let mut tab_size: Option<LU> = None;
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
                let s = item.text.get(cs as usize..ce as usize).unwrap_or("");
                let first = s.chars().next().unwrap_or('\u{FFFD}');
                let last = s.chars().next_back().unwrap_or('\u{FFFD}');
                let single_space = s == " ";
                let is_tab = s == "\t" && !item.collapsible;
                let mut kind = UnitKind::Normal;
                let mut width = LU::from_f32(adv);
                if is_tab {
                    kind = UnitKind::Tab;
                    // tab-size is 8 (the initial value): eight space advances.
                    let ts = *tab_size.get_or_insert_with(|| {
                        let sp = self.platform.shape(" ", &q, rf.size).first().map_or(0.0, |g| g.advance);
                        LU::from_f32(sp * 8.0)
                    });
                    width = ts;
                }
                units.push(Unit {
                    kind,
                    item: ii as u32,
                    start: cs,
                    end: ce,
                    adv: width,
                    glyphs: (gbase + i as u32, gbase + j as u32),
                    first,
                    last,
                    collapsible_space: item.collapsible && single_space,
                    // Preserved spaces at the end of a wrapped line hang (4.1.2); `break-spaces` ones do not.
                    hang_space: !item.collapsible && item.wrap && !item.break_spaces && (single_space || is_tab),
                });
                i = j;
            }
        }
        let lang = bx.style.text.lang.as_deref();
        let breaks = self.platform.line_breaks(&para_text, lang);

        // Move the texts into the result; fragments refer to them.
        let mut text_ids: Vec<u32> = Vec::with_capacity(items.len());
        let mut item_texts: Vec<String> = Vec::new();
        for item in &mut items {
            text_ids.push((self.texts.len() + item_texts.len()) as u32);
            item_texts.push(std::mem::take(&mut item.text));
        }
        // `items[..].text` is only needed for the units (already cut), so move the strings now.
        let para = Para { items: &items, units, base, breaks };

        let strut_font = self.font_for(&bx.style);
        let strut_asc = strut_font.asc + strut_font.lead_top;
        let strut_desc = strut_font.desc + strut_font.lead_bottom;
        let align = align_of(bx.style.text.text_align);
        let first_line = self.lines.len() as u32;

        let mut y = LU::ZERO;
        let mut s = 0usize;
        let mut groups: Vec<Group> = Vec::new();
        let mut widths: Vec<LU> = Vec::new();
        let n = para.units.len();
        loop {
            // Collapsible spaces at the start of a line are removed (4.1.2).
            while para.units.get(s).is_some_and(|u| u.collapsible_space) {
                s += 1;
            }
            if s >= n {
                break;
            }
            let (mut end, forced) = para.break_line(s, content_w);
            if end <= s && !forced {
                end = s + 1;
            }
            let next = if forced { end + 1 } else { end };
            // ... and at the end of a line.
            let mut e = end;
            while e > s && para.units.get(e - 1).is_some_and(|u| u.collapsible_space) {
                e -= 1;
            }
            if e == s && !forced {
                s = next;
                continue;
            }

            // Measure.
            groups.clear();
            widths.clear();
            let mut x = LU::ZERO;
            for k in s..e {
                let Some(u) = para.units.get(k) else { break };
                let w = para.width_at(u, x);
                widths.push(w);
                match groups.last_mut() {
                    Some(g) if g.item == u.item => {
                        g.b = k + 1;
                        g.w += w;
                    }
                    _ => groups.push(Group { item: u.item, a: k, b: k + 1, x, w }),
                }
                x += w;
            }
            // Hanging spaces do not count for alignment, except that ones before a forced break
            // hang only if they would overflow (4.1.2).
            let mut hang = LU::ZERO;
            let mut k = e;
            while k > s && para.units.get(k - 1).is_some_and(|u| u.hang_space) {
                hang += widths.get(k - 1 - s).copied().unwrap_or(LU::ZERO);
                k -= 1;
            }
            if forced && x <= content_w {
                hang = LU::ZERO;
            }
            let line_w = x - hang;
            let free = content_w - line_w;
            let offset = match align {
                Align::Left => LU::ZERO,
                Align::Center if free > LU::ZERO => free.half(),
                Align::Right if free > LU::ZERO => free,
                _ => LU::ZERO,
            };

            // Height: the strut and every inline box with content on this line (CSS2 10.8.1).
            self.line_no += 1;
            let (mut asc, mut desc) = (strut_asc, strut_desc);
            for g in &groups {
                if let Some(it) = para.items.get(g.item as usize) {
                    self.contribute(it.owner, block, &mut asc, &mut desc);
                }
            }
            let forced_item = if forced { para.units.get(end).and_then(|u| para.items.get(u.item as usize)) } else { None };
            if let Some(it) = forced_item {
                self.contribute(it.owner, block, &mut asc, &mut desc);
            }
            let line_h = asc + desc;

            // Fragments.
            let frag_start = self.fragments.len() as u32;
            for g in &groups {
                let (Some(it), Some(ua), Some(ub)) = (para.items.get(g.item as usize), para.units.get(g.a), g.b.checked_sub(1).and_then(|k| para.units.get(k))) else {
                    continue;
                };
                let rf = self.font_for(&it.style);
                self.fragments.push(Fragment {
                    box_id: it.box_id,
                    kind: FragKind::Text {
                        text: text_ids.get(g.item as usize).copied().unwrap_or(0),
                        start: ua.start,
                        end: ub.end,
                        glyphs: (ua.glyphs.0, ub.glyphs.1),
                    },
                    x: left + offset + g.x,
                    y: top + y + asc - rf.asc,
                    w: g.w,
                    h: rf.asc + rf.desc,
                });
            }
            if let Some(it) = forced_item {
                let rf = self.font_for(&it.style);
                self.fragments.push(Fragment {
                    box_id: it.box_id,
                    kind: FragKind::Br,
                    x: left + offset + x,
                    y: top + y + asc - rf.asc,
                    w: LU::ZERO,
                    h: rf.asc + rf.desc,
                });
            }
            self.lines.push(Line {
                x: left,
                y: top + y,
                w: content_w,
                h: line_h,
                baseline: asc,
                frags: (frag_start, self.fragments.len() as u32),
                forced,
            });
            y += line_h;
            s = next;
        }

        self.texts.extend(item_texts);
        let last_line = self.lines.len() as u32;
        if let Some(o) = self.out.get_mut(block as usize) {
            o.lines = (first_line, last_line);
        }
        y
    }
}
