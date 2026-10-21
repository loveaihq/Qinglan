//! WOFF2 (W3C WOFF File Format 2.0): the tables of an sfnt in one Brotli stream, with the `glyf`/`loca` and `hmtx`
//! tables in a more compact transformed form (sections 5.1 to 5.4 of the specification), which is undone here.
//!
//! Font collections (`ttcf`) are refused. Every size a file declares is added up and checked against
//! [`MAX_FONT_BYTES`] before the Brotli stream is started, and the stream may not produce more than the sum.

use super::sfnt::{Table, assemble};
use super::{FontError, MAX_FONT_BYTES, MAX_TABLES, Rd, at, be16, brotli};

/// The tags a table directory entry can name by number (WOFF2 3.3).
const KNOWN: [&[u8; 4]; 63] = [
    b"cmap", b"head", b"hhea", b"hmtx", b"maxp", b"name", b"OS/2", b"post", b"cvt ", b"fpgm", b"glyf", b"loca", b"prep", b"CFF ", b"VORG", b"EBDT", b"EBLC", b"gasp", b"hdmx", b"kern", b"LTSH", b"PCLT", b"VDMX", b"vhea",
    b"vmtx", b"BASE", b"GDEF", b"GPOS", b"GSUB", b"EBSC", b"JSTF", b"MATH", b"CBDT", b"CBLC", b"COLR", b"CPAL", b"SVG ", b"sbix", b"acnt", b"avar", b"bdat", b"bloc", b"bsln", b"cvar", b"fdsc", b"feat", b"fmtx", b"fvar",
    b"gvar", b"hsty", b"just", b"lcar", b"mort", b"morx", b"opbd", b"prop", b"trak", b"Zapf", b"Silf", b"Glat", b"Gloc", b"Feat", b"Sill",
];

struct Entry {
    tag: [u8; 4],
    transformed: bool,
    /// Bytes of this table in the Brotli stream.
    stream_len: usize,
}

/// UIntBase128 (WOFF2 6.1.1).
fn base128(r: &mut Rd<'_>) -> Result<u32, FontError> {
    let mut acc = 0u32;
    for i in 0..5 {
        let b = r.u8()?;
        if i == 0 && b == 0x80 {
            return Err(FontError::Corrupt("UIntBase128 leading zero"));
        }
        if acc & 0xFE00_0000 != 0 {
            return Err(FontError::Corrupt("UIntBase128 overflow"));
        }
        acc = (acc << 7) | u32::from(b & 0x7F);
        if b & 0x80 == 0 {
            return Ok(acc);
        }
    }
    Err(FontError::Corrupt("UIntBase128 too long"))
}

/// 255UInt16 (WOFF2 6.1.2).
fn read255(r: &mut Rd<'_>) -> Result<u32, FontError> {
    match r.u8()? {
        253 => Ok(u32::from(r.u16()?)),
        255 => Ok(253 + u32::from(r.u8()?)),
        254 => Ok(506 + u32::from(r.u8()?)),
        c => Ok(u32::from(c)),
    }
}

