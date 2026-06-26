use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

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

/// RAII guard that refcounts a "raised" flag held in an [`AtomicUsize`].
/// Construction increments; Drop decrements. The flag is "raised" while
/// the count is > 0. Composes safely: multiple holders may raise the gate
/// independently, and Drop never lowers it past another holder's contribution.
///
/// Where [`AtomicFlagGuard`] is correct only when one party owns the flag,
/// this guard is the analog of the registry's `ClearingGuard` refcount,
/// for cases where two coordinated teardown paths (a public `quiesce()`
/// and an inner-flow `hold_quiescing_gate`) must compose without one
/// path's Drop lowering the other path's barrier.
#[must_use = "RefcountedFlagGuard decrements the count on drop; binding to `_` or using as a statement drops it immediately"]
pub struct RefcountedFlagGuard<'a>(&'a AtomicUsize);

impl<'a> RefcountedFlagGuard<'a> {
    /// Increment the refcount; the flag is observed "raised" while > 0.
    pub fn raise(counter: &'a AtomicUsize) -> Self {
        // SeqCst: composes into the same handshake `begin_pass` reads the
        // gate under; see the wallet-side `quiescing` doc.
        counter.fetch_add(1, Ordering::SeqCst);
        Self(counter)
    }
}

impl Drop for RefcountedFlagGuard<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
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

    /// Two holders compose: raising twice yields count 2; dropping one
    /// leaves the gate raised at 1 (still observed > 0); dropping the
    /// second returns to 0. Mirrors the production composition where a
    /// public `quiesce()` and an inner-flow `hold_quiescing_gate` both
    /// raise the same gate independently.
    #[test]
    fn composes_holders() {
        let counter = AtomicUsize::new(0);
        let g1 = RefcountedFlagGuard::raise(&counter);
        assert_eq!(counter.load(Ordering::Acquire), 1);
        let g2 = RefcountedFlagGuard::raise(&counter);
        assert_eq!(counter.load(Ordering::Acquire), 2);
        drop(g1);
        assert_eq!(
            counter.load(Ordering::Acquire),
            1,
            "dropping one holder must not lower the gate past the surviving holder's contribution"
        );
        drop(g2);
        assert_eq!(counter.load(Ordering::Acquire), 0);
    }

    /// The decrement also runs while unwinding a panic, so a panicked
    /// holder cannot leave the refcount permanently inflated.
    #[test]
    fn decrements_while_unwinding_panic() {
        let counter = AtomicUsize::new(0);
        let _outer = RefcountedFlagGuard::raise(&counter);
        assert_eq!(counter.load(Ordering::Acquire), 1);
        let result = catch_unwind(AssertUnwindSafe(|| {
            let _guard = RefcountedFlagGuard::raise(&counter);
            assert_eq!(counter.load(Ordering::Acquire), 2);
            panic!("boom while holding the refcount");
        }));
        assert!(result.is_err(), "the panic propagated out of catch_unwind");
        assert_eq!(
            counter.load(Ordering::Acquire),
            1,
            "Drop ran during unwinding and decremented the refcount"
        );
    }
}
