use thiserror::Error;
use zerocopy::FromBytes;
use zerocopy_derive::{FromBytes, Immutable, KnownLayout, Unaligned};

use crate::display::ARGB;

/// The header located at the start of the bitmap
#[derive(Debug, Clone, Copy, FromBytes, Immutable, Unaligned, KnownLayout)]
#[repr(C, packed)]
struct BMPHeader {
    // should be BM
    magic: [u8; 2],
    size: u32,
    _reserved0: [u8; 2],
    _reserved1: [u8; 2],
    pixels_off: u32,
}

const _: () = assert!(size_of::<BMPHeader>() == 14);

const COMPRESS_B_BITFIELDS: u32 = 0x3;
const COMPRESS_B_RGB: u32 = 0x0;

/// BITMAPINFOHEADER
/// Located after [`BMPHeader`]
#[derive(FromBytes, Immutable, Debug)]
#[repr(C)]
struct DIBHeader {
    size: u32,
    width: i32,
    height: i32,
    color_panels_num: u16,
    bpp: u16,
    compression: u32,
    image_size: u32,
    horizontal_resolution: i32,
    vertical_resolution: i32,
    color_platte_colors: u32,
    important_colors: u32,
}

const _: () = assert!(size_of::<DIBHeader>() == 40);

#[derive(Debug, Clone, Copy, Immutable, FromBytes, KnownLayout)]
#[repr(C)]
struct BMPBitmasks {
    red_channel: u32,
    green_channel: u32,
    blue_channel: u32,
    alpha_channel: u32,
}

impl Default for BMPBitmasks {
    fn default() -> Self {
        // RGB
        Self {
            red_channel: 0xFF0000,
            green_channel: 0x00FF00,
            blue_channel: 0x0000FF,
            alpha_channel: 0xFF000000,
        }
    }
}

#[derive(Debug, Clone, Copy, Error)]
pub enum BMPParseError {
    #[error("Bad BMP Header, unsupported magic value")]
    UnsupportedMagic,
    #[error("Bad BMP File Size")]
    InvalidSize,
    #[error("Unsupported Bits Per Pixel amount")]
    UnsupportedBPP,
    #[error("Unsupported compression kind")]
    UnsupportedComperssion,
    #[error("BMP File corrupted")]
    Corrupted,
    #[error("Unsupported, reason {0}")]
    Unsupported(&'static str),
}

/// A Parsed BMP Image
pub struct BMPImage<'a> {
    bitmasks: BMPBitmasks,
    width: u32,
    height: u32,
    bpp: u16,
    pixels: &'a [u8],
}

impl<'a> BMPImage<'a> {
    pub const fn width(&self) -> usize {
        self.width as usize
    }

