//! What this process knows of the flushes of each wallet's records.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError, Weak};

use crate::changeset::changeset::PlatformWalletChangeSet;
use crate::changeset::traits::{PersistenceError, PlatformWalletPersistence, StoreAndCommitError};
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
/// through [`Self::flush`], and every store committed in one step through
/// [`Self::store_and_commit`]: that step drains the same buffer, so it is
/// counted as the flush it contains.
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

/// One flush in progress, or one store being committed in one step, counted
/// in its [`FlushLedger`] until dropped.
///
/// Dropped before its outcome was recorded, its backend panicked: what that
/// did with what it took from the buffer is not known, so it is recorded as
/// failed for good before it stops being counted. A caller that catches the
/// panic and carries on then finds the loss, as it would any other.
///
/// A backend whose store is the commit buffers nothing, so no flush of it
/// drops anything, whatever the flush answers: it is counted as failed and
/// never as lost. That is a statement about its `store` and its `flush`
/// only. A store committed in one step that fails for good dropped what the
/// backend had buffered, by that step's contract, whatever the backend's
/// `store` does: it is counted as lost on every backend
/// ([`FlushLedger::store_and_commit`]).
struct InFlight<'a> {
    ledger: &'a FlushLedger,
    /// Whether a failure for good, or a panic, dropped what the backend had
    /// buffered: for a flush, whether the backend buffers what is stored
    /// until a flush, and always for a store committed in one step.
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

    /// Record the backend's answer, `failure` being how its write of what
    /// it took from the buffer failed, if it did: a failure is counted, and
    /// counted as lost too when it is for good and the backend buffers.
    fn record(&mut self, failure: Option<&PersistenceError>) {
        if let Some(error) = failure {
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
        in_flight.record(flushed.as_ref().err());
        flushed
    }

    /// Stage `changeset` and commit `wallet_id`'s buffer on `backend` in one
    /// step ([`PlatformWalletPersistence::store_and_commit`]), counted as
    /// [`Self::flush`] counts a flush: in progress from before the backend
    /// is called until its outcome is recorded, as failed when the commit
    /// fails, and as lost too when it fails for good or the backend panics.
    ///
    /// As lost on every backend, one whose store is the commit
    /// ([`PlatformWalletPersistence::store_commits_inline`]) included. That
    /// attestation says what a `store` does, and nothing of what this step
    /// holds: a backend may keep a changeset whose commit failed
    /// transiently, from a `store` or from this step, and commit it with
    /// the next one. A step that then fails for good drops it too. Its
    /// contract says so (everything buffered for the wallet is dropped), and
    /// whether anything but the step's own changeset was in that buffer is
    /// not known here, so the loss is counted.
    ///
    /// The caller needs none of this for its own answer, which the backend
    /// gives. It is counted for every other reader of the ledger: a commit
    /// that fails for good drops the whole buffer, what other handles stored
    /// plainly included, and they learn of it here.
    ///
    /// A changeset refused before it was staged took nothing from the
    /// buffer, so it is not counted as a failure.
    pub(crate) fn store_and_commit(
        &self,
        backend: &dyn PlatformWalletPersistence,
        wallet_id: WalletId,
        changeset: PlatformWalletChangeSet,
    ) -> Result<(), StoreAndCommitError> {
        let mut in_flight = InFlight::enter(self, true);
        let committed = backend.store_and_commit(wallet_id, changeset);
        in_flight.record(match &committed {
            Err(StoreAndCommitError::NotCommitted(error)) => Some(error),
            Ok(()) | Err(StoreAndCommitError::NotStaged(_)) => None,
        });
        committed
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
///
/// A backend instance is told apart by the address of its `Arc` allocation.
/// Every `Arc` has one, a backend type of no size included, as the
/// allocation holds the reference counts: two live backends never share it.
///
/// An entry whose backend is gone is dropped at the next lookup, by any
/// caller. So what outlives a backend is bounded: the entries of the
/// backends that went since the last lookup, each a few counters per wallet
/// id, until the next wallet handle is made anywhere in the process. They do
/// not add up over repeated manager or backend lifetimes.
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
    use crate::changeset::{ClientStartState, PersistenceErrorKind};

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

    /// A buffering backend of no size whose flush fails for good. `N` makes
    /// distinct types of it.
    struct Unit<const N: usize>;

    impl<const N: usize> PlatformWalletPersistence for Unit<N> {
        fn store(
            &self,
            _wallet_id: WalletId,
            _changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            Ok(())
        }
        fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
            Err(PersistenceError::backend("the database is corrupt"))
        }
        fn load(&self) -> Result<ClientStartState, PersistenceError> {
            Ok(ClientStartState::default())
        }
    }

    /// Backend instances are told apart by their `Arc` allocation, which a
    /// backend type of no size has too: two live backends of such types, the
    /// same type or not, have their own ledgers, and a flush of one that
    /// fails is not counted against the others. The same instance, asked
    /// again, has the same ledger.
    #[test]
    fn should_tell_apart_live_backends_of_zero_sized_types() {
        assert_eq!(std::mem::size_of::<Unit<0>>(), 0);
        let first: Arc<dyn PlatformWalletPersistence> = Arc::new(Unit::<0>);
        let second: Arc<dyn PlatformWalletPersistence> = Arc::new(Unit::<0>);
        let third: Arc<dyn PlatformWalletPersistence> = Arc::new(Unit::<1>);
        let wallet_id: WalletId = [6; 32];
        let ledger_of = |backend| FlushLedgers::of_backend(backend).of(wallet_id);
        let (of_first, of_second, of_third) =
            (ledger_of(&first), ledger_of(&second), ledger_of(&third));

        assert!(of_first.flush(first.as_ref(), wallet_id).is_err());

        assert_eq!(of_first.lost(), 1);
        assert_eq!(of_second.lost(), 0, "another instance of the same type");
        assert_eq!(of_third.lost(), 0, "an instance of another type");
        assert!(
            Arc::ptr_eq(&ledger_of(&first), &of_first),
            "the same instance has the same ledger"
        );
    }

    /// A backend that commits a store in one step, and answers each call
    /// with the next of `answers`, or panics once they run out. It buffers
    /// unless `inline`. It notes how many flushes its ledger counted as in
    /// progress while the step ran.
    struct InOneStep {
        inline: bool,
        answers: Mutex<Vec<Result<(), StoreAndCommitError>>>,
        ledger: Arc<FlushLedger>,
        in_flight_during_the_step: AtomicUsize,
    }

    impl InOneStep {
        /// A backend that gives `answers` in order, with its ledger.
        fn answering(inline: bool, mut answers: Vec<Result<(), StoreAndCommitError>>) -> Self {
            answers.reverse();
            Self {
                inline,
                answers: Mutex::new(answers),
                ledger: Arc::default(),
                in_flight_during_the_step: AtomicUsize::new(0),
            }
        }

        fn step(&self) -> Result<(), StoreAndCommitError> {
            self.ledger
                .store_and_commit(self, [0; 32], PlatformWalletChangeSet::default())
        }
    }

    impl PlatformWalletPersistence for InOneStep {
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
            Ok(())
        }
        fn offers_store_and_commit(&self) -> bool {
            true
        }
        fn store_and_commit(
            &self,
            _wallet_id: WalletId,
            _changeset: PlatformWalletChangeSet,
        ) -> Result<(), StoreAndCommitError> {
            self.in_flight_during_the_step
                .store(self.ledger.in_flight(), Ordering::SeqCst);
            self.answers
                .lock()
                .expect("answers")
                .pop()
                .expect("the backend panics in its step")
        }
        fn load(&self) -> Result<ClientStartState, PersistenceError> {
            Ok(ClientStartState::default())
        }
    }

    fn busy() -> PersistenceError {
        PersistenceError::backend_with_kind(PersistenceErrorKind::Transient, "the disk is busy")
    }

    fn corrupt() -> PersistenceError {
        PersistenceError::backend("the database is corrupt")
    }

    /// A store committed in one step drains the buffer every flush drains,
    /// so its commit is counted as a flush is: failed when it fails, and lost
    /// too when it fails for good, as it dropped what other handles had
    /// stored with it. A changeset refused before it was staged took nothing
    /// from the buffer, and one that committed dropped nothing: neither is
    /// counted.
    ///
    /// A backend whose store is the commit is counted the same way. Its
    /// attestation speaks for its `store`, not for what the step held: a
    /// changeset whose commit failed transiently earlier may still have been
    /// buffered, and a step that fails for good dropped it.
    #[test]
    fn should_count_a_failed_commit_of_a_store_made_in_one_step_as_a_failed_flush() {
        let backend = InOneStep::answering(
            false,
            vec![
                Ok(()),
                Err(StoreAndCommitError::NotStaged(corrupt())),
                Err(StoreAndCommitError::NotCommitted(busy())),
                Err(StoreAndCommitError::NotCommitted(corrupt())),
            ],
        );
        let counted = |backend: &InOneStep| (backend.ledger.failed(), backend.ledger.lost());

        assert!(backend.step().is_ok());
        assert_eq!(counted(&backend), (0, 0), "committed");
        assert!(matches!(
            backend.step(),
            Err(StoreAndCommitError::NotStaged(_))
        ));
        assert_eq!(counted(&backend), (0, 0), "refused before it was staged");
        assert!(matches!(
            backend.step(),
            Err(StoreAndCommitError::NotCommitted(_))
        ));
        assert_eq!(counted(&backend), (1, 0), "put back");
        assert!(matches!(
            backend.step(),
            Err(StoreAndCommitError::NotCommitted(_))
        ));
        assert_eq!(counted(&backend), (2, 1), "dropped");

        let inline = InOneStep::answering(
            true,
            vec![Err(StoreAndCommitError::NotCommitted(corrupt()))],
        );
        assert!(inline.step().is_err());
        assert_eq!(
            counted(&inline),
            (1, 1),
            "what the step dropped is a loss whatever the backend's store does"
        );
    }

    /// A store being committed in one step is counted as a flush in progress
    /// from before the backend is called until its outcome is recorded: a
    /// reader that infers from the ledger then waits for it, as it would for
    /// a flush. One whose backend panics is recorded as lost first.
    #[test]
    fn should_count_a_store_being_committed_in_one_step_as_a_flush_in_progress() {
        let backend = InOneStep::answering(false, vec![Ok(())]);

        assert!(backend.step().is_ok());
        assert_eq!(
            backend.in_flight_during_the_step.load(Ordering::SeqCst),
            1,
            "in progress while the backend ran it"
        );
        assert_eq!(backend.ledger.in_flight(), 0, "and not afterwards");

        let panicked = catch_unwind(AssertUnwindSafe(|| backend.step()));
        assert!(panicked.is_err(), "the panic reaches the caller");
        assert_eq!(backend.ledger.in_flight(), 0, "no longer in progress");
        assert_eq!(
            (backend.ledger.failed(), backend.ledger.lost()),
            (1, 1),
            "counted as failed for good"
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
