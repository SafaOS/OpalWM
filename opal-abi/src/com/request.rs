use std::num::NonZero;

use bincode::{Decode, Encode, impl_borrow_decode};
use bitflags::bitflags;

use crate::com::packet::{BINCODE_CONFIG, MAX_PACKET_SIZE, PacketParseErr};

bitflags! {
    /// Flags to create a new window with
    #[derive(Debug, Clone, Copy)]
    pub struct WindowFlags: u32 {
        /// The window shall come below normal windows, and cannot be dragged or focused on.
        const BG_WINDOW = 1 << 0;
        /// The window shall come on top of normal windows, and cannot be dragged or focused on.
        const OVERLAY_WINDOW = 1 << 1;
        /// The [`CreateWindow`] Request's x and y field refers to an absolute position within the screen,
        /// and shall not be ignored.
        const ABS_POS = 1 << 2;
        /// The window's creation/removal is public information,
        /// anyone can access the window ID and a global event will be bordcast on
        /// creation/removal and some window changes.
        const GLOBAL = 1 << 3;
    }
}

impl Encode for WindowFlags {
    fn encode<E: bincode::enc::Encoder>(
        &self,
        encoder: &mut E,
    ) -> Result<(), bincode::error::EncodeError> {
        u32::encode(&self.bits(), encoder)
    }
}

impl<Context> Decode<Context> for WindowFlags {
    fn decode<D: bincode::de::Decoder<Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, bincode::error::DecodeError> {
        u32::decode(decoder).map(|bits| WindowFlags::from_bits_retain(bits))
    }
}

impl_borrow_decode!(WindowFlags);

/// The maximum length of a window's name
pub const MAX_NAME_LEN: usize = 128;

/// Identifies an Icon as the result of [`PreloadIcon`].
pub type IconID = NonZero<u16>;

#[derive(Debug, Clone, Copy, Encode, Decode)]
/// Asks the WM to load a preloaded Icon given it's ID to the client, the results are in BMP.
#[repr(C)]
pub struct LoadIcon {
    __: u64,
    id: IconID,
}

impl LoadIcon {
    pub const fn new(id: IconID) -> Self {
        Self { __: 0, id }
    }

    pub const fn id(&self) -> IconID {
        self.id
    }
}

#[derive(Debug, Clone, Copy, Encode, Decode)]
#[repr(C)]
/// Asks the WM to preload an Icon, currently only in BMP format, responds with an [`IconID`].
pub struct PreloadIcon {
    __: u64,
    len: usize,
}

impl PreloadIcon {
    pub const fn new(len: usize) -> Self {
        Self { __: 0, len }
    }

    pub const fn icon_size(&self) -> usize {
        self.len
    }
}

/// A Request to ask the WM to Create a new Window
#[derive(Debug, Clone, Copy, Encode, Decode)]
#[repr(C)]
pub struct CreateWindow {
    flags: WindowFlags,
    width: u32,
    height: u32,
    cus_x: i32,
    cus_y: i32,
    icon_id: Option<IconID>,
    name_len: u16,
    name: [u8; MAX_NAME_LEN],
}

const _: () = assert!(MAX_NAME_LEN <= u16::MAX as usize);

impl CreateWindow {
    /// Constructs a new [`CreateWindow`] Request
    pub fn new(
        name: &str,
        flags: WindowFlags,
        width: u32,
        height: u32,
        icon_id: Option<IconID>,
    ) -> Self {
        let name_len = name
            .char_indices()
            .rev()
            .map(|(i, _)| i + 1)
            .find(|len| *len <= MAX_NAME_LEN)
            .unwrap_or(0);

        let mut buf = [0u8; MAX_NAME_LEN];
        buf[..name_len].copy_from_slice(&name.as_bytes()[..name_len]);
        Self {
            flags,
            width,
            height,
            cus_x: 0,
            cus_y: 0,
            name_len: name_len as u16,
            name: buf,
            icon_id,
        }
    }

