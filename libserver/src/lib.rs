//! Defines layout of structures to communicate with a userspace server built for SafaOS, and utils to help build the server

pub mod encoding;
pub mod executor;
pub mod logging;
mod misc;
pub mod vtty;

pub use encoding::{DecodeError, DecodeErrorOrIo};
pub use misc::{BufOfMax, Name, StrOfMax};
pub use msg_macros::*;

pub use encoding as msg_prelude;
