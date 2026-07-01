mod button;
pub(crate) mod event;
mod ext;
mod label;
mod layout;
pub mod lifecycle;
pub use event::EventCtx;
pub use lifecycle::LifeCycleCtx;
mod primitive;
mod render_ctx;
mod stack;

pub use button::*;
use cosmic_text::FontSystem;
pub use ext::*;
pub use label::*;
pub use layout::*;
pub use render_ctx::*;
pub use stack::*;

use crate::{
    Data, ShardEvent,
    render::{BoundingConstraints, BoundingRect, CanvasCache, CanvasContext, NoopCanvas, Point},
    shards::lifecycle::LifeCycle,
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
pub trait Shard<S = (), M = ()> {
    fn dirty(&self) -> bool;
    fn should_relayout(&self) -> bool {
        false
    }
    /// [`LifeCycle`] report.
    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &Data<S, M>) {
        _ = ctx;
        _ = event;
        _ = data;
    }
    /// Lays out [`Self`] according to ctxt, returning the layout.
    fn layout(&mut self, ctx: &mut LayoutCtx) -> ShardLayout;
    /// Renders self at the given `pos` with the given `layout` (results of calling [`Shard::layout`]) into the given [`Canvas`].
    ///
    /// Typically damages the whole width*height area within in that case return None,
    /// however if the damage is at a specific subpoint from the hitbox return Some((point from self, range bounds)).
    fn render(&mut self, ctx: &mut RenderCtx, data: &Data<S, M>) -> Option<(Point, BoundingRect)>;
    /// Executed on a new event, regardless if the event is relevant to the shard.
    ///
    /// Is followed by [`Shard::on_update`] lifecycle.
    fn on_event(&mut self, event_ctx: &mut EventCtx, event: &ShardEvent, data: &mut Data<S, M>) {
        _ = event_ctx;
        _ = event;
        _ = data;
    }
    /// Executed on a new message.
    ///
    /// Is followed by [`Shard::on_update`] lifecycle.
    fn on_message(
        &mut self,
        layout: &ShardLayout,
        pos: Point,
        context: &mut Data<S, M>,
        message: &M,
    ) {
        _ = pos;
        _ = context;
        _ = message;
        _ = layout;
    }
    /// Executed whenever `context` is potentially muttated.
    fn on_ctx_update(&mut self, context: &Data<S, M>) {
        _ = context;
    }
}

/// Does Absolutely Nothing
impl<S, M> Shard<S, M> for () {
    fn dirty(&self) -> bool {
        false
    }
    fn layout(&mut self, _: &mut LayoutCtx) -> ShardLayout {
        ShardLayout::default()
    }
    fn render(&mut self, ctx: &mut RenderCtx, data: &Data<S, M>) -> Option<(Point, BoundingRect)> {
        _ = ctx;
        _ = data;
        None
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
pub(crate) struct ShardNode<State, Message> {
    state: ShardState,
    /// The origin point of the shard.
    origin: Point,
    /// The last cached layout of the shard.
    layout: Option<ShardLayout>,
    pub(crate) shard: Box<CachedShard<dyn Shard<State, Message>>>,
}

impl<State, Message> ShardNode<State, Message> {
    pub fn layout_ref(&self) -> Option<&ShardLayout> {
        self.layout.as_ref()
    }

    /// Sets the origin point of the shard.
    pub fn plot_at(&mut self, origin: Point) {
        self.state.state_changed |= core::mem::replace(&mut self.origin, origin) != origin;
    }

    /// Prepares the layout of the shard.
    pub fn layout(&mut self, ctx: &mut LayoutCtx) -> (&mut ShardLayout, bool) {
        let laid = self.shard.layout(ctx);

        let is_new = self.layout != Some(laid);
        self.state.state_changed |= is_new;

        (self.layout.insert(laid), is_new)
    }

    /// Same as [`Self::layout`] but is only executed if no layout is already set.
    pub fn layout_if_none(&mut self, ctx: &mut LayoutCtx) -> &mut ShardLayout {
        if let Some(ref mut layout) = self.layout {
            layout
        } else {
            self.layout(ctx).0
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.shard.dirty()
    }

    pub fn should_relayout(&self) -> bool {
        self.shard.should_relayout()
    }

    pub fn position(&self) -> Point {
        self.origin
    }

    fn fix_hot<'a>(
        layout: &'a ShardLayout,
        state: &'a mut ShardState,
        shard: &mut CachedShard<dyn Shard<State, Message>>,
        cursor_at: Point,
        origin: Point,
        data: &mut Data<State, Message>,
    ) -> Option<EventCtx<'a>> {
        let is_hot = layout.bounds.contains_point(origin, cursor_at);

        let was_hot = core::mem::replace(&mut state.is_hot, is_hot);
        state.state_changed |= is_hot != was_hot;
        if is_hot != was_hot {
            shard.lifecycle(
                &mut LifeCycleCtx::new(state),
                &LifeCycle::HotChanged(is_hot),
                data,
            );
        }

        let mut ctx = EventCtx::new(origin, Some(cursor_at), state, layout);
        if is_hot && !was_hot {
            shard.on_event(&mut ctx, &crate::ShardEvent::MouseEnter, data);
        } else if was_hot {
            shard.on_event(&mut ctx, &crate::ShardEvent::MouseLeave, data);
        }

        is_hot.then_some(ctx)
    }

    pub fn route_lifecycle(&mut self, cycle: &LifeCycle, data: &Data<State, Message>) {
        self.shard
            .lifecycle(&mut LifeCycleCtx::new(&mut self.state), cycle, data);
    }

    /// Routes an event to the shard, updating state as necessary.
    pub fn route_event(
        &mut self,
        anchor_by: Point,
        event_origin: Option<Point>,
        event: &ShardEvent,
        data: &mut Data<State, Message>,
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
                        data,
                    );
                } else {
                    let is_within = layout.bounds.contains_point(origin, event_origin);
                    routed_event = is_within.then(|| {
                        EventCtx::new(origin, Some(event_origin), &mut self.state, layout)
                    });
                }
            } else {
                routed_event = Some(EventCtx::new(origin, None, &mut self.state, layout));
            }

            if let Some(mut routed_ctx) = routed_event {
                self.shard.on_event(&mut routed_ctx, event, data);
            }
        }
    }

    /// Routes a message to the shard.
    ///
    /// Requires [`Self::layout`] to be executed at least once.
    pub fn route_message(
        &mut self,
        anchor_by: Point,
        data: &mut Data<State, Message>,
        message: &Message,
    ) {
        let layout = self
            .layout
            .as_ref()
            .expect("Route message called on non-layedout shard");
        let origin = self.origin + anchor_by;

        self.shard.on_message(layout, origin, data, message);
    }

    pub fn on_ctx_update(&mut self, data: &Data<State, Message>) {
        self.shard.on_ctx_update(data);
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

    pub(crate) fn new<S: Shard<State, Message> + 'static>(shard: S) -> Self {
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
        data: &Data<State, Message>,
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

        let results = self.shard.render(&mut ctx, data);
        self.state.state_changed = false;
        results
    }

    #[inline(always)]
    pub fn render(
        &mut self,
        parent_ctx: &mut RenderCtx,
        layout_changed: bool,
        cursor_at: Option<Point>,
        data: &Data<State, Message>,
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
            ctx.with_state(&self.state, layout, |ctx| self.shard.render(ctx, data))
        });
        self.state.state_changed = false;
        r
    }
}
