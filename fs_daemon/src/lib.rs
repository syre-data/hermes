#![feature(assert_matches)]

pub mod event;
pub use event::Event;

#[cfg(feature = "server")]
pub mod server;
