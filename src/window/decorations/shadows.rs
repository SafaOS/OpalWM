use std::time::Instant;

use libopal::window::Pixel;

use crate::window::primitive::{Point, Rect, UPoint};

fn point_in_rounded_rect(p: Point, r0: Point, r1: Point, radius: u32) -> bool {
    let (x, y) = *p;
    let (x0, y0) = *r0;
    let (x1, y1) = *r1;

    let r = radius as isize;

    let cx = x.clamp(x0 + r, x1 - r);
    let cy = y.clamp(y0 + r, y1 - r);

    let dx = x - cx;
    let dy = y - cy;

    dx * dx + dy * dy <= r * r
}

// fn box_blur(tmp_buf: &mut Vec<u32>, img: &mut [u8], width: u32, height: u32, blur_radius: u32) {
//     let w = width as usize;
//     let h = height as usize;
//     let r = blur_radius as usize;
//     let window = 2 * r + 1;
//     let pix_count = (window * window) as u32;

//     if w <= 2 * r || h <= 2 * r {
//         return;
//     }

//     let len = w * h;
//     if tmp_buf.len() < len {
//         tmp_buf.resize(len, 0);
//     }
//     let h_sum = &mut tmp_buf[..len];

//     // Pass 1: horizontal running-sum, per row
//     for y in 0..h {
//         let row = y * w;
//         let row_px = &img[row..row + w];

//         let mut s: u32 = row_px[0..window].iter().map(|&v| v as u32).sum();
//         h_sum[row + r] = s;

//         for x in (r + 1)..(w - r) {
//             s += row_px[x + r] as u32;
//             s -= row_px[x - r - 1] as u32;
//             h_sum[row + x] = s;
//         }
//     }

//     // Pass 2: vertical running-sum over h_sum, per column
//     for x in r..(w - r) {
//         let mut s: u32 = (0..window).map(|dy| h_sum[dy * w + x]).sum();
//         img[r * w + x] = (s / pix_count) as u8;

//         for y in (r + 1)..(h - r) {
//             s += h_sum[(y + r) * w + x];
//             s -= h_sum[(y - r - 1) * w + x];
//             img[y * w + x] = (s / pix_count) as u8;
//         }
//     }
// }

#[inline]
pub fn draw_shadow(
    pixels: &mut [Pixel],
    bounds: Rect,
    content_off: UPoint,
    content_bounds: Rect,
    corner_radius: u32,
    blur_radius: u32,
    color: Pixel,
    offset: UPoint,
    opacity: f32,
) {
    let content_begin = Point::new(content_off.x() as isize, content_off.y() as isize);
    let content_end = Point::new(
        (content_off.x() + content_bounds.width()) as isize,
        (content_off.y() + content_bounds.height()) as isize,
    );
    let time = Instant::now();
    let width = bounds.width();
    let height = bounds.height();
    let mut alphas = vec![0u8; pixels.len()];

    let mut alpha_width = 0;
    let mut alpha_height = 0;
    let mut alpha_x = width;
    let mut alpha_y = height;

    for y in 0..height {
        let i = y * width;

        for x in 0..width {
            let i = i + x;
            if point_in_rounded_rect(
                Point::new(x as isize, y as isize),
                content_begin,
                content_end,
                corner_radius,
            ) {
                alpha_width = x.max(alpha_width);
                alpha_height = y.max(alpha_height);
                alpha_x = x.min(alpha_x);
                alpha_y = y.min(alpha_y);

                alphas[i] = 255;
            }
        }
    }

    if blur_radius != 0 {
        _ = libblur::fast_gaussian(
            &mut libblur::BlurImageMut::borrow(
                &mut alphas,
                width as u32,
                height as u32,
                libblur::FastBlurChannels::Plane,
            ),
            libblur::AnisotropicRadius::new(blur_radius),
            libblur::ThreadingPolicy::Single,
            libblur::EdgeMode2D::new(libblur::EdgeMode::Clamp),
        );
    }

    let offset = UPoint::new(offset.x() + alpha_x, offset.y() + alpha_y);
    crate::log!(
        "alpha at x: {alpha_x}, y: {alpha_y}, w: {alpha_width}, h: {alpha_height} of w: {width}, h: {height} to be placed at: {offset:?} elapsed: {}",
        time.elapsed().as_millis()
    );
    assert!(alpha_width <= width);
    assert!(alpha_height <= height);

    for y in 0..alpha_height {
        let dst_y = offset.y() + y;
        if dst_y >= height {
            break;
        }

        let i = dst_y * width;

        let src_y = y + alpha_y;
        let src_i = src_y * width;

        for x in 0..alpha_width {
            let dst_x = offset.x() + x;
            if dst_x >= width {
                break;
            }

            let i = i + dst_x;

            let src_x = x + alpha_x;
            let src_i = src_i + src_x;

            pixels[i] = pixels[i].blend(&color.with_alpha((alphas[src_i] as f32 * opacity) as u8));
        }
    }
}
