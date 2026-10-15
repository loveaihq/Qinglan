//! The loader: applies the policy, schedules requests (priority, 6 per host, 16 in all), runs them on
//! worker threads through a [`Transport`], keeps the back/forward cache and returns results as events.
//!
//! The owner (the window, or a command-line driver) calls [`Loader::submit`] and later [`Loader::poll`]
//! (or [`Loader::wait`] when blocking is fine). Workers never touch the loader: they send a message
//! through a channel and call the `wake` callback, which the window turns into a window message. So
//! the UI thread never waits for the network.

use std::collections::{HashMap, VecDeque};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::Arc;
use std::time::Duration;

use super::cache::ResponseCache;
use super::policy::{self, Plan, Refusal};
use super::{MAX_IN_FLIGHT, MAX_PER_HOST, MAX_QUEUED, NetError, Request, ResourceKind, Response, Transport, data_url, file};
use crate::url::Url;

/// A `data:` URL as a response. Runs on a worker thread: decoding megabytes of base64 must not stall
/// the caller.
fn decode_data(url: &Url, max_bytes: usize) -> Result<Response, NetError> {
    match data_url::decode(url, max_bytes) {
        Ok(d) => Ok(Response { url: url.clone(), status: 200, headers: vec![("Content-Type".to_string(), d.mime)], body: d.body }),
        Err(data_url::DataUrlError::TooLarge) => Err(NetError::TooLarge { limit: max_bytes }),
        Err(data_url::DataUrlError::Malformed) => Err(NetError::Protocol("malformed data: address".into())),
    }
}

/// Identifies one submitted request in the events that come back.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Ticket(pub u64);

/// What to load.
#[derive(Clone, Debug)]
pub struct LoadRequest {
    /// The address (a fragment is dropped before fetching).
    pub url: Url,
    pub kind: ResourceKind,
    /// The address of the page asking (for the `Referer`, mixed content and `file:` rules); `None` when
    /// the user typed the address.
    pub page: Option<Url>,
    /// Answer from the back/forward cache when it has the URL (for history navigation). A normal load
    /// and a reload pass `false`; every successful response is cached either way.
    pub use_cache: bool,
}

/// A finished request. HTTP error statuses are `Ok`: the body of a 404 page is shown.
#[derive(Debug)]
pub struct LoadEvent {
    pub ticket: Ticket,
    /// The address that was requested (the response has the final one after redirects).
    pub url: Url,
    pub result: Result<Arc<Response>, NetError>,
    pub from_cache: bool,
}

struct Queued {
    ticket: Ticket,
    url: Url,
    kind: ResourceKind,
    plan: Plan,
    headers: Vec<(String, String)>,
    host: Option<String>,
}

struct Done {
    ticket: Ticket,
    generation: u64,
    url: Url,
    host: Option<String>,
    result: Result<Response, NetError>,
}

pub struct Loader {
    transport: Arc<dyn Transport>,
    wake: Arc<dyn Fn() + Send + Sync>,
    tx: Sender<Done>,
    rx: Receiver<Done>,
    /// Waiting requests by priority (documents, style sheets, images).
    queues: [VecDeque<Queued>; 3],
    in_flight: usize,
    per_host: HashMap<String, usize>,
    /// Results that need no transfer (cache hits, `data:`), delivered by the next `poll`.
    ready: Vec<LoadEvent>,
    cache: ResponseCache,
    next_ticket: u64,
    /// Bumped by `cancel_all`: results of older requests are dropped.
    generation: u64,
    cancel: Arc<AtomicBool>,
}

impl Loader {
    /// `wake` is called from a worker thread whenever a result is waiting for `poll`. It must be cheap
    /// and must not block (post a window message).
    pub fn new(transport: Arc<dyn Transport>, wake: Arc<dyn Fn() + Send + Sync>) -> Loader {
        let (tx, rx) = channel();
        Loader {
            transport,
            wake,
            tx,
            rx,
            queues: [VecDeque::new(), VecDeque::new(), VecDeque::new()],
            in_flight: 0,
            per_host: HashMap::new(),
            ready: Vec::new(),
            cache: ResponseCache::new(),
            next_ticket: 1,
            generation: 0,
            cancel: Arc::new(AtomicBool::new(false)),
        }
    }

    /// A loader nobody has to wake (command-line tools that use [`Loader::wait`]).
    pub fn without_wake(transport: Arc<dyn Transport>) -> Loader {
        Loader::new(transport, Arc::new(|| {}))
    }

    pub fn cache(&self) -> &ResponseCache {
        &self.cache
    }

    pub fn cache_mut(&mut self) -> &mut ResponseCache {
        &mut self.cache
    }

    /// Requests waiting for a slot plus requests on the wire.
    pub fn pending(&self) -> usize {
        self.in_flight + self.queues.iter().map(VecDeque::len).sum::<usize>()
    }

    /// Requests on the wire now.
    pub fn in_flight(&self) -> usize {
        self.in_flight
    }

