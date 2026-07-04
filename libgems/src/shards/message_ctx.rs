use crate::{
    BoundingRect, Point,
    shards::{DamageArea, ShardLayout, ShardState},
};

/// Represents an event context within a window.
#[derive(Debug, PartialEq)]
pub struct MsgCtx<'a> {
    origin: Point,
    shard_state: &'a mut ShardState,
    shard_layout: &'a ShardLayout,
    requested_removal: bool,
    damage: &'a mut DamageArea,
}

impl<'a> MsgCtx<'a> {
    pub(crate) const fn new(
        origin: Point,
        shard_state: &'a mut ShardState,
        shard_layout: &'a ShardLayout,
        damage: &'a mut DamageArea,
    ) -> Self {
        Self {
            origin,
            shard_state,
            shard_layout,
            requested_removal: false,
            damage,
        }
    }

    /// Sets the active state of the shard.
    /// (eg. button is pressed).
    pub fn set_active(&mut self, active: bool) {
        self.shard_state.set_active(active);
    }

    /// Sets the disabled state of the shard.
    /// (eg. button is disabled).
    pub fn set_disabled(&mut self, disabled: bool) {
        self.shard_state.set_disabled(disabled);
    }

    /// Returns whether the shard is currently hot. (eg. mouse is over the shard).
    pub fn is_hot(&self) -> bool {
        self.shard_state.is_hot()
    }

    pub fn request_remove(&mut self) {
        self.requested_removal = true;
    }

    pub fn request_redraw(&mut self) {
        self.request_redraw_at(Point::new(0., 0.), self.layout().full_bounds());
    }

    pub fn damage_area(&mut self) -> &mut DamageArea {
        self.damage
    }

    pub fn request_redraw_at(&mut self, at: Point, area: BoundingRect) {
        self.damage.request_redraw_at(at + self.origin(), area);
    }

    pub fn requested_remove(&self) -> bool {
        self.requested_removal
    }

    /// Returns whether the shard is currently active. (eg. button is pressed).
    pub fn is_active(&self) -> bool {
        self.shard_state.is_active()
    }

    /// Returns whether the shard is currently disabled. (eg. button is disabled).
    pub fn is_disabled(&self) -> bool {
        self.shard_state.is_disabled()
    }

    /// Returns the layout of the shard.
    pub fn layout(&self) -> &ShardLayout {
        self.shard_layout
    }

    pub fn origin(&self) -> Point {
        self.origin
    }
}
