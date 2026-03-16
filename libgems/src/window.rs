use libopal::{
    WindowEvent,
    defs::{HeldMouseButtons, WindowFlags},
    window::Pixel,
};

use crate::{
    AppCtx, EventCtx,
    render::{BoundingConstraints, BoundingRect, CanvasCache, Point},
    shards::{LayoutCtx, Shard, ShardNode},
};

#[derive(Debug, Clone)]
pub struct WindowBuilder<'a> {
    width: u32,
    height: u32,

    title: &'a str,
}

impl<'a> WindowBuilder<'a> {
    /// Constructs a WindowBuilder with width and height.
    pub const fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            title: "",
        }
    }

    /// Sets the window title.
    pub const fn title(mut self, title: &'a str) -> Self {
        self.title = title;
        self
    }

    /// Builds the window given a root [`Shard`].
    pub fn build<Root, Ctx: AppCtx>(self, root: Root) -> Window<Ctx>
    where
        Root: Shard<Ctx> + 'static,
    {
        Window::new_with_root(
            libopal::window::Window::create(
                self.title,
                WindowFlags::GLOBAL,
                self.width,
                self.height,
                None,
                None,
            ),
            root,
        )
    }
}

pub struct Window<Ctx: AppCtx> {
    root: ShardNode<Ctx>,
    // root_layout: Option<ShardLayout>,
    // root_canvas: TinySkiaCanvas,
    cache: CanvasCache,
    inner: libopal::window::Window,
    mouse_position: Option<Point>,
    mouse_button_state: HeldMouseButtons,
}

impl<Ctx: AppCtx> Window<Ctx> {
    fn new_with_root<Root: Shard<Ctx> + 'static>(
        inner: libopal::window::Window,
        root: Root,
    ) -> Self {
        Self {
            root: ShardNode::new(root),
            cache: CanvasCache::new(),
            mouse_position: None,
            mouse_button_state: HeldMouseButtons::empty(),
            inner,
        }
    }

    /// Damages the area at `at` with the bounds `area`.
    ///
    /// Damaging is the act of requesting redraw from the WM.
    pub fn damage(&mut self, at: Point, area: BoundingRect) {
        let Some(canvas) = self.root.shard.cache_mut() else {
            return;
        };
        let pixmap = &mut canvas.pixmap;

        let damage_x = at.x().ceil() as i32;
        let damage_y = at.y().ceil() as i32;
        let damage_w = area.width().ceil() as i32;
        let damage_h = area.height().ceil() as i32;
        if damage_h == 0 || damage_w == 0 {
            return;
        }

        assert!(
            damage_x >= 0 && damage_y >= 0 && damage_w.is_positive() && damage_h.is_positive(),
            "Damage negative: {at:?}, area: {area:#?} => ({damage_x}, {damage_y}) ({damage_w}, {damage_h})"
        );

        let pix_width = pixmap.width();
        let pix_height = pixmap.height();

        let real_pixels = pixmap.pixels_mut();
        let win_pixels = self.inner.pixels_mut();

        let damage_y = damage_y as u32;
        let damage_h = damage_h as u32;
        let damage_x = damage_x as u32;
        let damage_w = damage_w as u32;

        if damage_y >= pix_height || damage_x >= pix_width {
            return;
        }

        let damage_h = damage_h.min(pix_height - damage_y);
        let damage_w = damage_w.min(pix_width - damage_x);
        for row in damage_y..(damage_y + damage_h) {
            let start_index = ((row * pix_width) + damage_x) as usize;
            let end_index = (start_index + damage_w as usize) as usize;

            let real_pixs = &mut real_pixels[start_index..end_index];
            let win_pixs = &mut win_pixels[start_index..end_index];
            for (r_pix, win_pix) in real_pixs.iter_mut().zip(win_pixs.iter_mut()) {
                *win_pix = Pixel::rgba(r_pix.red(), r_pix.green(), r_pix.blue(), r_pix.alpha());
            }
        }

        self.inner.redraw(damage_x, damage_y, damage_w, damage_h);
    }

    /// Returns the bounds of the window (width and height around window).
    pub fn bounds(&self) -> BoundingRect {
        BoundingRect::new(self.inner.width() as f32, self.inner.height() as f32)
    }

    fn constraints(&self) -> BoundingConstraints {
        BoundingConstraints::from_max(self.bounds())
    }

    /// Broadcast's a message to the window's elements.
    pub fn broadcast_message(&mut self, state: &mut Ctx, msg: &Ctx::Message) {
        let constraints = self.constraints();
        self.root.try_layout(
            &mut LayoutCtx {
                font_system: self.cache.font_system(),
                constraints,
            },
            |_| {},
        );
        self.root.route_message(Point::default(), state, msg);
    }

    /// Broadcast's an event to the window's elements.
    pub fn broadcast_event(&mut self, app_state: &mut Ctx, event: WindowEvent) {
        let constraints = self.constraints();
        self.root.try_layout(
            &mut LayoutCtx {
                font_system: self.cache.font_system(),
                constraints,
            },
            |_| {},
        );
        EventCtx::with_event(
            &mut self.mouse_button_state,
            &mut self.mouse_position,
            &event,
            |eve_origin, event| {
                self.root
                    .route_event(Point::default(), eve_origin, &event, app_state);
            },
        );
        self.root.on_ctx_update(app_state);
    }

    /// Returns wetheher or not the window has to be redrawn using [`Self::redraw`].
    #[inline(always)]
    pub fn dirty(&self) -> bool {
        self.root.is_dirty()
    }

    /// Re-renders the window even if it isn't dirty, may be costy.
    pub fn redraw(&mut self) {
        let constraints = self.constraints();
        self.root.layout(
            &mut LayoutCtx {
                font_system: self.cache.font_system(),
                constraints,
            },
            |_| {},
        );

        match self.root.render_as_root(&mut self.cache) {
            None => {
                self.damage(Point::new(0., 0.), self.bounds());
            }
            Some((point, area)) => {
                self.damage(point, area);
            }
        }
    }

    /// Attempts to render window if it is [`Self::dirty`], returning wetheher or not it was changed.
    #[inline(always)]
    pub fn try_redraw(&mut self) -> bool {
        if self.dirty() {
            self.redraw();
            true
        } else {
            false
        }
    }
}
