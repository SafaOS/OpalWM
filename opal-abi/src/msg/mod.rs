//! Module containing ABI definitions and helper encoders and decoders for OpalWM's Message Protocol.
pub mod event;
pub mod request;
use macros::EncodeableMessage;
pub use request::*;
pub mod response;
pub use response::*;

use crate::msg::event::Event;

/// Defines the maximum size of a message in bytes.
pub const MAX_MESSAGE_SIZE: usize = 1024;

/// OpalV1 message type.
#[derive(Debug, Clone, PartialEq, Eq, EncodeableMessage)]
#[wrapped]
#[repr(u16)]
pub enum OpalV1 {
    /// See [`Request`].
    Request(Request) = 0xFEED,
    /// See [`Response`].
    Response(Response) = 0xFED,
    Event(Event) = 0xF00D,
}

/// Generic message type.
#[derive(Debug, Clone, PartialEq, Eq, EncodeableMessage)]
#[repr(u16)]
pub enum Message {
    OpalV1(OpalV1) = 0xA001,
}

impl Message {
    /// Constructs a new OpalV1 request.
    pub const fn new_request(request: Request) -> Self {
        Self::OpalV1(OpalV1::Request(request))
    }

    /// Constructs a new OpalV1 response.
    pub const fn new_response(response: Response) -> Self {
        Self::OpalV1(OpalV1::Response(response))
    }

    /// Constructs a new OpalV1 window event.
    pub const fn new_event(event: Event) -> Self {
        Self::OpalV1(OpalV1::Event(event))
    }
}

const _: () = assert!(
    size_of::<Message>() <= MAX_MESSAGE_SIZE,
    "Message size exceeds maximum allowed size"
);

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use crate::{
        DecodeError, DecodeErrorOrIo,
        msg::{OpalV1, Request},
    };

    #[test]
    pub fn test_wrapped_encoding() {
        let request = OpalV1::Request(Request::Ping(super::Ping));
        let mut buffer = Vec::new();
        let wrote = request.encode_into(&mut buffer).expect("Failed to encode");

        assert_eq!(wrote, buffer.len());
        assert_eq!(buffer, [0xED, 0xFE, 0xA0, 0x00, 0x00, 0xED, 0xFE]);

        let decoded = OpalV1::decode_from(&mut Cursor::new(&buffer)).expect("Failed to decode");
        assert_eq!(decoded, (request, wrote));
    }

    #[test]
    pub fn test_error_wrapped_encoding() {
        let buffer0 = [0xED, 0xFE, 0xA0, 0x00, 0, 0xF0, 0x0D];
        let buffer1 = [0xED, 0xFE, 0xA0, 0x00];

        match OpalV1::decode_from(&mut Cursor::new(buffer0)).expect_err("Expected an Error") {
            DecodeErrorOrIo::DecodeError(DecodeError::UnexpectedEnd) => (),
            e => unreachable!("Unexpected error 1: {e:#?}"),
        }

        match OpalV1::decode_from(&mut Cursor::new(buffer1)).expect_err("Expected an Error") {
            DecodeErrorOrIo::DecodeError(DecodeError::BufferTooSmall) => (),
            e => unreachable!("Unexpected error 2: {e:#?}"),
        }
    }
}
