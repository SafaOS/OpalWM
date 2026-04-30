use std::{
    collections::HashMap,
    io::ErrorKind,
    ptr::NonNull,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    task::Waker,
};

use indexmap::IndexSet;
use libopal::{
    defs::{IconID, WindowStatus},
    event::{Event, WindowAttached, WindowDeatached, WindowEvent, WindowFocusChanged},
    window::WindowFlags,
};
use opal_abi::{Name, msg::Message};
use opal_img::bmp::BMPImage;
use rustc_hash::{FxBuildHasher, FxHashMap};

pub mod decorations;
pub mod primitive;
use crate::{
    REALLY_VERBOSE,
    com::ClientComPipe,
    framebuffer::{self, BG_PIXEL, FB_INFO, Framebuffer, Pixel},
    window::{
        decorations::WindowDecorationsMeta,
        primitive::{
            DamageRegion, IntersectionPoint, Point, Rect, TransformRect, UPoint, WindowDamageReason,
        },
    },
};
use libserver::{dlog, elog};

use safa_api::shm::SharedObject;

/// Describes resources for a shared window with another program.
pub struct SharedWindow {
    border: Option<WindowDecorationsMeta>,
    /// The pixels of the window, safe to use because they live as long as the window itself.
    pixels: NonNull<[Pixel]>,
    bounds: Rect,
    _shm_object: Arc<SharedObject>,
    com_pipe: Arc<ClientComPipe>,
}

impl SharedWindow {
    #[must_use = "Returns the damaged region within the window if it exists"]
    #[inline]
    /// Synchronizes pixels from the shared buffer to the given buffer, given a sync area to sync within.
    fn sync_pixels_to(
        &self,
        dst: &mut [Pixel],
        sync_x: usize,
        sync_y: usize,
        sync_width: usize,
        sync_height: usize,
    ) -> Option<(UPoint, Rect)> {
        let max_height = self.bounds.height();
        let max_width = self.bounds.width();

        if sync_y >= max_height || sync_x >= max_width {
            return None;
        }

        let width = (max_width - sync_x).min(sync_width);
        let height = (max_height - sync_y).min(sync_height);
        let sync_bounds = Rect::new(width, height);

        let location = WindowDecorationsMeta::copy_pixels(
            self.border.as_ref(),
            unsafe { self.pixels.as_ref() },
            dst,
            sync_bounds,
            sync_x,
            sync_y,
            self.bounds,
        );

        Some((location, sync_bounds))
    }
    /// Allocates a user interface for the window.
    ///
    /// Returns None if the SHM object is too small to hold window data.
    fn create(
        win_bounds: Rect,
        shm_object: Arc<SharedObject>,
        com_pipe: Arc<ClientComPipe>,
        decorate: bool,
        fill_with: Pixel,
    ) -> Option<(Self, Box<[Pixel]>, Rect, Point)> {
        let width = win_bounds.width();
        let height = win_bounds.height();
        let pixels_ptr = shm_object.data_ptr();
        if !(width * height * size_of::<Pixel>() <= pixels_ptr.len()) {
            return None;
        }
        let mut pixels = NonNull::slice_from_raw_parts(pixels_ptr.cast::<Pixel>(), width * height);

        unsafe {
            pixels.as_mut().fill(fill_with);
        }

        let border;
        let win_wm_pixels;
        let actual_bounds;
        let anchor;
        if decorate {
            let (borders, back_pixels, bounds, anchor_by) =
                WindowDecorationsMeta::new(win_bounds, fill_with);
            border = Some(borders);
            win_wm_pixels = back_pixels;
            actual_bounds = bounds;
            anchor = anchor_by;
        } else {
            border = None;
            win_wm_pixels = vec![fill_with; width * height].into_boxed_slice();
            actual_bounds = win_bounds;
            anchor = Point::new(0, 0);
        }

        Some((
            SharedWindow {
                pixels,
                _shm_object: shm_object,
                com_pipe,
                border,
                bounds: win_bounds,
            },
            win_wm_pixels,
            actual_bounds,
            anchor,
        ))
    }
}

/// A Rectangle
pub struct Window {
    rect: TransformRect,
    pixels: Box<[Pixel]>,
    shared_resources: Option<SharedWindow>,
    icon: Option<IconID>,
    name: Name,
    flags: WindowFlags,
    status: WindowStatus,
}

unsafe impl Send for Window {}
unsafe impl Sync for Window {}

