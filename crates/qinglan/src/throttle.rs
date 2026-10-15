//! The relayout throttle (L1 task 17): images that arrive change the layout, but the page is laid out again at most
//! once every 100 ms. The gap is counted from the end of the previous relayout, so a slow layout does not starve the
//! message loop.

use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Plan {
    /// Nothing is waiting.
    Nothing,
    RunNow,
    /// Something is waiting; ask again after this many milliseconds (a timer).
    WaitMs(u32),
}

pub struct Throttle {
    gap: Duration,
    last_done: Option<Instant>,
    pending: bool,
}

impl Throttle {
    pub fn new(gap: Duration) -> Throttle {
        Throttle { gap, last_done: None, pending: false }
    }

    /// A relayout is wanted.
    pub fn request(&mut self) {
        self.pending = true;
    }

    pub fn pending(&self) -> bool {
        self.pending
    }

    pub fn plan(&self, now: Instant) -> Plan {
        if !self.pending {
            return Plan::Nothing;
        }
        match self.last_done {
            Some(done) => {
                let since = now.saturating_duration_since(done);
                if since >= self.gap { Plan::RunNow } else { Plan::WaitMs((self.gap - since).as_millis().max(1) as u32) }
            }
            None => Plan::RunNow,
        }
    }

    /// The relayout was done at `now`.
    pub fn done(&mut self, now: Instant) {
        self.pending = false;
        self.last_done = Some(now);
    }

    /// Forget everything (a new page).
    pub fn reset(&mut self) {
        self.pending = false;
        self.last_done = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GAP: Duration = Duration::from_millis(100);

    #[test]
    fn nothing_to_do_until_asked() {
        let t = Throttle::new(GAP);
        assert_eq!(t.plan(Instant::now()), Plan::Nothing);
    }

    #[test]
    fn the_first_request_runs_at_once() {
        let mut t = Throttle::new(GAP);
        t.request();
        assert_eq!(t.plan(Instant::now()), Plan::RunNow);
    }

    #[test]
    fn requests_inside_the_gap_wait_for_the_rest_of_it() {
        let base = Instant::now();
        let mut t = Throttle::new(GAP);
        t.request();
        t.done(base);
        assert_eq!(t.plan(base + Duration::from_millis(10)), Plan::Nothing);
        t.request();
        assert_eq!(t.plan(base + Duration::from_millis(10)), Plan::WaitMs(90));
        assert_eq!(t.plan(base + Duration::from_millis(99)), Plan::WaitMs(1));
        assert_eq!(t.plan(base + Duration::from_millis(100)), Plan::RunNow);
        assert_eq!(t.plan(base + Duration::from_secs(5)), Plan::RunNow);
    }

    #[test]
    fn many_requests_make_one_relayout() {
        let base = Instant::now();
        let mut t = Throttle::new(GAP);
        t.request();
        t.done(base);
        for _ in 0..50 {
            t.request();
        }
        assert!(t.pending());
        t.done(base + Duration::from_millis(100));
        assert_eq!(t.plan(base + Duration::from_millis(101)), Plan::Nothing);
    }

    #[test]
    fn a_new_page_starts_fresh() {
        let base = Instant::now();
        let mut t = Throttle::new(GAP);
        t.request();
        t.done(base);
        t.request();
        t.reset();
        assert_eq!(t.plan(base), Plan::Nothing);
        t.request();
        assert_eq!(t.plan(base), Plan::RunNow);
    }
}
