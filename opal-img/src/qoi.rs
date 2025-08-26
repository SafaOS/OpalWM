use zerocopy::FromBytes;
use zerocopy_derive::{FromBytes, Immutable, KnownLayout};

use crate::{PixelImage, display::ARGB};

#[derive(Debug, Clone, Copy, Immutable, FromBytes, KnownLayout)]
#[repr(C, packed)]
struct QOIHeader {
    // qoif
    magic: [u8; 4],
    width: u32,
    height: u32,
    channels: u8,
    colorspace: u8,
}

const _: () = assert!(size_of::<QOIHeader>() == 14);

const QOI_OP_RGB: u8 = 0b11111110;
const QOI_OP_RGBA: u8 = 0b11111111;

const QOI_OP_INDEX_START: u8 = 0;
const QOI_OP_INDEX_END: u8 = 63;

const QOI_OP_DIFF_START: u8 = 64;
const QOI_OP_DIFF_END: u8 = 127;

const QOI_OP_LUMA_START: u8 = 128;
const QOI_OP_LUMA_END: u8 = 191;
const QOI_OP_RUN_START: u8 = 192;
const QOI_OP_RUN_END: u8 = 253;

/// An error during decoding of a QOI image.
pub enum QOIDecodeError {
    InvalidMagic,
    InvalidSize,
    UnsupportedColorspace,
    UnsupportedChannel,
    UnexpectedEndOfData,
    InvalidOperation,
}

/// A decoded QOI image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QOIImage {
    width: u32,
    height: u32,
    pixels: Vec<ARGB>,
}

