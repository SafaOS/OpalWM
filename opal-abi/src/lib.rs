//! Defines layout of structures to communicate with the OpalWM

/// The abstract socket address to use to connect with the OpalWM
pub const CONNECT_ABSTRACT_ADDR: &str = "opal_wm::connect";

pub mod defs;
pub mod display;
pub mod msg;

mod encoding;
mod misc;

pub use encoding::{DecodeError, DecodeErrorOrIo};
pub use misc::{BufOfMax, Name, StrOfMax};
