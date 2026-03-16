use cosmic_text::{FontSystem, SwashCache};
use tiny_skia::Pixmap;

use crate::render::{BoundingRect, Point, Shape};

use super::Color;

/// Represents a Paint brush used to paint pixels such as a Color(only for now) or a gradient.
#[derive(Debug, Clone)]
pub enum PaintBrush {
    Color(Color),
}

impl PaintBrush {
    pub const fn with_alpha(&self, alpha: u8) -> Self {
        match self {
            PaintBrush::Color(color) => PaintBrush::Color(color.with_alpha(alpha)),
        }
    }
}

/// Represents a Paint Canvas where you can draw.
pub trait Canvas {
    fn stroke(
        &mut self,
        cache: &mut CanvasCache,
        position: Point,
        brush: &PaintBrush,
        stroke_width: f32,
        shape: &dyn Shape,
    );
    fn fill(
        &mut self,
        cache: &mut CanvasCache,
        position: Point,
        brush: &PaintBrush,
        shape: &dyn Shape,
    );
    fn fill_with_pixmap(
        &mut self,
        cache: &mut CanvasCache,
        position: Point,
        other: &tiny_skia::Pixmap,
    );
    fn draw_text(
        &mut self,
        cache: &mut CanvasCache,
        position: Point,
        brush: &PaintBrush,
        text: &cosmic_text::Buffer,
    );

    fn clear(&mut self, brush: &PaintBrush, position: Point, area: BoundingRect);
}

/// Tiny Skia cache to save rendering allocations
pub struct CanvasCache {
    swash_cache: SwashCache,
    font_system: FontSystem,
}

impl CanvasCache {
    pub fn new() -> Self {
        let font_data =
            std::fs::read("sys:/fonts/DejaVuSansMono.ttf").expect("Failed to open font file");
        let font_system = FontSystem::new_with_fonts([cosmic_text::fontdb::Source::Binary(
            std::sync::Arc::new(font_data.into_boxed_slice()),
        )]);

        Self {
            swash_cache: SwashCache::new(),
            font_system: font_system,
        }
    }

    pub fn font_system(&mut self) -> &mut FontSystem {
        &mut self.font_system
    }

    pub fn swash_cache(&mut self) -> &mut SwashCache {
        &mut self.swash_cache
    }
}

pub struct CanvasContext<'c> {
    pub cache: &'c mut CanvasCache,
    canvas: &'c mut dyn Canvas,
}

impl<'c> CanvasContext<'c> {
    pub fn new<C: Canvas>(cache: &'c mut CanvasCache, canvas: &'c mut C) -> Self {
        Self { cache, canvas }
    }

    #[inline]
    pub fn fill<S: Shape>(&mut self, position: Point, brush: &PaintBrush, shape: &S) {
        self.canvas.fill(self.cache, position, brush, shape);
    }

    #[inline]
    pub fn stroke<S: Shape>(
        &mut self,
        position: Point,
        brush: &PaintBrush,
        stroke_width: f32,
        shape: &S,
    ) {
        self.canvas
            .stroke(self.cache, position, brush, stroke_width, shape)
    }

    #[inline]
    pub fn draw_text(&mut self, position: Point, brush: &PaintBrush, text: &cosmic_text::Buffer) {
        self.canvas.draw_text(self.cache, position, brush, text);
    }
    #[inline]
    pub fn fill_with_pixmap(&mut self, position: Point, other: &tiny_skia::Pixmap) {
        self.canvas.fill_with_pixmap(self.cache, position, other);
    }
    #[inline]
    pub fn clear(&mut self, brush: &PaintBrush, position: Point, area: BoundingRect) {
        self.canvas.clear(brush, position, area);
    }
}

#[derive(Debug, Clone, Copy)]
pub struct NoopCanvas;

impl Canvas for NoopCanvas {
    fn clear(&mut self, brush: &PaintBrush, position: Point, area: BoundingRect) {
        _ = brush;
        _ = position;
        _ = area;
    }
    fn draw_text(
        &mut self,
        cache: &mut CanvasCache,
        position: Point,
        brush: &PaintBrush,
        text: &cosmic_text::Buffer,
    ) {
        _ = cache;
        _ = position;
        _ = brush;
        _ = text;
    }

    fn fill(
        &mut self,
        cache: &mut CanvasCache,
        position: Point,
        brush: &PaintBrush,
        shape: &dyn Shape,
    ) {
        _ = cache;
        _ = position;
        _ = brush;
        _ = shape;
    }

    fn fill_with_pixmap(
        &mut self,
        cache: &mut CanvasCache,
        position: Point,
        other: &tiny_skia::Pixmap,
    ) {
        _ = cache;
        _ = position;
        _ = other;
    }

