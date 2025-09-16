use libopal::{event::HeldMouseButtons, window::Pixel};

use crate::{Gem, canvas::DrawingCanvas, element::Element, text::Text};

#[derive(Debug, Clone, Copy)]
/// Represents the style of a button element.
pub struct ButtonStyle {
    hover_color: Pixel,
    normal_color: Pixel,
    border_color: Option<Pixel>,
    normal_text_color: Pixel,
    hover_text_color: Pixel,
    corner_radius: u32,
    max_width: u32,
    max_height: u32,
    font_height: f32,
    text_line_padding: f32,
}

impl ButtonStyle {
    const DEFAULT_HOVER_COLOR: Pixel = Pixel::from_rgb(0xFD, 0xB0, 0xC0);
    const DEFAULT_NORMAL_COLOR: Pixel = Pixel::from_rgb(0xFD, 0xB0, 0xF0);
    const TEXT_COLOR: Pixel = Pixel::BLACK;
    const DEFAULT_BORDER_RADIUS: u32 = 8;
    const DEFAULT_FONT_HEIGHT: f32 = 12.0;
    const DEFAULT_TEXT_LINE_PADDING: f32 = 3.0;

    /// Creates a new button style with default values.
    pub const fn new(width: u32, height: u32) -> Self {
        let corner_radius = Self::DEFAULT_BORDER_RADIUS;
        let max_width = width + corner_radius;
        let max_height = height;

        Self {
            hover_color: Self::DEFAULT_HOVER_COLOR,
            normal_color: Self::DEFAULT_NORMAL_COLOR,
            border_color: None,
            normal_text_color: Self::TEXT_COLOR,
            hover_text_color: Self::TEXT_COLOR,
            font_height: Self::DEFAULT_FONT_HEIGHT,
            text_line_padding: Self::DEFAULT_TEXT_LINE_PADDING,
            corner_radius,
            max_width,
            max_height,
        }
    }

    /// Sets the color of the button's background when it is hovered over.
    pub const fn with_hover_color(mut self, color: Pixel) -> Self {
        self.hover_color = color;
        self
    }

    /// Sets the color of the button's background when it is not hovered over.
    pub const fn with_normal_color(mut self, color: Pixel) -> Self {
        self.normal_color = color;
        self
    }

    /// Sets the color of the button border.
    /// if color is None, the border will be the same as the buttons color.
    pub const fn with_border_color(mut self, color: Option<Pixel>) -> Self {
        self.border_color = color;
        self
    }

    /// Sets the color of the button text when it is not hovered over.
    pub const fn with_normal_text_color(mut self, color: Pixel) -> Self {
        self.normal_text_color = color;
        self
    }

    /// Sets the color of the button text when it is hovered over.
    pub const fn with_hover_text_color(mut self, color: Pixel) -> Self {
        self.hover_text_color = color;
        self
    }

    /// does both [`Self::with_normal_text_color`] and [`Self::with_hover_text_color`] at the same time.
    pub const fn with_text_color(mut self, color: Pixel) -> Self {
        self.normal_text_color = color;
        self.hover_text_color = color;
        self
    }

    /// Sets the radius of the button's border.
    pub const fn with_border_radius(mut self, radius: u32) -> Self {
        self.max_width = self.max_width - self.corner_radius + radius;
        self.corner_radius = radius;
        self
    }

    /// Sets the height of the button's font.
    pub const fn with_font_height(mut self, height: f32) -> Self {
        self.font_height = height;
        self
    }

    /// Sets the padding between lines of text.
    pub const fn with_text_line_padding(mut self, padding: f32) -> Self {
        self.text_line_padding = padding;
        self
    }

    pub const fn width(&self) -> u32 {
        self.max_width - self.corner_radius
    }

    pub const fn real_width(&self) -> u32 {
        self.max_width
    }

    pub const fn height(&self) -> u32 {
        self.max_height
    }

    pub const fn min_x(&self) -> u32 {
        self.corner_radius / 2
    }
}

/// A button element that can be clicked.
pub struct Button<G: Gem> {
    on_click: Option<Box<dyn Fn(&mut Self, &mut G)>>,
    text: Text,
    mouse_hovering: bool,
    was_pressed: bool,
    need_redraw: bool,
    style: ButtonStyle,
}

