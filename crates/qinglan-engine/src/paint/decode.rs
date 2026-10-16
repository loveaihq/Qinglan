//! Image decoding on worker threads (step 2-0). Decoding a big image takes 100 ms and more; on the UI thread that is
//! a stall in the middle of scrolling. The window hands the work to this pool, goes on, and is woken (the same
//! wake-up as for the loader) when a bitmap is ready; [`super::images::ImageStore`] then takes it in.
//!
//! The queue lives on the UI thread (like the loader's): at most `max_running` jobs run, each on a thread of its own
//! that ends with its job (a decode is milliseconds to a quarter of a second, a thread is tens of microseconds), and a
//! job that has not started can be taken back without any locking. The only thing the threads share with the UI
//! thread is the box their results are put in.
//!
//! The pool knows nothing about pages: a job is an id, the address and size it is decoded for (the key of the
//! decoded-image cache) and the response that holds the compressed bytes. Whether a result is still wanted is the
//! store's call: it keeps the table of jobs it started, and a result that is not in it (the page was left, the image
//! went far away, the size changed) is dropped.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use crate::net::Response;
use crate::net::loader::spawn_named;
use crate::platform::image::{DecodedImage, ImageDecoder, ImageError};

/// Decodes that run at the same time, at most.
pub const DECODE_WORKERS: usize = 2;
/// Stack of a decode thread: WIC and the codecs behind it keep their buffers on the heap.
const WORKER_STACK: usize = 2 << 20;

/// The id of a job; never used twice by one pool.
pub type JobId = u64;
/// What a job decodes: (address, width, height), the key of the decoded-image cache.
pub type DecodeKey = (String, u32, u32);

struct Job {
    id: JobId,
    key: DecodeKey,
    resp: Arc<Response>,
}

/// A job that has run.
pub struct Finished {
    pub id: JobId,
    pub key: DecodeKey,
    pub result: Result<DecodedImage, ImageError>,
}

type Wake = Arc<dyn Fn() + Send + Sync>;
type Outbox = Arc<Mutex<Vec<Finished>>>;

/// Decodes images on threads of their own. Used from the UI thread only.
pub struct DecodePool {
    decoder: Arc<dyn ImageDecoder>,
    wake: Wake,
    max_running: usize,
    /// Jobs that have not started, oldest first.
    waiting: Vec<Job>,
    /// Threads that have not put their result in the box yet, as far as `poll` has seen.
    running: usize,
    outbox: Outbox,
    /// Results taken from the box but not handed out ([`DecodePool::wait`] gives one at a time).
    taken: Vec<Finished>,
    next_id: JobId,
}

impl DecodePool {
    /// `wake` is called from a decode thread whenever a result is waiting for [`DecodePool::poll`]; like the loader's it
    /// must be cheap and must not block.
    pub fn new(decoder: Arc<dyn ImageDecoder>, max_running: usize, wake: Wake) -> DecodePool {
        DecodePool { decoder, wake, max_running: max_running.max(1), waiting: Vec::new(), running: 0, outbox: Arc::new(Mutex::new(Vec::new())), taken: Vec::new(), next_id: 1 }
    }

    /// Queue a decode of `resp`'s bytes to `key`'s size. Jobs start in the order they were queued.
    pub fn submit(&mut self, key: DecodeKey, resp: Arc<Response>) -> JobId {
        let id = self.next_id;
        self.next_id += 1;
        self.waiting.push(Job { id, key, resp });
        self.start_waiting();
        id
    }

    fn start_waiting(&mut self) {
        while self.running < self.max_running && !self.waiting.is_empty() {
            let job = self.waiting.remove(0);
            let (decoder, wake, outbox) = (self.decoder.clone(), self.wake.clone(), self.outbox.clone());
            let (id, key) = (job.id, job.key.clone());
            let started = spawn_named("qinglan-decode", WORKER_STACK, Box::new(move || {
                // A decoder that panics on a bad file is a failed image, not a dead thread.
                let result = catch_unwind(AssertUnwindSafe(|| decoder.decode(&job.resp.body, job.key.1, job.key.2))).unwrap_or(Err(ImageError::Corrupt));
                outbox.lock().unwrap_or_else(PoisonError::into_inner).push(Finished { id: job.id, key: job.key, result });
                wake();
            }));
            if started {
                self.running += 1;
            } else {
                // No thread to run it on: it fails (the image is shown as broken) instead of waiting for ever.
                self.taken.push(Finished { id, key, result: Err(ImageError::Unsupported) });
                (self.wake)();
            }
        }
    }

    /// Take a job back if it has not started. False when it runs (or ran): its result still comes out of
    /// [`DecodePool::poll`].
    pub fn cancel(&mut self, id: JobId) -> bool {
        let before = self.waiting.len();
        self.waiting.retain(|j| j.id != id);
        self.waiting.len() != before
    }

    /// Take back every job that has not started (the page was left). Running jobs finish; their results are dropped by
    /// whoever asked for them.
    pub fn cancel_all(&mut self) {
        self.waiting.clear();
    }

    /// Jobs waiting for their turn.
    pub fn queued(&self) -> usize {
        self.waiting.len()
    }

    /// The results that are ready; a waiting job starts in the place of each.
    pub fn poll(&mut self) -> Vec<Finished> {
        let fresh = std::mem::take(&mut *self.outbox.lock().unwrap_or_else(PoisonError::into_inner));
        self.running = self.running.saturating_sub(fresh.len());
        self.start_waiting();
        let mut out = std::mem::take(&mut self.taken);
        out.extend(fresh);
        out
    }

    /// The next result, waiting for it at most `timeout` (tests and command-line tools; the window uses `poll`).
    pub fn wait(&mut self, timeout: Duration) -> Option<Finished> {
        let end = Instant::now() + timeout;
        loop {
            if self.taken.is_empty() {
                let fresh = self.poll();
                self.taken = fresh;
            }
            if !self.taken.is_empty() {
                return Some(self.taken.remove(0));
            }
            if Instant::now() >= end {
                return None;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    }
}
