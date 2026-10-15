//! Windows platform layer for qinglan: DirectWrite fonts, text shaping, line breaking and glyph
//! masks, and legacy text encodings through `MultiByteToWideChar` (step 1b-3); the WinHTTP transport
//! and WIC image decoding (step 1c-1). The window arrives in step 1c-2.
//!
//! `unsafe` is only allowed under `sys/`; everything else in this crate is safe Rust on top of the
//! safe wrappers `sys` exports.
#![deny(unsafe_code)]

mod encodings;
mod fx;
mod images;
mod lru;
mod net;
mod platform;
#[allow(unsafe_code)]
mod sys;

pub use encodings::{code_page_for, decode_legacy};
pub use images::WicDecoder;
pub use net::{WinHttpConfig, WinHttpTransport, map_error};
pub use platform::{CacheStats, WinPlatform};
