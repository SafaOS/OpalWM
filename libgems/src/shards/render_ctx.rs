use cosmic_text::FontSystem;

use crate::{
    BoundingRect, Point,
    render::{CanvasContext, PaintBrush, Shape},
    shards::{DamageArea, ShardLayout, ShardState},
};

/// Render context for a rendering operation.
///
/// TODO: Make this more inspired by druid's PaintCtx and add environments and themes,
/// for now I'm getting things started so there is a lot of inconsistency and spaghetti everywhere.
pub struct RenderCtx<'s, 'c> {
    origin: Point,
    canvas: &'s mut CanvasContext<'c>,
    pub(crate) state: &'s ShardState,
    layout: &'s ShardLayout,
    damage: &'s mut DamageArea,
}

impl<'s, 'c> RenderCtx<'s, 'c> {
    #[inline(always)]
    pub(crate) fn new(
        origin: Point,
        canvas: &'c mut CanvasContext<'c>,
        state: &'s ShardState,
        layout: &'s ShardLayout,
        damage: &'s mut DamageArea,
    ) -> Self {
        Self {
            origin,
            canvas,
            state,
            layout,
            damage,
        }
    }
    #[inline]
    pub fn move_to_abs(&mut self, origin: Point) -> &mut Self {
        self.origin = origin;
        self
    }

    #[inline]
    pub fn move_to(&mut self, relative: Point) -> &mut Self {
        self.move_to_abs(self.origin + relative)
    }

    #[inline]
    pub fn with_state<R, F: FnOnce(&mut RenderCtx) -> R>(
        &mut self,
        state: &ShardState,
        layout: &ShardLayout,
        f: F,
    ) -> R {
        let mut ctx = RenderCtx {
            layout: layout,
            origin: self.origin,
            canvas: self.canvas,
            state: state,
            damage: self.damage,
        };

        f(&mut ctx)
    }

    #[inline]
    pub fn with_pixmap<R, F: FnOnce(&mut RenderCtx) -> R>(
        &mut self,
        pixmap: tiny_skia::PixmapMut,
        f: F,
    ) -> R {
        let mut ctx = CanvasContext::new(self.canvas.cache, pixmap);
        let mut ctx = RenderCtx {
            origin: self.origin,
            canvas: &mut ctx,
            state: self.state,
            layout: self.layout,
            damage: self.damage,
        };
        f(&mut ctx)
    }

    #[inline]
    pub fn nest_ctx<R, F: FnOnce(&mut RenderCtx) -> R>(
        &mut self,
        offset: Point,
        bounds: BoundingRect,
        f: F,
    ) -> R {
        // TODO: bounds?
        _ = bounds;
        let mut ctx = RenderCtx {
            origin: self.origin + offset,
            canvas: self.canvas,
            state: self.state,
            layout: self.layout,
            damage: self.damage,
        };
        f(&mut ctx)
    }

    #[inline]
    /// Fills the given [`Shape`], at the current position.
    pub fn fill<S: Shape>(&mut self, brush: &PaintBrush, shape: &S) {
        self.canvas.fill(self.origin, brush, shape);
    }

    #[inline]
    /// Fills and renders text from the given cosmic text buffer with the given brush, at the current location.
    pub fn fill_text(&mut self, brush: &PaintBrush, text: &cosmic_text::Buffer) {
        self.canvas.draw_text(self.origin, brush, text);
    }

    #[inline]
    pub fn fill_with_pixmap(&mut self, pixmap: tiny_skia::PixmapRef) {
        self.canvas.fill_with_pixmap(self.origin, pixmap);
    }

    /// Fills the given area with background (brush paint) as by clearing it.
    #[inline]
    pub fn clear(&mut self, brush: PaintBrush, area: BoundingRect) {
        self.canvas.clear(brush, self.origin, area);
    }

    #[inline]
    /// Strokes the given [`Shape`], at the current position.
    pub fn stroke<S: Shape>(&mut self, brush: &PaintBrush, stroke_width: f32, shape: &S) {
        self.canvas.stroke(self.origin, brush, stroke_width, shape);
    }

    #[inline]
    pub fn canvas(&mut self) -> &mut CanvasContext<'c> {
        self.canvas
    }

    #[inline]
    pub fn pixmap(&mut self) -> &mut tiny_skia::PixmapMut<'c> {
        &mut self.canvas.pixmap
    }

    #[inline]
    pub fn path(&mut self) -> &mut tiny_skia::PathBuilder {
        self.canvas.cache.path()
    }

    #[inline]
    pub fn font_system(&mut self) -> &mut FontSystem {
        self.canvas.cache.font_system()
    }

    #[inline(always)]
    pub fn origin(&self) -> Point {
        self.origin
    }

    #[inline(always)]
    pub fn layout(&self) -> &ShardLayout {
        self.layout
    }

    #[inline(always)]
    pub fn damage(&mut self) -> &mut DamageArea {
        self.damage
    }

    #[inline(always)]
    pub fn state_changed(&self) -> bool {
        self.state.state_changed
    }

    /// See [`ShardState::is_active`].
    #[inline(always)]
    pub fn is_active(&self) -> bool {
        self.state.is_active()
    }

    /// See [`ShardState::is_hot`].
    #[inline(always)]
    pub fn is_hot(&self) -> bool {
        self.state.is_hot()
    }

    /// See [`ShardState::is_disabled`].
    #[inline(always)]
    pub fn is_disabled(&self) -> bool {
        self.state.is_disabled()
    }
}
