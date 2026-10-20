//! Computed style: the groups (`Rc`-shared), initial values and the computation of a group from
//! the cascaded declarations (css-cascade-5 "defaulting", css-values-4 "computed value").

use std::borrow::Cow;
use std::cell::RefCell;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use super::bg::{Backgrounds, BgImage, BgSize, BgVal, Pos1};
use super::color::{Color, ColorValue};
use super::fx::{Fx, Shadow};
use super::syntax::Cv;
use super::types::*;
use super::values::{LenCtx, LenUnit, Lp, fmt_num};
use crate::html::atoms::{Atom, FxHashMap, FxHasher};
use crate::platform::{FontQuery, FontSlant, Platform};

/// Custom property values (inherited), keyed by the interned name including the `--`.
pub type CustomProps = FxHashMap<Atom, Rc<Vec<Cv>>>;

#[derive(Clone, Debug, PartialEq)]
pub struct BoxStyle {
    pub display: Display,
    pub position: Position,
    pub float: Float,
    pub clear: Clear,
    pub table_layout: TableLayout,
    pub box_sizing: BoxSizing,
    pub z_index: Option<i32>,
    pub overflow_x: Overflow,
    pub overflow_y: Overflow,
    /// top, right, bottom, left.
    pub inset: [Dim; 4],
    pub width: Dim,
    pub height: Dim,
    pub min_width: Dim,
    pub min_height: Dim,
    pub max_width: Dim,
    pub max_height: Dim,
    pub margin: [Dim; 4],
    pub padding: [Lp; 4],
}

#[derive(Clone, Debug, PartialEq)]
pub struct BorderStyle {
    /// top, right, bottom, left; 0 where the style is none or hidden.
    pub width: [f32; 4],
    pub style: [BorderLineStyle; 4],
    pub color: [ColorValue; 4],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LineHeight {
    Normal,
    /// A multiplier of the element's own font size.
    Number(f32),
    Px(f32),
}

#[derive(Clone, Debug, PartialEq)]
pub struct FontStyle {
    pub family: Rc<Vec<FontFamily>>,
    pub size: f32,
    /// Set while the size comes from an absolute-size keyword (so that a change to/from monospace re-derives it).
    pub size_keyword: Option<u8>,
    pub weight: f32,
    pub style: FontStyleKind,
    pub line_height: LineHeight,
    pub letter_spacing: f32,
    pub word_spacing: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextStyle {
    pub color: Color,
    pub text_align: TextAlign,
    pub text_indent: Lp,
    pub text_transform: TextTransform,
    pub white_space_collapse: WhiteSpaceCollapse,
    pub text_wrap_mode: TextWrapMode,
    pub word_break: WordBreak,
    pub overflow_wrap: OverflowWrap,
    pub direction: Direction,
    pub visibility: Visibility,
    pub cursor: CursorKind,
    pub list_style_type: ListStyleType,
    pub list_style_position: ListStylePosition,
    pub tab_size: TabSize,
    /// `quotes` (inherited, CSS2 12.3.1).
    pub quotes: Quotes,
    /// Table properties (CSS2 17.6, 17.6.1.1, 17.4.1), all inherited: `border-spacing` is horizontal, vertical.
    pub border_collapse: BorderCollapse,
    pub border_spacing: [Lp; 2],
    pub empty_cells: EmptyCells,
    pub caption_side: CaptionSide,
    /// The element's language (nearest `lang` attribute), inherited.
    pub lang: Option<Rc<str>>,
    /// `text-shadow` (inherited).
    pub shadow: Option<Rc<Vec<Shadow>>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MiscStyle {
    pub vertical_align: VerticalAlign,
    pub text_decoration_line: TextDecorationLine,
    pub text_decoration_style: TextDecorationStyle,
    pub text_decoration_color: ColorValue,
    pub text_decoration_thickness: Option<Lp>,
    pub background_color: ColorValue,
    pub bg: Rc<Backgrounds>,
    /// `border-top-left-radius`, top-right, bottom-right, bottom-left: horizontal and vertical radius. `None`: all square (kept
    /// out of the struct because most elements have none: 128 bytes each).
    pub radius: Option<Rc<[(Lp, Lp); 4]>>,
    /// `None`: no outline.
    pub outline: Option<Rc<Outline>>,
    /// opacity, box-shadow, transform: `None` while all are initial.
    pub fx: Option<Rc<Fx>>,
}

/// The outline longhands (css-ui-3 6). The width is 0 when the style is `none`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Outline {
    pub width: f32,
    pub style: BorderLineStyle,
    pub color: ColorValue,
    pub offset: f32,
}

impl Outline {
    pub const NONE: Outline = Outline { width: 0.0, style: BorderLineStyle::None, color: ColorValue::CurrentColor, offset: 0.0 };
}

impl MiscStyle {
    /// The outline longhands (the initial ones when the element has none).
    pub fn outline(&self) -> Outline {
        self.outline.as_deref().copied().unwrap_or(Outline::NONE)
    }
}

/// Generated content (CSS2 12.2, 12.4): `content` and the counter properties. None of them is inherited.
#[derive(Clone, Debug, PartialEq)]
pub struct GenStyle {
    pub content: Content,
    pub counter_reset: Rc<Vec<(Rc<str>, i32)>>,
    pub counter_increment: Rc<Vec<(Rc<str>, i32)>>,
    pub counter_set: Rc<Vec<(Rc<str>, i32)>>,
}

impl GenStyle {
    pub fn initial() -> GenStyle {
        let none = Rc::new(Vec::new());
        GenStyle { content: Content::Normal, counter_reset: none.clone(), counter_increment: none.clone(), counter_set: none }
    }
}

/// The flex container and flex item properties (css-flexbox-1 5-8, css-align-3 gaps). None are inherited.
#[derive(Clone, Debug, PartialEq)]
pub struct FlexStyle {
    pub direction: FlexDirection,
    pub wrap: FlexWrap,
    pub order: i32,
    pub grow: f32,
    pub shrink: f32,
    pub basis: FlexBasis,
    pub justify_content: AlignValue,
    pub align_items: AlignValue,
    pub align_self: AlignValue,
    pub align_content: AlignValue,
    pub justify_items: AlignValue,
    pub justify_self: AlignValue,
    /// `None` is `normal` (0 in a flex container).
    pub row_gap: Option<Lp>,
    pub column_gap: Option<Lp>,
}

impl FlexStyle {
    pub fn initial() -> FlexStyle {
        FlexStyle {
            direction: FlexDirection::Row,
            wrap: FlexWrap::Nowrap,
            order: 0,
            grow: 0.0,
            shrink: 1.0,
            basis: FlexBasis::Auto,
            justify_content: AlignValue::NORMAL,
            align_items: AlignValue::NORMAL,
            align_self: AlignValue::AUTO,
            align_content: AlignValue::NORMAL,
            justify_items: AlignValue::NORMAL,
            justify_self: AlignValue::AUTO,
            row_gap: None,
            column_gap: None,
        }
    }
}

/// The computed style of an element.
#[derive(Clone, Debug)]
pub struct ComputedStyle {
    pub boxs: Rc<BoxStyle>,
    pub border: Rc<BorderStyle>,
    pub font: Rc<FontStyle>,
    pub text: Rc<TextStyle>,
    pub misc: Rc<MiscStyle>,
    pub gen_: Rc<GenStyle>,
    pub flex: Rc<FlexStyle>,
    pub grid: Rc<GridStyle>,
    pub custom: Rc<CustomProps>,
}

/// The grid container and grid item properties (css-grid-1 7, 8). None are inherited.
#[derive(Clone, Debug, PartialEq)]
pub struct GridStyle {
    pub cols: Option<Rc<TrackList>>,
    pub rows: Option<Rc<TrackList>>,
    pub areas: Option<Rc<Areas>>,
    pub auto_cols: Rc<Vec<Track>>,
    pub auto_rows: Rc<Vec<Track>>,
    /// `grid-auto-flow`: `column` and `dense`.
    pub flow_col: bool,
    pub dense: bool,
    /// `grid-row-start`, `grid-column-start`, `grid-row-end`, `grid-column-end`.
    pub line: [GridLine; 4],
}

impl GridStyle {
    pub fn initial() -> GridStyle {
        let auto = Rc::new(vec![Track::AUTO]);
        GridStyle { cols: None, rows: None, areas: None, auto_cols: auto.clone(), auto_rows: auto, flow_col: false, dense: false, line: Default::default() }
    }
}

impl BoxStyle {
    pub fn initial() -> BoxStyle {
        BoxStyle {
            display: Display::Inline,
            position: Position::Static,
            float: Float::None,
            clear: Clear::None,
            table_layout: TableLayout::Auto,
            box_sizing: BoxSizing::ContentBox,
            z_index: None,
            overflow_x: Overflow::Visible,
            overflow_y: Overflow::Visible,
            inset: [Dim::Auto, Dim::Auto, Dim::Auto, Dim::Auto],
            width: Dim::Auto,
            height: Dim::Auto,
            min_width: Dim::Auto,
            min_height: Dim::Auto,
            max_width: Dim::None,
            max_height: Dim::None,
            margin: [Dim::ZERO, Dim::ZERO, Dim::ZERO, Dim::ZERO],
            padding: [Lp::ZERO, Lp::ZERO, Lp::ZERO, Lp::ZERO],
        }
    }
}

impl BorderStyle {
    pub fn initial() -> BorderStyle {
        BorderStyle { width: [0.0; 4], style: [BorderLineStyle::None; 4], color: [ColorValue::CurrentColor; 4] }
    }
}

impl FontStyle {
    pub fn initial() -> FontStyle {
        FontStyle {
            family: Rc::new(vec![FontFamily::Generic(GenericFamily::Serif)]),
            size: 16.0,
            size_keyword: Some(3),
            weight: 400.0,
            style: FontStyleKind::Normal,
            line_height: LineHeight::Normal,
            letter_spacing: 0.0,
            word_spacing: 0.0,
        }
    }
}

impl TextStyle {
    pub fn initial() -> TextStyle {
        TextStyle {
            color: Color::BLACK,
            text_align: TextAlign::Start,
            text_indent: Lp::ZERO,
            text_transform: TextTransform::None,
            white_space_collapse: WhiteSpaceCollapse::Collapse,
            text_wrap_mode: TextWrapMode::Wrap,
            word_break: WordBreak::Normal,
            overflow_wrap: OverflowWrap::Normal,
            direction: Direction::Ltr,
            visibility: Visibility::Visible,
            cursor: CursorKind::Auto,
            list_style_type: ListStyleType::Disc,
            list_style_position: ListStylePosition::Outside,
            tab_size: TabSize::Spaces(8.0),
            quotes: Quotes::Auto,
            border_collapse: BorderCollapse::Separate,
            border_spacing: [Lp::ZERO, Lp::ZERO],
            empty_cells: EmptyCells::Show,
            caption_side: CaptionSide::Top,
            lang: None,
            shadow: None,
        }
    }
}

impl MiscStyle {
    pub fn initial() -> MiscStyle {
        MiscStyle::with_bg(Rc::new(Backgrounds::initial()))
    }