impl<G: Gem> Button<G> {
    pub fn new(label: &str, style: ButtonStyle) -> Self {
        let font_height = style.font_height;

        let mut text = Text::new(
            font_height,
            font_height + style.text_line_padding,
            Some(style.height() as f32),
            Some(style.width() as f32),
        );

        text.align(Some(crate::text::Align::Center));
        text.set_text(label);

        Self {
            on_click: None,
            text,
            was_pressed: false,
            mouse_hovering: false,
            need_redraw: true,
            style,
        }
    }

    pub fn on_click<F: Fn(&mut Self, &mut G) + 'static>(&mut self, on_click: F) {
        self.on_click = Some(Box::new(on_click));
    }

    pub fn style(&self) -> ButtonStyle {
        self.style
    }

    pub fn set_style(&mut self, style: ButtonStyle) {
        self.style = style;
        self.need_redraw = true;
    }

    pub fn set_label(&mut self, label: &str) {
        self.text.set_text(label);
        self.need_redraw = true;
    }
}

impl<RootCanvas: DrawingCanvas, G: Gem> Element<RootCanvas, G> for Button<G> {
    fn draw(
        &mut self,
        canvas: &mut RootCanvas,
        x: u32,
        y: u32,
        bg_color: Pixel,
    ) -> (Option<(u32, u32)>, Option<(u32, u32)>) {
        let width = self.style.max_width;
        let height = self.style.max_height;

        canvas.draw_round_rect(
            x,
            y,
            width,
            height,
            6,
            |is_border, _| match (is_border, self.style.border_color, self.mouse_hovering) {
                (true, Some(color), _) => color,
                (_, _, true) => self.style.hover_color,
                (_, _, false) => self.style.normal_color,
            },
            Some(bg_color),
        );

        let align_y = y + ((height.saturating_sub(self.text.height() as u32)) / 2);

        if self.mouse_hovering {
            self.text.set_color(self.style.hover_text_color);
        } else {
            self.text.set_color(self.style.normal_text_color);
        }

        canvas.draw_text(
            x + self.style.min_x(),
            align_y,
            x + width,
            y + height,
            &mut self.text,
            None,
        );
        self.need_redraw = false;
        (Some((x, y)), Some((x + width, y + height)))
    }

    fn needs_redraw(&self) -> bool {
        self.need_redraw
    }

    fn draw_height(&self) -> u32 {
        self.style.max_height
    }

    fn draw_width(&self) -> u32 {
        self.style.max_width
    }

    fn container_height(&self) -> u32 {
        self.style.max_height
    }

    fn container_width(&self) -> u32 {
        self.style.max_width
    }

    fn handle_event(&mut self, gem: &mut G, event: libopal::Event, ele_x: u32, ele_y: u32) {
        let last_mouse_hovering = self.mouse_hovering;

        let (mouse_x, mouse_y, is_held) = match event {
            libopal::Event::MouseChange(change_event) => (
                Some(change_event.x()),
                Some(change_event.y()),
                change_event.buttons_changed()
                    && change_event.held_buttons().contains(HeldMouseButtons::LEFT),
            ),
            libopal::Event::MouseEnter(enter_event) => {
                (Some(enter_event.x()), Some(enter_event.y()), false)
            }
            _ => {
                self.mouse_hovering = false;
                (None, None, false)
            }
        };
        self.was_pressed = is_held;

        if let Some(mouse_x) = mouse_x
            && let Some(mouse_y) = mouse_y
        {
            let is_inside = super::is_inside_rect(
                mouse_x,
                mouse_y,
                ele_x,
                ele_y,
                self.style.max_width,
                self.style.max_height,
            );
            self.mouse_hovering = is_inside;
        }

        if is_held && self.mouse_hovering {
            // FIXME: perhaps only detect on button release, but the WM currently cannot send release events for some odd reason
            if is_held {
                if let Some(f) = self.on_click.take() {
                    f(self, gem);
                    if self.on_click.is_none() {
                        self.on_click = Some(f);
                    }
                }
            }
        }

        self.need_redraw = self.need_redraw || (self.mouse_hovering != last_mouse_hovering);
    }
}
