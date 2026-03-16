use crate::render::Point;
use crate::shards::{ShardLayout, ShardState};
use libopal::defs::KeyModifiers;
use libopal::event::WindowEvent as OpalEvent;
use libopal::{defs::HeldMouseButtons, event::KeyCode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyEvent {
    code: KeyCode,
    modifiers: KeyModifiers,
}

impl KeyEvent {
    pub const fn code(&self) -> KeyCode {
        self.code
    }

    pub const fn modifiers(&self) -> KeyModifiers {
        self.modifiers
    }
}

/// Represents an event that occurred within a shard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShardEvent {
    MouseClick(HeldMouseButtons),
    MouseRelease(HeldMouseButtons),
    KeyPress(KeyEvent),
    KeyRelease(KeyEvent),
    MouseLeave,
    MouseEnter,
    MouseMove,
}

impl ShardEvent {
    pub const fn is_mouse_event(&self) -> bool {
        matches!(
            self,
            ShardEvent::MouseMove
                | ShardEvent::MouseLeave
                | ShardEvent::MouseEnter
                | Self::MouseRelease(_)
                | Self::MouseClick(_)
        )
    }
}

/// Represents an event context within a window.
#[derive(Debug, PartialEq)]
pub struct EventCtx<'a> {
    event_origin: Option<Point>,
    shard_state: &'a mut ShardState,
    shard_origin: Point,
    shard_layout: &'a ShardLayout,
}

impl<'a> EventCtx<'a> {
    pub(crate) const fn new(
        shard_origin: Point,
        origin: Option<Point>,
        shard_state: &'a mut ShardState,
        shard_layout: &'a ShardLayout,
    ) -> Self {
        Self {
            event_origin: origin,
            shard_state,
            shard_origin,
            shard_layout,
        }
    }

    /// Returns the origin point where the event occurred.
    pub const fn event_origin(&self) -> Option<Point> {
        self.event_origin
    }

    /// Returns the origin point of the shard.
    pub const fn shard_origin(&self) -> Point {
        self.shard_origin
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

    pub(crate) fn with_event(
        held_buttons: &mut HeldMouseButtons,
        mouse_origin: &mut Option<Point>,
        raw: &OpalEvent,
        mut f: impl FnMut(Option<Point>, ShardEvent),
    ) -> bool {
        let origin = None;
        // FIXME: A bit spaghetti
        let s_event = match raw {
            OpalEvent::WindowFocusChanged(f)
                if !f.is_focused() && mouse_origin.take().is_some() =>
            {
                Some(ShardEvent::MouseLeave)
            }
            OpalEvent::GlobalWindowAttached(_)
            | OpalEvent::GlobalWindowDeatached(_)
            | OpalEvent::GlobalWindowFocusChanged(_, _)
            | OpalEvent::WindowFocusChanged(_) => None,
            OpalEvent::MouseLeave(libopal::event::MouseLeaveEvent) => {
                *mouse_origin = None;
                Some(ShardEvent::MouseLeave)
            }
            OpalEvent::MouseEnter(eve) => {
                let origin = Point::new(eve.x() as f32, eve.y() as f32);
                *mouse_origin = Some(origin);
                Some(ShardEvent::MouseEnter)
            }
            OpalEvent::MouseChange(eve) => {
                let origin = Point::new(eve.x() as f32, eve.y() as f32);
                if core::mem::replace(mouse_origin, Some(origin)) != Some(origin) {
                    f(Some(origin), ShardEvent::MouseMove);
                }

                if let Some(change) = eve.buttons_change() {
                    let old = core::mem::replace(held_buttons, change);
                    let added_buttons = change.difference(old);
                    let removed_buttons = old.difference(change);
                    if !added_buttons.is_empty() {
                        f(Some(origin), ShardEvent::MouseClick(added_buttons));
                    }
                    if !removed_buttons.is_empty() {
                        f(Some(origin), ShardEvent::MouseRelease(removed_buttons));
                    }
                }

                return true;
            }
            OpalEvent::Key(key) => {
                let event = KeyEvent {
                    code: key.code,
                    modifiers: key.modifiers,
                };

                match key.kind {
                    libopal::event::KeyEventKind::Press => Some(ShardEvent::KeyPress(event)),
                    libopal::event::KeyEventKind::Release => Some(ShardEvent::KeyRelease(event)),
                    _ => None,
                }
            }
        };

        if let Some(s_event) = s_event {
            f(origin, s_event);
            true
        } else {
            false
        }
    }
}
