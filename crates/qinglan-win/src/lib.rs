//! Windows platform layer for qinglan: DirectWrite fonts, text shaping, line breaking and glyph
//! masks, and legacy text encodings through `MultiByteToWideChar` (step 1b-3). WinHTTP, WIC and the
//! window arrive in step 1c.
//!
//! `unsafe` is only allowed under `sys/`; everything else in this crate is safe Rust on top of the
//! safe wrappers `sys` exports.
#![deny(unsafe_code)]

mod encodings;
mod fx;
mod lru;
mod platform;
#[allow(unsafe_code)]
mod sys;

pub use encodings::{code_page_for, decode_legacy};
pub use platform::{CacheStats, WinPlatform};
