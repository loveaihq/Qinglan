//! The process's allocator: slab pools for small blocks, whole pages for big ones, the Windows heap in between.
//!
//! Why: the Windows heap puts a 16-byte header in front of every block and keeps freed memory as committed free space;
//! a page like the Chinese Wikipedia article has 160 000 live blocks of about 200 bytes, and the heap held about a
//! quarter of the process's private bytes as headers and free space (docs/decisions.md, 2a-3). Rust tells `dealloc` the
//! size of the block, so the pools need no header per block.
//!
//! * **Small** (1..=1024 bytes, alignment up to 8): 40 size classes (8-byte steps up to 128, then 16, 32 and 64). Each
//!   class takes 16 KiB slabs from one reserved address range; a slab has a 64-byte header, a free list of its freed
//!   blocks and a bump pointer. Which slab a block belongs to is `address & !(16 KiB - 1)`. A slab that is full leaves
//!   the class's list of slabs with room, one with a free block comes back, and one with no live block left is
//!   decommitted (the class keeps its last slab), so the memory a phase of the page needed (parsing, styling) does not
//!   stay committed after it.
//! * **Big** (128 KiB and up, alignment up to a page): its own `VirtualAlloc` pages, given back when freed. The Windows
//!   heap keeps such blocks as free space inside its segments (a doubling `Vec` of 2 MB leaves 1 + 0.5 + ... MB holes).
//! * Everything else (and small blocks when the address range or `VirtualAlloc` is exhausted): `System`.
//!
//! Each class has its own spin lock and the range has one (taken while a class lock is held, never the other way round);
//! an uncontended lock is one atomic operation. Nothing in here allocates or panics.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};
use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::{Acquire, Relaxed, Release}};

/// A lock for sections of a few dozen instructions. A `Mutex` that is contended puts the thread to sleep and wakes it
/// again, which costs far more than the section itself (the parallel style-sheet parse allocates from several threads
/// at once); this one spins briefly, then yields the time slice.
struct SpinLock<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
}

// SAFETY: the data is only reachable through a `SpinGuard`, and there is at most one guard at a time.
unsafe impl<T: Send> Sync for SpinLock<T> {}

struct SpinGuard<'a, T> {
    lock: &'a SpinLock<T>,
}

impl<T> SpinLock<T> {
    const fn new(data: T) -> Self {
        SpinLock { locked: AtomicBool::new(false), data: UnsafeCell::new(data) }
    }

    #[inline]
    fn lock(&self) -> SpinGuard<'_, T> {
        let mut spins = 0u32;
        while self.locked.compare_exchange_weak(false, true, Acquire, Relaxed).is_err() {
            while self.locked.load(Relaxed) {
                spins += 1;
                if spins < 200 {
                    std::hint::spin_loop();
                } else {
                    spins = 0;
                    std::thread::yield_now();
                }
            }
        }
        SpinGuard { lock: self }
    }
}

impl<T> Deref for SpinGuard<'_, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &T {
        // SAFETY: this guard holds the lock, so nobody else has a reference to the data.
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> DerefMut for SpinGuard<'_, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: this guard holds the lock, so nobody else has a reference to the data.
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T> Drop for SpinGuard<'_, T> {
    #[inline]
    fn drop(&mut self) {
        self.lock.locked.store(false, Release);
    }
}

/// The biggest block a slab serves.
const SMALL_MAX: usize = 1024;
/// The strictest alignment a slab block has (every size is a multiple of 8 and the blocks start 64 bytes into a slab).
const SMALL_ALIGN: usize = 8;
const CLASSES: usize = 40;
const SLAB: usize = 16 * 1024;
const HEADER: usize = 64;
/// Address space reserved for slabs (nothing is committed until a slab is taken): 512 MiB.
const ARENA: usize = 512 * 1024 * 1024;
const SLABS: usize = ARENA / SLAB;
const WORDS: usize = SLABS / 64;
/// Slabs are committed four at a time (one system call instead of four); the extra ones wait in `Arena::spare`.
const GROUP: usize = 4;
/// Empty committed slabs kept for the next class that needs one, so a phase that frees and allocates the same amount
/// again (the style pass) does not commit and decommit pages over and over. More than this are decommitted.
const SPARE_MAX: usize = 32;
/// Blocks of this size and up get their own pages from `VirtualAlloc` and give them back when freed.
const BIG_MIN: usize = 128 * 1024;
const PAGE: usize = 4096;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn VirtualAlloc(address: *mut u8, size: usize, allocation_type: u32, protect: u32) -> *mut u8;
    fn VirtualFree(address: *mut u8, size: usize, free_type: u32) -> i32;
}
const MEM_COMMIT: u32 = 0x1000;
const MEM_RESERVE: u32 = 0x2000;
const MEM_DECOMMIT: u32 = 0x4000;
const MEM_RELEASE: u32 = 0x8000;
const PAGE_READWRITE: u32 = 0x04;

