use std::time::Duration;

use libserver::{log, logging::disable_terminal_logging};

use crate::{
    com::listener::listener_thread,
    stream::{TICK_DURATION_MS, flush_pending, mixer_audio_info},
};

mod com;
pub mod stream;

fn mix_loop() -> ! {
    let audio_format = mixer_audio_info();
    log!("Starting mix loop with audio: {audio_format:#?}");

    loop {
        let (_, _) = flush_pending();
        std::thread::sleep(Duration::from_millis(TICK_DURATION_MS as u64));
    }
}
fn main() {
    println!("Hello, world!");
    disable_terminal_logging();

    std::thread::spawn(mix_loop);
    // std::thread::spawn(|| play_pcm(YOU_CAN_ALWAYS, 100.));

    listener_thread()
}
