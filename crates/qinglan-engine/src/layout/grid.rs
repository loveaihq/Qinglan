//! CSS grid layout (css-grid-1): the explicit and implicit grid (7), line and area placement (8), auto-placement (8.5),
//! the track sizing algorithm (11.4-11.8) with spanning items, `fr`, `minmax()`, `fit-content()` and `repeat()`
//! (`auto-fill` / `auto-fit`), alignment (10), out-of-flow children (9) and the intrinsic sizes of a grid container (12.1
//! of the 2020 draft: "Grid Container Intrinsic Sizes"). Writing modes other than `horizontal-tb` and `subgrid` are not
//! done; `direction: rtl` mirrors the columns.
//!
//! A grid container is a formatting context root laid out like a flex container: whole, knowing only its containing
//! block's width. Every number is a raw layout unit (1/64 px) in an `i64`. Axis 0 is the rows (block axis), axis 1 the
//! columns (inline axis); [`Ax`] keeps one value per axis so that code can pick the axis without indexing.
//!
//! Cost: the columns are sized from intrinsic widths (memoised, no layout). A row whose size depends on an item needs the
//! item's height at its final width: one throw-away layout per item (`flex_measure`, remembered by (item, width), so
//! nested grids measure each level once and the work stays polynomial in the depth, see `tests/grid.rs`), then the item
//! is laid out once for real. Items in rows with fixed sizes are not measured.

use std::rc::Rc;

use super::block::{BlockResult, Flow, Geom, border_px, lp_has_percent, pad_px, relative_offset, resolve_dim, resolve_margin};
use super::boxtree::{BoxId, BoxKind, LayoutBox};
use super::flex::Pos;
use super::units::{LayoutUnit as LU, Margin};
use super::{Layouter, Pending, StaticAlign};
use crate::css::types::{AlignKind, AlignValue, Bread, BoxSizing, Dim, Direction, GRID_MAX_TRACKS, GridLine, Overflow, Position, Track, TrackList};
use crate::css::values::Lp;

/// A size that is not known (a growth limit before an item has been looked at).
const INF: i64 = i64::MAX / 16;
const MAXT: usize = GRID_MAX_TRACKS;

fn r(l: LU) -> i64 {
    i64::from(l.raw())
}

fn lu(v: i64) -> LU {
    LU::from_raw(v)
}

fn div_round(a: i64, b: i64) -> i64 {
    if b <= 0 { 0 } else { (a + b / 2) / b }
}

/// One value per axis: `r` for the rows (axis 0), `c` for the columns (axis 1).
#[derive(Clone, Copy, Default)]
struct Ax<T> {
    r: T,
    c: T,
}

impl<T> Ax<T> {
    fn g(&self, a: usize) -> &T {
        if a == 0 { &self.r } else { &self.c }
    }

    fn m(&mut self, a: usize) -> &mut T {
        if a == 0 { &mut self.r } else { &mut self.c }
    }
}

impl<T: Copy> Ax<T> {
    fn v(&self, a: usize) -> T {
        *self.g(a)
    }
}

/// How the container is being sized along the axis: to a definite size, or under a min-content / max-content constraint
/// (css-grid-1 11.5; an indefinite block size is sized like max-content).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Def,
    Min,
    Max,
}

#[derive(Clone, Copy, PartialEq)]
enum Mn {
    Fixed(i64),
    Min,
    Max,
    Auto,
}

#[derive(Clone, Copy, PartialEq)]
enum Mx {
    Fixed(i64),
    Fr,
    Min,
    Max,
    Auto,
}

/// A track while it is sized (11.4): its sizing function, base size and growth limit.
#[derive(Clone, Copy)]
struct Tr {
    min: Mn,
    max: Mx,
    base: i64,
    limit: i64,
    /// Marked "infinitely growable" (11.5).
    grow: bool,
    flex: f32,
    /// The argument of `fit-content()`, `INF` for other tracks.
    fit: i64,
    /// An `auto-fit` track without items: sized 0 and its gutters collapse.
    gone: bool,
    /// Start of the track from the start of the content box.
    pos: i64,
}

impl Tr {
    fn intrinsic_min(&self) -> bool {
        matches!(self.min, Mn::Min | Mn::Max | Mn::Auto)
    }

    fn intrinsic_max(&self) -> bool {
        matches!(self.max, Mx::Min | Mx::Max | Mx::Auto)
    }

    fn flexible(&self) -> bool {
        self.max == Mx::Fr
    }
}

/// A `<track-size>` with the lengths and percentages worked out. A percentage with no definite `pct` base behaves as `auto`.
fn mk_tr(t: &Track, pct: Option<i64>) -> Tr {
    let res = |l: &Lp| -> Option<i64> {
        if lp_has_percent(l) {
            pct.map(|b| r(LU::from_f32(l.resolve(lu(b).to_f32()))))
        } else {
            Some(r(LU::from_f32(l.resolve(0.0))))
        }
    };
    let min = match &t.min {
        Bread::Lp(l) => res(l).map_or(Mn::Auto, |v| Mn::Fixed(v.max(0))),
        Bread::MinContent => Mn::Min,
        Bread::MaxContent => Mn::Max,
        _ => Mn::Auto,
    };
    let (mut fit, mut flex) = (INF, 0.0);
    let max = match &t.max {
        Bread::Lp(l) if t.fit => {
            fit = res(l).unwrap_or(INF).max(0);
            Mx::Max
        }
        Bread::Lp(l) => res(l).map_or(Mx::Auto, |v| Mx::Fixed(v.max(0))),
        Bread::Fr(f) => {
            flex = *f;
            Mx::Fr
        }
        Bread::MinContent => Mx::Min,
        Bread::MaxContent => Mx::Max,
        Bread::Auto => Mx::Auto,
    };
    let base = if let Mn::Fixed(v) = min { v } else { 0 };
    let limit = if let Mx::Fixed(v) = max { v.max(base) } else { INF };
    Tr { min, max, base, limit, grow: false, flex, fit, gone: false, pos: 0 }
}

/// The sizes the container gives an axis: a definite content size, and the `min-*` / `max-*` limits of it.
#[derive(Clone, Copy)]
struct Space {
    avail: Option<i64>,
    min: i64,
    max: Option<i64>,
}

#[derive(Clone)]
struct GItem {
    id: BoxId,
    /// Start track and span per axis; `def`: the start is definite. Start tracks are in implicit-grid coordinates once placed.
    st: Ax<i64>,
    sp: Ax<usize>,
    def: Ax<bool>,
    ok: bool,
    /// (minimum, min-content, max-content) contribution per axis, once asked for.
    cc: Ax<Option<(i64, i64, i64)>>,
}

struct Grid {
    items: Vec<GItem>,
    tr: Ax<Vec<Tr>>,
    gap: Ax<i64>,
    /// `justify-items` and `align-items` of the container.
    ji: AlignValue,
    ai: AlignValue,
    rtl: bool,
    /// The tracks that `auto-fit` may collapse, per axis: (first, count).
    fit: Ax<Option<(usize, usize)>>,
    /// The names of the lines of the explicit grid, the number of explicit tracks and of implicit ones before them.
    names: Ax<Names>,
    ex: Ax<usize>,
    neg: Ax<usize>,
    /// First-screen-first is on inside this container: the estimated page y of its content box (raw units).
    stop: Option<i64>,
}

// ---------------------------------------------------------------------------------------------
// Lines and placement (8.3, 8.5)

/// Indices of the lines (0-based, 0 is the first line of the explicit grid) called `name`, ascending.
fn named_lines(names: &[Vec<Rc<str>>], name: &str) -> Vec<i64> {
    names.iter().enumerate().filter(|(_, v)| v.iter().any(|s| &**s == name)).map(|(i, _)| i as i64).collect()
}

/// The `n`-th line called `name`, counted from the start (n > 0) or the end of the explicit grid; when there are not enough,
/// the implicit lines beyond the explicit grid count as having the name (8.3).
fn find_named(names: &[Vec<Rc<str>>], name: &str, n: i32, ex: i64) -> i64 {
    let lines = named_lines(names, name);
    let (len, k) = (lines.len() as i64, i64::from(n));
    if k > 0 {
        if k <= len { lines.get((k - 1) as usize).copied().unwrap_or(ex + 1) } else { ex + (k - len) }
    } else {
        let m = -k;
        if m <= len { lines.get((len - m) as usize).copied().unwrap_or(0) } else { -(m - len) }
    }
}

enum E<'a> {
    Auto,
    Line(i64),
    Span(usize, Option<&'a str>),
}

/// The line `n` tracks after `from`, or the one with the `n`-th occurrence of `name` after it.
fn span_end(names: &[Vec<Rc<str>>], ex: i64, from: i64, n: usize, name: Option<&str>) -> i64 {
    let Some(nm) = name else { return from + n as i64 };
    let after: Vec<i64> = named_lines(names, nm).into_iter().filter(|&l| l > from).collect();
    match after.get(n.saturating_sub(1)) {
        Some(&l) if n <= after.len() => l,
        _ => ex.max(from) + (n - after.len().min(n)) as i64,
    }
}

fn span_start(names: &[Vec<Rc<str>>], to: i64, n: usize, name: Option<&str>) -> i64 {
    let Some(nm) = name else { return to - n as i64 };
    let before: Vec<i64> = named_lines(names, nm).into_iter().rev().filter(|&l| l < to).collect();
    match before.get(n.saturating_sub(1)) {
        Some(&l) if n <= before.len() => l,
        _ => to.min(0) - (n - before.len().min(n)) as i64,
    }
}

