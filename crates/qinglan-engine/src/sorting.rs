//! Stable sorting by an integer key through one shared sort.
//!
//! `sort_by_key` and `sort_by` compile a whole copy of the sorting code (quicksort, merge, small-sort networks) for
//! every closure type: 6 to 15 KB each, about 50 KB for the ten places that sort in the engine and the window.
//! Here the keys go into `(key, index)` pairs that one non-generic function sorts; the index makes the order stable,
//! and the items are then moved into place by following the permutation's cycles. Use it for small and medium slices
//! off the hot paths; the style cascade still sorts its rule matches directly.

/// Sort `items` by `key`, equal keys keeping their order. Slices longer than `u32::MAX` are left alone.
pub fn sort_by_u64_key<T>(items: &mut [T], key: impl Fn(&T) -> u64) {
    let n = items.len();
    if n < 2 || u32::try_from(n).is_err() {
        return;
    }
    let mut order: Vec<(u64, u32)> = Vec::with_capacity(n);
    for (i, t) in items.iter().enumerate() {
        order.push((key(t), u32::try_from(i).unwrap_or(u32::MAX)));
    }
    sort_pairs(&mut order);
    permute(items, &order);
}

/// The one copy of the sorting code. Indexes are unique, so an unstable sort gives the stable order.
#[inline(never)]
fn sort_pairs(pairs: &mut [(u64, u32)]) {
    pairs.sort_unstable();
}

/// Put `items[order[k].1]` at position `k`, cycle by cycle.
fn permute<T>(items: &mut [T], order: &[(u64, u32)]) {
    let mut done = vec![false; order.len()];
    for start in 0..order.len() {
        if done.get(start).copied().unwrap_or(true) {
            continue;
        }
        let mut cur = start;
        loop {
            if let Some(d) = done.get_mut(cur) {
                *d = true;
            }
            let src = order.get(cur).map_or(start, |p| p.1 as usize);
            if src == start || src >= items.len() {
                break;
            }
            items.swap(cur, src);
            cur = src;
        }
    }
}

/// A key that orders `f32`s like `f32::total_cmp`.
pub fn f32_key(x: f32) -> u64 {
    let b = x.to_bits();
    u64::from(if b >> 31 == 1 { !b } else { b | 0x8000_0000 })
}

/// A key that orders `f64`s like `f64::total_cmp`.
pub fn f64_key(x: f64) -> u64 {
    let b = x.to_bits();
    if b >> 63 == 1 { !b } else { b | (1 << 63) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorts_stably_like_sort_by_key() {
        let mut state = 12345u32;
        for n in [0usize, 1, 2, 3, 10, 200, 1000] {
            let items: Vec<(u32, usize)> = (0..n)
                .map(|i| {
                    state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                    ((state >> 24) % 7, i)
                })
                .collect();
            let mut mine = items.clone();
            let mut std_sorted = items;
            sort_by_u64_key(&mut mine, |t| u64::from(t.0));
            std_sorted.sort_by_key(|t| t.0);
            assert_eq!(mine, std_sorted, "n = {n}");
        }
    }

    #[test]
    fn float_keys_follow_total_cmp() {
        let xs = [f64::NEG_INFINITY, -2.5, -0.0, 0.0, 1e-300, 3.0, f64::INFINITY, f64::NAN];
        for a in xs {
            for b in xs {
                assert_eq!(f64_key(a).cmp(&f64_key(b)), a.total_cmp(&b), "{a} {b}");
                assert_eq!(f32_key(a as f32).cmp(&f32_key(b as f32)), (a as f32).total_cmp(&(b as f32)), "{a} {b}");
            }
        }
    }
}
