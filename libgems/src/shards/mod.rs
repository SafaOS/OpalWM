mod button;
mod container;
pub(crate) mod event;
mod ext;
mod label;
mod primitive;
mod render_ctx;

pub use button::*;
pub use container::*;
use cosmic_text::FontSystem;
pub use ext::*;
pub use label::*;
pub use render_ctx::*;

use crate::{
    AppCtx, ShardEvent,
    render::{
        Alignment, BoundingConstraints, BoundingRect, CanvasCache, CanvasContext, NoopCanvas,
        Padding, Point,
    },
    shards::event::EventCtx,
};

#[derive(Debug)]
/// Describes a layout request context.
pub struct LayoutCtx<'f> {
    pub(crate) font_system: &'f mut FontSystem,
    pub(crate) constraints: BoundingConstraints,
}

impl<'f> LayoutCtx<'f> {
    pub fn with_constraints<R>(
        &mut self,
        constraints: BoundingConstraints,
        f: impl FnOnce(&mut Self) -> R,
    ) -> R {
        let old = core::mem::replace(&mut self.constraints, constraints);
        let r = f(self);
        self.constraints = old;
        r
    }

    /// Returns the font system used for rendering and shaping text.
    pub fn font_system(&mut self) -> &mut FontSystem {
        self.font_system
    }

    /// Returns the bounds constraints of the requested layout.
    pub fn constraints(&self) -> BoundingConstraints {
        self.constraints
    }

    /// Returns the max bounds of the requested layout.
    pub fn max_box(&self) -> BoundingRect {
        self.constraints().max()
    }

    /// Returns the minimum bounds of the requested layout.
    pub fn min_box(&self) -> BoundingRect {
        self.constraints().min()
    }
}

