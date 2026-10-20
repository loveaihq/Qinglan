//! WOFF 1.0 (W3C WOFF File Format 1.0): an sfnt whose tables are each zlib-compressed, behind a 44-byte header and a
//! 20-byte directory entry per table.

use super::inflate;
use super::sfnt::{Table, assemble};
use super::{FontError, MAX_FONT_BYTES, MAX_TABLES, Rd};

pub fn decode(data: &[u8]) -> Result<Vec<u8>, FontError> {
    let mut r = Rd::new(data);
    r.take(4)?; // 'wOFF'
    let flavor = r.u32()?;
    let length = r.u32()? as usize;
    let num_tables = usize::from(r.u16()?);
    r.u16()?; // reserved
    let total_sfnt_size = r.u32()? as usize;
    if length > data.len() || length < 44 {
        return Err(FontError::Corrupt("WOFF length"));
    }
    // What the file says it will unpack to is checked before any table is touched.
    if total_sfnt_size > MAX_FONT_BYTES {
        return Err(FontError::TooLarge);
    }
    if num_tables == 0 || num_tables > MAX_TABLES {
        return Err(FontError::Corrupt("table count"));
    }
    let mut dir = Rd::new(data.get(44..).ok_or(FontError::Truncated)?);
    let mut entries = Vec::with_capacity(num_tables);
    let mut declared = 0usize;
    for _ in 0..num_tables {
        let tag = dir.tag()?;
        let (off, comp, orig) = (dir.u32()? as usize, dir.u32()? as usize, dir.u32()? as usize);
        dir.u32()?; // checksum: recomputed
        if comp > orig {
            return Err(FontError::Corrupt("compressed table longer than its original"));
        }
        declared = declared.saturating_add(orig.saturating_add(3) & !3);
        if declared > MAX_FONT_BYTES {
            return Err(FontError::TooLarge);
        }
        let end = off.checked_add(comp).ok_or(FontError::Truncated)?;
        if end > length {
            return Err(FontError::Corrupt("table outside the file"));
        }
        entries.push((tag, off, comp, orig));
    }
    let mut tables = Vec::with_capacity(num_tables);
    for (tag, off, comp, orig) in entries {
        let raw = data.get(off..off + comp).ok_or(FontError::Truncated)?;
        let out = if comp == orig {
            raw.to_vec()
        } else {
            let t = inflate::zlib(raw, orig)?;
            if t.len() != orig {
                return Err(FontError::Corrupt("table length"));
            }
            t
        };
        tables.push(Table { tag, data: out });
    }
    assemble(flavor, tables)
}
