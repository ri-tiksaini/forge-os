#![forbid(unsafe_code)]

//! Standalone Forge OS Developer Labs (System Internals + Virtual OS Simulator).

pub mod sim;

#[cfg(feature = "gtk-ui")]
pub mod ui;
