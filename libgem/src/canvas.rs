pub use libopal::window::Pixel;
use libopal::window::Window;

use crate::text::Text;

pub trait DrawingCanvas {
    fn draw_pixel(&mut self, x: u32, y: u32, pixel: Pixel);

    fn width(&self) -> u32;
    fn height(&self) -> u32;

    fn draw_text(&mut self, start_x: u32, start_y: u32, max_x: u32, max_y: u32, text: &mut Text) {
        text.draw(|x, y, _, _, color| {
            let x = x as u32 + start_x;
            let y = y as u32 + start_y;

            if x >= max_x || y >= max_y {
                return;
            }

            self.draw_pixel(x, y, color);
        });
    }

    #[inline]
    fn draw_rect_points(&mut self, x1: u32, y1: u32, x2: u32, y2: u32, pixel: Pixel) {
        // includes x2 and y2
        let width = (x2 - x1) + 1;
        let height = (y2 - y1) + 1;

        self.draw_rect(x1, y1, width, height, pixel);
    }

    #[inline]
    fn draw_rect(&mut self, x: u32, y: u32, width: u32, height: u32, pixel: Pixel) {
        for row in 0..height {
            for col in 0..width {
                self.draw_pixel(col + x, row + y, pixel);
            }
        }
    }

    #[inline]
    fn draw_line(&mut self, x0: u32, y0: u32, x1: u32, y1: u32, pixel: Pixel) {
        self.draw_rect_points(x0, y0, x1, y1, pixel)
    }

    /// Draw a circle, starting at (x, y) which is the top-left corner of the circle,
    /// and ending at (x + radius*2, y + radius*2).
    #[inline]
    fn draw_circle(&mut self, x: u32, y: u32, radius: u32, border_color: Pixel) {
        let x = x + (radius * 2);
        let y = y + (radius * 2);

        let mut f = 1 - radius as i32;
        let mut ddf_x = 1;
        let mut ddf_y = -2 * radius as i32;

        let mut xx = 0u32;
        let mut yy = radius;

        while xx < yy {
            if f >= 0 {
                yy -= 1;
                ddf_y += 2;
                f += ddf_y;
            }

            xx += 1;
            ddf_x += 2;
            f += ddf_x;

            // Bottom Right corner
            self.draw_pixel(x + xx - radius, y + yy - radius, border_color);
            self.draw_pixel(x + yy - radius, y + xx - radius, border_color);
            // Top Right corner
            self.draw_pixel(x + xx - radius, y - yy - radius, border_color);
            self.draw_pixel(x + yy - radius, y - xx - radius, border_color);
            // Bottom Left corner
            self.draw_pixel(x - xx - radius, y + yy - radius, border_color);
            self.draw_pixel(x - yy - radius, y + xx - radius, border_color);
            // Top Left corner
            self.draw_pixel(x - xx - radius, y - yy - radius, border_color);
            self.draw_pixel(x - yy - radius, y - xx - radius, border_color);
        }
    }

    /// Draws a rounded rectangle on the canvas with the given border color and fills with the given fill color got by calling get_color(false, line_num),
    /// the border color got by calling get_color(true, line_num)
    ///
    /// as get_color(is_border: bool, line_num: u32) -> Pixel
    fn draw_round_rect<F: Fn(bool, u32) -> Pixel>(
        &mut self,
        start_x: u32,
        start_y: u32,
        width: u32,
        height: u32,
        radius: u32,
        get_color: F,
    ) {
        // Draws two corners of a rounded rectangle, and then connects them with a line of a color got by the function get_color
        let mut draw_2corners = |x0: u32, x1: u32, y: u32, top: bool| {
            let x0 = x0 + (radius * 2);
            let x1 = x1 + (radius * 2);

            let y = y + (radius * 2);

            let mut f = 1 - radius as i32;
            let mut ddf_x = 1;
            let mut ddf_y = -2 * radius as i32;

            let mut xx = 0u32;
            let mut yy = radius;

            while xx < yy {
                let last_yy = yy;
                let last_xx = xx;

                if f >= 0 {
                    yy -= 1;
                    ddf_y += 2;
                    f += ddf_y;
                }

                xx += 1;
                ddf_x += 2;
                f += ddf_x;

                let draw_y = if !top {
                    y + yy - radius
                } else {
                    y - yy - radius
                };
                let draw_y_flipped = if !top {
                    y + xx - radius
                } else {
                    y - xx - radius
                };

                let draw_x0 = x0 - xx - radius;
                let draw_x0_flipped = x0 - yy - radius;

                let draw_x1 = x1 + xx - radius;
                let draw_x1_flipped = x1 + yy - radius;

                let y_line = draw_y - start_y;
                let y_line_flipped = draw_y_flipped - start_y;

                let color = get_color(true, y_line);
                let fill_color = get_color(false, y_line);

                let flipped_color = get_color(true, y_line_flipped);
                let fill_flipped_color = get_color(false, y_line_flipped);

                // Bottom or Top left corner
                self.draw_pixel(draw_x0, draw_y, color);
                self.draw_pixel(draw_x0_flipped, draw_y_flipped, flipped_color);

                // Bottom or Top right corner
                self.draw_pixel(draw_x1, draw_y, color);
                self.draw_pixel(draw_x1_flipped, draw_y_flipped, flipped_color);

                // Draw the fill
                // not flipped
                if yy != last_yy {
                    self.draw_line(draw_x0 + 1, draw_y, draw_x1 - 1, draw_y, fill_color);
                }

                // flipped
                if xx != last_xx {
                    self.draw_line(
                        draw_x0_flipped + 1,
                        draw_y_flipped,
                        draw_x1_flipped - 1,
                        draw_y_flipped,
                        fill_flipped_color,
                    );
                }
            }
        };
        let x0 = start_x;
        let y0 = start_y;
        let x1 = (x0 + width) - 1;
        let y1 = (y0 + height) - 1;

        draw_2corners(x0, x1 - (radius * 2), y0, true);
        draw_2corners(x0, x1 - (radius * 2), y1 - (radius * 2), false);

        let border_top_color = get_color(true, 0);
        let border_bottom_color = get_color(true, height);

        // Draws the border
        // Top line
        self.draw_line(x0 + radius, y0, x1 - radius, y0, border_top_color);
        // Bottom line
        self.draw_line(x0 + radius, y1, x1 - radius, y1, border_bottom_color);
        // Left line
        self.draw_line(x0, y0 + radius, x0, y1 - radius, border_top_color);
        // Right line
        self.draw_line(x1, y0 + radius, x1, y1 - radius, border_top_color);

        for y in (y0 + radius)..=(y1 - radius) {
            let fill_color = get_color(false, y - start_y);
            self.draw_line(x0 + 1, y, x1 - 1, y, fill_color);
        }
    }
}

impl DrawingCanvas for Window {
    #[inline]
    fn height(&self) -> u32 {
        self.height()
    }

    #[inline]
    fn width(&self) -> u32 {
        self.width()
    }

    #[inline]
    fn draw_pixel(&mut self, x: u32, y: u32, pixel: Pixel) {
        let index = (y * self.width() + x) as usize;
        let bottom = &mut self.pixels_mut()[index];
        *bottom = pixel.blend(bottom);
    }
}
