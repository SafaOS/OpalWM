use bincode::{Decode, Encode, impl_borrow_decode};
use bitflags::bitflags;

use crate::com::{
    packet::{BINCODE_CONFIG, MAX_PACKET_SIZE, PacketParseErr},
    request::{IconID, MAX_NAME_LEN, WindowFlags},
    response::error::ResponseError,
};
/// Possible response errors.
pub mod error;

/// The layout of the events the WM can send to the client, an event is a kind of [response](self)
pub mod event;

bitflags! {
    /// Information about the window status, such as if it is focused or not.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct WindowStatus: u32 {
        const FOCUSED = 1 << 0;
    }
}

impl Encode for WindowStatus {
    fn encode<E: bincode::enc::Encoder>(
        &self,
        encoder: &mut E,
    ) -> Result<(), bincode::error::EncodeError> {
        u32::encode(&self.bits(), encoder)
    }
}

impl<Context> Decode<Context> for WindowStatus {
    fn decode<D: bincode::de::Decoder<Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, bincode::error::DecodeError> {
        u32::decode(decoder).map(|bits| Self::from_bits_retain(bits))
    }
}

impl_borrow_decode!(WindowStatus);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Decode, Encode)]
#[repr(C)]
/// Response of [`super::request::GetWindowInfo`].
pub struct WindowInfo {
    icon_id: Option<IconID>,
    __0: u16,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    flags: WindowFlags,
    status: WindowStatus,
    __1: u32,
    name_len: u16,
    name_bytes: [u8; MAX_NAME_LEN],
}

impl WindowInfo {
    /// Constructs a new [`WindowInfo`] response.
    ///
    /// `name`.len() must be less than or equal to [`MAX_NAME_LEN`] or otherwise it'd panick.
    pub fn new(
        name: &str,
        icon_id: Option<IconID>,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        flags: WindowFlags,
        status: WindowStatus,
    ) -> Self {
        let len = name.len();
        assert!(
            len <= MAX_NAME_LEN,
            "Window name '{name}' has len {len}, expected {MAX_NAME_LEN} or less"
        );

        let mut buf = [0u8; MAX_NAME_LEN];
        buf[..len].copy_from_slice(name.as_bytes());

        Self {
            icon_id,
            __0: 0,
            x,
            y,
            width,
            height,
            flags,
            status,
            __1: 0,
            name_len: len as u16,
            name_bytes: buf,
        }
    }

    pub const fn name(&self) -> &str {
        let len = self.name_len as usize;
        unsafe {
            // const hack
            std::str::from_utf8_unchecked(std::slice::from_raw_parts(self.name_bytes.as_ptr(), len))
        }
    }

    pub const fn status(&self) -> WindowStatus {
        self.status
    }

    pub const fn flags(&self) -> WindowFlags {
        self.flags
    }

    pub const fn x(&self) -> i32 {
        self.x
    }

    pub const fn y(&self) -> i32 {
        self.y
    }

    pub const fn icon_id(&self) -> Option<IconID> {
        self.icon_id
    }
}

#[derive(Debug, Encode, Decode, Clone, Copy, PartialEq, Eq)]
/// Response of [`super::request::LoadIcon`]
pub struct IconData {
    __0: u64,
    __1: u64,
    icon_size: usize,
}

impl IconData {
    pub const fn new(icon_size: usize) -> Self {
        Self {
            __0: 0,
            __1: 0,
            icon_size,
        }
    }
    /// The amount of bytes to read next from the connection for the icon's data
    pub const fn size(&self) -> usize {
        self.icon_size
    }
}

#[derive(Debug, Encode, Decode, Clone, Copy, PartialEq, Eq)]
/// Response of [`super::request::PreloadIcon`]
pub struct IconPreloaded {
    __0: u64,
    __1: u64,
    icon_id: IconID,
}

impl IconPreloaded {
    pub const fn new(id: IconID) -> Self {
        Self {
            __0: 0,
            __1: 0,
            icon_id: id,
        }
    }

    pub const fn id(&self) -> IconID {
        self.icon_id
    }
}

#[derive(Debug, Encode, Decode, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
/// Response of [`super::request::CreateWindow`]
pub struct CreateWindowResp {
    /// The created window's shared memory key, it can be used to write to the window's pixels.
    shm_key: usize,
    win_id: u16,
    __0: u16,
    __1: u32,
}

impl CreateWindowResp {
    /// The created window's ID
    pub const fn window_id(&self) -> u16 {
        self.win_id
    }

    pub const fn shm_key(&self) -> usize {
        self.shm_key
    }

    pub const fn new(win_id: u16, shm_key: usize) -> Self {
        Self {
            win_id,
            shm_key,
            __0: 0,
            __1: 0,
        }
    }
}

#[derive(Debug, Encode, Decode, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
/// Response of [`super::request::RequestKind::GetScreenInfo`]
pub struct ScreenInfo {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, PartialEq, Eq, Encode, Decode)]
#[repr(u32)]
/// Represents an Ok response sent by the WM as a reply to a Request
pub enum OkResponse {
    Success,
    WindowCreated(CreateWindowResp),
    ScreenInfo(ScreenInfo),
    IconPreloaded(IconPreloaded),
    LoadingIcon(IconData),
    WindowInfo(WindowInfo),
}

#[derive(Debug, Encode, Decode, PartialEq, Eq)]
#[repr(u32)]
pub enum Response {
    Ok(OkResponse) = 0xA1E_F00D_D,
    Err(ResponseError) = 0xBAD_F00D_D,
    Event(event::WindowEvent) = 0x100_F00D_D,
}

impl Response {
    /// Encodes the response into a byte array, also returns the length of the encoded data.
    pub fn encode(&self) -> ([u8; MAX_PACKET_SIZE], usize) {
        let mut dst = [0u8; MAX_PACKET_SIZE];
        let len = bincode::encode_into_slice(self, &mut dst, BINCODE_CONFIG)
            .expect("Encoding a Response should never fail");
        (dst, len)
    }

    /// Decodes a byte array into a Response.
    pub fn decode(bytes: &[u8]) -> Result<Self, PacketParseErr> {
        Ok(bincode::decode_from_slice(bytes, BINCODE_CONFIG)?.0)
    }
}
