#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
/// Represents a single pixel
#[repr(C)]
pub struct Pixel {
    blue: u8,
    green: u8,
    red: u8,
    alpha: u8,
}

impl Pixel {
    pub const BLACK: Self = Self::rgb(0, 0, 0);
    pub const WHITE: Self = Self::rgb(0xFF, 0xFF, 0xFF);
    pub const NONE: Self = Self::rgba(0, 0, 0, 0);

    pub const fn r(&self) -> u8 {
        self.red
    }

    pub const fn g(&self) -> u8 {
        self.green
    }

    pub const fn b(&self) -> u8 {
        self.blue
    }

    pub const fn a(&self) -> u8 {
        self.alpha
    }

    /// Applies alpha to the pixel
    pub const fn with_alpha(self, alpha: u8) -> Self {
        let alpha = alpha as u16;
        let red = self.red as u16;
        let green = self.green as u16;
        let blue = self.blue as u16;

        Self {
            blue: (blue * alpha / 255) as u8,
            green: (green * alpha / 255) as u8,
            red: (red * alpha / 255) as u8,
            alpha: alpha as u8,
        }
    }

    /// Constructs a pixel from RGB values
    #[inline(always)]
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self {
            blue: b,
            green: g,
            red: r,
            alpha: 0xFF,
        }
    }

    #[inline(always)]
    /// Construct a Pixel from an RGBA Color
    pub const fn rgba(r: u8, g: u8, b: u8, alpha: u8) -> Self {
        Self::rgb(r, g, b).with_alpha(alpha)
    }

    #[inline(always)]
    /// Construct a Pixel from a hex ARGB Color
    pub const fn hex_rgba(argb: u32) -> Self {
        let alpha = (argb >> 24) as u8;
        let red = (argb >> 16) as u8;
        let green = (argb >> 8) as u8;
        let blue = argb as u8;

        Self::rgba(red, green, blue, alpha)
    }

    #[inline(always)]
    /// Construct a Pixel from a hex RGB Color
    pub const fn hex_rgb(rgb: u32) -> Self {
        let red = (rgb >> 16) as u8;
        let green = (rgb >> 8) as u8;
        let blue = rgb as u8;

        Self::rgb(red, green, blue)
    }

    #[inline]
    pub fn blend_4(top: &[Pixel; 4], bottom: &mut [Pixel; 4]) {
        const ALPHAS_SWIZZLE: wide::u8x16 =
            wide::u8x16::new([3, 3, 3, 3, 7, 7, 7, 7, 11, 11, 11, 11, 15, 15, 15, 15]);

        let u8_top_bytes: &[u8; 16] = unsafe { core::mem::transmute(top) };
        let u8_bottom_bytes: &mut [u8; 16] = unsafe { core::mem::transmute(bottom) };

        let u8_top_cells = wide::u8x16::new(*u8_top_bytes);
        let u8_bottom_cells = wide::u8x16::new(*u8_bottom_bytes);

        let alphas = u8_top_cells.swizzle_relaxed(ALPHAS_SWIZZLE) ^ wide::u8x16::splat(0xFF);

        // 11 11 11 11 15 15 15 15
        let alphas_high = wide::u16x8::from_u8x16_high(alphas);
        // 3 3 3 3 7 7 7 7
        let alphas_low = wide::u16x8::from_u8x16_low(alphas);

        // 11 11 11 11 15 15 15 15
        let bott_high: wide::u16x8 =
            ((wide::u16x8::from_u8x16_high(u8_bottom_cells) * alphas_high) + 0x80)
                .mul_keep_high(0x101.into());

        // 3 3 3 3 7 7 7 7
        let bott_low: wide::u16x8 = ((wide::u16x8::from_u8x16_low(u8_bottom_cells) * alphas_low)
            + 0x80)
            .mul_keep_high(0x101.into());

        let bott = wide::u8x16::narrow_i16x8(unsafe { core::mem::transmute(bott_low) }, unsafe {
            core::mem::transmute(bott_high)
        });

        let res: wide::u8x16 = u8_top_cells + bott;

        *u8_bottom_bytes = res.to_array();
    }
    /// Alpha blends a pixel with another
    pub const fn blend(&self, bottom: &Self) -> Self {
        if self.alpha == 0 {
            return *bottom;
        }

        if bottom.alpha == 0 || self.alpha == 0xFF {
            return *self;
        }

        let top_red = self.red as u16;
        let bottom_red = bottom.red as u16;

        let top_green = self.green as u16;
        let bottom_green = bottom.green as u16;

        let top_blue = self.blue as u16;
        let bottom_blue = bottom.blue as u16;

        let top_alpha = self.alpha as u16;
        let bottom_alpha = bottom.alpha as u16;

        const fn calc_color(top: u16, bottom: u16, alpha: u16) -> u16 {
            top + (bottom * (alpha ^ 0xFF) / 255)
        }

        let red = calc_color(top_red, bottom_red, top_alpha);
        let green = calc_color(top_green, bottom_green, top_alpha);
        let blue = calc_color(top_blue, bottom_blue, top_alpha);
        let alpha = calc_color(top_alpha, bottom_alpha, top_alpha);

        Pixel {
            red: red as u8,
            green: green as u8,
            blue: blue as u8,
            alpha: alpha as u8,
        }
    }
}
