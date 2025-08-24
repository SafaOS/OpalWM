mod canvas;
pub mod element;
mod text;

pub use libopal;
use libopal::{
    Event,
    window::{Pixel, Window},
};

use crate::{
    canvas::DrawingCanvas,
    element::{Button, Container, Element, Label},
};

struct RootContainer {
    root: Window,
    width: u32,
    height: u32,
    window_x: u32,
    window_y: u32,
    body: Container<Window>,
    title_bar: Container<Window>,
    title_bar_y: u32,
}

impl RootContainer {
    const CORNER_RADIUS: u32 = 8;
    const BORDER_COLOR: Pixel = Pixel::from_rgb(0xFD, 0xB0, 0xC0);
    const BG_COLOR: Pixel = Pixel::from_rgb_with_alpha(0, 0, 0, 0x80);
    const TITLE_HEIGHT: u32 = Self::DEFAULT_FONT_SIZE + 10;
    const DEFAULT_FONT_SIZE: u32 = 12;

    fn new(width: u32, height: u32, title: &str) -> Self {
        let real_width = width + Self::CORNER_RADIUS;
        let real_height = height + Self::TITLE_HEIGHT;
        let window_x = Self::CORNER_RADIUS / 2;
        let window_y = Self::TITLE_HEIGHT;

        let mut win = Window::create(0, 0, real_width, real_height);

        win.draw_round_rect(
            0,
            0,
            real_width,
            real_height,
            Self::CORNER_RADIUS,
            |is_border, line_num| {
                if is_border {
                    Self::BORDER_COLOR
                } else {
                    if line_num < Self::TITLE_HEIGHT {
                        Self::BORDER_COLOR
                    } else {
                        Self::BG_COLOR
                    }
                }
            },
        );
        win.redraw(0, 0, real_width, real_height);

        let x_button_width = Self::DEFAULT_FONT_SIZE;

        let mut title_bar = Container::new(element::ContainerKind::Horizontal);
        let title_bar_y = (Self::TITLE_HEIGHT - Self::DEFAULT_FONT_SIZE) / 2;
        let label = Label::new(
            title,
            Self::DEFAULT_FONT_SIZE as f32,
            (width - x_button_width) as f32,
            Self::TITLE_HEIGHT as f32,
        );

        let mut x_button = Button::new(
            x_button_width,
            x_button_width,
            Self::DEFAULT_FONT_SIZE as f32,
        );

        x_button.set_label("X");
        x_button.set_background_color(Pixel::from_hex_argb(0));
        x_button.set_border_color(Pixel::from_hex_argb(0));
        x_button.set_hover_color(Pixel::from_hex_argb(0));
        x_button.set_hover_text_color(Pixel::from_rgb(0xFF, 0, 0));
        x_button.on_click(|_| std::process::exit(0));

        title_bar.add_element(Box::new(label));
        title_bar.add_element(Box::new(x_button));

        Self {
            root: win,
            width,
            height,
            window_x,
            window_y,
            body: Container::new(element::ContainerKind::Vertical),
            title_bar,
            title_bar_y,
        }
    }
}
pub struct Gem {
    root: RootContainer,
}

impl Gem {
    pub fn init(width: u32, height: u32, title: &str) -> Self {
        libopal::init();
        let root_container = RootContainer::new(width, height, title);
        Self {
            root: root_container,
        }
    }

    pub fn add_element(&mut self, element: Box<dyn Element<Window>>) {
        self.root.body.add_element(element);
    }

    pub fn redraw(&mut self) {
        if self.root.title_bar.needs_redraw() {
            let (start_x, start_y) = (self.root.window_x, self.root.title_bar_y);

            let end = self
                .root
                .title_bar
                .draw(&mut self.root.root, start_x, start_y);

            if let Some((end_x, end_y)) = end {
                let width = end_x - start_x;
                let height = end_y - start_y;

                self.root.root.redraw(start_x, start_y, width, height);
            }
        }

        if self.root.body.needs_redraw() {
            let (start_x, start_y) = (self.root.window_x, self.root.window_y);

            let end = self.root.body.draw(&mut self.root.root, start_x, start_y);

            if let Some((end_x, end_y)) = end {
                let width = end_x - start_x;
                let height = end_y - start_y;

                // TODO: Only sync changes
                _ = width;
                _ = height;

                self.root
                    .root
                    .redraw(0, 0, self.root.root.width(), self.root.root.height());
            }
        }
    }

    pub fn handle_event_blocking(&mut self) -> Event {
        let event = libopal::wait_for_event_blocking().expect("Failed to wait for an event");

        self.root
            .title_bar
            .handle_event(event, self.root.window_x, self.root.title_bar_y);

        dbg!(&event);
        self.root
            .body
            .handle_event(event, self.root.window_x, self.root.window_y);
        event
    }
}
