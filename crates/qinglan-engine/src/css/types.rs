//! Property identifiers, keyword types and value types of the properties layer 1 computes.

use std::rc::Rc;

use super::color::ColorValue;
use super::syntax::Cv;
use super::values::{Calc, LenUnit, Lp};
use crate::html::atoms::Atom;

macro_rules! keyword_enum {
    ($(#[$m:meta])* $name:ident { $($var:ident = $kw:literal),+ $(,)? } default $def:ident) => {
        $(#[$m])*
        #[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
        pub enum $name { $($var),+ }
        impl Default for $name {
            fn default() -> Self { $name::$def }
        }
        impl $name {
            pub fn from_keyword(s: &str) -> Option<Self> {
                $(if s.eq_ignore_ascii_case($kw) { return Some($name::$var); })+
                None
            }
            pub fn keyword(self) -> &'static str {
                match self { $($name::$var => $kw),+ }
            }
        }
    };
}


keyword_enum!(Display {
    None = "none", Contents = "contents", Block = "block", Inline = "inline", InlineBlock = "inline-block",
    ListItem = "list-item", FlowRoot = "flow-root", Table = "table", InlineTable = "inline-table",
    TableRowGroup = "table-row-group", TableHeaderGroup = "table-header-group", TableFooterGroup = "table-footer-group",
    TableRow = "table-row", TableColumnGroup = "table-column-group", TableColumn = "table-column",
    TableCell = "table-cell", TableCaption = "table-caption", Flex = "flex", InlineFlex = "inline-flex",
    Grid = "grid", InlineGrid = "inline-grid", Ruby = "ruby", RubyText = "ruby-text",
} default Inline);

keyword_enum!(Position {
    Static = "static", Relative = "relative", Absolute = "absolute", Fixed = "fixed", Sticky = "sticky",
} default Static);

keyword_enum!(Float { None = "none", Left = "left", Right = "right" } default None);
keyword_enum!(Clear { None = "none", Left = "left", Right = "right", Both = "both" } default None);
keyword_enum!(BoxSizing { ContentBox = "content-box", BorderBox = "border-box" } default ContentBox);
keyword_enum!(Overflow { Visible = "visible", Hidden = "hidden", Clip = "clip", Scroll = "scroll", Auto = "auto" } default Visible);
keyword_enum!(Visibility { Visible = "visible", Hidden = "hidden", Collapse = "collapse" } default Visible);
keyword_enum!(TextAlign {
    Start = "start", End = "end", Left = "left", Right = "right", Center = "center", Justify = "justify",
    MatchParent = "match-parent", JustifyAll = "justify-all",
} default Start);
keyword_enum!(TextTransform {
    None = "none", Capitalize = "capitalize", Uppercase = "uppercase", Lowercase = "lowercase",
    FullWidth = "full-width", FullSizeKana = "full-size-kana",
} default None);
keyword_enum!(WhiteSpaceCollapse {
    Collapse = "collapse", Preserve = "preserve", PreserveBreaks = "preserve-breaks", PreserveSpaces = "preserve-spaces",
    BreakSpaces = "break-spaces",
} default Collapse);
keyword_enum!(TextWrapMode { Wrap = "wrap", Nowrap = "nowrap" } default Wrap);
keyword_enum!(WordBreak { Normal = "normal", BreakAll = "break-all", KeepAll = "keep-all", BreakWord = "break-word" } default Normal);
keyword_enum!(OverflowWrap { Normal = "normal", Anywhere = "anywhere", BreakWord = "break-word" } default Normal);
keyword_enum!(Direction { Ltr = "ltr", Rtl = "rtl" } default Ltr);
keyword_enum!(FontStyleKind { Normal = "normal", Italic = "italic", Oblique = "oblique" } default Normal);
keyword_enum!(ListStylePosition { Outside = "outside", Inside = "inside" } default Outside);
keyword_enum!(BorderCollapse { Separate = "separate", Collapse = "collapse" } default Separate);
keyword_enum!(EmptyCells { Show = "show", Hide = "hide" } default Show);
keyword_enum!(CaptionSide { Top = "top", Bottom = "bottom" } default Top);
keyword_enum!(TableLayout { Auto = "auto", Fixed = "fixed" } default Auto);
keyword_enum!(FlexDirection { Row = "row", RowReverse = "row-reverse", Column = "column", ColumnReverse = "column-reverse" } default Row);
keyword_enum!(FlexWrap { Nowrap = "nowrap", Wrap = "wrap", WrapReverse = "wrap-reverse" } default Nowrap);
keyword_enum!(AlignKind {
    Auto = "auto", Normal = "normal", Stretch = "stretch", FlexStart = "flex-start", FlexEnd = "flex-end", Start = "start",
    End = "end", SelfStart = "self-start", SelfEnd = "self-end", Center = "center", Left = "left", Right = "right",
    Baseline = "baseline", LastBaseline = "last baseline", SpaceBetween = "space-between", SpaceAround = "space-around",
    SpaceEvenly = "space-evenly",
} default Normal);

/// A value of `justify-content`, `align-content`, `align-items` or `align-self` (css-align-3): the position or
/// distribution keyword and the `safe` / `unsafe` overflow keyword in front of it (`Some(true)` is `safe`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct AlignValue {
    pub kind: AlignKind,
    pub safe: Option<bool>,
}

