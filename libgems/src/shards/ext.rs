use crate::{
    BoundingRect, Data, EventCtx, Padding, Point, ShardEvent,
    render::{BoundingConstraints, CanvasCache, PaintBrush, shapes::Rect},
    shards::{
        AxisAlign, DamageArea, LayoutCtx, LifeCycleCtx, MsgCtx, RenderCtx, Shard, ShardLayout,
        ShardNode, ShardState, UpdateCtx, lifecycle::LifeCycle,
    },
};

trait ExtShard<S, M, Inner: Shard<S, M> + ?Sized> {
    fn inner(&self) -> &Inner;
    fn inner_mut(&mut self) -> &mut Inner;

    #[inline(always)]
    fn should_relayout(&self) -> bool {
        self.inner().should_relayout()
    }
    #[inline(always)]
    fn layout(&mut self, ctx: &mut LayoutCtx) -> ShardLayout {
        self.inner_mut().layout(ctx)
    }
    #[inline(always)]
    fn render(&mut self, ctx: &mut RenderCtx, data: &Data<S, M>) {
        self.inner_mut().render(ctx, data)
    }
    #[inline(always)]
    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &Data<S, M>) {
        self.inner_mut().lifecycle(ctx, event, data)
    }
    #[inline(always)]
    fn on_event(&mut self, event_ctx: &mut EventCtx, event: &ShardEvent, data: &mut Data<S, M>) {
        self.inner_mut().on_event(event_ctx, event, data)
    }
    #[inline(always)]
    fn on_message(&mut self, ctx: &mut MsgCtx, data: &mut Data<S, M>, message: &M) {
        self.inner_mut().on_message(ctx, data, message);
    }
    #[inline(always)]
    fn with_children(&mut self, f: &mut dyn FnMut(&mut dyn Iterator<Item = &mut ShardNode<S, M>>)) {
        self.inner_mut().with_children(f)
    }
    #[inline(always)]
    fn on_ctx_update(&mut self, update_ctx: &mut UpdateCtx, context: &Data<S, M>) {
        self.inner_mut().on_ctx_update(update_ctx, context)
    }
    #[inline(always)]
    fn dirty(&self) -> bool {
        self.inner().dirty()
    }
}

#[macro_export(local_inner_macros)]
/// Avoids stupid compiler error preventing implementing Shard<Ctx> for all ExtShard<Ctx>
macro_rules! ext_impl {
    ($ty:ty, $($rest:tt)*) => {
        impl<T, M, $($rest)*> Shard<T, M> for $ty {
            #[inline(always)]
            fn dirty(&self) -> bool {
                <Self as ExtShard<T, M, _>>::dirty(self)
            }
            #[inline(always)]
            fn should_relayout(&self) -> bool {
                <Self as ExtShard<T, M, _>>::should_relayout(self)
            }
            #[inline(always)]
            fn layout(&mut self, ctx: &mut LayoutCtx) -> ShardLayout {
                <Self as ExtShard<T, M, _>>::layout(self, ctx)
            }
            #[inline(always)]
            fn on_ctx_update(&mut self, update_ctx: &mut UpdateCtx, context: &Data<T, M>) {
                <Self as ExtShard<T, M, _>>::on_ctx_update(self, update_ctx, context)
            }
            #[inline(always)]
            fn with_children(&mut self, f: &mut dyn FnMut(&mut dyn Iterator<Item = &mut ShardNode<T, M>>)) {
                  <Self as ExtShard<T, M, _>>::with_children(self, f)
            }
            #[inline(always)]
            fn on_event(
                &mut self,
                event_ctx: &mut EventCtx,
                event: &ShardEvent,
                data: &mut Data<T, M>,
            ) {
                <Self as ExtShard<T, M, _>>::on_event(self, event_ctx, event, data)
            }
            #[inline(always)]
            fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &Data<T, M>) {
                <Self as ExtShard<T, M, _>>::lifecycle(self, ctx, event, data)
            }
            #[inline(always)]
            fn on_message(
                &mut self,
                ctx: &mut MsgCtx,
                data: &mut Data<T, M>,
                message: &M,
            ) {
                <Self as ExtShard<T, M, _>>::on_message(self, ctx, data, message)
            }
            #[inline(always)]
            fn render(&mut self, ctx: &mut RenderCtx, data: &Data<T, M>) {
                <Self as ExtShard<T, M, _>>::render(self, ctx, data)
            }
        }
    };
}
pub(super) use ext_impl;