/// The lines the start and end of an item resolve to (8.3.1) -- `None` is an `auto` edge -- and its span when both are
/// `auto`. Lines are 0-based indices in the explicit grid and can be negative or beyond its end.
fn edges(names: &[Vec<Rc<str>>], ex: i64, s: &GridLine, e: &GridLine) -> (Option<i64>, Option<i64>, usize) {
    fn conv<'a>(names: &[Vec<Rc<str>>], ex: i64, l: &'a GridLine, end: bool) -> E<'a> {
        match l {
            GridLine::Auto => E::Auto,
            GridLine::Num(n) => E::Line(if *n > 0 { i64::from(*n) - 1 } else { ex + 1 + i64::from(*n) }),
            GridLine::Ident(nm) => {
                // The edge of a named area (`name-start` / `name-end`), else the first line with that name.
                let key = format!("{nm}-{}", if end { "end" } else { "start" });
                E::Line(named_lines(names, &key).first().copied().unwrap_or_else(|| find_named(names, nm, 1, ex)))
            }
            GridLine::NumIdent(n, nm) => E::Line(find_named(names, nm, *n, ex)),
            GridLine::Span(n) => E::Span(*n as usize, None),
            GridLine::SpanIdent(n, nm) => E::Span(*n as usize, Some(nm)),
        }
    }
    match (conv(names, ex, s, false), conv(names, ex, e, true)) {
        (E::Line(a), E::Line(b)) if a == b => (Some(a), None, 1),
        (E::Line(a), E::Line(b)) => (Some(a.min(b)), Some(a.max(b)), 1),
        (E::Line(a), E::Span(n, nm)) => (Some(a), Some(span_end(names, ex, a, n, nm)), 1),
        (E::Span(n, nm), E::Line(b)) => (Some(span_start(names, b, n, nm)), Some(b), 1),
        (E::Line(a), E::Auto) => (Some(a), None, 1),
        (E::Auto, E::Line(b)) => (None, Some(b), 1),
        (E::Span(n, _), _) | (E::Auto, E::Span(n, _)) => (None, None, n.clamp(1, MAXT)),
        (E::Auto, E::Auto) => (None, None, 1),
    }
}

/// An item's position on one axis (8.3, 8.3.1): (definite, start line, span). An `auto` end is a span of 1.
fn resolve_axis(names: &[Vec<Rc<str>>], ex: i64, s: &GridLine, e: &GridLine) -> (bool, i64, usize) {
    match edges(names, ex, s, e) {
        (Some(a), Some(b), _) => (true, a, ((b - a) as usize).clamp(1, MAXT)),
        (Some(a), None, _) => (true, a, 1),
        (None, Some(b), _) => (true, b - 1, 1),
        (None, None, n) => (false, 0, n),
    }
}

/// Which cells of the implicit grid are taken: one bit per cell, a row of 64-bit words per line of the major axis.
#[derive(Default)]
struct Occ {
    lines: Vec<Vec<u64>>,
}

/// The bits `lo..hi` of a word (`lo < hi <= 64`).
fn word_mask(lo: usize, hi: usize) -> u64 {
    let top = if hi >= 64 { !0 } else { (1u64 << hi) - 1 };
    top & !((1u64 << lo) - 1)
}

/// The first and last word the cells `n0..n0 + ns` touch.
fn words(n0: usize, ns: usize) -> (usize, usize) {
    (n0 / 64, (n0 + ns.max(1) - 1) / 64)
}

/// The mask of those cells inside the word `w`.
fn mask_in(w: usize, n0: usize, ns: usize) -> u64 {
    let (w0, w1) = words(n0, ns);
    let lo = if w == w0 { n0 % 64 } else { 0 };
    let hi = if w == w1 { (n0 + ns.max(1) - 1) % 64 + 1 } else { 64 };
    word_mask(lo, hi)
}

impl Occ {
    /// What looking at or filling `ms` lines of `ns` cells from column `n0` costs (work units): a step, plus a unit per
    /// eight words.
    fn cost(ms: usize, n0: usize, ns: usize) -> u64 {
        let (w0, w1) = words(n0, ns);
        1 + (ms as u64).saturating_mul((w1 - w0 + 1) as u64) / 8
    }

    /// The last taken cell in lines `m0..m0 + ms`, columns `n0..n0 + ns` as (line, column), or `None` if all are free. Every
    /// start that does not clear it overlaps it, so the caller can jump past it.
    fn hit(&self, m0: usize, ms: usize, n0: usize, ns: usize) -> Option<(usize, usize)> {
        let (w0, w1) = words(n0, ns);
        let mut last: Option<(usize, usize)> = None;
        for m in m0..m0 + ms {
            let Some(row) = self.lines.get(m) else { break };
            let mut col: Option<usize> = None;
            for w in w0..=w1 {
                let Some(&bits) = row.get(w) else { break };
                let h = bits & mask_in(w, n0, ns);
                if h != 0 {
                    col = Some(w * 64 + 63 - h.leading_zeros() as usize);
                }
            }
            if let Some(c) = col {
                last = Some((m, last.map_or(c, |(_, l)| l.max(c))));
            }
        }
        last
    }

    fn set(&mut self, m0: usize, ms: usize, n0: usize, ns: usize) {
        let (w0, w1) = words(n0, ns);
        if self.lines.len() < m0 + ms {
            self.lines.resize(m0 + ms, Vec::new());
        }
        for m in m0..m0 + ms {
            let Some(row) = self.lines.get_mut(m) else { continue };
            if row.len() <= w1 {
                row.resize(w1 + 1, 0);
            }
            for w in w0..=w1 {
                if let Some(x) = row.get_mut(w) {
                    *x |= mask_in(w, n0, ns);
                }
            }
        }
    }

    /// Whether the cell is taken.
    fn taken(&self, m: usize, c: usize) -> bool {
        self.lines.get(m).and_then(|row| row.get(c / 64)).is_some_and(|w| (w >> (c % 64)) & 1 != 0)
    }

    /// Whether every one of the first `n` cells of line `m` is taken.
    fn full(&self, m: usize, n: usize) -> bool {
        let Some(row) = self.lines.get(m) else { return false };
        let (_, w1) = words(0, n);
        (0..=w1).all(|w| row.get(w).is_some_and(|&b| b & mask_in(w, 0, n) == mask_in(w, 0, n)))
    }
}

fn us(v: i64) -> usize {
    v.clamp(0, MAXT as i64) as usize
}

/// The work `place` has done and the most it may do (what the page budget has left).
struct Spend {
    work: u64,
    limit: u64,
}

impl Spend {
    /// Count `n` units; false once the limit is passed.
    fn go(&mut self, n: u64) -> bool {
        self.work = self.work.saturating_add(n);
        self.work <= self.limit
    }

    fn out(&self) -> bool {
        self.work > self.limit
    }
}

/// 8.5: put every item at a definite position; `n` is the size of the implicit grid so far and grows. Every cell or word
/// looked at is counted, and when `limit` units are used up the rest of the items are dropped (`ok = false`). Returns the
/// work done and whether the limit was hit.
///
/// A scan jumps over taken cells (`Occ::hit` names the last one in the window). In a dense grid the search starts at the
/// first line with a free cell, and, for an item with a column of its own, at the first free cell of that column, so
/// packing a lot of items stays linear instead of starting over from line 0 every time.
fn place(items: &mut [GItem], n: &mut Ax<usize>, maj: usize, dense: bool, limit: u64) -> (u64, bool) {
    let mn = 1 - maj;
    let mut sp = Spend { work: items.len() as u64, limit };
    let mut occ = Occ::default();
    // Step 2: items with a definite position on both axes.
    for it in items.iter_mut().filter(|i| i.ok && i.def.r && i.def.c) {
        let (m, ms, c, ns) = (us(it.st.v(maj)), it.sp.v(maj), us(it.st.v(mn)), it.sp.v(mn));
        if !sp.go(Occ::cost(ms, c, ns)) {
            it.ok = false;
            continue;
        }
        occ.set(m, ms, c, ns);
    }
    // Step 3: items locked to a line of the major axis. Sparse packing goes on past the items already put on that line.
    let mut last_end: Vec<usize> = Vec::new();
    for it in items.iter_mut().filter(|i| i.ok && i.def.v(maj) && !i.def.v(mn)) {
        let (m, ms, ns) = (us(it.st.v(maj)), it.sp.v(maj), it.sp.v(mn));
        let mut c = if dense { 0 } else { (m..m + ms).filter_map(|l| last_end.get(l).copied()).max().unwrap_or(0) };
        while c < MAXT && !sp.out() {
            sp.go(Occ::cost(ms, c, ns));
            match occ.hit(m, ms, c, ns) {
                Some((_, col)) => c = col + 1,
                None => break,
            }
        }
        if sp.out() || c >= MAXT {
            it.ok = false;
            continue;
        }
        *it.st.m(mn) = c as i64;
        *it.def.m(mn) = true;
        sp.go(Occ::cost(ms, c, ns));
        occ.set(m, ms, c, ns);
        if last_end.len() < m + ms {
            last_end.resize(m + ms, 0);
        }
        for l in m..m + ms {
            if let Some(e) = last_end.get_mut(l) {
                *e = c + ns;
            }
        }
    }
    // Step 4: the size of the implicit grid in the minor axis, and in the major one for what is placed.
    for it in items.iter().filter(|i| i.ok) {
        for a in [maj, mn] {
            let end = if it.def.v(a) { us(it.st.v(a)) + it.sp.v(a) } else { it.sp.v(a) };
            *n.m(a) = n.v(a).max(end.min(MAXT));
        }
    }
    // Step 5: the rest, with the auto-placement cursor.
    let (mut cm, mut cn) = (0usize, 0usize);
    let (mut first_free, mut col_free): (usize, Vec<usize>) = (0, Vec::new());
    for it in items.iter_mut().filter(|i| i.ok && !i.def.v(maj)) {
        if sp.out() {
            it.ok = false;
            continue;
        }
        let (ms, ns, nmn) = (it.sp.v(maj), it.sp.v(mn), n.v(mn));
        if dense {
            while first_free < MAXT && occ.full(first_free, nmn) && sp.go(1 + nmn as u64 / 512) {
                first_free += 1;
            }
        }
        if it.def.v(mn) {
            let c = us(it.st.v(mn));
            if dense {
                // Nothing above the first free cell of the column, or the first line with a free cell, can take the item.
                if col_free.len() <= c {
                    col_free.resize(c + 1, 0);
                }
                if let Some(f) = col_free.get_mut(c) {
                    while *f < MAXT && occ.taken(*f, c) && sp.go(1) {
                        *f += 1;
                    }
                    cm = first_free.max(*f);
                }
            } else if c < cn {
                cm += 1;
            }
            cn = c;
            while cm < MAXT && !sp.out() {
                sp.go(Occ::cost(ms, c, ns));
                match occ.hit(cm, ms, c, ns) {
                    Some((line, _)) => cm = line + 1,
                    None => break,
                }
            }
        } else {
            if dense {
                (cm, cn) = (first_free, 0);
            }
            while cm < MAXT && !sp.out() {
                if cn + ns > nmn {
                    sp.go(1);
                    cm += 1;
                    cn = 0;
                    continue;
                }
                sp.go(Occ::cost(ms, cn, ns));
                match occ.hit(cm, ms, cn, ns) {
                    Some((_, col)) => cn = col + 1,
                    None => break,
                }
            }
        }
        if sp.out() || cm + ms > MAXT {
            it.ok = false;
            continue;
        }
        *it.st.m(maj) = cm as i64;
        *it.st.m(mn) = cn as i64;
        *it.def.m(maj) = true;
        *it.def.m(mn) = true;
        sp.go(Occ::cost(ms, cn, ns));
        occ.set(cm, ms, cn, ns);
        *n.m(maj) = n.v(maj).max(cm + ms);
    }
    (sp.work, sp.out())
}

