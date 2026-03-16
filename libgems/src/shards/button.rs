use crate::render::Color;
use crate::shards::{CachedShard, Label, RenderCtx, ShardLayout, ShardsExt};
use crate::{EventCtx, ShardEvent};

use crate::{
    AppCtx, BoundingRect,
    render::{Padding, PaintBrush, shapes::Rect},
    shards::Shard,
};

/// A Clickable button with a label.
pub struct Button<Ctx: AppCtx> {
    label: CachedShard<Label<Ctx>>,
    label_layout: Option<ShardLayout>,
    radius: f32,
    paint: PaintBrush,
    dirty: bool,
}

impl<Ctx: AppCtx> Button<Ctx> {
    pub fn new(label: Label<Ctx>) -> Self {
        Self {
            radius: 8.,
            paint: PaintBrush::Color(Color::rgb(0xFD, 0xB0, 0xC0)),
            dirty: true,
            label_layout: None,
            label: label.cached(),
        }
    }

    /// Sets the paint brush for the button.
    pub fn with_paint(mut self, paint: PaintBrush) -> Self {
        self.paint = paint;
        self
    }

    /// Sets the paint brush for the button.
    pub fn set_paint(&mut self, paint: PaintBrush) {
        self.paint = paint;
        self.dirty = true;
    }
}

impl<Ctx: AppCtx> Shard<Ctx> for Button<Ctx> {
    fn dirty(&self) -> bool {
        self.dirty || self.label.dirty()
    }
    fn layout(&mut self, ctx: &mut super::LayoutCtx) -> super::ShardLayout {
        let label_laid = self.label.layout(ctx);
        self.label_layout = Some(label_laid);

        let constraints = ctx.constraints();
        let max = constraints.max();
        let w = max.width();
        let h = max.height();
        super::ShardLayout {
            bounds: BoundingRect::new(w, h),
            padding: Padding::none(),
            alignment: crate::Alignment::default(),
        }
    }

    fn on_event(&mut self, event_ctx: &mut EventCtx, event: &ShardEvent, _app_ctx: &mut Ctx) {
        match event {
            ShardEvent::MouseClick(_) => {
                event_ctx.set_active(true);
            }
            ShardEvent::MouseRelease(_) => {
                event_ctx.set_active(false);
            }
            _ => {}
        }
    }

    fn render(&mut self, ctx: &mut RenderCtx) -> Option<(crate::Point, BoundingRect)> {
        let layout = ctx.layout();
        let is_active = ctx.is_active();
        let is_hot = ctx.is_hot();

        let w = layout.bounds.width();
        let h = layout.bounds.height();

        ctx.fill(&self.paint, &Rect::new_rect(w, h).round(self.radius));

        let overlay_alpha = if !is_active { 20 } else { 36 };

        let label_layout = self
            .label_layout
            .expect("Didn't layout label during buttons layout");
        let label_bounds = label_layout.bounds;
        let l_w = label_bounds.width();
        let l_h = label_bounds.height();

        ctx.nest_ctx(
            crate::Point::new((w - l_w) / 2., (h - l_h) / 2.),
            label_bounds,
            |ctx| self.label.render(ctx),
        );

        if is_hot {
            ctx.fill(
                &self.paint.with_alpha(overlay_alpha),
                &Rect::new_rect(w, h).round(self.radius),
            );
        }
        self.dirty = false;
        None
    }
}
