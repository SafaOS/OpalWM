use std::{
    iter::Sum,
    ops::{Add, Deref, Neg, Sub},
};

use libopal::window::Pixel;

use crate::framebuffer::Framebuffer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DamageRegion {
    pub position: Point,
    pub rect: Rect,
}

impl DamageRegion {
    pub const fn position(&self) -> Point {
        self.position
    }

    pub const fn x(&self) -> isize {
        self.position.x()
    }

    pub const fn y(&self) -> isize {
        self.position.y()
    }

    pub const fn width(&self) -> usize {
        self.rect.width
    }

    pub const fn height(&self) -> usize {
        self.rect.height
    }

    pub const fn max_position(&self) -> Point {
        Point::new(
            self.position.x() + self.rect.width as isize,
            self.position.y() + self.rect.height as isize,
        )
    }
    /// Checks if self overlaps with `rect`.
    #[inline]
    pub fn overlaps_with(&self, rect: &TransformRect) -> Option<IntersectionPoint> {
        self.overlaps_with_within(Point::new(0, 0), rect)
    }

    #[inline]
    /// Checks if self overlaps with `rect` returning the point which is covered from the rectangle within off_within from rect.
    pub fn overlaps_with_within(
        &self,
        off_within: Point,
        rect: &TransformRect,
    ) -> Option<IntersectionPoint> {
        let d0 = self.position();
        let d1 = self.max_position();

        let r0 = rect.position() + off_within;
        let r1 = rect.max_position() - off_within;

        if (d0.x() < r1.x() && d1.x() > r0.x()) && (d0.y() < r1.y() && d1.y() > r0.y()) {
            let i_x0 = d0.x().max(r0.x()) - r0.x();
            let i_y0 = d0.y().max(r0.y()) - r0.y();

            let i_x1 = d1.x().min(r1.x()) - r0.x();
            let i_y1 = d1.y().min(r1.y()) - r0.y();

            Some(IntersectionPoint {
                left_most: Point::new(i_x0, i_y0),
                right_most: Point::new(i_x1, i_y1),
            })
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum WindowDamageReason {
    Moving {
        old: DamageRegion,
        new: DamageRegion,
    },
    Redraw {
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        region: DamageRegion,
    },
    Whole(DamageRegion),
}

impl WindowDamageReason {
    pub fn place_regions(&self, to: &mut Vec<DamageRegion>) {
        match self {
            Self::Whole(w) => to.push(*w),
            Self::Redraw { region, .. } => to.push(*region),
            Self::Moving { old, new } => {
                to.reserve(2);
                to.push(*old);
                to.push(*new);
            }
        }
    }
}

/// Rectangle
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub width: usize,
    pub height: usize,
}
impl Rect {
    pub const fn new(width: usize, height: usize) -> Self {
        Self { width, height }
    }

    pub const fn width(&self) -> usize {
        self.width
    }

    pub const fn height(&self) -> usize {
        self.height
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntersectionPoint {
    left_most: Point,
    right_most: Point,
}

impl IntersectionPoint {
    pub const fn none() -> Self {
        Self {
            left_most: Point::new(0, 0),
            right_most: Point::new(0, 0),
        }
    }

    pub fn width(&self) -> usize {
        let (top_x, _) = *self.left_most;
        let (bott_x, _) = *self.right_most;
        bott_x.abs_diff(top_x)
    }

    pub fn height(&self) -> usize {
        let (_, top_y) = *self.left_most;
        let (_, bott_y) = *self.right_most;
        bott_y.abs_diff(top_y)
    }

    /// Returns the x-coordinate of the intersection point, from the top-left corner.
    pub const fn x(&self) -> isize {
        self.left_most.x()
    }

    /// Returns the y-coordinate of the intersection point, from the top-left corner.
    pub const fn y(&self) -> isize {
        self.left_most.y()
    }
}

impl Add<IntersectionPoint> for IntersectionPoint {
    type Output = IntersectionPoint;
    fn add(self, rhs: IntersectionPoint) -> Self::Output {
        let (s_top_x, s_top_y) = *self.left_most;
        let (o_top_x, o_top_y) = *rhs.left_most;
        let (s_bott_x, s_bott_y) = *self.right_most;
        let (o_bott_x, o_bott_y) = *rhs.right_most;
        Self {
            left_most: Point::new(s_top_x.min(o_top_x), s_top_y.min(o_top_y)),
            right_most: Point::new(s_bott_x.max(o_bott_x), s_bott_y.max(o_bott_y)),
        }
    }
}

impl Sum for IntersectionPoint {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        let mut results = IntersectionPoint::none();

        for i in iter {
            if results == IntersectionPoint::none() {
                results = i;
            } else {
                results = results + i;
            }
        }

        results
    }
}

impl Add for Point {
    type Output = Point;
    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x() + rhs.x(), self.y() + rhs.y())
    }
}

impl Neg for Point {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Self::new(-self.x(), -self.y())
    }
}

