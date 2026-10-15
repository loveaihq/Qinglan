//! The in-memory response cache for back/forward (L1 task 13): LRU, at most 16 MB of response bytes.

use std::sync::Arc;

use super::{CACHE_BUDGET, Response};
use crate::lru::ByteLru;

pub struct ResponseCache {
    lru: ByteLru<String, Arc<Response>>,
}

impl ResponseCache {
    pub fn new() -> ResponseCache {
        ResponseCache::with_budget(CACHE_BUDGET)
    }

    pub fn with_budget(bytes: usize) -> ResponseCache {
        ResponseCache { lru: ByteLru::new(bytes) }
    }

    pub fn get(&mut self, key: &str) -> Option<Arc<Response>> {
        self.lru.get(&key.to_string()).cloned()
    }

    /// Keep `resp` under `key` if it is worth keeping: a success (2xx) that does not say `no-store`.
    pub fn insert(&mut self, key: String, resp: &Arc<Response>) {
        if !(200..300).contains(&resp.status) {
            return;
        }
        let no_store = resp.header("cache-control").is_some_and(|v| v.to_ascii_lowercase().contains("no-store"));
        if no_store {
            return;
        }
        self.lru.insert(key, resp.clone(), resp.cost());
    }

    /// Bytes counted against the budget.
    pub fn bytes(&self) -> usize {
        self.lru.bytes()
    }

    pub fn len(&self) -> usize {
        self.lru.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lru.is_empty()
    }

    pub fn clear(&mut self) {
        self.lru.clear();
    }
}

impl Default for ResponseCache {
    fn default() -> Self {
        ResponseCache::new()
    }
}
