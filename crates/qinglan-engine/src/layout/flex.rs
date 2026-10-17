//! Flexbox (css-flexbox-1): flex containers and their items, including the intrinsic sizes of a container
//! (9.9), absolutely positioned children (4.1), `visibility: collapse` items (4.4, simplified) and the gaps of
//! css-align-3 8.
//!
//! A flex container is a formatting context root laid out like a table: whole (no first-screen stop line inside),
//! knowing only its containing block's width. The algorithm follows css-flexbox-1 9 step by step, working in
//! logical main / cross coordinates (`flex-start` first); the physical position of an item is worked out last, from
//! `Axes`.
//!
//! Cost: every item is laid out once at its final size. A column container (and a row container's items with a
//! percentage-free `height`) need an item's height *before* the sizes are final; that is a layout that is thrown
//! away again (`flex_measure`) and remembered by (item, width), so nested containers measure each level once and the
//! work stays polynomial (a 20-deep nest of containers is measured by `deep_nesting_is_not_exponential`). A row
//! container lays its items out once at their final widths; only stretched items whose content depends on their
//! own height (nested containers, tables, percentage heights, positioned descendants) are laid out again when the
//! line is taller than they are, and the work spent on such repeats is capped (`FLEX_REPEAT_SHARE`); past the cap the
//! stretched item just gets the taller box.

use super::block::{BlockResult, Flow, Geom, border_px, lp_has_percent, pad_px, relative_offset, resolve_dim, resolve_margin};
use super::boxtree::{BoxId, BoxKind};
use super::units::{LayoutUnit as LU, Margin};
use super::{Layouter, StaticAlign};
use crate::css::types::{AlignKind, AlignValue, BoxSizing, Dim, Direction, Display, FlexBasis, FlexDirection, FlexWrap, Overflow, Position, Visibility};
use crate::css::values::Lp;

/// The share of the page's work budget (1 / this) that laying stretched items out a second time may use.
const FLEX_REPEAT_SHARE: u64 = 8;

fn r(l: LU) -> i64 {
    i64::from(l.raw())
}

/// `a / b` rounded to the nearest layout unit (the browsers' `space-*` values come out that way).
fn div_round(a: i64, b: i64) -> i64 {
    if b <= 0 { 0 } else { (a + b / 2) / b }
}

fn lu(v: i64) -> LU {
    LU::from_raw(v)
}

/// Where a box sits in free space.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Pos {
    Start,
    Mid,
    End,
}

/// The physical meaning of the logical axes of one container. Side numbers are top 0, right 1, bottom 2, left 3.
#[derive(Clone, Copy)]
struct Axes {
    col: bool,
    /// The main-start side is the far one (`row-reverse`, `column-reverse`, `row` in `direction: rtl`).
    rev: bool,
    /// `flex-direction` is one of the `-reverse` ones.
    rev_flex: bool,
    wrap_rev: bool,
    /// The cross-start side is the far one (`wrap-reverse`, a column in `direction: rtl`).
    crev: bool,
    /// The sides at the main start / end and cross start / end.
    ms: usize,
    me: usize,
    cs: usize,
    ce: usize,
}

impl Axes {
    fn new(col: bool, rev_flex: bool, rtl: bool, wrap_rev: bool) -> Axes {
        let rev = rev_flex ^ (!col && rtl);
        let crev = wrap_rev ^ (col && rtl);
        let (near_m, far_m) = if col { (0, 2) } else { (3, 1) };
        let (near_c, far_c) = if col { (3, 1) } else { (0, 2) };
        let (ms, me) = if rev { (far_m, near_m) } else { (near_m, far_m) };
        let (cs, ce) = if crev { (far_c, near_c) } else { (near_c, far_c) };
        Axes { col, rev, rev_flex, wrap_rev, crev, ms, me, cs, ce }
    }

    /// Where an alignment keyword puts a box on the main axis (css-flexbox-1 8.2; `start` / `end` / `left` / `right`
    /// are the physical ones of css-align-3).
    fn pos_main(&self, k: AlignKind) -> Pos {
        match k {
            AlignKind::FlexEnd => Pos::End,
            AlignKind::Center => Pos::Mid,
            AlignKind::Start => {
                if self.rev_flex {
                    Pos::End
                } else {
                    Pos::Start
                }
            }
            AlignKind::End => {
                if self.rev_flex {
                    Pos::Start
                } else {
                    Pos::End
                }
            }
            // css-align-3 6.3: where `left` / `right` do not run along the axis they behave as `start`.
            AlignKind::Left | AlignKind::Right if self.col => self.pos_main(AlignKind::Start),
            AlignKind::Left => {
                if self.rev {
                    Pos::End
                } else {
                    Pos::Start
                }
            }
            AlignKind::Right => {
                if self.rev {
                    Pos::Start
                } else {
                    Pos::End
                }
            }
            _ => Pos::Start,
        }
    }

    /// The same on the cross axis (css-flexbox-1 8.3, 8.4).
    fn pos_cross(&self, k: AlignKind) -> Pos {
        match k {
            AlignKind::FlexEnd => Pos::End,
            AlignKind::Center => Pos::Mid,
            AlignKind::Start | AlignKind::SelfStart => {
                if self.wrap_rev {
                    Pos::End
                } else {
                    Pos::Start
                }
            }
            AlignKind::End | AlignKind::SelfEnd => {
                if self.wrap_rev {
                    Pos::Start
                } else {
                    Pos::End
                }
            }
            _ => Pos::Start,
        }
    }
}

/// The container facts the items need.
struct Ctx {
    ax: Axes,
    /// Content width of the container (always definite).
    cw: LU,
    inner_main: Option<LU>,
    inner_cross: Option<LU>,
    wrap: bool,
    align_items: AlignValue,
}

