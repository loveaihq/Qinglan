//! Inflate (RFC 1951) and the zlib wrapper (RFC 1950), for the tables of a WOFF 1.0 file.
//!
//! The output is capped by the caller (`limit`): a stream that would produce more is `TooLarge` as soon as it
//! tries, so a zlib bomb costs at most `limit` bytes. Every block consumes input, so the work is linear in the input
//! plus the (capped) output.

use super::bits::BitReader;
use super::huff::Huff;
use super::{FontError, at};

const LEN_BASE: [u16; 29] = [3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131, 163, 195, 227, 258];
const LEN_EXTRA: [u8; 29] = [0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0];
const DIST_BASE: [u16; 30] = [1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537, 2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577];
const DIST_EXTRA: [u8; 30] = [0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13];
/// The order in which the code lengths of the code length code are sent (RFC 1951 3.2.7).
const CL_ORDER: [usize; 19] = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];

/// A raw deflate stream. Returns the output and how many bytes of `data` the stream took.
pub fn inflate(data: &[u8], limit: usize) -> Result<(Vec<u8>, usize), FontError> {
    let mut br = BitReader::new(data);
    let mut out: Vec<u8> = Vec::new();
    let mut fixed: Option<(Huff, Huff)> = None;
    loop {
        let last = br.read(1)? == 1;
        match br.read(2)? {
            0 => {
                br.align()?;
                let hdr = br.bytes(4)?;
                let len = usize::from(u16::from_le_bytes([at(hdr, 0)?, at(hdr, 1)?]));
                let nlen = u16::from_le_bytes([at(hdr, 2)?, at(hdr, 3)?]);
                if nlen != !(len as u16) {
                    return Err(FontError::Corrupt("stored block length"));
                }
                if len > limit - out.len() {
                    return Err(FontError::TooLarge);
                }
                out.extend_from_slice(br.bytes(len)?);
            }
            1 => {
                if fixed.is_none() {
                    let mut l = [8u8; 288];
                    for (i, v) in l.iter_mut().enumerate() {
                        *v = match i {
                            144..=255 => 9,
                            256..=279 => 7,
                            _ => 8,
                        };
                    }
                    fixed = Some((Huff::from_lengths(&l)?, Huff::from_lengths(&[5u8; 30])?));
                }
                if let Some((lit, dist)) = &fixed {
                    block(&mut br, &mut out, lit, dist, limit)?;
                }
            }
            2 => {
                let (lit, dist) = dynamic_codes(&mut br)?;
                block(&mut br, &mut out, &lit, &dist, limit)?;
            }
            _ => return Err(FontError::Corrupt("block type")),
        }
        if last {
            return Ok((out, br.consumed()));
        }
    }
}

fn dynamic_codes(br: &mut BitReader<'_>) -> Result<(Huff, Huff), FontError> {
    let nlit = br.read(5)? as usize + 257;
    let ndist = br.read(5)? as usize + 1;
    let ncl = br.read(4)? as usize + 4;
    if nlit > 286 || ndist > 30 {
        return Err(FontError::Corrupt("too many codes"));
    }
    let mut cl = [0u8; 19];
    for i in 0..ncl {
        let v = br.read(3)? as u8;
        if let Some(s) = cl.get_mut(at(&CL_ORDER, i)?) {
            *s = v;
        }
    }
    let clcode = Huff::from_lengths(&cl)?;
    let mut lens = [0u8; 286 + 30];
    let total = nlit + ndist;
    let mut i = 0usize;
    while i < total {
        let sym = clcode.decode(br)?;
        let (value, rep) = match sym {
            0..=15 => (sym as u8, 1usize),
            16 => {
                let prev = if i > 0 { at(&lens, i - 1)? } else { return Err(FontError::Corrupt("repeat without a length")) };
                (prev, 3 + br.read(2)? as usize)
            }
            17 => (0, 3 + br.read(3)? as usize),
            _ => (0, 11 + br.read(7)? as usize),
        };
        if i + rep > total {
            return Err(FontError::Corrupt("code lengths overrun"));
        }
        for s in lens.iter_mut().skip(i).take(rep) {
            *s = value;
        }
        i += rep;
    }
    if at(&lens, 256)? == 0 {
        return Err(FontError::Corrupt("no end-of-block code"));
    }
    let lit = Huff::from_lengths(lens.get(..nlit).ok_or(FontError::Corrupt("lengths"))?)?;
    let dist = Huff::from_lengths(lens.get(nlit..total).ok_or(FontError::Corrupt("lengths"))?)?;
    Ok((lit, dist))
}

fn block(br: &mut BitReader<'_>, out: &mut Vec<u8>, lit: &Huff, dist: &Huff, limit: usize) -> Result<(), FontError> {
    loop {
        let sym = usize::from(lit.decode(br)?);
        if sym < 256 {
            if out.len() >= limit {
                return Err(FontError::TooLarge);
            }
            out.push(sym as u8);
            continue;
        }
        if sym == 256 {
            return Ok(());
        }
        let li = sym - 257;
        if li >= 29 {
            return Err(FontError::Corrupt("length code"));
        }
        let len = usize::from(at(&LEN_BASE, li)?) + br.read(u32::from(at(&LEN_EXTRA, li)?))? as usize;
        let di = usize::from(dist.decode(br)?);
        if di >= 30 {
            return Err(FontError::Corrupt("distance code"));
        }
        let d = usize::from(at(&DIST_BASE, di)?) + br.read(u32::from(at(&DIST_EXTRA, di)?))? as usize;
        if d > out.len() {
            return Err(FontError::Corrupt("distance too far back"));
        }
        if len > limit - out.len() {
            return Err(FontError::TooLarge);
        }
        let start = out.len() - d;
        if d >= len {
            out.extend_from_within(start..start + len);
        } else {
            for k in 0..len {
                let b = at(out, start + k)?;
                out.push(b);
            }
        }
    }
}

/// A zlib stream (RFC 1950): two header bytes, deflate, and the Adler-32 of the output.
pub fn zlib(data: &[u8], limit: usize) -> Result<Vec<u8>, FontError> {
    let (cmf, flg) = (at(data, 0)?, at(data, 1)?);
    if cmf & 15 != 8 || cmf >> 4 > 7 || (u16::from(cmf) * 256 + u16::from(flg)) % 31 != 0 || flg & 0x20 != 0 {
        return Err(FontError::Corrupt("zlib header"));
    }
    let body = data.get(2..).ok_or(FontError::Truncated)?;
    let (out, used) = inflate(body, limit)?;
    let tail = body.get(used..used + 4).ok_or(FontError::Truncated)?;
    let want = u32::from_be_bytes([at(tail, 0)?, at(tail, 1)?, at(tail, 2)?, at(tail, 3)?]);
    if adler32(&out) != want {
        return Err(FontError::Corrupt("adler32"));
    }
    Ok(out)
}

fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    // 5552 bytes is the longest run before the sums can overflow a u32.
    for chunk in data.chunks(5552) {
        for &x in chunk {
            a += u32::from(x);
            b += a;
        }
        a %= 65521;
        b %= 65521;
    }
    (b << 16) | a
}
