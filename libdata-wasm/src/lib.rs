#![forbid(future_incompatible)]
#![forbid(rust_2018_idioms, rust_2021_compatibility)]
#![deny(missing_debug_implementations)]
#![warn(unsafe_code, bad_style, nonstandard_style)]
#![deny(missing_docs)]
#![cfg_attr(test, allow(dead_code))]
#![cfg_attr(test, warn(warnings))]

//! Re-export [libdata] functionality in an opinionated wrapper.

pub mod keys;
pub mod storage;
pub mod core;
pub mod multicore;
pub mod websocket;
