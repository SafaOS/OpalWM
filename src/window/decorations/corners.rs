use libopal::window::Pixel;

use crate::window::primitive::UPoint;

pub const CORNER_RADIUS: usize = 8;

/// Describes the alpha channel of the top left corner.
pub const CORNERS_ALPHA: [[u8; 8]; 8] = [
    [0, 0, 0, 16, 111, 159, 191, 239],
    [0, 0, 80, 223, 255, 255, 255, 255],
    [0, 80, 255, 255, 255, 255, 255, 0],
    [16, 223, 255, 255, 255, 0, 0, 0],
    [112, 255, 255, 255, 0, 0, 0, 0],
    [176, 255, 255, 0, 0, 0, 0, 0],
    [192, 255, 255, 0, 0, 0, 0, 0],
    [255, 255, 0, 0, 0, 0, 0, 0],
];
/// Get the alpha mask for each corner's pixel, alpha is organized as you'd expect (x, y) from top to bottom, from left to right.
pub const fn get_corner_alpha(
    alpha_table: &[[u8; CORNER_RADIUS]; CORNER_RADIUS],
    top: bool,
    left: bool,
) -> [[u8; CORNER_RADIUS]; CORNER_RADIUS] {
    if top && left {
        return *alpha_table;
    }

    let mut results = [[0u8; CORNER_RADIUS]; CORNER_RADIUS];
    let mut i = 0;
    while i < CORNER_RADIUS {
        let mut j = 0;
        while j < CORNER_RADIUS {
            let o_i = if top { i } else { CORNER_RADIUS - i - 1 };
            let o_j = if left { j } else { CORNER_RADIUS - j - 1 };
            results[i][j] = alpha_table[o_i][o_j];
            j += 1;
        }
        i += 1;
    }
    results
}

/// Get the pixels for each corner given a color,
/// also in each row (y), returns the amount of pixels that are to be skipped before filling with window data.
/// For left corners min-x would be equal to skip, for right corners max-x would be width - skip - 1, the left and right corners skip are equal for each y.
pub const fn get_corner_pixels(
    alpha_table: &[[u8; CORNER_RADIUS]; CORNER_RADIUS],
    top: bool,
    left: bool,
    color: Pixel,
) -> [([Pixel; CORNER_RADIUS], usize); CORNER_RADIUS] {
    let alphas = get_corner_alpha(alpha_table, top, left);
    let mut results = [([Pixel::NONE; CORNER_RADIUS], 0usize); CORNER_RADIUS];
    let mut i = 0;
    while i < CORNER_RADIUS {
        let mut j = 0;
        let mut skip = 0;
        while j < CORNER_RADIUS {
            let alpha = alphas[i][j];
            // If is a left corner until all trailing alpha's are zero skip should update
            if left && alpha != 0 {
                skip = j;
            }

            // If is a right corner skip should be len - beginning zeros
            if !left && alpha != 0 /* at beginning of corner */ && skip == 0 {
                skip = CORNER_RADIUS - j - 1;
            }
            results[i].0[j] = color.with_alpha(alpha);
            j += 1;
        }

        results[i].1 = skip + 1;
        i += 1;
    }
    results
}

const _: () = {
    let l = get_corner_pixels(&CORNERS_ALPHA, true, true, Pixel::BLACK);
    let r = get_corner_pixels(&CORNERS_ALPHA, true, false, Pixel::BLACK);
    let _l_b = get_corner_pixels(&CORNERS_ALPHA, false, true, Pixel::BLACK);
    let _r_b = get_corner_pixels(&CORNERS_ALPHA, false, false, Pixel::BLACK);

    let mut i = 0;
    while i < CORNER_RADIUS {
        let l_skip = l[i].1;
        let r_skip = r[i].1;
        assert!(l_skip == r_skip);
        i += 1;
    }
};

