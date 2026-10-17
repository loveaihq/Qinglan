//! Images as layout sees them: the source picked from `srcset`/`sizes` (HTML 4.8.4.3) and the size
//! feed the host (step 1c) fills in as image headers arrive.
//!
//! Protocol for 1c:
//!
//! 1. `layout_document_with(.., &sizes)` lays the page out with whatever [`ImageSizes`] knows. An `<img>`
//!    with no entry is *pending*: 0x0 (or its alt text) unless the page gave it a size.
//! 2. `LayoutResult::images` lists, per `<img>` / `<input type=image>` that has a source, the node, the
//!    source text exactly as written in the page (relative to the document's base URL, which 1c resolves)
//!    and the pixel density of that source (`1x`, `2x`, or width descriptor / `sizes`).
//! 3. When the header of an image arrives, 1c calls [`ImageSizes::set_loaded`] with the pixel size (or
//!    [`ImageSizes::set_broken`]) and lays the page out again. The natural size of the image in CSS px is
//!    the pixel size divided by the density of the chosen source.
//!
//! Images are keyed by DOM node, because box ids change between layouts.

use crate::css::media::{MediaEnv, MediaQueryList};
use crate::css::syntax::{parse_cvs, split_commas, trim_ws};
use crate::css::values::{LenCtx, parse_length};
use crate::html::atoms::FxHashMap;
use crate::html::dom::{Document, NodeId};

/// What the host knows about one image.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageState {
    /// Decoded size in image pixels.
    Loaded { width: u32, height: u32 },
    /// Could not be fetched or decoded.
    Broken,
}

/// The image sizes the host has learned so far (the "image-size callback" is `set_loaded`).
#[derive(Clone, Debug, Default)]
pub struct ImageSizes {
    map: FxHashMap<NodeId, ImageState>,
}

impl ImageSizes {
    pub fn new() -> ImageSizes {
        ImageSizes::default()
    }

    /// Record the pixel size of the image of `node`. Returns true if this changes what layout knows.
    pub fn set_loaded(&mut self, node: NodeId, width: u32, height: u32) -> bool {
        self.map.insert(node, ImageState::Loaded { width, height }) != Some(ImageState::Loaded { width, height })
    }

    /// Record that the image of `node` cannot be shown.
    pub fn set_broken(&mut self, node: NodeId) -> bool {
        self.map.insert(node, ImageState::Broken) != Some(ImageState::Broken)
    }

