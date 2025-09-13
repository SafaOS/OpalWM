pub mod canvas;
pub mod element;
/// A basic wrapper around cosmic_text, you can use custom elements with cosmic_text for more advanced stuff.
pub mod text;

pub use libopal;
pub use opal_img as image;

use libopal::{
    DequeuedEvents,
    window::{Pixel, Window, WindowFlags},
};

use crate::{
    canvas::DrawingCanvas,
    element::{
        Element,
        button::{Button, ButtonStyle},
        container::{Container, ContainerLayout, ContainerStyles, GridLayout},
        text_box::{TextBox, TextBoxStyles},
    },
};

pub use ::cosmic_text;

pub const BORDER_COLOR0: Pixel = Pixel::from_rgb(0xFD, 0xB0, 0xC0);
pub const DARK_BG_COLOR0: Pixel = Pixel::from_rgb_with_alpha(0, 0, 0, 0x80);
pub const DARK_BG_COLOR1: Pixel = Pixel::from_rgb_with_alpha(0, 0, 0, 0xFF);
pub const LIGHT_BG_COLOR0: Pixel = Pixel::from_rgb_with_alpha(0xFB, 0xF1, 0xC7, 0xFF);

/// A Gem is the app state that [`App`] contains, it can be initialized into an app using [`Gem::init`].
pub trait Gem: Sized + 'static {
    /// From a given config turns the gem into an initialized app.
    fn init(self, config: GemConfig) -> App<Self> {
        config.build_app(self)
    }
}

#[derive(Debug, Clone, Copy)]
/// Configuration to build a [`Gem`] into an [`App`].
pub struct GemConfig<'a> {
    title: &'a str,
    bg_color: Pixel,
    border_color: Option<Pixel>,
    win_flags: WindowFlags,
    width: u32,
    height: u32,
    custom_position: Option<(i32, i32)>,
    body_styles: ContainerStyles,
}

impl<'a> GemConfig<'a> {
    /// Constructs a new [`GemBuilder`] with default values.
    pub const fn new(title: &'a str, width: u32, height: u32) -> Self {
        Self {
            title: title,
            bg_color: LIGHT_BG_COLOR0,
            border_color: Some(BORDER_COLOR0),
            win_flags: WindowFlags::empty(),
            body_styles: ContainerStyles::new(),
            width,
            height,
            custom_position: None,
        }
    }

    /// Sets the border color of the App, if color is None, no border will be drawn.
    pub const fn with_border(mut self, color: Option<Pixel>) -> Self {
        self.border_color = color;
        self
    }

    /// Sets the title of the App.
    pub const fn with_title(mut self, title: &'a str) -> Self {
        self.title = title;
        self
    }

    /// Sets the background color of the App.
    pub const fn with_bg_color(mut self, color: Pixel) -> Self {
        self.bg_color = color;
        self
    }

    /// Sets the window flags of the App.
    pub const fn with_win_flags(mut self, flags: WindowFlags) -> Self {
        self.win_flags = flags;
        self
    }

    /// Sets the width of the App.
    pub const fn with_width(mut self, width: u32) -> Self {
        self.width = width;
        self
    }

    /// Sets the height of the App.
    pub const fn with_height(mut self, height: u32) -> Self {
        self.height = height;
        self
    }

    pub const fn with_position(mut self, x: i32, y: i32) -> Self {
        self.custom_position = Some((x, y));
        self
    }

    /// Sets the styles of the App's body.
    pub const fn with_body_styles(mut self, styles: ContainerStyles) -> Self {
        self.body_styles = styles;
        self
    }

    fn build_container<G: Gem>(self) -> RootContainer<G> {
        match self.border_color {
            Some(color) => RootContainer::new_with_border(
                self.win_flags,
                self.width,
                self.height,
                self.custom_position,
                self.title,
                self.bg_color,
                color,
                self.body_styles,
            ),
            None => RootContainer::new_without_border(
                self.win_flags,
                self.width,
                self.height,
                self.custom_position,
                self.bg_color,
                self.body_styles,
            ),
        }
    }

    /// Builds an App with the given Gem and configuration.
    pub fn build_app<G: Gem>(self, gem: G) -> App<G> {
        let container = self.build_container();
        App::new(container, gem)
    }
}

struct RootContainer<G: Gem> {
    root: Window,
    window_x: u32,
    window_y: u32,
    body: Container<Window, G>,
    title_bar: Option<(Container<Window, G>, u32)>,
    bg_color: Pixel,
    border_color: Pixel,
}

impl<G: Gem> RootContainer<G> {
    const CORNER_RADIUS: u32 = 8;
    const TITLE_HEIGHT: u32 = Self::DEFAULT_TITLE_FONT_SIZE + 10;
    const DEFAULT_TITLE_FONT_SIZE: u32 = 12;
    const TRANSPARENT: Pixel = Pixel::from_hex_argb(0);

