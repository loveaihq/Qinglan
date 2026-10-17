//! `qinglan.exe --bench`: what is measured and how the report looks. The window code feeds a [`Bench`] while it
//! loads the page and scrolls it from the top to the bottom, then prints [`Report::format`].
//!
//! Clock rules (L1 task 17, "cold first screen"): the clock starts when the HTML arrives; the HTML is parsed only
//! after that (there is no streaming parser), so the parse is counted, and what is left out is the time the UI thread
//! spends waiting for the `<head>` style sheets to come in (the network's time). `first-screen-ms` is therefore the
//! sum of the time the UI thread was busy with the page from the arrival of the HTML to the moment the first screen's
//! pixels were blitted to the window (after the blit and `GdiFlush` returned). `first-screen-wall-ms` is the plain
//! wall-clock time over the same span, which also contains the wait for the sheets (a replay is fast, but each sheet is
//! still a thread and a file read), so it is an upper bound.

use std::time::{Duration, Instant};

use crate::args::BenchOptions;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Stage {
    /// Waiting for the page, the images and the layouts to settle.
    Loading,
    /// Scrolling frame by frame.
    Scrolling,
    Done,
}

pub struct Bench {
    pub opts: BenchOptions,
    pub stage: Stage,
    /// When the run began (the overall deadline counts from here).
    pub started: Instant,
    pub startup_ms: Option<f64>,
    /// When the HTML arrived.
    pub html_at: Option<Instant>,
    /// Time the UI thread was busy for the page, summed over the events before the current one.
    pub busy: Duration,
    /// Still adding up (until the full layout is done).
    pub counting: bool,
    pub first_screen: Option<Span>,
    pub full_layout: Option<Span>,
    /// Raster + blit time of every scroll step, ms.
    pub frames: Vec<f64>,
    /// The scroll position (CSS px) of each of those steps.
    pub frame_y: Vec<f32>,
    /// Of each step's time, the part spent in the GDI calls that put the pixels on the window (blit and flush).
    pub present_ms: Vec<f64>,
    /// Time spent decoding and dropping images between frames, ms (not part of the frame time).
    pub decode_ms: f64,
    pub next_y: f32,
    /// Layouts after the first whole-page one (images arriving, the window changing).
    pub relayouts: u32,
    /// Of those, the ones made while scrolling, and the longest one, ms.
    pub scroll_relayouts: u32,
    pub scroll_relayout_max_ms: f64,
    /// The longest time the UI thread spent in one event while scrolling (a step, a layout, an image arriving), ms: the
    /// longest time a scroll input could have waited.
    pub scroll_event_max_ms: f64,
    /// The same without the time in the GDI present calls: what this program's own code kept the UI thread busy.
    pub scroll_ours_max_ms: f64,
    /// Notches left in the current burst of the scroll.
    pub burst_left: u32,
    /// When the scroll reached the bottom and began to wait for the page to settle.
    pub bottom_at: Option<Instant>,
    pub error: Option<String>,
    /// Private bytes at some points of the run: (name, bytes).
    pub marks: Vec<(&'static str, u64)>,
    /// The `mem-*` lines of the memory breakdown (see `Browser::memory_breakdown`).
    pub breakdown: String,
    /// The most decoded-image bytes held at any frame of the scroll.
    pub peak_decoded: usize,
}

/// A phase measured two ways (see the module comment).
#[derive(Clone, Copy, Debug)]
pub struct Span {
    pub busy: Duration,
    pub wall: Duration,
}

impl Bench {
    pub fn new(opts: BenchOptions) -> Bench {
        Bench {
            opts,
            stage: Stage::Loading,
            started: Instant::now(),
            startup_ms: None,
            html_at: None,
            busy: Duration::ZERO,
            counting: false,
            first_screen: None,
            full_layout: None,
            frames: Vec::new(),
            frame_y: Vec::new(),
            present_ms: Vec::new(),
            decode_ms: 0.0,
            next_y: 0.0,
            relayouts: 0,
            scroll_relayouts: 0,
            scroll_relayout_max_ms: 0.0,
            scroll_event_max_ms: 0.0,
            scroll_ours_max_ms: 0.0,
            burst_left: 0,
            bottom_at: None,
            error: None,
            marks: Vec::new(),
            breakdown: String::new(),
            peak_decoded: 0,
        }
    }