pub fn decode(data: &[u8]) -> Result<Vec<u8>, FontError> {
    let mut r = Rd::new(data);
    r.take(4)?; // 'wOF2'
    let flavor = r.u32()?;
    let length = r.u32()? as usize;
    let num_tables = usize::from(r.u16()?);
    r.u16()?; // reserved
    let total_sfnt = r.u32()? as usize;
    let total_compressed = r.u32()? as usize;
    r.take(24)?; // version, metadata block, private block
    if length > data.len() {
        return Err(FontError::Truncated);
    }
    if flavor == u32::from_be_bytes(*b"ttcf") {
        return Err(FontError::Unsupported("font collection"));
    }
    if total_sfnt > MAX_FONT_BYTES {
        return Err(FontError::TooLarge);
    }
    if num_tables == 0 || num_tables > MAX_TABLES {
        return Err(FontError::Corrupt("table count"));
    }
    let mut entries = Vec::with_capacity(num_tables);
    let mut stream_total = 0usize;
    for _ in 0..num_tables {
        let flags = r.u8()?;
        let (index, version) = (usize::from(flags & 63), flags >> 6);
        let tag = if index == 63 { r.tag()? } else { **KNOWN.get(index).ok_or(FontError::Corrupt("tag"))? };
        let orig_len = base128(&mut r)? as usize;
        let glyf_loca = tag == *b"glyf" || tag == *b"loca";
        let transformed = if glyf_loca {
            match version {
                0 => true,
                3 => false,
                _ => return Err(FontError::Corrupt("glyf transform version")),
            }
        } else if tag == *b"hmtx" {
            match version {
                0 => false,
                1 => true,
                _ => return Err(FontError::Corrupt("hmtx transform version")),
            }
        } else if version == 0 {
            false
        } else {
            return Err(FontError::Unsupported("table transform"));
        };
        let stream_len = if transformed {
            let t = base128(&mut r)? as usize;
            if tag == *b"loca" && t != 0 {
                return Err(FontError::Corrupt("transformed loca is not empty"));
            }
            t
        } else {
            orig_len
        };
        stream_total = stream_total.saturating_add(stream_len);
        if stream_total > MAX_FONT_BYTES {
            return Err(FontError::TooLarge);
        }
        entries.push(Entry { tag, transformed, stream_len });
    }
    let end = r.p.checked_add(total_compressed).ok_or(FontError::Truncated)?;
    let compressed = data.get(r.p..end).ok_or(FontError::Truncated)?;
    let raw = brotli::decompress(compressed, stream_total)?;
    if raw.len() != stream_total {
        return Err(FontError::Corrupt("table data length"));
    }
    let mut rd = Rd::new(&raw);
    let mut slices: Vec<&[u8]> = Vec::with_capacity(entries.len());
    for e in &entries {
        slices.push(rd.take(e.stream_len)?);
    }
    let find = |tag: &[u8; 4]| entries.iter().zip(&slices).find(|(e, _)| e.tag == *tag).map(|(e, s)| (e.transformed, *s));

    let glyf_t = find(b"glyf");
    let loca_t = find(b"loca");
    let rebuilt = match (glyf_t, loca_t) {
        (Some((true, g)), Some((true, _))) => Some(reconstruct_glyf(g)?),
        (Some((true, _)), _) | (_, Some((true, _))) => return Err(FontError::Corrupt("glyf and loca must both be transformed")),
        _ => None,
    };
    // The tables that go into the font are about to be copied out of `raw`: they have to fit before anything is
    // copied (the rebuilt `glyf` and `loca` are moved, not copied), so that `raw` and the tables together stay
    // under two times the limit, and `raw` goes before the file is written, so that the tables and the file do too.
    let rebuilt_len = rebuilt.as_ref().map_or(0, |g| g.glyf.len() + g.loca.len());
    let plain_len: usize = entries.iter().filter(|e| !e.transformed).map(|e| e.stream_len).sum();
    if rebuilt_len.saturating_add(plain_len) > MAX_FONT_BYTES {
        return Err(FontError::TooLarge);
    }
    let mut rebuilt = rebuilt;
    let mut tables = Vec::with_capacity(entries.len());
    for (e, s) in entries.iter().zip(&slices) {
        let data = if !e.transformed {
            s.to_vec()
        } else if e.tag == *b"glyf" {
            rebuilt.as_mut().map(|g| std::mem::take(&mut g.glyf)).ok_or(FontError::Corrupt("glyf"))?
        } else if e.tag == *b"loca" {
            rebuilt.as_mut().map(|g| std::mem::take(&mut g.loca)).ok_or(FontError::Corrupt("loca"))?
        } else {
            // hmtx: the advances come from the stream, the side bearings may come from the glyph outlines.
            let g = rebuilt.as_ref().ok_or(FontError::Unsupported("hmtx transform without a glyf transform"))?;
            let hhea = find(b"hhea").map(|(_, d)| d).ok_or(FontError::Corrupt("hmtx without hhea"))?;
            let num_hm = usize::from(be16(hhea, 34).ok_or(FontError::Corrupt("hhea"))?);
            reconstruct_hmtx(s, g.xmin.len(), num_hm, &g.xmin)?
        };
        tables.push(Table { tag: e.tag, data });
    }
    drop(rebuilt);
    drop(slices);
    drop(raw);
    assemble(flavor, tables)
}

struct Glyf {
    glyf: Vec<u8>,
    loca: Vec<u8>,
    /// xMin of each glyph (0 for an empty one), for the `hmtx` transform.
    xmin: Vec<i16>,
}

