use macros::EncodeableMessage;

use crate::{
    defs::{HeldMouseButtons, KeyModifiers, WindowFlags, WindowID},
    encoding::HasMaxEncodeSize,
    impl_inheritly,
};

/// When the mouse cursor enters a window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, EncodeableMessage)]
pub struct MouseEnterEvent {
    /// The x-coordinate of the mouse cursor, relative to the window.
    pos_x: u32,
    /// The y-coordinate of the mouse cursor, relative to the window.
    pos_y: u32,
}

impl MouseEnterEvent {
    /// Returns the x-coordinate of the mouse cursor, relative to the window.
    pub const fn x(&self) -> u32 {
        self.pos_x
    }

    /// Returns the y-coordinate of the mouse cursor, relative to the window.
    pub const fn y(&self) -> u32 {
        self.pos_y
    }
}

/// When the mouse cursor leaves a window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, EncodeableMessage)]
pub struct MouseLeaveEvent;

/// When the mouse cursor moves within a window or a change to it's buttons occurs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, EncodeableMessage)]
pub struct MouseChangeEvent {
    /// Whether or not the buttons has changed.
    buttons_changed: bool,
    /// The buttons that are currently held down.
    held_buttons: HeldMouseButtons,
    /// The x-coordinate of the mouse cursor, relative to the window.
    pos_x: u32,
    /// The y-coordinate of the mouse cursor, relative to the window.
    pos_y: u32,
}

impl MouseChangeEvent {
    /// Returns whether or not the buttons have changed.
    pub const fn buttons_changed(&self) -> bool {
        self.buttons_changed
    }

    /// Returns the buttons that are currently held down.
    pub const fn held_buttons(&self) -> HeldMouseButtons {
        self.held_buttons
    }

    /// Returns the change in buttons that occurred if the buttons have changed otherwise None.
    pub const fn buttons_change(&self) -> Option<HeldMouseButtons> {
        if self.buttons_changed {
            Some(self.held_buttons)
        } else {
            None
        }
    }

    /// Returns the x-coordinate of the mouse cursor, relative to the window.
    pub const fn x(&self) -> u32 {
        self.pos_x
    }

    /// Returns the y-coordinate of the mouse cursor, relative to the window.
    pub const fn y(&self) -> u32 {
        self.pos_y
    }
}

/// Keeps in sync with the `KeyCode` enum in the `safa-abi` crate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u16)]
pub enum KeyCode {
    Null = 0,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    PrintScr,

    Esc,
    Key1,
    Key2,
    Key3,
    Key4,
    Key5,
    Key6,
    Key7,
    Key8,
    Key9,
    Key0,
    Minus,
    Equals,
    Backspace,

    KeyQ,
    KeyW,
    KeyE,
    KeyR,
    KeyT,
    KeyY,
    KeyU,
    KeyI,
    KeyO,
    KeyP,
    LeftBrace,
    RightBrace,
    BackSlash,

    KeyA,
    KeyS,
    KeyD,
    KeyF,
    KeyG,
    KeyH,
    KeyJ,
    KeyK,
    KeyL,
    Semicolon,
    DoubleQuote,
    Return,

    KeyZ,
    KeyX,
    KeyC,
    KeyV,
    KeyB,
    KeyN,
    KeyM,
    BackQuote,
    Comma,
    Dot,
    Slash,

    Tab,
    CapsLock,
    Ctrl,
    Shift,
    Alt,
    Super,
    Space,
    Up,
    Down,
    Left,
    Right,

    PageUp,
    PageDown,
    Insert,
    Delete,
    Home,
    End,

    // used to figure out Max of KeyCode
    LastKey,
}
impl KeyCode {
    /// Attempts to convert a u16 value into a KeyCode.
    pub const fn try_from(value: u16) -> Result<Self, ()> {
        const LAST_KEY: u16 = KeyCode::LastKey as u16;

        match value {
            0..LAST_KEY => Ok(unsafe { std::mem::transmute(value) }),
            _ => Err(()),
        }
    }
}

