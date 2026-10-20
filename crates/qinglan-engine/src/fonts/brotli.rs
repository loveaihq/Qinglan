//! A Brotli decoder (RFC 7932), for the table data of a WOFF2 file.
//!
//! The decoder produces at most `limit` bytes: a meta-block that claims more is `TooLarge` before it is read, so a
//! Brotli bomb costs at most `limit` bytes of memory and the time to produce them. Every command consumes input
//! bits or, for the cheapest ones (a copy by a one-symbol code), produces at least two bytes of counted output.
//!
//! The static dictionary (RFC 7932 appendix A, 122,784 bytes) is stored deflate-compressed in the executable
//! (`brotli_dict.z`, 58,898 bytes) and inflated the first time a stream refers to it.

use std::sync::OnceLock;

use super::bits::BitReader;
use super::brotli_tables::{AFFIXES, TRANSFORMS, UTF8_LUT0, UTF8_LUT1};
use super::huff::Huff;
use super::{FontError, at, inflate};

static DICT_Z: &[u8] = include_bytes!("brotli_dict.z");
const DICT_SIZE: usize = 122_784;
static DICT: OnceLock<Option<Vec<u8>>> = OnceLock::new();

pub fn dictionary() -> Result<&'static [u8], FontError> {
    DICT.get_or_init(|| inflate::inflate(DICT_Z, DICT_SIZE).ok().map(|(d, _)| d).filter(|d| d.len() == DICT_SIZE))
        .as_deref()
        .ok_or(FontError::Corrupt("static dictionary"))
}

/// Bits of the word index per word length (RFC 7932 appendix A).
const NDBITS: [u8; 25] = [0, 0, 0, 0, 10, 10, 11, 11, 10, 10, 10, 10, 10, 9, 9, 8, 7, 7, 8, 7, 7, 6, 6, 5, 5];

fn dict_offset(len: usize) -> usize {
    (4..len).map(|l| l * (1usize << NDBITS.get(l).copied().unwrap_or(0))).sum()
}

const INSERT_BASE: [u32; 24] = [0, 1, 2, 3, 4, 5, 6, 8, 10, 14, 18, 26, 34, 50, 66, 98, 130, 194, 322, 578, 1090, 2114, 6210, 22594];
const INSERT_EXTRA: [u8; 24] = [0, 0, 0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 7, 8, 9, 10, 12, 14, 24];
const COPY_BASE: [u32; 24] = [2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 14, 18, 22, 30, 38, 54, 70, 102, 134, 198, 326, 582, 1094, 2118];
const COPY_EXTRA: [u8; 24] = [0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 7, 8, 9, 10, 24];
const BLOCK_BASE: [u32; 26] = [1, 5, 9, 13, 17, 25, 33, 41, 49, 65, 81, 97, 113, 145, 177, 209, 241, 305, 369, 497, 753, 1265, 2289, 4337, 8433, 16625];
const BLOCK_EXTRA: [u8; 26] = [2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 6, 6, 7, 8, 9, 10, 11, 12, 13, 24];
/// (insert code base, copy code base) of the command symbols in blocks of 64 (RFC 7932 5).
const CELLS: [(u8, u8); 11] = [(0, 0), (0, 8), (0, 0), (0, 8), (8, 0), (8, 8), (0, 16), (16, 0), (8, 16), (16, 8), (16, 16)];
/// The order in which the code length code lengths are sent (RFC 7932 3.5).
const CL_ORDER: [usize; 18] = [1, 2, 3, 4, 0, 5, 17, 6, 16, 7, 8, 9, 10, 11, 12, 13, 14, 15];
/// The fixed code of the code length code lengths, indexed by the next four bits: (bits used, value).
const CLCL_LEN: [u8; 16] = [2, 2, 2, 3, 2, 2, 2, 4, 2, 2, 2, 3, 2, 2, 2, 4];
const CLCL_VAL: [u8; 16] = [0, 4, 3, 2, 0, 4, 3, 1, 0, 4, 3, 2, 0, 4, 3, 5];

