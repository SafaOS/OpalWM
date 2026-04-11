use libserver::EncodeableMessage;

use crate::{
    Name,
    defs::{IconID, ShmKey, WindowFlags, WindowID},
};

/// Destroys a Shared Object allocated with [`AllocateSharedObject`], given its [`ShmKey`].
#[derive(Debug, Clone, Copy, EncodeableMessage, PartialEq)]
pub struct DestroyObject;

/// Gets information about a global window or one that belongs to the current process, such as the name, icon, focus status, position and etc.
///
/// the response is [`super::response::WindowInfo`].
#[derive(Debug, Clone, Copy, EncodeableMessage, PartialEq)]
pub struct GetWindowInfo;

/// Allocates a region of shared memory for the client to use, returning a [`ShmKey`].
#[derive(Debug, Clone, Copy, EncodeableMessage, PartialEq)]
pub struct AllocateSharedObject {
    size_bytes: usize,
}

impl AllocateSharedObject {
    /// Returns the size of the region to allocate.
    pub const fn size(&self) -> usize {
        self.size_bytes
    }
}

#[derive(Debug, Clone, Copy, EncodeableMessage, PartialEq)]
/// Asks the WM to load a preloaded Icon given it's ID to the client, in the given shared memory region, the results are in BMP.
pub struct LoadIcon {
    store_into: ShmKey,
}

impl LoadIcon {
    /// Returns the region's key to store the Icon in.
    pub const fn store_into(&self) -> ShmKey {
        self.store_into
    }
}

#[derive(Debug, Clone, EncodeableMessage, PartialEq, Eq)]
/// Asks the WM to preload an Icon, currently only in BMP format, responds with an [`IconID`].
pub struct PreloadIcon {
    load_from: ShmKey,
    load_bytes: usize,
}

impl PreloadIcon {
    /// Returns the region's key to load the Icon from.
    pub const fn load_from(&self) -> ShmKey {
        self.load_from
    }

    /// Returns the number of bytes of the Icon to load.
    pub const fn load_bytes(&self) -> usize {
        self.load_bytes
    }
}

/// A Request to ask the WM to Create a new Window
#[derive(Debug, Clone, Copy, EncodeableMessage, PartialEq, Eq)]
pub struct CreateWindow {
    flags: WindowFlags,
    width: u32,
    height: u32,
    cus_x: Option<i32>,
    cus_y: Option<i32>,
    icon_id: Option<IconID>,
    name: Name,
    use_region: ShmKey,
}

impl CreateWindow {
    /// Returns the requested name of the window.
    pub const fn name(&self) -> &str {
        self.name.as_str()
    }

    pub const fn name_inner(&self) -> &Name {
        &self.name
    }

    /// Same as calling both [`Self::with_cus_x`] and [`Self::with_cus_y`].
    pub const fn with_pos(mut self, x: i32, y: i32) -> Self {
        self.cus_x = Some(x);
        self.cus_y = Some(y);
        self
    }

    /// Returns the requested width of the window.
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Returns the requested height of the window.
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// Returns the requested x position of the window.
    pub const fn x(&self) -> Option<i32> {
        self.cus_x
    }

    /// Returns the requested y position of the window.
    pub const fn y(&self) -> Option<i32> {
        self.cus_y
    }

    /// Returns the requested icon of the window.
    pub const fn icon(&self) -> Option<IconID> {
        self.icon_id
    }

    /// Returns the requested flags of the window.
    pub const fn flags(&self) -> WindowFlags {
        self.flags
    }

    /// The region to use and watch over for window pixels changes.
    pub const fn shm_key(&self) -> ShmKey {
        self.use_region
    }
}

/// A Request to ask the WM to mark width*height pixels as Damaged (i.e should be updated).
#[derive(Debug, Clone, Copy, EncodeableMessage, PartialEq, Eq)]
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
}

impl DamageWindow {
    /// Returns the X position within the Window.
    pub const fn x(&self) -> u32 {
        self.x
    }

