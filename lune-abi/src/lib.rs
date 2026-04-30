/// The ID of a constructed Stream.
pub type StreamID = u32;
/// Describes a shared memory Key, the server doesn't accept a key it doesn't own for security reasons.
pub type ShmKey = usize;

/// The abstract socket address to use to connect with luneaudio
pub const CONNECT_ABSTRACT_ADDR: &str = "luneaudio::connect";

pub mod misc;
pub mod msg;
pub(crate) use libserver::encoding;
pub use libserver::{BufOfMax, Name, StrOfMax};
pub use libserver::{DecodeError, DecodeErrorOrIo};
