#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
/// Represents a single ARGB pixel
#[repr(C)]
pub struct ARGB {
    blue: u8,
    green: u8,
    red: u8,
    alpha: u8,
}

impl ARGB {
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

    /// Sets the alpha value of the color, Alpha is straight here and not gay (premultiplied)
    pub const fn with_alpha(self, alpha: u8) -> Self {
        Self {
            blue: self.blue,
            green: self.green,
            red: self.red,
            alpha,
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
    pub const fn from_rgba(r: u8, g: u8, b: u8, alpha: u8) -> Self {
        Self::from_rgb(r, g, b).with_alpha(alpha)
    }
}

impl Into<opal_abi::display::Pixel> for ARGB {
    fn into(self) -> opal_abi::display::Pixel {
        opal_abi::display::Pixel::rgba(self.red(), self.green(), self.blue(), self.alpha())
    }
}