/// The class (0..40) of a block of `size` bytes, 1 <= `size` <= 1024.
#[inline]
fn class_of(size: usize) -> usize {
    if size <= 128 {
        size.div_ceil(8) - 1
    } else if size <= 256 {
        15 + (size - 128).div_ceil(16)
    } else if size <= 512 {
        23 + (size - 256).div_ceil(32)
    } else {
        31 + (size - 512).div_ceil(64)
    }
}

/// The block size of class `c`.
#[inline]
fn class_size(c: usize) -> usize {
    if c < 16 {
        (c + 1) * 8
    } else if c < 24 {
        128 + (c - 15) * 16
    } else if c < 32 {
        256 + (c - 23) * 32
    } else {
        512 + (c - 31) * 64
    }
}

/// The header at the start of a slab (in the slab's own memory).
#[repr(C)]
struct Slab {
    /// The class's list of slabs that have room (a freed block or untouched space).
    next: *mut Slab,
    prev: *mut Slab,
    /// Freed blocks of this slab (each one's first 8 bytes point to the next).
    free: *mut u8,
    /// The untouched rest of the slab: `bump..end`.
    bump: *mut u8,
    end: *mut u8,
    /// Blocks handed out and not yet freed.
    live: u32,
    /// Is the slab in its class's list?
    listed: bool,
}

struct Class {
    head: *mut Slab,
}

// SAFETY: the pointers are only used while the class's mutex is held, and the slabs they point to stay mapped until
// the slab has no live block (which is checked under the same mutex).
unsafe impl Send for Class {}

static CLASS: [SpinLock<Class>; CLASSES] = [const { SpinLock::new(Class { head: null_mut() }) }; CLASSES];

struct Arena {
    /// Bit `i`: slab `i` is committed (in use by a class, or waiting in `spare`).
    used: [u64; WORDS],
    /// Committed slabs that no class uses.
    spare: [u32; SPARE_MAX],
    spare_n: usize,
    /// No word of `used` below this index is full.
    hint: usize,
    /// The reservation failed once; do not try again.
    failed: bool,
}

static ARENA_STATE: SpinLock<Arena> = SpinLock::new(Arena { used: [0; WORDS], spare: [0; SPARE_MAX], spare_n: 0, hint: 0, failed: false });
/// Start of the reserved range (0 until the first slab is taken).
static BASE: AtomicUsize = AtomicUsize::new(0);

fn lock_class(c: usize) -> SpinGuard<'static, Class> {
    // `c` < CLASSES by construction (`class_of`).
    CLASS.get(c).unwrap_or(&CLASS[0]).lock()
}

fn lock_arena() -> SpinGuard<'static, Arena> {
    ARENA_STATE.lock()
}

#[inline]
fn in_arena(p: *mut u8) -> bool {
    let base = BASE.load(Relaxed);
    base != 0 && (p as usize).wrapping_sub(base) < ARENA
}

