use std::{
    collections::HashMap,
    io::ErrorKind,
    iter::Sum,
    ops::Add,
    ptr::NonNull,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

use indexmap::IndexSet;
use opal_abi::com::response::{Response, event::Event};
use opal_img::bmp::BMPImage;
use rustc_hash::{FxBuildHasher, FxHashMap};
use safa_api::abi::mem::{MemMapFlags, ShmFlags};

use crate::{
    REALLY_VERBOSE,
    com::ClientComPipe,
    dlog, elog,
    framebuffer::{self, BG_PIXEL, FB_INFO, Framebuffer, Pixel},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DamageRegion {
    pos_x: usize,
    pos_y: usize,
    width: usize,
    height: usize,
}

impl DamageRegion {
    #[inline]
    /// Checks if self overlaps with `win` returning the point which is covered from the window
    pub fn overlaps_with(&self, win: &Window) -> Option<IntersectionPoint> {
        let d_x0 = self.pos_x;
        let d_x1 = self.pos_x + self.width;
        let d_y0 = self.pos_y;
        let d_y1 = self.pos_y + self.height;

        let w_x0 = win.pos_x;
        let w_x1 = win.pos_x + win.width;
        let w_y0 = win.pos_y;
        let w_y1 = win.pos_y + win.height;

        if (d_x0 < w_x1 && d_x1 > w_x0) && (d_y0 < w_y1 && d_y1 > w_y0) {
            let i_x0 = d_x0.max(w_x0) - w_x0;
            let i_x1 = d_x1.min(w_x1) - w_x0;
            let i_y0 = d_y0.max(w_y0) - w_y0;
            let i_y1 = d_y1.min(w_y1) - w_y0;

            Some(IntersectionPoint {
                top_left_within: (i_x0, i_y0),
                bottom_right_within: (i_x1, i_y1),
            })
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum WindowDamageReason {
    Moving {
        old: DamageRegion,
        new: DamageRegion,
    },
    Redraw {
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        region: DamageRegion,
    },
    Whole(DamageRegion),
}

impl WindowDamageReason {
    pub fn place_regions(&self, to: &mut Vec<DamageRegion>) {
        match self {
            Self::Whole(w) => to.push(*w),
            Self::Redraw { region, .. } => to.push(*region),
            Self::Moving { old, new } => {
                to.reserve(2);
                to.push(*old);
                to.push(*new);
            }
        }
    }
}

// a Rectangle
pub struct Window {
    //
    pos_x: usize,
    pos_y: usize,
    //
    width: usize,
    height: usize,
    /// The pixels of the window, safe to use because they live as long as the window itself.
    pixels: NonNull<[Pixel]>,
    // TODO: Implement a good shared memory wrapper to drop this automatically.
    shm_key: usize,
    // TODO: Implement a good shared memory or a resource wrapper to drop this automatically.
    shm_ri: usize,
    // TODO: Implement a good memory map or a resource wrapper to drop this automatically.
    mmap_ri: usize,
    com_pipe: Option<Arc<ClientComPipe>>,
    damage_reason: Option<WindowDamageReason>,
}

impl Drop for Window {
    fn drop(&mut self) {
        safa_api::syscalls::resources::destroy_resource(self.shm_ri)
            .expect("SHM was dropped before Window was dropped");
        safa_api::syscalls::resources::destroy_resource(self.mmap_ri)
            .expect("MMAP was dropped before Window was dropped");
    }
}

unsafe impl Send for Window {}
unsafe impl Sync for Window {}

impl Window {
    /// Returns a new instance of the Window with the given command pipe to send events to.
    pub fn with_com_pipe(mut self, com_pipe: Arc<ClientComPipe>) -> Self {
        self.com_pipe = Some(com_pipe);
        self
    }

    /// A shared memory key that lives as long as the window itself, and can be used to access the window's pixels.
    pub const fn shm_key(&self) -> &usize {
        &self.shm_key
    }

    /// Sends an event to the client that owns this window.
    pub fn send_event(&self, event: Event) {
        if let Some(com_pipe) = &self.com_pipe {
            if let Err(err) = com_pipe.sender().send_response(Response::Event(event))
                && err.kind() != ErrorKind::ConnectionAborted
                && err.kind() != ErrorKind::ConnectionReset
            {
                // TODO: Maybe this is fatal?
                elog!("Failed to send an event {event:#?} to the client err: {err:?}, ignoring...")
            }
        }
    }

    fn allocate_pixel_buffer(
        width: usize,
        height: usize,
        fill_pixel: Pixel,
    ) -> (NonNull<[Pixel]>, usize, usize, usize) {
        let pixels_required = width * height;
        let bytes_required = pixels_required * size_of::<Pixel>();
        let pages_required = bytes_required.div_ceil(4096);

        let (shm_key, shm_ri) =
            safa_api::syscalls::mem::shm_create(pages_required, ShmFlags::from_bits_retaining(0))
                .expect("Failed to create a new shared mem mapping for a Window");

        let (mmap_ri, pixels_bytes) = safa_api::syscalls::mem::map(
            core::ptr::null(),
            pages_required,
            0,
            Some(shm_ri),
            None,
            MemMapFlags::WRITE,
        )
        .expect("Failed to memmap a new Window's pixels");

        let mut pixels =
            NonNull::slice_from_raw_parts(pixels_bytes.cast::<Pixel>(), pixels_required);
        unsafe {
            pixels.as_mut().fill(fill_pixel);
        }
        (pixels, shm_ri, mmap_ri, shm_key)
    }

    /// Creates a new Window from a given BMP Image
    pub fn new_from_bmp(pos_x: usize, pos_y: usize, image: BMPImage) -> Window {
        Self::new_from_pixels(
            pos_x,
            pos_y,
            image.width() as usize,
            image.height() as usize,
            image
                .pixels()
                .map(|c| Pixel::from_rgb(c.red(), c.green(), c.blue()).with_alpha(c.alpha())),
        )
    }

    /// Creates a new Window and fills it with `fill_pixels`
    pub fn new_from_pixels(
        pos_x: usize,
        pos_y: usize,
        width: usize,
        height: usize,
        fill_pixels: impl ExactSizeIterator + Iterator<Item = Pixel>,
    ) -> Window {
        let (mut pixels, shm_ri, mmap_ri, shm_key) =
            Self::allocate_pixel_buffer(width, height, Pixel::from_hex_argb(0));
        let pixels_mut = unsafe { pixels.as_mut() };

        assert_eq!(
            pixels.len(),
            fill_pixels.len(),
            "The pixels to fill with must have a length of width*height"
        );

        let fill_pixels = fill_pixels.enumerate();
        for (i, pi) in fill_pixels {
            pixels_mut[i] = pi;
        }

        Window {
            pos_x,
            pos_y,
            width,
            height,
            pixels,
            shm_key,
            shm_ri,
            mmap_ri,
            com_pipe: None,
            damage_reason: None,
        }
    }

    /// Creates a new Window and fills it repeatedly with a given `pixel`
    pub fn new_filled_with(
        pos_x: usize,
        pos_y: usize,
        width: usize,
        height: usize,
        pixel: Pixel,
    ) -> Self {
        let (pixels, shm_ri, mmap_ri, shm_key) = Self::allocate_pixel_buffer(width, height, pixel);

        Window {
            pos_x,
            pos_y,
            width,
            height,
            pixels,
            shm_ri,
            mmap_ri,
            shm_key,
            com_pipe: None,
            damage_reason: None,
        }
    }

    /// Draws the whole window without syncing the results to the real framebuffer.
    ///
    /// [`fb.sync_pixels_rect`] must be called afterwards on the area the window is in.
    fn draw(&self, fb: &mut Framebuffer) {
        fb.draw_rect(self.pos_x, self.pos_y, self.width, self.height, unsafe {
            self.pixels.as_ref()
        });
    }

    /// Draws the window from intersection point without syncing the results to the real framebuffer.
    ///
    /// [`fb.sync_pixels_rect`] must be called afterwards on the area the window is in.
    fn draw_at(&self, fb: &mut Framebuffer, point: IntersectionPoint) {
        let (top_x_within, top_y_within) = point.top_left_within;
        let width = point.width();
        let height = point.height();

        let pixels = &self.pixels;
        let pixels_width = self.width;
        let pixels_height = self.height;

        if width == pixels_width && height == pixels_height {
            return self.draw(fb);
        }

        // The offset within the FB is the offset of self + the point
        let off_x = self.pos_x + top_x_within;
        let off_y = self.pos_y + top_y_within;

        // We want to draw pixels that `point` cover only
        fb.draw_rect_within(
            off_x,
            off_y,
            width,
            height,
            unsafe { pixels.as_ref() },
            pixels_width,
            pixels_height,
            top_x_within,
            top_y_within,
        );
    }

    const fn get_whole_damage(&self) -> DamageRegion {
        DamageRegion {
            pos_x: self.pos_x,
            pos_y: self.pos_y,
            width: self.width,
            height: self.height,
        }
    }

    fn moved(&mut self, prev: DamageRegion, new: DamageRegion) {
        self.update_damage(WindowDamageReason::Moving { old: prev, new });
    }

    /// Returns the damage a window may have caused on the framebuffer, if it's position or dimensions changed
    /// There is 2 damages: The damage before the operation, The damage after the operation
    fn damage_whole(&mut self) {
        let region = self.get_whole_damage();
        self.update_damage(WindowDamageReason::Whole(region));
    }

    /// Returns the potential damage to be done on given corridations inside of the window
    fn get_damage_within(&self, x: usize, y: usize, width: usize, height: usize) -> DamageRegion {
        let x = x.min(self.width);
        let y = y.min(self.height);

        let pos_x = (self.pos_x + x).min(self.pos_x + self.width);
        let pos_y = (self.pos_y + y).min(self.pos_y + self.height);
        let width = width.min(self.width - x);
        let height = height.min(self.height - y);

        DamageRegion {
            pos_x,
            pos_y,
            width,
            height,
        }
    }

    fn damage_within(&mut self, x: usize, y: usize, width: usize, height: usize) {
        self.update_damage(WindowDamageReason::Redraw {
            x,
            y,
            width,
            height,
            region: self.get_damage_within(x, y, width, height),
        });
    }

    /// Take the damage reason of self, places None in place of it
    const fn take_damage(&mut self) -> Option<WindowDamageReason> {
        self.damage_reason.take()
    }

    /// Updates the current damage reason
    fn update_damage(&mut self, reason: WindowDamageReason) {
        let new_reason = match (self.damage_reason, reason) {
            // Moving take priority, the newer the more prior
            (
                Some(WindowDamageReason::Moving { old, .. }),
                WindowDamageReason::Moving { new, .. },
            ) => WindowDamageReason::Moving { old, new },
            (Some(_), WindowDamageReason::Moving { .. }) => reason,
            (Some(r @ WindowDamageReason::Moving { .. }), _) => r,
            // Then comes whole damage
            (Some(_), WindowDamageReason::Whole(_)) => reason, /* both work */
            (Some(r @ WindowDamageReason::Whole(_)), _) => r,
            // Then finally we combine redraws
            (
                Some(WindowDamageReason::Redraw {
                    x: s_x,
                    y: s_y,
                    width: s_wid,
                    height: s_hei,
                    region: _,
                }),
                WindowDamageReason::Redraw {
                    x: o_x,
                    y: o_y,
                    width: o_wid,
                    height: o_hei,
                    region: _,
                },
            ) => {
                let n_x = s_x.min(o_x);
                let n_y = s_y.min(o_y);
                let max_n_x = s_x.max(o_x);
                let max_n_y = s_y.max(o_y);

                let diff_x = max_n_x - n_x;
                let diff_y = max_n_y - n_y;
                let width = s_wid.max(o_wid) + diff_x;
                let height = s_hei.max(o_hei) + diff_y;

                let n_r = self.get_damage_within(n_x, n_y, width, height);
                WindowDamageReason::Redraw {
                    x: n_x,
                    y: n_y,
                    width: width,
                    height: height,
                    region: n_r,
                }
            }
            (None, reason) => reason,
        };

        self.damage_reason = Some(new_reason);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntersectionPoint {
    top_left_within: (usize, usize),
    bottom_right_within: (usize, usize),
}

impl IntersectionPoint {
    pub const fn none() -> Self {
        Self {
            top_left_within: (0, 0),
            bottom_right_within: (0, 0),
        }
    }

    pub const fn width(&self) -> usize {
        let (top_x, _) = self.top_left_within;
        let (bott_x, _) = self.bottom_right_within;
        bott_x - top_x
    }

    pub const fn height(&self) -> usize {
        let (_, top_y) = self.top_left_within;
        let (_, bott_y) = self.bottom_right_within;
        bott_y - top_y
    }

    /// Returns the x-coordinate of the intersection point, from the top-left corner.
    pub const fn x(&self) -> usize {
        let (top_x, _) = self.top_left_within;
        top_x
    }

    /// Returns the y-coordinate of the intersection point, from the top-left corner.
    pub const fn y(&self) -> usize {
        let (_, top_y) = self.top_left_within;
        top_y
    }
}

impl Add<IntersectionPoint> for IntersectionPoint {
    type Output = IntersectionPoint;
    fn add(self, rhs: IntersectionPoint) -> Self::Output {
        let (s_top_x, s_top_y) = self.top_left_within;
        let (o_top_x, o_top_y) = rhs.top_left_within;
        let (s_bott_x, s_bott_y) = self.bottom_right_within;
        let (o_bott_x, o_bott_y) = rhs.bottom_right_within;
        Self {
            top_left_within: (s_top_x.min(o_top_x), s_top_y.min(o_top_y)),
            bottom_right_within: (s_bott_x.max(o_bott_x), s_bott_y.max(o_bott_y)),
        }
    }
}

impl Sum for IntersectionPoint {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        let mut results = IntersectionPoint::none();

        for i in iter {
            if results == IntersectionPoint::none() {
                results = i;
            } else {
                results = results + i;
            }
        }

        results
    }
}

const MAX_WINDOW_ID: usize = 1024 /* TODO: more windows? */;
/// A window ID
pub type WinID = u16;

/// The type of the Window, defines the ordering which a Window may come over another, for example the cursor uses [`WindowKind::Overlay`]
#[derive(Debug, Clone, Copy)]
pub enum WindowKind {
    /// Always displayed above all other windows
    Overlay,
    /// Normal ordering
    Normal,
    /// Background Window, always displayed below all other windows
    Background,
}

pub struct Windows {
    windows: FxHashMap<WinID, (Window, WindowKind)>,
    /// Windows that always come on top of other windows
    overlay_windows: IndexSet<WinID, FxBuildHasher>,
    /// The ordering of the windows in the Z Axis, the focused Window comes last
    normal_windows: IndexSet<WinID, FxBuildHasher>,
    /// Windows that always come below all other windows
    background_windows: IndexSet<WinID, FxBuildHasher>,

    /// A list of window IDs
    /// currently stored using a Bitmap and the max is 1024
    window_ids: [u128; 8],
    focused_window: Option<WinID>,

    damaged_regions_tmp: Vec<DamageRegion>,
}

impl Windows {
    pub const fn new() -> Self {
        Self {
            overlay_windows: IndexSet::with_hasher(FxBuildHasher),
            normal_windows: IndexSet::with_hasher(FxBuildHasher),
            background_windows: IndexSet::with_hasher(FxBuildHasher),
            focused_window: None,

            damaged_regions_tmp: Vec::new(),
            windows: HashMap::with_hasher(FxBuildHasher),
            window_ids: [0; 8],
        }
    }

    #[inline]
    fn signal_redraw(&mut self) {
        SHOULD_REDRAW.store(true, Ordering::Release);
    }

    /// Allocates a new Window ID
    fn add_id(&mut self) -> Option<WinID> {
        for (row, byte) in self.window_ids.iter_mut().enumerate() {
            let width = size_of_val(byte) * 8;

            for col in 0..width {
                let bit = ((*byte >> col) & 1) == 1;
                if !bit {
                    *byte |= 1 << col;
                    return Some((col + (row * width)) as WinID);
                }
            }
        }

        None
    }

    /// Deallocate an existing Window ID
    /// returns true if successful, false if the ID is invalid
    fn remove_id(&mut self, id: WinID) -> bool {
        if id as usize >= MAX_WINDOW_ID {
            return false;
        }

        let width = size_of_val(&self.window_ids[0]);
        let row = (id / width as u16) as usize;
        let col = (id % width as u16) as usize;

        let byte = &mut self.window_ids[row];
        let bit = ((*byte >> col) & 1) == 1;
        let will_succeed = bit;

        if will_succeed {
            *byte &= !(1 << col);
        }

        will_succeed
    }

    /// Redraw the damage caused by (and apply the results of) playing around with the windows using `self`
    pub fn damage_redraw(&mut self) {
        for (_, (win, _)) in self.windows.iter_mut() {
            if let Some(toke) = win.take_damage() {
                toke.place_regions(&mut self.damaged_regions_tmp);
            }
        }
        if self.damaged_regions_tmp.is_empty() {
            return;
        }

        let mut fb = framebuffer::framebuffer();

        let damage = &mut self.damaged_regions_tmp;

        for region in &*damage {
            // Clear the damaged region
            fb.draw_rect_filled_with(
                region.pos_x,
                region.pos_y,
                region.width,
                region.height,
                BG_PIXEL,
            );
        }

        // Fixes all the damages caused on a window if any
        macro_rules! fix_window {
            ($win: expr) => {{
                let win = $win;
                let intersection: IntersectionPoint =
                    damage.iter().filter_map(|d| d.overlaps_with(&win)).sum();

                if intersection != IntersectionPoint::none() {
                    win.draw_at(&mut fb, intersection);
                }
            }};
        }

        for win_id in &self.background_windows {
            let (window, _) = self
                .windows
                .get_mut(win_id)
                .expect("Background window wasn't removed from the Z-Ordering when it was removed");
            fix_window!(window);
        }

        for win_id in &self.normal_windows {
            let (window, _) = self
                .windows
                .get_mut(win_id)
                .expect("Window wasn't removed from the Z-Ordering when it was removed");
            fix_window!(window);
        }

        // Overlay on top of other windows
        for win_id in &self.overlay_windows {
            let (window, _) = self
                .windows
                .get_mut(win_id)
                .expect("Overlay window wasn't removed from the Z-Ordering when it was removed");
            fix_window!(window);
        }

        for r in damage.drain(..) {
            fb.sync_pixels_rect(r.pos_x, r.pos_y, r.width, r.height);
        }

        SHOULD_REDRAW.store(false, Ordering::Release);
    }

    /// Adds `x` to window with the ID  `win_id` x position and `y` to the window with the ID `win_id`'s Y position
    ///
    /// Returns the new position if the Window ID exist
    pub fn add_cord(&mut self, win_id: WinID, x: i32, y: i32) -> Option<(usize, usize)> {
        let (win, _) = self.windows.get_mut(&win_id)?;

        /* The guarantee that this will be successful, is that we have a mutable reference on Self and that all access on the Window will be performed from Self */
        if x == 0 && y == 0 {
            return Some((win.pos_x, win.pos_y));
        }

        let damage0 = win.get_whole_damage();

        let max_x = FB_INFO.width;
        let max_y = FB_INFO.height;

        win.pos_x = std::cmp::min(
            win.pos_x.saturating_add_signed(x as isize),
            max_x - win.width,
        );
        win.pos_y = std::cmp::min(
            win.pos_y.saturating_add_signed(y as isize),
            max_y - win.height,
        );

        if win.pos_x == damage0.pos_x && win.pos_y == damage0.pos_y {
            return Some((win.pos_x, win.pos_y));
        }

        let damage1 = win.get_whole_damage();

        if REALLY_VERBOSE {
            dlog!(
                "window changed from x: {}, y: {} to x: {}, y: {} as per: {x}, {y}",
                damage0.pos_x,
                damage0.pos_y,
                damage1.pos_x,
                damage1.pos_y
            );
        }

        win.moved(damage0, damage1);
        self.signal_redraw();
        Some((damage1.pos_x, damage1.pos_y))
    }

    /// Adds a window and organizes it depending on `kind` (see [`WindowKind`]).
    /// Reposititons the window to fit most of the screen.
    pub fn add_window(&mut self, mut window: Window, kind: WindowKind) -> Option<WinID> {
        let screen_width = FB_INFO.width;
        let screen_height = FB_INFO.height;

        if window.pos_x + window.width > screen_width {
            window.pos_x = 0;
        } else {
            window.pos_x = (screen_width - window.width) / 2;
        }

        if window.pos_y + window.height > screen_height {
            window.pos_y = 0;
        } else {
            window.pos_y = (screen_height - window.height) / 2;
        }
        // Damage the window
        window.damage_whole();

        let id = self.add_id()?;
        self.windows.insert(id, (window, kind));

        match kind {
            WindowKind::Normal => self.set_focused(id),
            WindowKind::Overlay => self.overlay_windows.insert(id),
            WindowKind::Background => self.background_windows.insert(id),
        };
        self.signal_redraw();
        Some(id)
    }

    /// Set the window with the id `win_id` as focused,
    /// handles everything including sending events and damage, and reordering the Z-list.
    pub fn set_focused(&mut self, win_id: WinID) -> bool {
        let Some((window, window_kind)) = self.windows.get_mut(&win_id) else {
            return false;
        };

        let old_value = self.focused_window.replace(win_id);

        window.send_event(Event::WindowFocused);
        window.damage_whole();
        let window_kind = *window_kind;

        if let Some(old_id) = old_value
            && let Some((win, _)) = self.windows.get(&old_id)
        {
            win.send_event(Event::WindowUnfocused);
        }

        match window_kind {
            WindowKind::Normal => {
                self.normal_windows.shift_remove(&win_id);
                self.normal_windows.insert(win_id);
            }
            WindowKind::Overlay => {
                self.normal_windows.shift_remove(&win_id);
                self.overlay_windows.insert(win_id);
            }
            WindowKind::Background => {
                self.normal_windows.shift_remove(&win_id);
                self.background_windows.insert(win_id);
            }
        };

        self.signal_redraw();
        true
    }

    /// Unfocus the currently focused window.
    pub fn unfocus_current(&mut self) {
        if let Some(win_id) = self.focused_window.take() {
            if let Some((win, _)) = self.windows.get_mut(&win_id) {
                win.send_event(Event::WindowUnfocused);
                win.damage_whole();
                self.signal_redraw();
            }
        }
    }

    /// Returns the ID of the top-most window that is in contact with the given position and size if any, and also the contact point
    pub fn window_in_contact(
        &self,
        pos_x: usize,
        pos_y: usize,
        width: usize,
        height: usize,
    ) -> Option<(WinID, IntersectionPoint)> {
        let region = DamageRegion {
            pos_x,
            pos_y,
            width,
            height,
        };

        self.normal_windows.iter().rev().find_map(|win_id| {
            let (win, _) = self
                .windows
                .get(win_id)
                .expect("Window wasn't removed from the Z-ordering when it's ID was deallocated");
            region.overlaps_with(win).map(|point| (*win_id, point))
        })
    }

    /// Returns the ID of the focused Window
    pub const fn focused_window(&self) -> Option<WinID> {
        self.focused_window
    }

    pub fn damage_window(
        &mut self,
        win_id: WinID,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
    ) -> Result<(), ()> {
        let (win, _) = self.windows.get_mut(&win_id).ok_or(())?;
        win.damage_within(x, y, width, height);
        self.signal_redraw();
        Ok(())
    }

    pub fn send_event(&mut self, win_id: WinID, event: Event) -> Result<(), ()> {
        let (win, _) = self.windows.get_mut(&win_id).ok_or(())?;
        win.send_event(event);
        Ok(())
    }

    /// Completely removes a window from the window manager.
    pub fn remove_window(&mut self, win_id: WinID) -> Result<(), ()> {
        if let Some(focused_id) = self.focused_window
            && focused_id == win_id
        {
            self.focused_window = None;
        }

        let (window, window_kind) = self.windows.remove(&win_id).ok_or(())?;
        let whole = window.get_whole_damage();
        self.damaged_regions_tmp.push(whole);

        match window_kind {
            WindowKind::Normal => {
                assert!(
                    self.normal_windows.shift_remove(&win_id),
                    "Window has not placed in the normal Z-ordering"
                );
            }
            WindowKind::Overlay => {
                assert!(
                    self.overlay_windows.shift_remove(&win_id),
                    "Window has not placed in the overlay Z-ordering"
                );
            }
            WindowKind::Background => {
                assert!(
                    self.background_windows.shift_remove(&win_id),
                    "Window has not placed in the background Z-ordering"
                );
            }
        }

        assert!(
            self.remove_id(win_id),
            "Unexpected behavior, ID should have been removed successfully"
        );
        dlog!("Window removed");
        self.signal_redraw();
        Ok(())
    }
}

pub static WINDOWS: Mutex<Windows> = Mutex::new(Windows::new());

/// Adds a window with `kind` kind, returns the ID of the window.
/// Repositions the window to fit most of it on the screen.
pub fn add_window(window: Window, kind: WindowKind) -> Option<WinID> {
    WINDOWS
        .lock()
        .expect("Failed to acquire lock on Windows while adding a Window")
        .add_window(window, kind)
}

pub fn damage_window(
    win_id: WinID,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
) -> Result<(), ()> {
    WINDOWS
        .lock()
        .expect("Failed to acquire lock on Windows while damaging a Window")
        .damage_window(win_id, x, y, width, height)
}

/// Whether we should redraw the screen
static SHOULD_REDRAW: AtomicBool = AtomicBool::new(false);

/// Returns true if you should call `redraw_screen`
fn should_redraw() -> bool {
    SHOULD_REDRAW.load(Ordering::Acquire)
}

/// Better called from a single thread at a time
/// Redraws changed areas of the screen in case we need to
pub fn redraw() {
    if should_redraw() {
        WINDOWS
            .lock()
            .expect("Failed to acquire lock on Windows while redrawing")
            .damage_redraw();
    }
}
