use crate::render::BoundingConstraints;
use crate::shards::lifecycle::LifeCycle;
use crate::shards::{CachedShard, Label, RenderCtx, ShardsExt};
use crate::theme::DEFAULT_BUTTON_COLOR;
use crate::{Data, EventCtx, ShardEvent};

use crate::{
    BoundingRect,
    render::{PaintBrush, shapes::Rect},
    shards::Shard,
};

/// A Clickable button with a label.
pub struct Button<S, M = ()> {
    label: CachedShard<Label<S, M>>,
    radius: f32,
    paint: Option<PaintBrush>,
    dirty: bool,
}

impl<S, M> Button<S, M> {
    pub fn new(label: Label<S, M>) -> Self {
        Self {
            radius: 8.,
            paint: None,
            dirty: true,
            label: label.center_text().cached(),
        }
    }

    /// Sets the paint brush for the button.
    pub fn with_paint(mut self, paint: impl Into<PaintBrush>) -> Self {
        self.paint = Some(paint.into());
        self
    }

    /// Sets the paint brush for the button.
    pub fn set_paint(&mut self, paint: impl Into<PaintBrush>) {
        self.paint = Some(paint.into());
        self.dirty = true;
    }
}

impl<T, M> Shard<T, M> for Button<T, M> {
    fn dirty(&self) -> bool {
        self.dirty || Shard::<T, M>::dirty(&self.label)
    }

    fn lifecycle(
        &mut self,
        _: &mut super::lifecycle::LifeCycleCtx,
        event: &LifeCycle,
        _: &Data<T, M>,
    ) {
        match event {
            LifeCycle::Init { .. } => {}
            LifeCycle::HotChanged(_) => self.dirty = true,
            _ => {}
        }
    }
    fn layout(&mut self, ctx: &mut super::LayoutCtx) -> super::ShardLayout {
        const PAD: f32 = 8.;
        let constraints = ctx.constraints();
        let min = constraints.min();
        let max = constraints.max();

        let label_laid = ctx.with_constraints(BoundingConstraints::from_max(max), |ctx| {
            Shard::<T, M>::layout(&mut self.label, ctx)
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

    fn on_event(
        &mut self,
        event_ctx: &mut EventCtx,
        event: &ShardEvent,
        _app_ctx: &mut Data<T, M>,
    ) {
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

    fn render(
        &mut self,
        ctx: &mut RenderCtx,
        data: &Data<T, M>,
    ) -> Option<(crate::Point, BoundingRect)> {
        let layout = ctx.layout();
        let is_active = ctx.is_active();
        let is_hot = ctx.is_hot();

        let w = layout.bounds.width();
        let h = layout.bounds.height();

        let paint = match self.paint {
            None => &data.env().get(DEFAULT_BUTTON_COLOR).into(),
            Some(ref p) => p,
        };

        ctx.fill(paint, &Rect::new_rect(w, h).round(self.radius));

        let overlay_alpha = if !is_active { 20 } else { 36 };

        self.label.render(ctx, data);

        if is_hot {
            ctx.fill(
                &paint.with_alpha(overlay_alpha),
                &Rect::new_rect(w, h).round(self.radius),
            );
        }
        self.dirty = false;
        None
    }
}
