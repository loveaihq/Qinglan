//! A small fast non-cryptographic hasher (the FxHash multiply-rotate scheme), for caches whose keys
//! are not attacker-chosen in a way that matters for speed (hash flooding only slows one page).

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

#[derive(Default, Clone, Copy)]
pub struct FxHasher {
    h: u64,
}

const K: u64 = 0x517c_c1b7_2722_0a95;

impl FxHasher {
    #[inline]
    fn add(&mut self, w: u64) {
        self.h = (self.h.rotate_left(5) ^ w).wrapping_mul(K);
    }
}

impl Hasher for FxHasher {
    fn write(&mut self, bytes: &[u8]) {
        let (chunks, rest) = bytes.as_chunks::<8>();
        for c in chunks {
            self.add(u64::from_le_bytes(*c));
        }
        let mut tail = 0u64;
        for (i, b) in rest.iter().enumerate() {
            tail |= u64::from(*b) << (8 * i);
        }
        self.add(tail ^ ((rest.len() as u64) << 56));
    }
    fn write_u8(&mut self, i: u8) {
        self.add(u64::from(i));
    }
    fn write_u32(&mut self, i: u32) {
        self.add(u64::from(i));
    }
    fn write_u64(&mut self, i: u64) {
        self.add(i);
    }
    fn write_usize(&mut self, i: usize) {
        self.add(i as u64);
    }
    fn finish(&self) -> u64 {
        // One more multiply so the high bits (used by hashbrown's control bytes) are mixed.
        self.h.wrapping_mul(K).rotate_left(26)
    }
}

pub type FxMap<K, V> = HashMap<K, V, BuildHasherDefault<FxHasher>>;
