use libopal::window::Pixel;

use crate::{
    framebuffer::Framebuffer,
    window::primitive::{DamageRegion, IntersectionPoint, Point, Rect, TransformRect},
};

#[derive(Debug, Clone, Copy)]
pub enum DrawVerb {
    DrawPoint(Point, Pixel),
    DrawRect(Point, Rect, Pixel),
}

#[derive(Debug, Clone)]
struct CachedDrawing {
    verbs: Vec<DrawVerb>,
}

impl CachedDrawing {
    const fn new() -> Self {
        Self { verbs: Vec::new() }
    }
    fn add_area(&mut self, at: Point, rect: Rect) {
        _ = at;
        _ = rect;
    }
    fn add_point(&mut self, at: Point, color: Pixel) {
        self.verbs.push(DrawVerb::DrawPoint(at, color));
        self.add_area(at, Rect::new(1, 1));
    }

    fn add_rect(&mut self, point: Point, rect: Rect, color: Pixel) {
        for verb in &mut self.verbs {
            match verb {
                DrawVerb::DrawRect(s_point, s_rect, s_color) => {
                    if color != *s_color {
                        continue;
                    }

                    let s_x_end = Point::new(s_point.x() + s_rect.width as isize, s_point.y());
                    let s_y_end = Point::new(s_point.x(), s_point.y() + s_rect.height as isize);

                    if s_x_end == point && s_rect.height == rect.height {
                        s_rect.width += rect.width;
                        return;
                    }

                    if s_y_end == point && s_rect.width == rect.width {
                        s_rect.height += rect.height;
                        return;
                    }
                }
                _ => {}
            }
        }

        self.verbs.push(DrawVerb::DrawRect(point, rect, color));
        self.add_area(point, rect);
    }

    fn add_rect_points(&mut self, point0: Point, point1: Point, color: Pixel) {
        let f_x = point0.x().min(point1.x());
        let f_y = point0.y().min(point1.y());

        let l_x = point0.x().max(point1.x());
        let l_y = point0.y().max(point1.y());

        let width = (l_x - f_x + 1) as usize;
        let height = (l_y - f_y + 1) as usize;
        if width == 0 || height == 0 {
            return;
        }

        if point0 == point1 {
            return self.add_point(point0, color);
        }

        let point = Point::new(f_x, f_y);
        let rect = Rect::new(width, height);

        return self.add_rect(point, rect, color);
    }

    fn add_round_corners(
        &mut self,
        to_rect: Rect,
        radius: u32,
        corner_mask: (bool, bool, bool, bool),
        color: Option<Pixel>,
        connect_with: Option<Pixel>,
    ) {
        let radius = radius as isize;
        // Draws two corners of a rounded rectangle, and then connects them with a line of a color got by the function get_color
        let mut draw_2corners =
            |s_x0: isize, s_x1: isize, s_y: isize, top: bool, mask: (bool, bool)| {
                let (draw_left, draw_right) = mask;
                let x0 = s_x0 + (radius * 2);
                let x1 = s_x1 + (radius * 2);

                let y = s_y + (radius * 2);

                let mut f = 1 - radius as i32;
                let mut ddf_x = 1;
                let mut ddf_y = -2 * radius as i32;

                let mut xx = 0isize;
                let mut yy = radius as isize;

                while xx < yy {
                    let last_xx = xx;
                    let last_yy = yy;

                    if f >= 0 {
                        yy -= 1;
                        ddf_y += 2;
                        f += ddf_y;
                    }

                    xx += 1;
                    ddf_x += 2;
                    f += ddf_x;

                    let radius = radius as isize;
                    let draw_y = if !top {
                        y + yy - radius
                    } else {
                        y - yy - radius
                    };
                    let draw_y_flipped = if !top {
                        y + xx - radius
                    } else {
                        y - xx - radius
                    };

                    let d_x0 = x0 - xx - radius;
                    let point0 = Point::new(d_x0, draw_y as isize);
                    let f_d_x0 = x0 - yy - radius;
                    let point0_flipped = Point::new(f_d_x0, draw_y_flipped as isize);

                    let d_x1 = x1 + xx - radius;
                    let point1 = Point::new(d_x1, draw_y as isize);
                    let f_d_x1 = x1 + yy - radius;
                    let point1_flipped = Point::new(f_d_x1, draw_y_flipped as isize);

                    if draw_left {
                        if let Some(color) = color {
                            self.add_point(point0, color);
                            self.add_point(point0_flipped, color);
                        } else {
                            // Bottom or Top left corner
                            self.add_rect_points(Point::new(s_x0, draw_y), point0, Pixel::NONE);
                            self.add_rect_points(
                                Point::new(s_x0, draw_y_flipped),
                                point0_flipped,
                                Pixel::NONE,
                            );
                        }
                    }

                    if draw_right {
                        if let Some(color) = color {
                            self.add_point(point1, color);
                            self.add_point(point1_flipped, color);
                        } else {
                            // Bottom or Top right corner
                            self.add_rect_points(Point::new(x1, draw_y), point1, Pixel::NONE);
                            self.add_rect_points(
                                Point::new(x1, draw_y_flipped),
                                point1_flipped,
                                Pixel::NONE,
                            );
                        }

                        if let Some(connect_with) = connect_with {
                            // Draw the fill
                            // not flipped
                            if yy != last_yy {
                                self.add_rect_points(point0, point1, connect_with);
                            }

                            // flipped
                            if xx != last_xx {
                                self.add_rect_points(point0_flipped, point1_flipped, connect_with);
                            }
                        }
                        // Bottom or Top right corner
                        // self.add_rect_points(Point::new(x1, draw_y), point1, color);
                        // self.add_rect_points(Point::new(x1, draw_y_flipped), point1_flipped, color);
                    }
                }
            };
        let x0 = 0;
        let y0 = 0;
        let x1 = (x0 + to_rect.width as isize) - 1;
        let y1 = (y0 + to_rect.height as isize) - 1;

        let (left, right, top, bottom) = corner_mask;
        if top {
            draw_2corners(x0, x1 - (radius * 2), y0, true, (left, right));
        }
        if bottom {
            draw_2corners(
                x0,
                x1 - (radius * 2),
                y1 - (radius * 2),
                false,
                (left, right),
            );
        }
    }

