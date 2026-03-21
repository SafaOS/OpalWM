use std::ops::{Add, AddAssign, Deref, DerefMut, Div, Mul, Sub, SubAssign};

pub use libopal::display::Pixel as Color;
use tiny_skia::PathBuilder;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec2((f32, f32));

impl Vec2 {
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

impl Add for Vec2 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x() + rhs.x(), self.y() + rhs.y())
    }
}

impl AddAssign for Vec2 {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for Vec2 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x() - rhs.x(), self.y() - rhs.y())
    }
}

impl SubAssign for Vec2 {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl From<(f32, f32)> for Vec2 {
    fn from(value: (f32, f32)) -> Self {
        Self(value)
    }
}

impl Deref for Vec2 {
    type Target = (f32, f32);
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Div<f32> for Vec2 {
    type Output = Self;
    fn div(self, rhs: f32) -> Self::Output {
        Self::new(self.x() / rhs, self.y() / rhs)
    }
}

impl Mul<f32> for Vec2 {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(self.x() * rhs, self.y() * rhs)
    }
}

/// Represents an (x, y) Point in the view.
pub type Point = Vec2;

/// Represents a (width, height) Rectangle that represents the bounds of a shape
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundingRect(Vec2);

impl BoundingRect {
    #[inline(always)]
    /// Returns true if the given point is within the bounds of the rectangle at `location`.
    pub const fn contains_point(&self, location: Point, point: Point) -> bool {
        let (x, y) = (location.x(), location.y());
        let (px, py) = (point.x(), point.y());

        px >= x && px <= x + self.width() && py >= y && py <= y + self.height()
    }
    pub const fn new(width: f32, height: f32) -> Self {
        Self(Vec2::new(width, height))
    }

    pub const fn width(&self) -> f32 {
        self.0.x()
    }
    pub const fn height(&self) -> f32 {
        self.0.y()
    }

    pub const fn with_width(mut self, width: f32) -> Self {
        self.0.0.0 = width;
        self
    }

    pub const fn with_height(mut self, height: f32) -> Self {
        self.0.0.1 = height;
        self
    }
}

impl Deref for BoundingRect {
    type Target = Vec2;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for BoundingRect {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
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
            min: BoundingRect::new(0., 0.),
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

/// Cross-axis alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Alignment {
    #[default]
    Default,
    Center,
    Start,
    End,
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

    /// Describes equal padding on the left and right.
    #[inline(always)]
    pub const fn lr(padding: f32) -> Self {
        Self {
            left: padding,
            right: padding,
            top: 0.,
            bottom: 0.,
        }
    }

    /// Describes equal padding on the top and bottom.
    #[inline(always)]
    pub const fn tp(padding: f32) -> Self {
        Self {
            left: 0.,
            right: 0.,
            top: padding,
            bottom: padding,
        }
    }

    pub const fn bottom(mut self, bottom: f32) -> Self {
        self.bottom = bottom;
        self
    }

    pub const fn left(mut self, left: f32) -> Self {
        self.left = left;
        self
    }

    pub const fn right(mut self, right: f32) -> Self {
        self.right = right;
        self
    }

    pub const fn padded_width(&self) -> f32 {
        self.left + self.right
    }

    pub const fn padded_height(&self) -> f32 {
        self.top + self.bottom
    }
}
