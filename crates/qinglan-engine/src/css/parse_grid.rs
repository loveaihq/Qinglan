//! Parsing of the grid properties (css-grid-1 7 and 8): track lists with `repeat()` and line names, areas, `<grid-line>`
//! and the shorthands `grid-template`, `grid`, `grid-area`, `grid-row`, `grid-column`.

use crate::fmath::Fclamp;
use std::rc::Rc;

use super::parse_prop::p_lp;
use super::syntax::{BlockKind, Cv, split_commas};
use super::tokenizer::Token;
use super::types::*;
use super::values::parse_integer;

type Decls = Vec<(PropId, Value)>;

/// The one component value of `part` (ignoring white space).
fn single(part: &[Cv]) -> Option<&Cv> {
    let mut it = part.iter().filter(|c| !c.is_ws());
    let a = it.next()?;
    it.next().is_none().then_some(a)
}

/// `<track-breadth>` (7.2.1); `flex` allows `<flex>` (not in the minimum of `minmax()`).
fn breadth(c: &Cv, flex: bool) -> Option<Bread> {
    if let Cv::Tok(Token::Dimension { value, unit, .. }) = c
        && unit.eq_ignore_ascii_case("fr")
    {
        return (flex && *value >= 0.0).then_some(Bread::Fr(*value as f32));
    }
    match c.ident() {
        Some(s) => match s.to_ascii_lowercase().as_str() {
            "auto" => Some(Bread::Auto),
            "min-content" => Some(Bread::MinContent),
            "max-content" => Some(Bread::MaxContent),
            _ => None,
        },
        None => p_lp(c, false, true).map(Bread::Lp),
    }
}

/// `<track-size>` (7.2): a breadth, `minmax()` or `fit-content()`.
fn track(c: &Cv) -> Option<Track> {
    if let Cv::Func(n, args) = c {
        if n.eq_ignore_ascii_case("minmax") {
            let parts = split_commas(args);
            let [a, b] = parts.as_slice() else { return None };
            return Some(Track { min: breadth(single(a)?, false)?, max: breadth(single(b)?, true)?, fit: false });
        }
        if n.eq_ignore_ascii_case("fit-content") {
            let l = p_lp(single(args)?, false, true)?;
            return Some(Track { min: Bread::Auto, max: Bread::Lp(l), fit: true });
        }
    }
    let b = breadth(c, true)?;
    Some(if matches!(b, Bread::Fr(_)) { Track { min: Bread::Auto, max: b, fit: false } } else { Track { min: b.clone(), max: b, fit: false } })
}

fn names_of(inner: &[Cv]) -> Option<Vec<Rc<str>>> {
    inner.iter().filter(|c| !c.is_ws()).map(|c| c.ident().map(Rc::from)).collect()
}

/// A track list under construction: `names` always has one entry more than `tracks` (two more once an auto repeat is in).
struct Acc {
    tracks: Vec<Track>,
    names: Vec<Vec<Rc<str>>>,
    auto: Option<AutoRepeat>,
}

impl Acc {
    fn new() -> Acc {
        Acc { tracks: Vec::new(), names: vec![Vec::new()], auto: None }
    }

    fn add_names(&mut self, n: Vec<Rc<str>>) {
        if let Some(l) = self.names.last_mut() {
            l.extend(n);
        }
    }

    /// False once the list is as long as a grid may be.
    fn add_track(&mut self, t: Track) -> bool {
        if self.tracks.len() >= GRID_MAX_TRACKS {
            return false;
        }
        self.tracks.push(t);
        self.names.push(Vec::new());
        true
    }

    fn done(self) -> TrackList {
        TrackList { tracks: self.tracks, names: self.names, auto: self.auto }
    }
}