    pub const fn height(&self) -> usize {
        self.height as usize
    }
    /// Prase a BMP Image from a given byte slice
    pub fn from_slice(slice: &'a [u8]) -> Result<Self, BMPParseError> {
        let mut curr = slice;
        let mut take_from_slice = |size: usize| {
            if curr.len() < size {
                Err(BMPParseError::InvalidSize)
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

        let header = take_from_slice(size_of::<BMPHeader>())?;
        let header: &BMPHeader =
            BMPHeader::ref_from_bytes(header).expect("getting a ref to BMP should never fail");

        if header.magic != *b"BM" {
            return Err(BMPParseError::UnsupportedMagic);
        }

        if slice.len() < header.size as usize {
            return Err(BMPParseError::Corrupted);
        }

        let dib_header_bytes = take_from_slice(size_of::<DIBHeader>())?;
        let dib_header: DIBHeader = DIBHeader::read_from_bytes(dib_header_bytes)
            .expect("reading DIBHeader should never fail");

        if dib_header.color_platte_colors != 0 {
            return Err(BMPParseError::Unsupported("Color tables are unsupported"));
        }

        if dib_header.bpp != 32 && dib_header.bpp != 24 {
            return Err(BMPParseError::UnsupportedBPP);
        }

        let unread = dib_header.size - size_of::<DIBHeader>() as u32;

        let bitmasks = match dib_header.compression {
            COMPRESS_B_BITFIELDS if unread as usize >= size_of::<BMPBitmasks>() => {
                let bitmasks_bytes = take_from_slice(size_of::<BMPBitmasks>())?;
                take_from_slice(unread as usize - bitmasks_bytes.len())?;

                let bitmasks: BMPBitmasks =
                    BMPBitmasks::read_from_bytes(bitmasks_bytes).expect("Should never fail");

                Some(bitmasks)
            }
            COMPRESS_B_RGB => {
                take_from_slice(unread as usize)?;
                None
            }
            _ => return Err(BMPParseError::UnsupportedComperssion),
        };

        let bitmasks = bitmasks.unwrap_or_default();

        let bpp = dib_header.bpp;
        let width = dib_header.width;
        let height = dib_header.height;

        if width.is_negative() {
            return Err(BMPParseError::Unsupported("Negative Width"));
        }

        if height.is_negative() {
            return Err(BMPParseError::Unsupported("Negative Height"));
        }

        let pixels = (width * height) as u32;

        let pixels_bytes = pixels * ((bpp / 8) as u32);

        if slice.len() < (header.pixels_off + pixels_bytes) as usize {
            return Err(BMPParseError::InvalidSize);
        }

        let pixels =
            &slice[header.pixels_off as usize..(header.pixels_off + pixels_bytes) as usize];
        Ok(Self {
            bitmasks,
            width: width as u32,
            height: height as u32,
            bpp,
            pixels,
        })
    }

    /// Returns an iterator of the pixels in the parsed BMP Image
    pub fn pixels(&'a self) -> BMPPixels<'a> {
        BMPPixels {
            image: self,
            bytes_per_pixel: self.bpp / 8,
            row_off: self.height.saturating_sub(1) as usize,
            col_off: 0,
        }
    }
}

/// An iterator over the pixels in a BMP Image
pub struct BMPPixels<'a> {
    image: &'a BMPImage<'a>,
    bytes_per_pixel: u16,
    /* pixels are stored from bottom to top */
    row_off: usize,
    col_off: usize,
}

impl<'a> Iterator for BMPPixels<'a> {
    type Item = ARGB;
    fn next(&mut self) -> Option<Self::Item> {
        if self.image.height == 0 || self.image.width == 0 {
            return None;
        }

        let width = self.image.width as usize;
        if self.col_off >= width {
            self.col_off = 0;
            self.row_off = self.row_off.checked_sub(1)?;
        }

        let bytes_per_pixels = self.bytes_per_pixel as usize;
        let start = (self.col_off * bytes_per_pixels) + (self.row_off * width * bytes_per_pixels);
        let end = start + bytes_per_pixels;

        let pixel_bytes = &self.image.pixels[start..end];
        self.col_off += 1;

        let mut pixel_u32_bytes: [u8; 4] = [0xFFu8; 4];
        match pixel_bytes.len() {
            4 => pixel_u32_bytes.copy_from_slice(pixel_bytes),
            3 => pixel_u32_bytes[..3].copy_from_slice(pixel_bytes),
            _ => unimplemented!("{} BPP Not yet implemented", self.image.bpp),
        }

        let pixel_as_u32 = u32::from_le_bytes(pixel_u32_bytes);

        let bitmasks = &self.image.bitmasks;

        let red = (pixel_as_u32 & bitmasks.red_channel) >> bitmasks.red_channel.trailing_zeros();
        let green =
            (pixel_as_u32 & bitmasks.green_channel) >> bitmasks.green_channel.trailing_zeros();
        let blue = (pixel_as_u32 & bitmasks.blue_channel) >> bitmasks.blue_channel.trailing_zeros();
        let alpha =
            (pixel_as_u32 & bitmasks.alpha_channel) >> bitmasks.alpha_channel.trailing_zeros();

        Some(ARGB::from_rgba(
            red as u8,
            green as u8,
            blue as u8,
            alpha as u8,
        ))
    }
}

impl<'a> ExactSizeIterator for BMPPixels<'a> {
    fn len(&self) -> usize {
        self.image.pixels.len() / self.bytes_per_pixel as usize
    }
}
