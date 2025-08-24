mod canvas;
pub mod element;
mod text;

pub use libopal;
use libopal::window::{Pixel, Window};

use crate::{
    canvas::DrawingCanvas,
    element::{Container, Element},
    text::Text,
};

struct RootContainer {
    root: Window,
    width: u32,
    height: u32,
    window_x: u32,
    window_y: u32,
    inner: Container<Window>,
    title: Text,
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

        let mut title_text = Text::new(
            Self::DEFAULT_FONT_SIZE as f32,
            Self::DEFAULT_FONT_SIZE as f32,
            Some(Self::TITLE_HEIGHT as f32),
            Some(width as f32),
        );
        title_text.set_text(title);

        win.draw_text(
            window_x,
            (Self::TITLE_HEIGHT - Self::DEFAULT_FONT_SIZE) / 2,
            window_x + width,
            Self::TITLE_HEIGHT,
            &mut title_text,
        );

        win.redraw(0, 0, real_width, real_height);
        Self {
            root: win,
            width,
            height,
            window_x,
            window_y,
            inner: Container::new(),
            title: title_text,
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
        self.root.inner.add_element(element);
    }

    pub fn redraw(&mut self) {
        if self.root.inner.needs_redraw() {
            self.root
                .inner
                .draw(&mut self.root.root, self.root.window_x, self.root.window_y);

            self.root
                .root
                .redraw(0, 0, self.root.root.width(), self.root.root.height());
        }
    }

    pub fn handle_event_blocking(&mut self) {
        let event = libopal::wait_for_event_blocking().expect("Failed to wait for an event");
        dbg!(&event);
        self.root
            .inner
            .handle_event(event, self.root.window_x, self.root.window_y);
    }
}
