//! Writing an sfnt (TrueType / OpenType) file from tables: the output of both WOFF decoders.
//!
//! The tables are written in order of their tags, each padded to four bytes, with the checksums of the table
//! directory and of the whole file (`head.checkSumAdjustment`) computed here, so the result does not depend on how the
//! container ordered or described the tables.

use super::{FontError, MAX_FONT_BYTES, MAX_TABLES};

pub struct Table {
    pub tag: [u8; 4],
    pub data: Vec<u8>,
}

fn checksum(data: &[u8]) -> u32 {
    let mut sum = 0u32;
    let mut chunks = data.chunks_exact(4);
    for c in chunks.by_ref() {
        sum = sum.wrapping_add(u32::from_be_bytes([c.first().copied().unwrap_or(0), c.get(1).copied().unwrap_or(0), c.get(2).copied().unwrap_or(0), c.get(3).copied().unwrap_or(0)]));
    }
    let rest = chunks.remainder();
    if !rest.is_empty() {
        let mut w = [0u8; 4];
        for (d, s) in w.iter_mut().zip(rest) {
            *d = *s;
        }
        sum = sum.wrapping_add(u32::from_be_bytes(w));
    }
    sum
}

pub fn assemble(flavor: u32, mut tables: Vec<Table>) -> Result<Vec<u8>, FontError> {
    let n = tables.len();
    if n == 0 || n > MAX_TABLES {
        return Err(FontError::Corrupt("table count"));
    }
    tables.sort_by_key(|t| t.tag);
    if tables.windows(2).any(|w| w.first().map(|a| a.tag) == w.get(1).map(|b| b.tag)) {
        return Err(FontError::Corrupt("duplicate table"));
    }
    let mut total = 12 + 16 * n;
    for t in &tables {
        total = total.saturating_add(t.data.len().saturating_add(3) & !3);
        if total > MAX_FONT_BYTES {
            return Err(FontError::TooLarge);
        }
    }
    let mut entry = 0u16;
    while (2usize << entry) <= n {
        entry += 1;
    }
    let search = (1u16 << entry) * 16;
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(&flavor.to_be_bytes());
    out.extend_from_slice(&(n as u16).to_be_bytes());
    out.extend_from_slice(&search.to_be_bytes());
    out.extend_from_slice(&entry.to_be_bytes());
    out.extend_from_slice(&((n as u16) * 16 - search).to_be_bytes());
    let mut offset = 12 + 16 * n;
    let mut head_at = None;
    for t in &mut tables {
        // The checksum of `head` is taken with checkSumAdjustment zero.
        if t.tag == *b"head" && t.data.len() >= 12 {
            head_at = Some(offset);
            for b in t.data.iter_mut().skip(8).take(4) {
                *b = 0;
            }
        }
        out.extend_from_slice(&t.tag);
        out.extend_from_slice(&checksum(&t.data).to_be_bytes());
        out.extend_from_slice(&(offset as u32).to_be_bytes());
        out.extend_from_slice(&(t.data.len() as u32).to_be_bytes());
        offset += (t.data.len() + 3) & !3;
    }
    for t in &tables {
        out.extend_from_slice(&t.data);
        out.resize((out.len() + 3) & !3, 0);
    }
    if let Some(h) = head_at {
        let adj = 0xB1B0_AFBAu32.wrapping_sub(checksum(&out)).to_be_bytes();
        for (d, s) in out.iter_mut().skip(h + 8).take(4).zip(adj) {
            *d = s;
        }
    }
    Ok(out)
}