impl AlignValue {
    pub const NORMAL: AlignValue = AlignValue { kind: AlignKind::Normal, safe: None };
    pub const AUTO: AlignValue = AlignValue { kind: AlignKind::Auto, safe: None };

    pub fn to_css(self) -> String {
        let k = self.kind.keyword();
        match self.safe {
            Some(true) => format!("safe {k}"),
            Some(false) => format!("unsafe {k}"),
            None => k.into(),
        }
    }
}

/// `flex-basis` (css-flexbox-1 7.3): `auto`, `content`, or a size.
#[derive(Clone, Debug, PartialEq)]
pub enum FlexBasis {
    Auto,
    Content,
    Size(Dim),
}

impl FlexBasis {
    pub fn to_css(&self) -> String {
        match self {
            FlexBasis::Auto => "auto".into(),
            FlexBasis::Content => "content".into(),
            FlexBasis::Size(d) => d.to_css(),
        }
    }
}

/// `tab-size` (css-text-3 7.3): a number of space advances, or a length.
#[derive(Clone, Debug, PartialEq)]
pub enum TabSize {
    Spaces(f32),
    Len(Lp),
}

impl TabSize {
    pub fn to_css(&self) -> String {
        match self {
            TabSize::Spaces(n) => super::values::fmt_num(*n),
            TabSize::Len(l) => l.to_css(),
        }
    }
}
keyword_enum!(BorderLineStyle {
    None = "none", Hidden = "hidden", Dotted = "dotted", Dashed = "dashed", Solid = "solid", Double = "double",
    Groove = "groove", Ridge = "ridge", Inset = "inset", Outset = "outset",
} default None);
keyword_enum!(TextDecorationStyle {
    Solid = "solid", Double = "double", Dotted = "dotted", Dashed = "dashed", Wavy = "wavy",
} default Solid);
keyword_enum!(CursorKind {
    Auto = "auto", Default = "default", None = "none", ContextMenu = "context-menu", Help = "help", Pointer = "pointer",
    Progress = "progress", Wait = "wait", Cell = "cell", Crosshair = "crosshair", Text = "text", VerticalText = "vertical-text",
    Alias = "alias", Copy = "copy", Move = "move", NoDrop = "no-drop", NotAllowed = "not-allowed", Grab = "grab",
    Grabbing = "grabbing", AllScroll = "all-scroll", ColResize = "col-resize", RowResize = "row-resize",
    NResize = "n-resize", EResize = "e-resize", SResize = "s-resize", WResize = "w-resize", NeResize = "ne-resize",
    NwResize = "nw-resize", SeResize = "se-resize", SwResize = "sw-resize", EwResize = "ew-resize", NsResize = "ns-resize",
    NeswResize = "nesw-resize", NwseResize = "nwse-resize", ZoomIn = "zoom-in", ZoomOut = "zoom-out",
} default Auto);
keyword_enum!(GenericFamily {
    Serif = "serif", SansSerif = "sans-serif", Monospace = "monospace", Cursive = "cursive", Fantasy = "fantasy",
    SystemUi = "system-ui", UiSerif = "ui-serif", UiSansSerif = "ui-sans-serif", UiMonospace = "ui-monospace",
    UiRounded = "ui-rounded", Emoji = "emoji", Math = "math", Fangsong = "fangsong",
} default Serif);

