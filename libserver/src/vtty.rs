//! SafaOS VTTYs Implementation
//! VTTYs can work as a pipe
//!
//! TODO: Move to the safa-api crate

use safa_api::{
    abi::poll::{PollEntry, PollEvents},
    errors::ErrorStatus,
    syscalls::{self, types::Ri},
};

use crate::executor;

#[derive(Debug)]
pub struct MotherVTTY {
    ri: Ri,
}

#[derive(Debug)]
pub struct ChildVTTY {
    ri: Ri,
}

impl Drop for MotherVTTY {
    fn drop(&mut self) {
        // TODO: add Resource type with drops like these to the api
        safa_api::syscalls::resources::destroy_resource(self.ri)
            .expect("Failed to destroy mother TTY")
    }
}

impl Drop for ChildVTTY {
    fn drop(&mut self) {
        // TODO: add Resource type with drops like these to the api
        safa_api::syscalls::resources::destroy_resource(self.ri)
            .expect("Failed to destroy child TTY")
    }
}

impl MotherVTTY {
    pub const SET_FLAGS: u16 = 1;

    #[inline(always)]
    pub const fn ri(&self) -> Ri {
        self.ri
    }

    /// Reads data from the VTTY at the specified offset into the provided buffer.
    pub fn read(&self, offset: isize, buf: &mut [u8]) -> Result<usize, ErrorStatus> {
        syscalls::io::read(self.ri, offset, buf)
    }

    /// Sends a command to the VTTY with the specified command and argument.
    pub fn send_command(&self, command: u16, argument: u64) -> Result<(), ErrorStatus> {
        syscalls::io::io_command(self.ri, command, argument)
    }

    /// Sets the flags for the VTTY.
    pub fn set_flags(&self, flags: u64) -> Result<(), ErrorStatus> {
        self.send_command(Self::SET_FLAGS, flags)
    }
}

impl ChildVTTY {
    /// Writes data to the VTTY at the specified offset from the provided buffer.
    pub fn write(&self, offset: isize, buf: &[u8]) -> Result<usize, ErrorStatus> {
        syscalls::io::write(self.ri, offset, buf)
    }
}

struct MotherVTTYReaderFuture<'a, 'b> {
    vtty: &'a MotherVTTY,
    buf: &'b mut [u8],
}

impl<'a, 'b> Future for MotherVTTYReaderFuture<'a, 'b> {
    type Output = Result<usize, ErrorStatus>;
    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = &mut *self;
        match this.vtty.read(0, &mut this.buf) {
            Ok(0) => {
                executor::poll_resource(
                    PollEntry::new(this.vtty.ri(), PollEvents::DATA_AVAILABLE),
                    cx.waker().clone(),
                );
                std::task::Poll::Pending
            }
            Ok(n) => std::task::Poll::Ready(Ok(n)),
            Err(e) => std::task::Poll::Ready(Err(e)),
        }
    }
}

impl MotherVTTY {
    /// Async-read from a MotherVTTY, blocks until there is data.
    pub async fn read_async(&self, buf: &mut [u8]) -> Result<usize, ErrorStatus> {
        MotherVTTYReaderFuture { vtty: self, buf }.await
    }
}

/// Construct new pair of (`MotherVTTY`, `ChildVTTY`)
pub fn new() -> (MotherVTTY, ChildVTTY) {
    let (mother_ri, child_ri) = syscalls::io::vtty_alloc().expect("Failed to allocate VTTY");
    (MotherVTTY { ri: mother_ri }, ChildVTTY { ri: child_ri })
}
