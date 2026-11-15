//! What this process knows of the flushes of each wallet's records.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError, Weak};

use crate::changeset::traits::{PersistenceError, PlatformWalletPersistence};
use crate::wallet::platform_wallet::WalletId;

/// What this process knows of the flushes of one wallet's records.
///
/// A backend buffers a wallet's records under its id, whoever stored them,
/// and a flush takes that buffer, whoever flushed: one that fails for good
/// drops what it took, and one that fails transiently puts it back. So what
/// a flush did to the buffer is known only where every flush of the id is
/// counted: one ledger per backend instance and wallet id, shared by every
/// manager made over the backend and by every handle of a wallet under that
/// id, a removed wallet's handles and its same-id successor's alike
/// ([`FlushLedgers::of_backend`]). Every flush of the wallet's records goes
/// through [`Self::flush`].
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
///
/// A backend whose store is the commit buffers nothing, so no flush of it
/// drops anything, whatever the flush answers: it is counted as failed and
/// never as lost.
struct InFlight<'a> {
    ledger: &'a FlushLedger,
    /// Whether the backend buffers what is stored until a flush.
    buffers: bool,
    recorded: bool,
}

impl<'a> InFlight<'a> {
    fn enter(ledger: &'a FlushLedger, buffers: bool) -> Self {
        ledger.in_flight.fetch_add(1, Ordering::SeqCst);
        Self {
            ledger,
            buffers,
            recorded: false,
        }
    }

    /// Record the backend's answer: a failure is counted, and counted as
    /// lost too when it is for good and the backend buffers.
    fn record(&mut self, flushed: &Result<(), PersistenceError>) {
        if let Err(error) = flushed {
            self.ledger
                .record_failure(self.buffers && !error.is_transient());
        }
        self.recorded = true;
    }
}

impl Drop for InFlight<'_> {
    fn drop(&mut self) {
        if !self.recorded {
            self.ledger.record_failure(self.buffers);
        }
        self.ledger.in_flight.fetch_sub(1, Ordering::SeqCst);
    }
}

impl FlushLedger {
    /// Flush `wallet_id`'s records on `backend`: in progress from before the
    /// backend is called until its outcome is recorded, counted as failed
    /// when it fails, and as lost too when it fails for good (anything but a
    /// transient failure, which keeps the buffer) or the backend panics.
    ///
    /// Never as lost on a backend whose store is the commit
    /// ([`PlatformWalletPersistence::store_commits_inline`]): a flush there
    /// only notifies the host, and what was stored is already on disk
    /// whatever the notification answers.
    pub(crate) fn flush(
        &self,
        backend: &dyn PlatformWalletPersistence,
        wallet_id: WalletId,
    ) -> Result<(), PersistenceError> {
        let mut in_flight = InFlight::enter(self, !backend.store_commits_inline());
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

/// The [`FlushLedger`] of each wallet id, for the records one backend
/// instance holds. An entry outlives its wallet, as the backend's buffer
/// under the id does.
#[derive(Debug, Default)]
pub(crate) struct FlushLedgers(Mutex<BTreeMap<WalletId, Arc<FlushLedger>>>);

/// The ledgers of each backend instance that is still alive
/// ([`FlushLedgers::of_backend`]): kept for as long as anything holds the
/// backend, and dropped once nothing does. The backend is held weakly, which
/// keeps its allocation from being reused while its entry stands, so an
/// entry is never taken for another backend's.
static BACKEND_LEDGERS: Mutex<Vec<BackendLedgers>> = Mutex::new(Vec::new());

type BackendLedgers = (Weak<dyn PlatformWalletPersistence>, Arc<FlushLedgers>);

impl FlushLedgers {
    /// The ledgers of `backend`'s records: the same ones for every caller
    /// that passes this backend instance, for as long as the instance lives.
    ///
    /// A backend buffers a wallet's records under its id whoever stored
    /// them, so the count of its flushes belongs with the backend and lives
    /// as long as it does, not as long as one manager or one handle: whoever
    /// can still flush the backend holds it, and so finds the same ledgers
    /// here. Every handle of a wallet takes its ledger from here
    /// (`WalletPersister::new`), however it was made.
    pub(crate) fn of_backend(backend: &Arc<dyn PlatformWalletPersistence>) -> Arc<Self> {
        let mut known = BACKEND_LEDGERS
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        known.retain(|(held, _)| held.strong_count() > 0);
        let address = Arc::as_ptr(backend).cast::<()>();
        let shared = known
            .iter()
            .find(|(held, _)| held.as_ptr().cast::<()>() == address)
            .map(|(_, ledgers)| Arc::clone(ledgers));
        if let Some(ledgers) = shared {
            return ledgers;
        }
        let ledgers = Arc::new(Self::default());
        known.push((Arc::downgrade(backend), Arc::clone(&ledgers)));
        ledgers
    }

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

    /// A buffering backend whose flush fails for good, and one whose store is
    /// the commit, whose flush only notifies the host and fails the same way.
    struct Failing {
        inline: bool,
    }

    impl PlatformWalletPersistence for Failing {
        fn store_commits_inline(&self) -> bool {
            self.inline
        }
        fn store(
            &self,
            _wallet_id: WalletId,
            _changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            Ok(())
        }
        fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
            Err(PersistenceError::backend("the host refused the flush"))
        }
        fn load(&self) -> Result<ClientStartState, PersistenceError> {
            Ok(ClientStartState::default())
        }
    }

    /// A backend whose store is the commit buffers nothing, so a flush of it
    /// that fails dropped nothing: it is counted as failed, never as lost. A
    /// buffering backend's flush that fails the same way is counted as both.
    #[test]
    fn should_not_count_a_failed_flush_of_an_inline_backend_as_lost() {
        for inline in [true, false] {
            let ledger = FlushLedger::default();

            let flushed = ledger.flush(&Failing { inline }, [0; 32]);

            assert!(flushed.is_err_and(|error| !error.is_transient()));
            assert_eq!(ledger.failed(), 1, "inline: {inline}");
            assert_eq!(ledger.lost(), u64::from(!inline), "inline: {inline}");
        }
    }

    /// The ledgers of a backend instance live as long as the instance does,
    /// whoever asks for them and whatever was dropped in between: a handle
    /// that outlived its manager keeps only its own wallet id's ledger, and
    /// a handle made later over the same backend still counts in that one.
    /// Another backend instance has its own.
    #[test]
    fn should_keep_a_backends_ledgers_for_as_long_as_the_backend_lives() {
        let backend: Arc<dyn PlatformWalletPersistence> = Arc::new(Failing { inline: false });
        let wallet_id: WalletId = [5; 32];

        // All an earlier handle keeps: the ledger of its own wallet id.
        let earlier = FlushLedgers::of_backend(&backend).of(wallet_id);
        let later = FlushLedgers::of_backend(&backend).of(wallet_id);
        assert!(Arc::ptr_eq(&earlier, &later), "one ledger for the id");

        assert!(earlier.flush(backend.as_ref(), wallet_id).is_err());
        assert_eq!(
            later.lost(),
            1,
            "the later handle sees the earlier one's loss"
        );

        let other: Arc<dyn PlatformWalletPersistence> = Arc::new(Failing { inline: false });
        assert_eq!(
            FlushLedgers::of_backend(&other).of(wallet_id).lost(),
            0,
            "another backend instance counts apart"
        );
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