/// An entry of a `font-family` list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FontFamily {
    Named(Rc<str>),
    Generic(GenericFamily),
}

impl FontFamily {
    pub fn to_css(&self) -> String {
        match self {
            FontFamily::Named(n) => {
                let mut s = String::from("\"");
                for c in n.chars() {
                    if c == '"' || c == '\\' {
                        s.push('\\');
                    }
                    s.push(c);
                }
                s.push('"');
                s
            }
            FontFamily::Generic(g) => g.keyword().to_string(),
        }
    }
}

/// `list-style-type`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ListStyleType {
    None,
    Disc,
    Circle,
    Square,
    Decimal,
    DecimalLeadingZero,
    LowerAlpha,
    UpperAlpha,
    LowerRoman,
    UpperRoman,
    LowerGreek,
    CjkDecimal,
    /// Any other counter style name (rendered with the closest known style by the layout code).
    Other(Rc<str>),
    /// A string marker.
    Str(Rc<str>),
}

impl ListStyleType {
    pub fn from_keyword(k: &str) -> Option<ListStyleType> {
        let l = k.to_ascii_lowercase();
        Some(match l.as_str() {
            "none" => ListStyleType::None,
            "disc" => ListStyleType::Disc,
            "circle" => ListStyleType::Circle,
            "square" => ListStyleType::Square,
            "decimal" => ListStyleType::Decimal,
            "decimal-leading-zero" => ListStyleType::DecimalLeadingZero,
            "lower-alpha" | "lower-latin" => ListStyleType::LowerAlpha,
            "upper-alpha" | "upper-latin" => ListStyleType::UpperAlpha,
            "lower-roman" => ListStyleType::LowerRoman,
            "upper-roman" => ListStyleType::UpperRoman,
            "lower-greek" => ListStyleType::LowerGreek,
            "cjk-decimal" => ListStyleType::CjkDecimal,
            "inherit" | "initial" | "unset" | "revert" | "revert-layer" | "default" => return None,
            _ => ListStyleType::Other(Rc::from(l.as_str())),
        })
    }

    pub fn to_css(&self) -> String {
        match self {
            ListStyleType::None => "none".into(),
            ListStyleType::Disc => "disc".into(),
            ListStyleType::Circle => "circle".into(),
            ListStyleType::Square => "square".into(),
            ListStyleType::Decimal => "decimal".into(),
            ListStyleType::DecimalLeadingZero => "decimal-leading-zero".into(),
            ListStyleType::LowerAlpha => "lower-alpha".into(),
            ListStyleType::UpperAlpha => "upper-alpha".into(),
            ListStyleType::LowerRoman => "lower-roman".into(),
            ListStyleType::UpperRoman => "upper-roman".into(),
            ListStyleType::LowerGreek => "lower-greek".into(),
            ListStyleType::CjkDecimal => "cjk-decimal".into(),
            ListStyleType::Other(n) => n.to_string(),
            ListStyleType::Str(s) => format!("\"{s}\""),
        }
    }
}

/// Lengths for box properties, with the keywords they accept.
#[derive(Clone, Debug, PartialEq)]
pub enum Dim {
    Auto,
    None,
    MinContent,
    MaxContent,
    FitContent,
    Lp(Lp),
}

impl Dim {
    pub const ZERO: Dim = Dim::Lp(Lp::ZERO);

    pub fn compute(&self, ctx: &super::values::LenCtx) -> Dim {
        match self {
            Dim::Lp(l) => Dim::Lp(l.compute(ctx)),
            other => other.clone(),
        }
    }

