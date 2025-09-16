/// Microsoft BMP image format
pub mod bmp;
/// QOI image format
pub mod qoi;

pub use bmp::BMPImage;
pub use qoi::QOIImage;

use resize::{Pixel::RGBA8, Resizer, px::RGBA};

use crate::display::ARGB;

pub mod display;
pub use resize::Type as ScaleType;

#[derive(Debug, Clone)]
/// Generic representation of an image that has been decoded.
pub struct PixelImage {
    pixels: Vec<ARGB>,
    width: u32,
    height: u32,
}

impl PixelImage {
    pub fn new(pixels: Vec<ARGB>, width: u32, height: u32) -> Self {
        PixelImage {
            pixels,
            width,
            height,
        }
    }

    pub const fn width(&self) -> u32 {
        self.width
    }

    pub const fn height(&self) -> u32 {
        self.height
    }

    pub fn get_pixels(&self) -> &[ARGB] {
        &self.pixels
    }

    pub fn scale(&mut self, width: u32, height: u32, scale_kind: ScaleType) {
        if self.width == width && self.height == height {
            return;
        }

        let old_pixels = self
            .pixels
            .iter()
            .map(|color| RGBA::new(color.red(), color.green(), color.blue(), color.alpha()))
            .collect::<Vec<_>>();

        let mut resizer = Resizer::new(
            self.width as usize,
            self.height as usize,
            width as usize,
            height as usize,
            RGBA8,
            scale_kind,
        )
        .expect("Failed to construct a Resizer");

        let mut new_pixels = vec![RGBA::new(0, 0, 0, 0); height as usize * width as usize];
        resizer
            .resize(&old_pixels, &mut new_pixels)
            .expect("Failed to resize Image");
        let new_image = PixelImage {
            pixels: new_pixels
                .iter()
                .map(|color| ARGB::from_rgba(color.r, color.g, color.b, color.a))
                .collect(),
            width,
            height,
        };
        *self = new_image
    }
    pub fn new_scaled(
        pixels: impl Iterator<Item = ARGB>,
        old_width: u32,
        old_height: u32,
        new_width: u32,
        new_height: u32,
        scale_kind: ScaleType,
    ) -> PixelImage {
        let old_pixels = pixels
            .map(|color| RGBA::new(color.red(), color.green(), color.blue(), color.alpha()))
            .collect::<Vec<_>>();

        let mut resizer = Resizer::new(
            old_width as usize,
            old_height as usize,
            new_width as usize,
            new_height as usize,
            RGBA8,
            scale_kind,
        )
        .expect("Failed to construct a Resizer");

        let mut new_pixels = vec![RGBA::new(0, 0, 0, 0); new_height as usize * new_width as usize];
        resizer
            .resize(&old_pixels, &mut new_pixels)
            .expect("Failed to resize Image");
        PixelImage {
            pixels: new_pixels
                .iter()
                .map(|color| ARGB::from_rgba(color.r, color.g, color.b, color.a))
                .collect(),
            width: new_width,
            height: new_height,
        }
    }

    pub fn iter_rows_from<'a>(&'a self, start_row: u32) -> IterRows<'a> {
        IterRows {
            image: self,
            row_index: start_row,
        }
    }
}

/// An iterator over the rows of a [`PixelImage`].
pub struct IterRows<'a> {
    image: &'a PixelImage,
    row_index: u32,
}

impl<'a> Iterator for IterRows<'a> {
    type Item = &'a [ARGB];

    fn next(&mut self) -> Option<Self::Item> {
        if self.row_index < self.image.height {
            let start = (self.row_index * self.image.width) as usize;
            let end = start + self.image.width as usize;
            let row = &self.image.pixels[start..end];
            self.row_index += 1;
            Some(row)
        } else {
            None
        }
    }
}

impl<'a> From<BMPImage<'a>> for PixelImage {
    fn from(image: BMPImage<'a>) -> Self {
        let pixels = image.pixels().collect::<Vec<_>>();
        PixelImage::new(pixels, image.width(), image.height())
    }
}