// ---------------------------------------------------------------------------------------------
// The explicit grid (7.1, 7.2)

type Names = Vec<Vec<Rc<str>>>;

/// The tracks and line names of `tl` with the auto repeat expanded `reps` times, and the range of its tracks.
fn expand(tl: &TrackList, reps: usize) -> (Vec<Track>, Names, Option<(usize, usize, bool)>) {
    fn put(names: &mut Names, src: Option<&Vec<Rc<str>>>) {
        if let (Some(l), Some(s)) = (names.last_mut(), src) {
            l.extend(s.iter().cloned());
        }
    }
    fn add(tracks: &mut Vec<Track>, names: &mut Names, t: &Track) {
        tracks.push(t.clone());
        names.push(Vec::new());
    }
    let (mut tracks, mut names, mut range): (Vec<Track>, Names, _) = (Vec::new(), vec![Vec::new()], None);
    for i in 0..=tl.tracks.len() {
        if let Some(a) = tl.auto.as_ref().filter(|a| a.at == i) {
            put(&mut names, tl.names.get(i));
            let start = tracks.len();
            for _ in 0..reps {
                for (k, t) in a.tracks.iter().enumerate() {
                    put(&mut names, a.names.get(k));
                    add(&mut tracks, &mut names, t);
                }
                put(&mut names, a.names.last());
            }
            range = Some((start, tracks.len() - start, a.fit));
            put(&mut names, tl.names.get(i + 1));
        } else {
            put(&mut names, tl.names.get(i + usize::from(tl.auto.as_ref().is_some_and(|a| a.at < i))));
        }
        if let Some(t) = tl.tracks.get(i) {
            add(&mut tracks, &mut names, t);
        }
    }
    (tracks, names, range)
}

/// 7.2.3.2: how many times an `auto-fill` / `auto-fit` repeat fits in the axis.
fn auto_reps(tl: &TrackList, gap: i64, sp: Space) -> usize {
    let Some(a) = &tl.auto else { return 1 };
    let (target, up) = match (sp.avail, sp.max) {
        (Some(v), _) | (None, Some(v)) => (v, false),
        (None, None) if sp.min > 0 => (sp.min, true),
        _ => return 1,
    };
    let px = |t: &Track| -> i64 {
        let one = |b: &Bread| if let Bread::Lp(l) = b { Some(r(LU::from_f32(l.resolve(lu(target).to_f32())))) } else { None };
        // The max if it is definite, else the min, the max floored by the min; at least 1px (a safe divisor).
        one(&t.max).map_or_else(|| one(&t.min).unwrap_or(0), |mx| one(&t.min).map_or(mx, |mn| mx.max(mn))).max(64)
    };
    let fixed: i64 = tl.tracks.iter().map(px).sum();
    let per: i64 = a.tracks.iter().map(px).sum();
    let t = a.tracks.len().max(1) as i64;
    let rhs = target - fixed - gap * (tl.tracks.len() as i64 - 1);
    let denom = (per + gap * t).max(1);
    let k = if up { (rhs + denom - 1).div_euclid(denom) } else { rhs.div_euclid(denom) };
    (k.max(1) as usize).min(MAXT.saturating_sub(tl.tracks.len()) / t as usize).max(1)
}

// ---------------------------------------------------------------------------------------------
// Sizing helpers

/// 11.7.1: the largest size of an `fr` that fits `space` for the tracks `tr` (gutters in `gaps`).
fn find_fr(tr: &[Tr], space: i64, gaps: i64) -> f64 {
    let mut flexible: Vec<bool> = tr.iter().map(|t| t.flexible() && !t.gone).collect();
    for _ in 0..=tr.len() {
        let fixed: i64 = tr.iter().zip(&flexible).filter(|(t, f)| !**f && !t.gone).map(|(t, _)| t.base).sum();
        let sum = tr.iter().zip(&flexible).filter(|(_, f)| **f).map(|(t, _)| f64::from(t.flex)).sum::<f64>().max(1.0);
        let hyp = (space - gaps - fixed) as f64 / sum;
        let mut bad = false;
        for (t, f) in tr.iter().zip(flexible.iter_mut()) {
            if *f && hyp * f64::from(t.flex) < t.base as f64 {
                *f = false;
                bad = true;
            }
        }
        if !bad {
            return hyp.max(0.0);
        }
    }
    0.0
}

/// A lower bound of the offset of track `upto`: the fixed minimum sizes of the tracks before it and the gutters.
fn fixed_prefix(tr: &[Tr], upto: usize, gap: i64) -> i64 {
    tr.iter().take(upto).filter(|t| !t.gone).map(|t| gap + if let Mn::Fixed(v) = t.min { v } else { 0 }).sum()
}

fn gaps_in(tr: &[Tr], gap: i64) -> i64 {
    gap * (tr.iter().filter(|t| !t.gone).count() as i64 - 1).max(0)
}

/// Spread `amount` over `cnt` shares as evenly as whole units allow: the share of the `k`-th.
fn share(amount: i64, cnt: usize, k: usize) -> i64 {
    let c = cnt.max(1) as i64;
    amount / c + i64::from((k as i64) < amount % c)
}

/// Distribute `amount` equally among the tracks `idx` of `tr`, each limited by `head(track, increase so far)`; the tracks
/// that reach their limit are frozen and the rest go on. Returns the increase of each track in `idx`.
fn spread(tr: &[Tr], idx: &[usize], amount: i64, head: &dyn Fn(&Tr, i64) -> i64) -> Vec<i64> {
    let mut inc = vec![0i64; idx.len()];
    let mut frozen = vec![false; idx.len()];
    let mut left = amount;
    for _ in 0..=idx.len() {
        let live: Vec<usize> = (0..idx.len()).filter(|&j| !frozen.get(j).copied().unwrap_or(true)).collect();
        if live.is_empty() || left <= 0 {
            break;
        }
        let mut used = 0;
        for (k, &j) in live.iter().enumerate() {
            let want = share(left, live.len(), k);
            let h = idx.get(j).and_then(|&i| tr.get(i)).map_or(0, |t| head(t, inc.get(j).copied().unwrap_or(0))).max(0);
            let add = want.min(h);
            if let Some(v) = inc.get_mut(j) {
                *v += add;
            }
            used += add;
            if add < want && let Some(f) = frozen.get_mut(j) {
                *f = true;
            }
        }
        left -= used;
        if used == 0 {
            break;
        }
    }
    inc
}

/// What size an item contributes: see 11.5.
#[derive(Clone, Copy, PartialEq)]
enum Kind {
    /// The minimum contribution (or the limited min-content one under a min- or max-content constraint).
    MinOrLimited,
    Minc,
    LimitedMax,
    Maxc,
}

/// The phases of 11.5 for items that span several tracks.
struct Phase {
    limits: bool,
    kind: Kind,
    aff: fn(&Tr, Mode) -> bool,
    beyond: fn(&Tr) -> bool,
    /// Only in this mode (`None`: always).
    only: Option<Mode>,
}

const PHASES: [Phase; 6] = [
    Phase { limits: false, kind: Kind::MinOrLimited, aff: |t, _| t.intrinsic_min(), beyond: |t| t.intrinsic_max(), only: None },
    Phase { limits: false, kind: Kind::Minc, aff: |t, _| matches!(t.min, Mn::Min | Mn::Max), beyond: |t| t.intrinsic_max(), only: None },
    Phase { limits: false, kind: Kind::LimitedMax, aff: |t, _| matches!(t.min, Mn::Auto | Mn::Max), beyond: |t| t.max == Mx::Max || t.max == Mx::Auto, only: Some(Mode::Max) },
    Phase { limits: false, kind: Kind::Maxc, aff: |t, _| t.min == Mn::Max, beyond: |t| t.max == Mx::Max || t.max == Mx::Auto, only: None },
    Phase { limits: true, kind: Kind::Minc, aff: |t, _| t.intrinsic_max(), beyond: |t| t.intrinsic_max(), only: None },
    Phase { limits: true, kind: Kind::Maxc, aff: |t, _| t.max == Mx::Max || t.max == Mx::Auto, beyond: |t| t.intrinsic_max(), only: None },
];