/// The tokens of a track list: tracks, `[line names]` and (when `rep`) `repeat()`.
fn list(items: &[&Cv], rep: bool) -> Option<Acc> {
    let mut acc = Acc::new();
    for &c in items {
        match c {
            Cv::Block(BlockKind::Square, inner) => acc.add_names(names_of(inner)?),
            Cv::Func(n, args) if rep && n.eq_ignore_ascii_case("repeat") => {
                let parts = split_commas(args);
                let [count, body] = parts.as_slice() else { return None };
                let count = single(count)?;
                let body: Vec<&Cv> = body.iter().filter(|c| !c.is_ws()).collect();
                let inner = list(&body, false)?;
                if inner.tracks.is_empty() {
                    return None;
                }
                if let Some(id) = count.ident() {
                    // 7.2.3.2: only one auto repeat, with tracks that have a fixed breadth.
                    let fit = match id.to_ascii_lowercase().as_str() {
                        "auto-fill" => false,
                        "auto-fit" => true,
                        _ => return None,
                    };
                    if acc.auto.is_some() || inner.tracks.iter().any(|t| t.fit || !(matches!(t.min, Bread::Lp(_)) || matches!(t.max, Bread::Lp(_)))) {
                        return None;
                    }
                    acc.auto = Some(AutoRepeat { at: acc.tracks.len(), fit, tracks: inner.tracks, names: inner.names });
                    acc.names.push(Vec::new());
                } else {
                    let n = parse_integer(count)?;
                    if n < 1 {
                        return None;
                    }
                    'outer: for _ in 0..(n as usize).min(GRID_MAX_TRACKS) {
                        for (k, t) in inner.tracks.iter().enumerate() {
                            acc.add_names(inner.names.get(k).cloned().unwrap_or_default());
                            if !acc.add_track(t.clone()) {
                                break 'outer;
                            }
                        }
                        acc.add_names(inner.names.last().cloned().unwrap_or_default());
                    }
                }
            }
            _ => {
                acc.add_track(track(c)?);
            }
        }
    }
    Some(acc)
}

/// `grid-template-columns` / `grid-template-rows`: `none | <track-list> | <auto-track-list>`.
pub(super) fn parse_tracks(items: &[&Cv]) -> Option<Value> {
    if let [c] = items
        && c.is_ident("none")
    {
        return Some(Value::Tracks(None));
    }
    let acc = list(items, true)?;
    if acc.tracks.is_empty() && acc.auto.is_none() {
        return None;
    }
    Some(Value::Tracks(Some(Rc::new(acc.done()))))
}

/// `grid-auto-columns` / `grid-auto-rows`: `<track-size>+`.
pub(super) fn parse_auto_tracks(items: &[&Cv]) -> Option<Value> {
    let v = items.iter().take(GRID_MAX_TRACKS).map(|c| track(c)).collect::<Option<Vec<_>>>()?;
    (!v.is_empty()).then(|| Value::AutoTracks(Rc::new(v)))
}

/// `grid-auto-flow`: `[ row | column ] || dense`.
pub(super) fn parse_flow(items: &[&Cv]) -> Option<Value> {
    let (mut dir, mut col, mut dense) = (false, false, false);
    for c in items {
        match c.ident()?.to_ascii_lowercase().as_str() {
            "row" if !dir => dir = true,
            "column" if !dir => (dir, col) = (true, true),
            "dense" if !dense => dense = true,
            _ => return None,
        }
    }
    (!items.is_empty()).then_some(Value::GridFlow(col, dense))
}

/// The named areas of the rows of a `grid-template-areas` (7.3): every name must fill a rectangle.
fn areas_of(rows: &[&str]) -> Option<Areas> {
    let cells: Vec<Vec<Option<&str>>> =
        rows.iter().map(|r| r.split_ascii_whitespace().map(|t| if t.bytes().all(|b| b == b'.') { None } else { Some(t) }).collect()).collect();
    let cols = cells.first()?.len();
    if cols == 0 || cols > GRID_MAX_TRACKS || cells.len() > GRID_MAX_TRACKS || cells.iter().any(|r| r.len() != cols) {
        return None;
    }
    let mut list: Vec<(Rc<str>, u32, u32, u32, u32)> = Vec::new();
    for (r, row) in cells.iter().enumerate() {
        for (c, cell) in row.iter().enumerate() {
            let Some(n) = cell else { continue };
            if list.iter().any(|a| &*a.0 == *n) {
                continue;
            }
            let (mut w, mut h) = (1, 1);
            while row.get(c + w) == Some(cell) {
                w += 1;
            }
            while cells.get(r + h).and_then(|rr| rr.get(c)) == Some(cell) {
                h += 1;
            }
            let inside = (r..r + h).all(|y| (c..c + w).all(|x| cells.get(y).and_then(|rr| rr.get(x)) == Some(cell)));
            let total = cells.iter().flatten().filter(|x| *x == cell).count();
            if !inside || total != w * h {
                return None;
            }
            list.push((Rc::from(*n), r as u32 + 1, c as u32 + 1, (r + h) as u32 + 1, (c + w) as u32 + 1));
        }
    }
    Some(Areas { rows: cells.len() as u32, cols: cols as u32, list })
}

