//! What this process knows of the flushes of each wallet's records.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use crate::changeset::traits::{PersistenceError, PlatformWalletPersistence};
use crate::wallet::platform_wallet::WalletId;

/// What this process knows of the flushes of one wallet's records.
///
/// A backend buffers a wallet's records under its id, whoever stored them,
/// and a flush takes that buffer, whoever flushed: one that fails for good
/// drops what it took, and one that fails transiently puts it back. So what
/// a flush did to the buffer is known only where every flush of the id is
/// counted: one ledger per wallet id, shared by the manager and by every
/// handle of a wallet under that id, a removed wallet's handles and its
/// same-id successor's alike ([`FlushLedgers`]). Every flush of the wallet's
/// records goes through [`Self::flush`].
#[derive(Debug, Default)]
pub(crate) struct FlushLedger {
    /// How many flushes are in progress: each is counted from before it calls
    /// the backend until its outcome is recorded in `failed` and `lost`, so
    /// while none is, they tell of every flush that failed. One whose backend
    /// panicked is recorded too, as failed for good, before it stops being
    /// counted.
    in_flight: AtomicUsize,
    /// How many flushes failed, for good or not: none wrote what it had taken
    /// from the buffer, which the backend dropped or put back.
    failed: AtomicU64,
    /// How many of them failed for good, the backend dropping what it
    /// buffered.
    lost: AtomicU64,
}

/// One flush in progress, counted in its [`FlushLedger`] until dropped.
///
/// Dropped before its outcome was recorded, its backend panicked: what that
/// did with what it took from the buffer is not known, so it is recorded as
/// failed for good before it stops being counted. A caller that catches the
/// panic and carries on then finds the loss, as it would any other.
struct InFlight<'a> {
    ledger: &'a FlushLedger,
    recorded: bool,
}

impl<'a> InFlight<'a> {
    fn enter(ledger: &'a FlushLedger) -> Self {
        ledger.in_flight.fetch_add(1, Ordering::SeqCst);
        Self {
            ledger,
            recorded: false,
        }
    }

    /// Record the backend's answer: a failure is counted, and counted as
    /// lost too when it is for good.
    fn record(&mut self, flushed: &Result<(), PersistenceError>) {
        if let Err(error) = flushed {
            self.ledger.record_failure(!error.is_transient());
        }
        self.recorded = true;
    }
}

impl Drop for InFlight<'_> {
    fn drop(&mut self) {
        if !self.recorded {
            self.ledger.record_failure(true);
        }
        self.ledger.in_flight.fetch_sub(1, Ordering::SeqCst);
    }
}

impl FlushLedger {
    /// Flush `wallet_id`'s records on `backend`: in progress from before the
    /// backend is called until its outcome is recorded, counted as failed
    /// when it fails, and as lost too when it fails for good (anything but a
    /// transient failure, which keeps the buffer) or the backend panics.
    pub(crate) fn flush(
        &self,
        backend: &dyn PlatformWalletPersistence,
        wallet_id: WalletId,
    ) -> Result<(), PersistenceError> {
        let mut in_flight = InFlight::enter(self);
        let flushed = backend.flush(wallet_id);
        in_flight.record(&flushed);
        flushed
    }

    /// Count one flush that failed, `for_good` when the backend dropped what
    /// it had taken.
    fn record_failure(&self, for_good: bool) {
        if for_good {
            self.lost.fetch_add(1, Ordering::SeqCst);
        }
        self.failed.fetch_add(1, Ordering::SeqCst);
    }

    /// How many flushes are in progress.
    pub(crate) fn in_flight(&self) -> usize {
        self.in_flight.load(Ordering::SeqCst)
    }

    /// How many flushes failed so far, for good or not.
    pub(crate) fn failed(&self) -> u64 {
        self.failed.load(Ordering::SeqCst)
    }

    /// How many flushes failed for good so far.
    pub(crate) fn lost(&self) -> u64 {
        self.lost.load(Ordering::SeqCst)
    }
}

/// The [`FlushLedger`] of each wallet id, for the wallets of one manager. An
/// entry outlives its wallet, as the backend's buffer under the id does.
#[derive(Debug, Default)]
pub(crate) struct FlushLedgers(Mutex<BTreeMap<WalletId, Arc<FlushLedger>>>);

impl FlushLedgers {
    /// The ledger of `wallet_id`'s records, made on first use.
    pub(crate) fn of(&self, wallet_id: WalletId) -> Arc<FlushLedger> {
        Arc::clone(
            self.0
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .entry(wallet_id)
                .or_default(),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::panic::{catch_unwind, AssertUnwindSafe};

    use super::*;
    use crate::changeset::{ClientStartState, PlatformWalletChangeSet};

    /// A backend whose flush panics.
    struct Panicking;

    impl PlatformWalletPersistence for Panicking {
        fn store(
            &self,
            _wallet_id: WalletId,
            _changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            Ok(())
        }
        fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
            panic!("the backend panics in its flush")
        }
        fn load(&self) -> Result<ClientStartState, PersistenceError> {
            Ok(ClientStartState::default())
        }
    }

    /// A flush whose backend panics is recorded as failed for good before it
    /// stops being counted as in progress: what it did with what it took from
    /// the buffer is not known, so a caller that catches the panic and
    /// carries on finds the loss rather than a ledger showing nothing.
    #[test]
    fn should_count_a_flush_that_panics_as_lost() {
        let ledger = FlushLedger::default();

        let flushed = catch_unwind(AssertUnwindSafe(|| ledger.flush(&Panicking, [0; 32])));

        assert!(flushed.is_err(), "the panic reaches the caller");
        assert_eq!(ledger.in_flight(), 0, "no longer in progress");
        assert_eq!(ledger.lost(), 1, "counted as failed for good");
        assert_eq!(ledger.failed(), 1, "counted as failed");
    }
}