#[derive(Clone)]
struct Item {
    id: BoxId,
    pad: [LU; 4],
    bor: [LU; 4],
    /// `None` is `auto`.
    marg: [Option<LU>; 4],
    /// The margins as used (auto resolved), by side.
    mu: [LU; 4],
    grow: f32,
    shrink: f32,
    /// Flex base size, hypothetical main size, used min and max main size, target main size: all content box.
    base: LU,
    hyp: LU,
    min: LU,
    max: Option<LU>,
    target: LU,
    frozen: bool,
    /// Main-axis padding, border and margins that are not `auto`.
    extra: i64,
    collapsed: bool,
    /// The content-box main size the item is laid out at (the target, except for collapsed items).
    lay_main: LU,
    align: AlignKind,
    safe: bool,
    cross_auto: bool,
    /// Content-box min / max cross size.
    cmin: LU,
    cmax: Option<LU>,
    /// Border-box cross size (margins not included).
    cross: LU,
    baseline: Option<LU>,
    last_baseline: Option<LU>,
    /// A replaced element with an intrinsic size and auto height: its (width, height).
    ratio: Option<(LU, LU)>,
    /// The cross size the item is stretched to (content box), when it is.
    stretch: Option<LU>,
    /// The post-flexing main size is definite for percentages inside (9.8): the container's main size is, or the
    /// flex basis is.
    def_main: bool,
    /// The content height of a column item, measured at its width.
    content_h: Option<LU>,
    rel: (LU, LU),
    /// Border-box start of the item along the main / cross axis, relative to the content box of the container.
    mpos: LU,
    cpos: LU,
}

impl Item {
    fn side(&self, a: &[LU; 4], i: usize) -> LU {
        a.get(i).copied().unwrap_or(LU::ZERO)
    }

    fn pb(&self, a: usize, b: usize) -> LU {
        self.side(&self.pad, a) + self.side(&self.pad, b) + self.side(&self.bor, a) + self.side(&self.bor, b)
    }

    fn m0(&self, i: usize) -> LU {
        self.marg.get(i).copied().flatten().unwrap_or(LU::ZERO)
    }

    fn is_auto(&self, i: usize) -> bool {
        self.marg.get(i).is_none_or(Option::is_none)
    }

    fn mu(&self, i: usize) -> LU {
        self.side(&self.mu, i)
    }

    /// The outer hypothetical main size (9.3: line breaking).
    fn outer_hyp(&self) -> i64 {
        r(self.hyp) + self.extra
    }
}

struct FlexLine {
    s: usize,
    e: usize,
    /// Sum of the outer hypothetical main sizes and the gaps.
    sum: i64,
    cross: LU,
    /// Logical offset of the line from the cross start of the content box.
    pos: LU,
}

