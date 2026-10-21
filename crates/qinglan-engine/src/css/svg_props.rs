//! The SVG-only presentation properties (SVG 2 painting 13, text 11, masking 14): `fill`, `stroke` and their relatives.
//!
//! The cascade has a fixed bitmap of known properties (`PropId`, 128 bits, 119 used), so these are not added to it. They are
//! kept as entries of the custom-property map under their own names (which never start with `--`, so no `var()` can reach
//! them): the cascade then gives them the origin, specificity, `!important` and inheritance order of any other property,
//! and `svg::build` reads them as text. They are only parsed when the document has SVG in it.

pub const NAMES: [&str; 23] = [
    "fill", "stroke", "fill-opacity", "stroke-opacity", "fill-rule", "clip-rule", "stroke-width", "stroke-linecap", "stroke-linejoin",
    "stroke-miterlimit", "stroke-dasharray", "stroke-dashoffset", "text-anchor", "clip-path", "stop-color", "stop-opacity",
    // SVG 2 geometry properties (geometry 10): `rx: 40px` in a style sheet is the same as the attribute.
    "cx", "cy", "r", "rx", "ry", "x", "y",
];

/// Is `name` one of [`NAMES`] (property names are ASCII case-insensitive)?
pub fn is_svg_prop(name: &str) -> bool {
    matches!(name.as_bytes().first(), Some(b'f' | b'F' | b's' | b'S' | b'c' | b'C' | b't' | b'T' | b'r' | b'R' | b'x' | b'X' | b'y' | b'Y')) && NAMES.iter().any(|n| n.eq_ignore_ascii_case(name))
}

/// The presentation attributes of SVG elements that are ordinary CSS properties.
pub fn is_css_attr(name: &str) -> bool {
    matches!(name, "opacity" | "display" | "visibility" | "color" | "font-family" | "font-size" | "font-style" | "font-weight" | "overflow" | "direction")
}
