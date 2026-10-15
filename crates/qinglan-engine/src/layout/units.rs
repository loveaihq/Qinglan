//! Layout units: integers in 1/64 CSS px (L1 task 10), like Blink's `LayoutUnit`, so that rounding
//! errors do not pile up. All arithmetic saturates at +-2^24 px, so hostile lengths (1e9px) cannot
//! overflow.

use std::ops::{Add, AddAssign, Neg, Sub, SubAssign};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub struct LayoutUnit(i32);

/// Raw bound: 2^24 px.
const RAW_MAX: i64 = 1 << 30;
const SHIFT: i64 = 6;

impl LayoutUnit {
    pub const ZERO: LayoutUnit = LayoutUnit(0);
    pub const MAX: LayoutUnit = LayoutUnit(RAW_MAX as i32);
    pub const MIN: LayoutUnit = LayoutUnit(-(RAW_MAX as i32));

    fn sat(v: i64) -> LayoutUnit {
        LayoutUnit(v.clamp(-RAW_MAX, RAW_MAX) as i32)
    }

    pub fn from_raw(raw: i64) -> LayoutUnit {
        Self::sat(raw)
    }

    pub fn raw(self) -> i32 {
        self.0
    }

    /// Whole pixels.
    pub fn px(v: i32) -> LayoutUnit {
        Self::sat((v as i64) << SHIFT)
    }

    /// From a px value, rounded to the nearest 1/64. NaN is 0.
    pub fn from_f32(px: f32) -> LayoutUnit {
        if px.is_nan() {
            return LayoutUnit::ZERO;
        }
        Self::sat((px as f64 * 64.0).round() as i64)
    }

    /// From a px value, rounded up (widths of text, so that a text never looks narrower than it is).
    pub fn from_f32_ceil(px: f32) -> LayoutUnit {
        if px.is_nan() {
            return LayoutUnit::ZERO;
        }
        Self::sat((px as f64 * 64.0).ceil() as i64)
    }

    pub fn to_f32(self) -> f32 {
        self.0 as f32 / 64.0
    }

    /// Round down to whole pixels.
    pub fn floor_px(self) -> LayoutUnit {
        LayoutUnit(self.0.div_euclid(64) * 64)
    }

    /// Round to the nearest whole pixel (halves up).
    pub fn round_px(self) -> LayoutUnit {
        LayoutUnit((self.0 + 32).div_euclid(64) * 64)
    }

    /// Half of the value, rounded toward zero in 1/64 units.
    pub fn half(self) -> LayoutUnit {
        LayoutUnit(self.0 / 2)
    }

    pub fn max0(self) -> LayoutUnit {
        LayoutUnit(self.0.max(0))
    }
}

impl Add for LayoutUnit {
    type Output = LayoutUnit;
    fn add(self, o: LayoutUnit) -> LayoutUnit {
        Self::sat(self.0 as i64 + o.0 as i64)
    }
}

impl Sub for LayoutUnit {
    type Output = LayoutUnit;
    fn sub(self, o: LayoutUnit) -> LayoutUnit {
        Self::sat(self.0 as i64 - o.0 as i64)
    }
}

impl Neg for LayoutUnit {
    type Output = LayoutUnit;
    fn neg(self) -> LayoutUnit {
        LayoutUnit(-self.0)
    }
}

impl AddAssign for LayoutUnit {
    fn add_assign(&mut self, o: LayoutUnit) {
        *self = *self + o;
    }
}

impl SubAssign for LayoutUnit {
    fn sub_assign(&mut self, o: LayoutUnit) {
        *self = *self - o;
    }
}

/// A (possibly collapsed) vertical margin: the largest positive and the most negative margin that
/// meet at one place (CSS2 8.3.1: "the maximum of the positive margins plus the minimum of the
/// negative ones"). `neg` is kept as a positive magnitude.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub struct Margin {
    pub pos: LayoutUnit,
    pub neg: LayoutUnit,
}

impl Margin {
    pub fn from(v: LayoutUnit) -> Margin {
        if v >= LayoutUnit::ZERO { Margin { pos: v, neg: LayoutUnit::ZERO } } else { Margin { pos: LayoutUnit::ZERO, neg: -v } }
    }

    /// Two margins that become adjoining.
    pub fn join(self, o: Margin) -> Margin {
        Margin { pos: self.pos.max(o.pos), neg: self.neg.max(o.neg) }
    }

    /// The resulting margin.
    pub fn resolve(self) -> LayoutUnit {
        self.pos - self.neg
    }
}

/// Text for dumps: whole pixels, or up to six decimals (1/64 = 0.015625), no trailing zeros.
pub fn fmt_lu(v: LayoutUnit) -> String {
    let raw = v.raw() as i64;
    let neg = raw < 0;
    let a = raw.abs();
    let (whole, frac) = (a / 64, a % 64);
    let mut s = String::new();
    if neg && (whole != 0 || frac != 0) {
        s.push('-');
    }
    s.push_str(&whole.to_string());
    if frac != 0 {
        let micro = frac * 15625;
        let digits = format!("{micro:06}");
        s.push('.');
        s.push_str(digits.trim_end_matches('0'));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_and_rounds() {
        assert_eq!(fmt_lu(LayoutUnit::px(16)), "16");
        assert_eq!(fmt_lu(LayoutUnit::from_f32(12.8)), "12.796875");
        assert_eq!(fmt_lu(LayoutUnit::from_raw(-32)), "-0.5");
        assert_eq!(LayoutUnit::from_f32(1.0e12), LayoutUnit::MAX);
        assert_eq!((LayoutUnit::MAX + LayoutUnit::MAX), LayoutUnit::MAX);
        assert_eq!(LayoutUnit::from_raw(-65).floor_px(), LayoutUnit::px(-2));
    }

    #[test]
    fn margins_collapse() {
        let m = Margin::from(LayoutUnit::px(20)).join(Margin::from(LayoutUnit::px(-8))).join(Margin::from(LayoutUnit::px(30)));
        assert_eq!(m.resolve(), LayoutUnit::px(22));
    }
}