    pub fn get(&self, node: NodeId) -> Option<ImageState> {
        self.map.get(&node).copied()
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

/// One image the page wants: the chosen source of an `<img>`.
#[derive(Clone, Debug, PartialEq)]
pub struct ImageRequest {
    pub node: NodeId,
    /// The source as written in the page (not resolved against the base URL).
    pub src: String,
    /// Image pixels per CSS px of the chosen source.
    pub density: f32,
}

/// What source selection needs to know about the device.
#[derive(Clone, Copy, Debug)]
pub struct SelectEnv {
    pub viewport_w: f32,
    pub viewport_h: f32,
    /// Device pixels per CSS px.
    pub dpr: f32,
}

/// Most candidates considered in one `srcset` (hostile pages).
const MAX_CANDIDATES: usize = 256;

struct Candidate {
    url: String,
    w: Option<u32>,
    x: Option<f32>,
}

/// HTML 4.8.4.3 "parse a srcset attribute", simplified: URLs end at white space (trailing commas are
/// dropped), descriptors run to the next comma outside parentheses; invalid candidates are skipped.
fn parse_srcset(s: &str) -> Vec<Candidate> {
    let chars: Vec<char> = s.chars().collect();
    let mut out = Vec::new();
    let mut i = 0usize;
    while out.len() < MAX_CANDIDATES {
        while chars.get(i).is_some_and(|c| c.is_ascii_whitespace() || *c == ',') {
            i += 1;
        }
        if i >= chars.len() {
            break;
        }
        let start = i;
        while chars.get(i).is_some_and(|c| !c.is_ascii_whitespace()) {
            i += 1;
        }
        let mut url: String = chars.get(start..i).unwrap_or(&[]).iter().collect();
        let mut desc = String::new();
        if url.ends_with(',') {
            url = url.trim_end_matches(',').to_string();
        } else {
            let mut depth = 0u32;
            while let Some(&c) = chars.get(i) {
                i += 1;
                match c {
                    '(' => depth += 1,
                    ')' => depth = depth.saturating_sub(1),
                    ',' if depth == 0 => break,
                    _ => {}
                }
                desc.push(c);
            }
        }
        if url.is_empty() {
            continue;
        }
        let (mut w, mut x, mut bad) = (None, None, false);
        for tok in desc.split_ascii_whitespace() {
            if let Some(n) = tok.strip_suffix('w') {
                match n.parse::<u32>() {
                    Ok(v) if v > 0 && w.is_none() => w = Some(v),
                    _ => bad = true,
                }
            } else if let Some(n) = tok.strip_suffix('x') {
                match n.parse::<f32>() {
                    Ok(v) if v > 0.0 && v.is_finite() && x.is_none() => x = Some(v),
                    _ => bad = true,
                }
            } else {
                bad = true;
            }
        }
        if !bad && !(w.is_some() && x.is_some()) {
            out.push(Candidate { url, w, x });
        }
    }
    out
}

/// The source size from `sizes` (HTML 4.8.4.3 "parse a sizes attribute"): the first entry whose media
/// condition matches, a bare entry at the end, else `100vw`.
fn source_size(sizes: Option<&str>, env: &SelectEnv) -> f32 {
    let default = env.viewport_w.max(1.0);
    let Some(text) = sizes else { return default };
    let cvs = parse_cvs(text);
    let menv = MediaEnv { resolution: env.dpr, ..MediaEnv::screen(env.viewport_w, env.viewport_h) };
    let ctx = LenCtx { font_size: 16.0, root_font_size: 16.0, viewport_w: env.viewport_w, viewport_h: env.viewport_h, ex: 8.0, ch: 8.0 };
    for entry in split_commas(&cvs) {
        let e = trim_ws(entry);
        let Some((last, cond)) = e.split_last() else { continue };
        let Some(len) = parse_length(last, true) else { continue };
        let px = len.compute(&ctx).resolve(0.0);
        if !px.is_finite() || px < 0.0 {
            continue;
        }
        let cond = trim_ws(cond);
        if cond.is_empty() || MediaQueryList::parse(cond).matches(&menv) {
            return px;
        }
    }
    default
}

/// Pick the source of an `<img>` or `<input type=image>`: all `srcset` candidates plus `src` (as `1x`),
/// then the smallest density that is at least the device pixel ratio, else the largest.
/// Returns the URL text and its density.
pub fn select_source(doc: &Document, n: NodeId, env: &SelectEnv) -> Option<(String, f32)> {
    let attr = |name: &str| doc.atoms.lookup(name).and_then(|a| doc.attr(n, a));
    let src = attr("src").map(str::trim).filter(|s| !s.is_empty());
    let mut cands: Vec<(String, f32)> = Vec::new();
    if let Some(set) = attr("srcset") {
        let parsed = parse_srcset(set);
        let size = if parsed.iter().any(|c| c.w.is_some()) { source_size(attr("sizes"), env) } else { 0.0 };
        for c in parsed {
            let density = match (c.w, c.x) {
                (Some(w), _) => w as f32 / size.max(1.0),
                (None, Some(x)) => x,
                (None, None) => 1.0,
            };
            cands.push((c.url, density));
        }
    }
    if let Some(s) = src
        && !cands.iter().any(|(_, d)| *d == 1.0)
    {
        cands.push((s.to_string(), 1.0));
    }
    crate::sorting::sort_by_u64_key(&mut cands, |c| crate::sorting::f32_key(c.1));
    let dpr = if env.dpr.is_finite() && env.dpr > 0.0 { env.dpr } else { 1.0 };
    let pick = cands.iter().find(|(_, d)| *d >= dpr).or_else(|| cands.last())?;
    Some((pick.0.clone(), pick.1))
}