/// `grid-template-areas`: `none | <string>+`.
pub(super) fn parse_areas(items: &[&Cv]) -> Option<Value> {
    if let [c] = items
        && c.is_ident("none")
    {
        return Some(Value::Areas(None));
    }
    let rows: Vec<&str> = items.iter().map(|c| if let Cv::Tok(Token::Str(s)) = c { Some(s.as_str()) } else { None }).collect::<Option<_>>()?;
    areas_of(&rows).map(|a| Value::Areas(Some(Rc::new(a))))
}

fn template_decls(rows: Option<TrackList>, cols: Option<Value>, areas: Option<Areas>) -> Decls {
    vec![
        (PropId::GridTemplateRows, Value::Tracks(rows.map(Rc::new))),
        (PropId::GridTemplateColumns, cols.unwrap_or(Value::Tracks(None))),
        (PropId::GridTemplateAreas, Value::Areas(areas.map(Rc::new))),
    ]
}

/// `grid-template`: `none | <rows> / <columns> | [ <line-names>? <string> <track-size>? <line-names>? ]+ [ / <columns> ]?`.
pub(super) fn parse_template(items: &[&Cv]) -> Option<Decls> {
    if let [c] = items
        && c.is_ident("none")
    {
        return Some(template_decls(None, None, None));
    }
    let slash = items.iter().position(|c| c.is_delim('/'));
    let (lhs, rhs) = match slash {
        Some(i) => (items.get(..i)?, Some(items.get(i + 1..)?)),
        None => (items, None),
    };
    let cols = match rhs {
        Some(r) => Some(parse_tracks(r)?),
        None => None,
    };
    if lhs.iter().any(|c| matches!(c, Cv::Tok(Token::Str(_)))) {
        let mut acc = Acc::new();
        let mut strs: Vec<&str> = Vec::new();
        let mut explicit = false;
        for &c in lhs {
            match c {
                Cv::Block(BlockKind::Square, inner) => acc.add_names(names_of(inner)?),
                Cv::Tok(Token::Str(s)) => {
                    if !acc.add_track(Track::AUTO) {
                        return None;
                    }
                    strs.push(s.as_str());
                    explicit = false;
                }
                _ => {
                    if explicit || strs.is_empty() {
                        return None;
                    }
                    *acc.tracks.last_mut()? = track(c)?;
                    explicit = true;
                }
            }
        }
        let areas = areas_of(&strs)?;
        return Some(template_decls(Some(acc.done()), cols, Some(areas)));
    }
    let rows = parse_tracks(lhs)?;
    Some(vec![(PropId::GridTemplateRows, rows), (PropId::GridTemplateColumns, cols?), (PropId::GridTemplateAreas, Value::Areas(None))])
}

/// `auto-flow && dense?` at the start of `side`, then the `grid-auto-*` tracks after it.
fn auto_flow_side(side: &[&Cv]) -> Option<(bool, Value)> {
    let (mut flow, mut dense, mut n) = (false, false, 0);
    for c in side {
        if c.is_ident("auto-flow") && !flow {
            flow = true;
        } else if c.is_ident("dense") && !dense {
            dense = true;
        } else {
            break;
        }
        n += 1;
    }
    if !flow {
        return None;
    }
    let rest = side.get(n..)?;
    let tracks = if rest.is_empty() { Value::AutoTracks(Rc::new(vec![Track::AUTO])) } else { parse_auto_tracks(rest)? };
    Some((dense, tracks))
}

