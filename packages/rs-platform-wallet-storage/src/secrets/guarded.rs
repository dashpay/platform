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
//!
//! # Verification tools this module forecloses
//!
//! `memsec` takes its pages from the Rust global allocator and then
//! `mprotect`s them in place, so the `secrets` tree cannot be run under
//! Miri (which cannot execute the `mprotect`/`mlock` FFI) nor under
//! ASan/LSan/libFuzzer (segfaults, memsec issue
//! [#14](https://github.com/quininer/memsec/issues/14)). It also assumes
//! the system allocator: a downstream binary installing a
//! `#[global_allocator]` may hand memsec pages whose allocator metadata
//! sits inside the block it protects. Accepted cost of the dependency;
//! the `unsafe` below is small enough to hold by inspection, and any
//! future sanitizer job must keep `secrets` off.

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
    /// A failed page lock is warned about but not fatal: the buffer is
    /// still guard-paged and wiped, it may merely be swappable.
    ///
    /// # Panics
    ///
    /// Panics if `cap` is `0` — memsec would place the payload pointer on
    /// the first byte of the trailing `PROT_NONE` guard page, so handing
    /// that address to anything that writes takes an immediate `SIGSEGV`.
    /// Empty secrets hold no allocation at all; see `SecretString` and
    /// `SecretBytes`.
    ///
    /// Panics (via [`std::alloc::handle_alloc_error`]) if guarded memory
    /// is exhausted. The secret constructors are infallible by contract,
    /// so this is handled the way the global allocator handles ordinary
    /// exhaustion.
    pub(super) fn new(cap: usize) -> Self {
        assert!(cap > 0, "a guarded buffer must hold at least one byte");
        // SAFETY: `malloc_sized` takes a plain byte count and returns a
        // pointer to that many writable bytes, or `None` on failure.
        #[allow(unsafe_code)]
        let ptr = unsafe { memsec::malloc_sized(cap) }.unwrap_or_else(|| alloc_failed(cap));
        let mut buf = Self {
            ptr: ptr.cast(),
            cap,
        };
        // `malloc_sized` locks the region but discards the result, so a
        // failed lock would otherwise be indistinguishable from a
        // successful one. Re-locking is a no-op when it already took.
        if !lock_payload(buf.ptr, cap) {
            tracing::warn!(
                "secret pages could not be locked into RAM and may reach swap; \
                 raise RLIMIT_MEMLOCK for this process"
            );
        }
        // memsec hands back a garbage-filled block, so zero it here: every
        // byte past a secret's length is then guaranteed zero from the
        // first write onwards.
        buf.zeroize_all();
        #[cfg(test)]
        gauge::allocated(locked_cost(cap));
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
    ///
    /// # Panics
    ///
    /// Panics if `len` exceeds [`capacity`](Self::capacity) — a caller
    /// tracking a length its buffer cannot hold is a bug in this module,
    /// and the bound is what keeps the slice construction sound, so it
    /// must survive into release builds.
    pub(super) fn as_slice(&self, len: usize) -> &[u8] {
        assert!(len <= self.cap, "secret length exceeds guarded capacity");
        // SAFETY: the allocation is valid and uniquely owned for `cap`
        // bytes, all of which `new` initialised, and `len <= cap`.
        #[allow(unsafe_code)]
        unsafe {
            std::slice::from_raw_parts(self.ptr.as_ptr(), len)
        }
    }

    /// The first `len` bytes, mutably.
    ///
    /// # Panics
    ///
    /// As [`as_slice`](Self::as_slice).
    pub(super) fn as_mut_slice(&mut self, len: usize) -> &mut [u8] {
        assert!(len <= self.cap, "secret length exceeds guarded capacity");
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
        #[cfg(test)]
        gauge::freed(locked_cost(self.cap));
        // SAFETY: `ptr` came from `memsec::malloc_sized`, is owned solely
        // by this value, and is freed exactly once — here.
        #[allow(unsafe_code)]
        unsafe {
            memsec::free(self.ptr)
        };
    }
}

/// Bytes `memsec` locks for a `payload`-byte secret: its 16-byte canary
/// prepended, then rounded up to whole pages.
///
/// The unit the crate's locked-memory budget is denominated in — see the
/// table at [`MAX_SECRET_LEN`](crate::secrets::MAX_SECRET_LEN).
pub(super) const fn locked_cost(payload: usize) -> usize {
    const PAGE: usize = 4096;
    (16 + payload).div_ceil(PAGE) * PAGE
}

