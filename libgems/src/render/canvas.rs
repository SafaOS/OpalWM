use std::{ffi::OsStr, path::PathBuf};

use cosmic_text::{FontSystem, SwashCache};
use tiny_skia::{PathBuilder, PixmapMut};

use crate::render::{BoundingRect, Point, Shape, utils};

use super::Color;

/// Represents a Paint brush used to paint pixels such as a Color or a gradient.
#[derive(Debug, Clone)]
pub struct PaintBrush<'a>(tiny_skia::Paint<'a>);

impl From<Color> for PaintBrush<'static> {
    fn from(value: Color) -> Self {
        let mut paint = tiny_skia::Paint::default();
        paint.set_color_rgba8(value.r(), value.g(), value.b(), value.a());
        Self(paint)
    }
}

impl<'a> From<tiny_skia::Paint<'a>> for PaintBrush<'a> {
    fn from(value: tiny_skia::Paint<'a>) -> Self {
        Self(value)
    }
}

impl<'a> Into<tiny_skia::Paint<'a>> for PaintBrush<'a> {
    fn into(self) -> tiny_skia::Paint<'a> {
        self.0
    }
}

impl<'a> PaintBrush<'a> {
    pub fn with_opacity(mut self, opacity: f32) -> Self {
        self.as_paint_mut().shader.apply_opacity(opacity);
        self
    }

    #[inline]
    pub fn with_blend(mut self, blend_mode: tiny_skia::BlendMode) -> Self {
        self.as_paint_mut().blend_mode = blend_mode;
        self
    }

    #[inline]
    pub fn no_aa(mut self) -> Self {
        self.as_paint_mut().anti_alias = false;
        self
    }

    #[inline]
    pub fn as_paint_mut(&mut self) -> &mut tiny_skia::Paint<'a> {
        &mut self.0
    }

    #[inline]
    pub fn as_paint(&self) -> &tiny_skia::Paint<'a> {
        &self.0
    }
}
/// Tiny Skia cache to save rendering allocations
pub struct CanvasCache {
    pub swash_cache: SwashCache,
    pub font_system: FontSystem,
    path: PathBuilder,
}

impl CanvasCache {
    pub fn new() -> Self {
        let fonts_list = std::fs::read("sys:/fonts/fontlist").expect("No fonts found");

        let fonts = fonts_list
            .split(|c| *c == b'\n')
            .map(|s| {
                PathBuf::from("sys:/fonts/").join(unsafe { OsStr::from_encoded_bytes_unchecked(s) })
            })
            .filter(|p| p.is_file())
            .map(|p| {
                let font_data = std::fs::read(p).expect("Failed to read font data");
                cosmic_text::fontdb::Source::Binary(std::sync::Arc::new(
                    font_data.into_boxed_slice(),
                ))
            });

        let font_system = FontSystem::new_with_fonts(fonts);
        Self {
            swash_cache: SwashCache::new(),
            font_system: font_system,
            path: PathBuilder::new(),
        }
    }

    pub fn font_system(&mut self) -> &mut FontSystem {
        &mut self.font_system
    }

    pub fn swash_cache(&mut self) -> &mut SwashCache {
        &mut self.swash_cache
    }

    pub fn path(&mut self) -> &mut PathBuilder {
        &mut self.path
    }
}

pub struct CanvasContext<'c> {
    pub cache: &'c mut CanvasCache,
    pub pixmap: PixmapMut<'c>,
}

impl<'c> CanvasContext<'c> {
    pub fn new(cache: &'c mut CanvasCache, pixmap: PixmapMut<'c>) -> Self {
        Self { cache, pixmap }
    }

    #[inline]
    pub fn fill<S: Shape>(&mut self, position: Point, brush: &PaintBrush, shape: &S) {
        let mut path = core::mem::take(&mut self.cache.path);
        shape.add_to_path(&mut path, position);
        let final_path = path.finish().expect("Shape invalid");

        let paint = brush.as_paint();

        self.pixmap.fill_path(
            &final_path,
            paint,
            tiny_skia::FillRule::default(),
            tiny_skia::Transform::identity(),
            None,
        );

        self.cache.path = final_path.clear();
    }

    #[inline]
    pub fn stroke<S: Shape>(
        &mut self,
        position: Point,
        brush: &PaintBrush,
        stroke_width: f32,
        shape: &S,
    ) {
        let mut path = core::mem::take(&mut self.cache.path);
        shape.add_to_path(&mut path, position);
        let final_path = path.finish().expect("Shape invalid");

        let paint = brush.as_paint();
        let mut stroke = tiny_skia::Stroke::default();
        stroke.width = stroke_width;

        self.pixmap.stroke_path(
            &final_path,
            paint,
            &stroke,
            tiny_skia::Transform::identity(),
            None,
        );

        self.cache.path = final_path.clear();
    }

    #[inline]
    pub fn draw_text(&mut self, position: Point, color: Color, text: &cosmic_text::Buffer) {
        let c = color.demultiply();
        let target_y = position.y().ceil() as i32;
        let target_x = position.x().ceil() as i32;

        let pixmap = &mut self.pixmap;
        let px_width = pixmap.width();
        let px_height = pixmap.height();

        let mut fill_rect_fast =
            |x: u32, y: u32, width: u32, height: u32, c: tiny_skia::PremultipliedColorU8| {
                let pixels = pixmap.pixels_mut();

                let height = height.min(px_height.saturating_sub(y));
                let width = width.min(px_width.saturating_sub(x));

                for h in 0..height {
                    let y = y + h;
                    for w in 0..width {
                        let index = ((y * px_width) + x + w) as usize;
                        utils::blend_pixel(&c, &mut pixels[index]);
                    }
                }
            };
        let cache = &mut self.cache.swash_cache;
        let font_system = &mut self.cache.font_system;

        text.draw(
            font_system,
            cache,
            cosmic_text::Color::rgba(c.r, c.g, c.b, c.a),
            |o_x, o_y, w, h, color| {
                let x = o_x + target_x;
                let y = o_y + target_y;

                let (x, w) = if x < 0 {
                    let overhang = (-x) as u32;
                    if overhang >= w {
                        return;
                    }
                    (0u32, w - overhang)
                } else {
                    (x as u32, w)
                };
                let (y, h) = if y < 0 {
                    let overhang = (-y) as u32;
                    if overhang >= h {
                        return;
                    }
                    (0u32, h - overhang)
                } else {
                    (y as u32, h)
                };

                fill_rect_fast(
                    x as u32,
                    y as u32,
                    w,
                    h,
                    tiny_skia::ColorU8::from_rgba(color.r(), color.g(), color.b(), color.a())
                        .premultiply(),
                );
            },
        );
    }
    #[inline]
    pub fn fill_with_pixmap(
        &mut self,
        position: Point,
        other: tiny_skia::PixmapRef,
        paint: &tiny_skia::PixmapPaint,
    ) {
        self.pixmap.draw_pixmap(
            0,
            0,
            other,
            &paint,
            tiny_skia::Transform::from_translate(position.x(), position.y()),
            None,
        );
    }
    #[inline]
    pub fn clear(&mut self, brush: PaintBrush, position: Point, area: BoundingRect) {
        self.pixmap.fill_rect(
            tiny_skia::Rect::from_xywh(position.x(), position.y(), area.width(), area.height())
                .expect("Failed to construct rect"),
            brush.with_blend(tiny_skia::BlendMode::Source).as_paint(),
            tiny_skia::Transform::default(),
            None,
        );
    }
}