    pub fn to_css(&self) -> String {
        match self {
            Dim::Auto => "auto".into(),
            Dim::None => "none".into(),
            Dim::MinContent => "min-content".into(),
            Dim::MaxContent => "max-content".into(),
            Dim::FitContent => "fit-content".into(),
            Dim::Lp(l) => l.to_css(),
        }
    }
}

/// One item of a `content` value (CSS2 12.2).
#[derive(Clone, Debug, PartialEq)]
pub enum ContentItem {
    Str(Rc<str>),
    /// `attr(name)`: the value of that attribute of the element the pseudo-element belongs to.
    Attr(Rc<str>),
    /// `counter(name, style)`.
    Counter(Rc<str>, ListStyleType),
    /// `counters(name, separator, style)`.
    Counters(Rc<str>, Rc<str>, ListStyleType),
    OpenQuote,
    CloseQuote,
    NoOpenQuote,
    NoCloseQuote,
    /// `url(...)`, as written (resolved against the page when the image is asked for).
    Url(Rc<str>),
}

/// `content` (CSS2 12.2): `normal` and `none` make no pseudo-element box (on `::marker`, `normal` is the usual marker).
#[derive(Clone, Debug, PartialEq)]
pub enum Content {
    Normal,
    None,
    Items(Rc<Vec<ContentItem>>),
}

/// `quotes` (CSS2 12.3.1): `auto` is picked by language.
#[derive(Clone, Debug, PartialEq)]
pub enum Quotes {
    Auto,
    None,
    Pairs(Rc<Vec<(Rc<str>, Rc<str>)>>),
}

/// Longest `content` list, `quotes` list and counter list taken from one declaration.
pub const MAX_GEN_ITEMS: usize = 64;

impl Content {
    pub fn to_css(&self) -> String {
        match self {
            Content::Normal => "normal".into(),
            Content::None => "none".into(),
            Content::Items(v) => v
                .iter()
                .map(|i| match i {
                    ContentItem::Str(s) => format!("\"{s}\""),
                    ContentItem::Attr(n) => format!("attr({n})"),
                    ContentItem::Counter(n, t) => format!("counter({n}, {})", t.to_css()),
                    ContentItem::Counters(n, sep, t) => format!("counters({n}, \"{sep}\", {})", t.to_css()),
                    ContentItem::OpenQuote => "open-quote".into(),
                    ContentItem::CloseQuote => "close-quote".into(),
                    ContentItem::NoOpenQuote => "no-open-quote".into(),
                    ContentItem::NoCloseQuote => "no-close-quote".into(),
                    ContentItem::Url(u) => format!("url({u})"),
                })
                .collect::<Vec<_>>()
                .join(" "),
        }
    }
}

impl Quotes {
    pub fn to_css(&self) -> String {
        match self {
            Quotes::Auto => "auto".into(),
            Quotes::None => "none".into(),
            Quotes::Pairs(p) => p.iter().map(|(a, b)| format!("\"{a}\" \"{b}\"")).collect::<Vec<_>>().join(" "),
        }
    }
}

/// `counter-reset` and friends as text.
pub fn counters_to_css(v: &[(Rc<str>, i32)]) -> String {
    if v.is_empty() {
        return "none".into();
    }
    v.iter().map(|(n, i)| format!("{n} {i}")).collect::<Vec<_>>().join(" ")
}