/// Per-thread high-water mark of locked bytes, so a test can assert what
/// a flow actually costs instead of what its doc comment claims.
///
/// Thread-local, not global: every allocation in a store operation
/// happens on the calling thread, so this measures the flow under test
/// without the other tests in this binary perturbing it.
#[cfg(test)]
pub(super) mod gauge {
    use std::cell::Cell;

    thread_local! {
        static LIVE: Cell<usize> = const { Cell::new(0) };
        static PEAK: Cell<usize> = const { Cell::new(0) };
    }

    pub(super) fn allocated(locked: usize) {
        let live = LIVE.with(|l| {
            let next = l.get() + locked;
            l.set(next);
            next
        });
        PEAK.with(|p| p.set(p.get().max(live)));
    }

    pub(super) fn freed(locked: usize) {
        LIVE.with(|l| l.set(l.get().saturating_sub(locked)));
    }

    /// Drop the high-water mark to what is live right now, so the next
    /// [`peak`] reports only what the code under test adds on top.
    pub(crate) fn reset_peak() {
        LIVE.with(|l| PEAK.with(|p| p.set(l.get())));
    }

    /// The greatest number of locked bytes live on this thread since
    /// [`reset_peak`], resident buffers included.
    pub(crate) fn peak() -> usize {
        PEAK.with(Cell::get)
    }
}

/// Lock the `cap` payload bytes at `ptr` into RAM, reporting whether the
/// kernel accepted it.
///
/// The payload sits at the tail of the region memsec already locked, so
/// this re-lock covers a subset of it and is idempotent. Callers get a
/// `bool` rather than a log line because the failure is worth reporting
/// exactly once, at the allocation that suffered it.
fn lock_payload(ptr: NonNull<u8>, cap: usize) -> bool {
    // SAFETY: `mlock` only passes the address to the kernel, which
    // validates it; nothing here dereferences `ptr`.
    #[allow(unsafe_code)]
    unsafe {
        memsec::mlock(ptr.as_ptr(), cap)
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

    /// The allocation-failure path really is reachable and really does
    /// panic rather than returning a bogus buffer.
    ///
    /// `usize::MAX` trips memsec's own `size >= usize::MAX - PAGE_SIZE *
    /// 4` guard, so `malloc_sized` returns `None` without attempting a
    /// mapping — a deterministic drive of `alloc_failed`, no memory
    /// pressure required. Its other arm, [`std::alloc::handle_alloc_error`],
    /// aborts the process and so cannot be exercised in-process.
    #[test]
    #[should_panic(expected = "exceeds the maximum allocation size")]
    fn allocation_failure_panics() {
        let _ = GuardedBuf::new(usize::MAX);
    }

    /// A zero-length request is refused instead of handing back a pointer
    /// to the first byte of the trailing `PROT_NONE` guard page.
    #[test]
    #[should_panic(expected = "at least one byte")]
    fn zero_length_allocation_is_refused() {
        let _ = GuardedBuf::new(0);
    }

    /// A length past capacity panics instead of silently truncating —
    /// in release builds too, which a `debug_assert` would not cover.
    #[test]
    #[should_panic(expected = "exceeds guarded capacity")]
    fn slice_past_capacity_panics() {
        let buf = GuardedBuf::new(16);
        let _ = buf.as_slice(17);
    }

    /// mlock refusal is detectable, so [`GuardedBuf::new`]'s warning
    /// branch is reachable — and a buffer whose lock failed is still
    /// usable, because the design is fail-open.
    ///
    /// Driven by probing a region the kernel must reject rather than by
    /// exhausting `RLIMIT_MEMLOCK`, which is process-wide and would race
    /// every other test in this binary. The second page is page-aligned
    /// and below the default `mmap_min_addr`, so nothing can ever be
    /// mapped there and `mlock` cannot start succeeding on it.
    #[test]
    fn mlock_refusal_is_detected_and_not_fatal() {
        let page = region::page::size();
        let unmapped = NonNull::new(std::ptr::without_provenance_mut::<u8>(page))
            .expect("a non-zero address is non-null");
        assert!(
            !lock_payload(unmapped, page),
            "test needs an address mlock refuses; {page:#x} was accepted"
        );

        // Fail-open: a real allocation still yields a working buffer.
        let mut buf = GuardedBuf::new(64);
        buf.as_mut_slice(64).copy_from_slice(&[0x3Cu8; 64]);
        assert_eq!(buf.as_slice(64), &[0x3Cu8; 64]);
    }
}