/// Commit a fresh slab, or null when the range or the system is out of memory.
fn new_slab() -> *mut Slab {
    let mut a = lock_arena();
    let mut base = BASE.load(Relaxed);
    if base == 0 {
        if a.failed {
            return null_mut();
        }
        // SAFETY: a null address asks for a new region of address space; nothing is committed.
        let p = unsafe { VirtualAlloc(null_mut(), ARENA, MEM_RESERVE, PAGE_READWRITE) };
        if p.is_null() {
            a.failed = true;
            return null_mut();
        }
        base = p as usize;
        BASE.store(base, Relaxed);
    }
    if a.spare_n > 0 {
        a.spare_n -= 1;
        let index = a.spare.get(a.spare_n).copied().unwrap_or(0) as usize;
        return (base + index * SLAB) as *mut Slab;
    }
    // Four slabs that are all free and start on a multiple of four, else any free slab.
    let (mut group, mut single) = (None, None);
    let mut w = a.hint;
    while w < WORDS && group.is_none() {
        let word = a.used.get(w).copied().unwrap_or(u64::MAX);
        if word != u64::MAX {
            single = single.or(Some(w * 64 + (!word).trailing_zeros() as usize));
            group = (0..64 / GROUP).find(|j| (word >> (j * GROUP)) & 0xF == 0).map(|j| w * 64 + j * GROUP);
        }
        w += 1;
    }
    let (first, count) = match (group, single) {
        (Some(g), _) => (g, GROUP),
        (None, Some(i)) => (i, 1),
        _ => return null_mut(),
    };
    if first >= SLABS {
        return null_mut();
    }
    // SAFETY: `first..first + count` slabs lie inside the reserved range and are not committed (their bits are clear).
    let p = unsafe { VirtualAlloc((base + first * SLAB) as *mut u8, count * SLAB, MEM_COMMIT, PAGE_READWRITE) };
    if p.is_null() {
        return null_mut();
    }
    for i in first..first + count {
        if let Some(word) = a.used.get_mut(i / 64) {
            *word |= 1 << (i % 64);
        }
    }
    a.hint = first / 64;
    for i in first + 1..first + count {
        let n = a.spare_n;
        if let Some(slot) = a.spare.get_mut(n) {
            *slot = i as u32;
            a.spare_n = n + 1;
        }
    }
    p.cast()
}

/// Keep a slab with no live block for the next class that needs one, or decommit it when enough are kept.
fn release_slab(s: *mut Slab) {
    let base = BASE.load(Relaxed);
    let index = ((s as usize) - base) / SLAB;
    let mut a = lock_arena();
    let n = a.spare_n;
    if let Some(slot) = a.spare.get_mut(n) {
        *slot = index as u32;
        a.spare_n = n + 1;
        return;
    }
    // SAFETY: `s` is a committed slab of the range and nothing points into it any more.
    unsafe { VirtualFree(s.cast(), SLAB, MEM_DECOMMIT) };
    if let Some(word) = a.used.get_mut(index / 64) {
        *word &= !(1 << (index % 64));
    }
    a.hint = a.hint.min(index / 64);
}

/// Set up a fresh slab.
///
/// # Safety
/// `s` points to `SLAB` committed, writable bytes that nobody uses.
unsafe fn init_slab(s: *mut Slab) {
    let start = s.cast::<u8>();
    // SAFETY: the slab is `SLAB` bytes, so `HEADER` and `SLAB` are in bounds (`end` is one past the end).
    unsafe {
        s.write(Slab { next: null_mut(), prev: null_mut(), free: null_mut(), bump: start.add(HEADER), end: start.add(SLAB), live: 0, listed: false });
    }
}

/// Put `s` at the head of the class's list.
///
/// # Safety
/// The class lock is held through `class`, and `s` is a slab of this class that is not in the list.
unsafe fn link(class: &mut Class, s: *mut Slab) {
    // SAFETY: `s` and the old head are valid slabs of this class (the lock is held).
    unsafe {
        (*s).prev = null_mut();
        (*s).next = class.head;
        if !class.head.is_null() {
            (*class.head).prev = s;
        }
        (*s).listed = true;
    }
    class.head = s;
}

/// Take `s` out of the class's list.
///
/// # Safety
/// The class lock is held through `class`, and `s` is in the list.
unsafe fn unlink(class: &mut Class, s: *mut Slab) {
    // SAFETY: `s` and its neighbours are valid slabs of this class (the lock is held).
    unsafe {
        let (prev, next) = ((*s).prev, (*s).next);
        if prev.is_null() {
            class.head = next;
        } else {
            (*prev).next = next;
        }
        if !next.is_null() {
            (*next).prev = prev;
        }
        (*s).next = null_mut();
        (*s).prev = null_mut();
        (*s).listed = false;
    }
}

