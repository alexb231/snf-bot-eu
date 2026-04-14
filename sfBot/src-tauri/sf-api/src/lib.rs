
#![warn(
    clippy::indexing_slicing,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::print_stdout,
    clippy::print_stderr,
    missing_debug_implementations,
    clippy::pedantic,
    
)]
#![allow(
    clippy::redundant_closure_for_method_calls,
    clippy::wildcard_imports,
    clippy::too_many_lines,
    clippy::field_reassign_with_default,
    clippy::match_bool
)]
#![deny(unsafe_code)]

pub mod command;
pub mod error;
pub mod gamestate;
pub mod misc;
pub mod response;
#[cfg(feature = "session")]
pub mod session;
#[cfg(feature = "simulation")]
pub mod simulate;
#[cfg(feature = "sso")]
pub mod sso;





pub type PlayerId = u32;

#[cfg(feature = "session")]
pub use session::SimpleSession;