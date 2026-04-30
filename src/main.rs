use std::thread;

use opal_abi::log;
use safa_api::abi::input::KeyCode;

use crate::com::listener;
use crate::kbd::Keyboard;
use crate::mice::MiceCursor;
use crate::window::redraw;

/// Set to true if you want really verbose slow information
///
/// TODO: make this a cmd line arg or perhaps a feature
const REALLY_VERBOSE: bool = false;

mod com;
mod framebuffer;
mod icons;
mod kbd;
pub use libserver::logging;
pub use logging::*;
mod mice;
mod window;

fn main_loop() {
    let mut keyboard = Keyboard::create();
    let mut cursor = MiceCursor::create();
    loop {
        let pressed_keys = keyboard.handle_events();
        let curr_pressed_keys = pressed_keys.unwrap_or(keyboard.current_pressed_keys());
        if let Some(keys) = pressed_keys {
            if keys.contains(KeyCode::Shift) && keys.contains(KeyCode::Ctrl) {
                if keys.contains(KeyCode::KeyT) {
                    listener::spawn_terminal();
                } else if keys.contains(KeyCode::KeyE) {
                    std::process::exit(0);
                }
            }
        }

        if cursor.handle_event(curr_pressed_keys) {
            redraw();
        }
        thread::yield_now();
    }
}
fn main() {
    log!("WM Starting");
    disable_terminal_logging();
    framebuffer::clear();
    std::thread::spawn(main_loop);
    listener::listener_thread()
}