/// Decode a whole Brotli stream. The output may not be longer than `limit`.
pub fn decompress(input: &[u8], limit: usize) -> Result<Vec<u8>, FontError> {
    let mut d = Decoder { br: BitReader::new(input), out: Vec::new(), limit, window: 0, rb: [4, 11, 15, 16] };
    d.run()?;
    Ok(d.out)
}

struct Decoder<'a> {
    br: BitReader<'a>,
    out: Vec<u8>,
    limit: usize,
    /// The farthest a copy may reach back: `(1 << WBITS) - 16`.
    window: usize,
    /// The last four distances, the latest first.
    rb: [usize; 4],
}

/// One of the three kinds of blocks (literals, commands, distances) of a meta-block.
struct Blocks {
    ntypes: usize,
    types: Option<Huff>,
    counts: Option<Huff>,
    /// Block type now, the one before it, and the symbols left in this block.
    cur: usize,
    last: usize,
    last2: usize,
    left: usize,
}

impl Blocks {
    fn read(br: &mut BitReader<'_>) -> Result<Blocks, FontError> {
        let ntypes = varlen(br)?;
        let mut b = Blocks { ntypes, types: None, counts: None, cur: 0, last: 0, last2: 1, left: 1 << 24 };
        if ntypes >= 2 {
            b.types = Some(prefix_code(br, ntypes + 2)?);
            b.counts = Some(prefix_code(br, 26)?);
            b.left = block_count(br, b.counts.as_ref())?;
        }
        Ok(b)
    }

    /// Take one symbol of the current block, switching blocks first when it is used up.
    #[inline]
    fn next(&mut self, br: &mut BitReader<'_>) -> Result<usize, FontError> {
        if self.left == 0 {
            self.switch(br)?;
        }
        self.left -= 1;
        Ok(self.cur)
    }

    fn switch(&mut self, br: &mut BitReader<'_>) -> Result<(), FontError> {
        let code = self.types.as_ref().ok_or(FontError::Corrupt("block switch"))?.decode(br)? as usize;
        let t = match code {
            0 => self.last2,
            1 => self.last + 1,
            c => c - 2,
        };
        let t = if t >= self.ntypes && code == 1 { t - self.ntypes } else { t };
        if t >= self.ntypes {
            return Err(FontError::Corrupt("block type"));
        }
        self.last2 = self.last;
        self.last = t;
        self.cur = t;
        self.left = block_count(br, self.counts.as_ref())?;
        if self.left == 0 {
            return Err(FontError::Corrupt("empty block"));
        }
        Ok(())
    }
}

fn block_count(br: &mut BitReader<'_>, code: Option<&Huff>) -> Result<usize, FontError> {
    let s = code.ok_or(FontError::Corrupt("block count"))?.decode(br)? as usize;
    Ok(at(&BLOCK_BASE, s)? as usize + br.read(u32::from(at(&BLOCK_EXTRA, s)?))? as usize)
}

/// NBLTYPES / NTREES (RFC 7932 9.2): 1 to 256.
fn varlen(br: &mut BitReader<'_>) -> Result<usize, FontError> {
    if br.read(1)? == 0 {
        return Ok(1);
    }
    let n = br.read(3)?;
    Ok((1usize << n) + 1 + br.read(n)? as usize)
}

