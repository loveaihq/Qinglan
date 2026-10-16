//! A byte-counted LRU map for the engine's caches (responses, decoded images): every entry carries a
//! cost, and the least recently used entries are dropped when the total goes over the budget.
//!
//! Eviction scans for the oldest entry, so it is O(n) per evicted entry. The caches hold at most a few
//! thousand entries (16 MB / 24 MB budgets), and eviction only happens when an insert overflows.

use std::hash::Hash;

use crate::html::atoms::FxHashMap;

struct Entry<V> {
    val: V,
    cost: usize,
    used: u64,
}

pub struct ByteLru<K, V> {
    map: FxHashMap<K, Entry<V>>,
    bytes: usize,
    budget: usize,
    tick: u64,
}

impl<K: Eq + Hash + Clone, V> ByteLru<K, V> {
    pub fn new(budget: usize) -> Self {
        ByteLru { map: FxHashMap::default(), bytes: 0, budget, tick: 0 }
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Total cost of the entries held.
    pub fn bytes(&self) -> usize {
        self.bytes
    }

    pub fn budget(&self) -> usize {
        self.budget
    }

    /// The value for `key`, marking it most recently used.
    pub fn get(&mut self, key: &K) -> Option<&V> {
        self.tick += 1;
        let tick = self.tick;
        let e = self.map.get_mut(key)?;
        e.used = tick;
        Some(&e.val)
    }

    pub fn remove(&mut self, key: &K) {
        if let Some(e) = self.map.remove(key) {
            self.bytes = self.bytes.saturating_sub(e.cost);
        }
    }

    pub fn clear(&mut self) {
        self.map.clear();
        self.bytes = 0;
    }

    /// Drop the least recently used entry that `evictable` allows. False when there is none.
    pub fn evict_oldest_where(&mut self, evictable: &dyn Fn(&K) -> bool) -> bool {
        let oldest = self.map.iter().filter(|(k, _)| evictable(k)).min_by_key(|(_, e)| e.used).map(|(k, _)| k.clone());
        match oldest {
            Some(k) => {
                self.remove(&k);
                true
            }
            None => false,
        }
    }

    /// Store `val` with `cost`, evicting the least recently used entries to make room. An entry that
    /// alone is bigger than the budget is not stored (returns false).
    pub fn insert(&mut self, key: K, val: V, cost: usize) -> bool {
        self.remove(&key);
        if cost > self.budget {
            return false;
        }
        while self.bytes + cost > self.budget {
            if !self.evict_oldest_where(&|_| true) {
                break;
            }
        }
        self.tick += 1;
        self.bytes += cost;
        self.map.insert(key, Entry { val, cost, used: self.tick });
        true
    }
}

#[cfg(test)]
mod tests {
    use super::ByteLru;

    #[test]
    fn evicts_least_recently_used_and_respects_budget() {
        let mut c: ByteLru<u32, u32> = ByteLru::new(100);
        assert!(c.insert(1, 10, 40));
        assert!(c.insert(2, 20, 40));
        assert_eq!(c.get(&1), Some(&10)); // 1 is now newer than 2
        assert!(c.insert(3, 30, 40)); // evicts 2
        assert_eq!(c.get(&2), None);
        assert_eq!(c.get(&1), Some(&10));
        assert_eq!(c.get(&3), Some(&30));
        assert!(c.bytes() <= 100);
        assert!(!c.insert(4, 40, 101), "bigger than the whole budget");
        assert_eq!(c.len(), 2);
    }
}
