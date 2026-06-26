use std::sync::atomic::{AtomicBool, Ordering};

/// RAII guard that clears an [`AtomicBool`] flag to `false` on drop.
///
/// Callers set the flag to `true` before constructing the guard (typically
/// via a `compare_exchange`); the guard resets it on every exit path,
/// including panics, so a panicked holder can never leave the flag wedged.
///
/// **Panic-strategy caveat:** the clear-on-panic guarantee relies on
/// destructors running while the stack unwinds, so it holds under
/// `panic = "unwind"` (the default). Under `panic = "abort"` — e.g. the
/// iOS release profiles — a panic aborts the process immediately and no
/// `Drop` runs; there is simply no "after" left for the flag to gate.
/// When the binary is built with `panic = "abort"`, constructing a
/// [`ThreadRegistry`](crate::ThreadRegistry) emits a one-shot
/// `tracing::warn!` so operators can audit the risk.
#[must_use = "AtomicFlagGuard clears the flag on drop; binding to `_` or using as a statement drops it immediately"]
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::{catch_unwind, AssertUnwindSafe};

    /// A guard constructed over a `true` flag holds it while in scope and
    /// clears it to `false` on a normal scope exit.
    #[test]
    fn clears_flag_on_normal_drop() {
        let flag = AtomicBool::new(true);
        {
            let _guard = AtomicFlagGuard::new(&flag);
            assert!(flag.load(Ordering::Acquire), "flag stays set while held");
        }
        assert!(!flag.load(Ordering::Acquire), "flag cleared on drop");
    }

    /// The clear also runs while unwinding a panic — the load-bearing
    /// property the sync coordinators lean on so a panicked pass can't
    /// leave `is_syncing` latched and wedge `quiesce()`'s drain.
    #[test]
    fn clears_flag_while_unwinding_panic() {
        let flag = AtomicBool::new(true);
        let result = catch_unwind(AssertUnwindSafe(|| {
            let _guard = AtomicFlagGuard::new(&flag);
            panic!("boom while holding the guard");
        }));
        assert!(result.is_err(), "the panic propagated out of catch_unwind");
        assert!(
            !flag.load(Ordering::Acquire),
            "Drop ran during unwinding and cleared the flag"
        );
    }
}
