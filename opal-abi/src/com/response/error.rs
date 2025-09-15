use bincode::{Decode, Encode};

use crate::com::packet::PacketParseErr;

/// A Response Error
#[derive(Debug, Clone, Copy, Encode, Decode, PartialEq, Eq)]
#[repr(u32)]
pub enum ResponseError {
    InvalidMagic,
    InvalidRequestKind,
    PacketTooShort,
    InvalidData,
    UnknownFatalError,
    UnknownWindow,
    InvalidUtf8,
    UnknownIcon,
}

impl From<PacketParseErr> for ResponseError {
    fn from(value: PacketParseErr) -> Self {
        match value {
            PacketParseErr::InvalidMagic => ResponseError::InvalidMagic,
            PacketParseErr::InvalidPacketSize => ResponseError::PacketTooShort,
            PacketParseErr::InvalidPacketKind => ResponseError::InvalidRequestKind,
            PacketParseErr::InvalidPacketData => ResponseError::InvalidData,
        }
    }
}
