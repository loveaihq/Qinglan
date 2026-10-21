//! Small replacements for the C library's `sin`, `cos`, `pow`, `atan2f` and `logf`. With their argument reduction, special cases
//! and tables they are about 20 KB of the executable (2c-1 budget); gradients, arcs, strokes and the color spaces need an
//! 8-bit channel (a step of 4e-3) at most, so short series do. Every function returns a finite number for a finite input.

use std::f32::consts::{FRAC_PI_2, PI};

/// `(sin x, cos x)` for `x` in radians: Taylor series of the remainder after the nearest multiple of pi/2 (`|r| <= pi/4`,
/// error below 1e-12); 0 and 1 for an argument beyond 1e8 or not a number.
pub fn sin_cos(x: f64) -> (f64, f64) {
    if x.is_nan() || x.abs() >= 1.0e8 {
        return (0.0, 1.0);
    }
    let q = (x * std::f64::consts::FRAC_2_PI).round();
    let r = x - q * std::f64::consts::FRAC_PI_2;
    let (mut s, mut c, mut term) = (r, 1.0, 1.0);
    for k in 1..=7 {
        // term = r^(2k) / (2k)!
        term *= r * r / (f64::from(2 * k - 1) * f64::from(2 * k));
        let sign = if k % 2 == 1 { -1.0 } else { 1.0 };
        c += sign * term;
        s += sign * term * r / f64::from(2 * k + 1);
    }
    match (q as i64).rem_euclid(4) {
        0 => (s, c),
        1 => (c, -s),
        2 => (-s, -c),
        _ => (-c, s),
    }
}

/// `x^y` for a positive finite `x`, as `exp2(y * log2 x)`: the logarithm from `ln m = 2 atanh z` with `z = (m - 1) / (m + 1)`
/// for the mantissa `m` in `[sqrt(1/2), sqrt(2))`, the power of two from the exponent and the Taylor series of `exp` on
/// `0 ..= ln 2`. Relative error below 1e-9 for results in `1e-30 ..= 1e30`; the result is clamped to that range.
pub fn pow(x: f64, y: f64) -> f64 {
    let bits = x.max(1.0e-300).to_bits();
    let mut ex = ((bits >> 52) & 0x7ff) as i64 - 1023;
    let mut m = f64::from_bits((bits & 0x000f_ffff_ffff_ffff) | 0x3ff0_0000_0000_0000);
    if m > std::f64::consts::SQRT_2 {
        m /= 2.0;
        ex += 1;
    }
    let z = (m - 1.0) / (m + 1.0);
    let z2 = z * z;
    let mut series = 0.0;
    for k in (0..6).rev() {
        series = series * z2 + 1.0 / f64::from(2 * k + 1);
    }
    let l = (ex as f64 + 2.0 * z * series * std::f64::consts::LOG2_E) * y;
    let l = if l.is_nan() { 0.0 } else { l.clamp(-100.0, 100.0) };
    let (yi, yf) = (l.floor(), l - l.floor());
    let u = yf * std::f64::consts::LN_2;
    let (mut e, mut term) = (1.0, 1.0);
    for k in 1..=13 {
        term *= u / f64::from(k);
        e += term;
    }
    e * f64::from_bits(((yi as i64 + 1023) as u64) << 52)
}

/// `log2 t` for a positive finite `t` (`t` below 1e-30 counts as 1e-30), good to about 2e-5: the exponent and `ln m = 2 atanh z`
/// with `z = (m - 1) / (m + 1) <= 1/3` for the mantissa `m` in 1..2.
pub fn log2(t: f32) -> f32 {
    let bits = t.max(1.0e-30).to_bits();
    let ex = ((bits >> 23) & 0xff) as i32 - 127;
    let m = f32::from_bits((bits & 0x007f_ffff) | 0x3f80_0000);
    let z = (m - 1.0) / (m + 1.0);
    let z2 = z * z;
    ex as f32 + 2.0 * z * (1.0 + z2 * (1.0 / 3.0 + z2 * (0.2 + z2 / 7.0))) * std::f32::consts::LOG2_E
}

