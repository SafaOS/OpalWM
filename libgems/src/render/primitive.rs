use std::ops::{Add, Deref};

pub use libopal::display::Pixel as Color;
use tiny_skia::PathBuilder;

/// Represents an (x, y) Point in the view.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point((f32, f32));

impl Point {
    pub const fn new(x: f32, y: f32) -> Self {
        Self((x, y))
    }
    pub const fn x(&self) -> f32 {
        self.0.0
    }

    pub const fn y(&self) -> f32 {
        self.0.1
    }
}

impl Add for Point {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x() + rhs.x(), self.y() + rhs.y())
    }
}

impl From<(f32, f32)> for Point {
    fn from(value: (f32, f32)) -> Self {
        Self(value)
    }
}

impl Deref for Point {
    type Target = (f32, f32);
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Represents a (width, height) Rectangle that represents the bounds of a shape
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundingRect {
    width: f32,
    height: f32,
}

impl BoundingRect {
    #[inline(always)]
    /// Returns true if the given point is within the bounds of the rectangle at `location`.
    pub const fn contains_point(&self, location: Point, point: Point) -> bool {
        let (x, y) = (location.x(), location.y());
        let (px, py) = (point.x(), point.y());

        px >= x && px <= x + self.width && py >= y && py <= y + self.height
    }
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    pub const fn width(&self) -> f32 {
        self.width
    }
    pub const fn height(&self) -> f32 {
        self.height
    }
}

/// Represents constraints applied on a shape's bounds, min-max [`BoundingRect`]s.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundingConstraints {
    min: BoundingRect,
    max: BoundingRect,
}

impl BoundingConstraints {
    pub const fn new(min: BoundingRect, max: BoundingRect) -> Self {
        Self { min, max }
    }

    pub const fn from_max(max: BoundingRect) -> Self {
        Self {
            min: BoundingRect {
                width: 0.,
                height: 0.,
            },
            max,
        }
    }

    pub const fn min(&self) -> BoundingRect {
        self.min
    }

    pub const fn max(&self) -> BoundingRect {
        self.max
    }
}

/// Represents a Primitive Shape.
pub trait Shape {
    fn add_to_path(&self, path: &mut PathBuilder, position: Point);
    fn bounds(&self) -> BoundingRect;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Alignment {
    #[default]
    Default,
    Center,
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Padding {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

impl Padding {
    #[inline(always)]
    pub const fn none() -> Self {
        Self {
            left: 0.,
            right: 0.,
            top: 0.,
            bottom: 0.,
        }
    }

    #[inline(always)]
    pub const fn equal(padding: f32) -> Self {
        Self {
            left: padding,
            right: padding,
            top: padding,
            bottom: padding,
        }
    }

    pub const fn padded_width(&self) -> f32 {
        self.left + self.right
    }

    pub const fn padded_height(&self) -> f32 {
        self.top + self.bottom
    }
}
