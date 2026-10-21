//! Tables (CSS2 chapter 17; the column widths follow css-tables-3 and what browsers do).
//!
//! The box tree already has the anonymous boxes of CSS2 17.2.1 and lists a table's children as: columns and column
//! groups, captions, row groups (the first header group first, the first footer group last). The grid (`Grid`: HTML
//! 4.9.12 "forming a table" for the cell positions, the column minimum / maximum / specified widths, the collapsed
//! borders) is built once per table and memoised; both the intrinsic widths (nested tables) and the layout use it.
//!
//! A table box is one box: its `BoxOut` is the whole table wrapper (captions included); the table's own background and
//! border cover the grid box only (`TableInfo`). Row groups and rows are positioned relative to their parent like any
//! block; a cell is a block formatting context whose content sits in one anonymous block (made by the box tree
//! builder), which is moved down for `vertical-align: middle | bottom | baseline`. Collapsed borders (CSS2 17.6.2) are
//! resolved per grid edge; the cells and the table get their halves as border widths, and the painter draws the edges
//! (`TableInfo::segs`) instead of the boxes' own borders.

use std::rc::Rc;

use super::Layouter;
use super::block::{BlockResult, Flow, border_px, pad_px, resolve_dim, resolve_margin, solve_width};
use super::boxtree::{BoxId, Role, parse_html_int, role_of};
use super::units::{LayoutUnit as LU, Margin};
use crate::css::color::Color;
use crate::css::computed::ComputedStyle;
use crate::css::types::{BorderCollapse, BorderLineStyle, BoxSizing, CaptionSide, Dim, Direction, EmptyCells, Overflow, Position, TableLayout, VerticalAlign, Visibility};
use crate::css::values::Lp;
use crate::html::atoms::{Atom, a};
use crate::html::dom::NodeId;

/// HTML 4.9.11: `colspan` is at most 1000 and a table has at most that many columns; `rowspan` is at most 65534.
const MAX_COLS: usize = 1000;
const MAX_ROWS: usize = 65_534;
/// Collapsed borders are resolved for tables of at most this many grid slots (two edge arrays are built).
const MAX_EDGE_SLOTS: usize = 262_144;

/// A piece of collapsed border to paint: x, y, width, height (relative to the table's border box) and colour.
pub type Seg = (LU, LU, LU, LU, Color);

/// What the painter needs besides the box rectangles: the grid box of a table with captions (the table's background
/// and border cover it, not the captions) and the collapsed borders.
#[derive(Debug, Default)]
pub struct TableInfo {
    pub y: LU,
    pub h: LU,
    pub segs: Vec<Seg>,
}

struct Cell {
    id: BoxId,
    r: usize,
    c: usize,
    rs: usize,
    cs: usize,
    /// Used border widths (top, right, bottom, left): its own, or its halves of the collapsed ones.
    bor: [LU; 4],
    /// Border-box minimum and maximum content widths.
    min: LU,
    max: LU,
    /// Specified `width` (border box) or percentage (0: none) and `height` (border box).
    wfix: Option<LU>,
    wpct: f32,
    /// What a percentage width gets added for padding and border (`box-sizing: content-box`).
    pctx: LU,
    hfix: LU,
}

#[derive(Clone, Copy, Default)]
struct Col {
    min: LU,
    max: LU,
    /// Specified width of a cell in the column, and of a `col` element.
    fix: LU,
    cfix: LU,
    pct: f32,
    pctx: LU,
    /// Percentage `width` of a `col` element.
    cpct: f32,
}

#[derive(Clone, Copy)]
struct Edge {
    w: u16,
    st: u8,
    src: u8,
    hidden: bool,
    color: Color,
}

struct Edges {
    h: Vec<Edge>,
    v: Vec<Edge>,
}

pub(crate) struct Grid {
    cells: Vec<Cell>,
    rows: Vec<BoxId>,
    /// `height` of every row (the least it gets).
    row_h: Vec<LU>,
    cols: Vec<Col>,
    /// `col` and `colgroup` boxes: (first column, columns, box, is a group).
    specs: Vec<(usize, usize, BoxId, bool)>,
    collapse: bool,
    sp: [LU; 2],
    /// The table's border widths when collapsed.
    tbor: [LU; 4],
    edges: Option<Edges>,
    /// Min-content and max-content width of the grid (columns and spacing), inside the table's border and padding.
    pub(crate) min_total: LU,
    pub(crate) max_total: LU,
}

fn px(l: LU) -> u16 {
    (l.raw() / 64).clamp(0, i32::from(u16::MAX)) as u16
}

fn lu(p: u16) -> LU {
    LU::px(i32::from(p))
}

/// The two halves of a collapsed border of width `w`: the box before the edge gets the smaller one (CSS2 17.6.2).
fn lo(w: u16) -> u16 {
    w / 2
}

fn hi(w: u16) -> u16 {
    w - w / 2
}

fn mul(l: LU, n: usize) -> LU {
    LU::from_raw(i64::from(l.raw()) * n as i64)
}

fn at(v: &[LU], i: usize) -> LU {
    v.get(i).copied().unwrap_or(LU::ZERO)
}

fn bump(v: &mut [LU], i: usize, x: LU) {
    if let Some(e) = v.get_mut(i) {
        *e = (*e).max(x);
    }
}

fn sum(v: &[LU]) -> LU {
    v.iter().fold(LU::ZERO, |s, x| s + *x)
}

impl Edge {
    const NONE: Edge = Edge { w: 0, st: 0, src: 0, hidden: false, color: Color::TRANSPARENT };

