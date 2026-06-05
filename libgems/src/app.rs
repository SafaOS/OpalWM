use std::ops::{Deref, DerefMut};

use libopal::{DequeuedEvents, defs::WindowID};

use crate::{AppEnv, Window};

/// Describes a global App context
pub trait AppCtx: 'static {
    type State;
    type Message;

    fn send_message(&mut self, msg: Self::Message);
    fn state_mut(&mut self) -> &mut Self::State;
    fn env(&self) -> Option<&AppEnv> {
        None
    }
}

/// Represents an [`AppCtx`] core wrapper around [`AppState`].
pub struct Data<State: 'static = (), Message: 'static = ()> {
    pending_messages: Vec<Message>,
    env: AppEnv,
    state: State,
}

impl<State, Message> Data<State, Message> {
    pub fn broadcast_message(&mut self, msg: Message) {
        self.send_message(msg);
    }
}

impl AppCtx for () {
    type State = ();
    type Message = ();
    fn env(&self) -> Option<&AppEnv> {
        None
    }
    fn send_message(&mut self, msg: Self::Message) {
        _ = msg;
    }
    fn state_mut(&mut self) -> &mut Self::State {
        self
    }
}
impl<State, Message> AppCtx for Data<State, Message> {
    type State = State;
    type Message = Message;
    fn send_message(&mut self, msg: Message) {
        self.pending_messages.push(msg);
    }
    fn env(&self) -> Option<&AppEnv> {
        Some(&self.env)
    }
    fn state_mut(&mut self) -> &mut State {
        &mut self.state
    }
}

impl<S, O> Deref for Data<S, O> {
    type Target = S;
    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl<S, O> DerefMut for Data<S, O> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.state
    }
}

/// Describes an app made of multiple [`Window`]s and a state.
pub struct App<State: 'static = (), Message: 'static = ()> {
    core: Data<State, Message>,
    windows: Vec<Window<Data<State, Message>>>,
}

impl<State, Message> App<State, Message> {
    pub fn new(state: State) -> Self {
        Self {
            core: Data {
                pending_messages: Vec::new(),
                env: AppEnv::default(),
                state,
            },
            windows: Vec::new(),
        }
    }

    pub fn get_window(&mut self, win: WindowID) -> Option<&mut Window<Data<State, Message>>> {
        self.windows.iter_mut().find(|w| w.win_id() == win)
    }

    pub fn remove_window(&mut self, win: WindowID) -> Option<Window<Data<State, Message>>> {
        let index = self.windows.iter().position(|w| w.win_id() == win)?;
        Some(self.windows.remove(index))
    }

    /// Inserts a window [`win`] to the given app.
    pub fn window(mut self, win: Window<Data<State, Message>>) -> Self {
        self.windows.push(win);
        self
    }

    pub fn add_window(&mut self, win: Window<Data<State, Message>>) -> &mut Self {
        self.windows.push(win);
        self
    }

    /// Returns whether or not any of the App Windows needs to be redrawn.
    pub fn needs_redraw(&self) -> bool {
        self.windows.iter().any(|w| w.dirty())
    }

    /// Redraws all windows that needs to be redrawn.
    pub fn redraw_needed(&mut self) {
        for win in &mut self.windows {
            if win.dirty() {
                win.redraw();
            }
        }
    }

    /// Redraws all windows.
    pub fn redraw_all(&mut self) {
        for win in &mut self.windows {
            win.redraw();
        }
    }

    /// Blockingly wait for events and handles them.
    pub fn wait_for_events(&mut self) -> DequeuedEvents {
        self.try_wait_for_events(true).unwrap()
    }

    pub fn try_wait_for_events(&mut self, blocking: bool) -> Option<DequeuedEvents> {
        let events = if blocking {
            libopal::dequeue_events_blocking().expect("Failed to dequeue events")
        } else {
            libopal::dequeue_events_non_blocking().expect("Failed to dequeue events")?
        };

        for event in &*events {
            for win in &mut self.windows {
                if win.win_id() == event.receiver() {
                    win.broadcast_event(&mut self.core, event.event());
                    while !self.core.pending_messages.is_empty() {
                        while let Some(msg) = self.core.pending_messages.pop() {
                            win.broadcast_message(&mut self.core, &msg);
                        }

                        if self.core.pending_messages.is_empty() {
                            win.update_ctx(&self.core);
                        }
                    }
                    break;
                }
            }
        }

        Some(events)
    }
}