pub trait ShardsExt<S = (), M = ()>: Sized + Shard<S, M> {
    /// Registers a callback to be executed when the shard is clicked.
    fn on_click<F: FnMut(&EventCtx, &mut Data<S, M>, &mut Self) + 'static>(
        self,
        f: F,
    ) -> OnClick<S, M, Self> {
        OnClick {
            shard: self,
            action: Box::new(f),
        }
    }

    fn on_update<F: FnMut(&Data<S, M>, &mut Self) + 'static>(self, f: F) -> OnUpdate<S, M, Self> {
        OnUpdate {
            shard: self,
            action: Box::new(f),
        }
    }

    fn on_msg<F: FnMut(&mut MsgCtx, &mut Data<S, M>, &M, &mut Self) + 'static>(
        self,
        f: F,
    ) -> OnMessage<Self, F> {
        OnMessage {
            shard: self,
            action: f,
        }
    }

    fn on_lifecycle<F: FnMut(&mut LifeCycleCtx, &LifeCycle, &mut Self) + 'static>(
        self,
        f: F,
    ) -> OnLifeCycle<Self, F> {
        OnLifeCycle {
            shard: self,
            action: f,
        }
    }

    /// Fixes the width of the shard.
    fn fix_width(self, width: f32) -> SizedBox<Self> {
        SizedBox {
            shard: self,
            max_width: Some(width),
            max_height: None,
            dirty: true,
        }
    }

    /// Fixes the height of the shard.
    fn fix_height(self, height: f32) -> SizedBox<Self> {
        SizedBox {
            shard: self,
            max_width: None,
            max_height: Some(height),
            dirty: true,
        }
    }

    /// Fixes the size of the shard.
    ///
    /// Equivalent to both [`Self::fix_width`] and [`Self::fix_height`].
    fn fix_size(self, width: f32, height: f32) -> SizedBox<Self> {
        SizedBox {
            shard: self,
            max_width: Some(width),
            max_height: Some(height),
            dirty: true,
        }
    }

    /// Pads the given shard with the given padding.
    fn pad(self, padding: Padding) -> PaddedBox<Self> {
        PaddedBox {
            shard: self,
            padding,
        }
    }

    /// Similar to [`Self::pad`] but adds padding to the total size instead of doing special padding.
    ///
    /// May give different results
    fn size_pad(self, padding: Padding) -> SizePaddedBox<Self> {
        SizePaddedBox {
            shard: self,
            padding,
        }
    }

    /// cross-axis Aligns the given shard to the given alignment ofcourse.
    fn align(self, align: AxisAlign) -> AlignedBox<Self> {
        AlignedBox { shard: self, align }
    }

    fn cached(self) -> CachedShard<Self> {
        CachedShard {
            shard: self,
            cache: None,
        }
    }

    /// Constructs a new [`Container`] around self with a given background.
    fn background(self, paint: impl Into<PaintBrush<'static>>) -> Container<Self> {
        Container {
            background: paint.into(),
            radius: 0.,
            border: None,
            shard: self,
        }
    }
}

impl<T, M, S: Shard<T, M>> ShardsExt<T, M> for S {}

#[macro_export(local_inner_macros)]
macro_rules! impl_deref {
    ($t:ty,$o:ty,$($generics:tt)*) => {
        impl<$($generics)*> std::ops::Deref for $t {
            type Target = $o;
            fn deref(&self) -> &Self::Target {
                &self.shard
            }
        }

        impl<$($generics)*> std::ops::DerefMut for $t {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.shard
            }
        }
    };
}

pub(super) use impl_deref;
use tiny_skia::Pixmap;

/// Represents a shard that cross-axis aligns its child.
pub struct AlignedBox<S> {
    shard: S,
    align: AxisAlign,
}
impl<T, M, S: Shard<T, M>> ExtShard<T, M, S> for AlignedBox<S> {
    fn inner(&self) -> &S {
        &self.shard
    }

    fn inner_mut(&mut self) -> &mut S {
        &mut self.shard
    }

    fn layout(&mut self, ctx: &mut LayoutCtx) -> ShardLayout {
        let mut a_layout = self.shard.layout(ctx);
        a_layout.align = self.align;
        a_layout
    }
}
impl_deref!(AlignedBox<S>, S, S);
ext_impl!(AlignedBox<S>, S: Shard<T, M>);

/// Represents a shard that pads its child's surrounding.
pub struct PaddedBox<S> {
    shard: S,
    padding: Padding,
}

impl<T, M, S: Shard<T, M>> ExtShard<T, M, S> for PaddedBox<S> {
    fn inner(&self) -> &S {
        &self.shard
    }

    fn inner_mut(&mut self) -> &mut S {
        &mut self.shard
    }

