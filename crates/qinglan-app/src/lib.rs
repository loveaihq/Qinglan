//! The browser window of qinglan.exe (step 1c-2b). `qinglan.exe` itself is a thin binary that calls [`run`]; this
//! crate is size-optimized, and the binary crate (opt-level 3) sets the level of the final link-time optimization, which
//! the hot engine code needs (docs/decisions.md, "Hot and cold code").
//!
//!   qinglan.exe [address] [--replay <dir>]
//!   qinglan.exe --bench <address> [--replay <dir>] [--width W] [--height H] [--dpr D]
//!
//! `--bench` opens the real window, loads the page, scrolls it from the top to the bottom frame by frame, prints the
//! phase timings and the memory use (see `bench.rs`) and exits. `--replay` answers every request from a recording
//! made by `qinglan-cli record` (nothing goes to the network).
#![forbid(unsafe_code)]

mod address;
mod app;
mod args;
mod bench;
mod errorpage;
mod history;
mod i18n;
mod page;
mod throttle;

use std::process::ExitCode;
use std::sync::Arc;

use qinglan_engine::net::Transport;
use qinglan_engine::net::loader::Loader;
use qinglan_engine::net::replay::ReplayTransport;
use qinglan_win::WinHttpTransport;
use qinglan_win::window::{self, Waker};

/// `--replay-delay`: a replay that answers after a while, like a network does (the delay depends on the address, so the
/// responses of one page do not all arrive together).
struct Delayed {
    inner: ReplayTransport,
    delay_ms: u32,
}

impl Transport for Delayed {
    fn fetch(&self, req: &qinglan_engine::net::Request) -> Result<qinglan_engine::net::Response, qinglan_engine::net::NetError> {
        let hash = req.url.serialize(true).bytes().fold(0x811c_9dc5_u32, |h, b| (h ^ u32::from(b)).wrapping_mul(0x0100_0193));
        let extra = hash % (self.delay_ms / 2 + 1);
        std::thread::sleep(std::time::Duration::from_millis(u64::from(self.delay_ms + extra)));
        self.inner.fetch(req)
    }
}

/// The program: parse the command line, open the window (or run the bench) and return the exit code.
pub fn run() -> ExitCode {
    // `--bench ... --mem` (dev builds only) also prints what each structure holds; counting every allocation costs a
    // little time, so the timings of a run without `--mem` are the ones to compare.
    #[cfg(feature = "mem-stats")]
    {
        let flags: Vec<String> = std::env::args().collect();
        if flags.iter().any(|a| a == "--bench") && flags.iter().any(|a| a == "--mem") {
            qinglan_win::count_heap();
        }
    }
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let opts = match args::parse(&argv) {
        Ok(o) => o,
        Err(e) => {
            window::attach_console();
            eprintln!("qinglan: {e}\n{}", args::USAGE);
            return ExitCode::from(2);
        }
    };
    if opts.bench.is_some() {
        // The numbers go to stdout; a GUI process has none unless it asks for the parent's console.
        window::attach_console();
    }
    window::set_dpi_aware();

    let transport: Arc<dyn Transport> = if let Some(dir) = &opts.replay {
        match ReplayTransport::open(dir) {
            Ok(t) if opts.replay_delay_ms > 0 => Arc::new(Delayed { inner: t, delay_ms: opts.replay_delay_ms }),
            Ok(t) => Arc::new(t),
            Err(e) => {
                window::attach_console();
                eprintln!("qinglan: cannot read the recording in {}: {e}", dir.display());
                return ExitCode::FAILURE;
            }
        }
    } else {
        match WinHttpTransport::new() {
            Ok(t) => Arc::new(t),
            Err(e) => {
                window::attach_console();
                eprintln!("qinglan: cannot start WinHTTP: {e}");
                return ExitCode::FAILURE;
            }
        }
    };
    let waker = Waker::new();
    let wake = {
        let waker = waker.clone();
        Arc::new(move || waker.wake())
    };
    let loader = Loader::new(transport, wake);
    let lang = i18n::Lang::detect();
    let code = window::run(&waker, |ui| Box::new(app::Browser::new(*ui, opts, loader, lang)));
    ExitCode::from(u8::try_from(code).unwrap_or(1))
}