/// A block of class `c`; `System` serves it (with `layout`) when no slab can be had.
fn take(c: usize, layout: Layout) -> *mut u8 {
    let size = class_size(c);
    let mut class = lock_class(c);
    if class.head.is_null() {
        let s = new_slab();
        if s.is_null() {
            drop(class);
            // SAFETY: the caller's layout is valid and not zero-sized.
            return unsafe { System.alloc(layout) };
        }
        // SAFETY: `new_slab` gave us a committed slab nobody else has; the class lock is held.
        unsafe {
            init_slab(s);
            link(&mut class, s);
        }
    }
    let s = class.head;
    // SAFETY: the head of the list is a valid slab of this class with a free block or `size` untouched bytes (the
    // invariant of the list); the lock is held.
    unsafe {
        let p = if (*s).free.is_null() {
            let p = (*s).bump;
            (*s).bump = p.add(size);
            p
        } else {
            let p = (*s).free;
            (*s).free = p.cast::<*mut u8>().read();
            p
        };
        (*s).live += 1;
        if (*s).free.is_null() && ((*s).end as usize) - ((*s).bump as usize) < size {
            unlink(&mut class, s);
        }
        p
    }
}

/// Give the block `p` of class `c` back.
///
/// # Safety
/// `p` came from a slab through `take(c, ..)` and is not used afterwards.
unsafe fn give(p: *mut u8, c: usize) {
    let mut class = lock_class(c);
    let s = ((p as usize) & !(SLAB - 1)) as *mut Slab;
    // SAFETY: `p` is a live block of the slab `s`, so the slab is mapped and has this class (the lock is held).
    unsafe {
        p.cast::<*mut u8>().write((*s).free);
        (*s).free = p;
        (*s).live -= 1;
        if !(*s).listed {
            link(&mut class, s);
        }
        if (*s).live == 0 {
            if (*s).prev.is_null() && (*s).next.is_null() {
                // The class's last slab stays, as new as it was.
                (*s).free = null_mut();
                (*s).bump = s.cast::<u8>().add(HEADER);
            } else {
                unlink(&mut class, s);
                release_slab(s);
            }
        }
    }
}

/// Dev builds: `(bytes of committed slabs, bytes of free blocks and untouched space in slabs with room, a note)`.
#[cfg(feature = "mem-stats")]
pub fn pool_report() -> (usize, usize, String) {
    let (mut free_bytes, mut listed) = (0, 0);
    for c in 0..CLASSES {
        let class = lock_class(c);
        let mut s = class.head;
        while !s.is_null() {
            // SAFETY: `s` is in this class's list (the lock is held), so it is a mapped slab of the class.
            unsafe {
                listed += 1;
                free_bytes += ((*s).end as usize) - ((*s).bump as usize);
                let mut p = (*s).free;
                while !p.is_null() {
                    free_bytes += class_size(c);
                    p = p.cast::<*mut u8>().read();
                }
                s = (*s).next;
            }
        }
    }
    let committed = lock_arena().used.iter().map(|w| w.count_ones() as usize).sum::<usize>() * SLAB;
    let spare = lock_arena().spare_n;
    (committed, free_bytes, format!(" ({listed} slabs with room, {spare} spare)"))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// A slab block of this class.
    Small(usize),
    /// Whole pages from `VirtualAlloc`.
    Big,
    /// The Windows heap.
    Heap,
}

#[inline]
fn kind(size: usize, align: usize) -> Kind {
    if size <= SMALL_MAX && align <= SMALL_ALIGN && size != 0 {
        Kind::Small(class_of(size))
    } else if size >= BIG_MIN && align <= PAGE {
        Kind::Big
    } else {
        Kind::Heap
    }
}

fn big_alloc(size: usize) -> *mut u8 {
    // SAFETY: a null address asks for a new region; the pages are committed, zeroed and read-write. `size` is not zero.
    unsafe { VirtualAlloc(null_mut(), size.next_multiple_of(PAGE), MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE) }
}

/// The global allocator of `qinglan.exe` and `qinglan-cli`.
pub struct PoolAlloc;

