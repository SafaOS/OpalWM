//! Keyboard handling
//!
//! This module provides functionality for handling keyboard input.
//! It includes functions for reading key events from the keyboard device,
//! translating key codes to characters, and dispatching key events to the
//! active window.

use std::{
    fs::File,
    io::{self, BufReader, Read},
};

use libopal::event::KeyModifiers;
use safa_api::abi::input::{KeyCode, KeyEvent, KeyEventKind};

use crate::window::WINDOWS;

const MAX_KEYCODES: usize = KeyCode::LastKey as usize;
type KeysBitmap = u64;
const KEYS_BITMAP_BITS: usize = KeysBitmap::BITS as usize;
const BITMAP_SIZE: usize = MAX_KEYCODES.div_ceil(KEYS_BITMAP_BITS);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
/// A set of currently pressed keys.
pub struct PressedKeys {
    raw: [KeysBitmap; BITMAP_SIZE],
}

impl PressedKeys {
    /// Creates a new empty set of pressed keys.
    pub const fn default() -> Self {
        Self {
            raw: [0; BITMAP_SIZE],
        }
    }

    /// Inserts a key code into the set.
    pub const fn add(&mut self, key: KeyCode) {
        let index = key as usize / KEYS_BITMAP_BITS;
        let offset = key as usize % KEYS_BITMAP_BITS;
        self.raw[index] |= 1 << offset;
    }

    /// Removes a key code from the set.
    pub const fn remove(&mut self, key: KeyCode) {
        let index = key as usize / KEYS_BITMAP_BITS;
        let offset = key as usize % KEYS_BITMAP_BITS;
        self.raw[index] &= !(1 << offset);
    }

    /// Checks if a key code is in the set.
    pub const fn contains(&self, key: KeyCode) -> bool {
        let index = key as usize / KEYS_BITMAP_BITS;
        let offset = key as usize % KEYS_BITMAP_BITS;
        (self.raw[index] & (1 << offset)) != 0
    }

    /// Returns a list of modifier keys
    pub const fn get_modifiers(&self) -> KeyModifiers {
        let mut modifiers = KeyModifiers::empty();
        if self.contains(KeyCode::Super) {
            modifiers = modifiers.union(KeyModifiers::SUPER);
        }
        if self.contains(KeyCode::Ctrl) {
            modifiers = modifiers.union(KeyModifiers::CTRL);
        }
        if self.contains(KeyCode::Alt) {
            modifiers = modifiers.union(KeyModifiers::ALT);
        }
        if self.contains(KeyCode::Shift) {
            modifiers = modifiers.union(KeyModifiers::SHIFT);
        }
        if self.contains(KeyCode::CapsLock) {
            modifiers = modifiers.union(KeyModifiers::CAPSLOCK);
        }
        modifiers
    }
}

/// Represents a keyboard device.
pub struct Keyboard {
    reader: BufReader<File>,
    last_keystate: PressedKeys,
}

const EVENTS_PER_READ: usize = 1024;

impl Keyboard {
    /// Creates a new keyboard instance.
    pub fn create() -> Self {
        let file = File::open("dev:/inkbd").expect("Failed to open keyboard device");
        let reader =
            BufReader::with_capacity(EVENTS_PER_READ * 4 * std::mem::size_of::<KeyEvent>(), file);
        Keyboard {
            reader,
            last_keystate: PressedKeys::default(),
        }
    }

    fn read_events(&mut self) -> io::Result<([KeyEvent; EVENTS_PER_READ], usize)> {
        let mut buf_raw = [0u8; EVENTS_PER_READ * size_of::<KeyEvent>()];
        let n = self.reader.read(&mut buf_raw)?;

        let buf = unsafe { core::mem::transmute(buf_raw) };

        Ok((buf, n / size_of::<KeyEvent>()))
    }

    /// Returns the current pressed keys.
    pub const fn current_pressed_keys(&self) -> PressedKeys {
        self.last_keystate
    }

    /// Handles keyboard events, returns Some(PressedKeys) if any keys are pressed
    pub fn handle_events(&mut self) -> Option<PressedKeys> {
        let (events_raw, n) = self.read_events().expect("Failed to read keyboard events");
        let events = &events_raw[..n];
        if events.is_empty() {
            return None;
        }

        let mut pressed_keys = self.last_keystate;

        for event in events {
            let event_to_send;
            macro_rules! prepare_send {
                ($event_kind: ident) => {
                    event_to_send = Some(libopal::event::Event::Key(libopal::event::KeyEvent {
                        kind: libopal::event::KeyEventKind::$event_kind,
                        /* MUST BE KEPT IN SYNC */
                        code: unsafe { core::mem::transmute(event.code) },
                        modifiers: pressed_keys.get_modifiers(),
                    }));
                };
            }

            match event.kind {
                KeyEventKind::Null => unreachable!(),
                KeyEventKind::Press => {
                    let was_pressed = pressed_keys.contains(event.code);
                    if !was_pressed {
                        pressed_keys.add(event.code);
                    }
                    prepare_send!(Press);
                }
                KeyEventKind::Release => {
                    let was_pressed = pressed_keys.contains(event.code);
                    if was_pressed {
                        pressed_keys.remove(event.code);
                    }
                    prepare_send!(Release);
                }
            }

            if let Some(to_send) = event_to_send {
                let mut windows = WINDOWS
                    .lock()
                    .expect("Failed to acquire lock on windows while sending keys");
                if let Some(focused) = windows.focused_window() {
                    windows.send_event(focused, to_send).expect(
                        "Failed to send a key event to focused window, because it doesn't exists",
                    );
                }
            }
        }
        self.last_keystate = pressed_keys;
        Some(pressed_keys)
    }
}