    fn layout(&mut self, ctx: &mut LayoutCtx) -> ShardLayout {
        let constr = ctx.constraints();

        let max_w = constr.max().width() - self.padding.padded_width();
        let max_h = constr.max().height() - self.padding.padded_height();

        let min_w = constr.min().width().min(max_w);
        let min_h = constr.min().height().min(max_h);

        let mut a_layout = ctx.with_constraints(
            BoundingConstraints::new(
                BoundingRect::new(min_w, min_h),
                BoundingRect::new(max_w, max_h),
            ),
            |ctx| self.shard.layout(ctx),
        );
        a_layout.padding = self.padding;
        a_layout
    }
}

impl_deref!(PaddedBox<S>, S, S);
ext_impl!(PaddedBox<S>, S: Shard<T, M>);

/// Represents a shard that pads its child's surrounding.
pub struct SizePaddedBox<S> {
    shard: S,
    padding: Padding,
}

impl<T, M, S: Shard<T, M>> ExtShard<T, M, S> for SizePaddedBox<S> {
    fn inner(&self) -> &S {
        &self.shard
    }

    fn inner_mut(&mut self) -> &mut S {
        &mut self.shard
    }

    fn layout(&mut self, ctx: &mut LayoutCtx) -> ShardLayout {
        let constr = ctx.constraints();

        let max_w = constr.max().width() - self.padding.padded_width();
        let max_h = constr.max().height() - self.padding.padded_height();

        let min_w = constr.min().width().min(max_w);
        let min_h = constr.min().height().min(max_h);

        let mut a_layout = ctx.with_constraints(
            BoundingConstraints::new(
                BoundingRect::new(min_w, min_h),
                BoundingRect::new(max_w, max_h),
            ),
            |ctx| self.shard.layout(ctx),
        );

        let bounds = a_layout.bounds;
        a_layout.bounds = BoundingRect::new(
            bounds.width() + self.padding.padded_width(),
            bounds.height() + self.padding.padded_height(),
        );
        a_layout
    }

    fn render(&mut self, ctx: &mut RenderCtx, data: &Data<T, M>) {
        let ctx = ctx.move_to(Point::new(self.padding.left, self.padding.right));
        self.shard.render(ctx, data)
    }
}

impl_deref!(SizePaddedBox<S>, S, S);
ext_impl!(SizePaddedBox<S>, S: Shard<T, M>);

/// Represents a shard that limits its child's size.
pub struct SizedBox<S> {
    shard: S,
    max_width: Option<f32>,
    max_height: Option<f32>,
    dirty: bool,
}

impl<S> SizedBox<S> {
    pub fn set_width(&mut self, width: f32) {
        self.max_width = Some(width);
        self.dirty = true;
    }

    pub fn set_height(&mut self, height: f32) {
        self.max_height = Some(height);
        self.dirty = true;
    }

    pub fn fix_width(mut self, width: f32) -> Self {
        self.max_width = Some(width);
        self
    }

    pub fn fix_height(mut self, height: f32) -> Self {
        self.max_height = Some(height);
        self
    }

    pub fn fix_size(mut self, width: f32, height: f32) -> Self {
        self.max_width = Some(width);
        self.max_height = Some(height);
        self
    }
}

impl<T, M, S: Shard<T, M>> ExtShard<T, M, S> for SizedBox<S> {
    fn inner(&self) -> &S {
        &self.shard
    }

    fn inner_mut(&mut self) -> &mut S {
        &mut self.shard
    }
    #[inline(always)]
    fn dirty(&self) -> bool {
        self.dirty || self.shard.dirty()
    }

    fn layout(&mut self, ctx: &mut super::LayoutCtx) -> super::ShardLayout {
        let constraints = ctx.constraints();
        let max_width = constraints.max().width();
        let max_height = constraints.max().height();
        let max_width = self.max_width.map_or(max_width, |m| m.min(max_width));
        let max_height = self.max_height.map_or(max_height, |m| m.min(max_height));

        let min_width = constraints.min().width();
        let min_height = constraints.min().height();
        let min_width = self
            .max_width
            .map_or(min_width, |m| m.max(min_width).min(max_width));
        let min_height = self
            .max_height
            .map_or(min_height, |m| m.max(min_height).min(max_height));

        self.dirty = false;
        ctx.with_constraints(
            crate::render::BoundingConstraints::new(
                BoundingRect::new(min_width, min_height),
                BoundingRect::new(max_width, max_height),
            ),
            |s_ctx| self.shard.layout(s_ctx),
        )
    }
}

impl_deref!(SizedBox<S>, S, S);
ext_impl!(SizedBox<S>, S: Shard<T, M>);

