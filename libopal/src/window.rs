use opal_abi::{
    Name,
    defs::WindowID,
    msg::{
        request::{CreateWindow, DamageWindow, FocusWindow, GetWindowInfo, Request},
        response::{ResponseError, WindowInfo},
    },
};

use crate::{send_request_and_get, send_request_or_panic, shm::SharedObject};
pub use opal_abi::defs::{IconID, WindowFlags};
pub use opal_abi::display::Pixel;

pub struct Window {
    win_id: u16,
    width: u32,
    height: u32,
    shm_object: SharedObject,
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
        let req = DamageWindow::new(from_x, from_y, width, height);
        send_request_or_panic!(Request::DamageWindow(req, self.win_id), Success(_s));
    }

    #[inline(always)]
    /// Returns a mutable reference to the window's pixels.
    pub const fn pixels_mut(&mut self) -> &mut [Pixel] {
        let ptr = self.shm_object.data_inner();
        unsafe {
            core::slice::from_raw_parts_mut(ptr.as_ptr().cast(), ptr.len() / size_of::<Pixel>())
        }
    }

    fn new_inner(win_id: u16, shm_object: SharedObject, width: u32, height: u32) -> Self {
        let pixels_required = width as usize * height as usize;
        let bytes_required = pixels_required * size_of::<Pixel>();
        assert!(bytes_required <= shm_object.data().len());

        Self {
            win_id,
            shm_object,
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
        let object = SharedObject::allocate(width as usize * height as usize * size_of::<Pixel>())
            .expect("Failed to allocate shared memory object");

        let mut request = CreateWindow::new(
            flags,
            width,
            height,
            Name::new_truncate(title),
            object.shm_key(),
        );
        if let Some(icon) = icon {
            request = request.with_icon_id(icon);
        }

        if let Some((x, y)) = custom_pos {
            request = request.with_pos(x, y);
        }

        let window = send_request_or_panic!(Request::CreateWindow(request), WindowCreated(w));

        let id = window.window_id();
        let mut window = Self::new_inner(id, object, width, height);
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
    send_request_and_get!(Request::FocusWindow(FocusWindow, win_id), Success(_s))
        .map_err(|e| match e {
            ResponseError::UnknownWindow => WindowError::UnknownWindowID,
            e => panic!("Unexpected error: {:?}", e),
        })
        .map(|_| ())
}

/// Returns Info about a given window
pub fn window_info(win_id: WindowID) -> Result<WindowInfo, WindowError> {
    send_request_and_get!(Request::GetWindowInfo(GetWindowInfo, win_id), WindowInfo(i)).map_err(
        |e| match e {
            ResponseError::UnknownWindow => WindowError::UnknownWindowID,
            e => panic!("Unexpected error: {:?}", e),
        },
    )
}
