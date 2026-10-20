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
}