    /// The initial style with the (shared) initial backgrounds.
    fn with_bg(bg: Rc<Backgrounds>) -> MiscStyle {
        MiscStyle {
            bg,
            radius: None,
            outline: None,
            fx: None,
            vertical_align: VerticalAlign::Baseline,
            text_decoration_line: TextDecorationLine::NONE,
            text_decoration_style: TextDecorationStyle::Solid,
            text_decoration_color: ColorValue::CurrentColor,
            text_decoration_thickness: None,
            background_color: ColorValue::Color(Color::TRANSPARENT),
        }
    }
}

impl ComputedStyle {
    pub fn initial() -> ComputedStyle {
        ComputedStyle {
            boxs: Rc::new(BoxStyle::initial()),
            border: Rc::new(BorderStyle::initial()),
            font: Rc::new(FontStyle::initial()),
            text: Rc::new(TextStyle::initial()),
            misc: Rc::new(MiscStyle::initial()),
            gen_: Rc::new(GenStyle::initial()),
            flex: Rc::new(FlexStyle::initial()),
            grid: Rc::new(GridStyle::initial()),
            custom: Rc::new(CustomProps::default()),
        }
    }

    /// The text color of the element.
    pub fn color(&self) -> Color {
        self.text.color
    }
}

/// The winning declaration for each property of one element.
pub struct Declared<'a> {
    pub vals: Vec<Option<Cow<'a, Specified>>>,
    pub mask: u128,
}

impl<'a> Declared<'a> {
    pub fn new() -> Declared<'a> {
        Declared { vals: (0..PROP_COUNT).map(|_| None).collect(), mask: 0 }
    }
    pub fn clear(&mut self) {
        self.vals.fill(None);
        self.mask = 0;
    }
    pub fn set(&mut self, p: PropId, s: Cow<'a, Specified>) {
        if let Some(slot) = self.vals.get_mut(p.index()) {
            *slot = Some(s);
            self.mask |= p.bit();
        }
    }
    pub fn get(&self, p: PropId) -> Option<&Specified> {
        self.vals.get(p.index()).and_then(|v| v.as_deref())
    }
    pub fn touches(&self, bits: u128) -> bool {
        self.mask & bits != 0
    }
}

impl Default for Declared<'_> {
    fn default() -> Self {
        Self::new()
    }
}

/// Everything that stays the same while a whole document is styled.
pub struct Env<'a> {
    pub viewport_w: f32,
    pub viewport_h: f32,
    /// Where `ex` and `ch` get their font metrics from; without one they are half an em.
    pub platform: Option<&'a dyn Platform>,
    /// (x-height, advance of "0") as a fraction of the font size, by font description.
    ex_cache: RefCell<ExCache>,
}

/// (family hash, weight bits, style, language hash) -> (x-height, advance of "0") per em.
type ExCache = FxHashMap<(u64, u32, u8, u64), (f32, f32)>;

