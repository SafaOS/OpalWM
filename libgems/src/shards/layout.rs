use crate::{BoundingRect, Padding};

/// Cross-axis alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AxisAlign {
    /// The default align depending on the parent container.
    #[default]
    Default,
    /// Align to the center.
    Center,
    /// Align to the start axis.
    Start,
    /// Align to the end axis.
    End,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
/// Defines the layout of a [`Shard`].
pub struct ShardLayout {
    pub bounds: BoundingRect,
    pub padding: Padding,
    pub align: AxisAlign,
}

impl ShardLayout {
    pub const fn from_bounds(bounds: BoundingRect) -> Self {
        Self {
            bounds,
            padding: Padding::none(),
            align: AxisAlign::Default,
        }
    }

    /// returns the bounds with padding.
    pub const fn full_bounds(&self) -> BoundingRect {
        BoundingRect::new(
            self.bounds.width() + self.padding.padded_width(),
            self.bounds.height() + self.padding.padded_height(),
        )
    }
}