impl Layouter<'_> {
    /// Lay out the flex container `id` (css-flexbox-1 9).
    pub(super) fn layout_flex(&mut self, id: BoxId, cb_w: LU, cb_h: Option<LU>, flow: Flow) -> BlockResult {
        let tree = self.tree;
        let bx = tree.get(id);
        if !self.charge(8 + bx.children.len() as u64) {
            let z = Margin::default();
            return BlockResult { h: LU::ZERO, ml: LU::ZERO, mt: z, mb: z, top_only: z, bot_only: z, through: false, rel: (LU::ZERO, LU::ZERO) };
        }
        let g = self.block_geometry(id, cb_w, cb_h, flow);
        let (content_left, content_top) = (g.bor[3] + g.pad[3], g.bor[0] + g.pad[0]);
        let saved = (self.pos, self.touched, std::mem::take(&mut self.fs));
        self.pos = (LU::ZERO, LU::ZERO);
        let positioned = bx.style.boxs.position != Position::Static;
        if positioned {
            self.cb_stack.push(id);
        }
        let pmark = self.pending.len();
        let content_h = self.flex_inner(id, &g, (content_left, content_top));
        self.finish_box(id, &g, content_h, content_top);
        (self.pos, self.touched, self.fs) = saved;
        if positioned {
            self.cb_stack.pop();
            self.resolve_pending(Some(id), pmark);
        }
        let h = self.out.get(id as usize).map_or(LU::ZERO, |o| o.h);
        let (mt, mb) = (Margin::from(g.m_top), Margin::from(g.m_bot));
        BlockResult { h, ml: g.ml, mt, mb, top_only: mt, bot_only: mb, through: false, rel: relative_offset(&bx.style.boxs, cb_w, cb_h) }
    }

    /// The border-box height of `id` laid out with a content width of `w` and no definite height around it; the layout
    /// is taken back (it only measures). Remembered by item and width, so a nest of containers measures each level once.
    pub(super) fn flex_measure(&mut self, id: BoxId, cb_w: LU, w: LU, est: Option<LU>) -> LU {
        if let Some(&(mw, h)) = self.measured.get(&id)
            && mw == w
        {
            return h;
        }
        let mark = self.mark();
        let flow = Flow { forced_w: Some(w), ignore_h: true, ..Flow::default() };
        match est {
            // First-screen-first: the item is measured up to the stop line only (its height is then a lower bound that
            // still reaches the line, which is all the caller needs for what lies above it).
            Some(e) => {
                self.item_stop(e, |s| s.layout_block(id, cb_w, None, flow));
            }
            None => {
                self.no_stop += 1;
                self.layout_block(id, cb_w, None, flow);
                self.no_stop -= 1;
            }
        }
        let h = self.out.get(id as usize).map_or(LU::ZERO, |o| o.h);
        self.restore(mark);
        self.measured.insert(id, (w, h));
        h
    }

    /// A size property as a content-box length. The keywords (`min-content` ...) exist for widths only.
    #[allow(clippy::too_many_arguments)]
    fn flex_dim(&mut self, id: BoxId, d: &Dim, base: Option<LU>, avail: LU, is_w: bool, border_box: bool, extra: LU) -> Option<LU> {
        let to_c = |v: LU| if border_box { (v - extra).max0() } else { v };
        if is_w && let Some(v) = self.kw_size(id, d, avail) {
            return Some(v);
        }
        resolve_dim(d, base).map(to_c)
    }

    /// Is `id` laid out differently when its own height is forced (so that stretching it needs a new layout)?
    pub(super) fn height_sensitive(&mut self, id: BoxId) -> bool {
        let tree = self.tree;
        let bx = tree.get(id);
        if matches!(bx.fallback, Some(Display::Flex | Display::InlineFlex | Display::Table | Display::InlineTable)) {
            return true;
        }
        self.desc_sensitive(id)
    }

    /// Percentage heights, positioned descendants: what depends on the height of the box that holds them.
    fn desc_sensitive(&mut self, id: BoxId) -> bool {
        if self.flex_sens.is_empty() {
            self.flex_sens = vec![0; self.tree.boxes.len()];
        }
        match self.flex_sens.get(id as usize) {
            Some(1) => return false,
            Some(2) => return true,
            _ => {}
        }
        let tree = self.tree;
        let bx = tree.get(id);
        let bs = &bx.style.boxs;
        let pct = |d: &Dim| matches!(d, Dim::Lp(l) if lp_has_percent(l));
        let mut s = bx.oof || pct(&bs.height) || pct(&bs.min_height) || pct(&bs.max_height);
        if !s {
            for &c in &bx.children {
                if self.desc_sensitive(c) {
                    s = true;
                    break;
                }
            }
        }
        if let Some(v) = self.flex_sens.get_mut(id as usize) {
            *v = if s { 2 } else { 1 };
        }
        s
    }

    /// The facts about one flex item that do not depend on the other items (9.2 steps 3 to 5, 4.5).
    fn flex_item(&mut self, cid: BoxId, cx: &Ctx, est: Option<LU>) -> Item {
        let tree = self.tree;
        let cbx = tree.get(cid);
        let st = cbx.style.clone();
        let (bs, bd, fl) = (&st.boxs, &st.border, &st.flex);
        let (cw, ax) = (cx.cw, cx.ax);
        let pad = [pad_px(&bs.padding[0], cw), pad_px(&bs.padding[1], cw), pad_px(&bs.padding[2], cw), pad_px(&bs.padding[3], cw)];
        let bor = [border_px(bd.width[0]), border_px(bd.width[1]), border_px(bd.width[2]), border_px(bd.width[3])];
        let marg = [resolve_margin(&bs.margin[0], cw), resolve_margin(&bs.margin[1], cw), resolve_margin(&bs.margin[2], cw), resolve_margin(&bs.margin[3], cw)];
        let z = LU::ZERO;
        let mu = marg.map(|m| m.unwrap_or(z));
        let a = if fl.align_self.kind == AlignKind::Auto { cx.align_items } else { fl.align_self };
        let align = match a.kind {
            AlignKind::Normal | AlignKind::Auto => AlignKind::Stretch,
            // Baselines are only aligned along the block axis here; a column container's cross axis is the inline one.
            AlignKind::Baseline | AlignKind::LastBaseline if ax.col => AlignKind::FlexStart,
            k => k,
        };
        let mut it = Item {
            id: cid,
            pad,
            bor,
            marg,
            mu,
            grow: fl.grow,
            shrink: fl.shrink,
            base: z,
            hyp: z,
            min: z,
            max: None,
            target: z,
            frozen: false,
            extra: 0,
            collapsed: st.text.visibility == Visibility::Collapse,
            lay_main: z,
            align,
            safe: a.safe == Some(true),
            cross_auto: true,
            cmin: z,
            cmax: None,
            cross: z,
            baseline: None,
            last_baseline: None,
            ratio: None,
            stretch: None,
            def_main: cx.inner_main.is_some(),
            content_h: None,
            rel: (z, z),
            mpos: z,
            cpos: z,
        };
        let border_box = bs.box_sizing == BoxSizing::BorderBox;
        let (pm, pc) = (it.pb(ax.ms, ax.me), it.pb(ax.cs, ax.ce));
        let (mm, mc) = (it.m0(ax.ms) + it.m0(ax.me), it.m0(ax.cs) + it.m0(ax.ce));
        it.extra = r(pm + mm);
        // The width that keywords and fit-content refer to.
        let w_extra = if ax.col { pc } else { pm };
        let w_marg = if ax.col { mc } else { mm };
        let w_avail = (cw - w_marg - w_extra).max0();
        let (main_dim, min_dim, max_dim) = if ax.col { (&bs.height, &bs.min_height, &bs.max_height) } else { (&bs.width, &bs.min_width, &bs.max_width) };
        let (cross_dim, cmin_dim, cmax_dim) = if ax.col { (&bs.width, &bs.min_width, &bs.max_width) } else { (&bs.height, &bs.min_height, &bs.max_height) };
        let main_base = if ax.col { cx.inner_main } else { Some(cw) };
        let cross_base = if ax.col { Some(cw) } else { cx.inner_cross };

        // Cross-axis size specifications (column: the width the item is measured at).
        let spec_cross = self.flex_dim(cid, cross_dim, cross_base, w_avail, ax.col, border_box, pc);
        it.cmin = self.flex_dim(cid, cmin_dim, cross_base, w_avail, ax.col, border_box, pc).unwrap_or(z);
        it.cmax = self.flex_dim(cid, cmax_dim, cross_base, w_avail, ax.col, border_box, pc);
        it.cross_auto = spec_cross.is_none();
        let (cmin, cmax) = (it.cmin, it.cmax);
        let clamp_cross = |v: LU| cmax.map_or(v, |m| v.min(m)).max(cmin);
        let mut cross_w = z;
        if ax.col {
            let stretch_known = align == AlignKind::Stretch && !it.is_auto(ax.cs) && !it.is_auto(ax.ce) && !cx.wrap;
            cross_w = match spec_cross {
                Some(w) => clamp_cross(w),
                None if stretch_known => clamp_cross(w_avail),
                None => self.atomic_width(cid, cw),
            };
            it.cross = cross_w + pc;
        }
        let ratio = if cbx.kind == BoxKind::Replaced {
            let (rw, rh) = self.replaced_size(cid, cw, cx.inner_cross);
            (rw > z && rh > z).then_some((rw, rh))
        } else {
            None
        };
        it.ratio = ratio;

        // Main size: flex base size (9.2 step 3), min and max (4.5).
        let spec_main = self.flex_dim(cid, main_dim, main_base, w_avail, !ax.col, border_box, pm);
        let basis = match &fl.basis {
            FlexBasis::Auto => spec_main,
            FlexBasis::Content => None,
            FlexBasis::Size(d) => self.flex_dim(cid, d, main_base, w_avail, !ax.col, border_box, pm),
        };
        let min_spec = self.flex_dim(cid, min_dim, main_base, w_avail, !ax.col, border_box, pm);
        it.max = self.flex_dim(cid, max_dim, main_base, w_avail, !ax.col, border_box, pm);
        let scroll = !matches!(if ax.col { bs.overflow_y } else { bs.overflow_x }, Overflow::Visible | Overflow::Clip);
        // The content size of the item along the main axis: its max-content width, or its height at the cross size.
        let need_content = basis.is_none() || (min_spec.is_none() && !scroll);
        let mut content_max = z;
        let mut content_min = z;
        it.def_main |= basis.is_some() && !matches!(fl.basis, FlexBasis::Auto);
        if need_content {
            if ax.col {
                let h = match ratio {
                    Some((rw, rh)) if cbx.kind == BoxKind::Replaced => lu(r(rh) * r(cross_w) / r(rw).max(1)),
                    _ => (self.flex_measure(cid, cw, cross_w, est) - pm).max0(),
                };
                content_max = h;
                content_min = h;
                it.content_h = Some(h);
            } else {
                (content_min, content_max) = self.content_widths(cid);
            }
        }
        it.base = basis.unwrap_or(content_max);
        it.min = match min_spec {
            Some(m) => m,
            None if scroll => z,
            None => {
                let s = spec_main.map_or(content_min, |p| content_min.min(p));
                it.max.map_or(s, |m| s.min(m))
            }
        };
        it.hyp = it.max.map_or(it.base, |m| it.base.min(m)).max(it.min).max0();
        it.lay_main = it.hyp;
        if it.collapsed {
            // 4.4, simplified: a collapsed item takes no room along the main axis but keeps its cross size as the strut.
            it.base = z;
            it.hyp = z;
            it.min = z;
            it.max = Some(z);
            it.grow = 0.0;
            it.shrink = 0.0;
            it.extra = 0;
            for side in [ax.ms, ax.me] {
                if let Some(m) = it.marg.get_mut(side) {
                    *m = Some(z);
                }
            }
            it.mu = [z; 4];
        }
        it
    }

    /// 9.7 Resolving Flexible Lengths for one flex line; `inner` and `gaps` are in raw layout units.
    fn resolve_lengths(items: &mut [Item], inner: i64, gaps: i64) {
        let sum_hyp: i64 = items.iter().map(Item::outer_hyp).sum::<i64>() + gaps;
        let grow = sum_hyp < inner;
        let factor = |i: &Item| if grow { f64::from(i.grow) } else { f64::from(i.shrink) };
        for it in items.iter_mut() {
            it.target = it.base;
            it.frozen = false;
            // Freeze the inflexible items at their hypothetical size.
            if factor(it) == 0.0 || (grow && it.base > it.hyp) || (!grow && it.base < it.hyp) {
                it.frozen = true;
                it.target = it.hyp;
            }
        }
        let space = |items: &[Item]| inner - gaps - items.iter().map(|i| r(if i.frozen { i.target } else { i.base }) + i.extra).sum::<i64>();
        let init_free = space(items);
        for _ in 0..items.len() + 1 {
            if items.iter().all(|i| i.frozen) {
                break;
            }
            let mut free = space(items);
            let sum_factor: f64 = items.iter().filter(|i| !i.frozen).map(factor).sum();
            if sum_factor < 1.0 {
                let scaled = (init_free as f64 * sum_factor) as i64;
                if scaled.abs() < free.abs() {
                    free = scaled;
                }
            }
            if free != 0 {
                if grow {
                    for it in items.iter_mut().filter(|i| !i.frozen) {
                        let share = free as f64 * f64::from(it.grow) / sum_factor.max(f64::MIN_POSITIVE);
                        it.target = lu(r(it.base) + share.round() as i64);
                    }
                } else {
                    let sum_scaled: f64 = items.iter().filter(|i| !i.frozen).map(|i| f64::from(i.shrink) * r(i.base) as f64).sum();
                    if sum_scaled > 0.0 {
                        for it in items.iter_mut().filter(|i| !i.frozen) {
                            let share = (free.abs() as f64) * f64::from(it.shrink) * r(it.base) as f64 / sum_scaled;
                            it.target = lu(r(it.base) - share.round() as i64);
                        }
                    }
                }
            }
            // Fix min / max violations, then freeze (the sign of the total violation says which).
            let mut total = 0i64;
            let mut viol: Vec<i64> = Vec::with_capacity(items.len());
            for it in items.iter_mut() {
                if it.frozen {
                    viol.push(0);
                    continue;
                }
                let clamped = it.max.map_or(it.target, |m| it.target.min(m)).max(it.min).max0();
                let d = r(clamped) - r(it.target);
                it.target = clamped;
                total += d;
                viol.push(d);
            }
            for (it, d) in items.iter_mut().zip(&viol) {
                if !it.frozen && (total == 0 || (total > 0 && *d > 0) || (total < 0 && *d < 0)) {
                    it.frozen = true;
                }
            }
        }
        for it in items.iter_mut() {
            it.frozen = true;
            if !it.collapsed {
                it.lay_main = it.target;
            }
        }
    }

    /// Lay `it` out at content width `fw` (and content height `fh` when forced).
    fn lay_flex_item(&mut self, it: &mut Item, cx: &Ctx, fw: LU, fh: Option<LU>) {
        let cb_h = if cx.ax.col { cx.inner_main } else { cx.inner_cross };
        // A column item whose height is not definite for its content (9.8) is laid out by its content: `height` is
        // already part of its flex base size, and percentages inside must not resolve against it.
        let by_content = cx.ax.col && fh.is_none();
        let res = self.layout_block(it.id, cx.cw, cb_h, Flow { forced_w: Some(fw), forced_h: fh, ignore_h: by_content, ..Flow::default() });
        it.rel = res.rel;
    }

    fn flex_inner(&mut self, id: BoxId, g: &Geom, origin: (LU, LU)) -> LU {
        let tree = self.tree;
        let bx = tree.get(id);
        let st = bx.style.clone();
        let fl = &st.flex;
        let col = matches!(fl.direction, FlexDirection::Column | FlexDirection::ColumnReverse);
        let ax = Axes::new(col, matches!(fl.direction, FlexDirection::RowReverse | FlexDirection::ColumnReverse), st.text.direction == Direction::Rtl, fl.wrap == FlexWrap::WrapReverse);
        let wrap = fl.wrap != FlexWrap::Nowrap;
        let cw = g.content_w;
        let ch = g.spec_h;
        let (inner_main, inner_cross) = if col { (ch, Some(cw)) } else { (Some(cw), ch) };
        let (min_main, max_main) = if col { (g.min_h, g.max_h) } else { (LU::ZERO, None) };
        let (min_cross, max_cross) = if col { (LU::ZERO, None) } else { (g.min_h, g.max_h) };
        let gap = |o: &Option<Lp>, base: Option<LU>| -> LU {
            o.as_ref().map_or(LU::ZERO, |l| match base {
                Some(b) => LU::from_f32(l.resolve(b.to_f32())).max0(),
                None if lp_has_percent(l) => LU::ZERO,
                None => LU::from_f32(l.resolve(0.0)).max0(),
            })
        };
        let (row_gap, col_gap) = (gap(&fl.row_gap, ch), gap(&fl.column_gap, Some(cw)));
        let (gap_main, gap_cross) = if col { (row_gap, col_gap) } else { (col_gap, row_gap) };
        let cx = Ctx { ax, cw, inner_main, inner_cross, wrap, align_items: fl.align_items };

        // ---- 9.1 the items; out-of-flow children wait for the static position (4.1)
        let mut items: Vec<Item> = Vec::new();
        let mut oofs: Vec<BoxId> = Vec::new();
        // First-screen-first (L1 task 17). A column container (not reversed, one line, no definite height) puts its items
        // one below the other with no free space, so an item that starts below the stop line, and all that follow it, can be
        // left out: nothing above the line depends on them (the container's minimum height must be within reach). A row
        // container with one line keeps the stop line inside its items when they start at the top of the line or stretch (a
        // line cut short must not move them). Every item gets a stop line of its own (`item_stop`).
        let top = r(self.origin_y) + r(origin.1);
        let stop_on = self.no_stop == 0 && self.stop_at.is_some();
        let col_cut = stop_on && col && !wrap && !ax.rev && inner_main.is_none() && max_main.is_none() && self.stop_at.is_some_and(|s| r(s) - top - r(min_main) >= 0);
        let (mut run, mut hit) = (0i64, false);
        for &c in &bx.children {
            if tree.get(c).oof {
                oofs.push(c);
            } else if self.exhausted {
                break;
            } else {
                let est = col_cut.then(|| lu(top + run));
                if est.is_some_and(|e| self.below_stop(e)) {
                    hit = true;
                    break;
                }
                let it = self.flex_item(c, &cx, est);
                run += it.outer_hyp() + r(gap_main);
                items.push(it);
            }
        }
        self.charge(items.len() as u64 * 3);
        let cut = col_cut || (stop_on && !col && !wrap && items.iter().all(|i| matches!(i.align, AlignKind::Stretch | AlignKind::FlexStart | AlignKind::Start | AlignKind::SelfStart)));
        if !cut {
            self.no_stop += 1;
        }

        // ---- 9.3 flex lines
        let avail = inner_main.or(max_main).map_or(i64::MAX / 4, r);
        let mut lines: Vec<FlexLine> = Vec::new();
        let mut s = 0;
        while s < items.len() {
            let (mut e, mut sum) = (s, 0i64);
            while let Some(it) = items.get(e) {
                let add = it.outer_hyp() + if e > s { r(gap_main) } else { 0 };
                if wrap && e > s && sum + add > avail {
                    break;
                }
                sum += add;
                e += 1;
            }
            lines.push(FlexLine { s, e, sum, cross: LU::ZERO, pos: LU::ZERO });
            s = e;
        }
        // The main size of the container (9.2 step 5): definite, or what the items need, within min and max.
        let main_size = match inner_main {
            Some(m) => m,
            None => {
                let need = lu(lines.iter().map(|l| l.sum).max().unwrap_or(0));
                max_main.map_or(need, |m| need.min(m)).max(min_main)
            }
        };

        // ---- 9.7 flexible lengths, 9.5 main-axis alignment
        for ln in &lines {
            let Some(sl) = items.get_mut(ln.s..ln.e) else { continue };
            let gaps = r(gap_main) * (sl.len() as i64 - 1);
            Self::resolve_lengths(sl, r(main_size), gaps);
            self.justify_line(sl, &ax, fl.justify_content, r(main_size), r(gap_main), gaps);
        }

        // ---- 9.4 cross sizes. A row container lays its items out here (at their final width); a column container
        // knows their widths already and lays them out last.
        let known_cross = !wrap && inner_cross.is_some();
        let items_mark = self.mark();
        let spent_before = self.flex_spent;
        if !col {
            for it in items.iter_mut() {
                if self.exhausted {
                    break;
                }
                // A stretched item in a container with a definite cross size and a single line: its height is known.
                let stretched = known_cross && it.align == AlignKind::Stretch && it.cross_auto && !it.is_auto(ax.cs) && !it.is_auto(ax.ce);
                let pc = it.pb(ax.cs, ax.ce);
                let fh = match (stretched, inner_cross) {
                    (true, Some(c)) => {
                        let v = (c - it.mu(ax.cs) - it.mu(ax.ce) - pc).max0();
                        Some(it.cmax.map_or(v, |m| v.min(m)).max(it.cmin))
                    }
                    _ => it.ratio.filter(|_| it.cross_auto).map(|(rw, rh)| lu(r(rh) * r(it.lay_main) / r(rw).max(1))),
                };
                let fw = it.lay_main;
                if cut {
                    hit |= self.item_stop(lu(top), |s| s.lay_flex_item(it, &cx, fw, fh)).1;
                } else {
                    self.lay_flex_item(it, &cx, fw, fh);
                }
                let o = self.out.get(it.id as usize).copied().unwrap_or_default();
                it.cross = o.h;
                it.baseline = self.first_baseline(it.id);
                it.last_baseline = self.last_baseline(it.id);
            }
        }
        self.charge(items.len() as u64 * 2);

        // Line cross sizes (9.4): the tallest item, or with baseline alignment the most above + the most below.
        for ln in lines.iter_mut() {
            let Some(sl) = items.get(ln.s..ln.e) else { continue };
            ln.cross = Self::line_cross(sl, &ax);
        }
        if !wrap && let Some(l) = lines.first_mut() {
            l.cross = inner_cross.unwrap_or_else(|| max_cross.map_or(l.cross, |m| l.cross.min(m)).max(min_cross));
        }
        let total_cross = lines.iter().map(|l| r(l.cross)).sum::<i64>() + r(gap_cross) * (lines.len() as i64 - 1).max(0);
        let cross_final = match inner_cross {
            Some(c) => c,
            None => {
                let t = lu(total_cross);
                max_cross.map_or(t, |m| t.min(m)).max(min_cross)
            }
        };
        // ---- 9.6 align-content (also for a single line of a wrapping container)
        let (lead_cross, between_cross) = if wrap { Self::align_lines(&mut lines, &ax, fl.align_content, r(cross_final), total_cross) } else { (0, 0) };
        let mut at = lead_cross;
        for ln in lines.iter_mut() {
            ln.pos = lu(at);
            at += r(ln.cross) + r(gap_cross) + between_cross;
        }

        // ---- 9.6 cross-axis alignment of each item, then the final layout
        let mut redo = false;
        for ln in &lines {
            let Some(sl) = items.get_mut(ln.s..ln.e) else { continue };
            self.align_line(sl, ln, &ax);
            if !col {
                for it in sl.iter() {
                    if let Some(sh) = it.stretch {
                        let want = sh + it.pb(ax.cs, ax.ce);
                        if want != it.cross && self.row_needs_layout(it) {
                            redo = true;
                        }
                    }
                }
            }
        }
        let repeat_ok = self.flex_spent.saturating_sub(spent_before) + self.flex_spent < self.budget / FLEX_REPEAT_SHARE;
        if !col && redo && repeat_ok {
            // Some stretched item depends on its height: lay all items out again from the same start.
            let w0 = self.work;
            self.restore(items_mark);
            for it in items.iter_mut() {
                if self.exhausted {
                    break;
                }
                let fh = it.stretch.filter(|_| it.cross_auto).or_else(|| it.ratio.filter(|_| it.cross_auto).map(|(rw, rh)| lu(r(rh) * r(it.lay_main) / r(rw).max(1))));
                let fw = it.lay_main;
                if cut {
                    hit |= self.item_stop(lu(top), |s| s.lay_flex_item(it, &cx, fw, fh)).1;
                } else {
                    self.lay_flex_item(it, &cx, fw, fh);
                }
            }
            self.flex_spent += self.work.saturating_sub(w0);
        } else if !col {
            // Stretched items whose content does not care get a taller box.
            for it in items.iter() {
                if let Some(sh) = it.stretch {
                    let want = sh + it.pb(ax.cs, ax.ce);
                    if want != it.cross && let Some(o) = self.out.get_mut(it.id as usize) {
                        o.h = want;
                    }
                }
            }
        } else {
            for it in items.iter_mut() {
                if self.exhausted {
                    break;
                }
                let pc = it.pb(ax.cs, ax.ce);
                let fw = it.stretch.unwrap_or((it.cross - pc).max0());
                let fh = if it.def_main || it.content_h != Some(it.target) { Some(it.target) } else { None };
                if cut {
                    hit |= self.item_stop(lu(top + r(it.mpos)), |s| s.lay_flex_item(it, &cx, fw, fh)).1;
                } else {
                    self.lay_flex_item(it, &cx, fw, fh);
                }
            }
        }

        // ---- positions
        let (cl, ct) = origin;
        for it in items.iter() {
            let bm = r(it.target) + r(it.pb(ax.ms, ax.me));
            let bc = r(it.stretch.unwrap_or(it.cross - it.pb(ax.cs, ax.ce))) + r(it.pb(ax.cs, ax.ce));
            let m = if ax.rev { r(main_size) - (r(it.mpos) + bm) } else { r(it.mpos) };
            let c = if ax.crev { r(cross_final) - (r(it.cpos) + bc) } else { r(it.cpos) };
            let (x, y) = if col { (c, m) } else { (m, c) };
            if let Some(o) = self.out.get_mut(it.id as usize) {
                o.x = cl + lu(x) + it.rel.0;
                o.y = ct + lu(y) + it.rel.1;
            }
        }

        // ---- 8.5 the baselines of the container: those of the items aligned by baseline on the first / last line, else
        // of the first / last item
        if let (Some(l0), Some(ln)) = (lines.first(), lines.last()) {
            let first = items.get(l0.s..l0.e).and_then(|sl| sl.iter().find(|i| i.align == AlignKind::Baseline).or_else(|| sl.first())).map(|i| i.id);
            let last = items.get(ln.s..ln.e).and_then(|sl| sl.iter().rev().find(|i| i.align == AlignKind::LastBaseline).or_else(|| sl.last())).map(|i| i.id);
            let fb = first.and_then(|c| Some(self.first_baseline(c)? + self.out.get(c as usize)?.y));
            let lb = last.and_then(|c| Some(self.last_baseline(c)? + self.out.get(c as usize)?.y));
            self.flex_base.insert(id, (fb.unwrap_or(LU::MIN), lb.unwrap_or(LU::MIN)));
        }

        // ---- 4.1 out-of-flow children
        let (inner_w, inner_h) = if col { (cw, main_size) } else { (cw, cross_final) };
        for c in oofs {
            let cbx = &tree.get(c).style;
            let a = if cbx.flex.align_self.kind == AlignKind::Auto { fl.align_items } else { cbx.flex.align_self };
            let flip = |p: Pos, f: bool| match (p, f) {
                (Pos::Start, true) => 2u8,
                (Pos::End, true) => 0,
                (Pos::Start, false) => 0,
                (Pos::End, false) => 2,
                (Pos::Mid, _) => 1,
            };
            let pm = flip(ax.pos_main(fl.justify_content.kind), ax.rev);
            let pc = flip(ax.pos_cross(a.kind), ax.crev);
            let (ax_x, ax_y) = if col { (pc, pm) } else { (pm, pc) };
            let fixed = cbx.boxs.position == Position::Fixed;
            let cb = if fixed { None } else { self.cb_stack.last().copied() };
            self.pending.push(super::Pending { id: c, owner: id, sx: cl, sy: ct, cb, align: Some(StaticAlign { rw: inner_w, rh: inner_h, ax: ax_x, ay: ax_y }) });
        }

        if !cut {
            self.no_stop -= 1;
        } else if hit {
            self.cut.push(id);
            self.stopped = true;
        }
        if col { main_size } else { lu(total_cross).max(LU::ZERO) }
    }

    /// 9.5: auto margins, then `justify-content`. Sets `mpos` and the used main margins.
    fn justify_line(&mut self, sl: &mut [Item], ax: &Axes, jc: AlignValue, main: i64, gap: i64, gaps: i64) {
        let cnt = sl.len() as i64;
        let used: i64 = sl.iter().map(|i| r(i.target) + i.extra).sum();
        let free = main - used - gaps;
        let autos: i64 = sl.iter().map(|i| i64::from(i.is_auto(ax.ms)) + i64::from(i.is_auto(ax.me))).sum();
        let (mut lead, mut between) = (0i64, 0i64);
        if free > 0 && autos > 0 {
            let each = free / autos;
            for it in sl.iter_mut() {
                for side in [ax.ms, ax.me] {
                    if it.is_auto(side) && let Some(m) = it.mu.get_mut(side) {
                        *m = lu(each);
                    }
                }
            }
        } else {
            let safe = jc.safe == Some(true);
            match jc.kind {
                AlignKind::SpaceBetween if free > 0 && cnt > 1 => between = div_round(free, cnt - 1),
                AlignKind::SpaceAround if free > 0 => {
                    between = div_round(free, cnt);
                    lead = div_round(free, 2 * cnt);
                }
                AlignKind::SpaceEvenly if free > 0 => {
                    between = div_round(free, cnt + 1);
                    lead = between;
                }
                // With no room left `space-between` is `flex-start` and `space-around` and `space-evenly` are `safe center`
                // (css-align-3 8.3; the browsers, and the WPT tests, agree), which is `flex-start` as well.
                AlignKind::SpaceAround | AlignKind::SpaceEvenly | AlignKind::SpaceBetween => {}
                k => {
                    lead = match ax.pos_main(k) {
                        Pos::Start => 0,
                        Pos::Mid => free / 2,
                        Pos::End => free,
                    };
                    if safe && free < 0 {
                        lead = 0;
                    }
                }
            }
        }
        let mut pos = lead;
        for it in sl.iter_mut() {
            pos += r(it.mu(ax.ms));
            it.mpos = lu(pos);
            pos += r(it.target) + r(it.pb(ax.ms, ax.me)) + r(it.mu(ax.me)) + gap + between;
        }
    }

    /// 9.4: the cross size of a line.
    fn line_cross(sl: &[Item], ax: &Axes) -> LU {
        let (mut af, mut bf, mut al, mut bl, mut other) = (0i64, 0i64, 0i64, 0i64, 0i64);
        for it in sl {
            let outer = r(it.cross) + r(it.m0(ax.cs)) + r(it.m0(ax.ce));
            let baseline_item = !ax.col && !it.is_auto(ax.cs) && !it.is_auto(ax.ce) && matches!(it.align, AlignKind::Baseline | AlignKind::LastBaseline);
            if baseline_item {
                let first = it.align == AlignKind::Baseline;
                let b = if first { it.baseline } else { it.last_baseline }.map_or(r(it.cross), r);
                let above = r(it.m0(ax.cs)) + b;
                let below = outer - above;
                if first {
                    af = af.max(above);
                    bf = bf.max(below);
                } else {
                    al = al.max(above);
                    bl = bl.max(below);
                }
            } else {
                other = other.max(outer);
            }
        }
        lu((af + bf).max(al + bl).max(other).max(0))
    }

    /// 9.6 `align-content`: the lines' share of the free space. `normal` and `stretch` grow the lines' cross size; the
    /// result is the offset of the first line and the extra space between lines.
    fn align_lines(lines: &mut [FlexLine], ax: &Axes, ac: AlignValue, cross: i64, total: i64) -> (i64, i64) {
        let n = lines.len() as i64;
        let free = cross - total;
        match ac.kind {
            AlignKind::Normal | AlignKind::Stretch => {
                if free > 0 && n > 0 {
                    let each = free / n;
                    let mut extra = free - each * n;
                    for l in lines.iter_mut() {
                        let add = each + i64::from(extra > 0);
                        extra -= 1;
                        l.cross = lu(r(l.cross) + add);
                    }
                }
                (0, 0)
            }
            AlignKind::SpaceBetween if free > 0 && n > 1 => (0, div_round(free, n - 1)),
            AlignKind::SpaceAround if free > 0 => (div_round(free, 2 * n), div_round(free, n)),
            AlignKind::SpaceEvenly if free > 0 => (div_round(free, n + 1), div_round(free, n + 1)),
            AlignKind::SpaceAround | AlignKind::SpaceEvenly | AlignKind::SpaceBetween => (0, 0),
            k => {
                let off = match ax.pos_cross(k) {
                    Pos::Start => 0,
                    Pos::Mid => free / 2,
                    Pos::End => free,
                };
                (if ac.safe == Some(true) && free < 0 { 0 } else { off }, 0)
            }
        }
    }

    /// 9.6 (cross-axis auto margins, `align-self`) and 9.4 (stretch) for the items of one line: sets `cpos` and `stretch`.
    fn align_line(&mut self, sl: &mut [Item], ln: &FlexLine, ax: &Axes) {
        let (af, bf, bl) = {
            let (mut af, mut bf, mut bl) = (0i64, 0i64, 0i64);
            for it in sl.iter() {
                if !ax.col && !it.is_auto(ax.cs) && !it.is_auto(ax.ce) {
                    let outer = r(it.cross) + r(it.m0(ax.cs)) + r(it.m0(ax.ce));
                    match it.align {
                        AlignKind::Baseline => af = af.max(r(it.m0(ax.cs)) + it.baseline.map_or(r(it.cross), r)),
                        AlignKind::LastBaseline => {
                            let above = r(it.m0(ax.cs)) + it.last_baseline.map_or(r(it.cross), r);
                            bl = bl.max(outer - above);
                        }
                        _ => {}
                    }
                    if it.align == AlignKind::Baseline {
                        let above = r(it.m0(ax.cs)) + it.baseline.map_or(r(it.cross), r);
                        bf = bf.max(outer - above);
                    }
                }
            }
            (af, bf, bl)
        };
        let _ = bf;
        let line = r(ln.cross);
        for it in sl.iter_mut() {
            let pc = it.pb(ax.cs, ax.ce);
            let (auto_s, auto_e) = (it.is_auto(ax.cs), it.is_auto(ax.ce));
            // 9.4: a stretched item (auto cross size, no auto cross margin) fills its line within its min and max.
            if it.align == AlignKind::Stretch && it.cross_auto && !auto_s && !auto_e {
                let v = (lu(line) - it.m0(ax.cs) - it.m0(ax.ce) - pc).max0();
                let v = it.cmax.map_or(v, |m| v.min(m)).max(it.cmin);
                it.stretch = Some(v);
            }
            let size = r(it.stretch.map_or(it.cross, |s| s + pc));
            let (ms, me) = (r(it.m0(ax.cs)), r(it.m0(ax.ce)));
            let outer = size + ms + me;
            let mut off;
            if auto_s || auto_e {
                let free = line - outer;
                let mut a = ms;
                if auto_s {
                    a = if free <= 0 { 0 } else if auto_e { free / 2 } else { free };
                }
                if let Some(m) = it.mu.get_mut(ax.cs) {
                    *m = lu(a);
                }
                off = a;
            } else {
                let free = line - outer;
                off = match it.align {
                    AlignKind::Baseline => af - (ms + it.baseline.map_or(r(it.cross), r)) + ms,
                    AlignKind::LastBaseline => (line - bl) - (ms + it.last_baseline.map_or(r(it.cross), r)) + ms,
                    k => {
                        let p = match ax.pos_cross(k) {
                            Pos::Start => 0,
                            Pos::Mid => free / 2,
                            Pos::End => free,
                        };
                        let p = if it.safe && free < 0 { 0 } else { p };
                        p + ms
                    }
                };
            }
            if it.align == AlignKind::Stretch && !(auto_s || auto_e) {
                off = ms;
            }
            it.cpos = lu(r(ln.pos) + off);
        }
    }

    /// Does laying out `it` again with a taller height change anything but the height of its box?
    fn row_needs_layout(&mut self, it: &Item) -> bool {
        self.height_sensitive(it.id)
    }

    /// Min-content and max-content width of the content box of flex container `id` (9.9). Auto margins count as 0.
    pub(crate) fn flex_widths(&mut self, id: BoxId) -> (LU, LU) {
        let tree = self.tree;
        let bx = tree.get(id);
        let fl = &bx.style.flex;
        let col = matches!(fl.direction, FlexDirection::Column | FlexDirection::ColumnReverse);
        let wrap = fl.wrap != FlexWrap::Nowrap;
        let gap = fl.column_gap.as_ref().map_or(LU::ZERO, |l| if lp_has_percent(l) { LU::ZERO } else { LU::from_f32(l.resolve(0.0)).max0() });
        let (mut min_sum, mut max_sum, mut min_max, mut max_max) = (LU::ZERO, LU::ZERO, LU::ZERO, LU::ZERO);
        let mut n = 0i64;
        for &c in &bx.children {
            let cb = tree.get(c);
            if cb.oof || cb.style.text.visibility == Visibility::Collapse {
                continue;
            }
            if self.exhausted {
                break;
            }
            let (mn, mx) = self.flex_contribution(c, col);
            min_sum += mn;
            max_sum += mx;
            min_max = min_max.max(mn);
            max_max = max_max.max(mx);
            n += 1;
        }
        if col {
            (min_max, max_max)
        } else {
            let gaps = lu(r(gap) * (n - 1).max(0));
            (if wrap { min_max } else { min_sum + gaps }, max_sum + gaps)
        }
    }

    /// The min-content and max-content contribution of flex item `c` along the width (9.9.3, 9.9.1). An item that cannot
    /// grow counts its flex base size (a fixed `flex-basis`) clamped by its min and max width, as the browsers do; one that
    /// can counts its max-content size.
    fn flex_contribution(&mut self, c: BoxId, col: bool) -> (LU, LU) {
        let tree = self.tree;
        let cb = tree.get(c);
        let fl = &cb.style.flex;
        let (mut mn, mut mx) = self.outer_widths(c);
        if !col && fl.grow == 0.0 && let FlexBasis::Size(Dim::Lp(l)) = &fl.basis && !lp_has_percent(l) {
            let bs = &cb.style.boxs;
            let bd = &cb.style.border;
            let p = |i: usize| bs.padding.get(i).map_or(LU::ZERO, |l| if lp_has_percent(l) { LU::ZERO } else { pad_px(l, LU::ZERO) });
            let b = |i: usize| border_px(bd.width.get(i).copied().unwrap_or(0.0));
            let ext = p(1) + p(3) + b(1) + b(3);
            let m = |d: &Dim| match d {
                Dim::Lp(l) if !lp_has_percent(l) => LU::from_f32(l.resolve(0.0)),
                _ => LU::ZERO,
            };
            let margins = m(&bs.margin[1]) + m(&bs.margin[3]);
            let to_c = |v: LU| if bs.box_sizing == BoxSizing::BorderBox { (v - ext).max0() } else { v };
            let mut inner = to_c(LU::from_f32(l.resolve(0.0)).max0());
            if let Dim::Lp(ml) = &bs.max_width && !lp_has_percent(ml) {
                inner = inner.min(to_c(LU::from_f32(ml.resolve(0.0))));
            }
            match &bs.min_width {
                Dim::Lp(ml) if !lp_has_percent(ml) => inner = inner.max(to_c(LU::from_f32(ml.resolve(0.0)))),
                Dim::Auto if matches!(bs.overflow_x, Overflow::Visible | Overflow::Clip) => inner = inner.max(self.content_widths(c).0),
                _ => {}
            }
            let outer = inner + ext + margins;
            mx = outer;
            mn = if fl.shrink > 0.0 { mn.min(outer) } else { outer };
        }
        (mn, mx.max(mn))
    }
}