impl_inheritly!(u16, KeyCode, from_u16 => {
    let Ok(key) = KeyCode::try_from(from_u16) else {
        return Err(crate::DecodeError::InvalidData.into());
    };
    key
}, from_self => *from_self as u16);

/// Represents the kind of key event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum KeyEventKind {
    Null = 0,
    Press = 1,
    Release = 2,
}

impl KeyEventKind {
    /// Attempts to convert a u8 value into a KeyEventKind.
    pub const fn try_from(value: u8) -> Result<Self, ()> {
        match value {
            0 => Ok(Self::Null),
            1 => Ok(Self::Press),
            2 => Ok(Self::Release),
            _ => Err(()),
        }
    }
}

impl_inheritly!(u8, KeyEventKind, from_u8 => {
    let Ok(key) = Self::try_from(from_u8) else {
        return Err(crate::DecodeError::InvalidData.into());
    };
    key
}, from_self => *from_self as u8);

/// Represents a key event that occurred on an active window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, EncodeableMessage)]
#[repr(C)]
pub struct KeyEvent {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
    pub kind: KeyEventKind,
}

#[derive(Debug, Clone, Copy, EncodeableMessage, PartialEq, Eq)]
/// A broadcast event when a window is added.
pub struct WindowAttached {
    id: WindowID,
    x: i32,
    y: i32,
    flags: WindowFlags,
}

impl WindowAttached {
    /// Returns the ID of the attached Window.
    pub const fn win_id(&self) -> u16 {
        self.id
    }

    pub const fn win_flags(&self) -> WindowFlags {
        self.flags
    }

    pub const fn x(&self) -> i32 {
        self.x
    }

    pub const fn y(&self) -> i32 {
        self.y
    }
}

/// A broadcast event when a window is removed.
#[derive(Debug, Clone, Copy, EncodeableMessage, PartialEq, Eq)]
pub struct WindowDeatached {
    id: WindowID,
}

impl WindowDeatached {
    /// Returns the ID of the deatached Window
    pub const fn win_id(&self) -> u16 {
        self.id
    }
}

/// Broadcasted when a window focus status changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, EncodeableMessage)]
pub struct WindowFocusChanged {
    is_focused: bool,
}

impl WindowFocusChanged {
    /// Returns whether the window is focused.
    pub const fn is_focused(&self) -> bool {
        self.is_focused
    }
}

/// Represents an event that occurred on a window.
///
/// Is Always followed by a target WindowID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, EncodeableMessage)]
#[repr(u16)]
pub enum WindowEvent {
    MouseChange(MouseChangeEvent) = 0xE0,
    MouseLeave(MouseLeaveEvent) = 0xE1,
    MouseEnter(MouseEnterEvent) = 0xE2,
    WindowFocusChanged(WindowFocusChanged) = 0xE3,
    Key(KeyEvent) = 0xE4,
    GlobalWindowAttached(WindowAttached) = 0xE00,
    GlobalWindowDeatached(WindowDeatached) = 0xE01,
    GlobalWindowFocusChanged(WindowFocusChanged, WindowID) = 0xE03,
}

impl HasMaxEncodeSize for WindowEvent {
    const ENCODE_SIZE: usize = 128;
}

/// Represents an Event that occurred, for the current Application.
///
/// The event has a target Window with in the application, although it may not be the same as the window the event was triggered on, this is stored within [`WindowEvent`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, EncodeableMessage)]
pub struct Event {
    event: WindowEvent,
    target_window_id: WindowID,
}

impl Event {
    /// Returns the event that occurred.
    pub const fn event(&self) -> WindowEvent {
        self.event
    }

    /// Returns the ID of the window that the event is supposed to be sent to.
    pub const fn receiver(&self) -> WindowID {
        self.target_window_id
    }
}
