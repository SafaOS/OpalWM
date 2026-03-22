use std::ops::{Deref, DerefMut};

use libopal::DequeuedEvents;

use crate::{AppEnv, Window};

pub trait AppState: 'static {
    type Message;
}

/// Describes a global App context
pub trait AppCtx: 'static {
    type State;
    type Message;

    fn send_message(&mut self, msg: Self::Message);
    fn state_mut(&mut self) -> &mut Self::State;
    fn env(&self) -> &AppEnv;
}

/// Represents an [`AppCtx`] core wrapper around [`AppState`].
pub struct Data<State: AppState> {
    pending_messages: Vec<State::Message>,
    env: AppEnv,
    state: State,
}

impl<State: AppState> Data<State> {
    pub fn broadcast_message(&mut self, msg: State::Message) {
        self.send_message(msg);
    }
}

impl<State: AppState> AppCtx for Data<State> {
    type State = State;
    type Message = State::Message;
    fn send_message(&mut self, msg: Self::Message) {
        self.pending_messages.push(msg);
    }
    fn env(&self) -> &AppEnv {
        &self.env
    }
    fn state_mut(&mut self) -> &mut State {
        &mut self.state
    }
}

impl<S: AppState> Deref for Data<S> {
    type Target = S;
    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl<S: AppState> DerefMut for Data<S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.state
    }
}

/// Describes an app made of multiple [`Window`]s and a state.
pub struct App<State: AppState> {
    core: Data<State>,
    windows: Vec<Window<Data<State>>>,
}

impl<State: AppState> App<State> {
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

    /// Inserts a window [`win`] to the given app.
    pub fn window(mut self, win: Window<Data<State>>) -> Self {
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
        let events = libopal::dequeue_events_blocking().expect("Failed to dequeue events");
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

        events
    }
}
