use crate::com::listener;
use crate::kbd::Keyboard;
use crate::logging::disable_terminal_logging;
use crate::mice::MiceCursor;
use crate::window::redraw;

/// Set to true if you want really verbose slow information
///
/// TODO: make this a cmd line arg or perhaps a feature
const REALLY_VERBOSE: bool = false;

mod com;
mod framebuffer;
mod kbd;
mod logging;
mod mice;
mod window;

fn main_loop() {
    let mut keyboard = Keyboard::create();
    let mut cursor = MiceCursor::create();
    loop {
        let pressed_keys = keyboard.handle_events();
        cursor.handle_event(pressed_keys.unwrap_or(keyboard.current_pressed_keys()));
        redraw();
    }
}
fn main() {
    log!("WM Starting");
    disable_terminal_logging();
    framebuffer::clear();
    std::thread::spawn(main_loop);
    listener::listen()
}