/// A prefix code over `alphabet` symbols (RFC 7932 3.4 and 3.5).
fn prefix_code(br: &mut BitReader<'_>, alphabet: usize) -> Result<Huff, FontError> {
    let hskip = br.read(2)?;
    if hskip == 1 {
        return simple_code(br, alphabet);
    }
    // The code length code.
    let mut cl = [0u8; 18];
    let mut space = 32i32;
    let mut nonzero = 0;
    let mut last_sym = 0usize;
    for &slot in CL_ORDER.iter().skip(hskip as usize) {
        let v = br.peek(4) as usize;
        br.consume(u32::from(at(&CLCL_LEN, v)?))?;
        let len = at(&CLCL_VAL, v)?;
        if let Some(c) = cl.get_mut(slot) {
            *c = len;
        }
        if len != 0 {
            space -= 32 >> len;
            nonzero += 1;
            last_sym = slot;
            if space <= 0 {
                break;
            }
        }
    }
    if !(nonzero == 1 || space == 0) {
        return Err(FontError::Corrupt("code length code"));
    }
    let clcode = if nonzero == 1 { Huff::single(last_sym as u16) } else { Huff::from_lengths(&cl)? };
    // The code lengths of the symbols.
    let mut lens = vec![0u8; alphabet];
    let (mut sym, mut prev, mut repeat, mut repeat_len) = (0usize, 8u8, 0usize, 0u8);
    let mut space = 32768i32;
    while sym < alphabet && space > 0 {
        let c = clcode.decode(br)?;
        if c < 16 {
            repeat = 0;
            if let Some(s) = lens.get_mut(sym) {
                *s = c as u8;
            }
            sym += 1;
            if c != 0 {
                prev = c as u8;
                space -= 32768 >> c;
            }
            continue;
        }
        let extra = if c == 16 { 2 } else { 3 };
        let new_len = if c == 16 { prev } else { 0 };
        if repeat_len != new_len {
            repeat = 0;
            repeat_len = new_len;
        }
        let old = repeat;
        if repeat > 0 {
            repeat = (repeat - 2) << extra;
        }
        repeat += br.read(extra)? as usize + 3;
        let delta = repeat - old;
        if sym + delta > alphabet {
            return Err(FontError::Corrupt("code lengths overrun"));
        }
        for s in lens.iter_mut().skip(sym).take(delta) {
            *s = new_len;
        }
        sym += delta;
        if new_len != 0 {
            space -= (delta as i32) << (15 - new_len);
        }
    }
    if space != 0 {
        return Err(FontError::Corrupt("prefix code is not complete"));
    }
    Huff::from_lengths(&lens)
}

fn simple_code(br: &mut BitReader<'_>, alphabet: usize) -> Result<Huff, FontError> {
    let nsym = br.read(2)? as usize + 1;
    let bits = usize::BITS - (alphabet - 1).leading_zeros();
    let mut syms = [0u16; 4];
    for i in 0..nsym {
        let s = br.read(bits)? as usize;
        if s >= alphabet {
            return Err(FontError::Corrupt("symbol out of range"));
        }
        if let Some(slot) = syms.get_mut(i) {
            *slot = s as u16;
        }
    }
    let used = syms.get(..nsym).ok_or(FontError::Corrupt("symbols"))?;
    for (i, a) in used.iter().enumerate() {
        if used.iter().skip(i + 1).any(|b| a == b) {
            return Err(FontError::Corrupt("repeated symbol"));
        }
    }
    if nsym == 1 {
        return Ok(Huff::single(at(&syms, 0)?));
    }
    // (symbol, length): the lengths go to the symbols in the order they were sent, then ties are by symbol value.
    let mut lens = vec![0u8; alphabet];
    let shape: &[u8] = match nsym {
        2 => &[1, 1],
        3 => &[1, 2, 2],
        _ => {
            if br.read(1)? == 1 {
                &[1, 2, 3, 3]
            } else {
                &[2, 2, 2, 2]
            }
        }
    };
    for (&s, &l) in used.iter().zip(shape) {
        if let Some(slot) = lens.get_mut(usize::from(s)) {
            *slot = l;
        }
    }
    Huff::from_lengths(&lens)
}

/// A context map (RFC 7932 7.3): `size` entries naming one of the returned number of trees.
fn context_map(br: &mut BitReader<'_>, size: usize) -> Result<(usize, Vec<u8>), FontError> {
    let ntrees = varlen(br)?;
    if ntrees == 1 {
        return Ok((1, vec![0; size]));
    }
    let rle_max = if br.read(1)? == 1 { br.read(4)? as usize + 1 } else { 0 };
    let code = prefix_code(br, ntrees + rle_max)?;
    let mut map: Vec<u8> = Vec::with_capacity(size);
    while map.len() < size {
        let s = code.decode(br)? as usize;
        if s == 0 {
            map.push(0);
        } else if s <= rle_max {
            let reps = (1usize << s) + br.read(s as u32)? as usize;
            if reps > size - map.len() {
                return Err(FontError::Corrupt("context map run"));
            }
            map.resize(map.len() + reps, 0);
        } else {
            map.push((s - rle_max) as u8);
        }
    }
    if br.read(1)? == 1 {
        let mut mtf: Vec<u8> = (0..=255u8).collect();
        for v in map.iter_mut() {
            let i = usize::from(*v);
            let val = at(&mtf, i)?;
            *v = val;
            mtf.remove(i);
            mtf.insert(0, val);
        }
    }
    Ok((ntrees, map))
}