    fn apply_on(&self, bounds: Rect, pixels: &mut [Pixel]) {
        for verb in &*self.verbs {
            match verb {
                DrawVerb::DrawPoint(at, pixel) => {
                    let index = (at.y() * bounds.width as isize) + at.x();
                    pixels[index as usize] = *pixel;
                }
                DrawVerb::DrawRect(at, rect, pixel) => {
                    let x = at.x() as usize;
                    let y = at.y() as usize;

                    let width = rect.width.min(bounds.width.saturating_sub(x));
                    let height = rect.height.min(bounds.height.saturating_sub(y));
                    for row in 0..height {
                        let start_index = ((y + row) * bounds.width) as usize + x;
                        let end_index = start_index + width as usize;
                        pixels[start_index..end_index].fill(*pixel);
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
struct WindowAttachment {
    /// Placement from within the window
    placement: Point,
    bounds: Rect,
    damage_bounds: [(Point, Rect); 4],
    pixels: Box<[Pixel]>,
    mask: Vec<(Point, Rect)>,
}

impl WindowAttachment {
    const fn damage(&self, offset_from: Point) -> DamageRegion {
        let position = Point::new(
            self.placement.x() + offset_from.x(),
            self.placement.y() + offset_from.y(),
        );
        DamageRegion {
            position,
            rect: self.bounds,
        }
    }
    pub fn new_border_for(win_bounds: &Rect, radius: u32, border_color: Pixel) -> Self {
        let bounds = Rect::new(win_bounds.width + 2, win_bounds.height + 2);
        let diameter = (radius * 2) as usize;
        let mut rendering = CachedDrawing::new();

        rendering.add_rect(
            Point::new(0, radius as isize),
            Rect::new(1, bounds.height - diameter),
            border_color,
        );
        rendering.add_rect(
            Point::new(bounds.width as isize - 1, radius as isize),
            Rect::new(1, bounds.height - diameter),
            border_color,
        );

        rendering.add_rect(
            Point::new(radius as isize, 0),
            Rect::new(bounds.width - diameter, 1),
            border_color,
        );

        rendering.add_rect(
            Point::new(radius as isize, bounds.height as isize - 1),
            Rect::new(bounds.width - diameter, 1),
            border_color,
        );
        rendering.add_round_corners(
            bounds,
            radius,
            (true, true, true, true),
            Some(border_color),
            None,
        );

        let mut pixels = vec![Pixel::NONE; bounds.width * bounds.height].into_boxed_slice();
        rendering.apply_on(bounds, &mut pixels);

        let mut removal_mask = Vec::new();

        let mut f = |start: usize, end: usize| {
            for row in 0..bounds.height {
                let iter: &mut dyn Iterator<Item = usize> = if start < end {
                    &mut (start..end)
                } else {
                    &mut (end..=start).rev()
                };

                for col in iter {
                    let index = (row * bounds.width) + col;
                    if pixels[index].a() != 0 {
                        if col != start {
                            let rect = Rect::new(col.abs_diff(start), 1);
                            let point = Point::new(col.min(start) as isize, row as isize);
                            removal_mask.push((point, rect));
                        }

                        break;
                    }
                }
            }
        };

        f(0, bounds.width);
        f(bounds.width - 1, 0);

        Self {
            placement: Point::new(-1, -1),
            bounds,
            damage_bounds: [
                (Point::new(0, 0), Rect::new(diameter, bounds.height)),
                (Point::new(0, 0), Rect::new(bounds.width, diameter)),
                (
                    Point::new((bounds.width - diameter) as isize, 0),
                    Rect::new(diameter, bounds.height),
                ),
                (
                    Point::new(0, (bounds.height - diameter) as isize),
                    Rect::new(bounds.width, diameter),
                ),
            ],
            mask: removal_mask,
            pixels,
        }
    }
}

pub struct WindowDecorations {
    border: WindowAttachment,
}

impl WindowDecorations {
    pub fn get_whole_damage(&self, position: Point) -> DamageRegion {
        self.border.damage(position)
    }
    pub fn new_default(bounds: &Rect) -> Self {
        Self {
            border: WindowAttachment::new_border_for(bounds, 8, Pixel::rgb(0xFD, 0xB0, 0xC0)),
        }
    }

    #[inline]
    /// Applies decorations on window fix
    pub fn on_window_fix(
        &self,
        fb: &mut Framebuffer,
        window_rect: &TransformRect,
        damage: &[DamageRegion],
        pixels: &mut [Pixel],
        window_fix_f: impl FnOnce(&mut Framebuffer, IntersectionPoint, &[Pixel]),
    ) {
        let win_bounds = window_rect.rect;
        let mut round_corners = || {
            for (point, area) in &*self.border.mask {
                let mut width = area.width;
                let mut height = area.height;

                let rel_point = self.border.placement + *point;
                let mut x = rel_point.x();
                let mut y = rel_point.y();

                if x.is_negative() {
                    if let Some(w) = width.checked_add_signed(x) {
                        width = w;
                        x = 0;
                    } else {
                        continue;
                    }
                }

                if y.is_negative() {
                    if let Some(h) = height.checked_add_signed(y) {
                        height = h;
                        y = 0;
                    } else {
                        continue;
                    }
                }

                if y as usize > win_bounds.height || x as usize > win_bounds.width {
                    continue;
                }

                let x = x as usize;
                let y = y as usize;
                width = width.min(win_bounds.width - x);
                height = height.min(win_bounds.height - y);

                for row in 0..height {
                    let start = ((row + y) * win_bounds.width) + x;
                    let end = start + width;

                    pixels[start..end].fill(Pixel::NONE);
                }
            }
        };

        let border_place = self.border.placement + window_rect.position();
        let border_rect = TransformRect::new(border_place, self.border.bounds);

        let mut border_intersections = [IntersectionPoint::none(); 4];
        let mut win_inter = IntersectionPoint::none();
        let mut in_border = false;

        for damage in damage {
            for (index, (point, area)) in self.border.damage_bounds.iter().enumerate() {
                let rect = TransformRect::new((*point) + border_place, *area);
                if let Some(inter) = damage.overlaps_with(&rect) {
                    let (i_point, i_area) = inter.to_rect();
                    let rel_point = i_point + *point;
                    let rel_inter = IntersectionPoint::from_rect(rel_point, i_area);

                    border_intersections[index] = border_intersections[index] + rel_inter;
                    in_border = true;
                }
            }

            if let Some(inter) = damage.overlaps_with(window_rect) {
                win_inter = win_inter + inter;
            }
        }

        if win_inter != IntersectionPoint::none() {
            if in_border {
                round_corners();
            }

            window_fix_f(fb, win_inter, pixels);
        }

        if in_border {
            for inter in border_intersections
                .iter()
                .filter(|i| **i != IntersectionPoint::none())
            {
                border_rect.draw_at(fb, *inter, &self.border.pixels);
            }
        }
    }
}
