//! A bit reader for the streams of deflate (RFC 1951 3.1.1) and Brotli (RFC 7932 1.5.1): bits are taken from the low
//! end of each byte first.

use super::FontError;

pub struct BitReader<'a> {
    data: &'a [u8],
    /// Next byte of `data` to load into `buf`.
    pos: usize,
    buf: u64,
    /// Valid bits in `buf`.
    n: u32,
}

impl<'a> BitReader<'a> {
    pub fn new(data: &'a [u8]) -> BitReader<'a> {
        BitReader { data, pos: 0, buf: 0, n: 0 }
    }

    #[inline]
    pub fn refill(&mut self) {
        while self.n <= 56 {
            let Some(&b) = self.data.get(self.pos) else { break };
            self.buf |= u64::from(b) << self.n;
            self.n += 8;
            self.pos += 1;
        }
    }

    /// The next `k` (at most 32) bits without taking them; missing bits at the end of the data read as zero.
    #[inline]
    pub fn peek(&mut self, k: u32) -> u32 {
        self.refill();
        (self.buf & ((1u64 << k) - 1)) as u32
    }

    /// The bits in the buffer (call after `refill`/`peek`), for decoders that look at more than they take.
    #[inline]
    pub fn buffered(&self) -> (u64, u32) {
        (self.buf, self.n)
    }

    #[inline]
    pub fn consume(&mut self, k: u32) -> Result<(), FontError> {
        if k > self.n {
            return Err(FontError::Truncated);
        }
        self.buf >>= k;
        self.n -= k;
        Ok(())
    }

    #[inline]
    pub fn read(&mut self, k: u32) -> Result<u32, FontError> {
        if k == 0 {
            return Ok(0);
        }
        let v = self.peek(k);
        self.consume(k)?;
        Ok(v)
    }

    /// Skip to the next byte boundary. True when the skipped bits were all zero.
    pub fn align(&mut self) -> Result<bool, FontError> {
        let k = self.n % 8;
        if k == 0 {
            return Ok(true);
        }
        let v = self.buf & ((1u64 << k) - 1);
        self.consume(k)?;
        Ok(v == 0)
    }

    /// `len` whole bytes; the reader must be byte-aligned.
    pub fn bytes(&mut self, len: usize) -> Result<&'a [u8], FontError> {
        // Give back the whole bytes still in the buffer.
        self.pos -= (self.n / 8) as usize;
        self.buf = 0;
        self.n = 0;
        let end = self.pos.checked_add(len).ok_or(FontError::Truncated)?;
        let s = self.data.get(self.pos..end).ok_or(FontError::Truncated)?;
        self.pos = end;
        Ok(s)
    }

    /// Bytes of the input taken so far (a partly used byte counts as taken).
    pub fn consumed(&self) -> usize {
        self.pos - (self.n / 8) as usize
    }
}
