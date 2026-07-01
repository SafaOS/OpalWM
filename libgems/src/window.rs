use cosmic_text::Metrics;
use libopal::{
    WindowEvent,
    defs::{HeldMouseButtons, WindowFlags, WindowID},
    window::Pixel,
};

use crate::{
    Data, EventCtx, Padding,
    render::{BoundingConstraints, BoundingRect, CanvasCache, PaintBrush, Point},
    shards::{Button, Label, LayoutCtx, Shard, ShardNode, ShardsExt, Stack, lifecycle::LifeCycle},
    theme,
};

#[derive(Debug, Clone)]
pub struct WindowDesc<'a, Root: 'static> {
    config: WindowBuilder<'a>,
    root: Root,
}

impl<'a, Root> WindowDesc<'a, Root> {
    pub(crate) fn init<State, Message>(
        self,
        app: &mut Data<State, Message>,
    ) -> Window<State, Message>
    where
        Root: Shard<State, Message>,
    {
        let config = self.config;

        let mut height = config.height;
        if !config.title.is_empty() {
            height += TITLE_BAR_HEIGHT;
        }

        Window::new_with_root(
            config.title,
            libopal::window::Window::create(
                config.title,
                WindowFlags::GLOBAL,
                config.width,
                height,
                None,
                None,
            ),
            config
                .bg
                .or_else(|| {
                    app.env()
                        .try_get(theme::BACKGROUND_COLOR)
                        .ok()
                        .map(|c| c.into())
                })
                .unwrap_or(PaintBrush::Color(super::Color::WHITE)),
            self.root,
            config.use_all_space,
            app,
        )
    }
}

#[derive(Debug, Clone)]
pub struct WindowBuilder<'a> {
    width: u32,
    height: u32,
    x: Option<i32>,
    y: Option<i32>,
    use_all_space: bool,
    bg: Option<PaintBrush>,
    title: &'a str,
}

const TITLE_BAR_HEIGHT: u32 = 26;
impl<'a> WindowBuilder<'a> {
    /// Constructs a WindowBuilder with width and height.
    pub const fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            x: None,
            y: None,
            use_all_space: true,
            bg: None,
            title: "",
        }
    }

    /// Sets the window title.
    pub const fn title(mut self, title: &'a str) -> Self {
        self.title = title;
        self
    }

    /// Whether or not the window's root should fill the entire window
    /// DEFAULT = true.
    pub const fn use_all_space(mut self, u: bool) -> Self {
        self.use_all_space = u;
        self
    }

    #[inline]
    pub fn background(mut self, background: impl Into<PaintBrush>) -> Self {
        self.bg = Some(background.into());
        self
    }

    pub fn x(mut self, x: Option<i32>) -> Self {
        self.x = x;
        self
    }

    pub fn y(mut self, y: Option<i32>) -> Self {
        self.y = y;
        self
    }

    /// Builds the window given a root [`Shard`].
    pub fn build<State: 'static, Message: 'static, Root>(self, root: Root) -> WindowDesc<'a, Root>
    where
        Root: Shard<State, Message> + 'static,
    {
        WindowDesc { config: self, root }
    }
}

pub struct Window<State: 'static = (), Message: 'static = ()> {
    root: ShardNode<State, Message>,
    cache: CanvasCache,
    inner: libopal::window::Window,
    mouse_position: Option<Point>,
    mouse_button_state: HeldMouseButtons,
}

impl<State, Message> Window<State, Message> {
    pub fn set_title(&mut self, title: impl AsRef<str>, data: &Data<State, Message>) {
        self.root.route_lifecycle(
            &LifeCycle::WindowMetaChanged {
                title: title.as_ref(),
            },
            data,
        );

        self.try_redraw(data);
    }

    pub const fn inner_mut(&mut self) -> &mut libopal::window::Window {
        &mut self.inner
    }

    pub fn raw_pixels(&mut self) -> &mut [Pixel] {
        self.inner.pixels_mut()
    }

