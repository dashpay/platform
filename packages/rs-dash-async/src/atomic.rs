use std::sync::atomic::{AtomicBool, Ordering};

/// RAII guard that clears an [`AtomicBool`] flag to `false` on drop.
///
/// Callers set the flag to `true` before constructing the guard (typically
/// via a `compare_exchange`); the guard resets it on every exit path,
/// including panics, so a panicked holder can never leave the flag wedged.
pub struct AtomicFlagGuard<'a>(&'a AtomicBool);

impl<'a> AtomicFlagGuard<'a> {
    /// Wrap `flag`. Does **not** set it to `true` — the caller is
    /// responsible for doing that before constructing the guard.
    pub fn new(flag: &'a AtomicBool) -> Self {
        Self(flag)
    }
}

impl Drop for AtomicFlagGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}
