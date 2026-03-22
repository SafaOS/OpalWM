use std::marker::PhantomData;

use crate::{BoundingRect, Color, Point};
use cosmic_text::{Attrs, Buffer, Metrics, Wrap};

use crate::{AppCtx, render::PaintBrush, shards::Shard};

#[derive(Debug, Clone)]
pub struct Label<Ctx: AppCtx> {
    buffer: Buffer,
    wrap: Wrap,
    text: String,
    text_changed: bool,
    brush: PaintBrush,
    attrs: Attrs<'static>,
    center: bool,
    _ctx: PhantomData<Ctx>,
}

impl<Ctx: AppCtx> Label<Ctx> {
    #[inline]
    pub fn from_str(data: impl Into<String>) -> Self {
        Self {
            text: data.into(),
            buffer: Buffer::new_empty(Metrics::relative(12., 1.)),
            wrap: Wrap::WordOrGlyph,
            text_changed: true,
            brush: PaintBrush::Color(Color::BLACK), /* todo environment themeing */
            attrs: Attrs::new(),
            center: false,
            _ctx: PhantomData,
        }
    }

    #[inline]
    pub fn set_text(&mut self, text: impl AsRef<str>) -> &mut Self {
        if &*self.text != text.as_ref() {
            self.text = text.as_ref().into();
            self.text_changed = true;
        }

        self
    }

    #[inline]
    pub fn with_color(mut self, color: Color) -> Self {
        self.brush = PaintBrush::Color(color);
        self
    }

    #[inline]
    pub fn with_wrap(mut self, wrap: Wrap) -> Self {
        self.wrap = wrap;
        self
    }

    #[inline]
    pub fn with_attrs(mut self, attrs: Attrs<'static>) -> Self {
        self.attrs = attrs;
        self
    }

    #[inline]
    pub fn with_metrics(mut self, metrics: Metrics) -> Self {
        self.buffer = Buffer::new_empty(metrics);
        self.text_changed = true;
        self
    }

    pub fn height(&self) -> f32 {
        self.buffer.layout_runs().map(|run| run.line_height).sum()
    }

    pub fn width(&self) -> f32 {
        self.buffer
            .layout_runs()
            .map(|run| run.line_w)
            .max_by(|x, y| x.partial_cmp(y).unwrap_or(core::cmp::Ordering::Equal))
            .unwrap_or(0.)
    }

    #[inline]
    /// Centers text within it's own bounds.
    pub fn center_text(mut self) -> Self {
        self.center = true;
        self
    }
}

impl<Ctx: AppCtx> Shard<Ctx> for Label<Ctx> {
    fn dirty(&self) -> bool {
        self.text_changed || self.buffer.redraw()
    }

    fn layout(&mut self, ctx: &mut super::LayoutCtx) -> super::ShardLayout {
        let max_bounds = ctx.max_box();
        let width = max_bounds.width();
        let height = max_bounds.height();

        let buffer = &mut self.buffer;
        if self.text_changed {
            buffer.set_text(
                ctx.font_system(),
                &self.text,
                &self.attrs,
                cosmic_text::Shaping::Advanced,
            );

            self.text_changed = false;
        }

        buffer.set_wrap(ctx.font_system(), self.wrap);
        buffer.set_size(
            ctx.font_system(),
            width.is_finite().then_some(width),
            height.is_finite().then_some(height),
        );

        let min_box = ctx.min_box();
        let act_width = self.width().max(min_box.width());
        let act_height = self.height().max(min_box.height());

        super::ShardLayout::from_bounds(BoundingRect::new(act_width, act_height))
    }

    fn render(
        &mut self,
        ctx: &mut super::RenderCtx,
    ) -> Option<(crate::Point, crate::BoundingRect)> {
        let bounds = ctx.layout().bounds;

        let r_w = bounds.width();
        let r_h = bounds.height();
        let b_w = self.width();
        let b_h = self.height();

        let mut start = Point::default();
        if self.center && r_w >= b_w && r_h >= b_h {
            start = crate::Point::new((r_w - b_w) / 2., (r_h - b_h) / 2.);
        }

        ctx.nest_ctx(start, BoundingRect::new(b_w, b_h), |ctx| {
            ctx.fill_text(&self.brush, &self.buffer);
        });
        self.buffer.set_redraw(false);
        None
    }
}
