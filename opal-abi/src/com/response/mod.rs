use bincode::{Decode, Encode};

use crate::com::{
    packet::{BINCODE_CONFIG, MAX_PACKET_SIZE, PacketParseErr},
    response::error::ResponseError,
};
/// Possible response errors.
pub mod error;

/// The layout of the events the WM can send to the client, an event is a kind of [response](self)
pub mod event;

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
}

#[derive(Debug, Encode, Decode, PartialEq, Eq)]
#[repr(u32)]
pub enum Response {
    Ok(OkResponse) = 0xA1E_F00D_D,
    Err(ResponseError) = 0xBAD_F00D_D,
    Event(event::Event) = 0x100_F00D_D,
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
