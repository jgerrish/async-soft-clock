//! async_soft_clock is a crate that simulates a hardware clock.
#![warn(missing_docs)]
#![warn(unsafe_code)]

pub mod async_executor;
pub mod clock;
pub mod error;
pub mod listener;
pub mod time;