    /// The border `side` (top, right, bottom, left) of a box as a candidate for an edge (CSS2 17.6.2.1).
    fn of(s: &ComputedStyle, side: usize, src: u8) -> Edge {
        let style = s.border.style.get(side).copied().unwrap_or(BorderLineStyle::None);
        let w = px(border_px(s.border.width.get(side).copied().unwrap_or(0.0)));
        let st = match style {
            BorderLineStyle::Double => 8,
            BorderLineStyle::Solid => 7,
            BorderLineStyle::Dashed => 6,
            BorderLineStyle::Dotted => 5,
            BorderLineStyle::Ridge => 4,
            BorderLineStyle::Outset => 3,
            BorderLineStyle::Groove => 2,
            BorderLineStyle::Inset => 1,
            _ => 0,
        };
        let color = s.border.color.get(side).map_or(Color::TRANSPARENT, |c| c.resolve(s.text.color));
        Edge { w, st, src, hidden: style == BorderLineStyle::Hidden, color }
    }

    /// CSS2 17.6.2.1: `hidden` wins over everything; else the wider border, then the "more solid" style, then the
    /// box that is nearer to the cell (cell, row, row group, table); on a complete tie the one that came first
    /// (the cell above or to the left) stays.
    fn beats(&self, old: &Edge) -> bool {
        !old.hidden && (self.hidden || (self.w, self.st, self.src) > (old.w, old.st, old.src))
    }
}

fn put(e: &mut [Edge], i: usize, n: Edge) {
    if let Some(o) = e.get_mut(i)
        && n.beats(o)
    {
        *o = n;
    }
}

fn widest(e: &[Edge], it: impl Iterator<Item = usize>) -> u16 {
    it.filter_map(|i| e.get(i)).filter(|e| !e.hidden).map(|e| e.w).max().unwrap_or(0)
}

/// Add `extra` (raw units) to the entries `idx` of `w`, in proportion to `wt` (equally if all are 0); the last one
/// takes the rounding.
fn give(w: &mut [LU], idx: &[usize], wt: &[i64], extra: i64) {
    let total: i64 = wt.iter().sum();
    let n = idx.len() as i64;
    let mut left = extra;
    for (k, &i) in idx.iter().enumerate() {
        let share = if k + 1 == idx.len() {
            left
        } else if total > 0 {
            extra * wt.get(k).copied().unwrap_or(0) / total
        } else {
            extra / n
        };
        left -= share;
        if let Some(v) = w.get_mut(i) {
            *v = LU::from_raw(i64::from(v.raw()) + share);
        }
    }
}

/// Automatic table layout (css-tables-3 "distributing excess width", what browsers do): every column starts at its
/// minimum; the room above that goes first to the columns with a percentage (up to it), then to those with a
/// specified width (up to it), then to the others (up to their maximum), and what is left to the auto columns in
/// proportion to their widths.
fn auto_widths(cols: &[Col], avail: LU) -> Vec<LU> {
    let mut w: Vec<LU> = cols.iter().map(|c| c.min).collect();
    let mut rem = i64::from(avail.raw()) - i64::from(sum(&w).raw());
    if rem <= 0 {
        return w;
    }
    let pct_total: f32 = cols.iter().map(|c| c.pct).sum();
    let scale = if pct_total > 100.0 { 100.0 / pct_total } else { 1.0 };
    // (phase, target width) of every column.
    let want: Vec<(u8, LU)> = cols
        .iter()
        .map(|c| {
            if c.pct > 0.0 {
                (0, c.min.max(LU::from_f32(avail.to_f32() * c.pct * scale / 100.0) + c.pctx))
            } else if c.fix > LU::ZERO {
                (1, c.min.max(c.fix))
            } else {
                (2, c.max.max(c.min))
            }
        })
        .collect();
    for phase in 0..3u8 {
        let idx: Vec<usize> = want.iter().enumerate().filter(|(_, t)| t.0 == phase).map(|(i, _)| i).collect();
        let def: Vec<i64> = idx.iter().map(|&i| (i64::from(want.get(i).map_or(0, |t| t.1.raw())) - i64::from(at(&w, i).raw())).max(0)).collect();
        let give_now = def.iter().sum::<i64>().min(rem);
        give(&mut w, &idx, &def, give_now);
        rem -= give_now;
        if rem == 0 {
            return w;
        }
    }
    // Whatever is left: to the auto columns, else the fixed ones, else all.
    let pick = (0..3u8).rev().find(|p| want.iter().any(|t| t.0 == *p)).unwrap_or(2);
    let idx: Vec<usize> = want.iter().enumerate().filter(|(_, t)| t.0 == pick).map(|(i, _)| i).collect();
    let wt: Vec<i64> = idx.iter().map(|&i| i64::from(at(&w, i).raw())).collect();
    give(&mut w, &idx, &wt, rem);
    w
}