    /// The HTML has arrived at `at` (the start of the event that carries it).
    pub fn html_arrived(&mut self, at: Instant) {
        if self.html_at.is_none() {
            self.html_at = Some(at);
            self.counting = true;
        }
    }

    /// A phase is done: `event_start` is the start of the event being handled, `now` the present.
    pub fn span(&self, event_start: Instant, now: Instant) -> Option<Span> {
        let html = self.html_at?;
        Some(Span { busy: self.busy + now.saturating_duration_since(event_start), wall: now.saturating_duration_since(html) })
    }

    /// A layout after the first whole-page one took `took_ms`.
    pub fn relayout_done(&mut self, took_ms: f64) {
        self.relayouts += 1;
        if self.stage == Stage::Scrolling {
            self.scroll_relayouts += 1;
            self.scroll_relayout_max_ms = self.scroll_relayout_max_ms.max(took_ms);
        }
    }

    /// An event that started at `start` ended at `end`.
    pub fn event_done(&mut self, start: Instant, end: Instant) {
        if self.counting {
            self.busy += end.saturating_duration_since(start);
        }
    }
}

/// The numbers of a finished (or failed) run.
pub struct Report<'a> {
    pub bench: &'a Bench,
    /// `(private bytes now, peak pagefile usage)`.
    pub memory: Option<(u64, u64)>,
}

pub fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

/// The value at quantile `q` (0..=1) of a sorted list (nearest rank).
pub fn percentile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let i = ((sorted.len() as f64 - 1.0) * q.clamp(0.0, 1.0)).round() as usize;
    sorted.get(i).copied().unwrap_or(0.0)
}