    /// Ask for a resource. `Err` means the policy refused it (nothing is fetched and no event follows);
    /// otherwise exactly one [`LoadEvent`] with the returned ticket comes out of `poll` / `wait`.
    pub fn submit(&mut self, req: LoadRequest) -> Result<Ticket, Refusal> {
        let LoadRequest { mut url, kind, page, use_cache } = req;
        url.fragment = None;
        let plan = policy::plan(&url, kind, page.as_ref())?;
        if self.pending() + self.ready.len() >= MAX_QUEUED {
            return Err(Refusal::QueueFull);
        }
        let ticket = Ticket(self.next_ticket);
        self.next_ticket += 1;
        if plan != Plan::Data
            && use_cache
            && let Some(hit) = self.cache.get(&url.serialize(true))
        {
            self.ready.push(LoadEvent { ticket, url, result: Ok(hit), from_cache: true });
            (self.wake)();
            return Ok(ticket);
        }
        let mut headers = vec![("Accept".to_string(), kind.accept().to_string())];
        if plan == Plan::Network
            && let Some(p) = &page
            && let Some(r) = policy::referer(p, &url)
        {
            headers.push(("Referer".to_string(), r));
        }
        let host = (plan == Plan::Network).then(|| policy::host_key(&url));
        if let Some(q) = self.queues.get_mut(kind.priority()) {
            q.push_back(Queued { ticket, url, kind, plan, headers, host });
        }
        self.pump();
        Ok(ticket)
    }

    /// Stop everything: waiting requests are dropped, running ones are told to give up, and the
    /// results of both are never delivered.
    pub fn cancel_all(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        self.cancel = Arc::new(AtomicBool::new(false));
        self.generation += 1;
        for q in &mut self.queues {
            q.clear();
        }
        self.ready.clear();
    }

    /// Collect finished requests without waiting.
    pub fn poll(&mut self) -> Vec<LoadEvent> {
        let mut out = std::mem::take(&mut self.ready);
        while let Ok(d) = self.rx.try_recv() {
            if let Some(e) = self.finish(d) {
                out.push(e);
            }
        }
        self.pump();
        out
    }

    /// Like `poll`, but waits up to `timeout` for the first result when there is none yet and requests
    /// are pending. May return nothing (timeout, or a result that was cancelled).
    pub fn wait(&mut self, timeout: Duration) -> Vec<LoadEvent> {
        let mut out = self.poll();
        if !out.is_empty() || self.in_flight == 0 {
            return out;
        }
        if let Ok(d) = self.rx.recv_timeout(timeout)
            && let Some(e) = self.finish(d)
        {
            out.push(e);
        }
        out.extend(self.poll());
        out
    }

    fn finish(&mut self, d: Done) -> Option<LoadEvent> {
        self.in_flight = self.in_flight.saturating_sub(1);
        if let Some(h) = &d.host
            && let Some(c) = self.per_host.get_mut(h)
        {
            *c = c.saturating_sub(1);
            if *c == 0 {
                self.per_host.remove(h);
            }
        }
        if d.generation != self.generation {
            return None;
        }
        let result = d.result.map(Arc::new);
        // `data:` answers are not kept: the key would be the whole (possibly megabytes long) URL.
        if let Ok(r) = &result
            && d.url.scheme != "data"
        {
            self.cache.insert(d.url.serialize(true), r);
        }
        Some(LoadEvent { ticket: d.ticket, url: d.url, result, from_cache: false })
    }

    /// Start waiting requests while there is room: highest priority first, skipping hosts that are full.
    fn pump(&mut self) {
        while self.in_flight < MAX_IN_FLIGHT {
            let Some((qi, pos)) = self.next_startable() else { break };
            let Some(q) = self.queues.get_mut(qi).and_then(|d| d.remove(pos)) else { break };
            self.start(q);
        }
    }

    fn next_startable(&self) -> Option<(usize, usize)> {
        for (qi, q) in self.queues.iter().enumerate() {
            for (pos, item) in q.iter().enumerate() {
                let open = match &item.host {
                    None => true,
                    Some(h) => self.per_host.get(h).copied().unwrap_or(0) < MAX_PER_HOST,
                };
                if open {
                    return Some((qi, pos));
                }
            }
        }
        None
    }

    fn start(&mut self, q: Queued) {
        self.in_flight += 1;
        if let Some(h) = &q.host {
            *self.per_host.entry(h.clone()).or_insert(0) += 1;
        }
        let generation = self.generation;
        let failed = Done { ticket: q.ticket, generation, url: q.url.clone(), host: q.host.clone(), result: Err(NetError::Other("cannot start a worker thread".into())) };
        // The thread only needs the closure's contents; the receiver is not shared.
        let job = {
            let tx = self.tx.clone();
            let wake = self.wake.clone();
            let transport = self.transport.clone();
            let cancel = self.cancel.clone();
            move || {
                let max_bytes = q.kind.max_bytes();
                let fetched = catch_unwind(AssertUnwindSafe(|| match q.plan {
                    Plan::File => file::read(&q.url, max_bytes),
                    Plan::Data => decode_data(&q.url, max_bytes),
                    _ => transport.fetch(&Request { url: q.url.clone(), kind: q.kind, headers: q.headers.clone(), max_bytes, cancel }),
                }))
                .unwrap_or_else(|_| Err(NetError::Other("internal error while fetching".into())));
                // A transport that forgot the cap is still held to it.
                let result = match fetched {
                    Ok(r) if r.body.len() > max_bytes => Err(NetError::TooLarge { limit: max_bytes }),
                    other => other,
                };
                let _ = tx.send(Done { ticket: q.ticket, generation, url: q.url, host: q.host, result });
                wake();
            }
        };
        // Network threads only copy bytes: a small stack is enough.
        if std::thread::Builder::new().name("qinglan-net".into()).stack_size(1 << 20).spawn(job).is_err() {
            let _ = self.tx.send(failed);
            (self.wake)();
        }
    }
}