/// Each element is a [Y] => (K) where K is the amount of pixels to cut from each side.
/// for the top `radius` rows, Y represents the Y coordinate of the row (Y = y if y < radius).
///
/// for the bottom `radius` rows for each y, Y = height - y (if y > height - radius)
pub type CornerMaskSpan = Box<[usize]>;

#[inline]
/// Renders a rounded rect to `pixels` with the given border color and thickness.
///
/// Radius is [`CORNER_RADIUS`], returns a [`CornerMaskSpan`] so you can fill the border from the inisde.
pub fn draw_rounded_rect(
    pixels: &mut [Pixel],
    off: UPoint,
    border_width: usize,
    border_height: usize,
    w_width: usize,
    w_height: usize,
    border_thickness: usize,
    color: Pixel,
    fill_color: Option<Pixel>,
) -> CornerMaskSpan {
    assert!(
        border_width >= border_thickness,
        "Width must have the border added expected at least: {border_thickness}, got: {border_width}"
    );
    assert!(
        border_height >= border_thickness,
        "Height must have the border added expected at least: {border_thickness}, got: {border_height}"
    );
    assert!(
        w_width >= border_width && w_height >= border_height,
        "Border width: {border_width} and height {border_height}, must be smaller than window width: {w_width}, and height: {w_height}"
    );

    let mut corner_mask_span = vec![0usize; CORNER_RADIUS as usize * 2].into_boxed_slice();
    let fill_border = |pixels: &mut [Pixel]| {
        let fill_y_t = off.y();
        let fill_y_b = (border_height - border_thickness) + off.y();
        for b_y in 0..border_thickness {
            for y in [fill_y_t + b_y, fill_y_b + b_y] {
                let idx = (y * w_width) + off.x();

                pixels[idx + CORNER_RADIUS..idx + border_width - CORNER_RADIUS].fill(color);
            }
        }
    };

    let fill_sides = |pixels: &mut [Pixel]| {
        let fill_x_l = off.x();
        let fill_x_r = (border_width - border_thickness) + off.x();
        for b_x in 0..border_thickness {
            for y in CORNER_RADIUS..border_height - CORNER_RADIUS {
                let begin_x = b_x + fill_x_l;
                let end_x = b_x + fill_x_r;

                let idx_y = (y + off.y()) * w_width;
                for x in [begin_x, end_x] {
                    pixels[idx_y + x] = color;
                }

                if let Some(c) = fill_color {
                    pixels[idx_y + begin_x + 1..idx_y + end_x - 1].fill(c);
                }
            }
        }
    };

    fill_border(&mut *pixels);
    fill_sides(&mut *pixels);

    let con_table = &CORNERS_ALPHA;
    let mut fill_corner = |top: bool, left: bool| {
        let fill_x = if left {
            0
        } else {
            border_width - CORNER_RADIUS
        } + off.x();
        let fill_y = if top {
            0
        } else {
            border_height - CORNER_RADIUS
        } + off.y();

        let corner_pixels = get_corner_pixels(&con_table, top, left, color);
        for (c_y, (row, skip)) in corner_pixels.iter().enumerate() {
            let y = fill_y + c_y;
            let x = fill_x;
            let idx_y = y * w_width;
            let idx = idx_y + x;

            if idx + CORNER_RADIUS > pixels.len() {
                break;
            }

            for (dst, src) in pixels[idx..idx + CORNER_RADIUS].iter_mut().zip(row.iter()) {
                if src.a() == 0 {
                    continue;
                }
                *dst = src.blend(dst);
            }

            if let Some(c) = fill_color
                && left
                && (((c_y >= border_thickness) && top)
                    || (y - off.y()) < border_height - border_thickness && !top)
            {
                pixels[idx + *skip..idx + border_width - *skip].fill(c);
            }

            let mask_idx = if top { c_y } else { CORNER_RADIUS + c_y };
            corner_mask_span[mask_idx] = corner_mask_span[mask_idx].max(*skip);
        }
    };

    fill_corner(true, true);
    fill_corner(true, false);
    fill_corner(false, true);
    fill_corner(false, false);

    corner_mask_span
}
