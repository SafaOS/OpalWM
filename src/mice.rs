//! Mouse and cursor handling
//!
//! This module provides functionality for handling mouse input.
//! It includes functions for reading mouse events from the mouse device,
//! handling the mouse cursor, and dispatching mouse events to the
//! active window.

use std::{
    fs::File,
    io::{BufReader, Read},
};

use libopal::WindowEvent;
use opal_abi::msg::event::{MouseChangeEvent, MouseEnterEvent, MouseLeaveEvent};
use opal_abi::{Name, defs::HeldMouseButtons};
use opal_img::bmp::BMPImage;
use safa_api::abi::input::{KeyCode, MiceBtnStatus, MiceEvent, MouseEventKind};

use crate::{
    dlog,
    kbd::PressedKeys,
    window::{WINDOWS, WinID, Window, WindowKind},
};

const CURSOR_BYTES: &[u8] = include_bytes!("../assets/epic-cursor-v2.bmp");

/// The MiceCursor struct represents a mouse cursor on the screen, also handles mouse events.
pub struct MiceCursor {
    win_id: WinID,
    x: isize,
    y: isize,
    height: usize,
    width: usize,
    last_mouse_event: MiceEvent,
    current_window: Option<WinID>,
    reader: BufReader<File>,
}

impl MiceCursor {
    /// Creates a new MiceCursor instance
    pub fn create() -> Self {
        let cursor_bmp = BMPImage::from_slice(CURSOR_BYTES).expect("Failed to parse cursor.bmp");
        let cursor_width = cursor_bmp.width();
        let cursor_height = cursor_bmp.height();

        let file = File::open("dev:/inmice").expect("Failed to open the Mouse Device");
        let reader = BufReader::with_capacity(size_of::<MiceEvent>() * 1, file);
        let win = {
            let mut windows = WINDOWS.lock().expect("failed to get lock on windows");
            windows
                .add_window(
                    Window::new_from_bmp(Name::new_truncate("cursor"), None, 0, 0, cursor_bmp),
                    WindowKind::Cursor,
                    true,
                )
                .expect("Failed to add the Mouse cursor's window")
        };

        dlog!("Added window {win} for cursor");
        Self {
            win_id: win,
            x: 0,
            y: 0,
            height: cursor_height as usize,
            width: cursor_width as usize,
            last_mouse_event: MiceEvent {
                kind: MouseEventKind::Null,
                buttons_status: MiceBtnStatus::NO_BUTTONS,
                x_rel_change: 0,
                y_rel_change: 0,
            },
            current_window: None,
            reader,
        }
    }

    /// Handles one mouse event if available
    pub fn handle_event(&mut self, pressed_keys: PressedKeys) -> bool {
        const EVENTS_COUNT: usize = 1024;

        let mut event_bytes = [0u8; size_of::<MiceEvent>() * EVENTS_COUNT];
        let len = self
            .reader
            .read(&mut event_bytes)
            .expect("Failed to read an event");

        if len == 0 {
            return false;
        }

        assert!(len.is_multiple_of(size_of::<MiceEvent>()));

        let events: [MiceEvent; EVENTS_COUNT] = unsafe { core::mem::transmute(event_bytes) };
        let events_count = len / size_of::<MiceEvent>();
        let events = &events[..events_count];

        let ctrl_pressed = pressed_keys.contains(KeyCode::Ctrl);
        for event in events {
            match event.kind {
                MouseEventKind::Change => {
                    let old_win_id = self.current_window;
                    let left_button_was_pressed = self
                        .last_mouse_event
                        .buttons_status
                        .contains(MiceBtnStatus::BTN_LEFT);

                    let x_change = (event.x_rel_change) as i32;
                    let y_change = (-event.y_rel_change) as i32;

                    let mut windows = WINDOWS.lock().expect("failed to get lock on windows");

                    if !(x_change == 0 && y_change == 0) {
                        let (new_x, new_y) =
                            windows.add_cord(self.win_id, x_change, y_change).unwrap();
                        self.x = new_x;
                        self.y = new_y;
                    }

                    let window_in_contact =
                        windows.window_in_contact(self.x, self.y, self.width, self.height);

                    let mut receive_events = true;

                    let left_button_is_pressed =
                        event.buttons_status.contains(MiceBtnStatus::BTN_LEFT);
                    if left_button_was_pressed
                        && let Some(focused_id) = windows.focused_window()
                        && left_button_is_pressed
                        && ctrl_pressed
                    {
                        windows.add_cord(focused_id, x_change, y_change);
                        receive_events = false;
                    }

                    match window_in_contact {
                        Some((curr_id, kind, contact_point)) => {
                            let can_focus = kind == WindowKind::Normal;

                            let mut mouse_enter = false;
                            let x = contact_point.x() as u32;
                            let y = contact_point.y() as u32;

                            if receive_events && old_win_id.is_none_or(|old_id| old_id != curr_id) {
                                windows
                                    .send_event(
                                        curr_id,
                                        WindowEvent::MouseEnter(MouseEnterEvent::new(x, y)),
                                    )
                                    .expect("Window removed before we could send an event to it");
                                mouse_enter = true;
                            }

                            if receive_events
                                && let Some(old_id) = old_win_id
                                && mouse_enter
                            {
                                /* It is ok the old window might be gone by now */
                                _ = windows.send_event(
                                    old_id,
                                    WindowEvent::MouseLeave(MouseLeaveEvent::new()),
                                );
                            }

                            // FIXME: for some reason mouse release events are not being sent by the kernel driver.
                            if receive_events && !mouse_enter {
                                let mut held_buttons = HeldMouseButtons::empty();

                                if left_button_is_pressed {
                                    held_buttons.insert(HeldMouseButtons::LEFT);
                                }

                                if event.buttons_status.contains(MiceBtnStatus::BTN_MID) {
                                    held_buttons.insert(HeldMouseButtons::MIDDLE);
                                }

                                if event.buttons_status.contains(MiceBtnStatus::BTN_RIGHT) {
                                    held_buttons.insert(HeldMouseButtons::RIGHT);
                                }

                                let buttons_changed =
                                    self.last_mouse_event.buttons_status != event.buttons_status;

                                let change_event =
                                    MouseChangeEvent::new(buttons_changed, held_buttons, x, y);
                                windows.send_event(curr_id, WindowEvent::MouseChange(change_event)).expect("Current Window was removed before we could handle a mouse event");
                            }

                            if can_focus
                                && windows
                                    .focused_window()
                                    .is_none_or(|focus_id| focus_id != curr_id)
                                && left_button_is_pressed
                                && !left_button_was_pressed
                            {
                                windows.set_focused(curr_id);
                            }
                        }
                        None => {
                            if let Some(old_id) = old_win_id {
                                /* It is ok the old window might be gone by now */
                                _ = windows.send_event(
                                    old_id,
                                    WindowEvent::MouseLeave(MouseLeaveEvent::new()),
                                );
                            }

                            if left_button_is_pressed && !left_button_was_pressed {
                                windows.unfocus_current();
                            }
                        }
                    }

                    self.last_mouse_event = event.clone();
                    self.current_window = window_in_contact.map(|(id, _, _)| id);
                }
                MouseEventKind::Null => unreachable!(),
            }
        }

        true
    }
}