impl Sub for Point {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        self + -rhs
    }
}

/// Rectangle's Position or a point within it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point(pub (isize, isize));

impl Point {
    pub const fn new(x: isize, y: isize) -> Self {
        Self((x, y))
    }

    pub const fn x(&self) -> isize {
        self.0.0
    }

    pub const fn y(&self) -> isize {
        self.0.1
    }
}

impl Deref for Point {
    type Target = (isize, isize);
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Same as [`Point`] but unsigned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UPoint(pub (usize, usize));

impl UPoint {
    pub const fn new(x: usize, y: usize) -> Self {
        Self((x, y))
    }

    pub const fn x(&self) -> usize {
        self.0.0
    }

    pub const fn y(&self) -> usize {
        self.0.1
    }
}

impl Deref for UPoint {
    type Target = (usize, usize);
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TransformRect {
    pub rect: Rect,
    pub position: Point,
    pub damage_reason: Option<WindowDamageReason>,
}

impl TransformRect {
    pub const fn new(position: Point, rect: Rect) -> Self {
        Self {
            rect,
            position,
            damage_reason: None,
        }
    }

    pub const fn position(&self) -> Point {
        self.position
    }

    pub const fn set_pos(&mut self, point: Point) {
        self.position = point;
    }

    pub fn max_position(&self) -> Point {
        self.position + Point::new(self.rect.width as isize, self.rect.height as isize)
    }

    pub const fn x(&self) -> isize {
        self.position.x()
    }

    pub const fn y(&self) -> isize {
        self.position.y()
    }

    pub const fn width(&self) -> usize {
        self.rect.width
    }

    pub const fn height(&self) -> usize {
        self.rect.height
    }

    pub const fn max_x(&self) -> isize {
        self.x() + self.width() as isize
    }

    pub const fn max_y(&self) -> isize {
        self.y() + self.height() as isize
    }

    pub const fn get_whole_damage(&self) -> DamageRegion {
        DamageRegion {
            rect: self.rect,
            position: self.position,
        }
    }

    pub fn moved(&mut self, prev: DamageRegion, new: DamageRegion) {
        self.update_damage(WindowDamageReason::Moving { old: prev, new });
    }

    // /// Returns the damage a window may have caused on the framebuffer, if it's position or dimensions changed
    // /// There is 2 damages: The damage before the operation, The damage after the operation
    // pub fn damage_whole(&mut self, get_whole_damage: impl FnOnce() -> DamageRegion) {
    //     self.update_damage(WindowDamageReason::Whole(region));
    // }

    /// Returns the potential damage to be done on given corridations inside of the window
    fn get_damage_within(&self, x: usize, y: usize, width: usize, height: usize) -> DamageRegion {
        let x = x.min(self.width());
        let y = y.min(self.height());

        let pos_x = (self.x().saturating_add_unsigned(x)).min(self.max_x());
        let pos_y = (self.y().saturating_add_unsigned(y)).min(self.max_y());
        let width = width.min(self.width() - x);
        let height = height.min(self.height() - y);

        DamageRegion {
            position: Point::new(pos_x, pos_y),
            rect: Rect { width, height },
        }
    }

