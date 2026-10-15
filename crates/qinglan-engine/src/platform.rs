//! The platform interface: everything the engine needs from the operating system.
//!
//! Step 1a only needs the encoding hook; fonts, text shaping, networking and image decoding are
//! added in 1b and 1c. The engine never calls Windows directly (CLAUDE.md "Technical decisions").

/// Services the engine asks of the host.
pub trait Platform {
    /// Decode `bytes` from the legacy encoding `encoding`, a WHATWG encoding name such as
    /// "GBK", "Big5" or "Shift_JIS" (Encoding Standard 4.2). Bad bytes become U+FFFD.
    /// Returns `None` when the platform has no decoder for that encoding.
    fn decode_legacy(&self, encoding: &str, bytes: &[u8]) -> Option<String>;
}

/// A platform with no legacy-encoding support: only what the engine decodes itself
/// (UTF-8, UTF-16, windows-1252, x-user-defined). Used by the 1a tests and tools.
pub struct BasicPlatform;

impl Platform for BasicPlatform {
    fn decode_legacy(&self, _encoding: &str, _bytes: &[u8]) -> Option<String> {
        None
    }
}