// SAFETY: slab blocks are distinct pieces of committed slabs (never overlapping, big and aligned enough for the class,
// 8-aligned at least, valid until freed); big blocks are whole `VirtualAlloc` regions (page-aligned, zeroed); a block
// goes back to where it came from because `dealloc` and `realloc` get the layout it was allocated with, `kind` depends
// on the layout only, and a small block that is not in the slab range came from `System`. The rest is `System`'s.
unsafe impl GlobalAlloc for PoolAlloc {
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        match kind(layout.size(), layout.align()) {
            Kind::Small(c) => take(c, layout),
            Kind::Big => big_alloc(layout.size()),
            // SAFETY: same layout, same caller contract.
            Kind::Heap => unsafe { System.alloc(layout) },
        }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        match kind(layout.size(), layout.align()) {
            Kind::Small(c) => {
                let p = take(c, layout);
                if !p.is_null() {
                    // SAFETY: the block has at least `layout.size()` bytes.
                    unsafe { p.write_bytes(0, layout.size()) };
                }
                p
            }
            // Fresh pages are zero.
            Kind::Big => big_alloc(layout.size()),
            // SAFETY: same layout, same caller contract.
            Kind::Heap => unsafe { System.alloc_zeroed(layout) },
        }
    }

    #[inline]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        match kind(layout.size(), layout.align()) {
            Kind::Small(c) if in_arena(ptr) => {
                // SAFETY: `ptr` is a slab block of class `c` that the caller got from `alloc(layout)`.
                unsafe { give(ptr, c) }
            }
            // SAFETY: a small block outside the slab range was `System`'s (the range was out of space).
            Kind::Small(_) | Kind::Heap => unsafe { System.dealloc(ptr, layout) },
            // SAFETY: `ptr` is the start of a region `big_alloc` made; releasing it frees the whole region.
            Kind::Big => unsafe {
                VirtualFree(ptr, 0, MEM_RELEASE);
            },
        }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // Where a block lives depends on its size and alignment only, so a block that grows or shrinks across a limit
        // moves (or `dealloc` would hand a block to the wrong owner).
        let (old, new) = (kind(layout.size(), layout.align()), kind(new_size, layout.align()));
        match (old, new) {
            (Kind::Heap, Kind::Heap) => {
                // SAFETY: `ptr` came from `System` with this layout.
                return unsafe { System.realloc(ptr, layout, new_size) };
            }
            (Kind::Small(a), Kind::Small(b)) if a == b && in_arena(ptr) => return ptr,
            (Kind::Big, Kind::Big) if layout.size().next_multiple_of(PAGE) == new_size.next_multiple_of(PAGE) => return ptr,
            _ => {}
        }
        // A new block, a copy, the old block back.
        // SAFETY: `new_size` is non-zero and does not overflow when rounded to the alignment (the caller's contract).
        let new_layout = unsafe { Layout::from_size_align_unchecked(new_size, layout.align()) };
        // SAFETY: a valid, non-zero layout.
        let p = unsafe { self.alloc(new_layout) };
        if !p.is_null() {
            // SAFETY: the old block has `layout.size()` bytes, the new one `new_size`; they are different blocks.
            unsafe { std::ptr::copy_nonoverlapping(ptr, p, layout.size().min(new_size)) };
            // SAFETY: `ptr` and `layout` are the pair the caller got from this allocator.
            unsafe { self.dealloc(ptr, layout) };
        }
        p
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// The tests share the global slabs; the one that counts slabs must not run beside the one that churns them.
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn serial() -> std::sync::MutexGuard<'static, ()> {
        TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn classes_round_up_and_cover_every_size() {
        let mut prev = 0;
        for size in 1..=SMALL_MAX {
            let c = class_of(size);
            assert!(c < CLASSES, "size {size}");
            assert!(class_size(c) >= size, "size {size} class {c}");
            assert!(c == 0 || class_size(c - 1) < size, "size {size} class {c} is not the smallest");
            assert!(c >= prev);
            assert_eq!(class_size(c) % 8, 0);
            assert!(HEADER + class_size(c) <= SLAB);
            prev = c;
        }
        assert_eq!(class_size(CLASSES - 1), SMALL_MAX);
        assert!(std::mem::size_of::<Slab>() <= HEADER);
    }

    fn fill(p: *mut u8, n: usize, seed: u8) {
        for i in 0..n {
            // SAFETY: the caller owns `n` bytes at `p`.
            unsafe { p.add(i).write(seed.wrapping_add(i as u8)) };
        }
    }

    fn check(p: *mut u8, n: usize, seed: u8) {
        for i in 0..n {
            // SAFETY: the caller owns `n` bytes at `p`.
            assert_eq!(unsafe { p.add(i).read() }, seed.wrapping_add(i as u8));
        }
    }

    fn check_zero(p: *mut u8, n: usize) {
        for i in 0..n {
            // SAFETY: the caller owns `n` bytes at `p`.
            assert_eq!(unsafe { p.add(i).read() }, 0);
        }
    }

    /// Several threads allocate, fill, grow, shrink and free blocks of all sizes; nobody sees another's bytes change.
    #[test]
    fn threads_do_not_share_blocks() {
        let _serial = serial();
        let handles: Vec<_> = (0..4u32)
            .map(|t| {
                std::thread::spawn(move || {
                    let a = PoolAlloc;
                    let mut state = 0x9E37_79B9u32.wrapping_mul(t + 1);
                    let mut live: Vec<(*mut u8, Layout, u8)> = Vec::new();
                    for step in 0..6_000u32 {
                        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                        let size = match state >> 28 {
                            0..=9 => 1 + (state >> 8) as usize % 200,
                            10..=11 => 1 + (state >> 8) as usize % 1100,
                            12 => 1 + (state >> 8) as usize % 5000,
                            13 => BIG_MIN - 3000 + (state >> 8) as usize % 100_000,
                            _ => 1 + (state >> 8) as usize % 5000,
                        };
                        let align = if state & 0x30 == 0x30 { 16 } else { 1 << (state & 3) };
                        let seed = (step & 0xFF) as u8;
                        match (state >> 4) & 3 {
                            0 if !live.is_empty() => {
                                let (p, l, s) = live.swap_remove(state as usize % live.len());
                                check(p, l.size(), s);
                                // SAFETY: allocated below with this layout.
                                unsafe { a.dealloc(p, l) };
                            }
                            1 if !live.is_empty() => {
                                let i = state as usize % live.len();
                                let (p, l, s) = live[i];
                                check(p, l.size(), s);
                                // SAFETY: `p` and `l` are a live pair; the new size is non-zero.
                                let q = unsafe { a.realloc(p, l, size) };
                                assert!(!q.is_null());
                                check(q, l.size().min(size), s);
                                let nl = Layout::from_size_align(size, l.align()).expect("layout");
                                fill(q, size, s);
                                live[i] = (q, nl, s);
                            }
                            _ => {
                                let l = Layout::from_size_align(size, align).expect("layout");
                                // SAFETY: a valid, non-zero layout.
                                let p = unsafe { if step % 7 == 0 { a.alloc_zeroed(l) } else { a.alloc(l) } };
                                assert!(!p.is_null());
                                assert_eq!(p as usize % align, 0);
                                if step % 7 == 0 {
                                    check_zero(p, size);
                                }
                                fill(p, size, seed);
                                live.push((p, l, seed));
                            }
                        }
                    }
                    for (p, l, s) in live {
                        check(p, l.size(), s);
                        // SAFETY: a live pair.
                        unsafe { a.dealloc(p, l) };
                    }
                })
            })
            .collect();
        for h in handles {
            h.join().expect("thread");
        }
    }

    /// Slabs fill up, come back when a block in them is freed, and are decommitted when they are empty.
    #[test]
    fn empty_slabs_are_given_back() {
        let _serial = serial();
        let a = PoolAlloc;
        let layout = Layout::from_size_align(48, 8).expect("layout");
        let committed = || lock_arena().used.iter().map(|w| w.count_ones() as usize).sum::<usize>();
        let before = committed();
        // 30 000 blocks of 48 bytes need about 90 slabs.
        let blocks: Vec<*mut u8> = (0..30_000)
            .map(|i| {
                // SAFETY: a valid, non-zero layout.
                let p = unsafe { a.alloc(layout) };
                assert!(!p.is_null());
                fill(p, 48, i as u8);
                p
            })
            .collect();
        let during = committed();
        assert!(during >= before + 50, "{before} -> {during}");
        for (i, &p) in blocks.iter().enumerate() {
            check(p, 48, i as u8);
            // SAFETY: allocated above with this layout.
            unsafe { a.dealloc(p, layout) };
        }
        let after = committed();
        assert!(after + 40 <= during, "{during} -> {after}");
        // Freed space comes back before new slabs are taken.
        // SAFETY (both blocks below): a valid, non-zero layout; each block is freed with the layout it came with.
        let again: Vec<*mut u8> = (0..1000).map(|_| unsafe { a.alloc(layout) }).collect();
        assert!(again.iter().all(|p| in_arena(*p)));
        assert!(committed() <= after + 5);
        for p in again {
            unsafe { a.dealloc(p, layout) };
        }
    }
}