    /// Returns the requested name of the window
    pub const fn name(&self) -> &str {
        let len = self.name_len as usize;
        unsafe {
            // const hack
            std::str::from_utf8_unchecked(std::slice::from_raw_parts(self.name.as_ptr(), len))
        }
    }

    pub const fn with_pos(mut self, x: i32, y: i32) -> Self {
        self.flags = self.flags.union(WindowFlags::ABS_POS);
        self.cus_x = x;
        self.cus_y = y;
        self
    }

    pub const fn width(&self) -> u32 {
        self.width
    }

    pub const fn height(&self) -> u32 {
        self.height
    }

    pub const fn x(&self) -> Option<i32> {
        if self.flags.contains(WindowFlags::ABS_POS) {
            Some(self.cus_x)
        } else {
            None
        }
    }

    pub const fn y(&self) -> Option<i32> {
        if self.flags.contains(WindowFlags::ABS_POS) {
            Some(self.cus_y)
        } else {
            None
        }
    }

    pub const fn flags(&self) -> WindowFlags {
        self.flags
    }
}

/// A Request to ask the WM to mark width*height pixels as Damaged (i.e should be updated).
#[derive(Debug, Clone, Copy, Encode, Decode)]
#[repr(C)]
pub struct DamageWindow {
    /// X Position within the Window
    x: u32,
    /// Y Position within the Window
    y: u32,
    /// Width of the given pixels to draw
    width: u32,
    /// Height of the given pixels to draw
    height: u32,
    /// The ID of the target Window
    win_id: u16,
    __0: u16,
}

impl DamageWindow {
    pub const fn new(win_id: u16, start_x: u32, start_y: u32, width: u32, height: u32) -> Self {
        Self {
            x: start_x,
            y: start_y,
            width,
            height,
            win_id,
            __0: 0,
        }
    }

    pub const fn win_id(&self) -> u16 {
        self.win_id
    }

    pub const fn x(&self) -> u32 {
        self.x
    }

    pub const fn y(&self) -> u32 {
        self.y
    }

    pub const fn height(&self) -> u32 {
        self.height
    }

    pub const fn width(&self) -> u32 {
        self.width
    }
}

/// The kind of request sent to the WM from a client
#[derive(Debug, Clone, Copy, Encode, Decode)]
#[repr(u32)]
pub enum RequestKind {
    /// A request to ping the WM (ensures the connection is alive)
    Ping,
    /// See [`CreateWindow`]
    CreateWindow(CreateWindow),
    /// See [`DamageWindow`]
    DamageWindow(DamageWindow),
    /// Gets screen info (eg. width, height)
    GetScreenInfo,
    PreloadIcon(PreloadIcon),
    LoadIcon(LoadIcon),
}

#[derive(Encode, Decode, Clone, Copy, Debug)]
#[repr(u32)]
pub(crate) enum ReqMagicNumInner {
    RequestMagic = 0xBC_FEED_AD,
}

/// The layout of a Request sent to the WM from a client
#[derive(Debug, Encode, Decode)]
#[repr(C)]
pub struct Request {
    magic: ReqMagicNumInner,
    kind: RequestKind,
}

impl Request {
    /// Constructs a new Request with the given kind.
    pub const fn new(kind: RequestKind) -> Self {
        Self {
            magic: ReqMagicNumInner::RequestMagic,
            kind,
        }
    }

    pub const fn kind(&self) -> &RequestKind {
        &self.kind
    }

    /// Encodes the Request into a byte array and returns the length of the encoded data.
    pub fn encode(self) -> ([u8; MAX_PACKET_SIZE], usize) {
        let mut dst = [0u8; MAX_PACKET_SIZE];
        let len = bincode::encode_into_slice(self, &mut dst, BINCODE_CONFIG)
            .expect("Encoding a Request should never fail");
        (dst, len)
    }

    /// Decodes a byte array into a Request.
    pub fn decode(data: &[u8]) -> Result<Self, PacketParseErr> {
        Ok((bincode::decode_from_slice(data, BINCODE_CONFIG)?).0)
    }
}