    /// A lightweight function to return a slice of the unoccupied pixels of the window and their area (i.e no shards currently)
    ///
    /// Shouldn't be used with window content that could potinetally grow (TODO add RawTexture's?).
    ///
    /// E.g you can use that to implement over this library's title bar.
    pub fn unoccupied_pixels(&mut self) -> Option<(&mut [Pixel], u32, BoundingRect)> {
        let constraints = self.constraints();
        let (root_layout, _) = self.root.layout(&mut LayoutCtx {
            font_system: self.cache.font_system(),
            constraints,
        });

        let occupied_height = root_layout.full_bounds().height().ceil() as u32;
        let window_height = self.inner.height();
        let window_width = self.inner.width();
        let unoccupied_height = window_height - occupied_height;

        if unoccupied_height == 0 {
            return None;
        }

        let pixels = &mut self.raw_pixels()[(occupied_height * window_width) as usize..];
        Some((
            pixels,
            occupied_height,
            BoundingRect::new(window_width as f32, unoccupied_height as f32),
        ))
    }

    /// Returns the Window ID of this Window.
    #[inline]
    pub fn win_id(&self) -> WindowID {
        self.inner.id()
    }

    fn new_with_root<Root: Shard<State, Message> + 'static>(
        title: &str,
        inner: libopal::window::Window,
        bg: PaintBrush,
        root: Root,
        fill_with_root: bool,
        data: &mut Data<State, Message>,
    ) -> Self {
        let used_root;
        if title.is_empty() {
            let mut it = Stack::row()
                .with(root)
                .with_padding(Padding::none())
                .fix_width(inner.width() as f32);
            if fill_with_root {
                it = it.fix_height(inner.height() as f32);
            }
            used_root = ShardNode::new(it.background(bg));
        } else {
            use super::Color;
            use super::shards::AxisAlign;
            let title_height = TITLE_BAR_HEIGHT as f32;
            let btn_width = title_height;
            let win_width = inner.width() as f32;

            let btn_flex = (3. * btn_width) / win_width;
            let spacer2_flex = 1.0 - btn_flex;

            let mut it = Stack::<State, Message>::row()
                .with_padding(Padding::none())
                .with(
                    Stack::<State, Message>::column()
                        .with_padding(Padding::none())
                        .with_spacer(1.)
                        .with_flex(
                            Label::from_str(title)
                                .with_metrics(Metrics::relative(13., 1.))
                                .center_text()
                                .on_lifecycle(|_, l, this| match l {
                                    LifeCycle::WindowMetaChanged { title, .. } => {
                                        this.set_text(title);
                                    }
                                    LifeCycle::Init { window_title, .. } => {
                                        this.set_text(window_title);
                                    }
                                    _ => {}
                                }),
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
                        .background(data.env().get(theme::ACCENT_COLOR))
                        .fix_height(title_height)
                        .fix_width(win_width),
                )
                .with(root)
                .fix_width(inner.width() as f32);
            if fill_with_root {
                it = it.fix_height(inner.height() as f32);
            }
            used_root = ShardNode::new(it.background(bg));
        }
        let mut this = Self {
            root: used_root,
            cache: CanvasCache::new(),
            mouse_position: None,
            mouse_button_state: HeldMouseButtons::empty(),
            inner,
        };
        this.root.route_lifecycle(
            &crate::shards::lifecycle::LifeCycle::Init {
                window_title: title,
            },
            data,
        );
        this
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

        let damage_h = damage_h
            .min(win_height - damage_y)
            .min(pix_height - damage_y);
        let damage_w = damage_w.min(win_width - damage_x).min(pix_width - damage_x);

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
    pub fn broadcast_message(&mut self, state: &mut Data<State, Message>, msg: &Message) {
        let constraints = self.constraints();
        self.root.layout_if_none(&mut LayoutCtx {
            font_system: self.cache.font_system(),
            constraints,
        });
        self.root.route_message(Point::default(), state, msg);
    }

    pub fn update_ctx(&mut self, app_state: &Data<State, Message>) {
        let constraints = self.constraints();
        self.root.layout_if_none(&mut LayoutCtx {
            font_system: self.cache.font_system(),
            constraints,
        });
        self.root.on_ctx_update(app_state);
    }

    /// Broadcast's an event to the window's elements.
    pub fn broadcast_event(&mut self, app_state: &mut Data<State, Message>, event: WindowEvent) {
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
    pub fn redraw(&mut self, data: &Data<State, Message>) {
        if self.root.should_relayout() {
            let constraints = self.constraints();
            self.root.layout(&mut LayoutCtx {
                font_system: self.cache.font_system(),
                constraints,
            });
        }

        match self.root.render_as_root(&mut self.cache, data) {
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
    pub fn try_redraw(&mut self, data: &Data<State, Message>) -> bool {
        if self.dirty() {
            self.redraw(data);
            true
        } else {
            false
        }
    }
}
