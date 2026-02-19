use libopal::window::Pixel;

use crate::window::primitive::{Point, Rect, UPoint};

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

    fn apply_on(&self, bounds: Rect, pixels: &mut [Pixel]) {
        for verb in &*self.verbs {
            match verb {
                DrawVerb::DrawPoint(at, pixel) => {
                    let index = (at.y() * bounds.width as isize) + at.x();
                    if index >= 0 && index < pixels.len() as isize {
                        pixels[index as usize] = *pixel;
                    }
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

#[derive(Debug, Clone)]
pub struct WindowDecorationsMeta {
    /// Each element is a [Y] => (K) where K is the amount of pixels to cut from each side.
    /// for the top `radius` rows, Y represents the Y coordinate of the row (Y = y if y < radius).
    ///
    /// for the bottom `radius` rows for each y, Y = height - y (if y > height - radius)
    corner_mask_span: Box<[usize]>,
    corner_radius: u32,
}

impl WindowDecorationsMeta {
    /// Copies pixels from a region within src to dst (if dst has a border).
    ///
    /// `bounds_in_src` is the bounds of the region within src
    /// `offset` is the offset of the region within src
    /// `src_dst_bounds` is the bounds of both src and dst, without the border
    pub fn copy_pixels(
        border: Option<&Self>,
        src: &[Pixel],
        dst: &mut [Pixel],
        bounds_in_src: Rect,
        src_x: usize,
        src_y: usize,
        src_dst_bounds: Rect,
    ) -> UPoint {
        let target_height = bounds_in_src.height();
        let target_width = bounds_in_src.width();

        let bounds = src_dst_bounds;
        let src_width = bounds.width;
        let dst_width;
        let dst_height;

        let x_diff;
        let y_diff;
        if border.is_some() {
            x_diff = 1;
            y_diff = 1;
            dst_width = bounds.width() + 2;
            dst_height = bounds.height() + 2;
        } else {
            x_diff = 0;
            y_diff = 0;
            dst_width = bounds.width();
            dst_height = bounds.height();
        }

        let dst_x = src_x + x_diff;
        for y in src_y..(src_y + target_height) {
            let dst_y = y + y_diff;
            let mut r_src_x = src_x;
            let mut r_dst_x = dst_x;
            let mut r_target_width = target_width;

            // Skip corner mask pixels
            if let Some(border) = border
                && border.corner_radius > 0
            {
                let radius = border.corner_radius as usize;
                let corner_mask = &border.corner_mask_span;

                let skip;
                if dst_y < radius {
                    skip = Some(corner_mask[dst_y]);
                } else if dst_y >= dst_height - radius {
                    skip = Some(corner_mask[dst_height - dst_y - 1]);
                } else {
                    skip = None;
                }

                if let Some(skip) = skip
                    && skip >= x_diff
                {
                    r_src_x = r_src_x.max(skip - x_diff);
                    r_target_width = target_width.min(src_width - r_src_x - (skip - x_diff));
                    r_dst_x = r_dst_x.max(skip);
                }
            }

            let src_start = (y * src_width) + r_src_x;
            let src_end = src_start + r_target_width;

            let dst_start = (dst_y * dst_width) + r_dst_x;
            let dst_end = dst_start + r_target_width;
            dst[dst_start..dst_end].copy_from_slice(&src[src_start..src_end]);
        }
        UPoint::new(src_x + x_diff, src_y + y_diff)
    }
    pub fn new(bounds: Rect, fill_with: Pixel) -> (Self, Box<[Pixel]>, Rect, Point) {
        Self::new_inner(bounds, 8, fill_with, Pixel::rgb(0xFD, 0xB0, 0xC0))
    }

    fn new_inner(
        bounds: Rect,
        corner_radius: u32,
        fill_with: Pixel,
        border_color: Pixel,
    ) -> (Self, Box<[Pixel]>, Rect, Point) {
        let width = bounds.width + 2;
        let height = bounds.height + 2;
        let mut pixels = vec![fill_with; width * height].into_boxed_slice();
        let diameter = (corner_radius * 2) as usize;

        let mut rendering = CachedDrawing::new();

        // Each element is a [Y] => (K) where K is the amount of pixels to cut from each side.
        // for the top `radius` rows, Y represents the Y coordinate of the row (Y = y if y < radius).
        //
        // for the bottom `radius` rows for each y, Y = height - y (if y > height - radius)
        let mut corner_mask_span = vec![0usize; corner_radius as usize].into_boxed_slice();

        // Math to mask rounded corners
        // (Thanks to sasdallas for code I stole, kinda)
        let f_radius = corner_radius as f32;
        let radius = corner_radius as isize;
        for dy in 0..radius {
            for dx in 0..radius {
                let dist_sq = dx * dx + dy * dy;
                if dist_sq <= radius * radius && dist_sq >= (radius - 1) * (radius - 1) {
                    let dist = (dist_sq as f32).sqrt();

                    // anti-aliasing I think
                    let alpha = if dist > f_radius - 1.0 {
                        1.0 - (dist - (f_radius - 1.0)).max(0.0)
                    } else {
                        1.0
                    };

                    let alpha_u8 = (alpha * 255.0) as u8;
                    let color = Pixel::rgba(
                        border_color.r(),
                        border_color.g(),
                        border_color.b(),
                        alpha_u8,
                    );

                    let x = radius - dx - 1;
                    let y = radius - dy - 1;
                    let x2 = width as isize - radius + dx;
                    let y2 = height as isize - radius + dy;

                    rendering.add_point(Point::new(x, y), color);
                    rendering.add_point(Point::new(x2, y), color);
                    rendering.add_point(Point::new(x, y2), color);
                    rendering.add_point(Point::new(x2, y2), color);

                    if y >= 0 {
                        let skip = corner_mask_span[y as usize].max(
                            ((x + 1).is_positive())
                                .then(|| (x + 1) as usize)
                                .unwrap_or(0),
                        );
                        corner_mask_span[y as usize] = skip;
                    }
                }
            }
        }

        rendering.add_rect(
            Point::new(0, corner_radius as isize),
            Rect::new(1, height - diameter),
            border_color,
        );
        rendering.add_rect(
            Point::new(width as isize - 1, corner_radius as isize),
            Rect::new(1, height - diameter),
            border_color,
        );

        rendering.add_rect(
            Point::new(corner_radius as isize, 0),
            Rect::new(width - diameter, 1),
            border_color,
        );

        rendering.add_rect(
            Point::new(corner_radius as isize, height as isize - 1),
            Rect::new(width - diameter, 1),
            border_color,
        );

        let bordered_bounds = Rect::new(width, height);
        rendering.apply_on(bordered_bounds, &mut pixels);
        (
            Self {
                corner_mask_span,
                corner_radius,
            },
            pixels,
            bordered_bounds,
            Point::new(-1, -1),
        )
    }
}