/// Fixed table layout (CSS2 17.5.2.1): the widths come from the `col` elements and the cells of the first row; the
/// columns without one share what is left equally.
fn fixed_widths(g: &Grid, avail: LU) -> Vec<LU> {
    let mut w: Vec<Option<LU>> = g
        .cols
        .iter()
        .map(|c| (c.cfix > LU::ZERO).then_some(c.cfix).or_else(|| (c.cpct > 0.0).then(|| LU::from_f32(avail.to_f32() * c.cpct / 100.0))))
        .collect();
    for cell in g.cells.iter().take_while(|c| c.r == 0) {
        let v = cell.wfix.or_else(|| (cell.wpct > 0.0).then(|| LU::from_f32(avail.to_f32() * cell.wpct / 100.0) + cell.pctx));
        if let Some(v) = v {
            let per = LU::from_raw(i64::from(v.raw()) / cell.cs.max(1) as i64);
            for slot in w.iter_mut().skip(cell.c).take(cell.cs) {
                slot.get_or_insert(per);
            }
        }
    }
    let used = w.iter().flatten().fold(LU::ZERO, |s, x| s + *x);
    let autos = w.iter().filter(|x| x.is_none()).count();
    let rem = (avail - used).max0();
    let share = if autos > 0 { LU::from_raw(i64::from(rem.raw()) / autos as i64) } else { LU::ZERO };
    let mut out: Vec<LU> = w.iter().map(|x| x.unwrap_or(share)).collect();
    if autos == 0 && rem > LU::ZERO {
        let idx: Vec<usize> = (0..out.len()).collect();
        let wt: Vec<i64> = out.iter().map(|x| i64::from(x.raw())).collect();
        give(&mut out, &idx, &wt, i64::from(rem.raw()));
    }
    out
}

fn empty_result() -> BlockResult {
    let z = Margin::default();
    BlockResult { h: LU::ZERO, ml: LU::ZERO, mt: z, mb: z, top_only: z, bot_only: z, through: false, rel: (LU::ZERO, LU::ZERO) }
}