impl Window {
    fn add_cord(&mut self, x: i32, y: i32) -> (isize, isize, bool) {
        let damage0 = self.get_whole_damage();

        let max_x = FB_INFO.width;
        let max_y = FB_INFO.height;

        let new_x = (self.x() + x as isize)
            .min(max_x as isize - 16)
            .max(-(self.width() as isize) + 16);

        let new_y = (self.y() + y as isize)
            .min(max_y as isize - 16)
            .max(-(self.height() as isize) + 16);

        // Rect didn't change position
        if (new_x, new_y) == (self.x(), self.y()) {
            return (self.x(), self.y(), false);
        }

        self.rect.position = Point::new(new_x, new_y);
        let damage1 = self.get_whole_damage();

        if REALLY_VERBOSE {
            dlog!(
                "Rect changed from {:?} to {:?} as per: {x}, {y}",
                damage0.position(),
                damage1.position(),
            );
        }

        self.rect.moved(damage0, damage1);
        (self.x(), self.y(), true)
    }

    fn set_position_in_place(&mut self, x: isize, y: isize) {
        let point = Point::new(x, y);
        self.rect.set_pos(point);
    }
    pub const fn x(&self) -> isize {
        self.rect.x()
    }

    pub const fn y(&self) -> isize {
        self.rect.y()
    }

    pub const fn width(&self) -> usize {
        self.rect.width()
    }

    pub const fn height(&self) -> usize {
        self.rect.height()
    }

    pub const fn max_x(&self) -> isize {
        self.rect.x() + self.rect.width() as isize
    }

    pub const fn max_y(&self) -> isize {
        self.rect.y() + self.rect.height() as isize
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub const fn icon(&self) -> Option<IconID> {
        self.icon
    }

    pub const fn flags(&self) -> WindowFlags {
        self.flags
    }

    pub const fn status(&self) -> WindowStatus {
        self.status
    }

    // /// Returns a new instance of the Window with the given command pipe to send events to.
    // pub fn with_com_pipe(mut self, com_pipe: Arc<ClientComPipe>) -> Self {
    //     self.com_pipe = Some(com_pipe);
    //     self
    // }
    //

    /// Sends an event to the client that owns this window.
    fn send_event_inner(&self, event: Event) {
        let message = Message::new_event(event);
        if let Some(int) = &self.shared_resources {
            if let Err(err) = int.com_pipe.sender().send_message_raw(&message)
                && err.kind() != ErrorKind::ConnectionAborted
                && err.kind() != ErrorKind::ConnectionReset
            {
                // TODO: Maybe this is fatal?
                elog!(
                    "Failed to send an event {message:#?} to the client err: {err:?}, ignoring..."
                )
            }
        }
    }

    /// Sends an event to the client that owns this window.
    pub fn send_event(&self, event: WindowEvent, win_id: WinID) {
        self.send_event_inner(Event::new(event, win_id))
    }

    /// Creates a new Window from a given BMP Image
    pub fn new_from_bmp(
        name: Name,
        icon: Option<IconID>,
        pos_x: isize,
        pos_y: isize,
        image: BMPImage,
    ) -> Window {
        Self::new_from_pixels(
            name,
            icon,
            pos_x,
            pos_y,
            image.width() as usize,
            image.height() as usize,
            image
                .pixels()
                .map(|c| Pixel::rgb(c.red(), c.green(), c.blue()).with_alpha(c.alpha())),
        )
    }

    /// Creates a new Window and fills it with `fill_pixels`
    pub fn new_from_pixels(
        name: Name,
        icon: Option<IconID>,
        x: isize,
        y: isize,
        width: usize,
        height: usize,
        fill_pixels: impl ExactSizeIterator + Iterator<Item = Pixel>,
    ) -> Window {
        let mut pixels = vec![Pixel::NONE; width * height];

        let fill_pixels = fill_pixels.enumerate();
        for (i, pi) in fill_pixels {
            pixels[i] = pi;
        }

        let rect = TransformRect::new(Point::new(x, y), Rect::new(width, height));

        Window::new_inner(
            name,
            icon,
            rect,
            pixels.into_boxed_slice(),
            WindowFlags::empty(),
            None,
        )
    }

    /// Creates a new Window and fills it repeatedly with a given `pixel`
    pub fn new_filled_with(
        name: Name,
        icon: Option<IconID>,
        x: isize,
        y: isize,
        width: usize,
        height: usize,
        pixel: Pixel,
        flags: WindowFlags,
        shared: Option<(Arc<SharedObject>, Arc<ClientComPipe>)>,
    ) -> Option<Self> {
        let bounds_rect;
        let pixels;
        let shared_resources;
        let anchor;

        if let Some((shm_object, com_pipe)) = shared {
            let win_bounds = Rect::new(width, height);
            let decorate = !flags.contains(WindowFlags::NO_DECORATIONS);
            let (sh, r_pixels, r_bounds, r_anchor) =
                SharedWindow::create(win_bounds, shm_object, com_pipe, decorate, pixel)?;
            pixels = r_pixels;
            shared_resources = Some(sh);
            bounds_rect = r_bounds;
            anchor = r_anchor;
        } else {
            shared_resources = None;
            bounds_rect = Rect::new(width, height);
            anchor = Point::new(0, 0);
            pixels = vec![pixel; width * height].into_boxed_slice();
        }

        Some(Window::new_inner(
            name,
            icon,
            TransformRect::new(Point::new(x, y) + anchor, bounds_rect),
            pixels,
            flags,
            shared_resources,
        ))
    }

    fn new_inner(
        name: Name,
        icon: Option<IconID>,
        rect: TransformRect,
        pixels: Box<[Pixel]>,
        flags: WindowFlags,
        shared_resources: Option<SharedWindow>,
    ) -> Self {
        Self {
            rect,
            pixels,
            icon,
            name,
            shared_resources,
            status: WindowStatus::empty(),
            flags,
        }
    }

    /// Draws the window from intersection point without syncing the results to the real framebuffer.
    ///
    /// [`fb.sync_pixels_rect`] must be called afterwards on the area the window is in.
    fn fix(&mut self, fb: &mut Framebuffer, damages: &[DamageRegion]) {
        let pixels = &mut self.pixels;

        let win_point = damages
            .iter()
            .filter_map(|d| d.overlaps_with(&self.rect))
            .sum();
        if win_point != IntersectionPoint::none() {
            self.rect.draw_at(fb, win_point, pixels)
        }
    }

    fn get_whole_damage(&self) -> DamageRegion {
        self.rect.get_whole_damage()
    }

    fn damage_whole(&mut self) {
        self.rect
            .update_damage(WindowDamageReason::Whole(self.get_whole_damage()));
    }

    fn damage_within(&mut self, x: usize, y: usize, width: usize, height: usize) {
        self.rect.damage_within(x, y, width, height);
    }
}

const MAX_WINDOW_ID: usize = 1024 /* TODO: more windows? */;
/// A window ID
pub type WinID = u16;

/// The type of the Window, defines the ordering which a Window may come over another, for example the cursor uses [`WindowKind::Overlay`]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowKind {
    /// Always displayed above all other windows
    Overlay,
    /// Normal ordering
    Normal,
    /// Background Window, always displayed below all other windows
    Background,
    Cursor,
}