    fn stroke(
        &mut self,
        cache: &mut CanvasCache,
        position: Point,
        brush: &PaintBrush,
        stroke_width: f32,
        shape: &dyn Shape,
    ) {
        _ = cache;
        _ = position;
        _ = brush;
        _ = stroke_width;
        _ = shape;
    }
}

#[derive(Debug, Clone)]
pub struct TinySkiaCanvas {
    pub(crate) pixmap: Pixmap,
}

impl TinySkiaCanvas {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            pixmap: Pixmap::new(width.max(1), height.max(1)).expect("Failed to construct Pixmap"),
        }
    }

    pub fn width(&self) -> u32 {
        self.pixmap.width()
    }

    pub fn height(&self) -> u32 {
        self.pixmap.height()
    }
}

impl Canvas for TinySkiaCanvas {
    fn clear(&mut self, brush: &PaintBrush, position: Point, area: BoundingRect) {
        match brush {
            PaintBrush::Color(pix) => {
                let color = tiny_skia::Color::from_rgba8(pix.r(), pix.g(), pix.b(), pix.a())
                    .premultiply()
                    .to_color_u8();
                let pix_width = self.pixmap.width();
                let pix_height = self.pixmap.height();

                let x = (position.x().floor().max(0.) as u32).min(pix_width);
                let y = (position.y().floor().max(0.) as u32).min(pix_height);

                let width = (area.width().ceil().clamp(0., f32::MAX) as u32).min(pix_width - x);
                let height = (area.height().ceil().clamp(0., f32::MAX) as u32).min(pix_height - y);

                let pixels = self.pixmap.pixels_mut();
                for y in y..(y + height) {
                    let start = (y * pix_width) + x;
                    let end = start + width;
                    pixels[start as usize..end as usize].fill(color);
                }
            }
        };
    }
    fn fill_with_pixmap(
        &mut self,
        _cache: &mut CanvasCache,
        position: Point,
        other: &tiny_skia::Pixmap,
    ) {
        self.pixmap.draw_pixmap(
            position.x().floor() as i32,
            position.y().floor() as i32,
            other.as_ref(),
            &tiny_skia::PixmapPaint::default(),
            tiny_skia::Transform::identity(),
            None,
        );
    }

    fn fill(
        &mut self,
        _cache: &mut CanvasCache,
        position: Point,
        brush: &PaintBrush,
        shape: &dyn Shape,
    ) {
        let mut path = tiny_skia::PathBuilder::with_capacity(1, 1);
        shape.add_to_path(&mut path, position);
        let final_path = path.finish().expect("Shape invalid");

        let paint = match brush {
            PaintBrush::Color(pix) => {
                let mut paint = tiny_skia::Paint::default();
                paint.set_color_rgba8(pix.r(), pix.g(), pix.b(), pix.a());
                paint
            }
        };

        self.pixmap.fill_path(
            &final_path,
            &paint,
            tiny_skia::FillRule::default(),
            tiny_skia::Transform::identity(),
            None,
        );
    }

    fn stroke(
        &mut self,
        _cache: &mut CanvasCache,
        position: Point,
        brush: &PaintBrush,
        stroke_width: f32,
        shape: &dyn Shape,
    ) {
        let mut path = tiny_skia::PathBuilder::with_capacity(1, 1);
        shape.add_to_path(&mut path, position);
        let final_path = path.finish().expect("Shape invalid");

        let mut paint = tiny_skia::Paint::default();
        let mut stroke = tiny_skia::Stroke::default();
        stroke.width = stroke_width;

        match brush {
            PaintBrush::Color(pix) => {
                paint.set_color_rgba8(pix.r(), pix.g(), pix.b(), pix.a());
            }
        };

        self.pixmap.stroke_path(
            &final_path,
            &paint,
            &stroke,
            tiny_skia::Transform::identity(),
            None,
        );
    }

    fn draw_text(
        &mut self,
        cache: &mut CanvasCache,
        position: Point,
        brush: &PaintBrush,
        text: &cosmic_text::Buffer,
    ) {
        let mut paint = tiny_skia::Paint::default();
        let cosmic_color;

        match brush {
            PaintBrush::Color(color) => {
                cosmic_color = cosmic_text::Color::rgba(color.r(), color.g(), color.b(), color.a());
            }
        }

        text.draw(
            &mut cache.font_system,
            &mut cache.swash_cache,
            cosmic_color,
            |x, y, w, h, c| {
                paint.set_color_rgba8(c.r(), c.g(), c.b(), c.a());

                self.pixmap.fill_rect(
                    tiny_skia::Rect::from_xywh(x as f32, y as f32, w as f32, h as f32)
                        .expect("Failed to construct rect"),
                    &paint,
                    tiny_skia::Transform::from_translate(position.x(), position.y()),
                    None,
                );
            },
        );
    }
}