/// The padding and border, and the margins, along the inline axis (percentages count as 0, as for intrinsic sizes).
fn h_extras(bx: &LayoutBox) -> (LU, LU) {
    let (bs, bd) = (&bx.style.boxs, &bx.style.border);
    let p = |i: usize| bs.padding.get(i).map_or(LU::ZERO, |l| if lp_has_percent(l) { LU::ZERO } else { pad_px(l, LU::ZERO) });
    let b = |i: usize| border_px(bd.width.get(i).copied().unwrap_or(0.0));
    let m = |i: usize| match bs.margin.get(i) {
        Some(Dim::Lp(l)) if !lp_has_percent(l) => LU::from_f32(l.resolve(0.0)),
        _ => LU::ZERO,
    };
    (p(1) + p(3) + b(1) + b(3), m(1) + m(3))
}

/// The same along the block axis.
fn v_extras(bx: &LayoutBox, base: LU) -> (LU, LU) {
    let (bs, bd) = (&bx.style.boxs, &bx.style.border);
    let p = |i: usize| bs.padding.get(i).map_or(LU::ZERO, |l| pad_px(l, base));
    let b = |i: usize| border_px(bd.width.get(i).copied().unwrap_or(0.0));
    let m = |i: usize| bs.margin.get(i).and_then(|d| resolve_margin(d, base)).unwrap_or(LU::ZERO);
    (p(0) + p(2) + b(0) + b(2), m(0) + m(2))
}

fn fixed_px(d: &Dim) -> Option<LU> {
    match d {
        Dim::Lp(l) if !lp_has_percent(l) => Some(LU::from_f32(l.resolve(0.0))),
        _ => None,
    }
}

/// How the grid sees `a` (an `align-*` / `justify-*` value) for one item: where it sits in its area, and whether it is
/// stretched. `rtl`: the inline axis runs right to left (positions come out physical).
fn item_pos(a: AlignValue, replaced: bool, inline: bool, rtl: bool) -> (Pos, bool) {
    let flip = |p: Pos| match (p, inline && rtl) {
        (Pos::Start, true) => Pos::End,
        (Pos::End, true) => Pos::Start,
        (p, _) => p,
    };
    match a.kind {
        AlignKind::Normal | AlignKind::Auto => (Pos::Start, !replaced),
        AlignKind::Stretch => (Pos::Start, true),
        AlignKind::Center => (Pos::Mid, false),
        AlignKind::End | AlignKind::FlexEnd | AlignKind::SelfEnd | AlignKind::LastBaseline => (flip(Pos::End), false),
        AlignKind::Left if inline => (Pos::Start, false),
        AlignKind::Right if inline => (Pos::End, false),
        _ => (flip(Pos::Start), false),
    }
}

/// 10.5: the offset of the first track and the extra space between tracks for a content distribution value.
fn dist(free: i64, n: i64, av: AlignValue, inline: bool, rtl: bool) -> (i64, i64) {
    let safe = av.safe == Some(true) && free < 0;
    match av.kind {
        AlignKind::SpaceBetween if free > 0 && n > 1 => (0, div_round(free, n - 1)),
        AlignKind::SpaceAround if free > 0 && n > 0 => (div_round(free, 2 * n), div_round(free, n)),
        AlignKind::SpaceEvenly if free > 0 => (div_round(free, n + 1), div_round(free, n + 1)),
        AlignKind::Center => (if safe { 0 } else { free / 2 }, 0),
        AlignKind::End | AlignKind::FlexEnd => (if safe { 0 } else { free }, 0),
        AlignKind::Left if inline && rtl => (if safe { 0 } else { free }, 0),
        AlignKind::Right if inline && !rtl => (if safe { 0 } else { free }, 0),
        _ => (0, 0),
    }
}