pub struct Windows {
    windows: FxHashMap<WinID, (Window, WindowKind)>,
    /// The cursor always comes on top of other windows, only one cursor may exist
    cursor: Option<WinID>,
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
    awaiting_fix: Vec<Waker>,
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
            cursor: None,
            awaiting_fix: Vec::new(),
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

        let width = size_of_val(&self.window_ids[0]) * 8;
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
            if let Some(toke) = win.rect.take_damage() {
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
                region.x(),
                region.y(),
                region.width(),
                region.height(),
                BG_PIXEL,
            );
        }

        // Fixes all the damages caused on a window if any
        macro_rules! fix_window {
            ($win: expr) => {{
                let win = $win;
                win.fix(&mut fb, &damage);
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

        // Cursor rules all
        if let Some(ref win_id) = self.cursor {
            let (window, _) = self
                .windows
                .get_mut(win_id)
                .expect("Cursor window doesn't exist");
            fix_window!(window);
        }

        for r in damage.drain(..) {
            fb.sync_pixels_rect(r.x(), r.y(), r.width(), r.height());
        }

        SHOULD_REDRAW.store(false, Ordering::Release);
        for awaiting in self.awaiting_fix.drain(..) {
            awaiting.wake();
        }
    }

    /// Adds `x` to window with the ID  `win_id` x position and `y` to the window with the ID `win_id`'s Y position
    ///
    /// Returns the new position if the Window ID exist
    pub fn add_cord(&mut self, win_id: WinID, x: i32, y: i32) -> Option<(isize, isize)> {
        let (win, _) = self.windows.get_mut(&win_id)?;

        let (new_x, new_y, redraw) = win.add_cord(x, y);

        if redraw {
            self.signal_redraw();
        }
        Some((new_x, new_y))
    }

    /// Adds a window and organizes it depending on `kind` (see [`WindowKind`]).
    /// Reposititons the window to fit most of the screen.
    pub fn add_window(
        &mut self,
        mut window: Window,
        kind: WindowKind,
        can_relocate: bool,
    ) -> Option<WinID> {
        let screen_width = FB_INFO.width as isize;
        let screen_height = FB_INFO.height as isize;

        if can_relocate {
            let mut new_x = 0;
            let mut new_y = 0;

            if !(window.max_x() > screen_width) {
                new_x = (screen_width - window.width() as isize) / 2;
            }

            if !(window.max_y() > screen_height) {
                new_y = (screen_height - window.height() as isize) / 2;
            }

            window.set_position_in_place(new_x, new_y);
        }

        // Damage the window
        window.damage_whole();
        let id = self.add_id()?;

        if window.flags.contains(WindowFlags::GLOBAL) {
            self.broadcast_global_event(WindowEvent::GlobalWindowAttached(WindowAttached::new(
                id,
                window.x() as i32,
                window.y() as i32,
                window.flags(),
            )));
        }

        self.windows.insert(id, (window, kind));

        match kind {
            WindowKind::Normal => {
                self.set_focused(id);
            }
            WindowKind::Overlay => {
                self.overlay_windows.insert(id);
            }
            WindowKind::Background => {
                self.background_windows.insert(id);
            }
            WindowKind::Cursor => self.cursor = Some(id),
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

        window.status.insert(WindowStatus::FOCUSED);
        let event = WindowFocusChanged::new(true);

        window.send_event(WindowEvent::WindowFocusChanged(event), win_id);
        window.damage_whole();

        let window_kind = *window_kind;
        if window.flags.contains(WindowFlags::GLOBAL) {
            self.broadcast_global_event(WindowEvent::GlobalWindowFocusChanged(event, win_id));
        }

        self.unfocus_current();
        self.focused_window = Some(win_id);

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
            WindowKind::Cursor => unreachable!("The cursor's window shall not be focused on"),
        };

        self.signal_redraw();
        true
    }

    /// Unfocus the currently focused window.
    pub fn unfocus_current(&mut self) {
        if let Some(win_id) = self.focused_window.take() {
            if let Some((win, _)) = self.windows.get_mut(&win_id) {
                let focus_event = WindowFocusChanged::new(false);
                win.send_event(WindowEvent::WindowFocusChanged(focus_event), win_id);
                win.damage_whole();
                win.status.remove(WindowStatus::FOCUSED);

                if win.flags.contains(WindowFlags::GLOBAL) {
                    self.broadcast_global_event(WindowEvent::GlobalWindowFocusChanged(
                        focus_event,
                        win_id,
                    ));
                }
                self.signal_redraw();
            }
        }
    }

    /// Returns the ID of the top-most window that is in contact with the given position and size if any, and also the contact point
    pub fn window_in_contact(
        &self,
        pos_x: isize,
        pos_y: isize,
        width: usize,
        height: usize,
    ) -> Option<(WinID, WindowKind, IntersectionPoint)> {
        let region = DamageRegion {
            position: Point::new(pos_x, pos_y),
            rect: Rect::new(width, height),
        };

        // FIXME: handle all kinds of windows after adding event susbscribing.
        let results = self.overlay_windows.iter().rev().find_map(|win_id| {
            let (win, _) = self
                .windows
                .get(win_id)
                .expect("Window wasn't removed from the Z-ordering when it's ID was deallocated");
            region
                .overlaps_with(&win.rect)
                .map(|point| (*win_id, WindowKind::Overlay, point))
        });
        results.or_else(|| {
            self.normal_windows.iter().rev().find_map(|win_id| {
                let (win, _) = self.windows.get(win_id).expect(
                    "Window wasn't removed from the Z-ordering when it's ID was deallocated",
                );
                region
                    .overlaps_with(&win.rect)
                    .map(|point| (*win_id, WindowKind::Normal, point))
            })
        })
    }

    /// Returns the ID of the focused Window
    pub const fn focused_window(&self) -> Option<WinID> {
        self.focused_window
    }

    pub fn damage_window(
        &mut self,
        win_id: WinID,
        mut x: usize,
        mut y: usize,
        mut width: usize,
        mut height: usize,
        waker: Option<&Waker>,
    ) -> Result<(), ()> {
        let (win, _) = self.windows.get_mut(&win_id).ok_or(())?;

        if let Some(sh) = win.shared_resources.as_ref() {
            if let Some((loc, bounds)) = sh.sync_pixels_to(&mut win.pixels, x, y, width, height) {
                x = loc.x();
                y = loc.y();
                width = bounds.width();
                height = bounds.height();
            } else {
                return Ok(());
            }
        }
        win.damage_within(x, y, width, height);

        if let Some(waker) = waker {
            self.awaiting_fix.push(waker.clone());
        }
        self.signal_redraw();

        Ok(())
    }

    pub fn send_event(&mut self, win_id: WinID, event: WindowEvent) -> Result<(), ()> {
        let (win, _) = self.windows.get_mut(&win_id).ok_or(())?;
        win.send_event(event, win_id);
        Ok(())
    }

    /// Broadcasts a global event to all the windows that are subscribed to that event.
    pub fn broadcast_global_event(&mut self, event: WindowEvent) {
        for (win_id, (win, _)) in self.windows.iter() {
            win.send_event_inner(Event::new(event, *win_id));
        }
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
            WindowKind::Cursor => {
                assert!(
                    self.cursor.take().is_some(),
                    "Window has not placed as the cursor"
                )
            }
        }

        assert!(
            self.remove_id(win_id),
            "Unexpected behavior, ID should have been removed successfully"
        );

        if window.flags.contains(WindowFlags::GLOBAL) {
            self.broadcast_global_event(WindowEvent::GlobalWindowDeatached(WindowDeatached::new(
                win_id,
            )));
        }

        dlog!("Window removed");
        self.signal_redraw();
        Ok(())
    }

    pub fn get_window(&self, id: WinID) -> Option<&Window> {
        self.windows.get(&id).map(|(win, _)| win)
    }
}