/// Undo the `glyf` transform (WOFF2 5.1): the streams are read in step, one glyph at a time.
fn reconstruct_glyf(t: &[u8]) -> Result<Glyf, FontError> {
    let mut h = Rd::new(t);
    h.u16()?; // reserved
    let option_flags = h.u16()?;
    let num_glyphs = usize::from(h.u16()?);
    let index_format = h.u16()?;
    if index_format > 1 {
        return Err(FontError::Corrupt("index format"));
    }
    let mut sizes = [0usize; 7];
    for s in &mut sizes {
        *s = h.u32()? as usize;
    }
    let mut take = |i: usize| -> Result<Rd<'_>, FontError> { Ok(Rd::new(h.take(at(&sizes, i)?)?)) };
    let mut contours = take(0)?;
    let mut points = take(1)?;
    let mut flags = take(2)?;
    let mut glyph = take(3)?;
    let mut composite = take(4)?;
    let mut bbox = take(5)?;
    let mut instr = take(6)?;
    let overlap: &[u8] = if option_flags & 1 != 0 { h.take(num_glyphs.div_ceil(8))? } else { &[] };
    let bitmap = bbox.take(4 * num_glyphs.div_ceil(32))?;
    let bit = |m: &[u8], i: usize| m.get(i >> 3).is_some_and(|b| b & (0x80 >> (i & 7)) != 0);

    // The glyf table is about one and a half times its transform: allocated once, so that growing it never asks for
    // twice the limit (and never holds the old and the new buffer when it is full-sized).
    let mut glyf: Vec<u8> = Vec::with_capacity((t.len() / 2 * 3).min(MAX_FONT_BYTES));
    let mut offsets: Vec<usize> = Vec::with_capacity(num_glyphs + 1);
    let mut xmin: Vec<i16> = Vec::with_capacity(num_glyphs);
    offsets.push(0);
    for i in 0..num_glyphs {
        let ncont = contours.u16()? as i16;
        let have_bbox = bit(bitmap, i);
        let mut x_min = 0i16;
        if ncont == 0 {
            if have_bbox {
                return Err(FontError::Corrupt("empty glyph with a bounding box"));
            }
        } else if ncont > 0 {
            let n = ncont as usize;
            let mut end_pts: Vec<u16> = Vec::with_capacity(n);
            let mut total = 0usize;
            for _ in 0..n {
                let np = read255(&mut points)? as usize;
                if np == 0 {
                    return Err(FontError::Corrupt("contour without points"));
                }
                total += np;
                if total > 0xFFFF {
                    return Err(FontError::Corrupt("too many points"));
                }
                end_pts.push((total - 1) as u16);
            }
            let flag_bytes = flags.take(total)?;
            let mut xy: Vec<(i32, i32)> = Vec::with_capacity(total);
            let mut on: Vec<bool> = Vec::with_capacity(total);
            let (mut x, mut y) = (0i32, 0i32);
            for &f in flag_bytes {
                let (dx, dy) = triplet(f & 0x7F, &mut glyph)?;
                x += dx;
                y += dy;
                if i16::try_from(x).is_err() || i16::try_from(y).is_err() {
                    return Err(FontError::Corrupt("coordinate out of range"));
                }
                xy.push((x, y));
                on.push(f & 0x80 == 0);
            }
            let ilen = read255(&mut glyph)? as usize;
            let code = instr.take(ilen)?;
            let (x0, y0, x1, y1) = if have_bbox {
                (bbox.u16()? as i16, bbox.u16()? as i16, bbox.u16()? as i16, bbox.u16()? as i16)
            } else {
                let xs = xy.iter().map(|p| p.0);
                let ys = xy.iter().map(|p| p.1);
                (xs.clone().min().unwrap_or(0) as i16, ys.clone().min().unwrap_or(0) as i16, xs.max().unwrap_or(0) as i16, ys.max().unwrap_or(0) as i16)
            };
            x_min = x0;
            glyf.extend_from_slice(&ncont.to_be_bytes());
            for v in [x0, y0, x1, y1] {
                glyf.extend_from_slice(&v.to_be_bytes());
            }
            for e in &end_pts {
                glyf.extend_from_slice(&e.to_be_bytes());
            }
            glyf.extend_from_slice(&(ilen as u16).to_be_bytes());
            glyf.extend_from_slice(code);
            encode_points(&mut glyf, &xy, &on, bit(overlap, i))?;
        } else if ncont == -1 {
            if !have_bbox {
                return Err(FontError::Corrupt("composite glyph without a bounding box"));
            }
            let mut comp: Vec<u8> = Vec::new();
            let mut have_instr = false;
            loop {
                let start = composite.p;
                let fl = composite.u16()?;
                composite.u16()?;
                let mut sz = if fl & 1 != 0 { 4 } else { 2 };
                if fl & 8 != 0 {
                    sz += 2;
                } else if fl & 0x40 != 0 {
                    sz += 4;
                } else if fl & 0x80 != 0 {
                    sz += 8;
                }
                composite.take(sz)?;
                comp.extend_from_slice(composite.d.get(start..composite.p).ok_or(FontError::Truncated)?);
                have_instr |= fl & 0x100 != 0;
                if fl & 0x20 == 0 {
                    break;
                }
            }
            let bb = [bbox.u16()?, bbox.u16()?, bbox.u16()?, bbox.u16()?];
            x_min = at(&bb, 0)? as i16;
            glyf.extend_from_slice(&ncont.to_be_bytes());
            for v in bb {
                glyf.extend_from_slice(&v.to_be_bytes());
            }
            glyf.extend_from_slice(&comp);
            if have_instr {
                let ilen = read255(&mut glyph)? as usize;
                glyf.extend_from_slice(&(ilen as u16).to_be_bytes());
                glyf.extend_from_slice(instr.take(ilen)?);
            }
        } else {
            return Err(FontError::Corrupt("number of contours"));
        }
        glyf.resize((glyf.len() + 3) & !3, 0);
        if glyf.len() > MAX_FONT_BYTES {
            return Err(FontError::TooLarge);
        }
        offsets.push(glyf.len());
        xmin.push(x_min);
    }
    let mut loca = Vec::with_capacity(offsets.len() * if index_format == 0 { 2 } else { 4 });
    for &o in &offsets {
        if index_format == 0 {
            let half = u16::try_from(o / 2).map_err(|_| FontError::Corrupt("glyf too long for short loca"))?;
            loca.extend_from_slice(&half.to_be_bytes());
        } else {
            loca.extend_from_slice(&(o as u32).to_be_bytes());
        }
    }
    Ok(Glyf { glyf, loca, xmin })
}

