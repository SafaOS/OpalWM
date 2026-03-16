use crate::{
    AppCtx, BoundingRect, EventCtx, Point, ShardEvent,
    render::TinySkiaCanvas,
    shards::{LayoutCtx, RenderCtx, Shard, ShardLayout},
};

trait ExtShard<Ctx: AppCtx, Inner: Shard<Ctx> + ?Sized> {
    fn inner(&self) -> &Inner;
    fn inner_mut(&mut self) -> &mut Inner;

    #[inline(always)]
    fn layout(&mut self, ctx: &mut LayoutCtx) -> ShardLayout {
        self.inner_mut().layout(ctx)
    }
    #[inline(always)]
    fn render(&mut self, ctx: &mut RenderCtx) -> Option<(Point, BoundingRect)> {
        self.inner_mut().render(ctx)
    }
    #[inline(always)]
    fn on_event(&mut self, event_ctx: &mut EventCtx, event: &ShardEvent, app_ctx: &mut Ctx) {
        self.inner_mut().on_event(event_ctx, event, app_ctx)
    }
    #[inline(always)]
    fn on_message(
        &mut self,
        layout: &ShardLayout,
        pos: Point,
        context: &mut Ctx,
        message: &Ctx::Message,
    ) {
        self.inner_mut().on_message(layout, pos, context, message);
    }
    #[inline(always)]
    fn on_ctx_update(&mut self, context: &Ctx) {
        self.inner_mut().on_ctx_update(context)
    }
    #[inline(always)]
    fn dirty(&self) -> bool {
        self.inner().dirty()
    }
}

/// Avoids stupid compiler error preventing implementing Shard<Ctx> for all ExtShard<Ctx>
macro_rules! ext_impl {
    ($ty:ty, $($rest:tt)*) => {
        impl<Ctx: AppCtx, $($rest)*> Shard<Ctx> for $ty {
            #[inline(always)]
            fn dirty(&self) -> bool {
                <Self as ExtShard<Ctx, _>>::dirty(self)
            }
            #[inline(always)]
            fn layout(&mut self, ctx: &mut LayoutCtx) -> ShardLayout {
                <Self as ExtShard<Ctx, _>>::layout(self, ctx)
            }
            #[inline(always)]
            fn on_ctx_update(&mut self, context: &Ctx) {
                <Self as ExtShard<Ctx, _>>::on_ctx_update(self, context)
            }
            #[inline(always)]
            fn on_event(
                &mut self,
                event_ctx: &mut EventCtx,
                event: &ShardEvent,
                app_ctx: &mut Ctx,
            ) {
                <Self as ExtShard<Ctx, _>>::on_event(self, event_ctx, event, app_ctx)
            }
            #[inline(always)]
            fn on_message(
                &mut self,
                layout: &ShardLayout,
                pos: Point,
                context: &mut Ctx,
                message: &<Ctx as AppCtx>::Message,
            ) {
                <Self as ExtShard<Ctx, _>>::on_message(self, layout, pos, context, message)
            }
            #[inline(always)]
            fn render(&mut self, ctx: &mut RenderCtx) -> Option<(Point, BoundingRect)> {
                <Self as ExtShard<Ctx, _>>::render(self, ctx)
            }
        }
    };
}

pub trait ShardsExt<Ctx: AppCtx>: Sized + Shard<Ctx> {
    /// Registers a callback to be executed when the shard is clicked.
    fn on_click<F: FnMut(&EventCtx, &mut Ctx, &mut Self) + 'static>(
        self,
        f: F,
    ) -> OnClick<Ctx, Self> {
        OnClick {
            shard: self,
            action: Box::new(f),
        }
    }

    fn on_update<F: FnMut(&Ctx, &mut Self) + 'static>(self, f: F) -> OnUpdate<Ctx, Self> {
        OnUpdate {
            shard: self,
            action: Box::new(f),
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

    fn cached(self) -> CachedShard<Self> {
        CachedShard {
            shard: self,
            cache: None,
        }
    }
}

impl<Ctx: AppCtx, S: Shard<Ctx>> ShardsExt<Ctx> for S {}

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