/// The context of a literal (RFC 7932 7.1) from the two bytes before it.
#[inline]
fn literal_context(mode: u8, p1: u8, p2: u8) -> usize {
    match mode {
        0 => usize::from(p1 & 63),
        1 => usize::from(p1 >> 2),
        2 => usize::from(UTF8_LUT0.get(usize::from(p1)).copied().unwrap_or(0) | UTF8_LUT1.get(usize::from(p2)).copied().unwrap_or(0)),
        _ => (usize::from(signed_class(p1)) << 3) | usize::from(signed_class(p2)),
    }
}

fn signed_class(b: u8) -> u8 {
    match b {
        0 => 0,
        1..=15 => 1,
        16..=63 => 2,
        64..=127 => 3,
        128..=191 => 4,
        192..=239 => 5,
        240..=254 => 6,
        255 => 7,
    }
}

impl Decoder<'_> {
    fn run(&mut self) -> Result<(), FontError> {
        // The window size (RFC 7932 9.1).
        let wbits = if self.br.read(1)? == 0 {
            16
        } else {
            match self.br.read(3)? {
                0 => match self.br.read(3)? {
                    0 => 17,
                    1 => return Err(FontError::Corrupt("large window")),
                    m => 8 + m,
                },
                n => 17 + n,
            }
        };
        self.window = (1usize << wbits) - 16;
        loop {
            let last = self.br.read(1)? == 1;
            if last && self.br.read(1)? == 1 {
                return Ok(());
            }
            let nibbles = self.br.read(2)?;
            if nibbles == 3 {
                // A metadata block: skipped (RFC 7932 9.2).
                if self.br.read(1)? != 0 {
                    return Err(FontError::Corrupt("reserved bit"));
                }
                let nbytes = self.br.read(2)?;
                let mut skip = 0usize;
                for i in 0..nbytes {
                    let b = self.br.read(8)? as usize;
                    if i + 1 == nbytes && nbytes > 1 && b == 0 {
                        return Err(FontError::Corrupt("metadata length"));
                    }
                    skip |= b << (8 * i);
                }
                if nbytes > 0 {
                    skip += 1;
                }
                if !self.br.align()? {
                    return Err(FontError::Corrupt("metadata padding"));
                }
                self.br.bytes(skip)?;
                if last {
                    return Ok(());
                }
                continue;
            }
            let n = nibbles + 4;
            let mlen_m1 = self.br.read(n * 4)? as usize;
            if n > 4 && (mlen_m1 >> ((n - 1) * 4)) == 0 {
                return Err(FontError::Corrupt("meta-block length"));
            }
            let mlen = mlen_m1 + 1;
            if mlen > self.limit - self.out.len() {
                return Err(FontError::TooLarge);
            }
            if !last && self.br.read(1)? == 1 {
                if !self.br.align()? {
                    return Err(FontError::Corrupt("uncompressed padding"));
                }
                let b = self.br.bytes(mlen)?;
                self.out.extend_from_slice(b);
            } else {
                self.meta_block(mlen)?;
            }
            if last {
                return Ok(());
            }
        }
    }

    fn meta_block(&mut self, mlen: usize) -> Result<(), FontError> {
        let (br, out, rb, window) = (&mut self.br, &mut self.out, &mut self.rb, self.window);
        let mut lit_b = Blocks::read(br)?;
        let mut cmd_b = Blocks::read(br)?;
        let mut dist_b = Blocks::read(br)?;
        let npostfix = br.read(2)?;
        let ndirect = (br.read(4)? as usize) << npostfix;
        let mut modes = Vec::with_capacity(lit_b.ntypes);
        for _ in 0..lit_b.ntypes {
            modes.push(br.read(2)? as u8);
        }
        let (nlit_trees, lit_map) = context_map(br, lit_b.ntypes * 64)?;
        let (ndist_trees, dist_map) = context_map(br, dist_b.ntypes * 4)?;
        let mut lit_codes = Vec::with_capacity(nlit_trees);
        for _ in 0..nlit_trees {
            lit_codes.push(prefix_code(br, 256)?);
        }
        let mut cmd_codes = Vec::with_capacity(cmd_b.ntypes);
        for _ in 0..cmd_b.ntypes {
            cmd_codes.push(prefix_code(br, 704)?);
        }
        let dist_alphabet = 16 + ndirect + (48usize << npostfix);
        let mut dist_codes = Vec::with_capacity(ndist_trees);
        for _ in 0..ndist_trees {
            dist_codes.push(prefix_code(br, dist_alphabet)?);
        }

        let mut remaining = mlen;
        while remaining > 0 {
            let ct = cmd_b.next(br)?;
            let cmd = usize::from(at_ref(&cmd_codes, ct)?.decode(br)?);
            let (ib, cb) = at(&CELLS, cmd >> 6)?;
            let implicit = cmd < 128;
            let ins_code = usize::from(ib) + ((cmd >> 3) & 7);
            let copy_code = usize::from(cb) + (cmd & 7);
            let ins_len = at(&INSERT_BASE, ins_code)? as usize + br.read(u32::from(at(&INSERT_EXTRA, ins_code)?))? as usize;
            let copy_len = at(&COPY_BASE, copy_code)? as usize + br.read(u32::from(at(&COPY_EXTRA, copy_code)?))? as usize;
            if ins_len > remaining {
                return Err(FontError::Corrupt("insert length past the meta-block"));
            }
            for _ in 0..ins_len {
                let bt = lit_b.next(br)?;
                let n = out.len();
                let p1 = if n >= 1 { at(out, n - 1)? } else { 0 };
                let p2 = if n >= 2 { at(out, n - 2)? } else { 0 };
                let ctx = literal_context(at(&modes, bt)?, p1, p2);
                let tree = usize::from(at(&lit_map, bt * 64 + ctx)?);
                let byte = at_ref(&lit_codes, tree)?.decode(br)?;
                out.push(byte as u8);
            }
            remaining -= ins_len;
            if remaining == 0 {
                break;
            }
            let dcode = if implicit {
                0
            } else {
                let dt = dist_b.next(br)?;
                let ctx = if copy_len > 4 { 3 } else { copy_len - 2 };
                let tree = usize::from(at(&dist_map, dt * 4 + ctx)?);
                usize::from(at_ref(&dist_codes, tree)?.decode(br)?)
            };
            let (dist, push) = distance(br, rb, dcode, ndirect, npostfix)?;
            let max_dist = out.len().min(window);
            if dist <= max_dist {
                if copy_len > remaining {
                    return Err(FontError::Corrupt("copy past the meta-block"));
                }
                if push {
                    *rb = [dist, rb[0], rb[1], rb[2]];
                }
                remaining -= copy_len;
                let start = out.len() - dist;
                if dist >= copy_len {
                    out.extend_from_within(start..start + copy_len);
                } else {
                    for k in 0..copy_len {
                        let b = at(out, start + k)?;
                        out.push(b);
                    }
                }
            } else {
                // A word of the static dictionary (RFC 7932 8).
                if !(4..=24).contains(&copy_len) {
                    return Err(FontError::Corrupt("dictionary word length"));
                }
                let word_id = dist - max_dist - 1;
                let nbits = u32::from(at(&NDBITS, copy_len)?);
                let (index, tid) = (word_id & ((1usize << nbits) - 1), word_id >> nbits);
                if tid >= TRANSFORMS.len() {
                    return Err(FontError::Corrupt("dictionary transform"));
                }
                let off = dict_offset(copy_len) + index * copy_len;
                let word = dictionary()?.get(off..off + copy_len).ok_or(FontError::Corrupt("dictionary word"))?;
                let before = out.len();
                transform(word, tid, out)?;
                let made = out.len() - before;
                if made == 0 && ins_len == 0 {
                    return Err(FontError::Corrupt("empty dictionary word"));
                }
                if made > remaining {
                    return Err(FontError::Corrupt("word past the meta-block"));
                }
                remaining -= made;
            }
        }
        Ok(())
    }
}

