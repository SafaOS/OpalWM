use std::marker::PhantomData;

use tiny_skia::{Pixmap, PixmapPaint};

use crate::{BoundingRect, shards::Shard};

/// Represents a Pixel image.
#[derive(Debug, Clone)]
pub struct Image<S = (), M = ()> {
    image: Pixmap,
    paint: Option<PixmapPaint>,
    changed: bool,
    _data: PhantomData<(S, M)>,
}

impl<S, M> Image<S, M> {
    pub fn from_pixels(pixmap: Pixmap) -> Self {
        Self {
            image: pixmap,
            changed: true,
            paint: None,
            _data: PhantomData,
        }
    }

    pub fn with_paint(mut self, paint: impl Into<PixmapPaint>) -> Self {
        self.paint = Some(paint.into());
        self
    }

    pub fn into_pixels(self) -> Pixmap {
        self.image
    }

    #[cfg(feature = "image")]
    pub fn from_image(image: image::DynamicImage) -> Self {
        use tiny_skia::ColorU8;

        let rgba = image.into_rgba8();
        let (width, height) = rgba.dimensions();
        let mut pixmap = Pixmap::new(width, height).expect("Image bad width and height");

        for (src, dest) in rgba.pixels().zip(pixmap.pixels_mut()) {
            let raw = src.0;
            *dest = ColorU8::from_rgba(raw[0], raw[1], raw[2], raw[3]).premultiply();
        }

        Self::from_pixels(pixmap)
    }

    pub fn set_pixels(&mut self, pixmap: Pixmap) {
        *self = Self::from_pixels(pixmap);
    }
}

impl<S, M> Shard<S, M> for Image<S, M> {
    fn dirty(&self) -> bool {
        self.changed
    }

    fn layout(&mut self, ctx: &mut super::LayoutCtx) -> super::ShardLayout {
        super::ShardLayout {
            bounds: BoundingRect::new(
                (self.image.width() as f32).max(ctx.min_box().width()),
                (self.image.height() as f32).max(ctx.min_box().height()),
            ),
            ..Default::default()
        }
    }

    fn render(&mut self, ctx: &mut super::RenderCtx, _: &crate::Data<S, M>) {
        ctx.fill_with_pixmap(self.image.as_ref(), &PixmapPaint::default());
    }
}
