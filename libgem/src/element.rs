use libopal::{event::HeldMouseButtons, window::Pixel};
use opal_img::display::ARGB;

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
    /// The amount of pixels this element takes up from the x axis, not including padding.
    fn draw_width(&self) -> u32;
    /// The amount of pixels this element takes up from the y axis, not including padding.
    fn draw_height(&self) -> u32;
    /// The amount of pixels this element takes up from the x axis, including padding.
    fn container_width(&self) -> u32;
    /// The amount of pixels this element takes up from the y axis, including padding.
    fn container_height(&self) -> u32;

    /// Draws the element onto the canvas, given a relative position of the element from the canvas.
    /// Returns either None or the end position of the element as if it was a rectangle, and that is (x, y) where x is the rightmost x coordinate and y is the lowest y coordinate of the element.
    fn draw(&mut self, canvas: &mut RootCanvas, x: u32, y: u32) -> Option<(u32, u32)>;
    /// Returns true if the element needs to be redrawn.
    fn needs_redraw(&self) -> bool;
    /// Handles an event for the element, given a relative position of the element from the canvas.
    fn handle_event(&mut self, event: libopal::Event, ele_x: u32, ele_y: u32) {
        _ = event;
        _ = ele_x;
        _ = ele_y;
    }
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
            need_redraw: true,
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
        self.hover_text_color = color;
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

    fn container_height(&self) -> u32 {
        self.height
    }

    fn container_width(&self) -> u32 {
        self.width
    }

    fn handle_event(&mut self, event: libopal::Event, ele_x: u32, ele_y: u32) {
        let last_mouse_hovering = self.mouse_hovering;

        let (mouse_x, mouse_y, is_held) = match event {
            libopal::Event::MouseChange(change_event) => (
                Some(change_event.x()),
                Some(change_event.y()),
                change_event.held_buttons().contains(HeldMouseButtons::LEFT),
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
            let is_inside = is_inside_rect(mouse_x, mouse_y, ele_x, ele_y, self.width, self.height);
            self.mouse_hovering = is_inside;
        }

        if is_held && self.mouse_hovering {
            // FIXME: perhaps only detect on button release, but the WM currently cannot send release events for some odd reason
            if is_held {
                if let Some(f) = self.on_click {
                    f(self);
                }
            }
        }

        self.need_redraw = self.need_redraw || (self.mouse_hovering != last_mouse_hovering);
    }
}

/// A label element that displays text.
pub struct Label {
    text: Text,
    width: u32,
    height: u32,
    needs_redraw: bool,
}

impl Label {
    pub fn new(content: &str, font_size: f32, max_width: f32, max_height: f32) -> Self {
        let mut text = Text::new(font_size, font_size, Some(max_height), Some(max_width));
        text.set_text(content);

        Label {
            width: max_width as u32,
            height: max_height as u32,
            text,
            needs_redraw: true,
        }
    }

    /// Sets the color of the label.
    pub fn set_color(&mut self, color: Pixel) {
        self.text.set_color(color);
        self.needs_redraw = true;
    }

    /// Sets the text of the label.
    pub fn set_text(&mut self, text: &str) {
        self.text.set_text(text);
        self.needs_redraw = true;
    }
}

impl<Canvas: DrawingCanvas> Element<Canvas> for Label {
    fn container_width(&self) -> u32 {
        self.width
    }

    fn container_height(&self) -> u32 {
        self.height
    }

    fn draw_height(&self) -> u32 {
        self.text.height() as u32
    }

    fn draw_width(&self) -> u32 {
        self.text.biggest_line_width() as u32
    }

    fn draw(&mut self, canvas: &mut Canvas, x: u32, y: u32) -> Option<(u32, u32)> {
        let max_x = x + self.width;
        let max_y = y + self.height;

        canvas.draw_text(x, y, max_x, max_y, &mut self.text);
        self.needs_redraw = false;
        Some((max_x, max_y))
    }

    fn needs_redraw(&self) -> bool {
        self.needs_redraw
    }
}

pub enum ImageData<'a> {
    BMP(opal_img::BMPImage<'a>),
    PixelImage(opal_img::PixelImage),
}

impl<'a> ImageData<'a> {
    pub const fn width(&self) -> u32 {
        match self {
            ImageData::BMP(bmp) => bmp.width(),
            ImageData::PixelImage(pixel_image) => pixel_image.width(),
        }
    }

    pub const fn height(&self) -> u32 {
        match self {
            ImageData::BMP(bmp) => bmp.height(),
            ImageData::PixelImage(pixel_image) => pixel_image.height(),
        }
    }
}

/// An element that displays an image.
pub struct Image<'a> {
    image: ImageData<'a>,
    needs_redraw: bool,
}

impl<'a> Image<'a> {
    pub fn new(image: ImageData<'a>) -> Self {
        Image {
            image,
            needs_redraw: false,
        }
    }

    /// Sets the image of the image element.
    pub fn set_image(&mut self, image: ImageData<'a>) {
        self.image = image;
        self.needs_redraw = true;
    }

    pub const fn width(&self) -> u32 {
        self.image.width() as u32
    }

