//! SafaOS VTTYs Implementation
//! VTTYs can work as a pipe
//!
//! TODO: Move to the safa-api crate

use safa_api::{
    abi::poll::{PollEntry, PollEvents},
    errors::ErrorStatus,
    vtty::MotherVTTY,
};

use crate::executor;

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

/// Async-read from a MotherVTTY, blocks until there is data.
pub async fn mother_read_async(mother: &MotherVTTY, buf: &mut [u8]) -> Result<usize, ErrorStatus> {
    MotherVTTYReaderFuture { vtty: mother, buf }.await
}
