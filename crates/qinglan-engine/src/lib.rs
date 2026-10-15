//! qinglan-engine: the pure-Rust browser core (no Windows calls, no unsafe).
#![forbid(unsafe_code)]

pub mod css;
pub mod encoding;
pub mod html;
pub mod interact;
pub mod layout;
pub mod lru;
pub mod net;
pub mod paint;
pub mod platform;
pub mod textclass;
pub mod url;