impl Layouter<'_> {
    /// Lay out the grid container `id` (css-grid-1 11).
    pub(super) fn layout_grid(&mut self, id: BoxId, cb_w: LU, cb_h: Option<LU>, flow: Flow) -> BlockResult {
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
        let content_h = self.grid_inner(id, &g, (content_left, content_top));
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

    /// The explicit grid, the items and their placement, and the tracks (not sized yet). `cols` / `rows` say how much room
    /// each axis has (for `auto-fill`) and what percentages in track sizes refer to.
    fn build_gx(&mut self, id: BoxId, rows: Space, cols: Space) -> Grid {
        let tree = self.tree;
        let bx = tree.get(id);
        let (gs, fl) = (&bx.style.grid, &bx.style.flex);
        let gap_of = |o: &Option<Lp>, base: Option<i64>| -> i64 {
            o.as_ref().map_or(0, |l| match base {
                Some(b) => r(LU::from_f32(l.resolve(lu(b).to_f32()))).max(0),
                None if lp_has_percent(l) => 0,
                None => r(LU::from_f32(l.resolve(0.0))).max(0),
            })
        };
        let gap = Ax { r: gap_of(&fl.row_gap, rows.avail), c: gap_of(&fl.column_gap, cols.avail) };
        // Explicit tracks.
        let default_list = TrackList::default();
        let mut ex: Ax<usize> = Ax::default();
        let mut lists: Ax<Vec<Track>> = Ax::default();
        let mut names: Ax<Names> = Ax::default();
        let mut fit: Ax<Option<(usize, usize)>> = Ax::default();
        for a in 0..2 {
            let (tl, sp) = if a == 0 { (gs.rows.as_deref(), rows) } else { (gs.cols.as_deref(), cols) };
            let tl = tl.unwrap_or(&default_list);
            let reps = auto_reps(tl, gap.v(a), sp);
            let (tracks, nm, range) = expand(tl, reps);
            if let Some((s, len, true)) = range {
                *fit.m(a) = Some((s, len));
            }
            let areas = gs.areas.as_ref().map_or(0, |ar| if a == 0 { ar.rows } else { ar.cols } as usize);
            *ex.m(a) = tracks.len().max(areas).min(MAXT);
            *lists.m(a) = tracks;
            *names.m(a) = nm;
        }
        // The lines of the named areas (7.3): `name-start` and `name-end`.
        if let Some(ar) = &gs.areas {
            for (nm, r0, c0, r1, c1) in &ar.list {
                for (a, s, e) in [(0, r0, r1), (1, c0, c1)] {
                    let v = names.m(a);
                    for (line, suffix) in [(*s, "start"), (*e, "end")] {
                        let line = line as usize - 1;
                        if v.len() <= line {
                            v.resize(line + 1, Vec::new());
                        }
                        if let Some(l) = v.get_mut(line) {
                            l.push(Rc::from(format!("{nm}-{suffix}")));
                        }
                    }
                }
            }
        }
        // Items and their lines.
        let mut items: Vec<GItem> = Vec::new();
        for &c in &bx.children {
            let cb = tree.get(c);
            if cb.oof {
                continue;
            }
            let ln = &cb.style.grid.line;
            let (rdef, rst, rsp) = resolve_axis(names.g(0), ex.r as i64, &ln[0], &ln[2]);
            let (cdef, cst, csp) = resolve_axis(names.g(1), ex.c as i64, &ln[1], &ln[3]);
            items.push(GItem { id: c, st: Ax { r: rst, c: cst }, sp: Ax { r: rsp, c: csp }, def: Ax { r: rdef, c: cdef }, ok: true, cc: Ax::default() });
        }
        // Implicit tracks before the explicit grid, for lines with negative numbers.
        let mut neg: Ax<usize> = Ax::default();
        for a in 0..2 {
            let lo = items.iter().filter(|i| i.def.v(a)).map(|i| i.st.v(a)).min().unwrap_or(0);
            *neg.m(a) = (-lo).clamp(0, MAXT as i64) as usize;
            for it in items.iter_mut() {
                if it.def.v(a) {
                    let s = (it.st.v(a) + neg.v(a) as i64).clamp(0, MAXT as i64 - 1);
                    *it.st.m(a) = s;
                    *it.sp.m(a) = it.sp.v(a).min(MAXT - s as usize).max(1);
                }
            }
        }
        let mut n = Ax { r: (neg.r + ex.r).min(MAXT), c: (neg.c + ex.c).min(MAXT) };
        let left = self.budget.saturating_sub(self.work);
        let (work, out) = place(&mut items, &mut n, usize::from(gs.flow_col), gs.dense, left);
        self.charge(work);
        self.exhausted |= out;
        // The track sizing functions (7.6): explicit, then the auto ones, in both directions from the explicit grid.
        let mut tr: Ax<Vec<Track>> = Ax::default();
        for a in 0..2 {
            let auto = if a == 0 { &gs.auto_rows } else { &gs.auto_cols };
            let (nn, ng, list) = (n.v(a), neg.v(a), lists.g(a));
            let al = auto.len().max(1);
            let v: Vec<Track> = (0..nn)
                .map(|k| {
                    let t = if k < ng {
                        auto.get(al - 1 - ((ng - 1 - k) % al))
                    } else if let Some(t) = list.get(k - ng) {
                        Some(t)
                    } else {
                        auto.get((k - ng - list.len()) % al)
                    };
                    t.cloned().unwrap_or(Track::AUTO)
                })
                .collect();
            *tr.m(a) = v;
        }
        let pct = Ax { r: rows.avail, c: cols.avail };
        let mut g = Grid {
            items,
            tr: Ax { r: tr.r.iter().map(|t| mk_tr(t, pct.r)).collect(), c: tr.c.iter().map(|t| mk_tr(t, pct.c)).collect() },
            gap,
            ji: fl.justify_items,
            ai: fl.align_items,
            rtl: bx.style.text.direction == Direction::Rtl,
            fit: Ax { r: fit.r.map(|(s, len)| (s + neg.r, len)), c: fit.c.map(|(s, len)| (s + neg.c, len)) },
            names,
            ex,
            neg,
            stop: None,
        };
        // `auto-fit`: the repeated tracks nobody uses collapse (7.2.3.2).
        for a in 0..2 {
            let Some((s, len)) = g.fit.v(a) else { continue };
            let used: Vec<bool> = (s..s + len).map(|t| g.items.iter().any(|i| i.ok && i.st.v(a) <= t as i64 && (t as i64) < i.st.v(a) + i.sp.v(a) as i64)).collect();
            for (k, u) in used.iter().enumerate() {
                if !u && let Some(t) = g.tr.m(a).get_mut(s + k) {
                    *t = Tr { min: Mn::Fixed(0), max: Mx::Fixed(0), base: 0, limit: 0, gone: true, ..*t };
                }
            }
        }
        g
    }

    /// (minimum, min-content, max-content) contribution of item `i` along axis `ax` (11.5), outer sizes.
    fn contribs(&mut self, g: &mut Grid, i: usize, ax: usize) -> (i64, i64, i64) {
        let Some(it) = g.items.get(i) else { return (0, 0, 0) };
        if let Some(c) = it.cc.v(ax) {
            return c;
        }
        let (id, st, sp) = (it.id, it.st.c as usize, it.sp.c);
        let (s0, sp0) = (it.st.v(ax) as usize, it.sp.v(ax));
        let tree = self.tree;
        let bx = tree.get(id);
        let bs = &bx.style.boxs;
        // 6.6: the content-based minimum size applies when the item spans a track with an `auto` minimum and, if it spans
        // several, none of them flexible; it is then capped by the fixed maximum sizes of the tracks when all of them have one.
        let span_tr = g.tr.g(ax).get(s0..s0 + sp0).unwrap_or(&[]);
        let live: Vec<Tr> = span_tr.iter().filter(|t| !t.gone).copied().collect();
        let auto_min = live.iter().any(|t| t.min == Mn::Auto) && (live.len() <= 1 || !live.iter().any(Tr::flexible));
        let fixed_cap: i64 = live.iter().map(|t| if let Mx::Fixed(v) = t.max { Some(v) } else { None }).sum::<Option<i64>>().map_or(INF, |s| s + gaps_in(span_tr, g.gap.v(ax)));
        let c = if ax == 1 {
            let (mn, mx) = self.outer_widths(id);
            let (ext, marg) = h_extras(bx);
            let to_cw = |v: LU| if bs.box_sizing == BoxSizing::BorderBox { (v - ext).max0() } else { v };
            let min_c = if fixed_px(&bs.width).is_some() {
                mn
            } else if let Some(m) = fixed_px(&bs.min_width) {
                to_cw(m) + ext + marg
            } else if !matches!(bs.overflow_x, Overflow::Visible | Overflow::Clip) || !auto_min {
                ext + marg
            } else {
                lu(r(mn).min(fixed_cap))
            };
            (r(min_c), r(mn), r(mx))
        } else {
            // The height at the width of the area (known: the columns are sized first).
            let tc = g.tr.g(1);
            let area = tc.get(st..st + sp).unwrap_or(&[]);
            let aw = lu(area.iter().filter(|t| !t.gone).map(|t| t.base).sum::<i64>() + gaps_in(area, g.gap.c));
            let (ext, marg) = v_extras(bx, aw);
            let box_h = |v: LU| if bs.box_sizing == BoxSizing::BorderBox { v } else { v + ext };
            let clamp = |h: LU| {
                let h = fixed_px(&bs.max_height).map_or(h, |m| h.min(box_h(m)));
                fixed_px(&bs.min_height).map_or(h, |m| h.max(box_h(m)))
            };
            let spec = fixed_px(&bs.height).map(box_h);
            // First-screen-first: an item that starts below the stop line for sure is not measured at all.
            let est = g.stop.map(|top| lu(top + fixed_prefix(g.tr.g(0), g.items.get(i).map_or(0, |it| it.st.r as usize), g.gap.r)));
            if est.is_some_and(|e| self.below_stop(e)) {
                return (0, 0, 0);
            }
            let h = match spec {
                Some(h) => h,
                None if bx.kind == BoxKind::Replaced => self.replaced_size(id, aw, None).1 + ext,
                None => {
                    let w = self.item_width(g, i, aw);
                    self.flex_measure(id, aw, w, est)
                }
            };
            let outer = clamp(h) + marg;
            let min_c = if spec.is_some() {
                outer
            } else if let Some(m) = fixed_px(&bs.min_height) {
                box_h(m) + marg
            } else if !matches!(bs.overflow_y, Overflow::Visible | Overflow::Clip) || !auto_min {
                ext + marg
            } else {
                lu(r(outer).min(fixed_cap))
            };
            (r(min_c), r(outer), r(outer))
        };
        if let Some(it) = g.items.get_mut(i) {
            *it.cc.m(ax) = Some(c);
        }
        c
    }

    /// The content width item `i` gets in an area `aw` wide (10.3: stretched when `justify-self` says so, else fit-content).
    fn item_width(&mut self, g: &Grid, i: usize, aw: LU) -> LU {
        let Some(it) = g.items.get(i) else { return LU::ZERO };
        let id = it.id;
        let tree = self.tree;
        let bx = tree.get(id);
        if bx.kind == BoxKind::Replaced {
            return self.replaced_size(id, aw, None).0;
        }
        let (bs, st) = (&bx.style.boxs, &bx.style);
        let js = if st.flex.justify_self.kind == AlignKind::Auto { g.ji } else { st.flex.justify_self };
        let (_, stretch) = item_pos(js, false, true, g.rtl);
        let auto_margin = bs.margin.get(1).is_none_or(|d| matches!(d, Dim::Auto)) || bs.margin.get(3).is_none_or(|d| matches!(d, Dim::Auto));
        // Percentages in the padding refer to the width of the area.
        let ext = {
            let (p, b) = (|k: usize| bs.padding.get(k).map_or(LU::ZERO, |l| pad_px(l, aw)), |k: usize| border_px(st.border.width.get(k).copied().unwrap_or(0.0)));
            p(1) + p(3) + b(1) + b(3)
        };
        let mrg = |k: usize| bs.margin.get(k).and_then(|d| resolve_margin(d, aw)).unwrap_or(LU::ZERO);
        let avail = (aw - mrg(1) - mrg(3) - ext).max0();
        if !(matches!(bs.width, Dim::Auto) && stretch && !auto_margin) {
            return self.atomic_width(id, aw);
        }
        let to_cw = |v: LU| if bs.box_sizing == BoxSizing::BorderBox { (v - ext).max0() } else { v };
        let mx = self.kw_size(id, &bs.max_width, avail).or_else(|| resolve_dim(&bs.max_width, Some(aw)).map(to_cw));
        let mn = self.kw_size(id, &bs.min_width, avail).unwrap_or_else(|| resolve_dim(&bs.min_width, Some(aw)).map_or(LU::ZERO, to_cw));
        mx.map_or(avail, |m| avail.min(m)).max(mn)
    }

    /// 11.5.1: distribute what the items of `group` need beyond the tracks they span.
    fn phase(&mut self, g: &mut Grid, ax: usize, group: &[usize], ph: &Phase, mode: Mode, flex_only: bool) {
        let gap = g.gap.v(ax);
        let mut plan: Vec<i64> = vec![0; g.tr.g(ax).len()];
        for &i in group {
            let Some(it) = g.items.get(i) else { continue };
            let (a, b) = (it.st.v(ax) as usize, it.st.v(ax) as usize + it.sp.v(ax));
            let tr = g.tr.g(ax);
            if tr.get(a..b).is_none() {
                continue;
            }
            let idx: Vec<usize> = (a..b).filter(|&k| tr.get(k).is_some_and(|t| !t.gone)).collect();
            let aff: Vec<usize> = idx.iter().copied().filter(|&k| tr.get(k).is_some_and(|t| (ph.aff)(t, mode) && (!flex_only || t.flexible()))).collect();
            if aff.is_empty() {
                continue;
            }
            let (mn_c, minc, maxc) = self.contribs(g, i, ax);
            let tr = g.tr.g(ax);
            let range = tr.get(a..b).unwrap_or(&[]);
            let limited = |c: i64| -> i64 {
                // The upper limit is the sum of the fixed max functions, when every spanned track has one.
                let fixed: Option<i64> = range.iter().filter(|t| !t.gone).map(|t| match t.max {
                    Mx::Fixed(v) => Some(v),
                    _ if t.fit < INF => Some(t.fit),
                    _ => None,
                }).sum();
                fixed.map_or(c, |f| c.min(f + gaps_in(range, gap))).max(mn_c)
            };
            let contribution = match ph.kind {
                Kind::MinOrLimited if mode == Mode::Def => mn_c,
                Kind::MinOrLimited => limited(minc),
                Kind::Minc => minc,
                Kind::LimitedMax => limited(maxc),
                Kind::Maxc => maxc,
            };
            let size = |t: &Tr| if ph.limits && t.limit < INF { t.limit } else { t.base };
            let spanned: i64 = idx.iter().filter_map(|&k| tr.get(k)).map(size).sum::<i64>() + gaps_in(range, gap);
            let space = (contribution - spanned).max(0);
            if space == 0 {
                continue;
            }
            // The limit of a track while growing: see 11.5.1 "Distribute space up to limits".
            let lim = |t: &Tr| -> i64 {
                if ph.limits {
                    if t.limit < INF && !t.grow { t.limit } else if t.fit < INF { t.fit } else { INF }
                } else {
                    t.limit.min(t.fit)
                }
            };
            let head = |t: &Tr, inc: i64| (lim(t) - size(t) - inc).max(0);
            let mut got: Vec<(usize, i64)> = Vec::new();
            if flex_only {
                let sum: f64 = aff.iter().filter_map(|&k| tr.get(k)).map(|t| f64::from(t.flex)).sum();
                let part = sum.min(1.0);
                for (n, &k) in aff.iter().enumerate() {
                    let f = tr.get(k).map_or(0.0, |t| f64::from(t.flex));
                    let by_ratio = if sum > 0.0 { space as f64 * part * f / sum } else { 0.0 };
                    got.push((k, by_ratio as i64 + share((space as f64 * (1.0 - part)) as i64, aff.len(), n)));
                }
            } else {
                let inc = spread(tr, &aff, space, &head);
                let mut left = space - inc.iter().sum::<i64>();
                got.extend(aff.iter().copied().zip(inc));
                let others: Vec<usize> = idx.iter().copied().filter(|k| !aff.contains(k)).collect();
                if left > 0 && !others.is_empty() {
                    let inc = spread(tr, &others, left, &head);
                    left -= inc.iter().sum::<i64>();
                    got.extend(others.iter().copied().zip(inc));
                }
                if left > 0 {
                    // Beyond the limits: to the affected tracks that fit the phase best, else all affected tracks.
                    let pick: Vec<usize> = aff.iter().copied().filter(|&k| tr.get(k).is_some_and(|t| (ph.beyond)(t))).collect();
                    let pick = if pick.is_empty() && !ph.limits { aff.clone() } else { pick };
                    for (n, &k) in pick.iter().enumerate() {
                        let add = share(left, pick.len(), n);
                        match got.iter_mut().find(|(j, _)| *j == k) {
                            Some(e) => e.1 += add,
                            None => got.push((k, add)),
                        }
                    }
                }
            }
            for (k, v) in got {
                if let Some(p) = plan.get_mut(k) {
                    *p = (*p).max(v);
                }
            }
        }
        for (t, p) in g.tr.m(ax).iter_mut().zip(plan) {
            if p > 0 {
                if ph.limits {
                    if t.limit >= INF {
                        t.limit = t.base + p;
                        t.grow = true;
                    } else {
                        t.limit += p;
                    }
                } else {
                    t.base += p;
                }
            }
        }
    }

    /// 11.5: resolve the intrinsic track sizes.
    fn resolve_intrinsic(&mut self, g: &mut Grid, ax: usize, mode: Mode) {
        let nt = g.tr.g(ax).len();
        let span_of = |g: &Grid, i: usize| g.items.get(i).map_or(1, |it| it.sp.v(ax));
        let mut order: Vec<usize> = (0..g.items.len()).filter(|&i| g.items.get(i).is_some_and(|it| it.ok)).collect();
        crate::sorting::sort_by_u64_key(&mut order, |&i| span_of(g, i) as u64);
        let range_of = |g: &Grid, i: usize| g.items.get(i).map_or((0, 0), |it| (it.st.v(ax) as usize, (it.st.v(ax) as usize + it.sp.v(ax)).min(nt)));
        let has_flex = |g: &Grid, i: usize| {
            let (a, b) = range_of(g, i);
            g.tr.g(ax).get(a..b).is_some_and(|s| s.iter().any(|t| t.flexible() && !t.gone))
        };
        // Items spanning one track.
        let singles: Vec<usize> = order.iter().copied().filter(|&i| span_of(g, i) == 1).collect();
        for i in singles {
            let (a, _) = range_of(g, i);
            let Some(t) = g.tr.g(ax).get(a).copied() else { continue };
            if t.gone || t.flexible() || !(t.intrinsic_min() || t.intrinsic_max()) {
                continue;
            }
            let (mn_c, minc, maxc) = self.contribs(g, i, ax);
            let Some(t) = g.tr.m(ax).get_mut(a) else { continue };
            let limit_fixed = match t.max {
                Mx::Fixed(v) => v,
                _ => t.fit,
            };
            match t.min {
                Mn::Min => t.base = t.base.max(minc),
                Mn::Max => t.base = t.base.max(maxc),
                Mn::Auto => t.base = t.base.max(if mode == Mode::Def { mn_c } else { minc.min(limit_fixed).max(mn_c) }),
                Mn::Fixed(_) => {}
            }
            let c = match t.max {
                Mx::Min => Some(minc),
                Mx::Max | Mx::Auto => Some(maxc.min(t.fit)),
                _ => None,
            };
            if let Some(c) = c {
                t.limit = if t.limit >= INF { c } else { t.limit.max(c) };
            }
        }
        for t in g.tr.m(ax).iter_mut() {
            if t.limit < INF && t.limit < t.base {
                t.limit = t.base;
            }
        }
        // Items spanning several tracks, smallest span first; those that cross a flexible track come last.
        // Looking at the tracks of every spanning item costs its span; each phase of the algorithm costs about that again, so
        // a page of items that span a thousand tracks each runs out of budget instead of taking seconds.
        let wide: u64 = order.iter().map(|&i| span_of(g, i) as u64).filter(|&s| s > 1).sum();
        self.charge(wide / 8);
        let flexy: Vec<bool> = if self.exhausted { Vec::new() } else { (0..g.items.len()).map(|i| span_of(g, i) > 1 && has_flex(g, i)).collect() };
        let flexy = |i: usize| flexy.get(i).copied().unwrap_or(true);
        let mut runs: Vec<(usize, Vec<usize>)> = Vec::new();
        for &i in order.iter().filter(|&&i| span_of(g, i) > 1 && !flexy(i)) {
            let s = span_of(g, i);
            match runs.last_mut() {
                Some((rs, group)) if *rs == s => group.push(i),
                _ => runs.push((s, vec![i])),
            }
        }
        for (s, group) in runs {
            if !self.charge(group.len() as u64 * (2 + 4 * s as u64)) {
                break;
            }
            for (n, ph) in PHASES.iter().enumerate() {
                if ph.only.is_some_and(|m| m != mode) {
                    continue;
                }
                self.phase(g, ax, &group, ph, mode, false);
                if n == 3 {
                    for t in g.tr.m(ax).iter_mut() {
                        if t.limit < t.base {
                            t.limit = t.base;
                        }
                    }
                }
            }
            for t in g.tr.m(ax).iter_mut() {
                t.grow = false;
            }
        }
        let flex_group: Vec<usize> = if self.exhausted { Vec::new() } else { order.iter().copied().filter(|&i| has_flex(g, i)).collect() };
        let flex_cost: u64 = flex_group.iter().map(|&i| 2 + 4 * span_of(g, i) as u64).sum();
        if !flex_group.is_empty() && self.charge(flex_cost) {
            for ph in PHASES.iter().take(4) {
                if ph.only.is_none_or(|m| m == mode) {
                    self.phase(g, ax, &flex_group, ph, mode, true);
                }
            }
        }
        for t in g.tr.m(ax).iter_mut() {
            if t.limit >= INF || t.limit < t.base {
                t.limit = t.base;
            }
            t.grow = false;
        }
    }

    /// 11.3: size the tracks of axis `ax`. `avail`: the definite size to fill; `mode` how the container is sized; `lim`: the
    /// container's own min and max size along the axis; `stretch`: `auto` tracks may grow into free space (11.8).
    fn size_axis(&mut self, g: &mut Grid, ax: usize, avail: Option<i64>, mode: Mode, lim: (i64, Option<i64>), stretch: bool) {
        let gap = g.gap.v(ax);
        self.resolve_intrinsic(g, ax, mode);
        let total = |g: &Grid| g.tr.g(ax).iter().filter(|t| !t.gone).map(|t| t.base).sum::<i64>() + gaps_in(g.tr.g(ax), gap);
        // 11.6 Maximize tracks.
        match mode {
            Mode::Def => {
                let free = avail.map_or(0, |a| a - total(g));
                if free > 0 {
                    let tr = g.tr.g(ax);
                    let idx: Vec<usize> = (0..tr.len()).filter(|&k| tr.get(k).is_some_and(|t| !t.gone)).collect();
                    let inc = spread(tr, &idx, free, &|t, i| t.limit - t.base - i);
                    for (&k, v) in idx.iter().zip(inc) {
                        if let Some(t) = g.tr.m(ax).get_mut(k) {
                            t.base += v;
                        }
                    }
                }
            }
            Mode::Max => {
                for t in g.tr.m(ax).iter_mut() {
                    if t.limit < INF {
                        t.base = t.base.max(t.limit);
                    }
                }
            }
            Mode::Min => {}
        }
        // 11.7 Expand flexible tracks.
        if g.tr.g(ax).iter().any(|t| t.flexible() && !t.gone) {
            let fr = match (mode, avail) {
                (Mode::Min, _) => 0.0,
                (Mode::Def, Some(a)) => find_fr(g.tr.g(ax), a, gaps_in(g.tr.g(ax), gap)),
                _ => {
                    let mut fr = g.tr.g(ax).iter().filter(|t| t.flexible() && !t.gone).map(|t| if t.flex > 1.0 { t.base as f64 / f64::from(t.flex) } else { t.base as f64 }).fold(0.0, f64::max);
                    for i in 0..g.items.len() {
                        let Some(it) = g.items.get(i) else { continue };
                        let (a, b) = (it.st.v(ax) as usize, it.st.v(ax) as usize + it.sp.v(ax));
                        if !it.ok || !g.tr.g(ax).get(a..b).is_some_and(|s| s.iter().any(|t| t.flexible() && !t.gone)) {
                            continue;
                        }
                        let (_, _, maxc) = self.contribs(g, i, ax);
                        let tr = g.tr.g(ax);
                        let sl = tr.get(a..b).unwrap_or(&[]);
                        fr = fr.max(find_fr(sl, maxc, gaps_in(sl, gap)));
                    }
                    // Smaller than the container's min size, or bigger than its max: size to those instead.
                    let sum = |fr: f64| g.tr.g(ax).iter().filter(|t| !t.gone).map(|t| if t.flexible() { t.base.max((fr * f64::from(t.flex)) as i64) } else { t.base }).sum::<i64>() + gaps_in(g.tr.g(ax), gap);
                    let t = sum(fr);
                    if t < lim.0 {
                        fr = find_fr(g.tr.g(ax), lim.0, gaps_in(g.tr.g(ax), gap));
                    } else if let Some(m) = lim.1 && t > m {
                        fr = find_fr(g.tr.g(ax), m, gaps_in(g.tr.g(ax), gap));
                    }
                    fr
                }
            };
            // The flexible tracks take whole layout units; what is left over of the exact sizes goes, a unit each, to the tracks
            // with the largest fractions, so that nothing is lost to rounding (the browsers' fr tracks add up to the space).
            let mut grown: Vec<(usize, f64)> = Vec::new();
            for (k, t) in g.tr.g(ax).iter().enumerate() {
                let want = fr * f64::from(t.flex);
                if t.flexible() && !t.gone && want > t.base as f64 {
                    grown.push((k, want));
                }
            }
            let floors: i64 = grown.iter().map(|(_, w)| w.floor() as i64).sum();
            let mut rest = (grown.iter().map(|(_, w)| *w).sum::<f64>().round() as i64 - floors).max(0);
            crate::sorting::sort_by_u64_key(&mut grown, |a| !crate::sorting::f64_key(a.1 - a.1.floor()));
            for (k, want) in grown {
                if let Some(t) = g.tr.m(ax).get_mut(k) {
                    t.base = want.floor() as i64 + i64::from(rest > 0);
                    rest -= 1;
                    t.limit = t.limit.max(t.base);
                }
            }
        }
        // 11.8 Stretch `auto` tracks.
        if stretch {
            let room = avail.unwrap_or_else(|| if mode == Mode::Def { 0 } else { lim.0 });
            let free = room - total(g);
            let idx: Vec<usize> = g.tr.g(ax).iter().enumerate().filter(|(_, t)| t.max == Mx::Auto && !t.gone).map(|(k, _)| k).collect();
            if free > 0 && !idx.is_empty() {
                for (n, &k) in idx.iter().enumerate() {
                    if let Some(t) = g.tr.m(ax).get_mut(k) {
                        t.base += share(free, idx.len(), n);
                    }
                }
            }
        }
        self.charge(g.tr.g(ax).len() as u64);
    }

    /// 9: the containing block of an out-of-flow child of a positioned grid container: the grid area its lines name, an `auto`
    /// edge or a line outside the grid being the padding edge. (x, y, w, h) in the container's border box.
    fn oof_area(g: &Grid, line: &[GridLine; 4], origin: (LU, LU), cw: i64, inner_h: i64, geo: &Geom) -> ([LU; 4], [LU; 4]) {
        let edge = |v: &[LU; 4], a: usize| v.get(a).copied().unwrap_or(LU::ZERO);
        let both = |a: usize| edge(&geo.bor, a) + edge(&geo.pad, a);
        let sum = |a: usize, b: usize| r(both(a)) + r(both(b));
        let bw = cw + sum(1, 3);
        let mut res: Ax<(i64, i64)> = Ax::default();
        let mut stat: Ax<(i64, i64)> = Ax::default();
        for a in 0..2 {
            let (ls, le) = if a == 0 { (line.first(), line.get(2)) } else { (line.get(1), line.get(3)) };
            let (Some(ls), Some(le)) = (ls, le) else { continue };
            let (s, e, _) = edges(g.names.g(a), g.ex.v(a) as i64, ls, le);
            // The padding edge and the content edge on the starting side (logical: columns count from the right in rtl).
            let side = if a == 0 { 0 } else if g.rtl { 1 } else { 3 };
            let (p0, c0) = (r(edge(&geo.bor, side)), r(both(side)));
            let pb = if a == 0 { inner_h + sum(0, 2) - r(geo.bor[0] + geo.bor[2]) } else { cw + sum(1, 3) - r(geo.bor[1] + geo.bor[3]) };
            let tr = g.tr.g(a);
            let k = |l: Option<i64>| l.map(|l| l + g.neg.v(a) as i64).filter(|k| *k >= 0 && *k as usize <= tr.len()).map(|k| k as usize);
            let start = k(s).map(|k| tr.get(k).map_or_else(|| tr.last().map_or(0, |t| t.pos + t.base), |t| t.pos));
            let end = k(e).map(|k| if k == 0 { tr.first().map_or(0, |t| t.pos) } else { tr.get(k - 1).map_or(0, |t| t.pos + t.base) });
            let a0 = start.map_or(p0, |v| c0 + v);
            let a1 = end.map_or(p0 + pb, |v| c0 + v);
            *res.m(a) = (a0, (a1 - a0).max(0));
            // With no line named on this axis the static position is that of the content box.
            *stat.m(a) = if a0 == p0 && a1 == p0 + pb && s.is_none() && e.is_none() { (c0, if a == 0 { inner_h } else { cw }) } else { (a0, (a1 - a0).max(0)) };
        }
        let mirror = |(mut x, w): (i64, i64)| {
            if g.rtl {
                x = bw - x - w;
            }
            (x, w)
        };
        let ((y, h), (x, w)) = (res.r, mirror(res.c));
        let ((sy, sh), (sx, sw)) = (stat.r, mirror(stat.c));
        let _ = origin;
        ([lu(x), lu(y), lu(w), lu(h)], [lu(sx), lu(sy), lu(sw), lu(sh)])
    }

    /// Set the position of every track of axis `ax` in a content box `inner` long; `av` is the content distribution.
    fn position_tracks(g: &mut Grid, ax: usize, inner: i64, av: AlignValue) -> i64 {
        let gap = g.gap.v(ax);
        let cnt = g.tr.g(ax).iter().filter(|t| !t.gone).count() as i64;
        let total = g.tr.g(ax).iter().filter(|t| !t.gone).map(|t| t.base).sum::<i64>() + gaps_in(g.tr.g(ax), gap);
        let (lead, extra) = dist(inner - total, cnt, av, ax == 1, g.rtl);
        let mut at = lead;
        for t in g.tr.m(ax).iter_mut() {
            t.pos = at;
            if !t.gone {
                at += t.base + gap + extra;
            }
        }
        total
    }

    fn grid_inner(&mut self, id: BoxId, geo: &Geom, origin: (LU, LU)) -> LU {
        let tree = self.tree;
        let bx = tree.get(id);
        let st = bx.style.clone();
        let cw = r(geo.content_w);
        let rows_sp = Space { avail: geo.spec_h.map(r), min: r(geo.min_h), max: geo.max_h.map(r) };
        let cols_sp = Space { avail: Some(cw), min: 0, max: None };
        let mut g = self.build_gx(id, rows_sp, cols_sp);
        // First-screen-first (L1 task 17): the rows below the stop line need not be sized or laid out, when nothing above the
        // line depends on them: the height is not definite (so the rows are not stretched), at most one row is flexible
        // (its size would depend on the cut items otherwise), `align-content` and the items' `align-self` start at the top
        // of their row or stretch (a row cut short must not move them), and the container's minimum height is within reach.
        let top = r(self.origin_y) + r(origin.1);
        let near = |d: i64| d >= 0;
        let start_like = |a: AlignValue| matches!(a.kind, AlignKind::Normal | AlignKind::Auto | AlignKind::Stretch | AlignKind::Start | AlignKind::FlexStart | AlignKind::SelfStart | AlignKind::Baseline);
        let mut items_ok = true;
        for i in 0..g.items.len() {
            let Some(id_i) = g.items.get(i).map(|it| it.id) else { continue };
            let sty = &tree.get(id_i).style;
            let a = if sty.flex.align_self.kind == AlignKind::Auto { g.ai } else { sty.flex.align_self };
            if !start_like(a) {
                items_ok = false;
                break;
            }
        }
        let cut_ok = self.no_stop == 0
            && rows_sp.avail.is_none()
            && g.tr.r.iter().filter(|t| t.flexible()).count() <= 1
            && start_like(st.flex.align_content)
            && items_ok
            && self.stop_at.is_some_and(|s| near(r(s) - top - rows_sp.min));
        if cut_ok {
            g.stop = Some(top);
        } else {
            self.no_stop += 1;
        }
        let stretch = |a: AlignValue| matches!(a.kind, AlignKind::Normal | AlignKind::Stretch);
        // Columns, then rows (which need the widths of the items).
        self.size_axis(&mut g, 1, Some(cw), Mode::Def, (0, None), stretch(st.flex.justify_content));
        let (mode, lim) = if rows_sp.avail.is_some() { (Mode::Def, (0, None)) } else { (Mode::Max, (rows_sp.min, rows_sp.max)) };
        self.size_axis(&mut g, 0, rows_sp.avail, mode, lim, stretch(st.flex.align_content));
        let sum_rows = g.tr.r.iter().filter(|t| !t.gone).map(|t| t.base).sum::<i64>() + gaps_in(&g.tr.r, g.gap.r);
        let inner_h = rows_sp.avail.unwrap_or_else(|| sum_rows.max(rows_sp.min).min(rows_sp.max.unwrap_or(INF)));
        Self::position_tracks(&mut g, 1, cw, st.flex.justify_content);
        Self::position_tracks(&mut g, 0, inner_h, st.flex.align_content);
        let hit = self.lay_grid_items(&mut g, origin, cw);

        // 9: out-of-flow children. Their static position is that of a lone item in the content box (alignment by
        // `justify-self` / `align-self` of the child, else the container's items); the grid-area containing block of a
        // positioned container is `grid_oof`.
        let (cl, ct) = origin;
        for &c in &bx.children {
            let cb = tree.get(c);
            if !cb.oof {
                continue;
            }
            let fixed = cb.style.boxs.position == Position::Fixed;
            let pcb = if fixed { None } else { self.cb_stack.last().copied() };
            if hit && pcb == Some(id) {
                // The container was cut: its positioned children wait for the full layout.
                continue;
            }
            let ps = |a: AlignValue, own: AlignValue, inline: bool| -> u8 {
                let a = if own.kind == AlignKind::Auto { a } else { own };
                match item_pos(a, false, inline, g.rtl).0 {
                    Pos::Start => 0,
                    Pos::Mid => 1,
                    Pos::End => 2,
                }
            };
            let ax = ps(g.ji, cb.style.flex.justify_self, true);
            let ay = ps(g.ai, cb.style.flex.align_self, false);
            let mut p = Pending { id: c, owner: id, sx: cl, sy: ct, cb: pcb, align: Some(StaticAlign { rw: lu(cw), rh: lu(inner_h), ax, ay }) };
            // 9.1: the grid area is the containing block of an out-of-flow child when the container is its containing block,
            // and (the browsers, and the tests of this part of the spec) the rectangle its static position is aligned in,
            // on the axes where a grid line is named; with `auto` lines the static position rectangle is the content box (9.2).
            match pcb {
                Some(cbid) if cbid == id => {
                    let (rect, stat) = Self::oof_area(&g, &cb.style.grid.line, (cl, ct), cw, inner_h, geo);
                    p.sx = stat[0];
                    p.sy = stat[1];
                    p.align = Some(StaticAlign { rw: stat[2], rh: stat[3], ax, ay });
                    self.layout_oof_in_rect(p, rect);
                }
                _ => self.pending.push(p),
            }
        }
        // 8.5 / 6: the first and last baseline of the container come from its first and last row.
        // (`min_by_key` keeps the first of equal items and `max_by_key` the last, like the ends of a stable sort.)
        let pos = |i: &&GItem| (i.st.r, i.st.c);
        if let (Some(first), Some(last)) = (g.items.iter().filter(|i| i.ok).min_by_key(pos), g.items.iter().filter(|i| i.ok).max_by_key(pos)) {
            let fb = self.first_baseline(first.id).zip(self.out.get(first.id as usize)).map(|(b, o)| b + o.y);
            let lb = self.last_baseline(last.id).zip(self.out.get(last.id as usize)).map(|(b, o)| b + o.y);
            self.flex_base.insert(id, (fb.unwrap_or(LU::MIN), lb.unwrap_or(LU::MIN)));
        }
        if cut_ok {
            if hit {
                self.cut.push(id);
                self.stopped = true;
            }
        } else {
            self.no_stop -= 1;
        }
        lu(sum_rows)
    }

    /// Lay the items out in their areas (10.3-10.6) and position them. Returns whether the stop line cut anything off.
    fn lay_grid_items(&mut self, g: &mut Grid, origin: (LU, LU), cw: i64) -> bool {
        let tree = self.tree;
        let (cl, ct) = origin;
        let mut hit = false;
        for i in 0..g.items.len() {
            if self.exhausted {
                break;
            }
            let Some(it) = g.items.get(i).cloned() else { continue };
            if !it.ok {
                continue;
            }
            let bx = tree.get(it.id);
            let (bs, sty) = (&bx.style.boxs, &bx.style);
            let edge = |g: &Grid, a: usize| -> (i64, i64) {
                let (s, e) = (it.st.v(a) as usize, (it.st.v(a) as usize + it.sp.v(a)).saturating_sub(1));
                let tr = g.tr.g(a);
                match (tr.get(s), tr.get(e)) {
                    (Some(x), Some(y)) => (x.pos, y.pos + y.base),
                    _ => (0, 0),
                }
            };
            let (x0, x1) = edge(g, 1);
            let (y0, y1) = edge(g, 0);
            let (aw, ah) = (lu(x1 - x0), lu(y1 - y0));
            let ax_phys = if g.rtl { cw - x1 } else { x0 };
            // Margins (percentages refer to the width of the area).
            let (mt, mr, mb, ml) = (resolve_margin(&bs.margin[0], aw), resolve_margin(&bs.margin[1], aw), resolve_margin(&bs.margin[2], aw), resolve_margin(&bs.margin[3], aw));
            let replaced = bx.kind == BoxKind::Replaced;
            let js = if sty.flex.justify_self.kind == AlignKind::Auto { g.ji } else { sty.flex.justify_self };
            let al = if sty.flex.align_self.kind == AlignKind::Auto { g.ai } else { sty.flex.align_self };
            let (jp, _) = item_pos(js, replaced, true, g.rtl);
            let (ap, astretch) = item_pos(al, replaced, false, g.rtl);
            let w = self.item_width(g, i, aw);
            let (v_ext, _) = v_extras(bx, aw);
            let auto_v = mt.is_none() || mb.is_none();
            let h_auto = matches!(bs.height, Dim::Auto) && !replaced;
            let forced_h = (astretch && !auto_v && h_auto).then(|| {
                // A stretched item still obeys its `min-height` and `max-height` (a table does not clamp a forced height itself).
                let to_ch = |m: LU| if bs.box_sizing == BoxSizing::BorderBox { (m - v_ext).max0() } else { m };
                let v = (ah - mt.unwrap_or(LU::ZERO) - mb.unwrap_or(LU::ZERO) - v_ext).max0();
                let v = fixed_px(&bs.max_height).map_or(v, |m| v.min(to_ch(m)));
                fixed_px(&bs.min_height).map_or(v, |m| v.max(to_ch(m)))
            });
            let flow = Flow { forced_w: Some(w), forced_h, ..Flow::default() };
            let res = match g.stop {
                Some(top) => {
                    let est = lu(top + y0 + mt.map_or(0, r));
                    if self.below_stop(est) {
                        hit = true;
                        continue;
                    }
                    let (res, cut) = self.item_stop(est, |s| s.layout_block(it.id, aw, Some(ah), flow));
                    hit |= cut;
                    res
                }
                None => self.layout_block(it.id, aw, Some(ah), flow),
            };
            let o = self.out.get(it.id as usize).copied().unwrap_or_default();
            let (ow, oh) = (r(o.w), r(o.h));
            let safe_free = |free: i64, a: AlignValue| if a.safe == Some(true) && free < 0 { 0 } else { free };
            // Inline axis: auto margins take the free space, else `justify-self` places the margin box.
            let (aw_i, ah_i) = (r(aw), r(ah));
            let free_x = aw_i - ow - mr.map_or(0, r) - ml.map_or(0, r);
            let (mlu, off_x) = match (ml, mr) {
                (None, None) if free_x > 0 => (free_x / 2, 0),
                (None, Some(_)) if free_x > 0 => (free_x, 0),
                (Some(m), None) => (r(m), 0),
                (m, _) => {
                    let m = m.map_or(0, r);
                    (m, match jp { Pos::Start => 0, Pos::Mid => safe_free(free_x, js) / 2, Pos::End => safe_free(free_x, js) })
                }
            };
            let free_y = ah_i - oh - mt.map_or(0, r) - mb.map_or(0, r);
            let (mtu, off_y) = match (mt, mb) {
                (None, None) if free_y > 0 => (free_y / 2, 0),
                (None, Some(_)) if free_y > 0 => (free_y, 0),
                (Some(m), None) => (r(m), 0),
                (m, _) => {
                    let m = m.map_or(0, r);
                    (m, match ap { Pos::Start => 0, Pos::Mid => safe_free(free_y, al) / 2, Pos::End => safe_free(free_y, al) })
                }
            };
            if let Some(oo) = self.out.get_mut(it.id as usize) {
                oo.x = cl + lu(ax_phys + off_x + mlu) + res.rel.0;
                oo.y = ct + lu(y0 + off_y + mtu) + res.rel.1;
            }
        }
        hit
    }

    /// Min-content and max-content width of the content box of grid container `id` (12.1): the sum of the column tracks
    /// when the columns are sized under a min-content / max-content constraint.
    pub(crate) fn grid_widths(&mut self, id: BoxId) -> (LU, LU) {
        let sp = Space { avail: None, min: 0, max: None };
        let mut g = self.build_gx(id, sp, sp);
        let mut base = g.tr.c.clone();
        let mut out = [0i64; 2];
        for (k, mode) in [Mode::Min, Mode::Max].into_iter().enumerate() {
            g.tr.c = base.clone();
            for it in g.items.iter_mut() {
                it.cc.c = None;
            }
            self.size_axis(&mut g, 1, None, mode, (0, None), false);
            let t = g.tr.c.iter().filter(|t| !t.gone).map(|t| t.base).sum::<i64>() + gaps_in(&g.tr.c, g.gap.c);
            if let Some(o) = out.get_mut(k) {
                *o = t;
            }
        }
        base.clear();
        (lu(out.first().copied().unwrap_or(0)), lu(out.get(1).copied().unwrap_or(0)))
    }
}
