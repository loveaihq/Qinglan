//! A counting wrapper around the process allocator ([`PoolAlloc`]), for `--bench` / `qinglan-cli bench`'s memory breakdown.
//!
//! Off by default: until [`count_heap`] is called each allocation costs one relaxed load and a branch. Once on, it
//! keeps the bytes requested and not yet freed (`live`), the highest value that reached (`peak`) and the number of
//! allocations. Requested sizes are counted, not the block sizes the Windows heap hands out, so the heap's own
//! overhead shows up as the gap to `PrivateUsage`.

use super::heap::PoolAlloc;
use std::alloc::{GlobalAlloc, Layout};
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU64, Ordering::Relaxed};

static ON: AtomicBool = AtomicBool::new(false);
// Signed: memory allocated before counting started and freed after it takes `LIVE` slightly below zero.
static LIVE: AtomicIsize = AtomicIsize::new(0);
static PEAK: AtomicIsize = AtomicIsize::new(0);
static COUNT: AtomicU64 = AtomicU64::new(0);
static BLOCKS: AtomicIsize = AtomicIsize::new(0);

pub struct CountingAlloc;

fn added(n: usize) {
    COUNT.fetch_add(1, Relaxed);
    BLOCKS.fetch_add(1, Relaxed);
    let live = LIVE.fetch_add(n as isize, Relaxed) + n as isize;
    PEAK.fetch_max(live, Relaxed);
}

// SAFETY: every method forwards to `PoolAlloc` with the arguments it was given, so `PoolAlloc`'s guarantees (and the
// caller's obligations under `GlobalAlloc`) carry over unchanged; the counters are plain atomics.
unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: same layout, same caller contract.
        let p = unsafe { PoolAlloc.alloc(layout) };
        if !p.is_null() && ON.load(Relaxed) {
            added(layout.size());
        }
        p
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: same layout, same caller contract.
        let p = unsafe { PoolAlloc.alloc_zeroed(layout) };
        if !p.is_null() && ON.load(Relaxed) {
            added(layout.size());
        }
        p
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if ON.load(Relaxed) {
            LIVE.fetch_sub(layout.size() as isize, Relaxed);
            BLOCKS.fetch_sub(1, Relaxed);
        }
        // SAFETY: `ptr` and `layout` are the pair the caller got from this allocator, which got them from `PoolAlloc`.
        unsafe { PoolAlloc.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: `ptr` and `layout` come from this allocator (so from `PoolAlloc`); `new_size` is the caller's.
        let p = unsafe { PoolAlloc.realloc(ptr, layout, new_size) };
        if !p.is_null() && ON.load(Relaxed) {
            COUNT.fetch_add(1, Relaxed);
            let live = LIVE.fetch_add(new_size as isize - layout.size() as isize, Relaxed) + new_size as isize - layout.size() as isize;
            PEAK.fetch_max(live, Relaxed);
        }
        p
    }
}

/// Is the counting on?
pub fn heap_counting() -> bool {
    ON.load(Relaxed)
}

/// Start counting (call it first thing in `main`).
pub fn count_heap() {
    ON.store(true, Relaxed);
}

/// Heap bytes requested and not yet freed since [`count_heap`] (0 when counting is off).
pub fn heap_live() -> usize {
    LIVE.load(Relaxed).max(0) as usize
}

/// The highest `heap_live` seen.
pub fn heap_peak() -> usize {
    PEAK.load(Relaxed).max(0) as usize
}

/// Allocations (and reallocations) made since [`count_heap`].
pub fn heap_allocs() -> u64 {
    COUNT.load(Relaxed)
}

/// Blocks allocated and not yet freed since [`count_heap`]; the Windows heap adds about 16 bytes to each.
pub fn heap_blocks() -> usize {
    BLOCKS.load(Relaxed).max(0) as usize
}

#[repr(C)]
struct HeapSummaryRaw {
    cb: u32,
    allocated: usize,
    committed: usize,
    reserved: usize,
    max_reserve: usize,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetProcessHeap() -> isize;
    fn HeapSummary(heap: isize, flags: u32, summary: *mut HeapSummaryRaw) -> i32;
}

/// `(bytes the Windows process heap has handed out, bytes it has committed)`: the second minus the first is the
/// heap's own overhead plus the free memory it keeps. `None` when the call fails.
pub fn heap_summary() -> Option<(usize, usize)> {
    let mut s = HeapSummaryRaw { cb: core::mem::size_of::<HeapSummaryRaw>() as u32, allocated: 0, committed: 0, reserved: 0, max_reserve: 0 };
    // SAFETY: `GetProcessHeap` returns the handle of the process's default heap, valid for the life of the process
    // (the heap Rust's `System` allocator uses); `s` is a valid `HEAP_SUMMARY` with `cb` set, which `HeapSummary` fills.
    let ok = unsafe { HeapSummary(GetProcessHeap(), 0, &mut s) };
    (ok != 0).then_some((s.allocated, s.committed))
}