impl Layouter<'_> {
    /// The grid of table `tid`, built on first use.
    pub(crate) fn grid(&mut self, tid: BoxId) -> Rc<Grid> {
        if let Some(g) = self.grids.get(&tid) {
            return g.clone();
        }
        let g = Rc::new(self.build_grid(tid));
        self.grids.insert(tid, g.clone());
        g
    }

    /// Min-content and max-content width of the grid of table `tid`.
    pub(crate) fn table_widths(&mut self, tid: BoxId) -> (LU, LU) {
        let g = self.grid(tid);
        (g.min_total, g.max_total)
    }

    /// Place the cells (HTML 4.9.12: a cell takes the first free slot of its row; `rowspan` ends with the row group),
    /// resolve the borders, and work out the widths of the columns.
    fn build_grid(&mut self, tid: BoxId) -> Grid {
        let tree = self.tree;
        let doc = self.doc;
        let tb = tree.get(tid);
        let ts = &tb.style;
        let (a_span, a_cs, a_rs) = (doc.atoms.lookup("span"), doc.atoms.lookup("colspan"), doc.atoms.lookup("rowspan"));
        let num = |n: NodeId, name: Option<Atom>| name.and_then(|x| doc.attr(n, x)).and_then(parse_html_int);
        let span_of = |b: BoxId| num(tree.get(b).node, a_span).filter(|v| *v >= 1).map_or(1, |v| (v as usize).min(MAX_COLS));
        let mut g = Grid {
            cells: Vec::new(),
            rows: Vec::new(),
            row_h: Vec::new(),
            cols: Vec::new(),
            specs: Vec::new(),
            collapse: false,
            sp: [LU::ZERO; 2],
            tbor: [LU::ZERO; 4],
            edges: None,
            min_total: LU::ZERO,
            max_total: LU::ZERO,
        };
        let mut busy: Vec<usize> = Vec::new();
        let (mut ncols, mut col_total) = (0usize, 0usize);
        for &gid in &tb.children {
            let gb = tree.get(gid);
            match role_of(gb) {
                role @ (Role::Col | Role::ColGroup) => {
                    let group = role == Role::ColGroup;
                    if group && !gb.children.is_empty() {
                        let first = col_total;
                        for &c in &gb.children {
                            let s = span_of(c);
                            g.specs.push((col_total, s, c, false));
                            col_total = (col_total + s).min(MAX_COLS);
                        }
                        g.specs.push((first, col_total - first, gid, true));
                    } else {
                        let s = span_of(gid);
                        g.specs.push((col_total, s, gid, group));
                        col_total = (col_total + s).min(MAX_COLS);
                    }
                }
                Role::Group => {
                    busy.fill(0);
                    let k = gb.children.len();
                    for (lr, &rid) in gb.children.iter().enumerate() {
                        if g.rows.len() >= MAX_ROWS {
                            break;
                        }
                        let r = g.rows.len();
                        let rb = tree.get(rid);
                        g.rows.push(rid);
                        g.row_h.push(resolve_dim(&rb.style.boxs.height, None).unwrap_or(LU::ZERO));
                        let mut x = 0usize;
                        for &cid in &rb.children {
                            let n = tree.get(cid).node;
                            let html = doc.is_html_element(n, a::TD) || doc.is_html_element(n, a::TH);
                            let cs = if html { num(n, a_cs).filter(|v| *v >= 1).map_or(1, |v| (v as usize).min(MAX_COLS)) } else { 1 };
                            let left = k - lr;
                            let rs = match num(n, a_rs) {
                                Some(0) if html => left,
                                Some(v) if v > 0 && html => (v as usize).min(left),
                                _ => 1,
                            };
                            while busy.get(x).is_some_and(|&b| b > r) {
                                x += 1;
                            }
                            if x >= MAX_COLS {
                                break;
                            }
                            let cs = cs.min(MAX_COLS - x);
                            if busy.len() < x + cs {
                                busy.resize(x + cs, 0);
                            }
                            for b in busy.iter_mut().skip(x).take(cs) {
                                *b = r + rs;
                            }
                            let z = LU::ZERO;
                            g.cells.push(Cell { id: cid, r, c: x, rs, cs, bor: [z; 4], min: z, max: z, wfix: None, wpct: 0.0, pctx: z, hfix: z });
                            ncols = ncols.max(x + cs);
                            x += cs;
                        }
                    }
                }
                _ => {}
            }
        }
        let ncols = ncols.max(col_total);
        self.charge((g.cells.len() + g.rows.len() + ncols) as u64 + 4);
        let want_collapse = ts.text.border_collapse == BorderCollapse::Collapse;
        g.collapse = want_collapse && g.rows.len() * ncols <= MAX_EDGE_SLOTS;
        g.sp = if want_collapse { [LU::ZERO; 2] } else { [0, 1].map(|i| LU::from_f32(ts.text.border_spacing.get(i).map_or(0.0, |l| l.resolve(0.0))).max0()) };
        if g.collapse {
            self.collapse_edges(tid, &mut g, ncols);
        } else {
            for cell in &mut g.cells {
                let bd = &tree.get(cell.id).style.border;
                cell.bor = [0, 1, 2, 3].map(|i| border_px(bd.width.get(i).copied().unwrap_or(0.0)));
            }
        }
        self.size_columns(&mut g, ncols);
        g
    }

    /// CSS2 17.6.2: resolve every edge of the grid among the borders of the table, row groups, rows and cells; the
    /// cells and the table get the halves of the edges next to them.
    fn collapse_edges(&mut self, tid: BoxId, g: &mut Grid, ncols: usize) {
        let tree = self.tree;
        let nrows = g.rows.len();
        let vw = ncols + 1;
        let mut e = Edges { h: vec![Edge::NONE; (nrows + 1) * ncols], v: vec![Edge::NONE; nrows * vw] };
        let ts = &tree.get(tid).style;
        // Column 0 is the first column of the row: on the right for `direction: rtl`, so the vertical edge at the start of a
        // row is the right border there (`sl`, `sr`: the side of the left and right edge of the first and the last column).
        let rtl = ts.text.direction == Direction::Rtl;
        let (sl, sr) = if rtl { (1, 3) } else { (3, 1) };
        for c in 0..ncols {
            put(&mut e.h, c, Edge::of(ts, 0, 0));
            put(&mut e.h, nrows * ncols + c, Edge::of(ts, 2, 0));
        }
        for r in 0..nrows {
            put(&mut e.v, r * vw, Edge::of(ts, sl, 0));
            put(&mut e.v, r * vw + ncols, Edge::of(ts, sr, 0));
        }
        for (r, &rid) in g.rows.iter().enumerate() {
            let rb = tree.get(rid);
            let gid = rb.parent;
            let gs = &tree.get(gid).style;
            let same = |i: Option<&BoxId>| i.is_some_and(|&o| tree.get(o).parent == gid);
            let (first, last) = (r == 0 || !same(g.rows.get(r - 1)), !same(g.rows.get(r + 1)));
            for (st, src) in [(gs, 2u8), (&rb.style, 3u8)] {
                for c in 0..ncols {
                    if first || src == 3 {
                        put(&mut e.h, r * ncols + c, Edge::of(st, 0, src));
                    }
                    if last || src == 3 {
                        put(&mut e.h, (r + 1) * ncols + c, Edge::of(st, 2, src));
                    }
                }
                put(&mut e.v, r * vw, Edge::of(st, sl, src));
                put(&mut e.v, r * vw + ncols, Edge::of(st, sr, src));
            }
        }
        for cell in &g.cells {
            let st = &tree.get(cell.id).style;
            for c in cell.c..cell.c + cell.cs {
                put(&mut e.h, cell.r * ncols + c, Edge::of(st, 0, 4));
                put(&mut e.h, (cell.r + cell.rs) * ncols + c, Edge::of(st, 2, 4));
            }
            for r in cell.r..cell.r + cell.rs {
                put(&mut e.v, r * vw + cell.c, Edge::of(st, sl, 4));
                put(&mut e.v, r * vw + cell.c + cell.cs, Edge::of(st, sr, 4));
            }
        }
        for cell in &mut g.cells {
            let (cols, rows) = (cell.c..cell.c + cell.cs, cell.r..cell.r + cell.rs);
            let top = widest(&e.h, cols.clone().map(|c| cell.r * ncols + c));
            let bottom = widest(&e.h, cols.map(|c| (cell.r + cell.rs) * ncols + c));
            let first = widest(&e.v, rows.clone().map(|r| r * vw + cell.c));
            let after = widest(&e.v, rows.map(|r| r * vw + cell.c + cell.cs));
            let (left, right) = if rtl { (after, first) } else { (first, after) };
            cell.bor = [lu(hi(top)), lu(lo(right)), lu(lo(bottom)), lu(hi(left))];
        }
        let top = widest(&e.h, 0..ncols);
        let bottom = widest(&e.h, nrows * ncols..(nrows + 1) * ncols);
        let first = widest(&e.v, (0..nrows).map(|r| r * vw));
        let after = widest(&e.v, (0..nrows).map(|r| r * vw + ncols));
        let (left, right) = if rtl { (after, first) } else { (first, after) };
        g.tbor = [lu(lo(top)), lu(hi(right)), lu(hi(bottom)), lu(lo(left))];
        g.edges = Some(e);
    }

    /// The minimum / maximum width of every cell and column, and of the grid.
    fn size_columns(&mut self, g: &mut Grid, ncols: usize) {
        let tree = self.tree;
        g.cols = vec![Col::default(); ncols];
        for cell in &mut g.cells {
            let bs = &tree.get(cell.id).style.boxs;
            let p = |i: usize| bs.padding.get(i).map_or(LU::ZERO, |l| pad_px(l, LU::ZERO));
            let (hx, vx) = (p(1) + p(3) + cell.bor[1] + cell.bor[3], p(0) + p(2) + cell.bor[0] + cell.bor[2]);
            let (cmin, cmax) = self.content_widths(cell.id);
            cell.min = cmin + hx;
            cell.max = cmax + hx;
            let border_box = bs.box_sizing == BoxSizing::BorderBox;
            match &bs.width {
                Dim::Lp(Lp::Pct(p)) => {
                    cell.wpct = *p;
                    cell.pctx = if border_box { LU::ZERO } else { hx };
                }
                Dim::Lp(l) if resolve_dim(&bs.width, None).is_some() => {
                    let v = LU::from_f32(l.resolve(0.0)).max0();
                    cell.wfix = Some(if border_box { v } else { v + hx });
                    cell.max = cell.wfix.unwrap_or(v).max(cell.min);
                }
                _ => {}
            }
            cell.hfix = resolve_dim(&bs.height, None).map_or(LU::ZERO, |h| if border_box { h } else { h + vx });
        }
        for cell in g.cells.iter().filter(|c| c.cs == 1) {
            if let Some(col) = g.cols.get_mut(cell.c) {
                col.min = col.min.max(cell.min);
                col.max = col.max.max(cell.max);
                if cell.wpct > col.pct {
                    (col.pct, col.pctx) = (cell.wpct, cell.pctx);
                }
                col.fix = col.fix.max(cell.wfix.unwrap_or(LU::ZERO));
            }
        }
        for &(start, span, id, group) in &g.specs {
            let d = &tree.get(id).style.boxs.width;
            for col in g.cols.iter_mut().skip(start).take(span).filter(|_| !group) {
                match d {
                    Dim::Lp(Lp::Pct(p)) => {
                        col.cpct = *p;
                        col.pct = col.pct.max(*p);
                    }
                    _ => {
                        if let Some(w) = resolve_dim(d, None) {
                            col.cfix = w;
                            col.fix = col.fix.max(w);
                            col.max = col.max.max(w);
                        }
                    }
                }
            }
        }
        // Cells over several columns: the room they need above what their columns have goes to those columns.
        let mut spans: Vec<usize> = (0..g.cells.len()).filter(|&i| g.cells.get(i).is_some_and(|c| c.cs > 1)).collect();
        crate::sorting::sort_by_u64_key(&mut spans, |&i| g.cells.get(i).map_or(0, |c| c.cs as u64));
        for i in spans {
            let Some(cell) = g.cells.get(i) else { continue };
            let gaps = mul(g.sp[0], cell.cs - 1);
            let idx: Vec<usize> = (cell.c..cell.c + cell.cs).collect();
            for (is_min, want) in [(true, (cell.min - gaps).max0()), (false, (cell.max - gaps).max0())] {
                let get = |k: &Col| if is_min { k.min } else { k.max };
                let mut v: Vec<LU> = idx.iter().map(|&c| g.cols.get(c).map_or(LU::ZERO, get)).collect();
                let wt: Vec<i64> = idx.iter().map(|&c| i64::from(g.cols.get(c).map_or(0, |k| k.max.raw()))).collect();
                let extra = i64::from(want.raw()) - i64::from(sum(&v).raw());
                if extra > 0 {
                    let rel: Vec<usize> = (0..idx.len()).collect();
                    give(&mut v, &rel, &wt, extra);
                    for (&c, nv) in idx.iter().zip(v) {
                        if let Some(k) = g.cols.get_mut(c) {
                            if is_min { k.min = nv } else { k.max = nv }
                        }
                    }
                }
            }
        }
        for k in &mut g.cols {
            k.max = k.max.max(k.min);
        }
        // The widest the grid wants to be: percentage columns need the table to be wide enough for them.
        let sp_total = if ncols > 0 { mul(g.sp[0], ncols + 1) } else { LU::ZERO };
        let pct_sum: f32 = g.cols.iter().map(|k| k.pct).sum::<f32>().min(100.0);
        let nonpct: LU = g.cols.iter().filter(|k| k.pct <= 0.0).fold(LU::ZERO, |s, k| s + k.max);
        let mut max = g.cols.iter().fold(LU::ZERO, |s, k| s + k.max);
        for k in g.cols.iter().filter(|k| k.pct > 0.0) {
            max = max.max(LU::from_f32(k.max.to_f32() * 100.0 / k.pct));
        }
        if pct_sum > 0.0 && pct_sum < 100.0 {
            max = max.max(LU::from_f32(nonpct.to_f32() * 100.0 / (100.0 - pct_sum)));
        }
        g.min_total = g.cols.iter().fold(sp_total, |s, k| s + k.min);
        g.max_total = max + sp_total;
    }

    /// Lay out table `id` (a block formatting context root; laid out whole, so no first-screen stop line inside).
    pub(super) fn layout_table(&mut self, id: BoxId, cb_w: LU, cb_h: Option<LU>, flow: Flow) -> BlockResult {
        let tree = self.tree;
        let bs = &tree.get(id).style.boxs;
        if !self.charge(8) {
            return empty_result();
        }
        let grid = self.grid(id);
        let saved = (self.pos, self.touched, std::mem::take(&mut self.fs));
        self.pos = (LU::ZERO, LU::ZERO);
        let positioned = bs.position != Position::Static || super::has_transform(&self.tree.get(id).style);
        if positioned {
            self.cb_stack.push(id);
        }
        let mark = self.pending.len();
        self.no_stop += 1;
        let (ml, mt, mb) = self.table_inner(id, &grid, cb_w, cb_h, flow);
        self.no_stop -= 1;
        (self.pos, self.touched, self.fs) = saved;
        if positioned {
            self.cb_stack.pop();
            self.resolve_pending(Some(id), mark);
        }
        let h = self.out.get(id as usize).map_or(LU::ZERO, |o| o.h);
        let rel = super::block::relative_offset(bs, cb_w, cb_h);
        let (mt, mb) = (Margin::from(mt), Margin::from(mb));
        BlockResult { h, ml, mt, mb, top_only: mt, bot_only: mb, through: false, rel }
    }

    fn place(&mut self, id: BoxId, x: LU, y: LU, w: Option<LU>, h: LU) {
        if let Some(o) = self.out.get_mut(id as usize) {
            o.x = x;
            o.y = y;
            if let Some(w) = w {
                o.w = w;
            }
            o.h = h;
        }
    }

    /// The width, the cells, the rows, the captions. Returns the left, top and bottom margin of the table.
    fn table_inner(&mut self, id: BoxId, g: &Grid, cb_w: LU, cb_h: Option<LU>, flow: Flow) -> (LU, LU, LU) {
        let tree = self.tree;
        let tb = tree.get(id);
        let style = tb.style.clone();
        let (bs, bd) = (&style.boxs, &style.border);
        let (ncols, nrows) = (g.cols.len(), g.rows.len());
        let (pad, bor) = if g.collapse {
            ([LU::ZERO; 4], g.tbor)
        } else {
            let p = |i: usize| bs.padding.get(i).map_or(LU::ZERO, |l| pad_px(l, cb_w));
            let b = |i: usize| border_px(bd.width.get(i).copied().unwrap_or(0.0));
            ([p(0), p(1), p(2), p(3)], [b(0), b(1), b(2), b(3)])
        };
        let (ex, vx) = (pad[1] + pad[3] + bor[1] + bor[3], pad[0] + pad[2] + bor[0] + bor[2]);
        let marg = [0, 1, 2, 3].map(|i| bs.margin.get(i).and_then(|d| resolve_margin(d, cb_w)));
        let border_box = bs.box_sizing == BoxSizing::BorderBox;
        let to_cw = |v: LU| if border_box { (v - ex).max0() } else { v };

        // ---- the widths of the columns
        let sp_x = if ncols > 0 { mul(g.sp[0], ncols + 1) } else { LU::ZERO };
        let spec_w = resolve_dim(&bs.width, Some(cb_w)).map(to_cw);
        let fixed = bs.table_layout == TableLayout::Fixed && matches!(bs.width, Dim::Lp(_));
        let mut gt = match (flow.forced_w, spec_w) {
            (Some(w), _) | (None, Some(w)) => w,
            (None, None) => (cb_w - marg[3].unwrap_or(LU::ZERO) - marg[1].unwrap_or(LU::ZERO) - ex).max0().max(g.min_total).min(g.max_total),
        };
        if let Some(m) = resolve_dim(&bs.max_width, Some(cb_w)).map(to_cw) {
            gt = gt.min(m);
        }
        if let Some(m) = resolve_dim(&bs.min_width, Some(cb_w)).map(to_cw) {
            gt = gt.max(m);
        }
        // The table is at least as wide as its widest caption.
        for &cid in &tb.children {
            if role_of(tree.get(cid)) == Role::Caption {
                gt = gt.max((self.outer_widths(cid).0 - ex).max0());
            }
        }
        if !fixed {
            gt = gt.max(g.min_total);
        }
        let room = (gt - sp_x).max0();
        let colw = if fixed { fixed_widths(g, room) } else { auto_widths(&g.cols, room) };
        let gw = (sum(&colw) + sp_x).max(gt);
        let tw = gw + ex;
        let (sp0, sp1) = (g.sp[0], g.sp[1]);
        let lead_x = if ncols > 0 { sp0 } else { LU::ZERO };
        // A table with `direction: rtl` lays its columns out from the right (CSS2 17.5.3), collapsed borders included.
        // `cx` is cumulated in painting order; `span` is the x range of columns.
        let rtl = style.text.direction == Direction::Rtl;
        let mut cx = vec![lead_x];
        let mut x = lead_x;
        for i in 0..colw.len() {
            x = x + colw.get(if rtl { colw.len() - 1 - i } else { i }).copied().unwrap_or(LU::ZERO) + sp0;
            cx.push(x);
        }
        let span = |c: usize, n: usize| {
            let (a, b) = if rtl { (ncols.saturating_sub(c + n), ncols.saturating_sub(c)) } else { (c, c + n) };
            (at(&cx, a), at(&cx, b) - sp0)
        };

        // ---- the cells, and the heights of the rows
        let mut rh = g.row_h.clone();
        let (mut above, mut below) = (vec![LU::ZERO; nrows], vec![LU::ZERO; nrows]);
        // Per cell: natural height, baseline (for `vertical-align: baseline`), alignment (0 top, 1 middle, 2 bottom, 3 baseline).
        let mut nat: Vec<(LU, LU, u8)> = Vec::with_capacity(g.cells.len());
        for cell in &g.cells {
            let cstyle = &tree.get(cell.id).style;
            let (cx0, cx1) = span(cell.c, cell.cs);
            let cw = cx1 - cx0;
            let p = |i: usize| cstyle.boxs.padding.get(i).map_or(LU::ZERO, |l| pad_px(l, gw));
            let (hx, vx) = (p(1) + p(3) + cell.bor[1] + cell.bor[3], p(0) + p(2) + cell.bor[0] + cell.bor[2]);
            self.layout_block(cell.id, gw, None, Flow { forced_w: Some((cw - hx).max0()), cell_bor: Some(cell.bor), ..Flow::default() });
            let h = self.out.get(cell.id as usize).map_or(LU::ZERO, |o| o.h);
            let va = match cstyle.misc.vertical_align {
                VerticalAlign::Top => 0,
                VerticalAlign::Middle => 1,
                VerticalAlign::Bottom => 2,
                _ => 3,
            };
            let base = if va == 3 { self.first_baseline(cell.id).unwrap_or(cell.bor[0] + p(0) + (h - vx).max0()) } else { LU::ZERO };
            if !g.collapse
                && cstyle.text.empty_cells == EmptyCells::Hide
                && h <= vx
                && let Some(o) = self.out.get_mut(cell.id as usize)
            {
                o.hide = true;
            }
            let hh = h.max(cell.hfix);
            if cell.rs == 1 {
                if va == 3 {
                    bump(&mut above, cell.r, base);
                    bump(&mut below, cell.r, hh - base);
                } else {
                    bump(&mut rh, cell.r, hh);
                }
            }
            nat.push((h, base, va));
        }
        for r in 0..nrows {
            bump(&mut rh, r, at(&above, r) + at(&below, r));
        }
        for (cell, n) in g.cells.iter().zip(&nat).filter(|(c, _)| c.rs > 1) {
            let rows: Vec<usize> = (cell.r..cell.r + cell.rs).collect();
            let have = rows.iter().fold(mul(sp1, cell.rs - 1), |s, &r| s + at(&rh, r));
            let need = n.0.max(cell.hfix);
            if need > have {
                give(&mut rh, &rows, &vec![1; rows.len()], i64::from((need - have).raw()));
            }
        }
        let lead_y = if nrows > 0 { sp1 } else { LU::ZERO };
        let rows_h = |rh: &[LU]| sum(rh) + if nrows > 0 { mul(sp1, nrows + 1) } else { LU::ZERO };
        let to_bb = |h: LU| if border_box { h } else { h + vx };
        // A forced height (a stretched flex item) acts as the height; `ignore_h` measures the table by its rows.
        let (spec, min_h) = if flow.ignore_h {
            (None, None)
        } else {
            (flow.forced_h.map(to_bb).or_else(|| resolve_dim(&bs.height, cb_h).map(to_bb)), resolve_dim(&bs.min_height, cb_h).map(to_bb))
        };
        let spec_h = match (spec, min_h) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (a, b) => a.or(b),
        };
        // `visibility: collapse` on a row (or a row group): the row and the spacing after it are gone (CSS2 17.5.5).
        let gone: Vec<bool> = g.rows.iter().map(|&r| tree.get(r).style.text.visibility == Visibility::Collapse).collect();
        for (r, _) in gone.iter().enumerate().filter(|(_, c)| **c) {
            if let Some(h) = rh.get_mut(r) {
                *h = LU::ZERO;
            }
        }
        if nrows > 0
            && let Some(t) = spec_h
        {
            let have = rows_h(&rh) + vx;
            if t > have {
                let idx: Vec<usize> = (0..nrows).collect();
                let wt: Vec<i64> = rh.iter().map(|h| i64::from(h.raw())).collect();
                give(&mut rh, &idx, &wt, i64::from((t - have).raw()));
            }
        }
        let mut ry = vec![lead_y];
        let mut y = lead_y;
        for (h, gone) in rh.iter().zip(&gone) {
            y = y + *h + if *gone { LU::ZERO } else { sp1 };
            ry.push(y);
        }
        let box_h = (if nrows > 0 { y } else { LU::ZERO } + vx).max(spec_h.unwrap_or(LU::ZERO));

        // ---- the captions
        let (mut top_h, mut bottom) = (LU::ZERO, Vec::new());
        for &cid in &tb.children {
            let cb = tree.get(cid);
            if role_of(cb) != Role::Caption {
                continue;
            }
            let r = self.layout_block(cid, tw, None, Flow::default());
            let h = self.out.get(cid as usize).map_or(LU::ZERO, |o| o.h);
            if cb.style.text.caption_side == CaptionSide::Bottom {
                bottom.push((cid, r.ml, r.mt.resolve(), h, r.mb.resolve()));
            } else {
                let cy = top_h + r.mt.resolve();
                self.place(cid, r.ml + r.rel.0, cy + r.rel.1, None, h);
                top_h = cy + h + r.mb.resolve();
            }
        }
        let mut total_h = top_h + box_h;
        for (cid, ml, mt, h, mb) in bottom {
            self.place(cid, ml, total_h + mt, None, h);
            total_h = total_h + mt + h + mb;
        }

        // ---- place the parts
        let (gx, gy) = (bor[3] + pad[3], top_h + bor[0] + pad[0]);
        let inner_w = (gw - lead_x - lead_x).max0();
        let mut r0 = 0usize;
        for &gid in &tb.children {
            let gb = tree.get(gid);
            if role_of(gb) != Role::Group {
                continue;
            }
            let k = gb.children.len().min(nrows - r0);
            let top = at(&ry, r0);
            let end = if k > 0 { at(&ry, r0 + k - 1) + at(&rh, r0 + k - 1) } else { top };
            self.place(gid, gx + lead_x, gy + top, Some(inner_w), end - top);
            for (j, &rid) in gb.children.iter().enumerate().take(k) {
                self.place(rid, LU::ZERO, at(&ry, r0 + j) - top, Some(inner_w), at(&rh, r0 + j));
            }
            r0 += k;
        }
        for (cell, &(h, base, va)) in g.cells.iter().zip(&nat) {
            let rows: Vec<usize> = (cell.r..cell.r + cell.rs).collect();
            let full = rows.iter().fold(mul(sp1, cell.rs - 1), |s, &r| s + at(&rh, r));
            let spare = (full - h).max0();
            let shift = match va {
                1 => spare.half(),
                2 => spare,
                3 if cell.rs == 1 => (at(&above, cell.r) - base).max0(),
                _ => LU::ZERO,
            };
            // The cell is as wide as its columns even when its padding is wider.
            let (cx0, cx1) = span(cell.c, cell.cs);
            let cw = cx1 - cx0;
            self.place(cell.id, cx0 - lead_x, LU::ZERO, Some(cw), full);
            if shift > LU::ZERO
                && let Some(&w) = tree.get(cell.id).children.first()
                && let Some(o) = self.out.get_mut(w as usize)
            {
                o.y += shift;
            }
        }
        let gh = if nrows > 0 { y } else { LU::ZERO };
        for pass in [true, false] {
            for &(start, n, bid, group) in &g.specs {
                if group != pass {
                    continue;
                }
                let (sx0, sx1) = span(start, n);
                let (x0, x1) = (gx + sx0, gx + sx1);
                let parent = tree.get(bid).parent;
                let (px_, py) = match self.out.get(parent as usize) {
                    Some(o) if role_of(tree.get(parent)) == Role::ColGroup => (o.x, o.y),
                    _ => (LU::ZERO, LU::ZERO),
                };
                self.place(bid, x0 - px_, gy - py, Some((x1 - x0).max0()), gh);
            }
        }

        // ---- collapsed borders to paint
        let mut segs: Vec<Seg> = Vec::new();
        if let Some(e) = &g.edges {
            let vw = ncols + 1;
            // `xs(b)`: the x of the boundary after `b` columns in painting order; the logical column of a place is `lc`.
            let (xs, ys) = (|b: usize| gx + at(&cx, b), |r: usize| gy + at(&ry, r));
            let lc = |p: usize| if rtl { ncols - p } else { p };
            let hw = |r: usize, c: usize| if c < ncols { e.h.get(r * ncols + c).map_or(0, |d| d.w) } else { 0 };
            let live = |d: &Edge| d.w > 0 && !d.hidden && d.color.a > 0;
            for r in 0..=nrows {
                for p in 0..ncols {
                    let c = lc(p + 1).min(ncols.saturating_sub(1)) * usize::from(rtl) + p * usize::from(!rtl);
                    let Some(d) = e.h.get(r * ncols + c).filter(|d| live(d)) else { continue };
                    let (x0, x1) = (xs(p), xs(p + 1));
                    let (sy, sh) = (ys(r) - lu(lo(d.w)), lu(d.w));
                    match segs.last_mut() {
                        Some(l) if l.1 == sy && l.3 == sh && l.4 == d.color && l.0 + l.2 == x0 => l.2 = x1 - l.0,
                        _ => segs.push((x0, sy, x1 - x0, sh, d.color)),
                    }
                }
            }
            for p in 0..=ncols {
                let c = lc(p);
                for r in 0..nrows {
                    let Some(d) = e.v.get(r * vw + c).filter(|d| live(d)) else { continue };
                    let top = lo(hw(r, c.wrapping_sub(1)).max(hw(r, c)));
                    let bot = hi(hw(r + 1, c.wrapping_sub(1)).max(hw(r + 1, c)));
                    let (y0, y1) = (ys(r) - lu(top), ys(r + 1) + lu(bot));
                    let sx = xs(p) - lu(lo(d.w));
                    match segs.last_mut() {
                        Some(l) if l.0 == sx && l.2 == lu(d.w) && l.4 == d.color && l.1 <= y1 && y0 <= l.1 + l.3 => {
                            let end = (l.1 + l.3).max(y1);
                            l.1 = l.1.min(y0);
                            l.3 = end - l.1;
                        }
                        _ => segs.push((sx, y0, lu(d.w), y1 - y0, d.color)),
                    }
                }
            }
        }
        if top_h > LU::ZERO || total_h > top_h + box_h || !segs.is_empty() {
            self.tinfo.insert(id, TableInfo { y: top_h, h: box_h, segs });
        }
        if g.edges.is_some() {
            self.halves.insert(id, g.tbor);
            for cell in &g.cells {
                self.halves.insert(cell.id, cell.bor);
            }
        }
        if let Some(o) = self.out.get_mut(id as usize) {
            o.w = tw;
            o.h = total_h;
            o.clip = bs.overflow_x != Overflow::Visible || bs.overflow_y != Overflow::Visible;
            o.lines = (0, 0);
            o.marker = 0;
        }
        let ml = if flow.forced_w.is_some() { marg[3].unwrap_or(LU::ZERO) } else { solve_width(cb_w, Some(gw), marg[3], marg[1], ex, LU::ZERO, None, self.tree.get(self.tree.get(id).parent).style.text.direction == Direction::Rtl).1 };
        (ml, marg[0].unwrap_or(LU::ZERO), marg[2].unwrap_or(LU::ZERO))
    }
}
