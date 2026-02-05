use macros::EncodeableMessage;

use crate::{
    DecodeError, Name,
    defs::{IconID, ShmKey, WindowFlags, WindowID, WindowStatus},
    impl_inheritly,
};

/// A Response Error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum ResponseError {
    /// An unknown error occurred, when the WM introduces a new error code that is not recognized by the client.
    Unknown(u16) = 0,
    /// A required parameter/structure field is missing.
    MissingParam = 1,
    /// An unexpected invalid parameter/structure field was provided.
    InvalidParam = 2,
    /// Packet was too short to contain the expected data.
    PacketTooShort = 3,
    /// The provided data is in an invalid format.
    InvalidData = 4,
    /// An unexpected fatal error occurred.
    OtherFatalError = 5,
    /// A non-existent window ID was provided.
    UnknownWindow = 6,
    /// A non-existent icon ID was provided.
    UnknownIcon = 7,
    /// The packet ended unexpectedly. (Another token was expected).
    UnexpectedEnd = 8,
    /// Given message contains an invalid opcode.
    InvalidOpCode = 9,
    /// The WM doesn't accept a key it doesn't own for security reasons.
    InvalidShmKey = 10,
    /// Allocated shared memory object is too small to hold the requested data.
    SharedObjectTooSmall = 11,
    /// The requested dimensions are invalid or not accepted (for example an Icon can be 256x256 pixels max).
    InvalidDimensions = 12,
    /// The data was loaded by the WM but not accepted, e.g. an Icon is expected to be in BMP.
    InvalidDataFormat = 13,
    /// The data was too large to be processed.
    TooLarge = 14,
    /// WM only accepts requests.
    ExpectedRequest = 15,
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
            5 => Ok(ResponseError::OtherFatalError),
            6 => Ok(ResponseError::UnknownWindow),
            7 => Ok(ResponseError::UnknownIcon),
            8 => Ok(ResponseError::UnexpectedEnd),
            9 => Ok(ResponseError::InvalidOpCode),
            10 => Ok(ResponseError::InvalidShmKey),
            11 => Ok(ResponseError::SharedObjectTooSmall),
            12 => Ok(ResponseError::InvalidDimensions),
            13 => Ok(ResponseError::InvalidDataFormat),
            14 => Ok(ResponseError::TooLarge),
            15 => Ok(ResponseError::ExpectedRequest),
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
            ResponseError::OtherFatalError => 5,
            ResponseError::UnknownWindow => 6,
            ResponseError::UnknownIcon => 7,
            ResponseError::UnexpectedEnd => 8,
            ResponseError::InvalidOpCode => 9,
            ResponseError::InvalidShmKey => 10,
            ResponseError::SharedObjectTooSmall => 11,
            ResponseError::InvalidDimensions => 12,
            ResponseError::InvalidDataFormat => 13,
            ResponseError::TooLarge => 14,
            ResponseError::ExpectedRequest => 15,
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

impl_inheritly!(u16, ResponseError, from_u16 => {
    ResponseError::from(from_u16)
}, from_self => from_self.to_u16());

#[derive(Debug, Clone, Copy, PartialEq, Eq, EncodeableMessage)]
pub struct NewSharedObject {
    key: ShmKey,
}

impl NewSharedObject {
    /// Returns the shared memory key of the shared object.
    pub const fn key(&self) -> ShmKey {
        self.key
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EncodeableMessage)]
/// Response of [`super::request::GetWindowInfo`].
pub struct WindowInfo {
    icon_id: Option<IconID>,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    flags: WindowFlags,
    status: WindowStatus,
    name: Name,
}

impl WindowInfo {
    /// Returns the name of the window.
    pub const fn name(&self) -> &str {
        self.name.as_str()
    }

    /// Returns the status of the window.
    pub const fn status(&self) -> WindowStatus {
        self.status
    }

    /// Returns the flags of the window.
    pub const fn flags(&self) -> WindowFlags {
        self.flags
    }

    /// Returns the x-coordinate of the window.
    pub const fn x(&self) -> i32 {
        self.x
    }

    /// Returns the y-coordinate of the window.
    pub const fn y(&self) -> i32 {
        self.y
    }

    /// Returns the icon ID of the window.
    pub const fn icon_id(&self) -> Option<IconID> {
        self.icon_id
    }

    /// Returns the width of the window.
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Returns the height of the window.
    pub const fn height(&self) -> u32 {
        self.height
    }
}

#[derive(Debug, Clone, EncodeableMessage, PartialEq, Eq, Copy)]
/// Response of [`super::request::LoadIcon`]
pub struct IconLoaded {
    size_bytes: usize,
}

impl IconLoaded {
    /// Returns the size of the icon in bytes.
    pub const fn size_bytes(&self) -> usize {
        self.size_bytes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EncodeableMessage)]
/// Response of [`super::request::PreloadIcon`]
pub struct IconPreloaded {
    icon_id: IconID,
}

impl IconPreloaded {
    /// Returns the ID of the preloaded icon.
    pub const fn id(&self) -> IconID {
        self.icon_id
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EncodeableMessage)]
/// Response of [`super::request::CreateWindow`]
pub struct WindowCreated {
    win_id: WindowID,
}

impl WindowCreated {
    /// The created window's ID
    pub const fn window_id(&self) -> u16 {
        self.win_id
    }
}

#[derive(Debug, EncodeableMessage, Clone, Copy, PartialEq, Eq)]
/// Response of [`super::request::RequestKind::GetScreenInfo`]
pub struct ScreenInfo {
    pub width: u32,
    pub height: u32,
}

/// Generic success response.
#[derive(Debug, EncodeableMessage, Clone, Copy, PartialEq, Eq)]
pub struct Success;

#[derive(Debug, PartialEq, Eq, EncodeableMessage, Clone, Copy)]
#[repr(u16)]
/// Represents a response sent by the WM as a reply to a Request
pub enum Response {
    Success(Success) = 0xAA0,
    WindowCreated(WindowCreated) = 0xAA1,
    ScreenInfo(ScreenInfo) = 0xAA3,
    IconPreloaded(IconPreloaded) = 0xAA4,
    LoadedIcon(IconLoaded) = 0xAA5,
    WindowInfo(WindowInfo) = 0xAA6,
    AllocatedObject(NewSharedObject) = 0xAA8,
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

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use crate::msg::{Response, ResponseError};

    #[test]
    fn test_error_encoding() {
        let error = ResponseError::InvalidParam;
        let response = Response::Error(error);

        let mut buffer = Vec::new();
        let wrote = response
            .encode_into(&mut buffer)
            .expect("Failed to encode response error");

        assert_eq!(wrote, buffer.len());
        assert_eq!(buffer, [0xFF, 0xFF, 0x2, 0x0]);

        let mut reader = Cursor::new(buffer);
        assert_eq!(
            Response::decode_from(&mut reader).expect("Failed to decode encoded response"),
            (response, wrote)
        );
    }

    #[test]
    fn test_unknown_error_encoding() {
        let error = ResponseError::Unknown(0xFFFF);
        let response = Response::Error(error);

        let mut buffer = Vec::new();
        let wrote = response
            .encode_into(&mut buffer)
            .expect("Failed to encode response error");

        assert_eq!(wrote, buffer.len());
        assert_eq!(buffer, [0xFF, 0xFF, 0xFF, 0xFF]);

        let mut reader = Cursor::new(buffer);
        assert_eq!(
            Response::decode_from(&mut reader).expect("Failed to decode encoded response"),
            (response, wrote)
        );
    }
}
