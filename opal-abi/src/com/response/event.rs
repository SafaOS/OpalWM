use bincode::{Decode, Encode};
use bitflags::bitflags;

use crate::com::request::WindowFlags;

/// When the mouse cursor enters a window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode)]
#[repr(C)]
pub struct MouseEnterEvent {
    /// The x-coordinate of the mouse cursor, relative to the window.
    pos_x: u32,
    /// The y-coordinate of the mouse cursor, relative to the window.
    pos_y: u32,
}

impl MouseEnterEvent {
    /// Creates a new `MouseEnterEvent`.
    pub fn new(pos_x: u32, pos_y: u32) -> Self {
        Self { pos_x, pos_y }
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

/// When the mouse cursor leaves a window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode)]
#[repr(C)]
pub struct MouseLeaveEvent;

impl MouseLeaveEvent {
    /// Creates a new `MouseLeaveEvent`.
    pub fn new() -> Self {
        Self
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct HeldMouseButtons: u8 {
        const LEFT = 1 << 0;
        const MIDDLE = 1 << 1;
        const RIGHT = 1 << 2;
    }
}

impl Encode for HeldMouseButtons {
    fn encode<E: bincode::enc::Encoder>(
        &self,
        encoder: &mut E,
    ) -> Result<(), bincode::error::EncodeError> {
        u8::encode(&self.bits(), encoder)
    }
}

impl<Context> Decode<Context> for HeldMouseButtons {
    fn decode<D: bincode::de::Decoder<Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, bincode::error::DecodeError> {
        u8::decode(decoder).map(|bits| HeldMouseButtons::from_bits_retain(bits))
    }
}

bincode::impl_borrow_decode!(HeldMouseButtons);

/// When the mouse cursor moves within a window or a change to it's buttons occurs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode)]
#[repr(C)]
pub struct MouseChangeEvent {
    /// Whether or not the buttons has changed.
    buttons_changed: bool,
    /// The buttons that are currently held down.
    held_buttons: HeldMouseButtons,
    __: u16,
    /// The x-coordinate of the mouse cursor, relative to the window.
    pos_x: u32,
    /// The y-coordinate of the mouse cursor, relative to the window.
    pos_y: u32,
}

impl MouseChangeEvent {
    /// Creates a new `MouseChangeEvent`.
    pub fn new(
        buttons_changed: bool,
        held_buttons: HeldMouseButtons,
        pos_x: u32,
        pos_y: u32,
    ) -> Self {
        Self {
            buttons_changed,
            held_buttons,
            __: 0,
            pos_x,
            pos_y,
        }
    }

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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode)]
#[repr(u32)]
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

bitflags! {
    /// Represents the state of modifier keys.
    /// the current modifier keys are:
    /// - Super
    /// - Ctrl
    /// - Alt
    /// - Shift
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct KeyModifiers: u16 {
        const CTRL = 1 << 0;
        const ALT = 1 << 1;
        const SHIFT = 1 << 2;
        const SUPER = 1 << 3;
    }
}

impl Encode for KeyModifiers {
    fn encode<E: bincode::enc::Encoder>(
        &self,
        encoder: &mut E,
    ) -> Result<(), bincode::error::EncodeError> {
        u16::encode(&self.bits(), encoder)
    }
}

impl<Context> Decode<Context> for KeyModifiers {
    fn decode<D: bincode::de::Decoder<Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, bincode::error::DecodeError> {
        u16::decode(decoder).map(|bits| Self::from_bits_retain(bits))
    }
}

bincode::impl_borrow_decode!(KeyModifiers);

/// Represents the kind of key event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode)]
pub enum KeyEventKind {
    Null = 0,
    Press,
    Release,
}

/// Represents a key event that occurred on an active window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode)]
#[repr(C)]
pub struct KeyEvent {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
    pub kind: KeyEventKind,
}

#[derive(Debug, Clone, Copy, Encode, Decode, PartialEq, Eq)]
#[repr(C)]
/// A globally broadcast event when a window is added.
pub struct GlobalWindowAttached {
    id: u16,
    __: u16,
    x: i32,
    y: i32,
    flags: WindowFlags,
}

impl GlobalWindowAttached {
    pub const fn new(id: u16, x: i32, y: i32, flags: WindowFlags) -> Self {
        Self {
            id,
            __: 0,
            x,
            y,
            flags,
        }
    }

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

/// A globally broadcast event when a window is removed.
#[derive(Debug, Clone, Copy, Encode, Decode, PartialEq, Eq)]
#[repr(C)]
pub struct GlobalWindowDeatached {
    id: u16,
    __0: u16,
    __1: u64,
    __2: u64,
}

impl GlobalWindowDeatached {
    pub const fn new(win_id: u16) -> Self {
        Self {
            id: win_id,
            __0: 0,
            __1: 0,
            __2: 0,
        }
    }
}

/// A globally broadcasted event when a global window is focused.
#[derive(Debug, Clone, Copy, Encode, Decode, PartialEq, Eq)]
#[repr(C)]
pub struct GlobalWindowFocused {
    id: u16,
}

impl GlobalWindowFocused {
    pub const fn new(id: u16) -> Self {
        Self { id }
    }

    pub const fn win_id(&self) -> u16 {
        self.id
    }
}

/// A globally broadcasted event when a global window is unfocused.
#[derive(Debug, Clone, Copy, Encode, Decode, PartialEq, Eq)]
#[repr(C)]
pub struct GlobalWindowUnfocused {
    id: u16,
}

impl GlobalWindowUnfocused {
    pub const fn new(id: u16) -> Self {
        Self { id }
    }

    pub const fn win_id(&self) -> u16 {
        self.id
    }
}

/// Represents an event that occurred on a window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode)]
#[repr(u32)]
pub enum Event {
    MouseChange(MouseChangeEvent),
    MouseLeave(MouseLeaveEvent),
    MouseEnter(MouseEnterEvent),
    WindowFocused,
    WindowUnfocused,
    Key(KeyEvent),
    GlobalWindowAttached(GlobalWindowAttached),
    GlobalWindowDeatached(GlobalWindowDeatached),
    GlobalWindowFocused(GlobalWindowFocused),
    GlobalWindowUnfocused(GlobalWindowUnfocused),
}

/// Represents an event, has the id of the window which shall receive that event
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode)]
pub struct WindowEvent {
    event: Event,
    win_id: u16,
    __: u16,
}

impl WindowEvent {
    pub const fn new(win_id: u16, event: Event) -> Self {
        Self {
            event,
            win_id,
            __: 0,
        }
    }

    /// ID of the window which should receive that event
    pub const fn win(&self) -> u16 {
        self.win_id
    }
    /// The payload event
    pub const fn event(&self) -> Event {
        self.event
    }
}