/// Represents a clickable shard.
pub struct OnClick<T, M, S: Shard<T, M>> {
    shard: S,
    action: Box<dyn FnMut(&EventCtx, &mut Data<T, M>, &mut S) + 'static>,
}

impl<T, M, S: Shard<T, M>> ExtShard<T, M, S> for OnClick<T, M, S> {
    #[inline]
    fn inner(&self) -> &S {
        &self.shard
    }
    #[inline]
    fn inner_mut(&mut self) -> &mut S {
        &mut self.shard
    }
    #[inline]
    fn on_event(&mut self, event_ctx: &mut EventCtx, event: &ShardEvent, data: &mut Data<T, M>) {
        match event {
            ShardEvent::MouseRelease(_) => {
                (self.action)(event_ctx, data, &mut self.shard);

                if self.shard.dirty() {
                    event_ctx.request_redraw();
                }
            }
            _ => {}
        }

        self.shard.on_event(event_ctx, event, data)
    }
}

impl_deref!(OnClick<T, M, S>, S, T, M, S: Shard<T, M>);
ext_impl!(OnClick<T, M, S>, S: Shard<T, M>);

/// Represents an action that can be performed when the Ctx is updated.
pub struct OnUpdate<T, M, S: Shard<T, M>> {
    shard: S,
    action: Box<dyn FnMut(&Data<T, M>, &mut S) + 'static>,
}

impl<T, M, S: Shard<T, M>> ExtShard<T, M, S> for OnUpdate<T, M, S> {
    fn inner(&self) -> &S {
        &self.shard
    }
    fn inner_mut(&mut self) -> &mut S {
        &mut self.shard
    }

    #[inline(always)]
    fn on_ctx_update(&mut self, update_ctx: &mut UpdateCtx, context: &Data<T, M>) {
        (self.action)(context, &mut self.shard);
        if self.shard.dirty() {
            update_ctx.request_redraw();
        }

        self.shard.on_ctx_update(update_ctx, context);
    }
}

impl_deref!(OnUpdate<T, M, S>, S, T, M, S: Shard<T, M>);
ext_impl!(OnUpdate<T, M, S>, S: Shard<T, M>);

/// Represents an action that can be performed when a LifeCycle Event is received.
pub struct OnLifeCycle<S, F: FnMut(&mut LifeCycleCtx, &LifeCycle, &mut S) + 'static> {
    shard: S,
    action: F,
}

impl<T, M, S: Shard<T, M>, F: FnMut(&mut LifeCycleCtx, &LifeCycle, &mut S) + 'static>
    ExtShard<T, M, S> for OnLifeCycle<S, F>
{
    fn inner(&self) -> &S {
        &self.shard
    }
    fn inner_mut(&mut self) -> &mut S {
        &mut self.shard
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &Data<T, M>) {
        (self.action)(ctx, event, &mut self.shard);
        if self.shard.dirty() {
            ctx.request_redraw();
        }
        self.shard.lifecycle(ctx, event, data);
    }
}