/// `grid`: a `grid-template`, or one axis explicit and the other automatic.
pub(super) fn parse_grid(items: &[&Cv]) -> Option<Decls> {
    use PropId::*;
    let auto = || Value::AutoTracks(Rc::new(vec![Track::AUTO]));
    if let Some(i) = items.iter().position(|c| c.is_delim('/')) {
        let (l, r) = (items.get(..i)?, items.get(i + 1..)?);
        if let Some((dense, rows)) = auto_flow_side(l) {
            return Some(vec![
                (GridTemplateRows, Value::Tracks(None)),
                (GridTemplateColumns, parse_tracks(r)?),
                (GridTemplateAreas, Value::Areas(None)),
                (GridAutoRows, rows),
                (GridAutoColumns, auto()),
                (GridAutoFlow, Value::GridFlow(false, dense)),
            ]);
        }
        if let Some((dense, cols)) = auto_flow_side(r) {
            return Some(vec![
                (GridTemplateRows, parse_tracks(l)?),
                (GridTemplateColumns, Value::Tracks(None)),
                (GridTemplateAreas, Value::Areas(None)),
                (GridAutoRows, auto()),
                (GridAutoColumns, cols),
                (GridAutoFlow, Value::GridFlow(true, dense)),
            ]);
        }
    }
    let mut v = parse_template(items)?;
    v.extend([(GridAutoRows, auto()), (GridAutoColumns, auto()), (GridAutoFlow, Value::GridFlow(false, false))]);
    Some(v)
}

/// `<grid-line>` (8.3): `auto | <custom-ident> | [ <integer> && <custom-ident>? ] | [ span && [ <integer> || <custom-ident> ] ]`.
pub(super) fn grid_line(items: &[&Cv]) -> Option<GridLine> {
    let (mut span, mut num, mut id): (bool, Option<i32>, Option<Rc<str>>) = (false, None, None);
    for c in items {
        if let Some(s) = c.ident() {
            match s.to_ascii_lowercase().as_str() {
                "auto" => return (items.len() == 1).then_some(GridLine::Auto),
                "span" if !span => span = true,
                "span" => return None,
                _ if id.is_none() => id = Some(Rc::from(s)),
                _ => return None,
            }
        } else if num.is_none() {
            num = Some(parse_integer(c)?);
        } else {
            return None;
        }
    }
    let max = GRID_MAX_TRACKS as i32;
    Some(match (span, num, id) {
        (true, n, id) => {
            let n = n.unwrap_or(1);
            if n < 1 {
                return None;
            }
            let n = n.min(max) as u32;
            match id {
                Some(i) => GridLine::SpanIdent(n, i),
                None => GridLine::Span(n),
            }
        }
        (false, Some(0), _) => return None,
        (false, Some(n), Some(i)) => GridLine::NumIdent(n.fclamp(-max, max), i),
        (false, Some(n), None) => GridLine::Num(n.fclamp(-max, max)),
        (false, None, Some(i)) => GridLine::Ident(i),
        (false, None, None) => return None,
    })
}

/// `grid-area`, `grid-row`, `grid-column`: `<grid-line> [ / <grid-line> ]{0,3}` (one for the last two).
pub(super) fn parse_lines(name: &str, items: &[&Cv]) -> Option<Decls> {
    use PropId::*;
    let v = items.split(|c| c.is_delim('/')).map(grid_line).collect::<Option<Vec<_>>>()?;
    let area = name == "grid-area";
    if v.len() > if area { 4 } else { 2 } {
        return None;
    }
    // An omitted end is the same custom ident as the start, else `auto`.
    let omitted = |first: &GridLine| if matches!(first, GridLine::Ident(_)) { first.clone() } else { GridLine::Auto };
    let first = v.first()?.clone();
    let get = |i: usize, from: &GridLine| v.get(i).cloned().unwrap_or_else(|| omitted(from));
    let out = if area {
        let cs = get(1, &first);
        let (re, ce) = (get(2, &first), get(3, &cs));
        [(GridRowStart, first), (GridColumnStart, cs), (GridRowEnd, re), (GridColumnEnd, ce)].into()
    } else {
        let end = get(1, &first);
        if name == "grid-row" { vec![(GridRowStart, first), (GridRowEnd, end)] } else { vec![(GridColumnStart, first), (GridColumnEnd, end)] }
    };
    Some(out.into_iter().map(|(p, l)| (p, Value::GridLine(l))).collect())
}