#[derive(Clone, Debug, PartialEq)]
pub enum FontSizeSpec {
    /// Absolute-size keyword index 0..=6 (xx-small .. xxx-large) or 7 (xxx-large).
    Absolute(u8),
    Larger,
    Smaller,
    Lp(Lp),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FontWeightSpec {
    Number(f32),
    Bolder,
    Lighter,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LineHeightSpec {
    Normal,
    Number(f32),
    Lp(Lp),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BorderWidthKw {
    Thin,
    Medium,
    Thick,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BorderWidthSpec {
    Kw(BorderWidthKw),
    Lp(Lp),
}

#[derive(Clone, Debug, PartialEq)]
pub enum VerticalAlign {
    Baseline,
    Sub,
    Super,
    TextTop,
    TextBottom,
    Middle,
    Top,
    Bottom,
    Lp(Lp),
}

impl VerticalAlign {
    pub fn to_css(&self) -> String {
        match self {
            VerticalAlign::Baseline => "baseline".into(),
            VerticalAlign::Sub => "sub".into(),
            VerticalAlign::Super => "super".into(),
            VerticalAlign::TextTop => "text-top".into(),
            VerticalAlign::TextBottom => "text-bottom".into(),
            VerticalAlign::Middle => "middle".into(),
            VerticalAlign::Top => "top".into(),
            VerticalAlign::Bottom => "bottom".into(),
            VerticalAlign::Lp(l) => l.to_css(),
        }
    }
}

/// `text-decoration-line` as flags.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct TextDecorationLine(pub u8);

impl TextDecorationLine {
    pub const NONE: TextDecorationLine = TextDecorationLine(0);
    pub const UNDERLINE: u8 = 1;
    pub const OVERLINE: u8 = 2;
    pub const LINE_THROUGH: u8 = 4;
    pub const BLINK: u8 = 8;

    pub fn has(self, flag: u8) -> bool {
        self.0 & flag != 0
    }

    pub fn to_css(self) -> String {
        if self.0 == 0 {
            return "none".into();
        }
        let mut v = Vec::new();
        if self.has(Self::UNDERLINE) {
            v.push("underline");
        }
        if self.has(Self::OVERLINE) {
            v.push("overline");
        }
        if self.has(Self::LINE_THROUGH) {
            v.push("line-through");
        }
        if self.has(Self::BLINK) {
            v.push("blink");
        }
        v.join(" ")
    }
}

/// A parsed (specified) property value.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Display(Display),
    Position(Position),
    Float(Float),
    Clear(Clear),
    BoxSizing(BoxSizing),
    Overflow(Overflow),
    Visibility(Visibility),
    ZIndex(Option<i32>),
    Dim(Dim),
    BorderWidth(BorderWidthSpec),
    BorderStyle(BorderLineStyle),
    Color(ColorValue),
    FontFamily(Rc<Vec<FontFamily>>),
    FontSize(FontSizeSpec),
    FontWeight(FontWeightSpec),
    FontStyle(FontStyleKind),
    LineHeight(LineHeightSpec),
    /// letter-spacing / word-spacing: `None` is `normal`.
    Spacing(Option<Lp>),
    TextAlign(TextAlign),
    TextIndent(Lp),
    TextTransform(TextTransform),
    WhiteSpaceCollapse(WhiteSpaceCollapse),
    TextWrapMode(TextWrapMode),
    WordBreak(WordBreak),
    OverflowWrap(OverflowWrap),
    Direction(Direction),
    VerticalAlign(VerticalAlign),
    TextDecorationLine(TextDecorationLine),
    TextDecorationStyle(TextDecorationStyle),
    TextDecorationThickness(Option<Lp>),
    ListStyleType(ListStyleType),
    ListStylePosition(ListStylePosition),
    BorderCollapse(BorderCollapse),
    EmptyCells(EmptyCells),
    CaptionSide(CaptionSide),
    TableLayout(TableLayout),
    FlexDirection(FlexDirection),
    FlexWrap(FlexWrap),
    /// `order`.
    Order(i32),
    /// `flex-grow` / `flex-shrink`.
    FlexFactor(f32),
    FlexBasis(FlexBasis),
    Align(AlignValue),
    /// `row-gap` / `column-gap`: `None` is `normal`.
    Gap(Option<Lp>),
    /// `border-spacing`: horizontal and vertical.
    TableSpacing(Lp, Lp),
    TabSize(TabSize),
    Cursor(CursorKind),
    Content(Content),
    Quotes(Quotes),
    /// `counter-reset` / `counter-increment` / `counter-set`: the counters named and their numbers.
    Counters(Rc<Vec<(Rc<str>, i32)>>),
    /// A property that is parsed and validated but has no effect in layer 1 (`border-radius`, ...).
    Ignored,
}

/// A declaration containing `var()`: substituted and parsed when the style of an element is computed.
#[derive(Debug, PartialEq)]
pub struct VarDecl {
    /// The property as written (may be a shorthand).
    pub prop: String,
    pub tokens: Vec<Cv>,
    pub id: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Specified {
    Inherit,
    Initial,
    Unset,
    Revert,
    RevertLayer,
    Value(Value),
    Var(Rc<VarDecl>),
}

/// Longhand property ids. The order is the order of the printed list of computed values.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum PropId {
    Display,
    Position,
    Float,
    Clear,
    BoxSizing,
    ZIndex,
    OverflowX,
    OverflowY,
    Top,
    Right,
    Bottom,
    Left,
    Width,
    Height,
    MinWidth,
    MinHeight,
    MaxWidth,
    MaxHeight,
    MarginTop,
    MarginRight,
    MarginBottom,
    MarginLeft,
    PaddingTop,
    PaddingRight,
    PaddingBottom,
    PaddingLeft,
    BorderTopWidth,
    BorderRightWidth,
    BorderBottomWidth,
    BorderLeftWidth,
    BorderTopStyle,
    BorderRightStyle,
    BorderBottomStyle,
    BorderLeftStyle,
    BorderTopColor,
    BorderRightColor,
    BorderBottomColor,
    BorderLeftColor,
    FontFamily,
    FontSize,
    FontWeight,
    FontStyle,
    LineHeight,
    LetterSpacing,
    WordSpacing,
    Color,
    TextAlign,
    TextIndent,
    TextTransform,
    WhiteSpaceCollapse,
    TextWrapMode,
    WordBreak,
    OverflowWrap,
    Direction,
    Visibility,
    Cursor,
    ListStyleType,
    ListStylePosition,
    TabSize,
    VerticalAlign,
    TextDecorationLine,
    TextDecorationStyle,
    TextDecorationColor,
    TextDecorationThickness,
    BackgroundColor,
    BorderTopLeftRadius,
    BorderTopRightRadius,
    BorderBottomRightRadius,
    BorderBottomLeftRadius,
    Content,
    Quotes,
    CounterReset,
    CounterIncrement,
    CounterSet,
    BorderCollapse,
    BorderSpacing,
    EmptyCells,
    CaptionSide,
    TableLayout,
    FlexDirection,
    FlexWrap,
    Order,
    FlexGrow,
    FlexShrink,
    FlexBasis,
    JustifyContent,
    AlignItems,
    AlignSelf,
    AlignContent,
    RowGap,
    ColumnGap,
}

pub const PROP_COUNT: usize = PropId::ColumnGap as usize + 1;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Group {
    Box,
    Border,
    Font,
    Text,
    Misc,
    /// Generated content: `content` and the counter properties (not inherited).
    Gen,
    /// css-flexbox-1 / css-align-3 properties of flex containers and items (not inherited).
    Flex,
    Ignored,
}

impl PropId {
    pub fn index(self) -> usize {
        self as usize
    }

    pub const fn bit(self) -> u128 {
        1u128 << (self as u32)
    }

    pub const fn group(self) -> Group {
        use PropId::*;
        match self {
            Display | Position | Float | Clear | BoxSizing | ZIndex | OverflowX | OverflowY | Top | Right | Bottom | Left | Width
            | Height | MinWidth | MinHeight | MaxWidth | MaxHeight | MarginTop | MarginRight | MarginBottom | MarginLeft
            | PaddingTop | PaddingRight | PaddingBottom | PaddingLeft | TableLayout => Group::Box,
            BorderTopWidth | BorderRightWidth | BorderBottomWidth | BorderLeftWidth | BorderTopStyle | BorderRightStyle
            | BorderBottomStyle | BorderLeftStyle | BorderTopColor | BorderRightColor | BorderBottomColor | BorderLeftColor => {
                Group::Border
            }
            FontFamily | FontSize | FontWeight | FontStyle | LineHeight | LetterSpacing | WordSpacing => Group::Font,
            Color | TextAlign | TextIndent | TextTransform | WhiteSpaceCollapse | TextWrapMode | WordBreak | OverflowWrap
            | Direction | Visibility | Cursor | ListStyleType | ListStylePosition | TabSize | Quotes | BorderCollapse | BorderSpacing
            | EmptyCells | CaptionSide => Group::Text,
            Content | CounterReset | CounterIncrement | CounterSet => Group::Gen,
            FlexDirection | FlexWrap | Order | FlexGrow | FlexShrink | FlexBasis | JustifyContent | AlignItems | AlignSelf
            | AlignContent | RowGap | ColumnGap => Group::Flex,
            VerticalAlign | TextDecorationLine | TextDecorationStyle | TextDecorationColor | TextDecorationThickness
            | BackgroundColor => Group::Misc,
            BorderTopLeftRadius | BorderTopRightRadius | BorderBottomRightRadius | BorderBottomLeftRadius => Group::Ignored,
        }
    }

    /// Inherited properties (css-cascade-5: `unset` behaves as `inherit` for these).
    pub fn inherited(self) -> bool {
        matches!(self.group(), Group::Font | Group::Text)
    }

    pub fn name(self) -> &'static str {
        use PropId::*;
        match self {
            Display => "display",
            Position => "position",
            Float => "float",
            Clear => "clear",
            BoxSizing => "box-sizing",
            ZIndex => "z-index",
            OverflowX => "overflow-x",
            OverflowY => "overflow-y",
            Top => "top",
            Right => "right",
            Bottom => "bottom",
            Left => "left",
            Width => "width",
            Height => "height",
            MinWidth => "min-width",
            MinHeight => "min-height",
            MaxWidth => "max-width",
            MaxHeight => "max-height",
            MarginTop => "margin-top",
            MarginRight => "margin-right",
            MarginBottom => "margin-bottom",
            MarginLeft => "margin-left",
            PaddingTop => "padding-top",
            PaddingRight => "padding-right",
            PaddingBottom => "padding-bottom",
            PaddingLeft => "padding-left",
            BorderTopWidth => "border-top-width",
            BorderRightWidth => "border-right-width",
            BorderBottomWidth => "border-bottom-width",
            BorderLeftWidth => "border-left-width",
            BorderTopStyle => "border-top-style",
            BorderRightStyle => "border-right-style",
            BorderBottomStyle => "border-bottom-style",
            BorderLeftStyle => "border-left-style",
            BorderTopColor => "border-top-color",
            BorderRightColor => "border-right-color",
            BorderBottomColor => "border-bottom-color",
            BorderLeftColor => "border-left-color",
            FontFamily => "font-family",
            FontSize => "font-size",
            FontWeight => "font-weight",
            FontStyle => "font-style",
            LineHeight => "line-height",
            LetterSpacing => "letter-spacing",
            WordSpacing => "word-spacing",
            Color => "color",
            TextAlign => "text-align",
            TextIndent => "text-indent",
            TextTransform => "text-transform",
            WhiteSpaceCollapse => "white-space-collapse",
            TextWrapMode => "text-wrap-mode",
            WordBreak => "word-break",
            OverflowWrap => "overflow-wrap",
            Direction => "direction",
            Visibility => "visibility",
            Cursor => "cursor",
            ListStyleType => "list-style-type",
            ListStylePosition => "list-style-position",
            TabSize => "tab-size",
            VerticalAlign => "vertical-align",
            TextDecorationLine => "text-decoration-line",
            TextDecorationStyle => "text-decoration-style",
            TextDecorationColor => "text-decoration-color",
            TextDecorationThickness => "text-decoration-thickness",
            BackgroundColor => "background-color",
            BorderTopLeftRadius => "border-top-left-radius",
            BorderTopRightRadius => "border-top-right-radius",
            BorderBottomRightRadius => "border-bottom-right-radius",
            BorderBottomLeftRadius => "border-bottom-left-radius",
            Content => "content",
            Quotes => "quotes",
            CounterReset => "counter-reset",
            CounterIncrement => "counter-increment",
            CounterSet => "counter-set",
            BorderCollapse => "border-collapse",
            BorderSpacing => "border-spacing",
            EmptyCells => "empty-cells",
            CaptionSide => "caption-side",
            TableLayout => "table-layout",
            FlexDirection => "flex-direction",
            FlexWrap => "flex-wrap",
            Order => "order",
            FlexGrow => "flex-grow",
            FlexShrink => "flex-shrink",
            FlexBasis => "flex-basis",
            JustifyContent => "justify-content",
            AlignItems => "align-items",
            AlignSelf => "align-self",
            AlignContent => "align-content",
            RowGap => "row-gap",
            ColumnGap => "column-gap",
        }
    }

    pub fn from_index(i: usize) -> Option<PropId> {
        ALL_PROPS.get(i).copied()
    }
}

/// Every longhand, in declaration order.
pub const ALL_PROPS: [PropId; PROP_COUNT] = {
    use PropId::*;
    [
        Display, Position, Float, Clear, BoxSizing, ZIndex, OverflowX, OverflowY, Top, Right, Bottom, Left, Width, Height,
        MinWidth, MinHeight, MaxWidth, MaxHeight, MarginTop, MarginRight, MarginBottom, MarginLeft, PaddingTop, PaddingRight,
        PaddingBottom, PaddingLeft, BorderTopWidth, BorderRightWidth, BorderBottomWidth, BorderLeftWidth, BorderTopStyle,
        BorderRightStyle, BorderBottomStyle, BorderLeftStyle, BorderTopColor, BorderRightColor, BorderBottomColor,
        BorderLeftColor, FontFamily, FontSize, FontWeight, FontStyle, LineHeight, LetterSpacing, WordSpacing, Color, TextAlign,
        TextIndent, TextTransform, WhiteSpaceCollapse, TextWrapMode, WordBreak, OverflowWrap, Direction, Visibility, Cursor,
        ListStyleType, ListStylePosition, TabSize, VerticalAlign, TextDecorationLine, TextDecorationStyle, TextDecorationColor,
        TextDecorationThickness, BackgroundColor, BorderTopLeftRadius, BorderTopRightRadius, BorderBottomRightRadius,
        BorderBottomLeftRadius, Content, Quotes, CounterReset, CounterIncrement, CounterSet, BorderCollapse, BorderSpacing,
        EmptyCells, CaptionSide, TableLayout, FlexDirection, FlexWrap, Order, FlexGrow, FlexShrink, FlexBasis, JustifyContent,
        AlignItems, AlignSelf, AlignContent, RowGap, ColumnGap,
    ]
};

/// Bit mask of all properties of a group.
#[allow(clippy::indexing_slicing)] // const fn: `i < PROP_COUNT` is checked by the loop
pub const fn group_bits(g: Group) -> u128 {
    let mut m = 0u128;
    let mut i = 0;
    while i < PROP_COUNT {
        let p = ALL_PROPS[i];
        if p.group() as u8 == g as u8 {
            m |= p.bit();
        }
        i += 1;
    }
    m
}

pub const BOX_BITS: u128 = group_bits(Group::Box);
pub const BORDER_BITS: u128 = group_bits(Group::Border);
pub const FONT_BITS: u128 = group_bits(Group::Font);
pub const TEXT_BITS: u128 = group_bits(Group::Text);
pub const MISC_BITS: u128 = group_bits(Group::Misc);
pub const GEN_BITS: u128 = group_bits(Group::Gen);
pub const FLEX_BITS: u128 = group_bits(Group::Flex);

/// One declaration of a declaration block: a longhand (or a custom property) with its value.
#[derive(Clone, Debug, PartialEq)]
pub struct Declaration {
    pub prop: DeclProp,
    pub value: Specified,
    pub important: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DeclProp {
    Known(PropId),
    /// A custom property (`--x`), its value tokens kept as written.
    Custom(Atom, Rc<Vec<Cv>>),
}

/// The declarations of one rule or style attribute.
#[derive(Debug, Default)]
pub struct DeclBlock {
    pub decls: Vec<Declaration>,
    pub id: u32,
}

#[allow(dead_code)]
fn _unused(_: &Calc, _: LenUnit) {}