    pub const fn height(&self) -> u32 {
        self.image.height() as u32
    }
}

impl<'a, Canvas: DrawingCanvas> Element<Canvas> for Image<'a> {
    fn draw(&mut self, canvas: &mut Canvas, x: u32, y: u32) -> Option<(u32, u32)> {
        let width = self.width();
        let height = self.height();

        let pixels: &mut dyn Iterator<Item = ARGB> = match &self.image {
            ImageData::BMP(bmp) => &mut bmp.pixels(),
            ImageData::PixelImage(p) => &mut p.get_pixels().iter().copied(),
        };

        for (i, color) in pixels.enumerate() {
            let x = (i % width as usize) as u32 + x;
            let y = (i / width as usize) as u32 + y;
            let pixel =
                Pixel::from_rgb(color.red(), color.green(), color.blue()).with_alpha(color.alpha());

            canvas.draw_pixel(x, y, pixel);
        }

        self.needs_redraw = false;
        Some((x + width as u32, y + height as u32))
    }

    fn needs_redraw(&self) -> bool {
        self.needs_redraw
    }

    fn container_height(&self) -> u32 {
        self.height() as u32
    }

    fn container_width(&self) -> u32 {
        self.width() as u32
    }

    fn draw_height(&self) -> u32 {
        self.height() as u32
    }

    fn draw_width(&self) -> u32 {
        self.width() as u32
    }
}

/// Describes how a container should be laid out.
pub enum ContainerLayout {
    Horizontal,
    Vertical {
        /// Aligns the elements in the center of the container.
        align_center: bool,
    },
}

/// A customizable container of elements, that handles their layout and such.
pub struct Container<Canvas: DrawingCanvas> {
    layout: ContainerLayout,
    elements: Vec<Box<dyn Element<Canvas>>>,
    /* FIXME: Save element height and width information and set this true if these were changed */
    elements_changed: bool,
    max_width: u32,
    max_height: u32,
}

impl<Canvas: DrawingCanvas> Container<Canvas> {
    pub const fn new(layout: ContainerLayout, max_width: u32, max_height: u32) -> Self {
        Container {
            layout,
            elements: Vec::new(),
            elements_changed: false,
            max_width,
            max_height,
        }
    }

    pub const fn set_layout(&mut self, layout: ContainerLayout) {
        self.layout = layout;
        self.elements_changed = true;
    }

    pub fn add_element(&mut self, element: Box<dyn Element<Canvas>>) {
        self.elements.push(element);
        self.elements_changed = true;
    }
}

impl<Canvas: DrawingCanvas> Element<Canvas> for Container<Canvas> {
    fn draw(&mut self, canvas: &mut Canvas, mut x: u32, mut y: u32) -> Option<(u32, u32)> {
        let is_centered = matches!(
            self.layout,
            ContainerLayout::Vertical { align_center: true }
        );
        let mut draw_ended_at = None;

        for element in self.elements.iter_mut() {
            let draw_x = if is_centered {
                let element_width = element.draw_width();
                let container_width = self.max_width;
                (container_width.saturating_sub(element_width)) / 2
            } else {
                x
            };
            let draw_y = y;

            if self.elements_changed || element.needs_redraw() {
                let results = element.draw(canvas, draw_x, draw_y);

                match (results, draw_ended_at) {
                    (None, None) => (),
                    (Some((x, y)), Some((x2, y2))) => {
                        draw_ended_at = Some((x.max(x2), y.max(y2)));
                    }
                    (Some((x, y)), None) => {
                        draw_ended_at = Some((x, y));
                    }
                    (None, Some(_)) => {}
                }
            }

            match self.layout {
                ContainerLayout::Horizontal => {
                    x += element.container_width();
                }
                ContainerLayout::Vertical { .. } => {
                    y += element.container_height();
                }
            }
        }

        draw_ended_at
    }

    fn needs_redraw(&self) -> bool {
        self.elements_changed || self.elements.iter().any(|ele| ele.needs_redraw())
    }

    #[inline]
    fn draw_height(&self) -> u32 {
        self.elements
            .iter()
            .map(|element| element.draw_height())
            .sum()
    }

    #[inline]
    fn draw_width(&self) -> u32 {
        self.elements
            .iter()
            .map(|element| element.draw_width())
            .max()
            .unwrap_or(0)
    }

    #[inline]
    fn container_height(&self) -> u32 {
        self.draw_height()
    }

    #[inline]
    fn container_width(&self) -> u32 {
        self.draw_width()
    }

    fn handle_event(&mut self, event: libopal::Event, mut ele_x: u32, mut ele_y: u32) {
        let is_centered = matches!(
            self.layout,
            ContainerLayout::Vertical { align_center: true }
        );

        for element in self.elements.iter_mut() {
            let draw_x = if is_centered {
                let element_width = element.draw_width();
                let container_width = self.max_width;
                (container_width - element_width) / 2
            } else {
                ele_x
            };

            element.handle_event(event, draw_x, ele_y);
            match self.layout {
                ContainerLayout::Horizontal => {
                    ele_x += element.container_width();
                }
                ContainerLayout::Vertical { .. } => {
                    ele_y += element.container_height();
                }
            }
        }
    }
}
