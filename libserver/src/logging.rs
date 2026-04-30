use std::{
    fmt::Arguments,
    fs::{File, OpenOptions},
    io::{LineWriter, Write},
    sync::{
        LazyLock, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

static SERIAL: LazyLock<Mutex<LineWriter<File>>> = LazyLock::new(|| {
    Mutex::new(LineWriter::new(
        OpenOptions::new()
            .write(true)
            .read(true)
            .open("dev:/ss")
            .expect("Failed to open serial device"),
    ))
});

static LOG_TERM: AtomicBool = AtomicBool::new(true);

/// Returns a clone of the console we log to
pub fn console_clone() -> File {
    SERIAL
        .lock()
        .expect("Failed to acquire lock on Serial")
        .get_ref()
        .try_clone()
        .expect("Failed to clone the Serial")
}

/// Disable logging to the parent TTY
pub fn disable_terminal_logging() {
    LOG_TERM.store(false, Ordering::Release);
}

/// Returns whether or not we can log to the TTY
pub fn terminal_logging_enabled() -> bool {
    LOG_TERM.load(Ordering::Acquire)
}

#[doc(hidden)]
pub fn _write_to_serial(args: Arguments) {
    SERIAL
        .lock()
        .expect("failed to acquire lock on serial")
        .write_fmt(args)
        .expect("failed to write to the serial")
}

/// Generic log something attributing it to OpalWM
#[macro_export]
macro_rules! generic_log {
    ($($arg: tt)*) => {{
        $crate::logging::_write_to_serial(format_args!("[ \x1b[97mOpalWM\x1b[0m ] {}\n", format_args!($($arg)*)));
        if $crate::logging::terminal_logging_enabled() {
            println!("[ \x1b[97mOpalWM\x1b[0m ] {}", format_args!($($arg)*));
        }
    }};
}

/// Log information about an event that isn't a debug event
#[macro_export]
macro_rules! log {
    ($($arg: tt)*) => ($crate::generic_log!("[  \x1b[32mInfo\x1b[0m  ]\x1b[90m:\x1b[0m {}", format_args!($($arg)*)));
}

/// Log debug information
#[macro_export]
macro_rules! dlog {
    ($($arg: tt)*) => ($crate::generic_log!("[  \x1b[91mDebug\x1b[0m  ]\x1b[90m:\x1b[0m {}", format_args!($($arg)*)));
}

/// Log a non fatal error (perhaps will only affect a single client)
#[macro_export]
macro_rules! elog {
    ($($arg: tt)*) => ($crate::generic_log!("[  \x1b[31mError\x1b[0m  ]\x1b[90m:\x1b[0m {}", format_args!($($arg)*)));
}

#[macro_export]
macro_rules! wlog {
    ($($arg: tt)*) => {
        $crate::generic_log!("[  \x1b[33mWarn\x1b[0m   ]\x1b[90m:\x1b[0m {}", format_args!($($arg)*));
    };
}
