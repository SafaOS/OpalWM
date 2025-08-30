use std::ptr::NonNull;

use opal_abi::com::{
    request::{CreateWindow, DamageWindow, RequestKind},
    response::{OkResponse, Response},
};
use safa_api::{
    abi::mem::{MemMapFlags, ShmFlags},
    syscalls::types::Ri,
};

use crate::send_request;
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
        assert_eq!(
            send_request(RequestKind::DamageWindow(DamageWindow::new(
                self.win_id,
                from_x,
                from_y,
                width,
                height,
            )))
            .expect("Failed to send Damage Window request"),
            Response::Ok(OkResponse::Success),
            "Damage Window request returned an unexpected response"
        );
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
    pub fn create(flags: WindowFlags, width: u32, height: u32) -> Self {
        let resp = send_request(RequestKind::CreateWindow(CreateWindow::new(
            flags, width, height,
        )))
        .expect("Failed to send Create Window Request");

        let window = match resp {
            Response::Ok(OkResponse::WindowCreated(w)) => w,
            Response::Err(e) => panic!("Failed to create window: {:?}", e),
            _ => panic!("Unexpected response, {:#?}", resp),
        };

        let id = window.window_id();
        let mut window = Self::new_inner(id, window.shm_key(), width, height);
        window
            .pixels_mut()
            .fill(Pixel::from_rgb_with_alpha(0, 0, 0, 0x0));

        window
    }
}
