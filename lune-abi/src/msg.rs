use libserver::{DecodeError, EncodeableMessage};

use crate::{ShmKey, StreamID};

/// Allocates a region of shared memory for the client to use, returning a [`ShmKey`].
#[derive(Debug, Clone, Copy, EncodeableMessage, PartialEq, Eq)]
pub struct AllocateSharedObject {
    size_bytes: usize,
}

impl AllocateSharedObject {
    pub const fn size(&self) -> usize {
        self.size_bytes
    }
}

#[derive(Debug, Clone, Copy, EncodeableMessage, PartialEq, Eq)]
pub struct ObjectAllocated {
    shm_key: ShmKey,
}

#[derive(Debug, Clone, Copy, EncodeableMessage, PartialEq, Eq)]
#[repr(C)]
pub struct CreateStream {
    use_region: ShmKey,
    samples: u32,
    channels: u8,
    bit_depth: u8,
    /// 0 => singed int
    /// 1 => floating point value, only f32 is currently supported.
    sample_kind: u8,
    freq_hz: u32,
}

impl CreateStream {
    pub const fn region(&self) -> ShmKey {
        self.use_region
    }
    pub const fn samples_count(&self) -> u32 {
        self.samples
    }
    pub const fn sample_kind(&self) -> u8 {
        self.sample_kind
    }
    pub const fn samples_per_second(&self) -> u32 {
        self.freq_hz
    }
    pub const fn channels_count(&self) -> u8 {
        self.channels
    }
    pub const fn bits_per_sample(&self) -> u8 {
        self.bit_depth
    }
}

#[derive(Debug, Clone, Copy, EncodeableMessage, PartialEq, Eq)]
pub struct StreamCreated {
    stream_id: StreamID,
}

#[derive(Debug, Clone, Copy, EncodeableMessage, PartialEq, Eq)]
pub struct SyncStream {
    sample_offset: u32,
    samples: u32,
}

impl SyncStream {
    pub const fn samples_count(&self) -> u32 {
        self.samples
    }

    pub const fn from_offset(&self) -> u32 {
        self.sample_offset
    }
}

#[derive(Debug, Clone, Copy, EncodeableMessage, PartialEq, Eq)]
pub struct StreamSynced {
    synced_samples: u32,
}

impl StreamSynced {
    /// Returns the amount of samples synced.
    pub const fn synced(&self) -> u32 {
        self.synced_samples
    }
}

#[derive(Debug, Clone, Copy, EncodeableMessage, PartialEq, Eq)]
pub struct Ping;

#[derive(Debug, Clone, Copy, EncodeableMessage, PartialEq, Eq)]
pub struct Success;

#[derive(Debug, Clone, Copy, EncodeableMessage, PartialEq)]
#[repr(u16)]
pub enum Request {
    Ping(Ping) = 0xA0,
    CreateStream(CreateStream) = 0xA1,
    AllocateObject(AllocateSharedObject) = 0xA8,
    DestroyObject(ShmKey) = 0xA9,
    SyncStream(SyncStream, StreamID) = 0xA5,
    DestroyStream(StreamID) = 0xE0,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum ResponseError {
    /// An unknown error occurred, when the Server introduces a new error code that is not recognized by the client.
    Unknown(u16) = 0,
    /// A required parameter/structure field is missing.
    MissingParam = 1,
    /// An unexpected invalid parameter/structure field was provided.
    InvalidParam = 2,
    /// Packet was too short to contain the expected data.
    PacketTooShort = 3,
    /// The provided data is in an invalid format.
    InvalidData = 4,
    /// The data was too large to be processed.
    TooLarge = 5,
    /// Server only accepts requests.
    ExpectedRequest = 6,
    /// An unexpected fatal error occurred.
    OtherFatalError = 7,
    UnexpectedEnd = 8,
    InvalidOpCode = 9,
    /// The given stream ID is not reconigzed by the server.
    UnknownStreamID = 10,
    /// The given Shm Key is not reconigzed by the server.
    UnknownShmKey = 11,
    /// Only 1 or 2 channels allowed
    InvalidChannelsCount = 12,
    /// Invalid Stream frequency.
    InvalidFreq = 13,
    InvalidSampleFormat = 14,
    ShmSizeTooSmall = 15,
}

impl From<DecodeError> for ResponseError {
    fn from(value: DecodeError) -> Self {
        match value {
            DecodeError::InvalidParam(_) | DecodeError::TooManyParams => {
                ResponseError::InvalidParam
            }
            DecodeError::BufferTooSmall => ResponseError::PacketTooShort,
            DecodeError::InvalidData => ResponseError::InvalidData,
            DecodeError::MissingParam => ResponseError::MissingParam,
            DecodeError::UnexpectedEnd => ResponseError::UnexpectedEnd,
            DecodeError::InvalidOpCode(_) => ResponseError::InvalidOpCode,
            DecodeError::UnexpectedMessage => ResponseError::ExpectedRequest,
        }
    }
}

impl ResponseError {
    /// Try to create a ResponseError from a u16 value.
    ///
    /// Returning a [`ResponseError::Unknown`] if the value is not recognized.
    #[inline(always)]
    pub const fn try_from(value: u16) -> Result<Self, Self> {
        match value {
            0 => Ok(ResponseError::Unknown(0)),
            1 => Ok(ResponseError::MissingParam),
            2 => Ok(ResponseError::InvalidParam),
            3 => Ok(ResponseError::PacketTooShort),
            4 => Ok(ResponseError::InvalidData),
            5 => Ok(ResponseError::TooLarge),
            6 => Ok(ResponseError::ExpectedRequest),
            7 => Ok(ResponseError::OtherFatalError),
            8 => Ok(ResponseError::UnexpectedEnd),
            9 => Ok(ResponseError::InvalidOpCode),
            10 => Ok(ResponseError::UnknownStreamID),
            11 => Ok(ResponseError::UnknownShmKey),
            12 => Ok(ResponseError::InvalidChannelsCount),
            13 => Ok(ResponseError::InvalidFreq),
            14 => Ok(ResponseError::InvalidSampleFormat),
            15 => Ok(ResponseError::ShmSizeTooSmall),
            o => Err(ResponseError::Unknown(o)),
        }
    }

