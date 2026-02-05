use std::ptr::NonNull;

use opal_abi::{
    defs::ShmKey,
    msg::{AllocateSharedObject, DestroyObject, Request},
};
use safa_api::{
    abi::mem::{MemMapFlags, ShmFlags},
    syscalls::{self, types::Ri},
};

use crate::send_request_or_panic;

/// SharedObject represents a shared memory object allocated by the WM.
#[derive(Debug)]
pub struct SharedObject {
    key: ShmKey,
    mem_map_resource: Ri,
    buf: NonNull<[u8]>,
}

impl SharedObject {
    /// Allocates a new shared memory object with the given size, shared with the WM.
    ///
    /// Returns a Result containing the SharedObject or an ErrorStatus if allocation fails.
    pub fn allocate(size: usize) -> Result<Self, safa_api::errors::ErrorStatus> {
        let key = send_request_or_panic!(
            Request::AllocateObject(AllocateSharedObject::new(size)),
            AllocatedObject(new)
        )
        .key();
        let shm_ri = syscalls::mem::shm_open(key, ShmFlags::from_bits_retaining(0))
            .expect("Failed to open ShmKey");

        let (mem_ri, buf) = syscalls::mem::map(
            core::ptr::null(),
            size.div_ceil(4096),
            0,
            Some(shm_ri),
            None,
            MemMapFlags::WRITE | MemMapFlags::DISABLE_EXEC,
        )?;

        syscalls::resources::destroy_resource(shm_ri)
            .expect("Failed to destroy SHM Descriptor resource");
        Ok(Self {
            key,
            mem_map_resource: mem_ri,
            buf,
        })
    }

    /// Returns the key of the shared memory object.
    pub fn shm_key(&self) -> ShmKey {
        self.key
    }

    /// Returns the pointer to the shared memory buffer.
    pub const fn data_inner(&self) -> NonNull<[u8]> {
        self.buf
    }

    /// Returns a reference to the shared memory buffer, should generally be safe as the WM doesn't write data unless requested
    #[inline(always)]
    pub const fn data(&self) -> &[u8] {
        unsafe { self.buf.as_ref() }
    }

    /// Returns a mutable reference to the shared memory buffer, should generally be safe as the WM doesn't write data unless requested
    #[inline(always)]
    pub const fn data_mut(&mut self) -> &mut [u8] {
        unsafe { self.buf.as_mut() }
    }
}

impl Drop for SharedObject {
    fn drop(&mut self) {
        syscalls::resources::destroy_resource(self.mem_map_resource)
            .expect("Failed to destroy memory mapping");
        send_request_or_panic!(Request::DestroyObject(DestroyObject, self.key), Success(_s));
    }
}