impl<'a> Env<'a> {
    pub fn new(viewport_w: f32, viewport_h: f32, platform: Option<&'a dyn Platform>) -> Env<'a> {
        Env { viewport_w, viewport_h, platform, ex_cache: RefCell::new(FxHashMap::default()) }
    }

    /// The size in px of `1ex` and `1ch` for `font` (css-values-4 6.1: when the font has no x-height
    /// or no "0" glyph, 0.5em is used).
    pub fn ex_ch(&self, font: &FontStyle, lang: Option<&str>) -> (f32, f32) {
        let half = font.size * 0.5;
        let Some(platform) = self.platform else { return (half, half) };
        let mut h = FxHasher::default();
        for f in font.family.iter() {
            match f {
                FontFamily::Named(n) => {
                    h.write_u8(1);
                    h.write(n.as_bytes());
                }
                FontFamily::Generic(g) => {
                    h.write_u8(2);
                    g.hash(&mut h);
                }
            }
        }
        let family = h.finish();
        let mut lh = FxHasher::default();
        if let Some(l) = lang {
            lh.write(l.as_bytes());
        }
        let key = (family, font.weight.to_bits(), font.style as u8, lh.finish());
        let cached = self.ex_cache.borrow().get(&key).copied();
        let (ex, ch) = match cached {
            Some(v) => v,
            None => {
                let q = FontQuery { families: &font.family, weight: font.weight, slant: slant_of(font.style), lang };
                let v = match platform.resolve_font(&q) {
                    Some(id) => {
                        let m = platform.font_metrics(id, 100.0);
                        (m.x_height.map_or(0.5, |x| x / 100.0), m.zero_advance.map_or(0.5, |z| z / 100.0))
                    }
                    None => (0.5, 0.5),
                };
                self.ex_cache.borrow_mut().insert(key, v);
                v
            }
        };
        (ex * font.size, ch * font.size)
    }
}

/// The platform's name for a `font-style`.
pub fn slant_of(s: FontStyleKind) -> FontSlant {
    match s {
        FontStyleKind::Normal => FontSlant::Upright,
        FontStyleKind::Italic => FontSlant::Italic,
        FontStyleKind::Oblique => FontSlant::Oblique,
    }
}

/// Resolve "what does this property become" for one declared (or not declared) property:
/// the CSS-wide keywords, inheritance and the `unset` rule are handled here.
enum Pick<'a> {
    Inherit,
    Initial,
    Value(&'a Value),
}

fn pick<'a>(d: &'a Declared<'_>, p: PropId) -> Pick<'a> {
    match d.get(p) {
        None => {
            if p.inherited() { Pick::Inherit } else { Pick::Initial }
        }
        Some(Specified::Inherit) => Pick::Inherit,
        Some(Specified::Initial) => Pick::Initial,
        Some(Specified::Unset | Specified::Revert | Specified::RevertLayer | Specified::Var(_)) => {
            if p.inherited() { Pick::Inherit } else { Pick::Initial }
        }
        Some(Specified::Value(v)) => Pick::Value(v),
    }
}

/// Absolute-size keyword table: standard and monospace-only font families.
fn keyword_size(idx: u8, mono: bool) -> f32 {
    const STD: [f32; 8] = [9.0, 10.0, 13.0, 16.0, 18.0, 24.0, 32.0, 48.0];
    const MONO: [f32; 8] = [9.0, 10.0, 12.0, 13.0, 16.0, 20.0, 26.0, 40.0];
    let t = if mono { &MONO } else { &STD };
    t.get(idx as usize).copied().unwrap_or(16.0)
}

fn is_mono_only(family: &[FontFamily]) -> bool {
    matches!(family, [FontFamily::Generic(GenericFamily::Monospace)])
}

pub fn compute_font(parent: &Rc<FontStyle>, d: &Declared<'_>, env: &Env<'_>, root_font_size: f32, lang: Option<&str>) -> Rc<FontStyle> {
    if !d.touches(FONT_BITS) {
        return parent.clone();
    }
    let mut f = (**parent).clone();
    let initial = FontStyle::initial();
    // font-family first: it decides the keyword size table.
    match pick(d, PropId::FontFamily) {
        Pick::Inherit => {}
        Pick::Initial => f.family = initial.family.clone(),
        Pick::Value(Value::FontFamily(l)) => f.family = l.clone(),
        Pick::Value(_) => {}
    }
    let mono = is_mono_only(&f.family);
    let (pex, pch) = env.ex_ch(parent, lang);
    let parent_ctx = LenCtx { font_size: parent.size, root_font_size, viewport_w: env.viewport_w, viewport_h: env.viewport_h, ex: pex, ch: pch };
    match pick(d, PropId::FontSize) {
        Pick::Inherit => {
            if let Some(k) = parent.size_keyword {
                f.size_keyword = Some(k);
                f.size = keyword_size(k, mono);
            }
        }
        Pick::Initial => {
            f.size_keyword = Some(3);
            f.size = keyword_size(3, mono);
        }
        Pick::Value(Value::FontSize(spec)) => match spec {
            FontSizeSpec::Absolute(k) => {
                f.size_keyword = Some(*k);
                f.size = keyword_size(*k, mono);
            }
            FontSizeSpec::Larger => {
                f.size_keyword = None;
                f.size = parent.size * 1.2;
            }
            FontSizeSpec::Smaller => {
                f.size_keyword = None;
                f.size = parent.size / 1.2;
            }
            FontSizeSpec::Lp(l) => {
                f.size_keyword = None;
                f.size = match l.compute(&parent_ctx) {
                    Lp::Len(v, _) => v,
                    Lp::Pct(p) => parent.size * p / 100.0,
                    Lp::Calc(c) => c.eval(parent.size),
                };
            }
        },
        Pick::Value(_) => {}
    }
    f.size = f.size.clamp(0.0, 10_000.0);
    // The element's own `em`, `ex` and `ch` (line-height, letter-spacing) use its final font.
    let (own_ex, own_ch) = if f.size == parent.size && f.family == parent.family && f.weight == parent.weight && f.style == parent.style {
        (pex, pch)
    } else {
        env.ex_ch(&f, lang)
    };
    let own = LenCtx { font_size: f.size, ex: own_ex, ch: own_ch, ..parent_ctx };
    match pick(d, PropId::FontWeight) {
        Pick::Inherit => {}
        Pick::Initial => f.weight = 400.0,
        Pick::Value(Value::FontWeight(w)) => {
            f.weight = match w {
                FontWeightSpec::Number(n) => *n,
                FontWeightSpec::Bolder => bolder(parent.weight),
                FontWeightSpec::Lighter => lighter(parent.weight),
            }
        }
        Pick::Value(_) => {}
    }
    match pick(d, PropId::FontStyle) {
        Pick::Inherit => {}
        Pick::Initial => f.style = FontStyleKind::Normal,
        Pick::Value(Value::FontStyle(s)) => f.style = *s,
        Pick::Value(_) => {}
    }
    match pick(d, PropId::LineHeight) {
        Pick::Inherit => {
            // A percentage / length line-height is inherited as the computed absolute length.
        }
        Pick::Initial => f.line_height = LineHeight::Normal,
        Pick::Value(Value::LineHeight(l)) => {
            f.line_height = match l {
                LineHeightSpec::Normal => LineHeight::Normal,
                LineHeightSpec::Number(n) => LineHeight::Number(*n),
                LineHeightSpec::Lp(lp) => match lp.compute(&own) {
                    Lp::Len(v, _) => LineHeight::Px(v),
                    Lp::Pct(p) => LineHeight::Px(f.size * p / 100.0),
                    Lp::Calc(c) => LineHeight::Px(c.eval(f.size)),
                },
            }
        }
        Pick::Value(_) => {}
    }
    for (p, slot) in [(PropId::LetterSpacing, 0usize), (PropId::WordSpacing, 1)] {
        let cur = match pick(d, p) {
            Pick::Inherit => None,
            Pick::Initial => Some(0.0),
            Pick::Value(Value::Spacing(None)) => Some(0.0),
            Pick::Value(Value::Spacing(Some(l))) => Some(match l.compute(&own) {
                Lp::Len(v, _) => v,
                _ => 0.0,
            }),
            Pick::Value(_) => None,
        };
        if let Some(v) = cur {
            if slot == 0 {
                f.letter_spacing = v;
            } else {
                f.word_spacing = v;
            }
        }
    }
    Rc::new(f)
}

fn bolder(w: f32) -> f32 {
    if w < 350.0 {
        400.0
    } else if w < 550.0 {
        700.0
    } else {
        900.0
    }
}

fn lighter(w: f32) -> f32 {
    if w < 550.0 {
        100.0
    } else if w < 750.0 {
        400.0
    } else {
        700.0
    }
}

pub fn compute_text(parent: &Rc<TextStyle>, d: &Declared<'_>, ctx: &LenCtx, lang: Option<&str>) -> Rc<TextStyle> {
    let lang_changed = lang.is_some_and(|l| parent.lang.as_deref() != Some(l));
    if !d.touches(TEXT_BITS) && !lang_changed {
        return parent.clone();
    }
    let mut t = (**parent).clone();
    let init = TextStyle::initial();
    macro_rules! simple {
        ($prop:expr, $variant:ident, $field:ident) => {
            match pick(d, $prop) {
                Pick::Inherit => {}
                Pick::Initial => t.$field = init.$field.clone(),
                Pick::Value(Value::$variant(v)) => t.$field = v.clone(),
                Pick::Value(_) => {}
            }
        };
    }
    match pick(d, PropId::Color) {
        Pick::Inherit => {}
        Pick::Initial => t.color = init.color,
        Pick::Value(Value::Color(ColorValue::Color(c))) => t.color = *c,
        // `currentcolor` for `color` means `inherit`.
        Pick::Value(Value::Color(ColorValue::CurrentColor)) => {}
        Pick::Value(_) => {}
    }
    simple!(PropId::TextAlign, TextAlign, text_align);
    match pick(d, PropId::TextIndent) {
        Pick::Inherit => {}
        Pick::Initial => t.text_indent = Lp::ZERO,
        Pick::Value(Value::TextIndent(l)) => t.text_indent = l.compute(ctx),
        Pick::Value(_) => {}
    }
    simple!(PropId::TextTransform, TextTransform, text_transform);
    simple!(PropId::WhiteSpaceCollapse, WhiteSpaceCollapse, white_space_collapse);
    simple!(PropId::TextWrapMode, TextWrapMode, text_wrap_mode);
    simple!(PropId::WordBreak, WordBreak, word_break);
    simple!(PropId::OverflowWrap, OverflowWrap, overflow_wrap);
    simple!(PropId::Direction, Direction, direction);
    simple!(PropId::Visibility, Visibility, visibility);
    simple!(PropId::Cursor, Cursor, cursor);
    simple!(PropId::ListStyleType, ListStyleType, list_style_type);
    simple!(PropId::ListStylePosition, ListStylePosition, list_style_position);
    simple!(PropId::Quotes, Quotes, quotes);
    simple!(PropId::BorderCollapse, BorderCollapse, border_collapse);
    simple!(PropId::EmptyCells, EmptyCells, empty_cells);
    simple!(PropId::CaptionSide, CaptionSide, caption_side);
    match pick(d, PropId::TextShadow) {
        Pick::Inherit => {}
        Pick::Initial => t.shadow = None,
        Pick::Value(Value::Shadows(l)) => t.shadow = l.as_ref().map(|l| Rc::new(l.iter().map(|s| s.compute(ctx)).collect())),
        Pick::Value(_) => {}
    }
    match pick(d, PropId::BorderSpacing) {
        Pick::Inherit => {}
        Pick::Initial => t.border_spacing = [Lp::ZERO, Lp::ZERO],
        Pick::Value(Value::TableSpacing(a, b)) => t.border_spacing = [a.compute(ctx), b.compute(ctx)],
        Pick::Value(_) => {}
    }
    match pick(d, PropId::TabSize) {
        Pick::Inherit => {}
        Pick::Initial => t.tab_size = TabSize::Spaces(8.0),
        Pick::Value(Value::TabSize(TabSize::Len(l))) => t.tab_size = TabSize::Len(l.compute(ctx)),
        Pick::Value(Value::TabSize(s)) => t.tab_size = s.clone(),
        Pick::Value(_) => {}
    }
    if let Some(l) = lang {
        t.lang = Some(Rc::from(l));
    }
    Rc::new(t)
}

fn blockify(d: Display) -> Display {
    match d {
        Display::Inline | Display::InlineBlock | Display::Ruby | Display::RubyText => Display::Block,
        Display::InlineTable => Display::Table,
        Display::InlineFlex => Display::Flex,
        Display::InlineGrid => Display::Grid,
        Display::TableRowGroup
        | Display::TableHeaderGroup
        | Display::TableFooterGroup
        | Display::TableRow
        | Display::TableColumnGroup
        | Display::TableColumn
        | Display::TableCell
        | Display::TableCaption => Display::Block,
        other => other,
    }
}

pub fn compute_box(parent: &Rc<BoxStyle>, init_rc: &Rc<BoxStyle>, d: &Declared<'_>, ctx: &LenCtx, is_root: bool) -> Rc<BoxStyle> {
    // css-flexbox-1 4: the children of a flex container are blockified and do not float.
    let flex_item = matches!(parent.display, Display::Flex | Display::InlineFlex | Display::Grid | Display::InlineGrid);
    if !d.touches(BOX_BITS) && !is_root && !flex_item {
        return init_rc.clone();
    }
    let init = BoxStyle::initial();
    let mut b = init.clone();
    macro_rules! simple {
        ($prop:expr, $variant:ident, $field:ident) => {
            match pick(d, $prop) {
                Pick::Inherit => b.$field = parent.$field.clone(),
                Pick::Initial => {}
                Pick::Value(Value::$variant(v)) => b.$field = v.clone(),
                Pick::Value(_) => {}
            }
        };
    }
    simple!(PropId::Display, Display, display);
    simple!(PropId::Position, Position, position);
    simple!(PropId::Float, Float, float);
    simple!(PropId::Clear, Clear, clear);
    simple!(PropId::TableLayout, TableLayout, table_layout);
    simple!(PropId::BoxSizing, BoxSizing, box_sizing);
    simple!(PropId::ZIndex, ZIndex, z_index);
    simple!(PropId::OverflowX, Overflow, overflow_x);
    simple!(PropId::OverflowY, Overflow, overflow_y);
    type DimAccess = (PropId, fn(&mut BoxStyle) -> &mut Dim, fn(&BoxStyle) -> &Dim);
    let dims: [DimAccess; 14] = [
        (PropId::Top, |b| &mut b.inset[0], |b| &b.inset[0]),
        (PropId::Right, |b| &mut b.inset[1], |b| &b.inset[1]),
        (PropId::Bottom, |b| &mut b.inset[2], |b| &b.inset[2]),
        (PropId::Left, |b| &mut b.inset[3], |b| &b.inset[3]),
        (PropId::Width, |b| &mut b.width, |b| &b.width),
        (PropId::Height, |b| &mut b.height, |b| &b.height),
        (PropId::MinWidth, |b| &mut b.min_width, |b| &b.min_width),
        (PropId::MinHeight, |b| &mut b.min_height, |b| &b.min_height),
        (PropId::MaxWidth, |b| &mut b.max_width, |b| &b.max_width),
        (PropId::MaxHeight, |b| &mut b.max_height, |b| &b.max_height),
        (PropId::MarginTop, |b| &mut b.margin[0], |b| &b.margin[0]),
        (PropId::MarginRight, |b| &mut b.margin[1], |b| &b.margin[1]),
        (PropId::MarginBottom, |b| &mut b.margin[2], |b| &b.margin[2]),
        (PropId::MarginLeft, |b| &mut b.margin[3], |b| &b.margin[3]),
    ];
    for (p, set, get) in dims {
        match pick(d, p) {
            Pick::Inherit => *set(&mut b) = get(parent).clone(),
            Pick::Initial => {}
            Pick::Value(Value::Dim(v)) => *set(&mut b) = v.compute(ctx),
            Pick::Value(_) => {}
        }
    }
    let pads = [PropId::PaddingTop, PropId::PaddingRight, PropId::PaddingBottom, PropId::PaddingLeft];
    for (i, p) in pads.into_iter().enumerate() {
        match pick(d, p) {
            Pick::Inherit => {
                if let (Some(dst), Some(src)) = (b.padding.get_mut(i), parent.padding.get(i)) {
                    *dst = src.clone();
                }
            }
            Pick::Initial => {}
            Pick::Value(Value::Dim(Dim::Lp(l))) => {
                if let Some(dst) = b.padding.get_mut(i) {
                    *dst = l.compute(ctx);
                }
            }
            Pick::Value(_) => {}
        }
    }
    // sticky is treated as relative (L1 task sheet).
    if b.position == Position::Sticky {
        b.position = Position::Relative;
    }
    // CSS2 9.7: absolute positioning and floats blockify the display and cancel the float.
    if matches!(b.position, Position::Absolute | Position::Fixed) {
        b.float = Float::None;
        b.display = blockify(b.display);
    } else if flex_item {
        b.float = Float::None;
        b.display = blockify(b.display);
        // css-flexbox-1 4.3: `z-index` applies to a flex item that is not positioned and gives it a stacking context; a
        // relatively positioned box without offsets paints and stacks the same way.
        if b.z_index.is_some() && b.position == Position::Static {
            b.position = Position::Relative;
        }
    } else if b.float != Float::None || is_root {
        b.display = blockify(b.display);
    }
    // overflow: if one axis is neither visible nor clip, visible becomes auto and clip becomes hidden.
    let (ox, oy) = (b.overflow_x, b.overflow_y);
    let scrollish = |o: Overflow| !matches!(o, Overflow::Visible | Overflow::Clip);
    if scrollish(ox) || scrollish(oy) {
        let fix = |o: Overflow| match o {
            Overflow::Visible => Overflow::Auto,
            Overflow::Clip => Overflow::Hidden,
            other => other,
        };
        b.overflow_x = fix(ox);
        b.overflow_y = fix(oy);
    }
    Rc::new(b)
}

pub fn compute_border(parent: &Rc<BorderStyle>, init_rc: &Rc<BorderStyle>, d: &Declared<'_>, ctx: &LenCtx) -> Rc<BorderStyle> {
    if !d.touches(BORDER_BITS) {
        return init_rc.clone();
    }
    let mut b = BorderStyle::initial();
    let widths = [PropId::BorderTopWidth, PropId::BorderRightWidth, PropId::BorderBottomWidth, PropId::BorderLeftWidth];
    let styles = [PropId::BorderTopStyle, PropId::BorderRightStyle, PropId::BorderBottomStyle, PropId::BorderLeftStyle];
    let colors = [PropId::BorderTopColor, PropId::BorderRightColor, PropId::BorderBottomColor, PropId::BorderLeftColor];
    for i in 0..4 {
        let (Some(&wp), Some(&sp), Some(&cp)) = (widths.get(i), styles.get(i), colors.get(i)) else { continue };
        let style = match pick(d, sp) {
            Pick::Inherit => parent.style.get(i).copied().unwrap_or(BorderLineStyle::None),
            Pick::Initial => BorderLineStyle::None,
            Pick::Value(Value::BorderStyle(s)) => *s,
            Pick::Value(_) => BorderLineStyle::None,
        };
        // The computed width is 0 when the style is none or hidden; 3px (medium) otherwise by default.
        let width = if matches!(style, BorderLineStyle::None | BorderLineStyle::Hidden) {
            0.0
        } else {
            match pick(d, wp) {
                Pick::Inherit => parent.width.get(i).copied().unwrap_or(3.0),
                Pick::Initial => 3.0,
                Pick::Value(Value::BorderWidth(BorderWidthSpec::Kw(k))) => match k {
                    BorderWidthKw::Thin => 1.0,
                    BorderWidthKw::Medium => 3.0,
                    BorderWidthKw::Thick => 5.0,
                },
                Pick::Value(Value::BorderWidth(BorderWidthSpec::Lp(l))) => match l.compute(ctx) {
                    Lp::Len(v, _) => v.max(0.0),
                    _ => 0.0,
                },
                Pick::Value(_) => 3.0,
            }
        };
        let color = match pick(d, cp) {
            Pick::Inherit => parent.color.get(i).copied().unwrap_or(ColorValue::CurrentColor),
            Pick::Initial => ColorValue::CurrentColor,
            Pick::Value(Value::Color(c)) => *c,
            Pick::Value(_) => ColorValue::CurrentColor,
        };
        if let (Some(w), Some(s), Some(c)) = (b.width.get_mut(i), b.style.get_mut(i), b.color.get_mut(i)) {
            *w = width;
            *s = style;
            *c = color;
        }
    }
    Rc::new(b)
}

pub fn compute_misc(parent: &Rc<MiscStyle>, init_rc: &Rc<MiscStyle>, d: &Declared<'_>, ctx: &LenCtx) -> Rc<MiscStyle> {
    if !d.touches(MISC_BITS) {
        return init_rc.clone();
    }
    let mut m = MiscStyle::with_bg(init_rc.bg.clone());
    match pick(d, PropId::VerticalAlign) {
        Pick::Inherit => m.vertical_align = parent.vertical_align.clone(),
        Pick::Initial => {}
        Pick::Value(Value::VerticalAlign(v)) => {
            m.vertical_align = match v {
                VerticalAlign::Lp(l) => VerticalAlign::Lp(l.compute(ctx)),
                other => other.clone(),
            }
        }
        Pick::Value(_) => {}
    }
    match pick(d, PropId::TextDecorationLine) {
        Pick::Inherit => m.text_decoration_line = parent.text_decoration_line,
        Pick::Initial => {}
        Pick::Value(Value::TextDecorationLine(v)) => m.text_decoration_line = *v,
        Pick::Value(_) => {}
    }
    match pick(d, PropId::TextDecorationStyle) {
        Pick::Inherit => m.text_decoration_style = parent.text_decoration_style,
        Pick::Initial => {}
        Pick::Value(Value::TextDecorationStyle(v)) => m.text_decoration_style = *v,
        Pick::Value(_) => {}
    }
    match pick(d, PropId::TextDecorationColor) {
        Pick::Inherit => m.text_decoration_color = parent.text_decoration_color,
        Pick::Initial => {}
        Pick::Value(Value::Color(v)) => m.text_decoration_color = *v,
        Pick::Value(_) => {}
    }
    match pick(d, PropId::TextDecorationThickness) {
        Pick::Inherit => m.text_decoration_thickness = parent.text_decoration_thickness.clone(),
        Pick::Initial => {}
        Pick::Value(Value::TextDecorationThickness(v)) => m.text_decoration_thickness = v.as_ref().map(|l| l.compute(ctx)),
        Pick::Value(_) => {}
    }
    match pick(d, PropId::BackgroundColor) {
        Pick::Inherit => m.background_color = parent.background_color,
        Pick::Initial => {}
        Pick::Value(Value::Color(v)) => m.background_color = *v,
        Pick::Value(_) => {}
    }
    if d.touches(BG_BITS) {
        m.bg = compute_backgrounds(&parent.bg, &init_rc.bg, d, ctx);
    }
    let radii = [PropId::BorderTopLeftRadius, PropId::BorderTopRightRadius, PropId::BorderBottomRightRadius, PropId::BorderBottomLeftRadius];
    let mut rad: [(Lp, Lp); 4] = std::array::from_fn(|_| (Lp::ZERO, Lp::ZERO));
    for (i, p) in radii.into_iter().enumerate() {
        let Some(dst) = rad.get_mut(i) else { continue };
        match pick(d, p) {
            Pick::Inherit => *dst = parent.radius.as_ref().and_then(|r| r.get(i)).cloned().unwrap_or((Lp::ZERO, Lp::ZERO)),
            Pick::Value(Value::Radius(a, b)) => *dst = (a.compute(ctx), b.compute(ctx)),
            _ => {}
        }
    }
    if rad.iter().any(|(a, b)| *a != Lp::ZERO || *b != Lp::ZERO) {
        m.radius = Some(Rc::new(rad));
    }
    if d.touches(PropId::OutlineWidth.bit() | PropId::OutlineStyle.bit() | PropId::OutlineColor.bit() | PropId::OutlineOffset.bit()) {
        let po = parent.outline();
        let mut o = Outline::NONE;
        o.style = match pick(d, PropId::OutlineStyle) {
            Pick::Inherit => po.style,
            Pick::Value(Value::BorderStyle(s)) => *s,
            _ => BorderLineStyle::None,
        };
        o.width = if o.style == BorderLineStyle::None {
            0.0
        } else {
            match pick(d, PropId::OutlineWidth) {
                Pick::Inherit => po.width,
                Pick::Value(Value::BorderWidth(BorderWidthSpec::Kw(k))) => match k {
                    BorderWidthKw::Thin => 1.0,
                    BorderWidthKw::Medium => 3.0,
                    BorderWidthKw::Thick => 5.0,
                },
                Pick::Value(Value::BorderWidth(BorderWidthSpec::Lp(l))) => match l.compute(ctx) {
                    Lp::Len(v, _) => v.max(0.0),
                    _ => 0.0,
                },
                _ => 3.0,
            }
        };
        o.color = match pick(d, PropId::OutlineColor) {
            Pick::Inherit => po.color,
            Pick::Value(Value::Color(c)) => *c,
            _ => ColorValue::CurrentColor,
        };
        o.offset = match pick(d, PropId::OutlineOffset) {
            Pick::Inherit => po.offset,
            Pick::Value(Value::Length(l)) => l.compute(ctx).as_px().unwrap_or(0.0),
            _ => 0.0,
        };
        if o != Outline::NONE {
            m.outline = Some(Rc::new(o));
        }
    }
    if d.touches(FX_BITS) {
        let pf = parent.fx.as_deref();
        let mut fx = Fx::initial();
        match pick(d, PropId::Opacity) {
            Pick::Inherit => fx.opacity = pf.map_or(255, |p| p.opacity),
            Pick::Value(Value::Opacity(v)) => fx.opacity = (v * 255.0).round() as u8,
            _ => {}
        }
        match pick(d, PropId::BoxShadow) {
            Pick::Inherit => fx.shadows = pf.map_or_else(Vec::new, |p| p.shadows.clone()),
            Pick::Value(Value::Shadows(Some(l))) => fx.shadows = l.iter().map(|s| s.compute(ctx)).collect(),
            _ => {}
        }
        match pick(d, PropId::Transform) {
            Pick::Inherit => fx.xform = pf.map_or_else(Vec::new, |p| p.xform.clone()),
            Pick::Value(Value::Transform(Some(l))) => fx.xform = l.iter().map(|f| f.compute(ctx)).collect(),
            _ => {}
        }
        match pick(d, PropId::TransformOrigin) {
            Pick::Inherit => fx.origin = pf.map_or(fx.origin.clone(), |p| p.origin.clone()),
            Pick::Value(Value::Origin(x, y)) => fx.origin = (x.compute(ctx), y.compute(ctx)),
            _ => {}
        }
        if fx != Fx::initial() {
            m.fx = Some(Rc::new(fx));
        }
    }
    // Declared but equal to the initial style (`vertical-align: baseline`, `background: none`): share that one.
    if m == **init_rc { init_rc.clone() } else { Rc::new(m) }
}

/// The background longhands (not inherited; `inherit` copies the parent's list).
fn compute_backgrounds(parent: &Rc<Backgrounds>, init: &Rc<Backgrounds>, d: &Declared<'_>, ctx: &LenCtx) -> Rc<Backgrounds> {
    // `background: <color>` and the like leave every list initial: share the initial one.
    let real = [PropId::BackgroundImage, PropId::BackgroundPositionX, PropId::BackgroundPositionY, PropId::BackgroundSize, PropId::BackgroundRepeat, PropId::BackgroundOrigin, PropId::BackgroundClip]
        .into_iter()
        .any(|p| matches!(pick(d, p), Pick::Inherit | Pick::Value(Value::Bg(_))));
    if !real {
        return init.clone();
    }
    let mut b = Backgrounds::initial();
    match pick(d, PropId::BackgroundImage) {
        Pick::Inherit => b.images = parent.images.clone(),
        Pick::Value(Value::Bg(v)) => {
            if let BgVal::Images(l) = &**v {
                b.images = l
                    .iter()
                    .map(|i| match i {
                        BgImage::Gradient(g) => BgImage::Gradient(Rc::new(g.compute(ctx))),
                        o => o.clone(),
                    })
                    .collect();
            }
        }
        _ => {}
    }
    for (p, x) in [(PropId::BackgroundPositionX, true), (PropId::BackgroundPositionY, false)] {
        let dst = if x { &mut b.pos_x } else { &mut b.pos_y };
        match pick(d, p) {
            Pick::Inherit => *dst = if x { parent.pos_x.clone() } else { parent.pos_y.clone() },
            Pick::Value(Value::Bg(v)) => {
                if let BgVal::Pos(l) = &**v {
                    *dst = l.iter().map(|p| p.compute(ctx)).collect();
                }
            }
            _ => {}
        }
    }
    match pick(d, PropId::BackgroundSize) {
        Pick::Inherit => b.size = parent.size.clone(),
        Pick::Value(Value::Bg(v)) => {
            if let BgVal::Sizes(l) = &**v {
                b.size = l.iter().map(|s| s.compute(ctx)).collect();
            }
        }
        _ => {}
    }
    match pick(d, PropId::BackgroundRepeat) {
        Pick::Inherit => b.repeat = parent.repeat.clone(),
        Pick::Value(Value::Bg(v)) => {
            if let BgVal::Repeats(l) = &**v {
                b.repeat = l.clone();
            }
        }
        _ => {}
    }
    for (p, origin) in [(PropId::BackgroundOrigin, true), (PropId::BackgroundClip, false)] {
        let dst = if origin { &mut b.origin } else { &mut b.clip };
        match pick(d, p) {
            Pick::Inherit => *dst = if origin { parent.origin.clone() } else { parent.clip.clone() },
            Pick::Value(Value::Bg(v)) => {
                if let BgVal::Boxes(l) = &**v {
                    *dst = l.clone();
                }
            }
            _ => {}
        }
    }
    b.has_url = b.images.iter().any(|i| matches!(i, BgImage::Url(_)));
    Rc::new(b)
}

/// `content` and the counter properties (not inherited: a missing or `inherit`-less declaration is the initial value).
pub fn compute_gen(parent: &Rc<GenStyle>, init_rc: &Rc<GenStyle>, d: &Declared<'_>) -> Rc<GenStyle> {
    if !d.touches(GEN_BITS) {
        return init_rc.clone();
    }
    let mut g = GenStyle::initial();
    match pick(d, PropId::Content) {
        Pick::Inherit => g.content = parent.content.clone(),
        Pick::Initial => {}
        Pick::Value(Value::Content(c)) => g.content = c.clone(),
        Pick::Value(_) => {}
    }
    macro_rules! counters {
        ($prop:expr, $field:ident) => {
            match pick(d, $prop) {
                Pick::Inherit => g.$field = parent.$field.clone(),
                Pick::Initial => {}
                Pick::Value(Value::Counters(c)) => g.$field = c.clone(),
                Pick::Value(_) => {}
            }
        };
    }
    counters!(PropId::CounterReset, counter_reset);
    counters!(PropId::CounterIncrement, counter_increment);
    counters!(PropId::CounterSet, counter_set);
    Rc::new(g)
}

/// The flex properties (not inherited: only `inherit` copies the value of the parent).
pub fn compute_flex(parent: &Rc<FlexStyle>, init_rc: &Rc<FlexStyle>, d: &Declared<'_>, ctx: &LenCtx) -> Rc<FlexStyle> {
    if !d.touches(FLEX_BITS) {
        return init_rc.clone();
    }
    let mut f = FlexStyle::initial();
    macro_rules! simple {
        ($prop:expr, $variant:ident, $field:ident) => {
            match pick(d, $prop) {
                Pick::Inherit => f.$field = parent.$field.clone(),
                Pick::Initial => {}
                Pick::Value(Value::$variant(v)) => f.$field = v.clone(),
                Pick::Value(_) => {}
            }
        };
    }
    simple!(PropId::FlexDirection, FlexDirection, direction);
    simple!(PropId::FlexWrap, FlexWrap, wrap);
    simple!(PropId::Order, Order, order);
    simple!(PropId::FlexGrow, FlexFactor, grow);
    simple!(PropId::FlexShrink, FlexFactor, shrink);
    simple!(PropId::JustifyContent, Align, justify_content);
    simple!(PropId::AlignItems, Align, align_items);
    simple!(PropId::AlignSelf, Align, align_self);
    simple!(PropId::AlignContent, Align, align_content);
    simple!(PropId::JustifyItems, Align, justify_items);
    simple!(PropId::JustifySelf, Align, justify_self);
    match pick(d, PropId::FlexBasis) {
        Pick::Inherit => f.basis = parent.basis.clone(),
        Pick::Initial => {}
        Pick::Value(Value::FlexBasis(FlexBasis::Size(s))) => f.basis = FlexBasis::Size(s.compute(ctx)),
        Pick::Value(Value::FlexBasis(b)) => f.basis = b.clone(),
        Pick::Value(_) => {}
    }
    match pick(d, PropId::RowGap) {
        Pick::Inherit => f.row_gap = parent.row_gap.clone(),
        Pick::Initial => {}
        Pick::Value(Value::Gap(g)) => f.row_gap = g.as_ref().map(|l| l.compute(ctx)),
        Pick::Value(_) => {}
    }
    match pick(d, PropId::ColumnGap) {
        Pick::Inherit => f.column_gap = parent.column_gap.clone(),
        Pick::Initial => {}
        Pick::Value(Value::Gap(g)) => f.column_gap = g.as_ref().map(|l| l.compute(ctx)),
        Pick::Value(_) => {}
    }
    Rc::new(f)
}

/// The grid properties (not inherited: only `inherit` copies the value of the parent).
pub fn compute_grid(parent: &Rc<GridStyle>, init_rc: &Rc<GridStyle>, d: &Declared<'_>, ctx: &LenCtx) -> Rc<GridStyle> {
    if !d.touches(GRID_BITS) {
        return init_rc.clone();
    }
    let mut g = GridStyle::initial();
    for (p, rows) in [(PropId::GridTemplateColumns, false), (PropId::GridTemplateRows, true)] {
        let dst = if rows { &mut g.rows } else { &mut g.cols };
        match pick(d, p) {
            Pick::Inherit => *dst = if rows { parent.rows.clone() } else { parent.cols.clone() },
            Pick::Value(Value::Tracks(t)) => *dst = t.as_ref().map(|l| Rc::new(l.compute(ctx))),
            _ => {}
        }
    }
    match pick(d, PropId::GridTemplateAreas) {
        Pick::Inherit => g.areas = parent.areas.clone(),
        Pick::Value(Value::Areas(a)) => g.areas = a.clone(),
        _ => {}
    }
    for (p, rows) in [(PropId::GridAutoColumns, false), (PropId::GridAutoRows, true)] {
        let dst = if rows { &mut g.auto_rows } else { &mut g.auto_cols };
        match pick(d, p) {
            Pick::Inherit => *dst = if rows { parent.auto_rows.clone() } else { parent.auto_cols.clone() },
            Pick::Value(Value::AutoTracks(t)) => *dst = Rc::new(t.iter().map(|x| x.compute(ctx)).collect()),
            _ => {}
        }
    }
    match pick(d, PropId::GridAutoFlow) {
        Pick::Inherit => (g.flow_col, g.dense) = (parent.flow_col, parent.dense),
        Pick::Value(Value::GridFlow(c, dn)) => (g.flow_col, g.dense) = (*c, *dn),
        _ => {}
    }
    for (i, p) in [PropId::GridRowStart, PropId::GridColumnStart, PropId::GridRowEnd, PropId::GridColumnEnd].into_iter().enumerate() {
        let Some(dst) = g.line.get_mut(i) else { continue };
        match pick(d, p) {
            Pick::Inherit => *dst = parent.line.get(i).cloned().unwrap_or_default(),
            Pick::Value(Value::GridLine(l)) => *dst = l.clone(),
            _ => {}
        }
    }
    Rc::new(g)
}

// -----------------------------------------------------------------------------------------------
// Serialization (for `qinglan-cli style` and tests)

fn px(v: f32) -> String {
    format!("{}px", fmt_num(v))
}

fn colors(c: ColorValue) -> String {
    c.to_css()
}

impl ComputedStyle {
    /// All properties as (name, serialized computed value), in the order of [`PropId`].
    pub fn properties(&self) -> Vec<(&'static str, String)> {
        let b = &self.boxs;
        let br = &self.border;
        let f = &self.font;
        let t = &self.text;
        let m = &self.misc;
        let mut v: Vec<(&'static str, String)> = Vec::new();
        let mut add = |n: PropId, s: String| v.push((n.name(), s));
        add(PropId::Display, b.display.keyword().into());
        add(PropId::Position, b.position.keyword().into());
        add(PropId::Float, b.float.keyword().into());
        add(PropId::Clear, b.clear.keyword().into());
        add(PropId::BoxSizing, b.box_sizing.keyword().into());
        add(PropId::ZIndex, b.z_index.map_or("auto".into(), |z| z.to_string()));
        add(PropId::OverflowX, b.overflow_x.keyword().into());
        add(PropId::OverflowY, b.overflow_y.keyword().into());
        for (i, p) in [PropId::Top, PropId::Right, PropId::Bottom, PropId::Left].into_iter().enumerate() {
            add(p, b.inset.get(i).map_or(String::new(), Dim::to_css));
        }
        add(PropId::Width, b.width.to_css());
        add(PropId::Height, b.height.to_css());
        add(PropId::MinWidth, b.min_width.to_css());
        add(PropId::MinHeight, b.min_height.to_css());
        add(PropId::MaxWidth, b.max_width.to_css());
        add(PropId::MaxHeight, b.max_height.to_css());
        for (i, p) in [PropId::MarginTop, PropId::MarginRight, PropId::MarginBottom, PropId::MarginLeft].into_iter().enumerate() {
            add(p, b.margin.get(i).map_or(String::new(), Dim::to_css));
        }
        for (i, p) in [PropId::PaddingTop, PropId::PaddingRight, PropId::PaddingBottom, PropId::PaddingLeft].into_iter().enumerate() {
            add(p, b.padding.get(i).map_or(String::new(), Lp::to_css));
        }
        for (i, p) in [PropId::BorderTopWidth, PropId::BorderRightWidth, PropId::BorderBottomWidth, PropId::BorderLeftWidth]
            .into_iter()
            .enumerate()
        {
            add(p, px(br.width.get(i).copied().unwrap_or(0.0)));
        }
        for (i, p) in [PropId::BorderTopStyle, PropId::BorderRightStyle, PropId::BorderBottomStyle, PropId::BorderLeftStyle]
            .into_iter()
            .enumerate()
        {
            add(p, br.style.get(i).map_or(String::new(), |s| s.keyword().into()));
        }
        for (i, p) in [PropId::BorderTopColor, PropId::BorderRightColor, PropId::BorderBottomColor, PropId::BorderLeftColor]
            .into_iter()
            .enumerate()
        {
            add(p, br.color.get(i).map_or(String::new(), |c| colors(*c)));
        }
        add(PropId::FontFamily, f.family.iter().map(FontFamily::to_css).collect::<Vec<_>>().join(", "));
        add(PropId::FontSize, px(f.size));
        add(PropId::FontWeight, fmt_num(f.weight));
        add(PropId::FontStyle, f.style.keyword().into());
        add(
            PropId::LineHeight,
            match f.line_height {
                LineHeight::Normal => "normal".into(),
                LineHeight::Number(n) => fmt_num(n),
                LineHeight::Px(p) => px(p),
            },
        );
        add(PropId::LetterSpacing, px(f.letter_spacing));
        add(PropId::WordSpacing, px(f.word_spacing));
        add(PropId::Color, t.color.to_css());
        add(PropId::TextAlign, t.text_align.keyword().into());
        add(PropId::TextIndent, t.text_indent.to_css());
        add(PropId::TextTransform, t.text_transform.keyword().into());
        add(PropId::WhiteSpaceCollapse, t.white_space_collapse.keyword().into());
        add(PropId::TextWrapMode, t.text_wrap_mode.keyword().into());
        add(PropId::WordBreak, t.word_break.keyword().into());
        add(PropId::OverflowWrap, t.overflow_wrap.keyword().into());
        add(PropId::Direction, t.direction.keyword().into());
        add(PropId::Visibility, t.visibility.keyword().into());
        add(PropId::Cursor, t.cursor.keyword().into());
        add(PropId::ListStyleType, t.list_style_type.to_css());
        add(PropId::ListStylePosition, t.list_style_position.keyword().into());
        add(PropId::TabSize, t.tab_size.to_css());
        add(PropId::VerticalAlign, m.vertical_align.to_css());
        add(PropId::TextDecorationLine, m.text_decoration_line.to_css());
        add(PropId::TextDecorationStyle, m.text_decoration_style.keyword().into());
        add(PropId::TextDecorationColor, colors(m.text_decoration_color));
        add(PropId::TextDecorationThickness, m.text_decoration_thickness.as_ref().map_or("auto".into(), Lp::to_css));
        add(PropId::BackgroundColor, colors(m.background_color));
        let list = |it: Vec<String>| it.join(", ");
        let radii = m.radius.as_deref().cloned().unwrap_or_else(|| std::array::from_fn(|_| (Lp::ZERO, Lp::ZERO)));
        for (p, r) in [PropId::BorderTopLeftRadius, PropId::BorderTopRightRadius, PropId::BorderBottomRightRadius, PropId::BorderBottomLeftRadius].into_iter().zip(radii.iter()) {
            add(p, if r.0 == r.1 { r.0.to_css() } else { format!("{} {}", r.0.to_css(), r.1.to_css()) });
        }
        let bg = &m.bg;
        add(PropId::BackgroundImage, list(bg.images.iter().map(BgImage::to_css).collect()));
        add(PropId::BackgroundPositionX, list(bg.pos_x.iter().map(Pos1::to_css).collect()));
        add(PropId::BackgroundPositionY, list(bg.pos_y.iter().map(Pos1::to_css).collect()));
        add(PropId::BackgroundSize, list(bg.size.iter().map(BgSize::to_css).collect()));
        add(PropId::BackgroundRepeat, list(bg.repeat.iter().map(|r| r.to_css()).collect()));
        add(PropId::BackgroundOrigin, list(bg.origin.iter().map(|b| b.to_css().to_string()).collect()));
        add(PropId::BackgroundClip, list(bg.clip.iter().map(|b| b.to_css().to_string()).collect()));
        let o = &m.outline();
        add(PropId::OutlineWidth, px(o.width));
        add(PropId::OutlineStyle, o.style.keyword().into());
        add(PropId::OutlineColor, colors(o.color));
        add(PropId::OutlineOffset, px(o.offset));
        let g = &self.gen_;
        add(PropId::Content, g.content.to_css());
        add(PropId::Quotes, t.quotes.to_css());
        add(PropId::CounterReset, counters_to_css(&g.counter_reset));
        add(PropId::CounterIncrement, counters_to_css(&g.counter_increment));
        add(PropId::CounterSet, counters_to_css(&g.counter_set));
        add(PropId::BorderCollapse, t.border_collapse.keyword().into());
        add(PropId::BorderSpacing, format!("{} {}", t.border_spacing[0].to_css(), t.border_spacing[1].to_css()));
        add(PropId::EmptyCells, t.empty_cells.keyword().into());
        add(PropId::CaptionSide, t.caption_side.keyword().into());
        add(PropId::TableLayout, b.table_layout.keyword().into());
        let fl = &self.flex;
        add(PropId::FlexDirection, fl.direction.keyword().into());
        add(PropId::FlexWrap, fl.wrap.keyword().into());
        add(PropId::Order, fl.order.to_string());
        add(PropId::FlexGrow, fmt_num(fl.grow));
        add(PropId::FlexShrink, fmt_num(fl.shrink));
        add(PropId::FlexBasis, fl.basis.to_css());
        add(PropId::JustifyContent, fl.justify_content.to_css());
        add(PropId::AlignItems, fl.align_items.to_css());
        add(PropId::AlignSelf, fl.align_self.to_css());
        add(PropId::AlignContent, fl.align_content.to_css());
        add(PropId::RowGap, fl.row_gap.as_ref().map_or("normal".into(), Lp::to_css));
        add(PropId::ColumnGap, fl.column_gap.as_ref().map_or("normal".into(), Lp::to_css));
        add(PropId::JustifyItems, fl.justify_items.to_css());
        add(PropId::JustifySelf, fl.justify_self.to_css());
        let gr = &self.grid;
        let tracks = |t: &Option<Rc<TrackList>>| t.as_ref().map_or("none".to_string(), |l| l.to_css());
        add(PropId::GridTemplateColumns, tracks(&gr.cols));
        add(PropId::GridTemplateRows, tracks(&gr.rows));
        let areas = gr.areas.as_ref().map_or("none".to_string(), |a| {
            (1..=a.rows)
                .map(|r| {
                    let row: Vec<&str> = (1..=a.cols).map(|c| a.list.iter().find(|x| r >= x.1 && r < x.3 && c >= x.2 && c < x.4).map_or(".", |x| &*x.0)).collect();
                    format!("\"{}\"", row.join(" "))
                })
                .collect::<Vec<_>>()
                .join(" ")
        });
        add(PropId::GridTemplateAreas, areas);
        let auto = |t: &Rc<Vec<Track>>| t.iter().map(Track::to_css).collect::<Vec<_>>().join(" ");
        add(PropId::GridAutoColumns, auto(&gr.auto_cols));
        add(PropId::GridAutoRows, auto(&gr.auto_rows));
        add(PropId::GridAutoFlow, format!("{}{}", if gr.flow_col { "column" } else { "row" }, if gr.dense { " dense" } else { "" }));
        for (p, l) in [PropId::GridRowStart, PropId::GridColumnStart, PropId::GridRowEnd, PropId::GridColumnEnd].into_iter().zip(gr.line.iter()) {
            add(p, l.to_css());
        }
        v.push(("lang", t.lang.as_deref().unwrap_or("").to_string()));
        v
    }
}

#[allow(dead_code)]
fn _keep(_: LenUnit) {}
