//! qinglan-engine: the pure-Rust browser core (no Windows calls, no unsafe).
#![forbid(unsafe_code)]

pub mod css;
pub mod encoding;
pub mod fmath;
pub mod html;
pub mod interact;
pub mod layout;
pub mod lru;
pub mod net;
pub mod paint;
pub mod platform;
pub mod sorting;
pub mod textclass;
pub mod url;
pub mod vector;