/// One coordinate pair from the glyph stream (WOFF2 5.2, "triplet encoding").
fn triplet(f: u8, glyph: &mut Rd<'_>) -> Result<(i32, i32), FontError> {
    let f = u32::from(f);
    let n = if f < 84 {
        1
    } else if f < 120 {
        2
    } else if f < 124 {
        3
    } else {
        4
    };
    let b = glyph.take(n)?;
    let by = |i: usize| -> Result<i32, FontError> { Ok(i32::from(at(b, i)?)) };
    let sign = |flag: u32, base: i32| if flag & 1 != 0 { base } else { -base };
    Ok(if f < 10 {
        (0, sign(f, (((f & 14) as i32) << 7) + by(0)?))
    } else if f < 20 {
        (sign(f, ((((f - 10) & 14) as i32) << 7) + by(0)?), 0)
    } else if f < 84 {
        let b0 = (f - 20) as i32;
        let b1 = by(0)?;
        (sign(f, 1 + (b0 & 0x30) + (b1 >> 4)), sign(f >> 1, 1 + ((b0 & 0x0C) << 2) + (b1 & 0x0F)))
    } else if f < 120 {
        let b0 = (f - 84) as i32;
        (sign(f, 1 + ((b0 / 12) << 8) + by(0)?), sign(f >> 1, 1 + (((b0 % 12) >> 2) << 8) + by(1)?))
    } else if f < 124 {
        let b2 = by(1)?;
        (sign(f, (by(0)? << 4) + (b2 >> 4)), sign(f >> 1, ((b2 & 0x0F) << 8) + by(2)?))
    } else {
        (sign(f, (by(0)? << 8) + by(1)?), sign(f >> 1, (by(2)? << 8) + by(3)?))
    })
}

