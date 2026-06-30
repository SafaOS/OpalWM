pub use opal_abi::display::Pixel;
use std::sync::{LazyLock, Mutex, MutexGuard};

use safa_api::syscalls::types::Ri;
use std::fs::OpenOptions;
use std::os::safaos::AsRawResource;
use std::os::safaos::io::IoUtils;
use std::usize;

use safa_api::abi::mem::MemMapFlags;

use libserver::dlog;

#[inline]
fn interpret_cords(
    x_within: isize,
    y_within: isize,
    width_within: usize,
    height_within: usize,
    self_width: usize,
    self_height: usize,
) -> ((usize, usize), (usize, usize), (usize, usize)) {
    let x_neg = x_within.is_negative();
    let y_neg = y_within.is_negative();

    let x_abs = x_within.unsigned_abs();
    let y_abs = y_within.unsigned_abs();

    let x = if x_neg { 0 } else { x_abs };
    let y = if y_neg { 0 } else { y_abs };

    let start_col = if x_neg { x_abs } else { 0 };
    let start_row = if y_neg { y_abs } else { 0 };

    let act_width = if x_neg {
        width_within.saturating_sub(x_abs)
    } else {
        width_within
    };
    let act_height = if y_neg {
        height_within.saturating_sub(y_abs)
    } else {
        height_within
    };

    let width = act_width.min(self_width.saturating_sub(x));
    let height = act_height.min(self_height.saturating_sub(y));
    ((x, y), (start_col, start_row), (width, height))
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
/// A struct represinting information about the virtual framebuffer
pub struct FramebufferDevInfo {
    pub width: usize,
    pub height: usize,
    /// Bits per pixel, for now the virtual framebuffer always have 32bits per pixel
    bpp: usize,
    /// Whether or not each pixel is encoded as BGR and not RGB (always false for now)
    bgr: bool,
}

const CMD_RECEIVE_FB_INFO: u16 = 1;
const CMD_SYNC_PIXELS: u16 = 2;

/// A framebuffer
pub struct Framebuffer {
    width: usize,
    height: usize,
    pixels: &'static mut [Pixel],
    mmap_ri: Ri,
}

static FRAMEBUFFER_MEMMAP: LazyLock<(FramebufferDevInfo, Ri, (usize, usize))> =
    LazyLock::new(|| {
        let fb_file = OpenOptions::new()
            .write(true)
            .open("dev:/fb")
            .expect("failed to open the framebuffer");
        // First we want to receive the framebuffer info
        let mut fb_info: FramebufferDevInfo = unsafe { core::mem::zeroed() };
        fb_file
            .send_command(CMD_RECEIVE_FB_INFO, &raw mut fb_info as usize as u64)
            .expect("Failed to receive information about the framebuffer");

        assert!(fb_info.bpp == size_of::<u32>() * 8);
        assert!(!fb_info.bgr);

        dlog!("Got Framebuffer: {fb_info:#?}");
        let pixels_required = fb_info.height * fb_info.width;
        let bytes_required = pixels_required * size_of::<Pixel>();

        // The Mapping should live as long as the Process
        let (fb_ri, bytes) = safa_api::syscalls::mem::map(
            core::ptr::null(),
            bytes_required.div_ceil(4096),
            0,
            Some(fb_file.as_raw_resource() as Ri),
            None,
            MemMapFlags::WRITE,
        )
        .expect("Failed to SysMemMap the Framebuffer");

        (
            fb_info,
            fb_ri,
            (bytes.as_ptr() as *mut u8 as usize, bytes.len()),
        )
    });

/// Information about the framebuffer
pub static FB_INFO: LazyLock<FramebufferDevInfo> = LazyLock::new(|| {
    let (dev, _, _) = &*FRAMEBUFFER_MEMMAP;
    *dev
});

static FRAMEBUFFER: LazyLock<Mutex<Framebuffer>> = LazyLock::new(|| {
    let (dev, mmap_ri, (pixels_bytes_addr, _)) = &*FRAMEBUFFER_MEMMAP;
    let pixels_count = dev.width * dev.height;
    let pixels =
        unsafe { std::slice::from_raw_parts_mut(*pixels_bytes_addr as *mut Pixel, pixels_count) };
    Mutex::new(Framebuffer {
        pixels,
        mmap_ri: *mmap_ri,
        width: dev.width,
        height: dev.height,
    })
});

impl Framebuffer {
    /// Draws a rectangle with the given pixels
    /// # Arguments
    /// - `off_x`: top-left X offset within the framebuffer.
    /// - `off_y`: top-right Y offset within the framebuffer.
    /// - `width`: amount of pixels to draw per row.
    /// - `height`: amount of rows to draw
    /// - `pixels: the pixels to draw, must be at least `width` * `height` long
    pub fn draw_rect(
        &mut self,
        off_x: isize,
        off_y: isize,
        width: usize,
        height: usize,
        pixels: &[Pixel],
    ) {
        let ((off_x, off_y), (d_col_s, d_row_s), (d_width, d_height)) =
            interpret_cords(off_x, off_y, width, height, self.width, self.height);

        for row in 0..d_height {
            let target_row_index = off_x + ((off_y + row) * self.width);
            let src_row_index = d_col_s + (row + d_row_s) * width;

            if target_row_index + d_width >= self.pixels.len() {
                return;
            }

            let target_pixels = &mut self.pixels[target_row_index..target_row_index + d_width];
            let src_pixels = &pixels[src_row_index..src_row_index + d_width];
            Pixel::blend_top(src_pixels, target_pixels);
        }
    }

    /// Similar to [`Self::draw_rect`] but it also takes a position with the pixels to draw
    /// # Arguments
    /// Same as [`Self::draw_rect`] with the additional arguments:
    /// - `pixels_width`: The total width of the given pixels rectangale.
    /// - `pixels_height`: The total height of the given pixels rectangle.
    /// - `pixel_rel_x`: The relative X offset within the rectangle in which we start to draw.
    /// - `pixel_rel_y`: The relative Y offset within the rectangle in which we start to draw.
    ///
    /// We will draw to the framebuffer starting from (`off_x`, `off_y`),
    /// BUT the pixels will start from (`pixel_rel_x`, `pixel_rel_y`) and both these offsets will
    /// be relative to the given rectangale.
    pub fn draw_rect_within(
        &mut self,
        off_x: isize,
        off_y: isize,
        width: usize,
        height: usize,
        pixels: &[Pixel],
        pixels_width: usize,
        pixels_height: usize,
        pixel_rel_x: usize,
        pixel_rel_y: usize,
    ) {
        let width = width.min(pixels_width - pixel_rel_x);
        let height = height.min(pixels_height - pixel_rel_y);

        let ((off_x, off_y), (d_col_s, d_row_s), (width, height)) =
            interpret_cords(off_x, off_y, width, height, self.width, self.height);

        let x_slip = d_col_s;
        let y_slip = d_row_s;
        let pixels_x = pixel_rel_x + x_slip;
        let pixels_y = pixel_rel_y + y_slip;

        for row in 0..height {
            let target_row_index = off_x + ((off_y + row) * self.width);
            let src_row_index = pixels_x + (pixels_y + row) * pixels_width;
            if src_row_index >= pixels.len() {
                return;
            }

            let end_target_row_index = (target_row_index + width).min(self.pixels.len());
            let end_src_row_index = (src_row_index + width).min(pixels.len());

            let target_pixels = &mut self.pixels[target_row_index..end_target_row_index];
            let src_pixels = &pixels[src_row_index..end_src_row_index];

            Pixel::blend_top(src_pixels, target_pixels);
        }
    }

    /// Draws a rectangle filled with a pixel `pixel`
    pub fn draw_rect_filled_with(
        &mut self,
        off_x: isize,
        off_y: isize,
        width: usize,
        height: usize,
        pixel: Pixel,
    ) {
        let ((off_x, off_y), (_, _), (width, height)) =
            interpret_cords(off_x, off_y, width, height, self.width, self.height);

        for row in 0..height {
            let row_index = off_x + ((off_y + row) * self.width);
            let pixels = &mut self.pixels[row_index..row_index + width];
            pixels.fill(pixel);
        }
    }

    /// Syncs the full framebuffer double buffer to the real buffer
    pub fn sync_pixels_full(&self) {
        self.sync_pixels_rect(0, 0, self.width, self.height);
    }

    /// Syncs a rectangle to the framebuffer
    pub fn sync_pixels_rect(&self, off_x: isize, off_y: isize, width: usize, height: usize) {
        let ((x, y), (_, _), (width, height)) =
            interpret_cords(off_x, off_y, width, height, self.width, self.height);

        #[derive(Debug, Clone, Copy)]
        #[repr(C)]
        struct SyncRect {
            off_x: usize,
            off_y: usize,
            width: usize,
            height: usize,
        }

        let rect = SyncRect {
            off_x: x,
            off_y: y,
            width,
            height,
        };

        safa_api::syscalls::io::io_command(
            self.mmap_ri,
            CMD_SYNC_PIXELS,
            (&raw const rect) as usize as u64,
        )
        .expect("Failed to Sync framebuffer")
    }
}

/// Returns a lock on the framebuffer interface
pub fn framebuffer() -> MutexGuard<'static, Framebuffer> {
    FRAMEBUFFER
        .lock()
        .expect("Failed to acquire lock on framebuffer")
}

pub const BG_PIXEL: Pixel = Pixel::hex_rgb(0x282828);

/// Clears the screen
pub fn clear() {
    let mut fb = FRAMEBUFFER
        .lock()
        .expect("Failed to hold lock on framebuffer");
    fb.draw_rect_filled_with(0, 0, FB_INFO.width, FB_INFO.height, BG_PIXEL);
    fb.sync_pixels_full();
    dlog!("Cleared screen");
}