impl_deref!(OnLifeCycle<S, F>, S, S, F: FnMut(&mut LifeCycleCtx, &LifeCycle, &mut S) + 'static);
ext_impl!(OnLifeCycle<S, F>, S: Shard<T, M>, F: FnMut(&mut LifeCycleCtx, &LifeCycle, &mut S) + 'static);

/// Represents an action that can be performed when a Message is received.
pub struct OnMessage<S, F> {
    shard: S,
    action: F,
}

impl<T, M, S: Shard<T, M>, F: FnMut(&mut MsgCtx, &mut Data<T, M>, &M, &mut S)> ExtShard<T, M, S>
    for OnMessage<S, F>
{
    fn inner(&self) -> &S {
        &self.shard
    }
    fn inner_mut(&mut self) -> &mut S {
        &mut self.shard
    }

    fn on_message(&mut self, ctx: &mut MsgCtx, data: &mut Data<T, M>, message: &M) {
        (self.action)(ctx, data, message, &mut self.shard);
        if self.shard.dirty() {
            ctx.request_redraw();
        }
        self.shard.on_message(ctx, data, message);
    }
}

impl_deref!(OnMessage<S, F>, S, S, F);
ext_impl!(OnMessage<S, F>, S: Shard<T, M>, F: FnMut(&mut MsgCtx, &mut Data<T, M>, &M, &mut S));

/// a Shard that is equalivent to [`S`], but instead of re-drawing the shard every render, it's pixels is cached and copied.
pub struct CachedShard<S: ?Sized> {
    cache: Option<Pixmap>,
    shard: S,
}

impl<S: ?Sized> CachedShard<S> {
    pub fn cache(&self) -> Option<&Pixmap> {
        self.cache.as_ref()
    }

    pub fn cache_mut(&mut self) -> Option<&mut Pixmap> {
        self.cache.as_mut()
    }
}

impl<S: ?Sized> CachedShard<S> {
    pub fn render_to_cache<T, M>(
        &mut self,
        canvas_cache: &mut CanvasCache,
        layout: &ShardLayout,
        state: &ShardState,
        data: &Data<T, M>,
        damage: &mut DamageArea,
    ) where
        S: Shard<T, M>,
    {
        let mut is_dirty = self.cache.is_none() || self.shard.dirty() || state.state_changed;
        let bounds = layout.bounds;

        let new_canvas = || {
            Pixmap::new(
                bounds.width().ceil().max(1.) as u32,
                bounds.height().ceil().max(1.) as u32,
            )
            .expect("Failed to construct pixmap")
        };
        let cache = self.cache.get_or_insert_with(new_canvas);

        if (cache.width() as f32) != bounds.width().ceil()
            || (cache.height() as f32) != bounds.height().ceil()
        {
            *cache = new_canvas();
            is_dirty = true;
        }

        if is_dirty {
            cache.fill(tiny_skia::Color::TRANSPARENT);
            let mut canvas_ctx = crate::render::CanvasContext::new(canvas_cache, cache.as_mut());
            let mut ctx = RenderCtx::new(
                crate::render::Vec2::new(0., 0.),
                &mut canvas_ctx,
                state,
                layout,
                damage,
            );
            self.shard.render(&mut ctx, data);
        }
    }
}

impl<T, M, S: Shard<T, M> + ?Sized> ExtShard<T, M, S> for CachedShard<S> {
    fn inner(&self) -> &S {
        &self.shard
    }

    fn inner_mut(&mut self) -> &mut S {
        &mut self.shard
    }

    fn render(&mut self, ctx: &mut RenderCtx, data: &Data<T, M>) {
        let mut is_dirty = self.cache.is_none() || self.shard.dirty() || ctx.state_changed();

        let layout = ctx.layout();
        let bounds = layout.bounds;

        let new_canvas = || {
            Pixmap::new(
                bounds.width().ceil().max(1.) as u32,
                bounds.height().ceil().max(1.) as u32,
            )
            .expect("Failed to construct pixmap")
        };
        let cache = self.cache.get_or_insert_with(new_canvas);

        if (cache.width() as f32) != bounds.width().ceil()
            || (cache.height() as f32) != bounds.height().ceil()
        {
            *cache = new_canvas();
            is_dirty = true;
        }

        if is_dirty {
            cache.fill(tiny_skia::Color::TRANSPARENT);
            ctx.with_pixmap(cache.as_mut(), |ctx| {
                ctx.move_to_abs(Point::default());
                self.shard.render(ctx, data)
            });
        }
        ctx.fill_with_pixmap(cache.as_ref());
    }
}

impl_deref!(CachedShard<S>, S, S: ?Sized);
ext_impl!(CachedShard<S>, S: Shard<T, M> + ?Sized);

/// Represents a Box Container around S.
///
/// Currently supports adding backgrounds and such.
#[derive(Debug)]
pub struct Container<S> {
    background: PaintBrush<'static>,
    border: Option<(PaintBrush<'static>, f32)>,
    radius: f32,
    shard: S,
}

impl<S> Container<S> {
    #[inline]
    pub fn background(mut self, background: impl Into<PaintBrush<'static>>) -> Self {
        self.background = background.into();
        self
    }

    #[inline]
    pub fn round(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }
    #[inline]
    pub fn border(mut self, color: impl Into<PaintBrush<'static>>, thickness: f32) -> Self {
        self.border = Some((color.into(), thickness));
        self
    }
}

impl<T, M, S: Shard<T, M>> ExtShard<T, M, S> for Container<S> {
    fn inner(&self) -> &S {
        &self.shard
    }
    fn inner_mut(&mut self) -> &mut S {
        &mut self.shard
    }

    fn render(&mut self, ctx: &mut RenderCtx, data: &Data<T, M>) {
        let bounds = ctx.layout().bounds;
        let shape = Rect::new_rect(bounds.width(), bounds.height()).round(self.radius);
        ctx.fill(&self.background, &shape);
        if let Some((color, thickness)) = &self.border {
            ctx.stroke(&color, *thickness, &shape);
        }
        self.shard.render(ctx, data)
    }
}
impl_deref!(Container<S>, S, S);
ext_impl!(Container<S>, S: Shard<T, M>);
