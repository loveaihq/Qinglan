//! Vertical writing modes (css-writing-modes-3 3, 6, 7.3).
//!
//! The way it is done: a box whose writing mode differs from that of its containing block's content (a vertical block in a
//! horizontal page, a horizontal table in vertical text, an image in vertical text) is the root of its own *frame*
//! ("orthogonal flow", 7.3; it also starts a formatting context). Everything inside a vertical frame is laid out by the
//! ordinary horizontal code in *logical* terms -- the inline axis is the "x" axis, the block axis the "y" axis -- on a
//! style whose physical sides and sizes have been permuted to match (`logical_view`, built with the box tree). When the
//! frame is done, its geometry is turned into physical coordinates (`turn_frame`: the block axis runs right to left in
//! `vertical-rl` and left to right in `vertical-lr`, the inline axis top to bottom). Nested frames are final already; only
//! their own rectangles are turned. Flex, grid and tables inside vertical text are laid out as horizontal frames (their
//! content is not turned); see docs/decisions.md.
#![allow(clippy::indexing_slicing)]

use std::rc::Rc;

use super::block::{BlockResult, Flow, Geom, border_px, pad_px, relative_offset, resolve_dim, resolve_margin, solve_width};
use super::boxtree::{BoxId, BoxKind, LayoutBox};
use super::units::{LayoutUnit as LU, Margin};
use super::{FragKind, Layouter};
use crate::css::computed::{BorderStyle, BoxStyle, ComputedStyle};
use crate::css::types::{BoxSizing, Display, WritingMode};

/// For each logical side (top, right, bottom, left of the frame's lines) the physical side (same order) it is.
/// `vertical-rl`: block-start is the right, line-left is the top. `vertical-lr`: block-start is the left.
pub(super) fn side_map(wm: WritingMode) -> [usize; 4] {
    match (wm.vertical(), wm.lr()) {
        (false, _) => [0, 1, 2, 3],
        (true, false) => [1, 2, 3, 0],
        (true, true) => [3, 2, 1, 0],
    }
}

/// Same block axis and flow direction (so no frame of its own is needed).
fn same_frame(a: WritingMode, b: WritingMode) -> bool {
    a.vertical() == b.vertical() && (!a.vertical() || a.lr() == b.lr())
}

/// Boxes that are always horizontal inside: replaced elements, and the flex, grid and table boxes this engine lays out
/// horizontally.
fn horizontal_only(b: &LayoutBox) -> bool {
    b.kind == BoxKind::Replaced || b.fallback.is_some_and(|d| !matches!(d, Display::Ruby | Display::RubyText | Display::RubyBase))
}

/// The style as the frame `wm` sees it: sizes, edges, insets and `overflow` of the logical axes.
pub(super) fn logical_view(s: &ComputedStyle, wm: WritingMode) -> ComputedStyle {
    let m = side_map(wm);
    let b = &s.boxs;
    let bs = BoxStyle {
        inset: std::array::from_fn(|k| b.inset[m[k]].clone()),
        margin: std::array::from_fn(|k| b.margin[m[k]].clone()),
        padding: std::array::from_fn(|k| b.padding[m[k]].clone()),
        width: b.height.clone(),
        height: b.width.clone(),
        min_width: b.min_height.clone(),
        min_height: b.min_width.clone(),
        max_width: b.max_height.clone(),
        max_height: b.max_width.clone(),
        overflow_x: b.overflow_y,
        overflow_y: b.overflow_x,
        ..(**b).clone()
    };
    let d = &s.border;
    let bd = BorderStyle { width: std::array::from_fn(|k| d.width[m[k]]), style: std::array::from_fn(|k| d.style[m[k]]), color: std::array::from_fn(|k| d.color[m[k]]) };
    ComputedStyle { boxs: Rc::new(bs), border: Rc::new(bd), ..s.clone() }
}

/// Mark the frame roots of a finished box tree and give every box inside a vertical frame the logical view of its style
/// (the physical one is kept for painting, see `LayoutBox::pstyle`). Only called for pages that use a vertical mode.
pub(super) fn assign_frames(boxes: &mut [LayoutBox], root: BoxId) {
    let mut stack: Vec<(BoxId, WritingMode)> = vec![(root, WritingMode::HorizontalTb)];
    while let Some((id, fw)) = stack.pop() {
        let Some(b) = boxes.get_mut(id as usize) else { continue };
        let block_level = matches!(b.kind, BoxKind::Block | BoxKind::InlineBlock | BoxKind::Replaced);
        let own = if horizontal_only(b) { WritingMode::HorizontalTb } else { b.style.text.writing_mode };
        let mut cf = fw;
        if block_level && !same_frame(own, fw) {
            b.froot = true;
            b.pf = fw;
            cf = own;
        } else if fw.vertical() && !matches!(b.kind, BoxKind::Text | BoxKind::LineBreak) {
            let phys = b.style.clone();
            b.style = Rc::new(logical_view(&phys, fw));
            b.set_phys(phys);
        }
        // A table, flex or grid box lays its content out horizontally whatever its own mode says.
        if horizontal_only(b) {
            cf = WritingMode::HorizontalTb;
        }
        stack.extend(b.children.iter().map(|&c| (c, cf)));
    }
}

