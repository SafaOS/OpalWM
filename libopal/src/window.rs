use std::ptr::NonNull;

use opal_abi::com::{
    request::{CreateWindow, DamageWindow, FocusWindow, GetWindowInfo, IconID, RequestKind},
    response::{WindowInfo, error::ResponseError},
};
use safa_api::{
    abi::mem::{MemMapFlags, ShmFlags},
    syscalls::types::Ri,
};

use crate::{send_request_and_get, send_request_or_panic};
pub use opal_abi::com::request::WindowFlags;
pub use opal_abi::fb::Pixel;

pub struct Window {
    win_id: u16,
    width: u32,
    height: u32,
    pixels: NonNull<[Pixel]>,
    pixels_mmap_ri: Ri,
}

impl Drop for Window {
    fn drop(&mut self) {
        safa_api::syscalls::resources::destroy_resource(self.pixels_mmap_ri)
            .expect("Window's pixels Dropped too early");
    }
}

unsafe impl Send for Window {}
unsafe impl Sync for Window {}

impl Window {
    #[inline]
    pub const fn height(&self) -> u32 {
        self.height
    }

    #[inline]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Redraws the window's pixels as a rectangle starting at (from_x, from_y) with the given width and height.
    pub fn redraw(&self, from_x: u32, from_y: u32, width: u32, height: u32) {
        let req = DamageWindow::new(self.win_id, from_x, from_y, width, height);
        send_request_or_panic!(RequestKind::DamageWindow(req), Success)
    }

    #[inline(always)]
    /// Returns a mutable reference to the window's pixels.
    pub const fn pixels_mut(&mut self) -> &mut [Pixel] {
        unsafe { self.pixels.as_mut() }
    }

    fn new_inner(win_id: u16, shm_key: usize, width: u32, height: u32) -> Self {
        let pixels_required = width as usize * height as usize;
        let bytes_required = pixels_required * size_of::<Pixel>();
        let pages_required = bytes_required.div_ceil(4096);

        let shm_resource =
            safa_api::syscalls::mem::shm_open(shm_key, ShmFlags::from_bits_retaining(0))
                .expect("WM Returned an Invalid SHM Key");

        let (pixels_mmap_ri, pixels_bytes) = safa_api::syscalls::mem::map(
            core::ptr::null(),
            pages_required,
            0,
            Some(shm_resource),
            None,
            MemMapFlags::WRITE,
        )
        .expect("Failed to map SHM given by the WM");

        safa_api::syscalls::resources::destroy_resource(shm_resource)
            .expect("Failed to destroy SHM Resource");

        let pixels = NonNull::slice_from_raw_parts(pixels_bytes.cast::<Pixel>(), pixels_required);
        Self {
            win_id,
            pixels,
            pixels_mmap_ri,
            width,
            height,
        }
    }

    /// Request the creation of a new window from the WM.
    pub fn create(
        title: &str,
        flags: WindowFlags,
        width: u32,
        height: u32,
        custom_pos: Option<(i32, i32)>,
        icon: Option<IconID>,
    ) -> Self {
        let mut request = CreateWindow::new(title, flags, width, height, icon);
        if let Some((x, y)) = custom_pos {
            request = request.with_pos(x, y);
        }

        let window = send_request_or_panic!(RequestKind::CreateWindow(request), WindowCreated(w));

        let id = window.window_id();
        let mut window = Self::new_inner(id, window.shm_key(), width, height);
        window.pixels_mut().fill(Pixel::NONE);

        window
    }
    // Returns the ID of this window
    pub const fn id(&self) -> u16 {
        self.win_id
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Error that can occur when interacting with a window
pub enum WindowError {
    UnknownWindowID,
}

/// Requests the WM to put a given window in focus
pub fn focus_window(win_id: u16) -> Result<(), WindowError> {
    send_request_and_get!(RequestKind::FocusWindow(FocusWindow::new(win_id)), Success).map_err(
        |e| match e {
            ResponseError::UnknownWindow => WindowError::UnknownWindowID,
            ResponseError::InvalidData
            | ResponseError::InvalidMagic
            | ResponseError::InvalidRequestKind
            | ResponseError::PacketTooShort
            | ResponseError::InvalidUtf8
            | ResponseError::UnknownFatalError
            | ResponseError::UnknownIcon => panic!("Unexpected error: {:?}", e),
        },
    )
}

/// Returns Info about a given window
pub fn window_info(win_id: u16) -> Result<WindowInfo, WindowError> {
    send_request_and_get!(
        RequestKind::GetWindowInfo(GetWindowInfo::new(win_id)),
        WindowInfo(i)
    )
    .map_err(|e| match e {
        ResponseError::UnknownWindow => WindowError::UnknownWindowID,
        ResponseError::InvalidData
        | ResponseError::InvalidMagic
        | ResponseError::InvalidRequestKind
        | ResponseError::PacketTooShort
        | ResponseError::InvalidUtf8
        | ResponseError::UnknownFatalError
        | ResponseError::UnknownIcon => panic!("Unexpected error: {:?}", e),
    })
}
