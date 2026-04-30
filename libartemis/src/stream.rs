///! Raw interface over audio streams.
use std::sync::Arc;

use lune_abi::{
    StreamID,
    misc::AudioFormat,
    msg::{CreateStream, Request, SyncStream},
};

use crate::{server, shm::SharedObject};

/// A raw Audio stream.
#[derive(Debug)]
pub struct Stream {
    shared_object: Arc<SharedObject>,
    format: AudioFormat,
    id: StreamID,
}

impl Stream {
    pub const fn bytes_per_sample(&self) -> u8 {
        self.format.bit_depth() as u8 / 8
    }

    /// Safely Returns the memory buffer of the stream.
    pub fn buf_mut(&mut self) -> &mut [u8] {
        unsafe { self.shared_object.data_inner().as_mut() }
    }

    /// Syncs `amount` samples from the stream's shared buffer [`Self::buf_mut`].
    ///
    /// Given an offset from within the buffer in samples.
    pub fn sync(&mut self, offset: u32, amount: u32) -> u32 {
        let (sync_results, _) = server::send_request_and_get!(
            Request::SyncStream(SyncStream::new(offset, amount as u32), self.id()),
            StreamSynced(s, _i)
        )
        .expect("Failed to sync stream");

        sync_results.synced()
    }

    /// Given bytes of samples in the [`Stream::format`], writes and syncs them with the stream.
    ///
    /// returns the amount of bytes wrote.
    pub fn write_bytes(&mut self, bytes: &[u8]) -> usize {
        let bytes_per_sample = self.bytes_per_sample() as usize;

        let buffer = self.buf_mut();
        let byte_amount = bytes.len().min(buffer.len());
        buffer.copy_from_slice(&bytes[..byte_amount]);

        let samples_count = byte_amount / bytes_per_sample;
        let synced = self.sync(0, samples_count as u32);

        synced as usize * bytes_per_sample
    }

    /// Creates a new stream with the given audio format and samples count.
    pub fn create(format: AudioFormat, samples: u32) -> Self {
        let obj = Arc::new(
            SharedObject::allocate(samples as usize * (format.bit_depth() as usize / 8))
                .expect("Failed to allocate memory for stream"),
        );

        unsafe { Self::create_in(obj, format, samples).expect("Failed to construct a new stream") }
    }

    /// Creates a new stream that holds it's data in the given shared object.
    ///
    /// # Safety:
    /// Stream will muttate the shared object's buffer.
    pub unsafe fn create_in(
        obj: Arc<SharedObject>,
        format: AudioFormat,
        samples: u32,
    ) -> Result<Self, lune_abi::msg::ResponseError> {
        let id = server::send_request_and_get!(
            Request::CreateStream(CreateStream::new(
                obj.shm_key(),
                samples,
                format.channels() as u8,
                format.bit_depth() as u8,
                format.sample_format() as u8,
                format.freq()
            )),
            StreamID(id)
        )?;

        Ok(Self {
            shared_object: obj,
            format,
            id,
        })
    }

    pub const fn format(&self) -> &AudioFormat {
        &self.format
    }
    pub const fn id(&self) -> StreamID {
        self.id
    }
    pub const fn shared_obj(&self) -> &Arc<SharedObject> {
        &self.shared_object
    }
}

impl Drop for Stream {
    fn drop(&mut self) {
        server::send_request_or_panic!(Request::DestroyStream(self.id()), Success(_s));
    }
}
