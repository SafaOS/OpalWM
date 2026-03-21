use cosmic_text::Metrics;
use libopal::{
    WindowEvent,
    defs::{HeldMouseButtons, WindowFlags, WindowID},
    window::Pixel,
};

use crate::{
    AppCtx, EventCtx, Padding,
    render::{BoundingConstraints, BoundingRect, CanvasCache, PaintBrush, Point},
    shards::{Button, Label, LayoutCtx, Shard, ShardNode, ShardsExt, Stack},
};

#[derive(Debug, Clone)]
pub struct WindowBuilder<'a> {
    width: u32,
    height: u32,
    bg: PaintBrush,
    title: &'a str,
}

const TITLE_BAR_HEIGHT: u32 = 26;
impl<'a> WindowBuilder<'a> {
    /// Constructs a WindowBuilder with width and height.
    pub const fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            bg: PaintBrush::Color(super::Color::WHITE),
            title: "",
        }
    }

    /// Sets the window title.
    pub const fn title(mut self, title: &'a str) -> Self {
        self.title = title;
        self
    }

    #[inline]
    pub fn background(mut self, background: impl Into<PaintBrush>) -> Self {
        self.bg = background.into();
        self
    }

    /// Builds the window given a root [`Shard`].
    pub fn build<Root, Ctx: AppCtx>(self, root: Root) -> Window<Ctx>
    where
        Root: Shard<Ctx> + 'static,
    {
        let mut height = self.height;
        if !self.title.is_empty() {
            height += TITLE_BAR_HEIGHT;
        }

        Window::new_with_root(
            self.title,
            libopal::window::Window::create(
                self.title,
                WindowFlags::GLOBAL,
                self.width,
                height,
                None,
                None,
            ),
            self.bg,
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

impl<Ctx: AppCtx + 'static> Window<Ctx> {
    /// Returns the Window ID of this Window.
    #[inline]
    pub fn win_id(&self) -> WindowID {
        self.inner.id()
    }

    fn new_with_root<Root: Shard<Ctx> + 'static>(
        title: &str,
        inner: libopal::window::Window,
        bg: PaintBrush,
        root: Root,
    ) -> Self {
        let used_root;
        if title.is_empty() {
            used_root = ShardNode::new(
                Stack::row()
                    .with(root)
                    .with_padding(Padding::none())
                    .fix_size(inner.width() as f32, inner.height() as f32)
                    .background(bg),
            );
        } else {
            use super::Color;
            use super::shards::AxisAlign;
            let title_height = TITLE_BAR_HEIGHT as f32;
            let btn_width = title_height;
            let win_width = inner.width() as f32;

            let btn_flex = (3. * btn_width) / win_width;
            let spacer2_flex = 1.0 - btn_flex;

            used_root = ShardNode::new(
                Stack::<Ctx>::row()
                    .with_padding(Padding::none())
                    .with(
                        Stack::<Ctx>::column()
                            .with_padding(Padding::none())
                            .with_spacer(1.)
                            .with_flex(
                                Label::from_str(title)
                                    .with_metrics(Metrics::relative(13., 1.))
                                    .center_text(),
                                1.,
                            )
                            .with_spacer(spacer2_flex)
                            .with_flex(
                                Button::new(Label::from_str("X"))
                                    .with_paint(Color::NONE)
                                    .on_click(|_, _, _| std::process::exit(0)),
                                btn_flex,
                            )
                            .align(AxisAlign::Center)
                            .background(Color::rgb(0xFD, 0xB0, 0xC0))
                            .fix_height(title_height)
                            .fix_width(win_width),
                    )
                    .with(root)
                    .fix_size(inner.width() as f32, inner.height() as f32)
                    .background(bg),
            );
        }
        Self {
            root: used_root,
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
        let win_width = self.inner.width();
        assert_eq!(pix_width, win_width);

        let win_height = self.inner.height();
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

        let damage_h = damage_h.min(win_height - damage_y);
        let damage_w = damage_w.min(win_width - damage_x);
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
        self.root.layout_if_none(&mut LayoutCtx {
            font_system: self.cache.font_system(),
            constraints,
        });
        self.root.route_message(Point::default(), state, msg);
    }

    /// Broadcast's an event to the window's elements.
    pub fn broadcast_event(&mut self, app_state: &mut Ctx, event: WindowEvent) {
        let constraints = self.constraints();
        self.root.layout_if_none(&mut LayoutCtx {
            font_system: self.cache.font_system(),
            constraints,
        });
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
        self.root.layout(&mut LayoutCtx {
            font_system: self.cache.font_system(),
            constraints,
        });

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