/// The distance a distance code stands for (RFC 7932 4), and whether it goes into the ring of last distances.
fn distance(br: &mut BitReader<'_>, rb: &[usize; 4], dcode: usize, ndirect: usize, npostfix: u32) -> Result<(usize, bool), FontError> {
    if dcode < 16 {
        let r = |i: usize| rb.get(i).copied().unwrap_or(4) as i64;
        let v = match dcode {
            0 => r(0),
            1 => r(1),
            2 => r(2),
            3 => r(3),
            4 => r(0) - 1,
            5 => r(0) + 1,
            6 => r(0) - 2,
            7 => r(0) + 2,
            8 => r(0) - 3,
            9 => r(0) + 3,
            10 => r(1) - 1,
            11 => r(1) + 1,
            12 => r(1) - 2,
            13 => r(1) + 2,
            14 => r(1) - 3,
            _ => r(1) + 3,
        };
        if v <= 0 {
            return Err(FontError::Corrupt("distance"));
        }
        return Ok((v as usize, dcode != 0));
    }
    let x = dcode - 16;
    if x < ndirect {
        return Ok((x + 1, true));
    }
    let x = x - ndirect;
    let nbits = 1 + (x >> (npostfix + 1));
    let hcode = x >> npostfix;
    let lcode = x & ((1usize << npostfix) - 1);
    let offset = ((2 + (hcode & 1)) << nbits) - 4;
    let extra = br.read(nbits as u32)? as usize;
    Ok((((offset + extra) << npostfix) + lcode + ndirect + 1, true))
}

