use libopal::window::Pixel;

use crate::window::primitive::{Point, Rect, UPoint};
mod corners;
pub mod shadows;

use corners::*;

const BORDER_THICKNESS: usize = 2;

#[derive(Debug, Clone)]
pub struct WindowDecorationsMeta {
    /// Each element is a [Y] => (K) where K is the amount of pixels to cut from each side.
    /// for the top `radius` rows, Y represents the Y coordinate of the row (Y = y if y < radius).
    ///
    /// for the bottom `radius` rows for each y, Y = height - y (if y > height - radius)
    corner_mask_span: Box<[usize]>,
    corner_radius: u32,
    thickness: u32,
    draw_x: usize,
    draw_y: usize,
    bounds: Rect,
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

        // The difference between the src width and the dst width
        let x_diff;
        // The difference between the src height and the dst height.
        let y_diff;
        let b_off_x;
        let b_off_y;

        if let Some(border) = border {
            x_diff = border.draw_x;
            y_diff = border.draw_y;
            dst_width = border.bounds.width();
            dst_height = border.bounds.height();
            b_off_x = border.draw_x.saturating_sub(border.thickness as usize);
            b_off_y = border.draw_y.saturating_sub(border.thickness as usize)
        } else {
            // No decorations dst == src.
            x_diff = 0;
            y_diff = 0;
            b_off_x = 0;
            b_off_y = 0;
            dst_width = bounds.width();
            dst_height = bounds.height();
        }

        // initial dst x
        let dst_x = src_x + x_diff;
        for sy in src_y..(src_y + target_height) {
            // initial dst y
            let dst_y = sy + y_diff;
            // rounded up src x, may change
            let mut r_src_x = src_x;
            // rounded up dst x, may change
            let mut r_dst_x = dst_x;
            // rounded up dst width, may change
            let mut r_target_width = target_width;

            // Skip corner mask pixels
            if let Some(border) = border
                && border.corner_radius > 0
            {
                // Y relative to the border
                let by = dst_y - b_off_y;
                let radius = border.corner_radius as usize;
                let corner_mask = &border.corner_mask_span;

                let skip;
                if by < radius {
                    skip = Some(corner_mask[by]);
                } else if by >= (dst_height - (b_off_y * 2)) - radius {
                    skip = Some(corner_mask[(dst_height - (b_off_y * 2)) - by - 1]);
                } else {
                    skip = None;
                }

                if let Some(skip) = skip
                    && skip >= x_diff - b_off_x
                {
                    // skip is relative to border, translate to src
                    let src_skip = skip - (x_diff - b_off_x);
                    r_src_x = r_src_x.max(src_skip);
                    // Width is src specific, and we want to cut both ends of skip from it.
                    r_target_width = target_width.min(src_width - r_src_x - src_skip);
                    // dst is just the offset of the border + pixels to skip
                    r_dst_x = r_dst_x.max(b_off_x + skip);
                }
            }

            let src_start = (sy * src_width) + r_src_x;
            let src_end = src_start + r_target_width;

            let dst_start = (dst_y * dst_width) + r_dst_x;
            let dst_end = dst_start + r_target_width;
            dst[dst_start..dst_end].copy_from_slice(&src[src_start..src_end]);
        }
        UPoint::new(src_x + x_diff, src_y + y_diff)
    }
    pub fn new(bounds: Rect, fill_with: Pixel) -> (Self, Box<[Pixel]>, Rect, Point) {
        Self::new_inner(bounds, fill_with, Pixel::rgb(0xFD, 0xB0, 0xC0))
    }

    fn new_inner(
        bounds: Rect,
        fill_with: Pixel,
        border_color: Pixel,
    ) -> (Self, Box<[Pixel]>, Rect, Point) {
        let border_off = UPoint::new(40, 40);
        let border_width = bounds.width + (2 * BORDER_THICKNESS);
        let border_height = bounds.height + (2 * BORDER_THICKNESS);

        let width = border_width + border_off.x() * 2;
        let height = border_height + border_off.y() * 2;

        let border_bounds = Rect::new(border_width, border_height);
        let full_bounds = Rect::new(width, height);

        let mut pixels = vec![fill_with; width * height].into_boxed_slice();

        let corner_mask_span = draw_rounded_rect(
            &mut *pixels,
            border_off,
            border_width,
            border_height,
            width,
            height,
            BORDER_THICKNESS,
            border_color,
            Some(Pixel::BLACK),
        );

        shadows::draw_shadow(
            &mut pixels,
            full_bounds,
            border_off,
            border_bounds,
            16,
            12,
            Pixel::BLACK,
            UPoint::new(2, 4),
            0.6,
        );
        let draw_x = border_off.x() + BORDER_THICKNESS;
        let draw_y = border_off.y() + BORDER_THICKNESS;

        (
            Self {
                corner_mask_span,
                corner_radius: CORNER_RADIUS as u32,
                thickness: BORDER_THICKNESS as u32,
                draw_x,
                draw_y,
                bounds: full_bounds,
            },
            pixels,
            full_bounds,
            Point::new(-(draw_x as isize), -(draw_y as isize)),
        )
    }
}