    /// Converts a [`ResponseError`] to a u16 value.
    pub const fn to_u16(&self) -> u16 {
        match self {
            ResponseError::Unknown(o) => *o,
            ResponseError::MissingParam => 1,
            ResponseError::InvalidParam => 2,
            ResponseError::PacketTooShort => 3,
            ResponseError::InvalidData => 4,
            ResponseError::TooLarge => 5,
            ResponseError::ExpectedRequest => 6,
            ResponseError::OtherFatalError => 7,
            ResponseError::UnexpectedEnd => 8,
            ResponseError::InvalidOpCode => 9,
            ResponseError::UnknownStreamID => 10,
            ResponseError::UnknownShmKey => 11,
            ResponseError::InvalidChannelsCount => 12,
            ResponseError::InvalidFreq => 13,
            ResponseError::InvalidSampleFormat => 14,
            ResponseError::ShmSizeTooSmall => 15,
        }
    }

    /// Creates a [`ResponseError`] from a u16 value.
    pub const fn from(value: u16) -> Self {
        match Self::try_from(value) {
            Ok(o) => o,
            Err(e) => e,
        }
    }
}

libserver::impl_inheritly!(u16, ResponseError, from_u16 => {
    ResponseError::from(from_u16)
}, from_self => from_self.to_u16());

#[derive(Debug, Clone, Copy, PartialEq, Eq, EncodeableMessage)]
pub struct AllocatedShmObject {
    key: ShmKey,
}

impl AllocatedShmObject {
    pub const fn key(&self) -> ShmKey {
        self.key
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EncodeableMessage)]
#[repr(u16)]
pub enum Response {
    Success(Success) = 0xAA0,
    StreamSynced(StreamSynced, StreamID) = 0xAA1,
    StreamID(StreamID) = 0xAA2,
    AllocatedObject(AllocatedShmObject) = 0xAA3,
    Error(ResponseError) = 0xFFFF,
}

impl Response {
    /// Returns [`Self::Success`]
    pub const fn ok() -> Self {
        Self::Success(Success)
    }

    /// Returns [`Self::Error`]
    pub const fn err(error: ResponseError) -> Self {
        Self::Error(error)
    }
}

/// Defines the maximum size of a message in bytes.
pub const MAX_MESSAGE_SIZE: usize = 1024;

/// Lune message type.
#[derive(Debug, Clone, PartialEq, EncodeableMessage)]
#[wrapped]
#[repr(u16)]
pub enum Message {
    /// See [`Request`].
    Request(Request) = 0xFEED,
    /// See [`Response`].
    Response(Response) = 0xFED,
}

const _: () = assert!(
    size_of::<Message>() <= MAX_MESSAGE_SIZE,
    "Message size exceeds maximum allowed size"
);