/// The angle of the vector (`x`, `y`) counterclockwise from the positive x axis, in `-pi..=pi` (the arctangent series of
/// Abramowitz and Stegun 4.4.49, error 2e-8 on 0..=1).
pub fn atan2(y: f32, x: f32) -> f32 {
    let (ax, ay) = (x.abs(), y.abs());
    let (lo, hi) = if ax < ay { (ax, ay) } else { (ay, ax) };
    let z = if hi > 0.0 { (lo / hi).min(1.0) } else { 0.0 };
    let z2 = z * z;
    let mut a = z
        * (1.0 + z2 * (-0.333_331_45 + z2 * (0.199_935_51 + z2 * (-0.142_089 + z2 * (0.106_562_64 + z2 * (-0.075_289_64 + z2 * (0.042_909_614 + z2 * (-0.016_165_737 + z2 * 0.002_866_225_7))))))));
    if ay > ax {
        a = FRAC_PI_2 - a;
    }
    if x < 0.0 {
        a = PI - a;
    }
    if y < 0.0 { -a } else { a }
}

/// `clamp` without the panic message. `f32::clamp` and `f64::clamp` assert `min <= max` with a message that prints both
/// bounds with `{:?}`, which pulls the whole float-to-text code (about 20 KB) into every program that clamps a float.
/// The result is what `clamp` returns whenever `min <= max`; a reversed pair gives `max`.
pub trait Fclamp: Copy + PartialOrd {
    fn fclamp(self, min: Self, max: Self) -> Self;
}

macro_rules! fclamp_impl {
    ($($t:ty),*) => {$(
        impl Fclamp for $t {
            #[inline]
            #[allow(clippy::manual_clamp)]
            fn fclamp(self, min: $t, max: $t) -> $t {
                let mut v = self;
                if v < min {
                    v = min;
                }
                if v > max {
                    v = max;
                }
                v
            }
        }
    )*};
}
fclamp_impl!(f32, f64, i32, i64, u32, u64, usize, isize, u8, u16);

