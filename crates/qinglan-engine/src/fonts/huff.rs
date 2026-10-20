//! Canonical prefix codes as deflate (RFC 1951 3.2.2) and Brotli (RFC 7932 3.2) use them: codes of up to 15 bits,
//! assigned in order of length and then of symbol, packed starting with the most significant bit of the code.

use super::bits::BitReader;
use super::{FontError, at};

/// Codes up to this long are found with one table lookup.
const FAST_BITS: u32 = 8;
const FAST: usize = 1 << FAST_BITS;

pub struct Huff {
    /// `symbol << 4 | length` for codes of at most `FAST_BITS` bits, indexed by the next bits of the stream; 0 = longer.
    fast: [u16; FAST],
    /// Codes of each length.
    count: [u16; 16],
    /// Symbols ordered by (code length, symbol).
    syms: Vec<u16>,
    /// A code of one symbol that takes no bits (Brotli's simple code with one symbol).
    single: Option<u16>,
}

impl Huff {
    pub fn single(sym: u16) -> Huff {
        Huff { fast: [0; FAST], count: [0; 16], syms: Vec::new(), single: Some(sym) }
    }

    /// The code with these lengths (0 = unused). More codes than the lengths allow is an error; unused codes are
    /// not (they fail when a stream uses them).
    pub fn from_lengths(lens: &[u8]) -> Result<Huff, FontError> {
        let mut count = [0u16; 16];
        for &l in lens {
            let c = count.get_mut(usize::from(l)).ok_or(FontError::Corrupt("code length"))?;
            *c = c.saturating_add(1);
        }
        count[0] = 0;
        let mut left: i32 = 1;
        for len in 1..16 {
            left = (left << 1) - i32::from(at(&count, len)?);
            if left < 0 {
                return Err(FontError::Corrupt("over-subscribed prefix code"));
            }
        }
        let mut next_code = [0u32; 16];
        let mut offs = [0usize; 16];
        let mut code = 0u32;
        let mut total = 0usize;
        for len in 1..16 {
            code = (code + u32::from(at(&count, len - 1)?)) << 1;
            if let Some(s) = next_code.get_mut(len) {
                *s = code;
            }
            if let Some(s) = offs.get_mut(len) {
                *s = total;
            }
            total += usize::from(at(&count, len)?);
        }
        let mut syms = vec![0u16; total];
        let mut fast = [0u16; FAST];
        for (sym, &l) in lens.iter().enumerate() {
            if l == 0 {
                continue;
            }
            let len = usize::from(l);
            let o = offs.get_mut(len).ok_or(FontError::Corrupt("code length"))?;
            if let Some(s) = syms.get_mut(*o) {
                *s = sym as u16;
            }
            *o += 1;
            let c = next_code.get_mut(len).ok_or(FontError::Corrupt("code length"))?;
            let this = *c;
            *c += 1;
            if u32::from(l) <= FAST_BITS {
                let rev = this.reverse_bits() >> (32 - u32::from(l));
                let entry = ((sym as u16) << 4) | u16::from(l);
                let mut i = rev as usize;
                while i < FAST {
                    if let Some(s) = fast.get_mut(i) {
                        *s = entry;
                    }
                    i += 1 << len;
                }
            }
        }
        Ok(Huff { fast, count, syms, single: None })
    }

    #[inline(never)]
    pub fn decode(&self, br: &mut BitReader<'_>) -> Result<u16, FontError> {
        if let Some(s) = self.single {
            return Ok(s);
        }
        let look = br.peek(FAST_BITS) as usize;
        let e = self.fast.get(look).copied().unwrap_or(0);
        if e != 0 {
            br.consume(u32::from(e & 15))?;
            return Ok(e >> 4);
        }
        self.decode_slow(br)
    }

    /// Codes longer than `FAST_BITS` (or no code at all): one bit at a time.
    fn decode_slow(&self, br: &mut BitReader<'_>) -> Result<u16, FontError> {
        let (bits, avail) = br.buffered();
        let (mut code, mut first, mut index) = (0i32, 0i32, 0i32);
        for len in 1..16u32 {
            if len > avail {
                return Err(FontError::Truncated);
            }
            code |= ((bits >> (len - 1)) & 1) as i32;
            let count = i32::from(at(&self.count, len as usize)?);
            if code - count < first {
                br.consume(len)?;
                return at(&self.syms, (index + (code - first)) as usize);
            }
            index += count;
            first += count;
            first <<= 1;
            code <<= 1;
        }
        Err(FontError::Corrupt("bad prefix code"))
    }
}