    pub fn damage_within(&mut self, x: usize, y: usize, width: usize, height: usize) {
        self.update_damage(WindowDamageReason::Redraw {
            x,
            y,
            width,
            height,
            region: self.get_damage_within(x, y, width, height),
        });
    }

    /// Take the damage reason of self, places None in place of it
    pub const fn take_damage(&mut self) -> Option<WindowDamageReason> {
        self.damage_reason.take()
    }

    /// Updates the current damage reason
    pub fn update_damage(&mut self, reason: WindowDamageReason) {
        let new_reason = match (self.damage_reason, reason) {
            // Moving take priority, the newer the more prior
            (
                Some(WindowDamageReason::Moving { old, .. }),
                WindowDamageReason::Moving { new, .. },
            ) => WindowDamageReason::Moving { old, new },
            (Some(_), WindowDamageReason::Moving { .. }) => reason,
            (Some(r @ WindowDamageReason::Moving { .. }), _) => r,
            // Then comes whole damage
            (Some(_), WindowDamageReason::Whole(_)) => reason, /* both work */
            (Some(r @ WindowDamageReason::Whole(_)), _) => r,
            // Then finally we combine redraws
            (
                Some(WindowDamageReason::Redraw {
                    x: s_x,
                    y: s_y,
                    width: s_wid,
                    height: s_hei,
                    region: _,
                }),
                WindowDamageReason::Redraw {
                    x: o_x,
                    y: o_y,
                    width: o_wid,
                    height: o_hei,
                    region: _,
                },
            ) => {
                let n_x = s_x.min(o_x);
                let n_y = s_y.min(o_y);
                let max_n_x = s_x.max(o_x);
                let max_n_y = s_y.max(o_y);

                let diff_x = max_n_x - n_x;
                let diff_y = max_n_y - n_y;
                let width = s_wid.max(o_wid) + diff_x;
                let height = s_hei.max(o_hei) + diff_y;

                let n_r = self.get_damage_within(n_x, n_y, width, height);
                WindowDamageReason::Redraw {
                    x: n_x,
                    y: n_y,
                    width: width,
                    height: height,
                    region: n_r,
                }
            }
            (None, reason) => reason,
        };

        self.damage_reason = Some(new_reason);
    }

    /// Draws the whole window without syncing the results to the real framebuffer.
    ///
    /// [`fb.sync_pixels_rect`] must be called afterwards on the area the window is in.
    pub fn draw(&self, fb: &mut Framebuffer, pixels: &[Pixel]) {
        fb.draw_rect(self.x(), self.y(), self.width(), self.height(), pixels);
    }

    /// Draws the window from intersection point without syncing the results to the real framebuffer.
    ///
    /// [`fb.sync_pixels_rect`] must be called afterwards on the area the window is in.
    pub fn draw_at(&self, fb: &mut Framebuffer, point: IntersectionPoint, pixels: &[Pixel]) {
        let (top_x_within, top_y_within) = *point.left_most;
        assert!(top_x_within >= 0);
        assert!(top_y_within >= 0);

        let width = point.width();
        let height = point.height();

        let pixels_width = self.width();
        let pixels_height = self.height();

        if width == pixels_width && height == pixels_height {
            return self.draw(fb, pixels);
        }

        // The offset within the FB is the offset of self + the point
        let off_x = self.x() + top_x_within;
        let off_y = self.y() + top_y_within;

        // We want to draw pixels that `point` cover only
        fb.draw_rect_within(
            off_x,
            off_y,
            width,
            height,
            pixels,
            pixels_width,
            pixels_height,
            top_x_within as usize,
            top_y_within as usize,
        );
    }
}
