//! qinglan.exe: the browser window (step 1c-2b).
//!
//!   qinglan.exe [address] [--replay <dir>]
//!   qinglan.exe --bench <address> [--replay <dir>] [--width W] [--height H] [--dpr D]
//!
//! `--bench` opens the real window, loads the page, scrolls it from the top to the bottom frame by frame, prints the
//! phase timings and the memory use (see `bench.rs`) and exits. `--replay` answers every request from a recording
//! made by `qinglan-cli record` (nothing goes to the network).
#![forbid(unsafe_code)]
#![cfg_attr(not(test), windows_subsystem = "windows")]

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

fn main() -> ExitCode {
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