/// The flags and coordinates of a simple glyph in the `glyf` table's own encoding (OpenType glyf, simple glyph).
fn encode_points(out: &mut Vec<u8>, xy: &[(i32, i32)], on: &[bool], overlap: bool) -> Result<(), FontError> {
    let mut flags: Vec<u8> = Vec::with_capacity(xy.len());
    let mut xs: Vec<u8> = Vec::new();
    let mut ys: Vec<u8> = Vec::new();
    let (mut px, mut py) = (0i32, 0i32);
    for (i, (&(x, y), &o)) in xy.iter().zip(on).enumerate() {
        let mut f = u8::from(o);
        if overlap && i == 0 {
            f |= 0x40;
        }
        let (dx, dy) = (x - px, y - py);
        (px, py) = (x, y);
        if dx == 0 {
            f |= 0x10;
        } else if dx.abs() < 256 {
            f |= 0x02 | if dx > 0 { 0x10 } else { 0 };
            xs.push(dx.unsigned_abs() as u8);
        } else {
            // The coordinates are in the 16-bit range but their difference need not be: a delta that does not fit
            // cannot be written in a glyf table.
            xs.extend_from_slice(&i16::try_from(dx).map_err(|_| FontError::Corrupt("coordinate delta out of range"))?.to_be_bytes());
        }
        if dy == 0 {
            f |= 0x20;
        } else if dy.abs() < 256 {
            f |= 0x04 | if dy > 0 { 0x20 } else { 0 };
            ys.push(dy.unsigned_abs() as u8);
        } else {
            ys.extend_from_slice(&i16::try_from(dy).map_err(|_| FontError::Corrupt("coordinate delta out of range"))?.to_be_bytes());
        }
        flags.push(f);
    }
    let mut i = 0;
    while let Some(&f) = flags.get(i) {
        let mut run = 1;
        while run < 256 && flags.get(i + run) == Some(&f) {
            run += 1;
        }
        if run >= 2 {
            out.push(f | 0x08);
            out.push((run - 1) as u8);
        } else {
            out.push(f);
        }
        i += run;
    }
    out.extend_from_slice(&xs);
    out.extend_from_slice(&ys);
    Ok(())
}

/// Undo the `hmtx` transform (WOFF2 5.4).
fn reconstruct_hmtx(t: &[u8], num_glyphs: usize, num_hm: usize, xmin: &[i16]) -> Result<Vec<u8>, FontError> {
    let mut r = Rd::new(t);
    let flags = r.u8()?;
    if flags & 0xFC != 0 || num_hm == 0 || num_hm > num_glyphs {
        return Err(FontError::Corrupt("hmtx transform"));
    }
    let adv = r.take(2 * num_hm)?;
    let lsb = if flags & 1 == 0 { Some(r.take(2 * num_hm)?) } else { None };
    let lsb2 = if flags & 2 == 0 { Some(r.take(2 * (num_glyphs - num_hm))?) } else { None };
    let mut out = Vec::with_capacity(2 * num_glyphs + 2 * num_hm);
    for i in 0..num_hm {
        out.extend_from_slice(adv.get(2 * i..2 * i + 2).ok_or(FontError::Truncated)?);
        match lsb {
            Some(b) => out.extend_from_slice(b.get(2 * i..2 * i + 2).ok_or(FontError::Truncated)?),
            None => out.extend_from_slice(&at(xmin, i)?.to_be_bytes()),
        }
    }
    for i in num_hm..num_glyphs {
        match lsb2 {
            Some(b) => out.extend_from_slice(b.get(2 * (i - num_hm)..2 * (i - num_hm) + 2).ok_or(FontError::Truncated)?),
            None => out.extend_from_slice(&at(xmin, i)?.to_be_bytes()),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A transformed `glyf` table of one glyph: one contour of two points, the first at x = -30000 and the second `dx`
    /// to the right of it (the triplet encoding can say a delta up to 65535; the glyf table itself can only say 16 bits).
    fn glyph_with_second_delta(dx: u16) -> Vec<u8> {
        let mut t = vec![0, 0, 0, 0, 0, 1, 0, 1];
        for size in [2u32, 1, 2, 9, 0, 4, 0] {
            t.extend_from_slice(&size.to_be_bytes());
        }
        t.extend_from_slice(&[0, 1]); // one contour
        t.push(2); // two points
        t.extend_from_slice(&[124, 125]); // 4-byte triplets: the first one negative, the second positive
        t.extend_from_slice(&[0x75, 0x30, 0, 0]); // x = -30000, y = 0
        t.extend_from_slice(&dx.to_be_bytes());
        t.extend_from_slice(&[0, 0, 0]); // dy = 0, no instructions
        t.extend_from_slice(&[0, 0, 0, 0]); // the bounding box bitmap: none given
        t
    }

    #[test]
    fn a_delta_that_does_not_fit_in_16_bits_is_corrupt_not_truncated() {
        assert!(reconstruct_glyf(&glyph_with_second_delta(20_000)).is_ok());
        assert!(matches!(reconstruct_glyf(&glyph_with_second_delta(60_000)), Err(FontError::Corrupt("coordinate delta out of range"))));
    }
}