fn at_ref<T>(v: &[T], i: usize) -> Result<&T, FontError> {
    v.get(i).ok_or(FontError::Corrupt("index out of range"))
}

/// Append the dictionary word `word` transformed by transform `id` (RFC 7932 8 and appendix B).
fn transform(word: &[u8], id: usize, out: &mut Vec<u8>) -> Result<(), FontError> {
    let (prefix, kind, suffix) = at(&TRANSFORMS, id)?;
    out.extend_from_slice(at(&AFFIXES, usize::from(prefix))?);
    let mut w = word;
    let kind = usize::from(kind);
    if (1..=9).contains(&kind) {
        w = word.get(..word.len().saturating_sub(kind)).unwrap_or(&[]);
    } else if (12..=20).contains(&kind) {
        w = word.get((kind - 11).min(word.len())..).unwrap_or(&[]);
    }
    let start = out.len();
    out.extend_from_slice(w);
    if kind == 10 || kind == 11 {
        let mut i = start;
        while i < out.len() {
            let step = match out.get_mut(i..) {
                Some(s) => upper(s),
                None => break,
            };
            i += step;
            if kind == 10 {
                break;
            }
        }
    }
    out.extend_from_slice(at(&AFFIXES, usize::from(suffix))?);
    Ok(())
}

/// Upper-case the UTF-8 character at the start of `p` (RFC 7932 8); returns its length.
fn upper(p: &mut [u8]) -> usize {
    let Some(&b0) = p.first() else { return 1 };
    if b0 < 0xC0 {
        if b0.is_ascii_lowercase()
            && let Some(b) = p.first_mut()
        {
            *b ^= 32;
        }
        1
    } else if b0 < 0xE0 {
        if let Some(b) = p.get_mut(1) {
            *b ^= 32;
        }
        2
    } else {
        if let Some(b) = p.get_mut(2) {
            *b ^= 5;
        }
        3
    }
}
