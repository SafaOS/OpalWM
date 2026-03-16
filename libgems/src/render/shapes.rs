use tiny_skia::PathBuilder;

use crate::render::{Point, Shape};

/// Describes a circle shape.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Circle {
    radius: f32,
}

impl Circle {
    pub const fn new(radius: f32) -> Self {
        Self { radius }
    }
}

impl Shape for Circle {
    fn add_to_path(&self, path: &mut PathBuilder, position: Point) {
        path.push_circle(position.x(), position.y(), self.radius);
    }

    fn bounds(&self) -> super::BoundingRect {
        let diameter = self.radius * 2.;
        super::BoundingRect::new(diameter, diameter)
    }
}

/// Describes a rectangle Shape, that may be rounded.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    corner_radius: f32,
    width: f32,
    height: f32,
    offset_x: f32,
    offset_y: f32,
}

impl Rect {
    /// Constructs a new rectangle.
    pub const fn new_rect(width: f32, height: f32) -> Self {
        Self {
            corner_radius: 0.,
            width,
            height,
            offset_x: 0.,
            offset_y: 0.,
        }
    }

    #[inline(always)]
    /// Rounds the rectangles corners.
    pub const fn round(mut self, radius: f32) -> Self {
        self.corner_radius = radius;
        self
    }

    #[inline(always)]
    /// Moves the rectangle by `x` x, and `y` y.
    pub const fn move_by(mut self, x: f32, y: f32) -> Self {
        self.offset_x = self.offset_x + x;
        self.offset_y = self.offset_y + y;
        self
    }
    #[inline(always)]
    /// Sets the render offset of the rectangle to `x` x, and `y` y.
    pub const fn with_offset(mut self, x: f32, y: f32) -> Self {
        self.offset_x = x;
        self.offset_y = y;
        self
    }
}

impl Shape for Rect {
    fn add_to_path(&self, pb: &mut PathBuilder, position: Point) {
        if self.corner_radius == 0. {
            pb.push_rect(
                tiny_skia::Rect::from_xywh(position.x(), position.y(), self.width, self.height)
                    .expect("Invalid Rectangle"),
            );
        } else {
            let x = position.x() + self.offset_x;
            let y = position.y() + self.offset_y;
            let w = self.width;
            let h = self.height;
            let r = self.corner_radius;

            pb.move_to(x + r, y);
            pb.line_to(x + w - r, y);
            pb.quad_to(x + w, y, x + w, y + r);

            pb.line_to(x + w, y + h - r);
            pb.quad_to(x + w, y + h, x + w - r, y + h);

            pb.line_to(x + r, y + h);
            pb.quad_to(x, y + h, x, y + h - r);

            pb.line_to(x, y + r);
            pb.quad_to(x, y, x + r, y);

            pb.close();
        }
    }
    fn bounds(&self) -> super::BoundingRect {
        super::BoundingRect::new(self.width, self.height)
    }
}
