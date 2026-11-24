#![forbid(unsafe_code)]

//! Forge OS desktop shell core (launcher, status pill, notifications) — Blueprint 6.7.

pub mod launcher;
pub mod notify;
pub mod pill;

#[cfg(feature = "gtk-ui")]
pub mod ui;
