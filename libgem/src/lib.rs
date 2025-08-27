mod canvas;
pub mod element;
mod text;

pub use libopal;
pub use opal_img as image;

use libopal::{
    DequeuedEvents,
    window::{Pixel, Window, WindowFlags},
};

use crate::{
    canvas::DrawingCanvas,
    element::{Button, Container, ContainerLayout, Element, Label},
};

struct RootContainer {
    root: Window,
    window_x: u32,
    window_y: u32,
    body: Container<Window>,
    title_bar: Container<Window>,
    title_bar_y: u32,
    bg_color: Pixel,
    border_color: Pixel,
}

pub const BORDER_COLOR0: Pixel = Pixel::from_rgb(0xFD, 0xB0, 0xC0);
pub const DARK_BG_COLOR0: Pixel = Pixel::from_rgb_with_alpha(0, 0, 0, 0x80);
pub const DARK_BG_COLOR1: Pixel = Pixel::from_rgb_with_alpha(0, 0, 0, 0xFF);
pub const LIGHT_BG_COLOR0: Pixel = Pixel::from_rgb_with_alpha(0xFB, 0xF1, 0xC7, 0xFF);

impl RootContainer {
    const CORNER_RADIUS: u32 = 8;
    const TITLE_HEIGHT: u32 = Self::DEFAULT_FONT_SIZE + 10;
    const DEFAULT_FONT_SIZE: u32 = 12;

    fn new(width: u32, height: u32, title: &str, bg_color: Pixel, border_color: Pixel) -> Self {
        let real_width = width + Self::CORNER_RADIUS;
        let real_height = height + Self::TITLE_HEIGHT + 2;
        let window_x = Self::CORNER_RADIUS / 2;
        let window_y = Self::TITLE_HEIGHT + 2;

        let mut win = Window::create(WindowFlags::empty(), real_width, real_height);

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

        let x_button_width = Self::DEFAULT_FONT_SIZE + 10;

        let mut title_bar = Container::new(
            element::ContainerLayout::Horizontal,
            width,
            Self::TITLE_HEIGHT,
        );
        let title_bar_y = (Self::TITLE_HEIGHT - Self::DEFAULT_FONT_SIZE) / 2;
        let mut label = Label::new(
            title,
            Self::DEFAULT_FONT_SIZE as f32,
            Self::DEFAULT_FONT_SIZE as f32,
            (width - (x_button_width)) as f32,
            Self::TITLE_HEIGHT as f32,
        );
        label.set_color(Pixel::BLACK);

        let mut x_button = Button::new(
            x_button_width,
            Self::DEFAULT_FONT_SIZE + 1,
            Self::DEFAULT_FONT_SIZE as f32,
        );

        x_button.set_label("X");
        x_button.set_background_color(Pixel::from_hex_argb(0));
        x_button.set_border_color(Pixel::from_hex_argb(0));
        x_button.set_hover_color(Pixel::from_hex_argb(0));
        x_button.set_normal_text_color(Pixel::BLACK);
        x_button.set_hover_text_color(Pixel::from_rgb(0xFF, 0, 0));
        x_button.on_click(|_| std::process::exit(0));

        title_bar.add_element(Box::new(label));
        title_bar.add_element(Box::new(x_button));

        Self {
            root: win,
            window_x,
            window_y,
            body: Container::new(
                element::ContainerLayout::Vertical {
                    align_center: false,
                },
                width,
                height,
            ),
            title_bar,
            title_bar_y,
            bg_color,
            border_color,
        }
    }
}
pub struct Gem {
    root: RootContainer,
}

impl Gem {
    pub fn init(
        width: u32,
        height: u32,
        title: &str,
        bg_color: Pixel,
        border_color: Pixel,
    ) -> Self {
        libopal::init();
        let root_container = RootContainer::new(width, height, title, bg_color, border_color);
        Self {
            root: root_container,
        }
    }

    pub fn set_layout(&mut self, layout: ContainerLayout) {
        self.root.body.set_layout(layout);
    }

    pub fn add_element(&mut self, element: Box<dyn Element<Window>>) {
        self.root.body.add_element(element);
    }

    pub fn redraw(&mut self) {
        let mut handle_container =
            |container: &mut Container<Window>, start_x: u32, start_y: u32, bg_color: Pixel| {
                if container.needs_redraw() {
                    let end = container.draw(&mut self.root.root, start_x, start_y, bg_color);

                    if let Some((end_x, end_y)) = end {
                        let width = end_x - start_x;
                        let height = end_y - start_y;

                        self.root.root.redraw(start_x, start_y, width, height);
                    }
                }
            };

        handle_container(
            &mut self.root.title_bar,
            self.root.window_x,
            self.root.title_bar_y,
            self.root.border_color,
        );

        handle_container(
            &mut self.root.body,
            self.root.window_x,
            self.root.window_y,
            self.root.bg_color,
        );
    }

    pub fn handle_events_blocking(&mut self) -> DequeuedEvents {
        let events = libopal::dequeue_events_blocking().expect("Failed to wait for an event");

        for event in &*events {
            self.root
                .title_bar
                .handle_event(*event, self.root.window_x, self.root.title_bar_y);

            self.root
                .body
                .handle_event(*event, self.root.window_x, self.root.window_y);
        }
        events
    }
}
