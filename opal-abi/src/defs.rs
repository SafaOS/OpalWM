//! Contains definitions of common Identifiers and flags types.

use crate::encoding::DecodeError;
use std::num::NonZero;

use bitflags::bitflags;

use crate::encoding::impl_inheritly;
macro_rules! impl_bitflags {
    ($ty: ty, $o_ty: ty) => {

        impl_inheritly!($o_ty, $ty, from_other => {
            let Some(r) = Self::from_bits(from_other) else {
                return Err(DecodeError::InvalidData.into());
            };
            r
        }, from_self => from_self.bits());
    };
}
bitflags! {
    /// Flags to create a new window with
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct WindowFlags: u32 {
        /// The window shall come below normal windows, and cannot be dragged or focused on.
        const BG_WINDOW = 1 << 0;
        /// The window shall come on top of normal windows, and cannot be dragged or focused on.
        const OVERLAY_WINDOW = 1 << 1;
        /// Window doesn't like the WM decorating it with dumb stuff.
        const NO_DECORATIONS = 1 << 2;
        /// The window's creation/removal is public information,
        /// anyone can access the window ID and a global event will be bordcast on
        /// creation/removal and some window changes.
        const GLOBAL = 1 << 3;
    }
}

bitflags! {
    /// Information about the window status, such as if it is focused or not.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct WindowStatus: u32 {
        const FOCUSED = 1 << 0;
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

impl_bitflags!(WindowFlags, u32);
impl_bitflags!(WindowStatus, u32);
impl_bitflags!(HeldMouseButtons, u8);

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
        const CAPSLOCK = 1 << 4;
    }
}

impl_bitflags!(KeyModifiers, u16);

/// Identifies an Icon.
pub type IconID = NonZero<u16>;
/// Identifies a Window.
pub type WindowID = u16;
/// Describes a shared memory Key, the WM doesn't accept a key it doesn't own for security reasons.
pub type ShmKey = usize;