    /// Returns the Y position within the Window.
    pub const fn y(&self) -> u32 {
        self.y
    }

    /// Returns the height of the given pixels to draw.
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// Returns the width of the given pixels to draw.
    pub const fn width(&self) -> u32 {
        self.width
    }
}

/// ask the WM to focus either a global window or a window belonging to self
#[derive(Debug, Clone, Copy, EncodeableMessage, PartialEq)]
pub struct FocusWindow;

/// A dummy ping request.
#[derive(Debug, Clone, Copy, EncodeableMessage, PartialEq)]
pub struct Ping;

/// Get the screen info (eg. width, height)
#[derive(Debug, Clone, Copy, EncodeableMessage, PartialEq)]
pub struct GetScreenInfo;

/// The OpalV1 request sent to the WM from a client
#[derive(Debug, Clone, EncodeableMessage, PartialEq)]
#[repr(u16)]
pub enum Request {
    /// A request to ping the WM (eg. assert the connection is alive).
    Ping(Ping) = 0xA0,
    /// See [`CreateWindow`]
    CreateWindow(CreateWindow) = 0xA1,
    /// See [`DamageWindow`]
    DamageWindow(DamageWindow, WindowID) = 0xA2,
    /// Gets screen info (eg. width, height)
    GetScreenInfo(GetScreenInfo) = 0xA3,
    PreloadIcon(PreloadIcon) = 0xA4,
    LoadIcon(LoadIcon, IconID) = 0xA5,
    GetWindowInfo(GetWindowInfo, WindowID) = 0xA6,
    FocusWindow(FocusWindow, WindowID) = 0xA7,
    AllocateObject(AllocateSharedObject) = 0xA8,
    DestroyObject(DestroyObject, ShmKey) = 0xA9,
}

impl Eq for Request {}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn test_simple_request_encoding() {
        let request = Request::Ping(Ping);
        let mut buffer = Vec::new();
        let wrote = request
            .encode_into(&mut buffer)
            .expect("Failed to encode request");
        assert_eq!(buffer.len(), wrote);
        assert_eq!(buffer, [0xA0, 0, 0]);

        let mut reader = Cursor::new(buffer);
        assert_eq!(
            Request::decode_from(&mut reader).expect("Failed to read"),
            (request, wrote)
        );
    }

    #[test]
    fn test_targeted_request_encoding() {
        let request = Request::FocusWindow(FocusWindow, 2);
        let mut buffer = Vec::new();
        let wrote = request
            .encode_into(&mut buffer)
            .expect("Failed to encode request");
        assert_eq!(buffer.len(), wrote);
        assert_eq!(buffer, [0xA7, 0, 0, 0x02, 0]);

        let mut reader = Cursor::new(buffer);
        assert_eq!(
            Request::decode_from(&mut reader).expect("Failed to read"),
            (request, wrote)
        );
    }

    #[test]
    fn test_arg_request_encoding() {
        let request_inner = CreateWindow::new(
            WindowFlags::empty(),
            0x00000640,
            0x00000420,
            Name::new("Hello!").expect("Failed to construct window name"),
            0,
        )
        .with_cus_y(0x42);
        let request = Request::CreateWindow(request_inner);
        let mut buffer = Vec::new();
        let wrote = request
            .encode_into(&mut buffer)
            .expect("Failed to encode request");

        assert_eq!(buffer.len(), wrote);
        assert_eq!(
            buffer,
            [
                0xA1, 0, 6, /* in */ 0, 0, 0, 0, 0, /* in */ 1, 0x40, 0x06, 0x00, 0x00,
                /* in */ 2, 0x20, 0x04, 0x00, 0x00, /* in */ 4, 0x42, 0x00, 0x00, 0x00,
                /* in */ 6, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, /* len */
                b'H', b'e', b'l', b'l', b'o', b'!', /* in */ 7, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, /* out */
            ]
        );

        let mut reader = Cursor::new(buffer);
        assert_eq!(
            Request::decode_from(&mut reader).expect("Failed to read"),
            (request, wrote)
        );
    }
}