/// A shard is a widget.
pub trait Shard<Context: AppCtx> {
    /// Lays out [`Self`] according to ctxt, returning the layout.
    fn layout(&mut self, ctx: &mut LayoutCtx) -> ShardLayout;
    /// Renders self at the given `pos` with the given `layout` (results of calling [`Shard::layout`]) into the given [`Canvas`].
    ///
    /// Typically damages the whole width*height area within in that case return None,
    /// however if the damage is at a specific subpoint from the hitbox return Some((point from self, range bounds)).
    fn render(&mut self, ctx: &mut RenderCtx) -> Option<(Point, BoundingRect)>;
    /// Executed on a new event, regardless if the event is relevant to the shard.
    ///
    /// Is followed by [`Shard::on_update`] lifecycle.
    fn on_event(&mut self, event_ctx: &mut EventCtx, event: &ShardEvent, app_ctx: &mut Context) {
        _ = event_ctx;
        _ = event;
        _ = app_ctx;
    }
    /// Executed on a new message.
    ///
    /// Is followed by [`Shard::on_update`] lifecycle.
    fn on_message(
        &mut self,
        layout: &ShardLayout,
        pos: Point,
        context: &mut Context,
        message: &Context::Message,
    ) {
        _ = pos;
        _ = context;
        _ = message;
        _ = layout;
    }
    /// Executed whenever `context` is potentially muttated.
    fn on_ctx_update(&mut self, context: &Context) {
        _ = context;
    }
    fn dirty(&self) -> bool;
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Defines the layout of a [`Shard`].
pub struct ShardLayout {
    bounds: BoundingRect,
    padding: Padding,
    alignment: Alignment,
}

impl ShardLayout {
    /// Returns the layout bounds with padding added.
    pub const fn bounds_with_padding(&self) -> BoundingRect {
        BoundingRect::new(
            self.bounds.width() + self.padding.left + self.padding.right,
            self.bounds.height() + self.padding.top + self.padding.bottom,
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Describes the state of a [`Shard`].
///
/// eg. hot/active/disabled.
pub struct ShardState {
    /// Whether the shard is active, eg. button is pressed.
    is_active: bool,
    /// Whether the shard is disabled.
    is_disabled: bool,
    /// The hot state of the shard, eg. mouse is hovering over it.
    is_hot: bool,
    /// Whether the shard's state has changed.
    state_changed: bool,
}

impl ShardState {
    /// Sets the shard's active state.
    pub fn set_active(&mut self, active: bool) {
        self.state_changed |= core::mem::replace(&mut self.is_active, active) != active;
    }

    /// Sets the shard's disabled state.
    ///
    /// eg. button is disabled, and cannot be pressed.
    pub fn set_disabled(&mut self, disabled: bool) {
        self.state_changed |= core::mem::replace(&mut self.is_disabled, disabled) != disabled;
    }
    /// Returns whether the shard is active, manually updated by [`Self::set_active`].
    ///
    /// eg. button is pressed.
    pub fn is_active(&self) -> bool {
        self.is_active
    }
    /// Returns whether the shard is disabled, manually updated by [`Self::set_disabled`].
    ///
    /// eg. button is disabled, and cannot be pressed.
    pub fn is_disabled(&self) -> bool {
        self.is_disabled
    }
    /// Returns whether the shard is hot, automatically updated according to the mouse position.
    ///
    /// eg. button is hovered.
    pub fn is_hot(&self) -> bool {
        self.is_hot
    }

    pub(crate) fn set_hot(&mut self, hot: bool) {
        self.state_changed |= core::mem::replace(&mut self.is_hot, hot) != hot;
    }
}

impl Default for ShardState {
    fn default() -> Self {
        Self {
            is_active: false,
            is_disabled: false,
            is_hot: false,
            state_changed: false,
        }
    }
}

/// A node in the shard hierarchy.
pub(crate) struct ShardNode<Ctx: AppCtx> {
    state: ShardState,
    /// The origin point of the shard.
    origin: Point,
    /// The last cached layout of the shard.
    layout: Option<ShardLayout>,
    pub(crate) shard: Box<CachedShard<dyn Shard<Ctx>>>,
}

impl<Ctx: AppCtx> ShardNode<Ctx> {
    pub fn layout_ref(&self) -> Option<&ShardLayout> {
        self.layout.as_ref()
    }

    /// Sets the origin point of the shard.
    pub fn plot_at(&mut self, origin: Point) {
        self.state.state_changed |= core::mem::replace(&mut self.origin, origin) != origin;
    }

    /// Sets the layout of the shard.
    pub fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        with_new_layout: impl FnOnce(&mut ShardLayout),
    ) -> (&mut ShardLayout, bool) {
        let mut laid = self.shard.layout(ctx);
        with_new_layout(&mut laid);

        let old_layout = self.layout.replace(laid);
        let is_new = old_layout != Some(laid);

        self.state.state_changed |= is_new;

        (unsafe { self.layout.as_mut().unwrap_unchecked() }, is_new)
    }

    pub fn try_layout(
        &mut self,
        ctx: &mut LayoutCtx,
        with_new_layout: impl FnOnce(&mut ShardLayout),
    ) -> &ShardLayout {
        if let Some(ref layout) = self.layout {
            layout
        } else {
            self.layout(ctx, with_new_layout).0
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.shard.dirty() || self.state.state_changed
    }

    pub fn position(&self) -> Point {
        self.origin
    }

    fn fix_hot<'a>(
        layout: &'a ShardLayout,
        state: &'a mut ShardState,
        shard: &mut CachedShard<dyn Shard<Ctx>>,
        cursor_at: Point,
        origin: Point,
        app_ctx: &mut Ctx,
    ) -> Option<EventCtx<'a>> {
        let is_hot = layout.bounds.contains_point(origin, cursor_at);

        let was_hot = core::mem::replace(&mut state.is_hot, is_hot);
        state.state_changed |= is_hot != was_hot;

        let mut ctx = EventCtx::new(origin, Some(cursor_at), state, layout);
        if is_hot && !was_hot {
            shard.on_event(&mut ctx, &crate::ShardEvent::MouseEnter, app_ctx);
        } else if was_hot {
            shard.on_event(&mut ctx, &crate::ShardEvent::MouseLeave, app_ctx);
        }

        is_hot.then_some(ctx)
    }
    /// Routes an event to the shard, updating state as necessary.
    pub fn route_event(
        &mut self,
        anchor_by: Point,
        event_origin: Option<Point>,
        event: &ShardEvent,
        app_ctx: &mut Ctx,
    ) {
        if let Some(ref layout) = self.layout {
            let is_mouse_event = event.is_mouse_event();
            let origin = self.origin + anchor_by;

            let routed_event;
            if let Some(event_origin) = event_origin {
                if is_mouse_event {
                    routed_event = Self::fix_hot(
                        layout,
                        &mut self.state,
                        &mut *self.shard,
                        event_origin,
                        origin,
                        app_ctx,
                    );
                } else {
                    routed_event = layout.bounds.contains_point(origin, event_origin).then(|| {
                        EventCtx::new(origin, Some(event_origin), &mut self.state, layout)
                    });
                }
            } else {
                routed_event = Some(EventCtx::new(origin, None, &mut self.state, layout));
            }

            if let Some(mut routed_ctx) = routed_event {
                self.shard.on_event(&mut routed_ctx, event, app_ctx);
            }
        }
    }

    /// Routes a message to the shard.
    pub fn route_message(&mut self, anchor_by: Point, app_ctx: &mut Ctx, message: &Ctx::Message) {
        let layout = self
            .layout
            .as_ref()
            .expect("Route message called on non-layedout shard");
        let origin = self.origin + anchor_by;

        self.shard.on_message(layout, origin, app_ctx, message);
    }

    pub fn on_ctx_update(&mut self, app_ctx: &Ctx) {
        self.shard.on_ctx_update(app_ctx);
    }

    pub fn on_relayout(&mut self, parent_is_hot: bool, anchor_by: Point, cursor_at: Option<Point>) {
        let origin = self.origin + anchor_by;
        let layout = self
            .layout
            .as_ref()
            .expect("Refresh state called on non-plotted shard");

        if let Some(cursor_at) = cursor_at {
            self.state
                .set_hot(parent_is_hot && layout.bounds.contains_point(origin, cursor_at));
        } else {
            self.state.set_hot(false);
        }

        self.state.set_active(false);
    }

    pub(crate) fn new<S: Shard<Ctx> + 'static>(shard: S) -> Self {
        Self {
            shard: Box::new(shard.cached()),
            origin: Point::default(),
            layout: None,
            state: ShardState::default(),
        }
    }

