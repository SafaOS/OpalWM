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
    pub const BLACK: Self = Self {
        blue: 0,
        green: 0,
        red: 0,
        alpha: 0xFF,
    };

    pub const WHITE: Self = Self {
        blue: 255,
        green: 255,
        red: 255,
        alpha: 0xFF,
    };

    pub const fn red(&self) -> u8 {
        self.red
    }

    pub const fn green(&self) -> u8 {
        self.green
    }

    pub const fn blue(&self) -> u8 {
        self.blue
    }

    pub const fn alpha(&self) -> u8 {
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
    pub const fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self {
            blue: b,
            green: g,
            red: r,
            alpha: 0xFF,
        }
    }

    #[inline(always)]
    /// Construct a Pixel from an RGBA Color
    pub const fn from_rgb_with_alpha(r: u8, g: u8, b: u8, alpha: u8) -> Self {
        Self::from_rgb(r, g, b).with_alpha(alpha)
    }

    #[inline(always)]
    /// Construct a Pixel from a hex ARGB Color
    pub const fn from_hex_argb(argb: u32) -> Self {
        let alpha = (argb >> 24) as u8;
        let red = (argb >> 16) as u8;
        let green = (argb >> 8) as u8;
        let blue = argb as u8;

        Self::from_rgb_with_alpha(red, green, blue, alpha)
    }

    #[inline(always)]
    /// Construct a Pixel from a hex RGB Color
    pub const fn from_hex_rgb(rgb: u32) -> Self {
        let red = (rgb >> 16) as u8;
        let green = (rgb >> 8) as u8;
        let blue = rgb as u8;

        Self::from_rgb(red, green, blue)
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
            top + (bottom * (255 - alpha) / 255)
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
