//! Generation-guarded cancel-token slot shared by the background sync
//! managers ([`DashPaySyncManager`](super::dashpay_sync::DashPaySyncManager),
//! [`IdentitySyncManager`](super::identity_sync::IdentitySyncManager),
//! [`PlatformAddressSyncManager`](super::platform_address_sync::PlatformAddressSyncManager),
//! and the shielded coordinator).
//!
//! Every manager runs its loop on a dedicated OS thread whose exit is
//! asynchronous with respect to `stop()`/`start()` calls, so they all
//! share the same shutdown hazard — and must all share the same guard.
//! Keeping the invariant in one type (instead of a per-manager copy)
//! means a fix or audit here covers every loop at once.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex as StdMutex;

use tokio_util::sync::CancellationToken;

/// Cancel-token slot for a background sync loop, guarded by a
/// monotonically increasing **loop generation** so a stale, draining
/// loop can never clobber a newer loop's token.
///
/// Without the guard a `stop()` + quick `start()` is a use-after-free
/// hazard: `stop()` takes + cancels loop A's token and `start()`
/// installs loop B's token, but loop A keeps draining its in-flight
/// pass. When loop A finally exits, an unconditional `slot = None`
/// would null **loop B's** live token, leaving loop B uncancellable —
/// a later shutdown `stop()`/`quiesce()` silently no-ops while loop B
/// keeps calling `persister.store(...)` (or firing host callbacks)
/// through a freed FFI context.
pub(crate) struct LoopCancelGuard {
    /// The active loop's cancel token, if one is running.
    slot: StdMutex<Option<CancellationToken>>,
    /// Bumped on every [`install`](Self::install). The background loop
    /// captures its generation at install time and clears the slot on
    /// exit **only if its generation is still current** (see
    /// [`clear_if_current`](Self::clear_if_current)).
    generation: AtomicU64,
}

impl LoopCancelGuard {
    pub fn new() -> Self {
        Self {
            slot: StdMutex::new(None),
            generation: AtomicU64::new(0),
        }
    }

    /// Install a fresh cancel token for a new background loop, returning
    /// the token (for the loop to watch) and its **generation** (for the
    /// loop to pass to [`clear_if_current`](Self::clear_if_current) on
    /// exit). Returns `None` if a loop is already running — preserving
    /// the managers' `start()` idempotency.
    ///
    /// The generation bump happens under the same lock that stores the
    /// token, so a draining older loop reading the generation under that
    /// lock always observes whether a newer loop has since replaced it.
    pub fn install(&self) -> Option<(CancellationToken, u64)> {
        let mut guard = self.slot.lock().expect("bg_cancel poisoned");
        if guard.is_some() {
            return None;
        }
        let cancel = CancellationToken::new();
        *guard = Some(cancel.clone());
        let generation = self
            .generation
            .fetch_add(1, Ordering::AcqRel)
            .wrapping_add(1);
        Some((cancel, generation))
    }

    /// Clear the stored cancel token **only if it still belongs to the
    /// loop identified by `my_generation`** — i.e. no later `install`
    /// has handed out a replacement. Called by the loop on exit.
    pub fn clear_if_current(&self, my_generation: u64) {
        let mut guard = self.slot.lock().expect("bg_cancel poisoned");
        if self.generation.load(Ordering::Acquire) == my_generation {
            *guard = None;
        }
    }

    /// Take the active loop's token out of the slot, if any. The
    /// managers' cancel-only `stop()` is `take()` + `token.cancel()`.
    pub fn take(&self) -> Option<CancellationToken> {
        self.slot.lock().expect("bg_cancel poisoned").take()
    }

    /// Whether a loop's token is currently installed.
    pub fn is_running(&self) -> bool {
        self.slot.lock().map(|g| g.is_some()).unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression: a stale, draining loop's cleanup must **not** clobber
    /// a newer loop's cancel token.
    ///
    /// We drive the token lifecycle directly (`install` /
    /// `clear_if_current`) rather than spawning a real loop: the loops
    /// run on OS threads under `Handle::block_on`, so their exit timing
    /// can't be pinned deterministically. With the generation guard
    /// removed (`*guard = None` unconditional) this test fails on the
    /// final assertions; with the guard it passes.
    #[test]
    fn stale_loop_cleanup_does_not_clobber_newer_loop_token() {
        let slot = LoopCancelGuard::new();

        // Loop A starts: installs token_A at generation G_A.
        let (token_a, gen_a) = slot.install().expect("first install starts a loop");
        assert!(slot.is_running());

        // Shutdown of loop A: stop() cancels + takes token_A immediately
        // (cancel-only), but loop A is still "draining" — its cleanup
        // has not run yet.
        slot.take().expect("token_A installed").cancel();
        assert!(token_a.is_cancelled());
        assert!(!slot.is_running(), "take() clears the slot immediately");

        // Loop B starts BEFORE loop A's cleanup runs: installs token_B
        // at a newer generation G_B.
        let (token_b, _gen_b) = slot.install().expect("second install starts a new loop");
        assert!(slot.is_running());

        // Loop A FINALLY drains and runs its cleanup with its own (now
        // stale) generation. The guard must make this a no-op; an
        // unconditional clear would null loop B's token here.
        slot.clear_if_current(gen_a);

        // Loop B's token must still be installed and uncancelled.
        assert!(
            slot.is_running(),
            "stale loop A cleanup must not clobber loop B's live token"
        );
        assert!(!token_b.is_cancelled());

        // …and a real shutdown can still cancel loop B.
        slot.take().expect("token_B still installed").cancel();
        assert!(
            token_b.is_cancelled(),
            "loop B must remain cancellable after the stale cleanup"
        );
        assert!(!slot.is_running());
    }

    /// `install` while a loop is running returns `None` (start
    /// idempotency), and a loop that exits *without* being replaced
    /// clears its own slot so a later install succeeds.
    #[test]
    fn install_is_exclusive_and_clear_reopens_slot() {
        let slot = LoopCancelGuard::new();

        let (_token, generation) = slot.install().expect("fresh slot installs");
        assert!(slot.install().is_none(), "second install must be refused");

        slot.clear_if_current(generation);
        assert!(!slot.is_running());
        assert!(
            slot.install().is_some(),
            "slot must be reusable after the loop clears it"
        );
    }
}