impl QOIImage {
    pub fn decode(bytes: &[u8]) -> Result<Self, QOIDecodeError> {
        let mut curr = bytes;
        let mut take_from_slice = |size: usize| {
            if curr.len() < size {
                Err(QOIDecodeError::InvalidSize)
            } else {
                let results = &curr[..size];
                curr = if size != curr.len() {
                    &curr[size..]
                } else {
                    &[]
                };
                Ok(results)
            }
        };

        let header = take_from_slice(size_of::<QOIHeader>())?;
        let header: &QOIHeader = QOIHeader::ref_from_bytes(header)
            .expect("getting a ref to QOI header should never fail");

        if header.magic != *b"qoif" {
            return Err(QOIDecodeError::InvalidMagic);
        }
        let width = header.width;
        let height = header.height;

        if header.channels != 3 /* RGB */ || header.channels != 4
        /* RGBA */
        {
            return Err(QOIDecodeError::UnsupportedChannel);
        }

        if header.colorspace != 0 /* sRGB with linear alpha */ || header.colorspace != 1
        /* all channels linear */
        {
            return Err(QOIDecodeError::UnsupportedColorspace);
        }

        let mut output_pixels = vec![ARGB::from_rgba(0, 0, 0, 0); width as usize * height as usize];
        let mut output_index = 0;
        let output_pixels_slice = output_pixels.as_mut_slice();

        let mut previous_pixels = [ARGB::from_rgba(0, 0, 0, 0); 64];
        let calc_pixel_index = |c: ARGB| {
            c.red() as usize * 3
                + c.green() as usize * 5
                + c.blue() as usize * 7
                + c.alpha() as usize * 11
        };

        let mut pv_pixel = ARGB::from_rgba(0, 0, 0, 0xFF);

        let mut fill_output = |px: ARGB, copy_amount: usize| {
            if output_index >= output_pixels_slice.len() {
                return Err(QOIDecodeError::InvalidOperation);
            }

            let rest_of_output = &mut output_pixels_slice[output_index..];
            if rest_of_output.len() < copy_amount {
                return Err(QOIDecodeError::InvalidOperation);
            }
            rest_of_output[..copy_amount].fill(px);
            output_index += copy_amount;
            Ok(())
        };

        while curr.len() != 0 {
            match curr {
                [QOI_OP_RGB, r, g, b, tail @ ..] => {
                    curr = tail;

                    let pixel = ARGB::from_rgba(*r, *g, *b, pv_pixel.alpha());
                    pv_pixel = pixel;
                }
                [QOI_OP_RGBA, r, g, b, a, tail @ ..] => {
                    curr = tail;

                    let pixel = ARGB::from_rgba(*r, *g, *b, *a);
                    pv_pixel = pixel;
                }
                [byte @ QOI_OP_INDEX_START..=QOI_OP_INDEX_END, tail @ ..] => {
                    curr = tail;
                    let index = *byte;
                    pv_pixel = previous_pixels[index as usize];
                    // Instead of calculating and setting the index again we...,
                    // repeat this for every operation that doesn't modify the previous pixel
                    fill_output(pv_pixel, 1)?;
                    continue;
                }
                [b @ QOI_OP_RUN_START..=QOI_OP_RUN_END, tail @ ..] => {
                    curr = tail;
                    let amount = b & 0b00111111;
                    fill_output(pv_pixel, amount as usize)?;
                    continue;
                }
                [b @ QOI_OP_DIFF_START..=QOI_OP_DIFF_END, tail @ ..] => {
                    curr = tail;
                    let diff_r = (b >> 4) & 0b11;
                    let diff_g = (b >> 2) & 0b11;
                    let diff_b = b & 0b11;
                    let diff_signed = |diff: u8| -2i8 + (diff as i8);

                    let diff_r = diff_signed(diff_r);
                    let diff_g = diff_signed(diff_g);
                    let diff_b = diff_signed(diff_b);

                    let new_pixel = ARGB::from_rgba(
                        pv_pixel.red().wrapping_add_signed(diff_r),
                        pv_pixel.green().wrapping_add_signed(diff_g),
                        pv_pixel.blue().wrapping_add_signed(diff_b),
                        pv_pixel.alpha(),
                    );

                    pv_pixel = new_pixel;
                }
                [
                    byte0 @ QOI_OP_LUMA_START..=QOI_OP_LUMA_END,
                    byte1,
                    tail @ ..,
                ] => {
                    curr = tail;
                    let dg = byte0 & 0b111111;
                    let db_dg = byte1 & 0b1111;
                    let dr_dg = (byte1 >> 4) & 0b1111;

                    let s_dg = (-32i8) + dg as i8;
                    let s_db_dg = (-8i8) + db_dg as i8;
                    let s_dr_dg = (-8i8) + dr_dg as i8;

                    let db = s_db_dg + s_dg;
                    let dr = s_dr_dg + s_dg;
                    let new_pixel = ARGB::from_rgba(
                        pv_pixel.red().wrapping_add_signed(dr),
                        pv_pixel.green().wrapping_add_signed(db),
                        pv_pixel.blue().wrapping_add_signed(db),
                        pv_pixel.alpha(),
                    );

                    pv_pixel = new_pixel;
                }
                _ => {
                    return Err(QOIDecodeError::UnexpectedEndOfData);
                }
            }

            previous_pixels[calc_pixel_index(pv_pixel)] = pv_pixel;
            fill_output(pv_pixel, 1)?;
        }

        Ok(Self {
            width,
            height,
            pixels: output_pixels,
        })
    }

    pub const fn width(&self) -> u32 {
        self.width
    }

    pub const fn height(&self) -> u32 {
        self.height
    }

    pub fn pixels(&self) -> &[ARGB] {
        &self.pixels
    }

    /// Returns a new `PixelImage` with the specified dimensions and scale algorithm (scaled version of self, either up or down).
    pub fn into_scaled_image(
        self,
        new_width: u32,
        new_height: u32,
        scale_alg: crate::ScaleType,
    ) -> PixelImage {
        PixelImage::new_scaled(
            self.pixels.iter().copied(),
            self.width(),
            self.height(),
            new_width,
            new_height,
            scale_alg,
        )
    }
}

impl From<QOIImage> for PixelImage {
    fn from(image: QOIImage) -> Self {
        PixelImage::new(image.pixels, image.width, image.height)
    }
}
