//! A byte-counted LRU map: every entry carries a cost, and the least recently used entries are
//! dropped when the total goes over the budget. Safe slab-and-links implementation, O(1) per call.

use std::hash::Hash;

use crate::fx::FxMap;

const NIL: usize = usize::MAX;

struct Node<K, V> {
    key: K,
    val: Option<V>,
    cost: usize,
    prev: usize,
    next: usize,
}

pub struct Lru<K, V> {
    map: FxMap<K, usize>,
    nodes: Vec<Node<K, V>>,
    free: Vec<usize>,
    /// Most recently used.
    head: usize,
    /// Least recently used.
    tail: usize,
    bytes: usize,
    cap: usize,
}

impl<K: Eq + Hash + Copy, V> Lru<K, V> {
    pub fn new(cap_bytes: usize) -> Self {
        Lru { map: FxMap::default(), nodes: Vec::new(), free: Vec::new(), head: NIL, tail: NIL, bytes: 0, cap: cap_bytes }
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Total cost of the entries held.
    pub fn bytes(&self) -> usize {
        self.bytes
    }

    fn unlink(&mut self, i: usize) {
        let (p, n) = match self.nodes.get(i) {
            Some(node) => (node.prev, node.next),
            None => return,
        };
        if let Some(pn) = self.nodes.get_mut(p) {
            pn.next = n;
        } else {
            self.head = n;
        }
        if let Some(nn) = self.nodes.get_mut(n) {
            nn.prev = p;
        } else {
            self.tail = p;
        }
        if let Some(node) = self.nodes.get_mut(i) {
            node.prev = NIL;
            node.next = NIL;
        }
    }

    fn push_front(&mut self, i: usize) {
        let old = self.head;
        if let Some(node) = self.nodes.get_mut(i) {
            node.prev = NIL;
            node.next = old;
        }
        if let Some(h) = self.nodes.get_mut(old) {
            h.prev = i;
        } else {
            self.tail = i;
        }
        self.head = i;
    }

    /// Look `key` up and mark it most recently used.
    pub fn get(&mut self, key: &K) -> Option<&V> {
        let i = *self.map.get(key)?;
        if self.head != i {
            self.unlink(i);
            self.push_front(i);
        }
        self.nodes.get(i)?.val.as_ref()
    }

    pub fn remove(&mut self, key: &K) {
        let Some(i) = self.map.remove(key) else { return };
        self.unlink(i);
        if let Some(node) = self.nodes.get_mut(i) {
            node.val = None;
            self.bytes = self.bytes.saturating_sub(node.cost);
            node.cost = 0;
        }
        self.free.push(i);
    }

    /// Insert (replacing an older entry with the same key) and drop old entries until the total
    /// fits the budget. An entry that alone is over the budget is not stored.
    pub fn insert(&mut self, key: K, val: V, cost: usize) {
        self.remove(&key);
        if cost > self.cap {
            return;
        }
        let node = Node { key, val: Some(val), cost, prev: NIL, next: NIL };
        let i = match self.free.pop() {
            Some(i) => {
                if let Some(slot) = self.nodes.get_mut(i) {
                    *slot = node;
                }
                i
            }
            None => {
                self.nodes.push(node);
                self.nodes.len() - 1
            }
        };
        self.map.insert(key, i);
        self.push_front(i);
        self.bytes += cost;
        while self.bytes > self.cap && self.tail != NIL && self.tail != i {
            let t = self.tail;
            let Some(k) = self.nodes.get(t).map(|n| n.key) else { break };
            self.remove(&k);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evicts_least_recently_used_and_counts_bytes() {
        let mut c: Lru<u32, &str> = Lru::new(100);
        c.insert(1, "a", 40);
        c.insert(2, "b", 40);
        assert_eq!(c.bytes(), 80);
        assert_eq!(c.get(&1), Some(&"a")); // 1 is now the most recent, 2 the oldest
        c.insert(3, "c", 40);
        assert_eq!(c.get(&2), None);
        assert_eq!(c.get(&1), Some(&"a"));
        assert_eq!(c.get(&3), Some(&"c"));
        assert_eq!(c.bytes(), 80);
        assert_eq!(c.len(), 2);
    }

    #[test]
    fn replace_and_oversize() {
        let mut c: Lru<u32, u32> = Lru::new(50);
        c.insert(1, 10, 30);
        c.insert(1, 11, 20);
        assert_eq!(c.bytes(), 20);
        assert_eq!(c.get(&1), Some(&11));
        c.insert(2, 5, 51); // over the whole budget: not stored
        assert_eq!(c.get(&2), None);
        assert_eq!(c.len(), 1);
    }

    #[test]
    fn many_inserts_stay_within_budget() {
        let mut c: Lru<u32, u32> = Lru::new(1000);
        for i in 0..10_000u32 {
            c.insert(i, i, 7 + (i as usize % 13));
            assert!(c.bytes() <= 1000);
        }
        assert!(c.len() > 50);
        assert_eq!(c.get(&9_999), Some(&9_999));
        assert_eq!(c.get(&0), None);
    }
}
