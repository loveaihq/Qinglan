//! The relayout throttle (L1 task 17): images that arrive change the layout, but the page is laid out again at most
//! once every 100 ms. The gap is counted from the end of the previous relayout, so a slow layout does not starve the
//! message loop.
//!
//! Step 2-0 adds the scroll rule: while the user is scrolling the relayout waits until `quiet` after the last scroll
//! input (a layout takes ~200 ms on a long page and would stop the scrolling), and every request made meanwhile is
//! served by the one layout that follows.

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
    quiet: Duration,
    last_done: Option<Instant>,
    last_input: Option<Instant>,
    pending: bool,
}

impl Throttle {
    /// `gap`: least time from the end of one relayout to the start of the next. `quiet`: time without scroll input
    /// before a relayout may start.
    pub fn new(gap: Duration, quiet: Duration) -> Throttle {
        Throttle { gap, quiet, last_done: None, last_input: None, pending: false }
    }

    /// The user scrolled at `now`.
    pub fn scrolled(&mut self, now: Instant) {
        self.last_input = Some(now);
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
        let left = |since: Option<Instant>, span: Duration| since.map_or(Duration::ZERO, |t| span.saturating_sub(now.saturating_duration_since(t)));
        let wait = left(self.last_done, self.gap).max(left(self.last_input, self.quiet));
        if wait.is_zero() {
            Plan::RunNow
        } else {
            // Round up: a timer that fires a hair early only asks again.
            let ms = u32::try_from(wait.as_millis()).unwrap_or(u32::MAX).saturating_add(u32::from(wait.subsec_nanos() % 1_000_000 != 0));
            Plan::WaitMs(ms.max(1))
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
        self.last_input = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GAP: Duration = Duration::from_millis(100);
    const QUIET: Duration = Duration::from_millis(150);

    #[test]
    fn nothing_to_do_until_asked() {
        let t = Throttle::new(GAP, QUIET);
        assert_eq!(t.plan(Instant::now()), Plan::Nothing);
    }

    #[test]
    fn the_first_request_runs_at_once() {
        let mut t = Throttle::new(GAP, QUIET);
        t.request();
        assert_eq!(t.plan(Instant::now()), Plan::RunNow);
    }

    #[test]
    fn requests_inside_the_gap_wait_for_the_rest_of_it() {
        let base = Instant::now();
        let mut t = Throttle::new(GAP, QUIET);
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
        let mut t = Throttle::new(GAP, QUIET);
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
        let mut t = Throttle::new(GAP, QUIET);
        t.request();
        t.done(base);
        t.request();
        t.reset();
        assert_eq!(t.plan(base), Plan::Nothing);
        t.request();
        assert_eq!(t.plan(base), Plan::RunNow);
    }

    #[test]
    fn a_relayout_waits_while_the_user_scrolls() {
        let base = Instant::now();
        let ms = Duration::from_millis;
        let mut t = Throttle::new(GAP, QUIET);
        t.scrolled(base);
        t.request();
        assert_eq!(t.plan(base + ms(10)), Plan::WaitMs(140));
        // Scrolling goes on: the wait starts again from the last input.
        t.scrolled(base + ms(100));
        assert_eq!(t.plan(base + ms(120)), Plan::WaitMs(130));
        assert_eq!(t.plan(base + ms(249)), Plan::WaitMs(1));
        // Requests made meanwhile are one relayout.
        for _ in 0..20 {
            t.request();
        }
        assert_eq!(t.plan(base + ms(250)), Plan::RunNow);
        t.done(base + ms(450));
        assert_eq!(t.plan(base + ms(460)), Plan::Nothing);
    }

    #[test]
    fn the_gap_and_the_quiet_time_both_count() {
        let base = Instant::now();
        let ms = Duration::from_millis;
        let mut t = Throttle::new(GAP, QUIET);
        t.request();
        t.done(base);
        t.scrolled(base + ms(60));
        t.request();
        // 100 ms gap would end at 100, the quiet time at 210: the longer wait wins.
        assert_eq!(t.plan(base + ms(70)), Plan::WaitMs(140));
        t.reset();
        t.request();
        assert_eq!(t.plan(base + ms(70)), Plan::RunNow, "a new page forgets the scrolling of the old one");
    }
}