    #[inline(always)]
    pub(crate) fn render_as_root(
        &mut self,
        cache: &mut CanvasCache,
    ) -> Option<(Point, BoundingRect)> {
        let mut noop = NoopCanvas;
        let mut canvas_ctx = CanvasContext::new(cache, &mut noop);
        let mut ctx = RenderCtx::new(
            self.origin,
            &mut canvas_ctx,
            &self.state,
            self.layout
                .as_ref()
                .expect("Attempt to render node with no layout"),
        );

        let results = self.shard.render(&mut ctx);
        self.state.state_changed = false;
        results
    }

    #[inline(always)]
    pub fn render(
        &mut self,
        parent_ctx: &mut RenderCtx,
        layout_changed: bool,
        cursor_at: Option<Point>,
    ) -> Option<(Point, BoundingRect)> {
        let parent_origin = parent_ctx.origin();
        if layout_changed {
            self.on_relayout(parent_ctx.is_hot(), parent_origin, cursor_at);
        }

        let layout = self
            .layout
            .as_ref()
            .expect("Attempt to render node with no layout");
        let bounds = layout.bounds;

        let r = parent_ctx.nest_ctx(self.origin, bounds, |ctx| {
            ctx.with_state(&self.state, layout, |ctx| self.shard.render(ctx))
        });
        self.state.state_changed = false;
        r
    }
}

// impl<S: Shard<Ctx> + ?Sized, Ctx: AppCtx> Shard<Ctx> for ShardState<S> {
//     #[inline(always)]
//     fn dirty(&self) -> bool {
//         self.shard.dirty()
//     }

//     #[inline(always)]
//     fn layout(&mut self, constraints: BoundingConstraints) -> ShardLayout {
//         self.shard.layout(constraints)
//     }

//     #[inline(always)]
//     fn on_event(
//         &mut self,
//         layout: &ShardLayout,
//         pos: Point,
//         app_ctx: &mut Ctx,
//         event_ctx: &EventCtx,
//     ) {
//         self.shard.on_event(layout, pos, app_ctx, event_ctx)
//     }

//     #[inline(always)]
//     fn on_ctx_update(&mut self, context: &Ctx) {
//         self.shard.on_ctx_update(context);
//     }

//     #[inline(always)]
//     fn on_message(
//         &mut self,
//         layout: &ShardLayout,
//         pos: Point,
//         state: &mut Ctx,
//         message: &Ctx::Message,
//     ) {
//         self.shard.on_message(layout, pos, state, message);
//     }

//     #[inline(always)]
//     fn render(
//         &mut self,
//         pos: Point,
//         layout: &ShardLayout,
//         parent_canvas: &mut CanvasContext,
//     ) -> Option<(Point, BoundingRect)> {
//         let is_dirty = self.dirty();
//         let bounds = layout.bounds;
//         let width = bounds.width().ceil() as i32 as u32;
//         let height = bounds.height().ceil() as i32 as u32;

//         let mut canvas = self.cached_canvas.get_or_insert_with(|| {
//             assert!(is_dirty, "Shard should be dirty by default");
//             TinySkiaCanvas::new(width, height)
//         });

//         if canvas.width() < width || canvas.height() < height {
//             assert!(is_dirty, "Shard should be dirty on change");
//             canvas = self
//                 .cached_canvas
//                 .insert(TinySkiaCanvas::new(width, height));
//         }

//         if is_dirty {
//             let mut context = CanvasContext::new(parent_canvas.cache, canvas);
//             self.shard.render(Point::new(0., 0.), layout, &mut context);
//         }

//         // Render into parent.
//         parent_canvas.fill_with_pixmap(pos, &canvas.pixmap);
//         None
//     }
// }

// /// Describes a Mouse Interaction event within a [`Shard`].
// #[derive(Debug, Clone, Copy)]
// pub enum MouseInteraction {
//     MouseEnter,
//     MouseLeave,
//     Buttons(HeldMouseButtons),
// }

// /// Makes a [`Shard`] interactable with the mouse.
// ///
// /// Intercepts mouse events.
// #[derive(Debug)]
// pub struct Interactable<S: ?Sized, OnEvent> {
//     mouse_in: bool,
//     on_event: Option<OnEvent>,
//     shard: S,
// }

// impl<S: ?Sized, OnEvent> Deref for Interactable<S, OnEvent> {
//     type Target = S;
//     fn deref(&self) -> &Self::Target {
//         &self.shard
//     }
// }

// impl<S: ?Sized, OnEvent> DerefMut for Interactable<S, OnEvent> {
//     fn deref_mut(&mut self) -> &mut Self::Target {
//         &mut self.shard
//     }
// }

// impl<S, OnEvent> Interactable<S, OnEvent> {
//     pub fn new(shard: S) -> Self {
//         Self {
//             mouse_in: false,
//             on_event: None,
//             shard,
//         }
//     }

//     pub fn on_event(mut self, f: OnEvent) -> Self {
//         self.on_event = Some(f);
//         self
//     }
// }
// impl<Ctx: AppCtx, S: Shard<Ctx> + ?Sized, OnEvent: Fn(MouseInteraction, &mut S, &mut Ctx)>
//     Shard<Ctx> for Interactable<S, OnEvent>
// {
//     #[inline(always)]
//     fn dirty(&self) -> bool {
//         self.shard.dirty()
//     }

//     #[inline(always)]
//     fn layout(&mut self, constraints: BoundingConstraints) -> ShardLayout {
//         self.shard.layout(constraints)
//     }

//     #[inline(always)]
//     fn on_message(
//         &mut self,
//         layout: &ShardLayout,
//         pos: Point,
//         state: &mut Ctx,
//         message: &Ctx::Message,
//     ) {
//         self.shard.on_message(layout, pos, state, message)
//     }

//     #[inline(always)]
//     fn render(
//         &mut self,
//         pos: Point,
//         layout: &ShardLayout,
//         canvas: &mut CanvasContext,
//     ) -> Option<(Point, BoundingRect)> {
//         self.shard.render(pos, layout, canvas)
//     }

//     #[inline(always)]
//     fn on_update(&mut self, context: &Ctx) {
//         self.shard.on_update(context)
//     }

//     fn on_event(
//         &mut self,
//         layout: &ShardLayout,
//         pos: Point,
//         state: &mut Ctx,
//         event: &libopal::WindowEvent,
//     ) {
//         let x = pos.x();
//         let y = pos.y();
//         let bounds = layout.bounds;

//         let max_x = x + bounds.width();
//         let max_y = y + bounds.height();

//         let mut on_event = |inter: MouseInteraction| {
//             if let Some(ref mut f) = self.on_event {
//                 (f)(inter, &mut self.shard, state)
//             }
//         };
//         match event {
//             WindowEvent::MouseChange(eve) => {
//                 let ev_x = eve.x() as f32;
//                 let ev_y = eve.y() as f32;
//                 let in_bounds = (ev_x >= x && ev_x <= max_x) && (ev_y >= y && ev_y <= max_y);
//                 if in_bounds {
//                     let was_in = self.mouse_in;
//                     if !self.mouse_in {
//                         self.mouse_in = true;
//                         on_event(MouseInteraction::MouseEnter);
//                     }

//                     if let Some(btns) = eve.buttons_change().or((!was_in)
//                         .then_some(eve.held_buttons())
//                         .filter(|btns| !btns.is_empty()))
//                     {
//                         on_event(MouseInteraction::Buttons(btns));
//                     }
//                 } else if self.mouse_in {
//                     self.mouse_in = false;
//                     on_event(MouseInteraction::MouseLeave);
//                 }
//             }
//             WindowEvent::MouseEnter(eve) => {
//                 let ev_x = eve.x() as f32;
//                 let ev_y = eve.y() as f32;
//                 let in_bounds = (ev_x >= x && ev_x <= max_x) && (ev_y >= y && ev_y <= max_y);

//                 if !self.mouse_in && in_bounds {
//                     self.mouse_in = true;
//                     on_event(MouseInteraction::MouseEnter);
//                 } else if self.mouse_in {
//                     self.mouse_in = false;
//                     on_event(MouseInteraction::MouseLeave);
//                 }
//             }
//             WindowEvent::MouseLeave(MouseLeaveEvent) => {
//                 if core::mem::replace(&mut self.mouse_in, false) {
//                     on_event(MouseInteraction::MouseLeave);
//                 }
//             }
//             _ => self.shard.on_event(layout, pos, state, event),
//         }
//     }
// }
