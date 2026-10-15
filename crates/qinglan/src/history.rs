//! Back / forward history of one window (L1 task 17). Pages are not kept, only their addresses and where the user had
//! scrolled to; the bytes come back from the loader's response cache (`net::cache`, 16 MB).

/// Entries kept; older ones fall off the front.
pub const MAX_ENTRIES: usize = 200;

#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    /// The address with its fragment, as the address bar shows it.
    pub url: String,
    /// Scroll position in CSS px when the user left the page.
    pub scroll: f32,
}

#[derive(Default)]
pub struct History {
    entries: Vec<Entry>,
    index: usize,
}

impl History {
    pub fn new() -> History {
        History::default()
    }

    pub fn current(&self) -> Option<&Entry> {
        self.entries.get(self.index)
    }

    pub fn can_back(&self) -> bool {
        self.index > 0
    }

    pub fn can_forward(&self) -> bool {
        self.index + 1 < self.entries.len()
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// A new page was opened: forget what could have been gone forward to and add it. Opening the address the
    /// window is already at adds nothing (it is a reload) and starts from the top.
    pub fn push(&mut self, url: String) {
        if let Some(cur) = self.entries.get_mut(self.index) {
            if cur.url == url {
                cur.scroll = 0.0;
                return;
            }
            self.entries.truncate(self.index + 1);
        }
        self.entries.push(Entry { url, scroll: 0.0 });
        if self.entries.len() > MAX_ENTRIES {
            self.entries.remove(0);
        }
        self.index = self.entries.len().saturating_sub(1);
    }

    /// The current entry's address changed (a redirect ended somewhere else).
    pub fn set_current_url(&mut self, url: String) {
        if let Some(cur) = self.entries.get_mut(self.index) {
            cur.url = url;
        }
    }

    /// Remember where the user is on the current page (called when leaving it).
    pub fn save_scroll(&mut self, scroll: f32) {
        if let Some(cur) = self.entries.get_mut(self.index) {
            cur.scroll = if scroll.is_finite() { scroll.max(0.0) } else { 0.0 };
        }
    }

    /// Go back one entry; returns it (with the scroll position to restore).
    pub fn back(&mut self) -> Option<Entry> {
        if !self.can_back() {
            return None;
        }
        self.index -= 1;
        self.current().cloned()
    }

    pub fn forward(&mut self) -> Option<Entry> {
        if !self.can_forward() {
            return None;
        }
        self.index += 1;
        self.current().cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(urls: &[&str]) -> History {
        let mut h = History::new();
        for u in urls {
            h.push((*u).to_string());
        }
        h
    }

    #[test]
    fn back_and_forward_walk_the_list() {
        let mut h = h(&["a", "b", "c"]);
        assert!(h.can_back() && !h.can_forward());
        assert_eq!(h.back().unwrap().url, "b");
        assert_eq!(h.back().unwrap().url, "a");
        assert!(h.back().is_none());
        assert!(!h.can_back() && h.can_forward());
        assert_eq!(h.forward().unwrap().url, "b");
        assert_eq!(h.forward().unwrap().url, "c");
        assert!(h.forward().is_none());
    }

    #[test]
    fn scroll_positions_come_back() {
        let mut h = h(&["a", "b"]);
        h.save_scroll(750.5); // on b
        h.back();
        h.save_scroll(120.0); // on a
        let b = h.forward().unwrap();
        assert_eq!((b.url.as_str(), b.scroll), ("b", 750.5));
        let a = h.back().unwrap();
        assert_eq!((a.url.as_str(), a.scroll), ("a", 120.0));
    }

    #[test]
    fn a_new_page_drops_the_forward_entries() {
        let mut h = h(&["a", "b", "c"]);
        h.back();
        h.back();
        h.push("d".into());
        assert_eq!(h.len(), 2);
        assert!(!h.can_forward());
        assert_eq!(h.back().unwrap().url, "a");
    }

    #[test]
    fn opening_the_same_address_is_a_reload() {
        let mut h = h(&["a", "b"]);
        h.save_scroll(300.0);
        h.push("b".into());
        assert_eq!(h.len(), 2);
        assert_eq!(h.current().unwrap().scroll, 0.0);
    }

    #[test]
    fn redirects_update_the_entry_and_junk_scrolls_are_clamped() {
        let mut h = h(&["a"]);
        h.set_current_url("a2".into());
        h.save_scroll(f32::NAN);
        assert_eq!(h.current().unwrap(), &Entry { url: "a2".into(), scroll: 0.0 });
        h.save_scroll(-5.0);
        assert_eq!(h.current().unwrap().scroll, 0.0);
    }

    #[test]
    fn the_list_is_capped() {
        let mut h = History::new();
        for i in 0..MAX_ENTRIES + 50 {
            h.push(format!("u{i}"));
        }
        assert_eq!(h.len(), MAX_ENTRIES);
        assert_eq!(h.current().unwrap().url, format!("u{}", MAX_ENTRIES + 49));
    }
}
