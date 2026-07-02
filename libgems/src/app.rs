use std::{
    ops::{Deref, DerefMut},
    sync::LazyLock,
};

use libopal::{DequeuedEvents, defs::WindowID};

use crate::{AppEnv, Window, WindowDesc, shards::Shard};

/// Describes a global App context
pub trait AppCtx: 'static {
    type State;
    type Message;

    fn send_message(&mut self, msg: Self::Message);
    fn state_mut(&mut self) -> &mut Self::State;
    fn env(&self) -> &AppEnv;
}

/// Represents an [`AppCtx`] core wrapper around [`AppState`].
pub struct Data<S = (), M = ()> {
    pending_messages: Vec<M>,
    env: AppEnv,
    state: S,
}

impl<State, Message> Data<State, Message> {
    pub fn broadcast_message(&mut self, msg: Message) {
        self.pending_messages.push(msg);
    }

    pub fn env(&self) -> &AppEnv {
        &self.env
    }
}

static DEFAULT_ENV: LazyLock<AppEnv> = LazyLock::new(|| AppEnv::default());

impl AppCtx for () {
    type State = ();
    type Message = ();
    fn env(&self) -> &AppEnv {
        &*DEFAULT_ENV
    }
    fn send_message(&mut self, msg: Self::Message) {
        _ = msg;
    }
    fn state_mut(&mut self) -> &mut Self::State {
        self
    }
}
impl<State: 'static, Message: 'static> AppCtx for Data<State, Message> {
    type State = State;
    type Message = Message;
    fn send_message(&mut self, msg: Message) {
        self.pending_messages.push(msg);
    }
    fn env(&self) -> &AppEnv {
        &self.env
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
    windows: Vec<Window<State, Message>>,
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

    pub fn env(&self) -> &AppEnv {
        self.core.env()
    }

    pub fn with_env(mut self, env: AppEnv) -> Self {
        self.core.env = env;
        self
    }

    pub fn get_window(&mut self, win: WindowID) -> Option<&mut Window<State, Message>> {
        self.windows.iter_mut().find(|w| w.win_id() == win)
    }

    pub fn remove_window(&mut self, win: WindowID) -> Option<Window<State, Message>> {
        let index = self.windows.iter().position(|w| w.win_id() == win)?;
        Some(self.windows.remove(index))
    }

    /// Inserts a window [`win`] to the given app.
    pub fn window<R: Shard<State, Message>>(mut self, win: WindowDesc<R>) -> Self {
        self.windows.push(win.init(&mut self.core));
        self
    }

    pub fn add_window<R: Shard<State, Message>>(&mut self, win: WindowDesc<R>) -> WindowID {
        let win = win.init(&mut self.core);
        let win_id = win.win_id();
        self.windows.push(win);
        win_id
    }

    /// Returns whether or not any of the App Windows needs to be redrawn.
    pub fn needs_redraw(&self) -> bool {
        self.windows.iter().any(|w| w.dirty())
    }

    /// Redraws all windows that needs to be redrawn.
    pub fn redraw_needed(&mut self) {
        for win in &mut self.windows {
            if win.dirty() {
                win.redraw(&self.core);
            }
        }
    }

    /// Redraws all windows.
    pub fn redraw_all(&mut self) {
        for win in &mut self.windows {
            win.redraw(&self.core);
        }
    }

    /// Blockingly wait for events and handles them.
    pub fn wait_for_events(&mut self) -> DequeuedEvents {
        self.try_wait_for_events(true).unwrap()
    }

    pub fn broadcast_message(&mut self, msg: Message) {
        self.core.broadcast_message(msg);
        self.handle_messages();
    }

    fn handle_messages(&mut self) {
        while !self.core.pending_messages.is_empty() {
            while let Some(msg) = self.core.pending_messages.pop() {
                for win in &mut self.windows {
                    win.broadcast_message(&mut self.core, &msg);
                }
            }

            if self.core.pending_messages.is_empty() {
                for win in &mut self.windows {
                    win.update_ctx(&self.core);
                }
            }
        }
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
                    self.handle_messages();
                    break;
                }
            }
        }

        Some(events)
    }
}
