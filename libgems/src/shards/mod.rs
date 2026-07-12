mod button;
mod image;
pub use image::*;

pub(crate) mod event;
mod ext;
mod label;
mod layout;
pub mod lifecycle;
pub use event::EventCtx;
pub use lifecycle::LifeCycleCtx;
mod message_ctx;
mod primitive;
mod render_ctx;
mod update_ctx;
pub use message_ctx::*;
pub use update_ctx::*;
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
    render::{BoundingConstraints, BoundingRect, CanvasCache, CanvasContext, PaintBrush, Point},
    shards::lifecycle::LifeCycle,
};

// TODO: General Ctx impl
#[derive(Debug)]
/// Describes a layout request context.
pub struct LayoutCtx<'f> {
    pub(crate) font_system: &'f mut FontSystem,
    pub(crate) constraints: BoundingConstraints,
    pub(crate) damage: &'f mut DamageArea,
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

    pub fn request_redraw(&mut self, layout: &ShardLayout) {
        self.damage
            .request_redraw_at(Point::default(), layout.bounds);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct DamageArea {
    damaged: Option<(Point, BoundingRect)>,
}

impl DamageArea {
    pub const fn damage(&self) -> Option<(Point, BoundingRect)> {
        self.damaged
    }

    pub const fn new(damaged: Option<(Point, BoundingRect)>) -> Self {
        Self { damaged }
    }

    pub fn collect_damage(&mut self, anchor: Point, from: &DamageArea) {
        if let Some((point, area)) = from.damaged {
            self.request_redraw_at(point + anchor, area);
        }
    }

    pub fn request_redraw_at(&mut self, at: Point, area: BoundingRect) {
        let render_pos = at;
        let render_rect = area;

        if let Some((ref mut d_pos, ref mut d_rect)) = self.damaged
            && d_rect.width() != 0.
            && d_rect.height() != 0.
        {
            let d_last_x = d_pos.x() + d_rect.width();
            let d_last_y = d_pos.y() + d_rect.height();

            let x = d_pos.x().min(render_pos.x());
            let y = d_pos.y().min(render_pos.y());
            let w = d_last_x.max(render_pos.x() + render_rect.width()) - x;
            let h = d_last_y.max(render_pos.y() + render_rect.height()) - y;

            *d_pos = Point::new(x, y);
            *d_rect = BoundingRect::new(w, h);
        } else {
            self.damaged = Some((render_pos, render_rect));
        }
    }

    pub fn intersection_with(
        &self,
        point: Point,
        area: BoundingRect,
    ) -> Option<(Point, BoundingRect)> {
        let Some((dp, dr)) = self.damaged else {
            return Some((point, area));
        };

        let d0 = dp;
        let d1 = dp + *dr;

        let r0 = point;
        let r1 = point + *area;

        if (d0.x() < r1.x() && d1.x() > r0.x()) && (d0.y() < r1.y() && d1.y() > r0.y()) {
            let i_x0 = d0.x().max(r0.x()) - r0.x();
            let i_y0 = d0.y().max(r0.y()) - r0.y();

            let i_x1 = d1.x().min(r1.x()) - r0.x();
            let i_y1 = d1.y().min(r1.y()) - r0.y();

            Some((
                Point::new(i_x0, i_y0),
                BoundingRect::new(i_x1 - i_x0, i_y1 - i_y0),
            ))
        } else {
            None
        }
    }
}

/// A shard is a widget.
pub trait Shard<S = (), M = ()> {
    /// Dirty flag specifies whether or not a widget needs to be [`Self::render`]ed as duo to an external widget data change.
    ///
    /// This is used by ext widgets to figure out if widget data changed and `ctx.request_render` you still need to manually request a re-render.
    ///
    /// Has to return false after the redraw is done.
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

    fn with_children(&mut self, f: &mut dyn FnMut(&mut dyn Iterator<Item = &mut ShardNode<S, M>>)) {
        f(&mut core::iter::empty())
    }
    /// Lays out [`Self`] according to ctxt, returning the layout.
    fn layout(&mut self, ctx: &mut LayoutCtx) -> ShardLayout;
    /// Renders self at the with a `ctx` containing `layout` (results of calling [`Shard::layout`]), position, state, helpers for rendering and more.
    ///
    /// see also [`Self::dirty`].
    /// You have to request a redraw using `ctx.request_redraw` (any method taking ctx and mut self) for this to be called.
    fn render(&mut self, ctx: &mut RenderCtx, data: &Data<S, M>);
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
    fn on_message(&mut self, ctx: &mut MsgCtx, data: &mut Data<S, M>, message: &M) {
        _ = ctx;
        _ = data;
        _ = message;
    }
    /// Executed whenever `context` is potentially muttated.
    fn on_ctx_update(&mut self, update_ctx: &mut UpdateCtx, context: &Data<S, M>) {
        _ = update_ctx;
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
    fn render(&mut self, ctx: &mut RenderCtx, data: &Data<S, M>) {
        _ = ctx;
        _ = data;
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
pub struct ShardNode<State, Message> {
    state: ShardState,
    /// The origin point of the shard.
    origin: Option<Point>,
    /// The last cached layout of the shard.
    layout: Option<ShardLayout>,
    old_origin: Option<Point>,
    old_layout: Option<ShardLayout>,
    damage: DamageArea,
    pub(crate) shard: Box<dyn Shard<State, Message>>,
}

impl<State, Message> ShardNode<State, Message> {
    pub fn layout_ref(&self) -> Option<&ShardLayout> {
        self.layout.as_ref()
    }

    /// Sets the origin point of the shard.
    pub fn plot_at(&mut self, origin: Point) {
        let old = core::mem::replace(&mut self.origin, Some(origin));
        let is_new = old != Some(origin);

        self.old_origin = old;
        self.state.state_changed |= is_new;
    }

    /// Prepares the layout of the shard.
    pub fn layout(&mut self, ctx: &mut LayoutCtx) -> (&mut ShardLayout, bool) {
        let mut ctx = LayoutCtx {
            damage: &mut self.damage,
            constraints: ctx.constraints,
            font_system: ctx.font_system,
        };
        let laid = self.shard.layout(&mut ctx);

        let is_new = self.layout != Some(laid);
        self.state.state_changed |= is_new;

        self.old_layout = self.layout;
        (self.layout.insert(laid), is_new)
    }

    /// Same as [`Self::layout`] but is only executed if no layout is already set.
    pub fn layout_if_none(&mut self, ctx: &mut LayoutCtx) -> (&mut ShardLayout, bool) {
        if let Some(ref mut layout) = self.layout {
            (layout, true)
        } else {
            self.layout(ctx)
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.shard.dirty()
    }

    pub fn should_relayout(&self) -> bool {
        self.shard.should_relayout()
    }

    fn fix_hot<'a>(
        layout: &'a ShardLayout,
        state: &'a mut ShardState,
        shard: &mut dyn Shard<State, Message>,
        cursor_at: Point,
        origin: Point,
        data: &mut Data<State, Message>,
        damage: &'a mut DamageArea,
    ) -> Option<EventCtx<'a>> {
        let is_hot = layout.bounds.contains_point(origin, cursor_at);

        let was_hot = core::mem::replace(&mut state.is_hot, is_hot);
        state.state_changed |= is_hot != was_hot;
        if is_hot != was_hot {
            shard.lifecycle(
                &mut LifeCycleCtx::new(state, Some((origin, layout)), damage),
                &LifeCycle::HotChanged(is_hot),
                data,
            );
        }

        let mut ctx = EventCtx::new(origin, Some(cursor_at), state, layout, damage);
        if is_hot && !was_hot {
            shard.on_event(&mut ctx, &crate::ShardEvent::MouseEnter, data);
        } else if was_hot {
            shard.on_event(&mut ctx, &crate::ShardEvent::MouseLeave, data);
        }

        is_hot.then_some(ctx)
    }

    pub fn route_lifecycle(
        &mut self,
        cycle: &LifeCycle,
        data: &Data<State, Message>,
        damage: &mut DamageArea,
    ) {
        self.shard.lifecycle(
            &mut LifeCycleCtx::new(
                &mut self.state,
                self.layout.as_ref().and_then(|l| Some((self.origin?, l))),
                damage,
            ),
            cycle,
            data,
        );
    }

    /// Routes an event to the shard, updating state as necessary.
    pub fn route_event<'a>(
        &'a mut self,
        anchor_by: Point,
        event_origin: Option<Point>,
        event: &ShardEvent,
        data: &mut Data<State, Message>,
        damage: &'a mut DamageArea,
    ) -> Option<EventCtx<'a>> {
        if let Some(ref layout) = self.layout {
            let is_mouse_event = event.is_mouse_event();
            let origin = self
                .origin
                .expect("Route event to a shard without position.")
                + anchor_by;

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
                        damage,
                    );
                } else {
                    let is_within = layout.bounds.contains_point(origin, event_origin);
                    routed_event = is_within.then(|| {
                        EventCtx::new(origin, Some(event_origin), &mut self.state, layout, damage)
                    });
                }
            } else {
                routed_event = Some(EventCtx::new(origin, None, &mut self.state, layout, damage));
            }

            if let Some(mut routed_ctx) = routed_event {
                self.shard.on_event(&mut routed_ctx, event, data);
                Some(routed_ctx)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Routes a message to the shard.
    ///
    /// Requires [`Self::layout`] to be executed at least once.
    pub fn route_message<'a>(
        &'a mut self,
        anchor_by: Point,
        data: &mut Data<State, Message>,
        message: &Message,
        damage: &'a mut DamageArea,
    ) -> Option<MsgCtx<'a>> {
        let layout = self.layout.as_ref()?;
        let origin = self.origin? + anchor_by;

        let mut ctx = MsgCtx::new(origin, &mut self.state, layout, damage);
        self.shard.on_message(&mut ctx, data, message);
        Some(ctx)
    }

    pub fn on_ctx_update(
        &mut self,
        anchor_by: Point,
        damage: &mut DamageArea,
        data: &Data<State, Message>,
    ) {
        if let Some(ref layout) = self.layout
            && let Some(origin) = self.origin
        {
            let mut ctx = UpdateCtx::new(anchor_by + origin, &mut self.state, layout, damage);
            self.shard.on_ctx_update(&mut ctx, data);
        }
    }

    pub fn on_relayout(&mut self, parent_is_hot: bool, anchor_by: Point, cursor_at: Option<Point>) {
        let origin = self
            .origin
            .expect("Attempt to render a shard without a position.")
            + anchor_by;
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
            shard: Box::new(shard),
            origin: None,
            layout: None,
            old_layout: None,
            old_origin: None,
            state: ShardState::default(),
            damage: DamageArea::new(None),
        }
    }

    #[inline(always)]
    pub(crate) fn render_as_root(
        &mut self,
        cache: &mut CanvasCache,
        mut pixmap: tiny_skia::PixmapMut,
        damage: &mut DamageArea,
        data: &Data<State, Message>,
    ) {
        if let Some((p, r)) = damage.damaged {
            pixmap.fill_rect(
                tiny_skia::Rect::from_xywh(p.x(), p.y(), r.width(), r.height())
                    .expect("Damage area bad"),
                PaintBrush::from(crate::Color::NONE)
                    .with_blend(tiny_skia::BlendMode::Source)
                    .no_aa()
                    .as_paint(),
                tiny_skia::Transform::default(),
                None,
            );
        }
        let mut canvas_ctx = CanvasContext::new(cache, pixmap);
        let mut ctx = RenderCtx::new(
            Point::new(0., 0.),
            &mut canvas_ctx,
            &self.state,
            self.layout
                .as_ref()
                .expect("Attempt to render node with no layout"),
            damage,
        );
        let results = self.shard.render(&mut ctx, data);
        self.state.state_changed = false;
        results
    }

    pub fn collect_damage(&mut self, anchor_by: Point, damage: &mut DamageArea) {
        let origin = self
            .origin
            .expect("Attempt to collect damage but node had no position.")
            + anchor_by;
        let layout = self
            .layout
            .as_ref()
            .expect("Attempt to collect damage but node had no layout.");

        match (self.old_origin.take(), self.old_layout.take()) {
            (None, None) => {}
            (Some(old_origin), Some(old_layout)) => {
                if &old_layout != layout {
                    damage.request_redraw_at(old_origin + anchor_by, old_layout.bounds);
                }
            }
            (None, Some(old_layout)) => {
                if &old_layout != layout {
                    damage.request_redraw_at(origin, old_layout.bounds);
                }
            }

            (Some(old_origin), None) => {
                let old_origin = old_origin + anchor_by;
                if old_origin != origin {
                    damage.request_redraw_at(old_origin, layout.bounds);
                }
            }
        }
        damage.collect_damage(origin, &self.damage);
        self.damage = DamageArea::new(None);

        self.shard.with_children(&mut |children| {
            for child in children {
                child.collect_damage(origin, damage);
            }
        });
    }
    #[inline(always)]
    pub fn render(
        &mut self,
        parent_ctx: &mut RenderCtx,
        layout_changed: bool,
        cursor_at: Option<Point>,
        data: &Data<State, Message>,
    ) {
        let parent_origin = parent_ctx.origin();
        if layout_changed {
            self.on_relayout(parent_ctx.is_hot(), parent_origin, cursor_at);
        }

        let layout = self
            .layout
            .as_ref()
            .expect("Attempt to render node with no layout");
        let origin = self
            .origin
            .expect("Attempt to render node with no position.");
        let bounds = layout.bounds;

        let r = parent_ctx.nest_ctx(origin, bounds, |ctx| {
            ctx.with_state(&self.state, layout, |ctx| self.shard.render(ctx, data))
        });
        self.state.state_changed = false;
        r
    }
}