    fn new_with_border(
        flags: WindowFlags,
        width: u32,
        height: u32,
        custom_position: Option<(i32, i32)>,
        title: &str,
        bg_color: Pixel,
        border_color: Pixel,
        styles: ContainerStyles,
    ) -> Self {
        let real_width = width + (Self::CORNER_RADIUS * 2);
        let real_height = height + Self::TITLE_HEIGHT + /* we draw 2 pixels after title */ 2 + /* border thickness */ 1;
        let window_x = Self::CORNER_RADIUS / 2;
        let window_y = Self::TITLE_HEIGHT + 2;

        let mut win = Window::create(flags, real_width, real_height, custom_position);

        win.draw_round_rect(
            0,
            0,
            real_width,
            real_height,
            Self::CORNER_RADIUS,
            |is_border, line_num| {
                if is_border {
                    border_color
                } else {
                    if line_num < Self::TITLE_HEIGHT {
                        border_color
                    } else {
                        bg_color
                    }
                }
            },
            None,
        );
        win.redraw(0, 0, real_width, real_height);

        let mut title_bar = Container::new(
            ContainerStyles::new()
                .with_layout(ContainerLayout::Grid(GridLayout::new()))
                .with_element_padding(0),
            width,
            Self::TITLE_HEIGHT,
        );
        let title_bar_y = (Self::TITLE_HEIGHT - Self::DEFAULT_TITLE_FONT_SIZE) / 2;

        let x_button_width = Self::DEFAULT_TITLE_FONT_SIZE + 10;

        let x_button_styles = ButtonStyle::new(x_button_width, Self::DEFAULT_TITLE_FONT_SIZE + 2)
            .with_font_height(Self::DEFAULT_TITLE_FONT_SIZE as f32)
            .with_normal_color(Self::TRANSPARENT)
            .with_hover_color(Self::TRANSPARENT)
            .with_border_color(None)
            .with_text_color(Pixel::BLACK)
            .with_hover_text_color(Pixel::from_rgb(0xFF, 0xFF, 0xFF));

        let mut x_button = Button::new("X", x_button_styles);
        x_button.on_click(|_, _| std::process::exit(0));

        let label_styles = TextBoxStyles::new(
            (width - x_button_styles.real_width()) as f32,
            (Self::TITLE_HEIGHT - title_bar_y) as f32,
        )
        .with_font_size(Self::DEFAULT_TITLE_FONT_SIZE as f32)
        .with_text_color(Pixel::BLACK);
        let label = TextBox::new(title, label_styles);
        _ = title_bar.add_element(Box::new(label));
        _ = title_bar.add_element(Box::new(x_button));

        Self {
            root: win,
            window_x,
            window_y,
            body: Container::new(styles, width, height),
            title_bar: Some((title_bar, title_bar_y)),
            bg_color,
            border_color,
        }
    }

    fn new_without_border(
        win_flags: WindowFlags,
        width: u32,
        height: u32,
        custom_position: Option<(i32, i32)>,
        bg_color: Pixel,
        styles: ContainerStyles,
    ) -> Self {
        let window_x = 0;
        let window_y = 0;

        let win = Window::create(win_flags, width, height, custom_position);

        Self {
            title_bar: None,
            root: win,
            window_x,
            window_y,
            body: Container::new(styles, width, height),
            bg_color,
            border_color: Pixel::from_hex_argb(0),
        }
    }
}

/// A container for a [`Gem`] and its associated [`Window`].
pub struct App<G: Gem> {
    cont: RootContainer<G>,
    gem: G,
}

impl<G: Gem> App<G> {
    fn new(container: RootContainer<G>, gem: G) -> Self {
        libopal::init();
        Self {
            cont: container,
            gem,
        }
    }

    /// Returns a mutable reference to the root container.
    pub fn body(&mut self) -> &mut Container<Window, G> {
        &mut self.cont.body
    }

    pub fn gem_mut(&mut self) -> &mut G {
        &mut self.gem
    }

    pub fn gem(&self) -> &G {
        &self.gem
    }

    /// Adds an element to the root container's body
    ///
    /// alias for `self.body().add_element(Box::new(element))`
    pub fn add_element<E: Element<Window, G>>(&mut self, element: E) -> usize {
        self.body().add_element(Box::new(element))
    }

    pub fn redraw(&mut self) {
        let mut handle_container =
            |container: &mut Container<Window, G>, start_x: u32, start_y: u32, bg_color: Pixel| {
                if container.needs_redraw() {
                    let end = container.draw(&mut self.cont.root, start_x, start_y, bg_color);

                    if let Some((end_x, end_y)) = end {
                        let width = end_x - start_x;
                        let height = end_y - start_y;

                        self.cont.root.redraw(start_x, start_y, width, height);
                    }
                }
            };

        if let Some((ref mut title_bar, title_bar_y)) = self.cont.title_bar {
            handle_container(
                title_bar,
                self.cont.window_x,
                title_bar_y,
                self.cont.border_color,
            );
        }

        handle_container(
            &mut self.cont.body,
            self.cont.window_x,
            self.cont.window_y,
            self.cont.bg_color,
        );
    }

    fn handle_events(&mut self, events: &DequeuedEvents) {
        for event in &**events {
            if let Some((ref mut title_bar, title_bar_y)) = self.cont.title_bar {
                title_bar.handle_event(&mut self.gem, *event, self.cont.window_x, title_bar_y);
            }

            self.cont.body.handle_event(
                &mut self.gem,
                *event,
                self.cont.window_x,
                self.cont.window_y,
            );
        }
    }

    /// Attempts to handle pending events if any or waits until there is an event to handle.
    ///
    /// the non-waiting equalivent is [`Self::try_handle_events`].
    pub fn handle_events_blocking(&mut self) -> DequeuedEvents {
        let events = libopal::dequeue_events_blocking().expect("Failed to wait for an event");
        self.handle_events(&events);
        events
    }

    /// Attempts to handle pending events if any, returning the dequeued events.
    ///
    /// The waiting equalivent would be [`Self::handle_events_blocking`],
    /// which is obviously better to use except if you really don't want to block.
    pub fn try_handle_events(&mut self) -> Option<DequeuedEvents> {
        let events = libopal::dequeue_events_non_blocking().expect("Failed to get current events");
        if let Some(ref events) = events {
            self.handle_events(events);
        }
        events
    }
}