pub static WINDOWS: Mutex<Windows> = Mutex::new(Windows::new());

/// Adds a window with `kind` kind, returns the ID of the window.
/// Repositions the window to fit most of it on the screen.
pub fn add_window(window: Window, kind: WindowKind, can_relocate: bool) -> Option<WinID> {
    WINDOWS
        .lock()
        .expect("Failed to acquire lock on Windows while adding a Window")
        .add_window(window, kind, can_relocate)
}

pub fn damage_window(
    win_id: WinID,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    waker: Option<&Waker>,
) -> Result<(), ()> {
    WINDOWS
        .lock()
        .expect("Failed to acquire lock on Windows while damaging a Window")
        .damage_window(win_id, x, y, width, height, waker)
}

struct DamageWindowFuture {
    win_id: WinID,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    results: Option<()>,
}

impl Future for DamageWindowFuture {
    type Output = Result<(), ()>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        if let Some(()) = self.results {
            return std::task::Poll::Ready(Ok(()));
        } else {
            let waker = cx.waker();
            let results = damage_window(
                self.win_id,
                self.x,
                self.y,
                self.width,
                self.height,
                Some(waker),
            );

            match results {
                Err(e) => std::task::Poll::Ready(Err(e)),
                Ok(o) => {
                    self.results = Some(o);
                    // Wait until task is awaken (FIXME: hopefully, this can be abused)
                    std::task::Poll::Pending
                }
            }
        }
    }
}

/// Asynchoursly damages a window
///
/// same as [`damage_window`] but async.
///
/// Awaits window to be fixed.
pub async fn damage_window_async(
    win_id: WinID,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
) -> Result<(), ()> {
    DamageWindowFuture {
        width,
        height,
        x,
        y,
        win_id,
        results: None,
    }
    .await
}

/// Whether we should redraw the screen
static SHOULD_REDRAW: AtomicBool = AtomicBool::new(false);

/// Returns true if you should call `redraw_screen`
fn should_redraw() -> bool {
    SHOULD_REDRAW.load(Ordering::Acquire)
}

/// Better called from a single thread at a time
/// Redraws changed areas of the screen in case we need to
pub fn redraw() -> bool {
    if should_redraw() {
        WINDOWS
            .lock()
            .expect("Failed to acquire lock on Windows while redrawing")
            .damage_redraw();
        true
    } else {
        false
    }
}

/// Set the window with the id `win_id` as focused,
/// handles everything including sending events and damage, and reordering the Z-list.
pub fn focus(win_id: WinID) -> bool {
    WINDOWS
        .lock()
        .expect("Failed to acquire lock on Windows while focusing window")
        .set_focused(win_id)
}
