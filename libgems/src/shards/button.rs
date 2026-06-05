use crate::render::{BoundingConstraints, Color};
use crate::shards::lifecycle::LifeCycle;
use crate::shards::{CachedShard, Label, RenderCtx, ShardsExt};
use crate::{EventCtx, ShardEvent};

use crate::{
    AppCtx, BoundingRect,
    render::{PaintBrush, shapes::Rect},
    shards::Shard,
};

/// A Clickable button with a label.
pub struct Button<Ctx: AppCtx> {
    label: CachedShard<Label<Ctx>>,
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
            label: label.center_text().cached(),
        }
    }

    /// Sets the paint brush for the button.
    pub fn with_paint(mut self, paint: impl Into<PaintBrush>) -> Self {
        self.paint = paint.into();
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

    fn lifecycle(&mut self, _: &mut super::lifecycle::LifeCycleCtx, event: &LifeCycle) {
        match event {
            LifeCycle::Init { .. } | LifeCycle::HotChanged(_) => self.dirty = true,
            _ => {}
        }
    }
    fn layout(&mut self, ctx: &mut super::LayoutCtx) -> super::ShardLayout {
        const PAD: f32 = 8.;
        let constraints = ctx.constraints();
        let min = constraints.min();
        let max = constraints.max();

        let label_laid = ctx.with_constraints(BoundingConstraints::from_max(max), |ctx| {
            self.label.layout(ctx)
        });

        let w = min
            .width()
            .max(label_laid.bounds.width() + PAD)
            .min(max.width());

        let h = min
            .height()
            .max(label_laid.bounds.height() + PAD)
            .min(max.height());
        super::ShardLayout::from_bounds(BoundingRect::new(w, h))
    }

    fn on_event(&mut self, event_ctx: &mut EventCtx, event: &ShardEvent, _app_ctx: &mut Ctx) {
        match event {
            ShardEvent::MouseClick(_) => {
                event_ctx.set_active(true);
                self.dirty = true;
            }
            ShardEvent::MouseRelease(_) => {
                event_ctx.set_active(false);
                self.dirty = true;
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

        self.label.render(ctx);

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
