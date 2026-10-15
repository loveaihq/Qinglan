//! Windows platform layer for qinglan (DirectWrite, WinHTTP, WIC, window).
//! Empty in step 1a. `unsafe` is only allowed under `sys/`.
#![deny(unsafe_code)]

#[allow(unsafe_code)]
mod sys;
