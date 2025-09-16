use libopal::window::Pixel;

use crate::{Gem, canvas::DrawingCanvas, element::Element, text::Text};

/// Text styles for a text box (see [`TextBox`]).
#[derive(Debug, Clone, Copy)]
pub struct TextBoxStyles {
    font_size: f32,
    text_color: Pixel,
    border_color: Option<Pixel>,
    line_padding: f32,
    max_width: f32,
    max_height: f32,
    text_alignment: Option<crate::text::Align>,
}

impl TextBoxStyles {
    const DEFAULT_FONT_SIZE: f32 = 12.0;
    const DEFAULT_LINE_PADDING: f32 = 2.0;
    const DEFAULT_TEXT_COLOR: Pixel = Pixel::BLACK;
    const DEFAULT_BORDER_COLOR: Option<Pixel> = None;
    const DEFAULT_TEXT_ALIGNMENT: Option<crate::text::Align> = Some(crate::text::Align::Center);

    /// Constructs a new text box styles with default values.
    pub const fn new(max_width: f32, max_height: f32) -> Self {
        Self {
            font_size: Self::DEFAULT_FONT_SIZE,
            text_color: Self::DEFAULT_TEXT_COLOR,
            border_color: Self::DEFAULT_BORDER_COLOR,
            line_padding: Self::DEFAULT_LINE_PADDING,
            text_alignment: Self::DEFAULT_TEXT_ALIGNMENT,
            max_width,
            max_height,
        }
    }

    /// Sets the text alignment of the text box.
    pub const fn with_text_alignment(mut self, text_alignment: Option<crate::text::Align>) -> Self {
        self.text_alignment = text_alignment;
        self
    }

    /// Sets the font size of the text box.
    pub const fn with_font_size(mut self, font_size: f32) -> Self {
        self.font_size = font_size;
        self
    }

    /// Sets the text color of the text box.
    pub const fn with_text_color(mut self, text_color: Pixel) -> Self {
        self.text_color = text_color;
        self
    }

    /// Sets the border color of the text box or if None there is no border.
    pub const fn with_border_color(mut self, border_color: Option<Pixel>) -> Self {
        self.border_color = border_color;
        self
    }

    /// Sets the padding between lines of text.
    pub const fn with_line_padding(mut self, line_padding: f32) -> Self {
        self.line_padding = line_padding;
        self
    }

    /// Returns the height of a line of text.
    pub const fn line_height(&self) -> f32 {
        self.font_size + self.line_padding
    }
}

/// An element that displays text.
pub struct TextBox {
    text: Text,
    styles: TextBoxStyles,
    needs_redraw: bool,
}

impl TextBox {
    /// Constructs a new [`TextBox`].
    pub fn new(content: &str, styles: TextBoxStyles) -> Self {
        let font_height = styles.font_size;
        let line_height = styles.line_height();
        let max_width = styles.max_width;
        let max_height = styles.max_height;

        let mut text = Text::new(font_height, line_height, Some(max_height), Some(max_width));
        text.align(styles.text_alignment);
        text.set_color(styles.text_color);
        text.set_text(content);

        Self {
            styles,
            text,
            needs_redraw: true,
        }
    }

    pub fn set_styles(&mut self, styles: TextBoxStyles) {
        // FIXME: There are a lots of things that aren't applied yet.
        self.styles = styles;
        self.text.set_color(styles.text_color);
        self.text
            .set_size(Some(styles.max_width), Some(styles.max_height));
        self.needs_redraw = true;
    }

    pub fn styles(&self) -> TextBoxStyles {
        self.styles
    }

    /// Sets the text of the label.
    pub fn set_text(&mut self, text: &str) {
        self.text.set_text(text);
        self.needs_redraw = true;
    }
}

impl<Canvas: DrawingCanvas, G: Gem> Element<Canvas, G> for TextBox {
    fn container_width(&self) -> u32 {
        self.styles.max_width as u32
    }

    fn container_height(&self) -> u32 {
        self.styles.max_height as u32
    }

    fn draw_height(&self) -> u32 {
        self.text.height() as u32
    }

    fn draw_width(&self) -> u32 {
        self.text.biggest_line_width() as u32
    }

    fn draw(
        &mut self,
        canvas: &mut Canvas,
        x: u32,
        y: u32,
        bg_color: Pixel,
    ) -> (Option<(u32, u32)>, Option<(u32, u32)>) {
        let max_x = x + self.styles.max_width as u32;
        let max_y = y + self.styles.max_height as u32;

        // TODO: Implement border drawing
        canvas.draw_rect(
            x,
            y,
            self.styles.max_width as u32,
            self.styles.max_height as u32,
            Pixel::NONE,
            Some(bg_color),
        );
        canvas.draw_text(x, y, max_x, max_y, &mut self.text, Some(bg_color));
        self.needs_redraw = false;
        (Some((x, y)), Some((max_x, max_y)))
    }

    fn needs_redraw(&self) -> bool {
        self.needs_redraw
    }
}
