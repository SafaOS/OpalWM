use crate::{
    BoundingRect, Point,
    shards::{DamageArea, ShardLayout, ShardState},
};

#[derive(Debug)]
pub struct UpdateCtx<'a> {
    state: &'a mut ShardState,
    origin: Point,
    layout: &'a ShardLayout,
    damage: &'a mut DamageArea,
}

impl<'a> UpdateCtx<'a> {
    pub(crate) fn new(
        origin: Point,
        state: &'a mut ShardState,
        layout: &'a ShardLayout,
        damage: &'a mut DamageArea,
    ) -> Self {
        Self {
            origin,
            state,
            layout,
            damage,
        }
    }

    pub fn origin(&self) -> Point {
        self.origin
    }
    /// Sets the active state of the shard.
    /// (eg. button is pressed).
    pub fn set_active(&mut self, active: bool) {
        self.state.set_active(active);
    }

    /// Sets the disabled state of the shard.
    /// (eg. button is disabled).
    pub fn set_disabled(&mut self, disabled: bool) {
        self.state.set_disabled(disabled);
    }

    /// Returns whether the shard is currently hot. (eg. mouse is over the shard).
    pub fn is_hot(&self) -> bool {
        self.state.is_hot()
    }

    /// Returns whether the shard is currently active. (eg. button is pressed).
    pub fn is_active(&self) -> bool {
        self.state.is_active()
    }

    /// Returns whether the shard is currently disabled. (eg. button is disabled).
    pub fn is_disabled(&self) -> bool {
        self.state.is_disabled()
    }

    pub fn request_redraw(&mut self) {
        self.request_redraw_at(Point::new(0., 0.), self.layout.full_bounds());
    }

    pub fn damage_area(&mut self) -> &mut DamageArea {
        self.damage
    }

    pub fn request_redraw_at(&mut self, at: Point, area: BoundingRect) {
        self.damage.request_redraw_at(at + self.origin, area);
    }
}
