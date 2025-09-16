use libopal::window::Pixel;
use opal_img::display::ARGB;

use crate::{Gem, canvas::DrawingCanvas, element::Element};

pub enum ImageData {
    StaticBMP(opal_img::BMPImage<'static>),
    Generic(opal_img::PixelImage),
}

impl ImageData {
    pub const fn width(&self) -> u32 {
        match self {
            ImageData::StaticBMP(bmp) => bmp.width(),
            ImageData::Generic(pixel_image) => pixel_image.width(),
        }
    }

    pub const fn height(&self) -> u32 {
        match self {
            ImageData::StaticBMP(bmp) => bmp.height(),
            ImageData::Generic(pixel_image) => pixel_image.height(),
        }
    }
}

/// An element that displays an image.
pub struct Image {
    image: ImageData,
    needs_redraw: bool,
}

impl Image {
    pub fn new(image: ImageData) -> Self {
        Image {
            image,
            needs_redraw: false,
        }
    }

    /// Sets the image of the image element.
    pub fn set_image(&mut self, image: ImageData) {
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

impl<Canvas: DrawingCanvas, G: Gem> Element<Canvas, G> for Image {
    fn draw(
        &mut self,
        canvas: &mut Canvas,
        x: u32,
        y: u32,
        bg_color: Pixel,
    ) -> (Option<(u32, u32)>, Option<(u32, u32)>) {
        let width = self.width();
        let height = self.height();

        let pixels: &mut dyn Iterator<Item = ARGB> = match &self.image {
            ImageData::StaticBMP(bmp) => &mut bmp.pixels(),
            ImageData::Generic(p) => &mut p.get_pixels().iter().copied(),
        };

        for (i, color) in pixels.enumerate() {
            let x = (i % width as usize) as u32 + x;
            let y = (i / width as usize) as u32 + y;
            let pixel =
                Pixel::from_rgb(color.red(), color.green(), color.blue()).with_alpha(color.alpha());

            canvas.draw_pixel(x, y, pixel, Some(bg_color));
        }

        self.needs_redraw = false;
        (Some((x, y)), Some((x + width as u32, y + height as u32)))
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
