//! The crate's only `unsafe` code: a guarded heap buffer for secret bytes.
//!
//! [`GuardedBuf`] wraps [`memsec`]'s hardened allocator. Every allocation
//! is page-aligned, fenced by inaccessible guard pages, canary-checked,
//! `mlock`ed, and excluded from core dumps (`MADV_DONTDUMP`). Because the
//! data pages belong to one buffer outright, two live secrets can never
//! share a page — so freeing one can never unlock memory another still
//! holds, the failure mode that makes page-granular locking hazardous
//! over ordinary allocations.
//!
//! # Why the `unsafe_code` carve-out lives here
//!
//! `memsec`'s allocator is an `unsafe` FFI-shaped API, so it cannot be
//! called under the crate-level `deny(unsafe_code)`. The carve-out is
//! confined to this module and applied **per item**, never as a
//! module-wide `allow`: a new `unsafe` block added anywhere in this file
//! still trips the crate lint and has to justify itself. Callers see a
//! safe, slice-shaped API and never handle a raw pointer.

use std::alloc::Layout;
use std::ptr::NonNull;

use zeroize::Zeroize;

/// A page-aligned, guard-paged, `mlock`ed byte buffer.
///
/// Owns its allocation exclusively for its whole lifetime and wipes it
/// on drop. `cap` is the usable payload length; `memsec` places it so the
/// payload ends flush against the trailing guard page.
pub(super) struct GuardedBuf {
    ptr: NonNull<u8>,
    cap: usize,
}

// SAFETY: `GuardedBuf` uniquely owns its allocation and offers no interior
// mutability, so it is exactly as safe to send and share as `Box<[u8]>`.
// Holding the raw pointer behind this type keeps `SecretString` and
// `SecretBytes` free of manual unsafe trait impls.
#[allow(unsafe_code)]
unsafe impl Send for GuardedBuf {}
#[allow(unsafe_code)]
unsafe impl Sync for GuardedBuf {}

impl GuardedBuf {
    /// Allocate `cap` zeroed bytes of guarded memory.
    ///
    /// # Panics
    ///
    /// Panics (via [`std::alloc::handle_alloc_error`]) if guarded memory
    /// is exhausted. The secret constructors are infallible by contract,
    /// so this is handled the way the global allocator handles ordinary
    /// exhaustion.
    pub(super) fn new(cap: usize) -> Self {
        // SAFETY: `malloc_sized` takes a plain byte count and returns a
        // pointer to that many writable bytes, or `None` on failure.
        #[allow(unsafe_code)]
        let ptr = unsafe { memsec::malloc_sized(cap) }.unwrap_or_else(|| alloc_failed(cap));
        let mut buf = Self {
            ptr: ptr.cast(),
            cap,
        };
        // memsec hands back a garbage-filled block, so zero it here: every
        // byte past a secret's length is then guaranteed zero from the
        // first write onwards.
        buf.zeroize_all();
        buf
    }

    /// The usable payload length. Only the tests need it: production
    /// code tracks a secret's live length separately and wipes through
    /// [`GuardedBuf::zeroize_all`].
    #[cfg(test)]
    pub(super) fn capacity(&self) -> usize {
        self.cap
    }

    /// The first `len` bytes. Callers guarantee they are initialised,
    /// which holds for the whole buffer from [`GuardedBuf::new`] onwards.
    pub(super) fn as_slice(&self, len: usize) -> &[u8] {
        debug_assert!(len <= self.cap);
        // SAFETY: the allocation is valid and uniquely owned for `cap`
        // bytes, all of which `new` initialised, and `len <= cap`.
        #[allow(unsafe_code)]
        unsafe {
            std::slice::from_raw_parts(self.ptr.as_ptr(), len.min(self.cap))
        }
    }

    /// The first `len` bytes, mutably.
    pub(super) fn as_mut_slice(&mut self, len: usize) -> &mut [u8] {
        debug_assert!(len <= self.cap);
        let len = len.min(self.cap);
        // SAFETY: as `as_slice`, and `&mut self` guarantees exclusivity.
        #[allow(unsafe_code)]
        unsafe {
            std::slice::from_raw_parts_mut(self.ptr.as_ptr(), len)
        }
    }

    /// The buffer's start address, for the page-isolation tests to
    /// compare against page boundaries. Never dereferenced by callers,
    /// and absent outside tests so no production path can hold one.
    #[cfg(test)]
    pub(super) fn addr(&self) -> usize {
        self.ptr.as_ptr() as usize
    }

    /// Volatile-zero every byte, including capacity past any live length.
    pub(super) fn zeroize_all(&mut self) {
        let cap = self.cap;
        self.as_mut_slice(cap).zeroize();
    }
}

impl Drop for GuardedBuf {
    fn drop(&mut self) {
        // `memsec::free` wipes the region too, but the wipe is this
        // module's guarantee rather than a dependency's implementation
        // detail, so it is not left to `free` alone.
        self.zeroize_all();
        // SAFETY: `ptr` came from `memsec::malloc_sized`, is owned solely
        // by this value, and is freed exactly once — here.
        #[allow(unsafe_code)]
        unsafe {
            memsec::free(self.ptr)
        };
    }
}

/// Report unrecoverable exhaustion of guarded memory.
fn alloc_failed(cap: usize) -> ! {
    match Layout::from_size_align(cap, 1) {
        Ok(layout) => std::alloc::handle_alloc_error(layout),
        Err(_) => panic!("secret capacity {cap} exceeds the maximum allocation size"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_is_zeroed_and_sized() {
        let buf = GuardedBuf::new(64);
        assert_eq!(buf.capacity(), 64);
        assert!(
            buf.as_slice(64).iter().all(|&b| b == 0),
            "memsec fills with garbage; `new` must zero it"
        );
    }

    /// The payload ends flush against the trailing guard page, which is
    /// what makes page sharing between two secrets impossible.
    #[test]
    fn allocation_ends_on_a_page_boundary() {
        let page = region::page::size();
        for cap in [1usize, 64, 4096 - 16, 8192] {
            let buf = GuardedBuf::new(cap);
            assert_eq!(
                (buf.addr() + cap) % page,
                0,
                "a {cap}-byte buffer must abut the guard page"
            );
        }
    }

    #[test]
    fn writes_are_readable_and_wipe_clears_them() {
        let mut buf = GuardedBuf::new(32);
        buf.as_mut_slice(32).copy_from_slice(&[0xA5u8; 32]);
        assert_eq!(buf.as_slice(32), &[0xA5u8; 32]);
        buf.zeroize_all();
        assert!(buf.as_slice(32).iter().all(|&b| b == 0));
    }
}
