//! Starting another copy of the program with `CreateProcessW` (a new window is a new process), instead of pulling in
//! `std::process::Command`, which brings about 28 KB of environment, handle-inheritance and pipe code.

use std::ffi::c_void;
use std::ptr::{null, null_mut};

type Handle = isize;

/// `STARTUPINFOW`.
#[repr(C)]
struct StartupInfoW {
    cb: u32,
    reserved: *mut u16,
    desktop: *mut u16,
    title: *mut u16,
    x: u32,
    y: u32,
    x_size: u32,
    y_size: u32,
    x_count_chars: u32,
    y_count_chars: u32,
    fill_attribute: u32,
    flags: u32,
    show_window: u16,
    cb_reserved2: u16,
    reserved2: *mut u8,
    std_input: Handle,
    std_output: Handle,
    std_error: Handle,
}

/// `PROCESS_INFORMATION`.
#[repr(C)]
struct ProcessInformation {
    process: Handle,
    thread: Handle,
    process_id: u32,
    thread_id: u32,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn CreateProcessW(
        application: *const u16,
        command_line: *mut u16,
        process_attributes: *const c_void,
        thread_attributes: *const c_void,
        inherit_handles: i32,
        creation_flags: u32,
        environment: *const c_void,
        current_directory: *const u16,
        startup_info: *const StartupInfoW,
        process_information: *mut ProcessInformation,
    ) -> i32;
    fn CloseHandle(handle: Handle) -> i32;
}

/// Start the program named by the first token of `command_line` (UTF-16, quoted as the C runtime parses it) with
/// default security, no inherited handles and the parent's environment and directory. Returns whether it started;
/// the child is not waited for.
pub fn create(command_line: &mut Vec<u16>) -> bool {
    // The buffer must end in a NUL, and is writable because `CreateProcessW` may edit it.
    if command_line.last() != Some(&0) {
        command_line.push(0);
    }
    let si = StartupInfoW {
        cb: std::mem::size_of::<StartupInfoW>() as u32,
        reserved: null_mut(),
        desktop: null_mut(),
        title: null_mut(),
        x: 0,
        y: 0,
        x_size: 0,
        y_size: 0,
        x_count_chars: 0,
        y_count_chars: 0,
        fill_attribute: 0,
        flags: 0,
        show_window: 0,
        cb_reserved2: 0,
        reserved2: null_mut(),
        std_input: 0,
        std_output: 0,
        std_error: 0,
    };
    let mut pi = ProcessInformation { process: 0, thread: 0, process_id: 0, thread_id: 0 };
    // SAFETY: `command_line` is a NUL-terminated, writable UTF-16 buffer that outlives the call; `si` is fully
    // initialised and its `cb` is its size; `pi` is a valid out-pointer; every other pointer argument is null, which
    // the function documents as "none / inherit".
    let ok = unsafe { CreateProcessW(null(), command_line.as_mut_ptr(), null(), null(), 0, 0, null(), null(), &si, &mut pi) };
    if ok == 0 {
        return false;
    }
    // SAFETY: both handles were returned by the successful call above and are closed exactly once, here.
    unsafe {
        CloseHandle(pi.process);
        CloseHandle(pi.thread);
    }
    true
}