impl Report<'_> {
    /// `key: value` lines, one per number; a number that was not measured says `n/a`.
    pub fn format(&self) -> String {
        let b = self.bench;
        let num = |v: Option<f64>| v.map_or_else(|| "n/a".to_string(), |v| format!("{v:.1}"));
        let mut sorted = b.frames.clone();
        qinglan_engine::sorting::sort_by_u64_key(&mut sorted, |&x| qinglan_engine::sorting::f64_key(x));
        let frames = !sorted.is_empty();
        let o = &b.opts;
        let mut s = String::new();
        let mut line = |k: &str, v: String| {
            s.push_str(k);
            s.push_str(": ");
            s.push_str(&v);
            s.push('\n');
        };
        line("bench-address", o.address.clone());
        line("bench-viewport", format!("{}x{} css px, dpr {} ({}x{} device px)", o.width, o.height, o.dpr, (o.width * o.dpr).round(), (o.height * o.dpr).round()));
        line("startup-ms", num(b.startup_ms));
        line("first-screen-ms", num(b.first_screen.map(|s| ms(s.busy))));
        line("first-screen-wall-ms", num(b.first_screen.map(|s| ms(s.wall))));
        line("full-layout-ms", num(b.full_layout.map(|s| ms(s.busy))));
        line("full-layout-wall-ms", num(b.full_layout.map(|s| ms(s.wall))));
        line("relayouts", b.relayouts.to_string());
        line("scroll-frames", b.frames.len().to_string());
        line("scroll-median-ms", num(frames.then(|| percentile(&sorted, 0.5))));
        line("scroll-p95-ms", num(frames.then(|| percentile(&sorted, 0.95))));
        line("scroll-max-frame-ms", num(frames.then(|| sorted.last().copied().unwrap_or(0.0))));
        line("scroll-max-ms", num(frames.then_some(b.scroll_event_max_ms.max(sorted.last().copied().unwrap_or(0.0)))));
        // A step's own work is its time without the present; the present of a frame is the system's (the compositor).
        let max_present = b.present_ms.iter().copied().fold(0.0, f64::max);
        let stalls = b.present_ms.iter().filter(|p| **p > 50.0).count();
        line("scroll-max-ours-ms", num(frames.then_some(b.scroll_ours_max_ms)));
        line("scroll-max-present-ms", num(frames.then_some(max_present)));
        line("scroll-present-stalls", format!("{stalls} frames over 50 ms"));
        let mut order: Vec<usize> = (0..b.frames.len()).collect();
        qinglan_engine::sorting::sort_by_u64_key(&mut order, |&i| !qinglan_engine::sorting::f64_key(b.frames.get(i).copied().unwrap_or(0.0)));
        let slowest: Vec<String> = order.iter().take(3).map(|&i| format!("{:.1} ms at y={:.0}", b.frames[i], b.frame_y.get(i).copied().unwrap_or(0.0))).collect();
        line("scroll-slowest-frames", slowest.join(", "));
        line("scroll-relayouts", format!("{} (longest {:.1} ms)", b.scroll_relayouts, b.scroll_relayout_max_ms));
        line("scroll-decode-ms", num(frames.then_some(b.decode_ms)));
        line("peak-private-bytes", self.memory.map_or_else(|| "n/a".to_string(), |m| m.1.to_string()));
        line("private-bytes-after-scroll", self.memory.map_or_else(|| "n/a".to_string(), |m| m.0.to_string()));
        for (name, bytes) in &b.marks {
            line(&format!("private-{name}-bytes"), bytes.to_string());
        }
        if let Some(e) = &b.error {
            line("bench-error", e.clone());
        }
        s.push_str(&b.breakdown);
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts() -> BenchOptions {
        BenchOptions { address: "http://x/".into(), width: 1920.0, height: 1080.0, dpr: 1.5 }
    }

    #[test]
    fn percentiles_use_the_nearest_rank() {
        let v: Vec<f64> = (1..=100).map(f64::from).collect();
        assert_eq!(percentile(&v, 0.5), 51.0);
        assert_eq!(percentile(&v, 0.95), 95.0);
        assert_eq!(percentile(&v, 1.0), 100.0);
        assert_eq!(percentile(&v, 0.0), 1.0);
        assert_eq!(percentile(&[], 0.5), 0.0);
        assert_eq!(percentile(&[7.0], 0.95), 7.0);
    }

    #[test]
    fn busy_time_is_summed_only_while_counting() {
        let base = Instant::now();
        let ms = Duration::from_millis;
        let mut b = Bench::new(opts());
        b.event_done(base, base + ms(5));
        assert_eq!(b.busy, Duration::ZERO, "before the HTML arrived");
        b.html_arrived(base + ms(10));
        b.event_done(base + ms(10), base + ms(30));
        b.event_done(base + ms(40), base + ms(45));
        assert_eq!(b.busy, ms(25));
        // A phase that ends 7 ms into an event that started at 50 ms: 25 + 7 busy, 57 ms of wall time.
        let s = b.span(base + ms(50), base + ms(57)).unwrap();
        assert_eq!((s.busy, s.wall), (ms(32), ms(47)));
        // The first arrival counts.
        b.html_arrived(base + ms(99));
        assert_eq!(b.html_at, Some(base + ms(10)));
    }

    #[test]
    fn the_report_has_every_key() {
        let mut b = Bench::new(opts());
        let r = Report { bench: &b, memory: None }.format();
        for key in [
            "bench-address",
            "bench-viewport",
            "startup-ms",
            "first-screen-ms",
            "first-screen-wall-ms",
            "full-layout-ms",
            "full-layout-wall-ms",
            "scroll-frames",
            "scroll-median-ms",
            "scroll-p95-ms",
            "scroll-max-frame-ms",
            "scroll-max-ms",
            "scroll-max-ours-ms",
            "scroll-max-present-ms",
            "scroll-present-stalls",
            "scroll-relayouts",
            "scroll-decode-ms",
            "peak-private-bytes",
            "private-bytes-after-scroll",
        ] {
            assert!(r.lines().any(|l| l.starts_with(&format!("{key}: "))), "{key} missing in:\n{r}");
        }
        assert!(r.contains("2880x1620 device px"));
        assert!(!r.contains("bench-error"));
        b.frames = vec![5.0, 7.0, 6.0];
        b.error = Some("boom".into());
        let r = Report { bench: &b, memory: Some((1000, 2000)) }.format();
        assert!(r.contains("scroll-median-ms: 6.0\n") && r.contains("peak-private-bytes: 2000\n") && r.contains("private-bytes-after-scroll: 1000\n"));
        assert!(r.contains("bench-error: boom"));
    }
}
