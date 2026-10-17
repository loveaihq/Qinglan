//! qinglan.exe: a thin shell around `qinglan-app`. It is the crate that is linked last, and the opt-level of the
//! final crate sets the level of the link-time optimization over the whole program, so it stays at 3 (hot engine code
//! stays fast) while everything it calls is size-optimized.
#![forbid(unsafe_code)]
#![windows_subsystem = "windows"]

// Dev builds (`--features mem-stats`) count live heap bytes in `--bench --mem` runs (the memory breakdown); release
// builds use the system allocator as is.
#[cfg(feature = "mem-stats")]
#[global_allocator]
static ALLOC: qinglan_win::CountingAlloc = qinglan_win::CountingAlloc;

#[cfg(not(feature = "mem-stats"))]
#[global_allocator]
static ALLOC: qinglan_win::PoolAlloc = qinglan_win::PoolAlloc;

fn main() -> std::process::ExitCode {
    qinglan_app::run()
}
