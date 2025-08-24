use libopal::{event::HeldMouseButtons, window::Pixel};

use crate::{canvas::DrawingCanvas, text::Text};

const fn is_inside_rect(
    x: u32,
    y: u32,
    rect_x: u32,
    rect_y: u32,
    rect_width: u32,
    rect_height: u32,
) -> bool {
    x >= rect_x && x < rect_x + rect_width && y >= rect_y && y < rect_y + rect_height
}

pub trait Element<RootCanvas: DrawingCanvas> {
    /// The amount of pixels this element takes up from the x axis.
    fn draw_width(&self) -> u32;
    /// The amount of pixels this element takes up from the y axis.
    fn draw_height(&self) -> u32;

    /// Draws the element onto the canvas, given a relative position of the element from the canvas.
    /// Returns either None or the end position of the element as if it was a rectangle, and that is (x, y) where x is the rightmost x coordinate and y is the lowest y coordinate of the element.
    fn draw(&mut self, canvas: &mut RootCanvas, x: u32, y: u32) -> Option<(u32, u32)>;
    /// Returns true if the element needs to be redrawn.
    fn needs_redraw(&self) -> bool;
    /// Handles an event for the element, given a relative position of the element from the canvas.
    fn handle_event(&mut self, event: libopal::Event, ele_x: u32, ele_y: u32);
}

/// A button element that can be clicked.
pub struct Button {
    on_click: Option<fn(&mut Self)>,
    text: Text,
    width: u32,
    height: u32,

    mouse_hovering: bool,
    was_pressed: bool,
    need_redraw: bool,
    hover_color: Pixel,
    normal_color: Pixel,
    border_color: Pixel,
    normal_text_color: Pixel,
    hover_text_color: Pixel,
}

impl Button {
    const DEFAULT_HOVER_COLOR: Pixel = Pixel::from_rgb(0xFD, 0xB0, 0xC0);
    const DEFAULT_NORMAL_COLOR: Pixel = Pixel::from_rgb(0xFD, 0xB0, 0xF0);
    const DEFAULT_BORDER_COLOR: Pixel = Pixel::from_hex_argb(0);
    const TEXT_COLOR: Pixel = Pixel::from_rgb(0xFF, 0xFF, 0xFF);

    pub fn new(width: u32, height: u32, font_height: f32) -> Self {
        let text = Text::new(
            font_height,
            font_height,
            Some(height as f32),
            Some(width as f32),
        );

        Self {
            on_click: None,
            text,
            width,
            height,
            was_pressed: false,
            mouse_hovering: false,
            need_redraw: false,
            hover_color: Self::DEFAULT_HOVER_COLOR,
            normal_color: Self::DEFAULT_NORMAL_COLOR,
            border_color: Self::DEFAULT_BORDER_COLOR,
            normal_text_color: Self::TEXT_COLOR,
            hover_text_color: Self::TEXT_COLOR,
        }
    }

    pub fn set_hover_color(&mut self, color: Pixel) {
        self.hover_color = color;
    }

    pub fn set_background_color(&mut self, color: Pixel) {
        self.normal_color = color;
    }

    pub fn set_border_color(&mut self, color: Pixel) {
        self.border_color = color;
    }

    pub fn set_normal_text_color(&mut self, color: Pixel) {
        self.normal_text_color = color;
    }

    pub fn set_hover_text_color(&mut self, color: Pixel) {
        self.hover_color = color;
    }

    pub fn on_click(&mut self, on_click: fn(&mut Self)) {
        self.on_click = Some(on_click);
    }

    pub fn set_label(&mut self, label: &str) {
        self.text.set_text(label);
        self.need_redraw = true;
    }
}

