use crate::shards::ShardState;

/// Represents a LifeCycle event to the widgets internal state or any changes that isn't picked by [`super::ShardEvent`].
#[derive(Debug, Clone, Copy)]
pub enum LifeCycle {
    /// The initial LifeCycle when a Widget is added.
    Init,
    /// Generated If the hot status (hover status) of the widget changed.
    HotChanged(bool),
    /// Generated If the disabled status of the widget changed by an outsider.
    DisabledChanged(bool),
}

#[derive(Debug)]
pub struct LifeCycleCtx<'a> {
    state: &'a mut ShardState,
}

impl<'a> LifeCycleCtx<'a> {
    pub(crate) fn new(state: &'a mut ShardState) -> Self {
        Self { state }
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
}