impl<Ctx: AppCtx, S: Shard<Ctx>> ExtShard<Ctx, S> for SizedBox<S> {
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
        let min_width = self.max_width.map_or(min_width, |m| m.min(min_width));
        let min_height = self.max_height.map_or(min_height, |m| m.min(min_height));

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
ext_impl!(SizedBox<S>, S: Shard<Ctx>);

/// Represents a clickable shard.
pub struct OnClick<Ctx: AppCtx, S: Shard<Ctx>> {
    shard: S,
    action: Box<dyn FnMut(&EventCtx, &mut Ctx, &mut S) + 'static>,
}

impl<Ctx: AppCtx, S: Shard<Ctx>> ExtShard<Ctx, S> for OnClick<Ctx, S> {
    #[inline]
    fn inner(&self) -> &S {
        &self.shard
    }
    #[inline]
    fn inner_mut(&mut self) -> &mut S {
        &mut self.shard
    }
    #[inline]
    fn on_event(&mut self, event_ctx: &mut EventCtx, event: &ShardEvent, app_ctx: &mut Ctx) {
        match event {
            ShardEvent::MouseRelease(_) => {
                (self.action)(event_ctx, app_ctx, &mut self.shard);
            }
            _ => {}
        }

        self.shard.on_event(event_ctx, event, app_ctx)
    }
}

impl_deref!(OnClick<Ctx, S>, S, Ctx: AppCtx, S: Shard<Ctx>);
ext_impl!(OnClick<Ctx, S>, S: Shard<Ctx>);

/// Represents an action that can be performed when the Ctx is updated.
pub struct OnUpdate<Ctx: AppCtx, S: Shard<Ctx>> {
    shard: S,
    action: Box<dyn FnMut(&Ctx, &mut S) + 'static>,
}

impl<Ctx: AppCtx, S: Shard<Ctx>> ExtShard<Ctx, S> for OnUpdate<Ctx, S> {
    fn inner(&self) -> &S {
        &self.shard
    }
    fn inner_mut(&mut self) -> &mut S {
        &mut self.shard
    }

    #[inline(always)]
    fn on_ctx_update(&mut self, context: &Ctx) {
        (self.action)(context, &mut self.shard);
        self.shard.on_ctx_update(context);
    }
}

impl_deref!(OnUpdate<Ctx, S>, S, Ctx: AppCtx, S: Shard<Ctx>);
ext_impl!(OnUpdate<Ctx, S>, S: Shard<Ctx>);

/// a Shard that is equalivent to [`S`], but instead of re-drawing the shard every render, it's pixels is cached and copied.
pub struct CachedShard<S: ?Sized> {
    cache: Option<TinySkiaCanvas>,
    shard: S,
}

impl<S: ?Sized> CachedShard<S> {
    pub fn cache(&self) -> Option<&TinySkiaCanvas> {
        self.cache.as_ref()
    }

    pub fn cache_mut(&mut self) -> Option<&mut TinySkiaCanvas> {
        self.cache.as_mut()
    }
}

impl<Ctx: AppCtx, S: Shard<Ctx> + ?Sized> ExtShard<Ctx, S> for CachedShard<S> {
    fn inner(&self) -> &S {
        &self.shard
    }

    fn inner_mut(&mut self) -> &mut S {
        &mut self.shard
    }

    fn render(&mut self, ctx: &mut RenderCtx) -> Option<(Point, BoundingRect)> {
        let mut is_dirty = self.cache.is_none() || self.shard.dirty() || ctx.state_changed();

        let layout = ctx.layout();
        let bounds = layout.bounds;

        let new_canvas =
            || TinySkiaCanvas::new(bounds.width().ceil() as u32, bounds.height().ceil() as u32);
        let cache = self.cache.get_or_insert_with(new_canvas);

        if (cache.width() as f32) != bounds.width().ceil()
            || (cache.height() as f32) != bounds.height().ceil()
        {
            *cache = new_canvas();
            is_dirty = true;
        }

        let results;
        if is_dirty {
            cache.pixmap.fill(tiny_skia::Color::TRANSPARENT);
            results = ctx.with_canvas(cache, |ctx| {
                ctx.move_to(Point::default());
                self.shard.render(ctx)
            });
        } else {
            results = None;
        }

        ctx.fill_with_pixmap(&cache.pixmap);
        results
    }
}

impl_deref!(CachedShard<S>, S, S);
ext_impl!(CachedShard<S>, S: Shard<Ctx> + ?Sized);