impl<RootCanvas: DrawingCanvas> Element<RootCanvas> for Button {
    fn draw(&mut self, canvas: &mut RootCanvas, x: u32, y: u32) -> Option<(u32, u32)> {
        let width = self.width;
        let height = self.height;

        canvas.draw_round_rect(x, y, width, height, 0x0, |_, _| Pixel::from_rgb(0, 0, 0));
        canvas.draw_round_rect(x, y, width, height, 6, |is_border, _| {
            if is_border {
                self.border_color
            } else if self.mouse_hovering {
                self.hover_color
            } else {
                self.normal_color
            }
        });

        // FIXME: Align multi-line buttons to center
        let align_x = x + ((self.width.saturating_sub(self.text.width() as u32)) / 2);
        let align_y = y + ((self.height.saturating_sub(self.text.height() as u32)) / 2);

        if self.mouse_hovering {
            self.text.set_color(self.hover_text_color);
        } else {
            self.text.set_color(self.normal_text_color);
        }

        canvas.draw_text(
            align_x,
            align_y,
            x + self.width,
            y + self.height,
            &mut self.text,
        );
        self.need_redraw = false;
        Some((x + self.width, y + self.height))
    }

    fn needs_redraw(&self) -> bool {
        self.need_redraw
    }

    fn draw_height(&self) -> u32 {
        self.height
    }

    fn draw_width(&self) -> u32 {
        self.width
    }

    fn handle_event(&mut self, event: libopal::Event, ele_x: u32, ele_y: u32) {
        let last_mouse_hovering = self.mouse_hovering;
        self.mouse_hovering = false;

        let (mouse_x, mouse_y, is_held) = match event {
            libopal::Event::MouseChange(change_event) => (
                Some(change_event.x()),
                Some(change_event.y()),
                change_event.held_buttons().contains(HeldMouseButtons::LEFT),
            ),
            libopal::Event::MouseEnter(enter_event) => {
                (Some(enter_event.x()), Some(enter_event.y()), false)
            }
            _ => (None, None, false),
        };
        self.was_pressed = is_held;

        if let Some(mouse_x) = mouse_x
            && let Some(mouse_y) = mouse_y
        {
            let is_inside = is_inside_rect(mouse_x, mouse_y, ele_x, ele_y, self.width, self.height);
            if is_inside {
                // FIXME: perhaps only detect on button release, but the WM currently cannot send release events for some odd reason
                if is_held {
                    if let Some(f) = self.on_click {
                        f(self);
                    }
                }
                self.mouse_hovering = true;
            }
        }

        if last_mouse_hovering != self.mouse_hovering {
            self.need_redraw = true;
        }
    }
}

/// A customizable container of elements, that handles their layout and such.
pub struct Container<Canvas: DrawingCanvas> {
    elements: Vec<Box<dyn Element<Canvas>>>,
    /* FIXME: Save element height and width information and set this true if these were changed */
    elements_changed: bool,
}

impl<Canvas: DrawingCanvas> Container<Canvas> {
    pub fn new() -> Self {
        Container {
            elements: Vec::new(),
            elements_changed: false,
        }
    }

    pub fn add_element(&mut self, element: Box<dyn Element<Canvas>>) {
        self.elements.push(element);
        self.elements_changed = true;
    }
}

impl<Canvas: DrawingCanvas> Element<Canvas> for Container<Canvas> {
    fn draw(&mut self, canvas: &mut Canvas, x: u32, mut y: u32) -> Option<(u32, u32)> {
        let mut draw_ended_at = None;

        for element in self.elements.iter_mut() {
            let height = element.draw_height();
            if self.elements_changed || element.needs_redraw() {
                let results = element.draw(canvas, x, y);

                if results.is_some_and(|res| draw_ended_at.is_none_or(|e| res > e)) {
                    draw_ended_at = results;
                }
            }
            y += height;
        }

        draw_ended_at
    }

    fn needs_redraw(&self) -> bool {
        self.elements_changed || self.elements.iter().any(|ele| ele.needs_redraw())
    }

    fn draw_height(&self) -> u32 {
        self.elements
            .iter()
            .map(|element| element.draw_height())
            .sum()
    }

    fn draw_width(&self) -> u32 {
        self.elements
            .iter()
            .map(|element| element.draw_width())
            .max()
            .unwrap_or(0)
    }

    fn handle_event(&mut self, event: libopal::Event, ele_x: u32, ele_y: u32) {
        let x = ele_x;
        let mut y = ele_y;
        for (_, element) in self.elements.iter_mut().enumerate() {
            element.handle_event(event, x, y);
            y += element.draw_height();
        }
    }
}