/// The physical rectangle of a logical one inside a box whose logical block size is `bh`.
fn phys(bh: LU, lr: bool, x: LU, y: LU, w: LU, h: LU) -> (LU, LU, LU, LU) {
    if lr { (y, x, h, w) } else { (bh - y - h, x, h, w) }
}

impl Layouter<'_> {
    /// Lay out the frame root `id` (css-writing-modes-3 7.3): its box as the containing block's frame sees it, its content
    /// in a frame of its own. The result is in the containing block's logical terms, like any other block's.
    pub(super) fn layout_frame(&mut self, id: BoxId, cb_w: LU, cb_h: Option<LU>, flow: Flow) -> BlockResult {
        let tree = self.tree;
        let bx = tree.get(id);
        let pw = bx.pf;
        let iw = if horizontal_only(bx) { WritingMode::HorizontalTb } else { bx.style.text.writing_mode };
        let (pm, im) = (side_map(pw), side_map(iw));
        let (bs, bd) = (&bx.style.boxs, &bx.style.border);
        // The percentages of widths and heights refer to the containing block's physical width and height.
        let (base_w, base_h) = if pw.vertical() { (cb_h, Some(cb_w)) } else { (Some(cb_w), cb_h) };
        let pad: [LU; 4] = std::array::from_fn(|i| pad_px(&bs.padding[i], cb_w));
        let bor: [LU; 4] = std::array::from_fn(|i| border_px(bd.width[i]));
        let mar: [Option<LU>; 4] = std::array::from_fn(|i| resolve_margin(&bs.margin[i], cb_w));
        let (h_ex, v_ex) = (pad[1] + pad[3] + bor[1] + bor[3], pad[0] + pad[2] + bor[0] + bor[2]);
        let bb = bs.box_sizing == BoxSizing::BorderBox;
        let cw = |v: LU| if bb { (v - h_ex).max0() } else { v };
        let ch = |v: LU| if bb { (v - v_ex).max0() } else { v };
        let (spec_w, spec_h) = (resolve_dim(&bs.width, base_w).map(cw), resolve_dim(&bs.height, base_h).map(ch));
        let (min_w, min_h) = (resolve_dim(&bs.min_width, base_w).map_or(LU::ZERO, cw), resolve_dim(&bs.min_height, base_h).map_or(LU::ZERO, ch));
        let (max_w, max_h) = (resolve_dim(&bs.max_width, base_w).map(cw), resolve_dim(&bs.max_height, base_h).map(ch));
        // Margins and edges on the sides of the containing block's frame.
        let (m_top, m_bot, m_l, m_r) = (mar[pm[0]], mar[pm[2]], mar[pm[3]], mar[pm[1]]);
        let hp = pad[pm[1]] + pad[pm[3]] + bor[pm[1]] + bor[pm[3]];
        let vp = pad[pm[0]] + pad[pm[2]] + bor[pm[0]] + bor[pm[2]];

        let vertical = iw.vertical();
        let (spec_i, min_i, max_i, ex_i, spec_b, min_b, max_b) = if vertical { (spec_h, min_h, max_h, v_ex, spec_w, min_w, max_w) } else { (spec_w, min_w, max_w, h_ex, spec_h, min_h, max_h) };
        let same_axis = pw.vertical() == vertical;
        let screen = if vertical { self.viewport.1 } else { self.viewport.0 };
        let avail = if same_axis {
            cb_w - m_l.unwrap_or(LU::ZERO) - m_r.unwrap_or(LU::ZERO) - hp
        } else {
            let room = if flow.root { screen } else { cb_h.unwrap_or(screen) };
            room - m_top.unwrap_or(LU::ZERO) - m_bot.unwrap_or(LU::ZERO) - vp
        }
        .max0();
        let repl = if bx.kind == BoxKind::Replaced { Some(self.replaced_size_final(id, cb_w, cb_h)) } else { None };
        let inline = if let Some((rw, _)) = repl {
            rw
        } else {
            let v = match spec_i {
                Some(v) => v,
                // The root fills the viewport; so does a box of the same axis; any other orthogonal flow is fit-content (7.3).
                None if same_axis || flow.root => avail,
                None => {
                    self.in_frame = id;
                    let (mn, mx) = self.content_widths(id);
                    self.in_frame = super::boxtree::NO_BOX;
                    mn.max(avail).min(mx)
                }
            };
            max_i.map_or(v, |m| v.min(m)).max(min_i)
        };

        self.no_stop += 1;
        if vertical {
            let ipad: [LU; 4] = std::array::from_fn(|k| pad[im[k]]);
            let ibor: [LU; 4] = std::array::from_fn(|k| bor[im[k]]);
            let g = Geom {
                pad: ipad,
                bor: ibor,
                h_extra: ipad[1] + ipad[3] + ibor[1] + ibor[3],
                v_extra: ipad[0] + ipad[2] + ibor[0] + ibor[2],
                m_top: LU::ZERO,
                m_bot: LU::ZERO,
                content_w: inline,
                ml: LU::ZERO,
                spec_h: spec_b.map(|h| max_b.map_or(h, |m| h.min(m)).max(min_b)),
                h_zero_or_auto: false,
                min_h: min_b,
                max_h: max_b,
                repl: None,
            };
            let h_extra = g.h_extra;
            self.inner_geom = Some(g);
            self.layout_block(id, inline + h_extra, None, Flow { inner: true, ..Flow::default() });
        } else {
            let forced_h = repl.map(|r| r.1);
            self.layout_block(id, inline + ex_i, None, Flow { inner: true, forced_w: Some(inline), forced_h, ..Flow::default() });
        }
        self.no_stop -= 1;

        let o = self.out.get(id as usize).copied().unwrap_or_default();
        let (phys_w, phys_h) = if vertical { (o.h, o.w) } else { (o.w, o.h) };
        if vertical {
            self.turn_frame(id, iw);
        }
        let (w, h) = if pw.vertical() { (phys_h, phys_w) } else { (phys_w, phys_h) };
        if let Some(o) = self.out.get_mut(id as usize) {
            o.w = w;
            o.h = h;
        }
        let (_, ml, _) = solve_width(cb_w, Some((w - hp).max0()), m_l, m_r, hp, LU::ZERO, None, false);
        let (mt, mb) = (Margin::from(m_top.unwrap_or(LU::ZERO)), Margin::from(m_bot.unwrap_or(LU::ZERO)));
        BlockResult { h, ml, mt, mb, top_only: mt, bot_only: mb, through: false, rel: relative_offset(&bx.style.boxs, cb_w, cb_h) }
    }

    /// Turn the logical geometry of the frame rooted at `id` (laid out in the writing mode `wm`) into physical geometry, relative
    /// to the same boxes: the rectangles of the boxes, lines, fragments, decorations and the marker.
    fn turn_frame(&mut self, id: BoxId, wm: WritingMode) {
        let tree = self.tree;
        let bx = tree.get(id);
        let bh = self.out.get(id as usize).map_or(LU::ZERO, |o| o.h);
        let lr = wm.lr();
        self.map_kids(&bx.children, wm, bh);
        let (la, lb, mk) = self.out.get(id as usize).map_or((0, 0, 0), |o| (o.lines.0, o.lines.1, o.marker));
        for li in la..lb {
            let Some(line) = self.lines.get(li as usize).copied() else { continue };
            for fi in line.frags.0..line.frags.1 {
                self.turn_fragment(fi as usize, bh, lr);
            }
            let (x, y, w, h) = phys(bh, lr, line.x, line.y, line.w, line.h);
            if let Some(l) = self.lines.get_mut(li as usize) {
                (l.x, l.y, l.w, l.h) = (x, y, w, h);
            }
        }
        if mk > 0
            && let Some(f) = self.markers.get_mut(mk as usize - 1)
        {
            (f.x, f.y, f.w, f.h) = phys(bh, lr, f.x, f.y, f.w, f.h);
        }
    }

    fn map_kids(&mut self, kids: &[BoxId], wm: WritingMode, bh: LU) {
        let tree = self.tree;
        for &c in kids {
            let cb = tree.get(c);
            match cb.kind {
                BoxKind::Block | BoxKind::InlineBlock | BoxKind::Replaced => {
                    if !cb.froot {
                        self.turn_frame(c, wm);
                    }
                    if let Some(o) = self.out.get_mut(c as usize) {
                        (o.x, o.y, o.w, o.h) = phys(bh, wm.lr(), o.x, o.y, o.w, o.h);
                    }
                }
                BoxKind::Inline => self.map_kids(&cb.children, wm, bh),
                _ => {}
            }
        }
    }

    fn turn_fragment(&mut self, fi: usize, bh: LU, lr: bool) {
        let Some(mut f) = self.fragments.get(fi).copied() else { return };
        let thick = f.h;
        (f.x, f.y, f.w, f.h) = phys(bh, lr, f.x, f.y, f.w, f.h);
        if let FragKind::Text { decor, v, ref mut asc, .. } = f.kind {
            // A sideways run: the baseline is `asc` from the block-start edge of the fragment; physically from its left edge.
            if v == 1 && !lr {
                *asc = thick - *asc;
            }
            for di in decor.0..decor.1 {
                if let Some(d) = self.decorations.get_mut(di as usize) {
                    (d.x, d.y, d.w, d.thickness) = phys(bh, lr, d.x, d.y, d.w, d.thickness);
                }
            }
        }
        if let Some(slot) = self.fragments.get_mut(fi) {
            *slot = f;
        }
    }
}
