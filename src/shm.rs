//! Shared memory objects used by the compositor.
//!
//! TODO: Move to the API.

use std::ptr::NonNull;

use libopal::defs::ShmKey;
use safa_api::{
    abi::mem::{MemMapFlags, ShmFlags},
    syscalls::{self, types::Ri},
};

/// SharedObject represents a shared memory object allocated by the WM.
#[derive(Debug)]
pub struct SharedObject {
    key: ShmKey,
    shm_resource: Ri,
    mem_map_resource: Ri,
    buf: NonNull<[u8]>,
}

unsafe impl Send for SharedObject {}
unsafe impl Sync for SharedObject {}

impl SharedObject {
    /// Allocates a new shared memory object with the given size, shared with the WM.
    ///
    /// Returns a Result containing the SharedObject or an ErrorStatus if allocation fails.
    pub fn allocate(size: usize) -> Result<Self, safa_api::errors::ErrorStatus> {
        let pages = size.div_ceil(4096);
        let flags = ShmFlags::from_bits_retaining(0);
        let (key, shm_ri) = syscalls::mem::shm_create(pages, flags).expect("Failed to open ShmKey");

        let (mem_ri, buf) = syscalls::mem::map(
            core::ptr::null(),
            pages,
            0,
            Some(shm_ri),
            None,
            MemMapFlags::WRITE | MemMapFlags::DISABLE_EXEC,
        )?;

        Ok(Self {
            key,
            mem_map_resource: mem_ri,
            shm_resource: shm_ri,
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
}

impl Drop for SharedObject {
    fn drop(&mut self) {
        syscalls::resources::destroy_resource(self.mem_map_resource)
            .expect("Failed to destroy memory mapping");
        syscalls::resources::destroy_resource(self.shm_resource)
            .expect("Failed to destroy SHM Descriptor resource");
    }
}