/// `format!("{v:.decimals$}")` for an `f32`: the exact binary value rounded to `decimals` places, ties to the even digit,
/// by integer arithmetic.
/// Values of 2^100 and more are written as integers with zero decimals.
pub fn fixed_f32(v: f32, decimals: u32) -> String {
    if v.is_nan() {
        return "NaN".to_string();
    }
    if v.is_infinite() {
        return if v < 0.0 { "-inf" } else { "inf" }.to_string();
    }
    let bits = v.to_bits();
    let neg = bits >> 31 == 1;
    let exp = ((bits >> 23) & 0xff) as i32;
    let frac = bits & 0x7f_ffff;
    let (m, e) = if exp == 0 { (frac, -149) } else { (frac | 0x80_0000, exp - 150) };
    let d = decimals.min(9);
    let scale = 10u128.pow(d);
    let n = u128::from(m) * scale;
    let q: u128 = if e >= 0 {
        if e > 60 { u128::from(m) << e.min(100) } else { n << e }
    } else {
        let k = (-e) as u32;
        if k >= 120 {
            0
        } else {
            let q = n >> k;
            let rem = n & ((1u128 << k) - 1);
            let half = 1u128 << (k - 1);
            if rem > half || (rem == half && q & 1 == 1) { q + 1 } else { q }
        }
    };
    let (int, rest) = if e > 60 { (q, 0) } else { (q / scale, q % scale) };
    let sign = if neg && (q > 0 || v.to_bits() >> 31 == 1) { "-" } else { "" };
    if d == 0 { format!("{sign}{int}") } else { format!("{sign}{int}.{rest:0w$}", w = d as usize) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trigonometry_matches_the_c_library() {
        for k in -4000..=4000 {
            let x = f64::from(k) * 0.0123;
            let (s, c) = sin_cos(x);
            assert!((s - x.sin()).abs() < 1.0e-11 && (c - x.cos()).abs() < 1.0e-11, "{x}: {s} {c} vs {} {}", x.sin(), x.cos());
        }
        for (y, x) in [(1.0f32, 0.0f32), (-1.0, 0.0), (0.0, -1.0), (0.0, 1.0), (0.0, 0.0), (3.0, -4.0), (-3.0, -4.0), (-3.0, 4.0), (1.0e-20, 1.0e20), (5.0, 5.0)] {
            assert!((atan2(y, x) - y.atan2(x)).abs() < 2.0e-6, "atan2({y}, {x}): {} vs {}", atan2(y, x), y.atan2(x));
        }
        for t in [1.0e-6f32, 0.001, 0.1, 0.5, 0.9, 1.0, 1.5, 7.0, 1.0e6] {
            assert!((log2(t) - t.log2()).abs() < 2.0e-4, "log2({t}): {} vs {}", log2(t), t.log2());
        }
        assert_eq!(sin_cos(f64::NAN), (0.0, 1.0));
        assert_eq!(sin_cos(1.0e30), (0.0, 1.0));
    }

    #[test]
    fn powers_match_the_c_library() {
        for x in [1.0e-6, 0.001, 0.0031308, 0.04, 0.2, 0.5, 0.9, 1.0, 1.5, 2.0, 10.0, 1234.5] {
            for y in [1.0 / 2.4, 2.4, 0.5, 3.0, -1.0, 0.0, 0.3333333333, -2.2] {
                let (a, b) = (pow(x, y), f64::powf(x, y));
                assert!(((a - b) / b).abs() < 1.0e-9, "{x}^{y}: {a} vs {b}");
            }
        }
        assert!(pow(f64::NAN, 2.0).is_finite() && pow(0.0, 2.0).is_finite() && pow(5.0, f64::INFINITY).is_finite());
    }

    #[test]
    fn fixed_f32_writes_what_the_standard_library_writes() {
        let mut x: u32 = 0x1234_5678;
        let mut cases: Vec<f32> = vec![0.0, -0.0, 0.5, 1.5, 0.03125, 0.09375, 0.0005, 0.00005, 1.0e-10, 123456.8, 1.0e9, 3.0e9, 255.0 / 255.0, 0.999_5, 2.5, 0.125, 1.0e-45];
        for a in 0..=255u32 {
            cases.push(a as f32 / 255.0);
        }
        for _ in 0..60_000 {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            let f = f32::from_bits(x);
            if f.is_finite() && f.abs() < 1.0e12 {
                cases.push(f);
            }
            cases.push((x % 2_000_000) as f32 / 1000.0 - 1000.0);
            cases.push((x % 100_000) as f32 / 1024.0);
        }
        for v in cases {
            for d in [0u32, 1, 3, 4] {
                assert_eq!(fixed_f32(v, d), format!("{v:.d$}", d = d as usize), "{v:?} with {d} decimals");
            }
        }
        assert_eq!(fixed_f32(f32::NAN, 3), "NaN");
        assert_eq!(fixed_f32(f32::NEG_INFINITY, 3), "-inf");
    }

    #[test]
    fn fclamp_is_clamp() {
        for (v, lo, hi) in [(5.0f32, 0.0, 3.0), (-1.0, 0.0, 3.0), (2.0, 0.0, 3.0), (f32::NAN, 0.0, 1.0), (0.0, -0.0, 0.0)] {
            assert_eq!(v.fclamp(lo, hi).to_bits(), v.clamp(lo, hi).to_bits());
        }
        assert_eq!(7i32.fclamp(1, 3), 3);
        assert_eq!(0usize.fclamp(1, 3), 1);
        assert_eq!(5.0f64.fclamp(2.0, 1.0), 1.0);
    }
}
