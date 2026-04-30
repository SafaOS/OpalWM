use std::ptr::NonNull;

use lune_abi::{
    ShmKey,
    msg::{AllocateSharedObject, Request},
};
use safa_api::{abi::mem::MemMapFlags, mem::MemoryMapper};

use crate::server;

/// SharedObject represents a shared memory object allocated by the WM.
#[derive(Debug)]
pub struct SharedObject {
    inner: safa_api::shm::SharedObject,
}

impl SharedObject {
    /// Allocates a new shared memory object with the given size, shared with the WM.
    ///
    /// Returns a Result containing the SharedObject or an ErrorStatus if allocation fails.
    pub fn allocate(size: usize) -> Result<Self, safa_api::errors::ErrorStatus> {
        let key = server::send_request_or_panic!(
            Request::AllocateObject(AllocateSharedObject::new(size)),
            AllocatedObject(new)
        )
        .key();

        let shared_object = safa_api::shm::SharedObject::map_open(
            &MemoryMapper::new().flags(MemMapFlags::WRITE | MemMapFlags::DISABLE_EXEC),
            key,
            size,
        )?;
        Ok(Self {
            inner: shared_object,
        })
    }

    /// Returns the key of the shared memory object.
    pub fn shm_key(&self) -> ShmKey {
        self.inner.shm_key()
    }

    /// Returns the pointer to the shared memory buffer.
    pub const fn data_inner(&self) -> NonNull<[u8]> {
        self.inner.data_ptr()
    }

    /// Returns a reference to the shared memory buffer, should generally be safe as the WM doesn't write data unless requested
    #[inline(always)]
    pub const fn data(&self) -> &[u8] {
        unsafe { self.inner.data() }
    }

    /// Returns a mutable reference to the shared memory buffer, should generally be safe as the WM doesn't write data unless requested
    #[inline(always)]
    pub const fn data_mut(&mut self) -> &mut [u8] {
        unsafe { self.inner.data_mut() }
    }
}

impl Drop for SharedObject {
    fn drop(&mut self) {
        server::send_request_or_panic!(Request::DestroyObject(self.inner.shm_key()), Success(_s));
    }
}
