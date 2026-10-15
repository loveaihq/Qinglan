//! Named character references (HTML 13.5, "Named character references") and the numeric
//! reference replacement table (HTML 13.2.5.80).
//!
//! The table is `entities.bin`, generated from the spec's entities.json by tools/gen_entities.py:
//! 2231 names sorted bytewise, packed into 25.8 KB. See that script for the byte layout.

use std::sync::OnceLock;

static DATA: &[u8] = include_bytes!("entities.bin");

/// Offsets of every entry in `DATA`, built on first use (about 9 KB).
fn index() -> &'static [u32] {
    static INDEX: OnceLock<Vec<u32>> = OnceLock::new();
    INDEX.get_or_init(|| {
        let mut v = Vec::with_capacity(2300);
        let mut pos = 0usize;
        while let Some(&len) = DATA.get(pos) {
            v.push(pos as u32);
            let flagged = DATA.get(pos + 1 + len as usize + 2).is_some_and(|b| b & 0x80 != 0);
            pos += 1 + len as usize + 3 + if flagged { 3 } else { 0 };
        }
        v
    })
}

fn entry(idx: usize) -> Option<(&'static [u8], [u32; 2])> {
    let off = *index().get(idx)? as usize;
    let len = *DATA.get(off)? as usize;
    let name = DATA.get(off + 1..off + 1 + len)?;
    let at = off + 1 + len;
    let b = DATA.get(at..at + 3)?;
    let first = u32::from(*b.first()?) | u32::from(*b.get(1)?) << 8 | u32::from(*b.get(2)?) << 16;
    let flagged = first & (1 << 23) != 0;
    let cp1 = first & 0x1F_FFFF;
    let cp2 = if flagged {
        let b = DATA.get(at + 3..at + 6)?;
        u32::from(*b.first()?) | u32::from(*b.get(1)?) << 8 | u32::from(*b.get(2)?) << 16
    } else {
        0
    };
    Some((name, [cp1, cp2]))
}

/// Find the longest named character reference at the start of `input` (which starts right after
/// the `&`). Returns the length of the matched name in bytes (including a trailing `;` if the
/// matched name has one) and the one or two code points (the second is 0 if unused).
pub fn longest_match(input: &[u8]) -> Option<(usize, [u32; 2])> {
    let n = index().len();
    let (mut lo, mut hi) = (0usize, n);
    let mut best = None;
    let mut i = 0usize;
    loop {
        // An entry exactly equal to the prefix sorts first among the entries sharing it.
        if lo < hi
            && i > 0
            && let Some((name, cps)) = entry(lo)
            && name.len() == i
        {
            best = Some((i, cps));
            lo += 1;
        }
        let Some(&b) = input.get(i) else { break };
        let key = |idx: usize| entry(idx).and_then(|(name, _)| name.get(i).copied());
        // Narrow to the entries whose byte i equals `b`.
        let start = partition_point(lo, hi, |idx| key(idx).is_none_or(|k| k < b));
        let end = partition_point(start, hi, |idx| key(idx).is_none_or(|k| k <= b));
        lo = start;
        hi = end;
        if lo >= hi {
            break;
        }
        i += 1;
    }
    best
}

/// First index in `lo..hi` for which `pred` is false (`pred` must be true then false).
fn partition_point(mut lo: usize, mut hi: usize, pred: impl Fn(usize) -> bool) -> usize {
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if pred(mid) {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    lo
}

/// Replacement for numeric references 0x80..=0x9F (HTML 13.2.5.80 "numeric character reference end state"),
/// the windows-1252 mapping. Returns `None` for the five that map to themselves.
pub fn c1_replacement(cp: u32) -> Option<char> {
    let c = match cp {
        0x80 => 0x20AC,
        0x82 => 0x201A,
        0x83 => 0x0192,
        0x84 => 0x201E,
        0x85 => 0x2026,
        0x86 => 0x2020,
        0x87 => 0x2021,
        0x88 => 0x02C6,
        0x89 => 0x2030,
        0x8A => 0x0160,
        0x8B => 0x2039,
        0x8C => 0x0152,
        0x8E => 0x017D,
        0x91 => 0x2018,
        0x92 => 0x2019,
        0x93 => 0x201C,
        0x94 => 0x201D,
        0x95 => 0x2022,
        0x96 => 0x2013,
        0x97 => 0x2014,
        0x98 => 0x02DC,
        0x99 => 0x2122,
        0x9A => 0x0161,
        0x9B => 0x203A,
        0x9C => 0x0153,
        0x9E => 0x017E,
        0x9F => 0x0178,
        _ => return None,
    };
    char::from_u32(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_loads() {
        assert_eq!(index().len(), 2231);
    }

    #[test]
    fn longest() {
        assert_eq!(longest_match(b"amp;x"), Some((4, [0x26, 0])));
        assert_eq!(longest_match(b"ampx"), Some((3, [0x26, 0])));
        assert_eq!(longest_match(b"notit;"), Some((3, [0xAC, 0])));
        assert_eq!(longest_match(b"notin;"), Some((6, [0x2209, 0])));
        assert_eq!(longest_match(b"zzz"), None);
        assert_eq!(longest_match(b"NotEqualTilde;"), Some((14, [0x2242, 0x338])));
        assert_eq!(longest_match(b"am"), None);
    }
}
