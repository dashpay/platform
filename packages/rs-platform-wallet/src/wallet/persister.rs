//! Per-wallet persistence handles.
//!
//! Contains:
//! - [`WalletPersister`] — wraps the shared [`PlatformWalletPersistence`] with
//!   a fixed `wallet_id` so callers don't need to pass the ID on every call.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use dashcore::{OutPoint, Txid};
use key_wallet::managed_account::transaction_record::TransactionRecord;

use crate::changeset::flush_ledger::{keeps_a_buffer, FlushLedgers};
use crate::changeset::{
    ClientStartState, DpnsNameStateEntry, FlushLedger, PersistenceCapabilities, PersistenceError,
    PersistenceErrorKind, PlatformWalletChangeSet, PlatformWalletPersistence, StoreAndCommitError,
};
use crate::error::PlatformWalletError;
use crate::wallet::platform_wallet::WalletId;
use dpp::prelude::Identifier;

/// Transient tx-record misses collapsed during one wait or sweep, reported as
/// a single line when it ends.
///
/// Owned by the caller and dropped at its exit, so every path out — success,
/// timeout, early return — reports exactly once, and a backend that is merely
/// busy cannot flood the log from inside an unbounded poll loop.
#[derive(Debug, Default)]
pub(crate) struct TransientMissTally {
    misses: usize,
}

impl TransientMissTally {
    #[cfg(test)]
    pub(crate) fn misses(&self) -> usize {
        self.misses
    }
}

impl Drop for TransientMissTally {
    fn drop(&mut self) {
        if self.misses > 0 {
            tracing::debug!(
                transient_misses = self.misses,
                "Core tx-record reads hit transient backend failures during this pass; \
                 each was read as a miss and will be retried"
            );
        }
    }
}

/// Per-wallet persistence handle.
///
/// Thin wrapper around the shared [`PlatformWalletPersistence`] that binds
/// a specific wallet's ID. Created by [`PlatformWallet::new`] and used
/// internally for `queue_persist` / `flush_persist`.
#[derive(Clone)]
pub struct WalletPersister {
    wallet_id: WalletId,
    inner: Arc<dyn PlatformWalletPersistence>,
    contact_payment_gate: Arc<tokio::sync::Mutex<()>>,
    /// Orders the raising of the hold on unrestored private payments
    /// ([`Self::hold_until_private_payments_restored`]) against a held
    /// public request's repeat broadcast ([`Self::held_request_dispatch`]):
    /// the broadcast holds it shared from its check of the hold through its
    /// dispatch, and the raise takes it alone. So a repeat broadcast is
    /// either decided after the hold stands, and not made, or wholly made
    /// before it. Shared by the clones the wallet and its views hold.
    held_request_dispatch: Arc<tokio::sync::RwLock<()>>,
    /// Commitments of private contact request entries a private sync read
    /// back and recorded durably, whose records in the shielded store's
    /// private payment journal are to be discharged: the wallet's view
    /// notes them, and whoever holds the store discharges them (the shielded
    /// coordinator's sync pass, a restoration, a private payment). Shared by
    /// the clones the wallet and the coordinator hold. In memory only: a
    /// record left undischarged by a restart holds nothing, as the request
    /// it pays for is recorded. Only a shielded build keeps a journal to
    /// discharge, so only it keeps them.
    #[cfg(feature = "shielded")]
    read_back_private_entries: Arc<std::sync::Mutex<BTreeSet<[u8; 32]>>>,
    /// Commitments of private contact request entries a private sync read
    /// back and recorded, whose records' flush failed: noted read back
    /// (`read_back_private_entries`) once a later flush succeeds
    /// ([`Self::note_read_back_once_durable`]), at the wallet's next private
    /// sync, send or reservation. Shared by the clones the wallet and the
    /// coordinator hold. In memory only: a restart loses them with the
    /// records the failed flush did not keep.
    #[cfg(feature = "shielded")]
    unflushed_read_back_private_entries: Arc<std::sync::Mutex<BTreeSet<[u8; 32]>>>,
    /// The commitments of the private contact request payments whose journal
    /// records were discharged in this session (read back, never delivered,
    /// or proven expired). Shared by the clones the wallet and the
    /// coordinator hold. In memory only, as are the reservations they guard.
    discharged_private_entries: Arc<std::sync::Mutex<DischargedPrivateEntries>>,
    /// Whether the reservations of this wallet's unconfirmed private contact
    /// request payments from an earlier process may still be unrestored:
    /// raised at load for a wallet that bound a shielded account before, and
    /// lowered once a restoration from the shielded store succeeds. While it
    /// is up, public contact requests are neither sent nor accepted. Shared by
    /// the clones the wallet and its views hold. In memory only.
    private_payments_unrestored: Arc<AtomicBool>,
    /// The marker each identity's position record was last stored with in
    /// this session (`PrivateContactRequestsPosition::private_payments_unresolved`):
    /// a record whose marker, its identity's and the journal's together,
    /// differs from it is stored again. Absent when none was stored, when the
    /// last store or its flush failed, or once any flush of the wallet id's
    /// records failed for good since it was noted
    /// ([`StoredPrivateMarkers`]). Shared by the clones the wallet and the
    /// coordinator hold. In memory only.
    stored_private_markers: Arc<std::sync::Mutex<StoredPrivateMarkers>>,
    /// What is known of the flushes of the records stored under this
    /// wallet's id: how many are in progress, and how many failed for good,
    /// the backend dropping what it buffered. Every flush of the id is
    /// counted in it, this handle's, those of the handles of a wallet removed
    /// or created under the id since, and the manager's own, as each acts on
    /// the one buffer ([`FlushLedger`]). In memory only.
    flush_ledger: Arc<FlushLedger>,
    /// How many flushes the ledger had counted as failed for good when this
    /// handle was made. One counted since may have dropped what this wallet
    /// stored: what it tracks in memory may then not be on disk, so nothing
    /// is discharged on the strength of it alone
    /// ([`Self::buffered_writes_lost`]). One counted before dropped nothing
    /// of this wallet's, which tracks what its load read from disk.
    #[cfg(feature = "shielded")]
    flush_losses_at_start: u64,
    /// The largest size of DashPay's private contact request store a page
    /// this wallet read proved, in this process
    /// ([`Self::private_store_size_proved`]). In memory only.
    private_store_size_proved: Arc<AtomicU64>,
    /// Why the last restoration of this wallet's private payments failed,
    /// while no later one succeeded (`PlatformWalletError::PrivateContactPaymentsNotRestored`
    /// then holds for it, not only for one not run yet). Shared by the
    /// clones the wallet and its views hold. In memory only.
    private_payment_restore_failure: Arc<std::sync::Mutex<Option<String>>>,
    /// How the wallet's views run one more restoration of its private
    /// payments, without waiting on the shielded store, before a contact
    /// request is refused as not restored: installed by the wallet whenever
    /// it builds a restoration with a shielded coordinator. Shared by the
    /// clones the wallet and its views hold. In memory only.
    #[cfg(feature = "shielded")]
    private_payment_restore_retry: Arc<std::sync::Mutex<Option<PrivatePaymentRestoreRetry>>>,
}

/// One more restoration of a wallet's private payments, run on the persister
/// it is given (`WalletPersister::retry_private_payment_restore`): taking it
/// as an argument, rather than holding one, keeps the persister that stores
/// it from holding itself.
#[cfg(feature = "shielded")]
pub(crate) type PrivatePaymentRestoreRetry = Arc<
    dyn Fn(WalletPersister) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
        + Send
        + Sync,
>;

/// The commitments [`WalletPersister::note_discharged_private_entries`] noted,
/// and the pairs the journal's unresolved payments hold, all updated under one
/// lock.
#[derive(Default)]
struct DischargedPrivateEntries {
    /// Noted since the last take, for the wallet to release the reservations
    /// it holds for them ([`WalletPersister::take_unreleased_discharged_private_entries`]).
    unreleased: BTreeSet<[u8; 32]>,
    /// Every one noted in this session, kept after the take: a restoration
    /// that read a record before it was discharged, and reserves its pair
    /// after, checks it here first under the wallet lock
    /// ([`WalletPersister::private_entry_discharged`]). Only a shielded build
    /// keeps a journal to discharge: empty otherwise.
    session: BTreeSet<[u8; 32]>,
    /// The (sender, recipient) pair of each private contact request payment
    /// the shielded store's journal keeps, by its entry's commitment
    /// ([`WalletPersister::note_journaled_private_payments`]): the wallet's
    /// hold on the pair, which no identity's removal drops. A commitment in
    /// `session` is never added, and one noted there is removed, so a
    /// discharged payment holds nothing. Only a shielded build journals:
    /// empty otherwise.
    journaled: BTreeMap<[u8; 32], (Identifier, Identifier)>,
}

/// The marker each identity's position record was last stored with
/// ([`WalletPersister::note_private_marker_stored`]), with how many flushes of
/// the wallet id's records had failed for good when they were noted. One more
/// since dropped what the backend had buffered, whoever made that flush, and
/// a store noted here may still have been in it: so none is known stored any
/// longer, and all are forgotten ([`WalletPersister::stored_markers`]).
#[derive(Default)]
struct StoredPrivateMarkers {
    markers: BTreeMap<Identifier, bool>,
    /// The ledger's count of flushes failed for good that `markers` were
    /// noted under ([`FlushLedger::lost`]).
    losses: u64,
}

/// Why records stored for a wallet are not known to be durable
/// ([`WalletPersister::store_durably`], [`WalletPersister::flush_durably`]).
#[derive(Debug, thiserror::Error)]
pub(crate) enum NotDurable {
    /// A flush of the wallet id's records failed for good, the backend
    /// dropping what it buffered: since before the store, for a store made
    /// durable, and since the handle was made otherwise
    /// (`WalletPersister::buffered_writes_lost`).
    #[error(
        "a flush of the wallet's records failed for good, and the backend dropped what it \
         had buffered"
    )]
    Lost,
    /// The store this call made failed, so nothing was flushed. From a
    /// backend that commits a store in one step
    /// ([`PlatformWalletPersistence::store_and_commit`]) it says more: the
    /// changeset was refused before it was staged, so none of it is buffered
    /// and none of it is on disk, and after a transient failure it may be
    /// issued again.
    #[error("{0}")]
    Store(PersistenceError),
    /// The flush this call made failed, or the commit of a store made in one
    /// step: a transient failure keeps what the backend buffered, any other
    /// drops it. A store made durable in two calls answers with it only when
    /// that flush alone could have taken what was stored. With another flush
    /// in progress after it, or returned since before the store, the
    /// record's outcome is that flush's, and the answer is [`Self::Lost`] or
    /// [`Self::OtherFlushUnsettled`] ([`WalletPersister::store_durably`]).
    #[error("{0}")]
    Flush(PersistenceError),
    /// Another flush of the wallet id's records has not returned, or
    /// returned while the call ran. One in progress, or one that failed with
    /// the backend keeping what it had taken from the buffer, may hold
    /// records the call answers for, which a later flush writes. One that
    /// succeeded while a store was made durable may have written an older
    /// state of what was stored after the call's own flush wrote the newer.
    /// A later call answers.
    #[error(
        "another flush of the wallet's records is in progress, or returned while this one \
         ran, so what was flushed may not be everything stored, or may not be what was \
         written last"
    )]
    OtherFlushUnsettled,
}

/// [`WalletPersister::losses`], for a handle being made.
fn losses_of(
    ledger: &FlushLedger,
    backend: &dyn PlatformWalletPersistence,
    wallet_id: WalletId,
) -> u64 {
    // A backend that keeps no buffer has nothing a drain could drop, as the
    // ledger has it too. One whose store is the commit and that commits a
    // store in one step does keep one (`keeps_a_buffer`): what a commit that
    // failed transiently left in it goes with the next write that fails for
    // good, a plain `store` included, which only the backend counts.
    let dropped_unseen = if keeps_a_buffer(backend) {
        backend.buffered_writes_dropped(wallet_id).unwrap_or(0)
    } else {
        0
    };
    ledger.lost().saturating_add(dropped_unseen)
}

/// What was counted for a wallet when it was read, before a store that is
/// then made durable: the losses ([`WalletPersister::losses`]), and in its
/// [`FlushLedger`] the flushes that failed at all and those that returned.
#[derive(Clone, Copy)]
struct FlushesSoFar {
    lost: u64,
    failed: u64,
    returned: u64,
}

impl WalletPersister {
    /// A handle for `wallet_id`'s records on `inner`. It counts its flushes
    /// in the ledger of that backend instance and wallet id
    /// ([`FlushLedgers::of_backend`]), the one every other handle made over
    /// the same backend instance for the id counts in: whichever manager, or
    /// no manager, made it, and for as long as the backend lives.
    pub fn new(wallet_id: WalletId, inner: Arc<dyn PlatformWalletPersistence>) -> Self {
        let flush_ledger = FlushLedgers::of_backend(&inner).of(wallet_id);
        #[cfg(feature = "shielded")]
        let flush_losses_at_start = losses_of(&flush_ledger, inner.as_ref(), wallet_id);
        Self {
            wallet_id,
            inner,
            contact_payment_gate: Arc::new(tokio::sync::Mutex::new(())),
            held_request_dispatch: Arc::default(),
            #[cfg(feature = "shielded")]
            read_back_private_entries: Arc::default(),
            #[cfg(feature = "shielded")]
            unflushed_read_back_private_entries: Arc::default(),
            discharged_private_entries: Arc::default(),
            private_payments_unrestored: Arc::default(),
            stored_private_markers: Arc::default(),
            #[cfg(feature = "shielded")]
            flush_losses_at_start,
            flush_ledger,
            private_store_size_proved: Arc::default(),
            private_payment_restore_failure: Arc::default(),
            #[cfg(feature = "shielded")]
            private_payment_restore_retry: Arc::default(),
        }
    }

    /// Note how the last restoration of this wallet's private payments went:
    /// `Some(reason)` when it failed, `None` once one succeeded
    /// ([`Self::private_payment_restore_failure`]).
    #[cfg(feature = "shielded")]
    pub(crate) fn note_private_payment_restore(&self, failure: Option<String>) {
        *self
            .private_payment_restore_failure
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = failure;
    }

    /// Why the last restoration of this wallet's private payments failed, if
    /// it did and none succeeded since.
    pub(crate) fn private_payment_restore_failure(&self) -> Option<String> {
        self.private_payment_restore_failure
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// Install how the wallet's views run one more restoration of its private
    /// payments ([`Self::retry_private_payment_restore`]).
    #[cfg(feature = "shielded")]
    pub(crate) fn set_private_payment_restore_retry(&self, retry: PrivatePaymentRestoreRetry) {
        *self
            .private_payment_restore_retry
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(retry);
    }

    /// Run one more restoration of this wallet's private payments, which waits
    /// on nothing: a busy shielded store leaves the wallet held. Returns
    /// whether one ran, as none does before the wallet built a restoration
    /// with a shielded coordinator.
    #[cfg(feature = "shielded")]
    pub(crate) async fn retry_private_payment_restore(&self) -> bool {
        let retry = self
            .private_payment_restore_retry
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        match retry {
            Some(retry) => {
                retry(self.clone()).await;
                true
            }
            None => false,
        }
    }

    /// Note that `owner`'s position record was stored with the marker
    /// `marker`, or, with `None`, that whether it was is not known: its last
    /// store or flush failed ([`Self::private_marker_stored`]).
    pub(crate) fn note_private_marker_stored(&self, owner: Identifier, marker: Option<bool>) {
        let mut stored = self.stored_markers();
        match marker {
            Some(marker) => stored.markers.insert(owner, marker),
            None => stored.markers.remove(&owner),
        };
    }

    /// Store `changeset`, which carries `owner`'s position record with the
    /// marker `marker`, and note the marker as stored
    /// ([`Self::note_private_marker_stored`]): only when the store succeeded
    /// and no flush of the wallet id's records failed for good since before
    /// it. One that did may have dropped the record from the buffer, so the
    /// marker is then not known stored and the next comparison stores the
    /// record again. The count is compared under the lock the marker is
    /// noted in, so a loss recorded later forgets it there
    /// ([`Self::stored_markers`]).
    pub(crate) fn store_private_marker_record(
        &self,
        owner: Identifier,
        marker: bool,
        changeset: PlatformWalletChangeSet,
    ) -> Result<(), PersistenceError> {
        let losses_before = self.losses();
        let outcome = self.store(changeset);
        let mut stored = self.stored_markers();
        if outcome.is_ok() && stored.losses == losses_before {
            stored.markers.insert(owner, marker);
        } else {
            stored.markers.remove(&owner);
        }
        drop(stored);
        outcome
    }

    /// The markers noted as stored, forgotten first when a flush of the
    /// wallet id's records failed for good since they were noted: this
    /// handle's flush, another handle's or the manager's, as the ledger
    /// counts them all. The next comparison then stores each marker again.
    fn stored_markers(&self) -> std::sync::MutexGuard<'_, StoredPrivateMarkers> {
        let mut stored = self
            .stored_private_markers
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let losses = self.losses();
        if stored.losses != losses {
            stored.markers.clear();
            stored.losses = losses;
        }
        stored
    }

    /// The marker `owner`'s position record was last stored with in this
    /// session, if one was, its store did not fail since, and no flush of the
    /// wallet id's records failed for good since
    /// ([`Self::note_private_marker_stored`]).
    pub(crate) fn private_marker_stored(&self, owner: &Identifier) -> Option<bool> {
        self.stored_markers().markers.get(owner).copied()
    }

    /// Note the commitments of private contact request entries a private sync
    /// read back and recorded durably, for their journal records to be
    /// discharged ([`Self::take_read_back_private_entries`]).
    #[cfg(feature = "shielded")]
    pub(crate) fn note_read_back_private_entries(
        &self,
        commitments: impl IntoIterator<Item = [u8; 32]>,
    ) {
        self.read_back_entries().extend(commitments);
    }

    /// Note `commitments`, of private requests a private sync read back and
    /// recorded, for their journal records to be discharged
    /// ([`Self::note_read_back_private_entries`]), once what records them is
    /// durable ([`Self::flush_durably`]): at once for a backend that keeps no
    /// buffer, after a flush otherwise. Those a transient flush failure left
    /// are kept, with every one kept before, and noted by the next call whose
    /// flush succeeds: an empty one retries them. So are those of a call made
    /// while another flush is in progress, whose outcome is not known yet, or
    /// during which another flush failed and put back what it had taken.
    /// Those any other failure left are dropped, as the backend dropped the
    /// records they rest on. Their journal records, and so the pairs they
    /// hold, stay until noted. The host's flush runs with none of the
    /// persister's locks held, so a call during it, from another thread or
    /// the flush itself, neither waits nor loses what it adds.
    ///
    /// None is noted once a flush of the wallet id's records failed for good
    /// since this handle was made ([`Self::buffered_writes_lost`]): a request
    /// read back since may be one the wallet tracked before the loss, which a
    /// read of its entry again finds tracked and stores nothing for, so its
    /// record may be in memory only. Those journal records stay for the next
    /// process, whose restoration reads their entries again from what is on
    /// disk.
    #[cfg(feature = "shielded")]
    pub(crate) fn note_read_back_once_durable(
        &self,
        commitments: impl IntoIterator<Item = [u8; 32]>,
    ) {
        let pending = {
            let mut unflushed = self.unflushed_read_back_entries();
            unflushed.extend(commitments);
            std::mem::take(&mut *unflushed)
        };
        if pending.is_empty() {
            return;
        }
        match self.flush_durably() {
            Ok(()) => self.note_read_back_private_entries(pending),
            Err(NotDurable::Flush(error)) if error.is_transient() => {
                tracing::warn!(
                    %error,
                    kept = pending.len(),
                    "private requests read back are not durably recorded; their journal \
                     records stay until a later flush succeeds"
                );
                // A transient failure keeps the buffer: with any added
                // meanwhile.
                self.unflushed_read_back_entries().extend(pending);
            }
            Err(NotDurable::OtherFlushUnsettled) => {
                tracing::warn!(
                    kept = pending.len(),
                    "private requests read back while another flush of this wallet's records \
                     is in progress, returned meanwhile, or failed keeping what it took, are \
                     not noted yet; a later call notes them"
                );
                self.unflushed_read_back_entries().extend(pending);
            }
            // `flush_durably` stores nothing, so `Store` is never its answer.
            Err(NotDurable::Flush(error) | NotDurable::Store(error)) => {
                // Any other failure dropped the buffer, the records these
                // notices rest on with it: the notices go, so their journal
                // records, and the pairs they hold, stay. Their read is not
                // rewound: a read again would find the requests already
                // tracked, store nothing, and note them again over records
                // that never reached disk. The markers stored into that
                // buffer are forgotten, as after any flush the ledger counts
                // as failed for good, so the next comparison stores them
                // again (`Self::private_marker_stored`).
                tracing::error!(
                    %error,
                    dropped = pending.len(),
                    "private requests read back are not durably recorded, and the backend \
                     dropped what it buffered; their journal records stay"
                );
            }
            Err(NotDurable::Lost) => tracing::warn!(
                dropped = pending.len(),
                "private requests read back after this wallet's records were lost are not \
                 noted; their journal records stay for the next process"
            ),
        }
    }

    /// Whether the read-back of `commitment` waits for a flush that failed
    /// transiently ([`Self::note_read_back_once_durable`]): that path
    /// discharges its journal record once a flush succeeds.
    #[cfg(feature = "shielded")]
    pub(crate) fn read_back_awaiting_flush(&self, commitment: &[u8; 32]) -> bool {
        self.unflushed_read_back_entries().contains(commitment)
    }

    #[cfg(feature = "shielded")]
    fn unflushed_read_back_entries(&self) -> std::sync::MutexGuard<'_, BTreeSet<[u8; 32]>> {
        self.unflushed_read_back_private_entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Take the commitments [`Self::note_read_back_private_entries`] noted:
    /// a caller that fails to discharge one notes it again.
    #[cfg(feature = "shielded")]
    pub(crate) fn take_read_back_private_entries(&self) -> BTreeSet<[u8; 32]> {
        std::mem::take(&mut *self.read_back_entries())
    }

    /// Note `commitments`, whose private payments' journal records were just
    /// discharged: a restoration that read them before skips them
    /// ([`Self::private_entry_discharged`]), and the wallet releases what it
    /// still reserves for them
    /// ([`Self::take_unreleased_discharged_private_entries`]).
    #[cfg(feature = "shielded")]
    pub(crate) fn note_discharged_private_entries(
        &self,
        commitments: impl IntoIterator<Item = [u8; 32]>,
    ) {
        let mut discharged = self.discharged_entries();
        for commitment in commitments {
            discharged.unreleased.insert(commitment);
            discharged.session.insert(commitment);
            discharged.journaled.remove(&commitment);
        }
    }

    /// Note `payments`, each a private contact request payment's entry
    /// commitment with its (sender, recipient) pair, as kept, or about to be
    /// kept, by the shielded store's private payment journal: until each is
    /// discharged ([`Self::note_discharged_private_entries`]) the wallet holds
    /// its pair ([`Self::private_pair_journaled`]), whatever happens to the
    /// sender identity. A payment discharged in this session is not noted.
    /// Noted before a payment is journaled, and by every restoration from the
    /// journal, so the pairs held cover every record the journal keeps.
    #[cfg(feature = "shielded")]
    pub(crate) fn note_journaled_private_payments(
        &self,
        payments: impl IntoIterator<Item = ([u8; 32], Identifier, Identifier)>,
    ) {
        let mut entries = self.discharged_entries();
        for (commitment, sender, recipient) in payments {
            if !entries.session.contains(&commitment) {
                entries.journaled.insert(commitment, (sender, recipient));
            }
        }
    }

    /// Forget `commitments`, noted by [`Self::note_journaled_private_payments`]
    /// for a payment whose journal records were then not saved: nothing was
    /// paid, so its pair is not held for it.
    #[cfg(feature = "shielded")]
    pub(crate) fn forget_journaled_private_payments(&self, commitments: &[[u8; 32]]) {
        let mut entries = self.discharged_entries();
        for commitment in commitments {
            entries.journaled.remove(commitment);
        }
    }

    /// A recipient of a private payment from `sender` the journal keeps
    /// unresolved ([`Self::note_journaled_private_payments`]), if any: the
    /// sender then holds a payment that may still execute, whatever its
    /// identity's own state says.
    pub(crate) fn private_sender_journaled(&self, sender: &Identifier) -> Option<Identifier> {
        self.discharged_entries()
            .journaled
            .values()
            .find(|(journaled_sender, _)| journaled_sender == sender)
            .map(|(_, recipient)| *recipient)
    }

    /// Whether a private payment the journal keeps unresolved holds the pair
    /// `sender`, `recipient` ([`Self::note_journaled_private_payments`]): then
    /// neither a public request nor another private one is sent for it.
    pub(crate) fn private_pair_journaled(
        &self,
        sender: &Identifier,
        recipient: &Identifier,
    ) -> bool {
        self.discharged_entries()
            .journaled
            .values()
            .any(|pair| *pair == (*sender, *recipient))
    }

    /// Whether the journal record of `commitment` was discharged in this
    /// session, its reservation released or not.
    pub(crate) fn private_entry_discharged(&self, commitment: &[u8; 32]) -> bool {
        self.discharged_entries().session.contains(commitment)
    }

    /// Whether [`Self::note_discharged_private_entries`] noted any commitment
    /// since the last take.
    pub(crate) fn has_unreleased_discharged_private_entries(&self) -> bool {
        !self.discharged_entries().unreleased.is_empty()
    }

    /// Take the commitments [`Self::note_discharged_private_entries`] noted
    /// since the last take. Take them only where they are applied with
    /// nothing awaited in between.
    pub(crate) fn take_unreleased_discharged_private_entries(&self) -> BTreeSet<[u8; 32]> {
        std::mem::take(&mut self.discharged_entries().unreleased)
    }

    fn discharged_entries(&self) -> std::sync::MutexGuard<'_, DischargedPrivateEntries> {
        self.discharged_private_entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Whether [`Self::note_read_back_private_entries`] noted any commitment
    /// not taken yet.
    #[cfg(feature = "shielded")]
    pub(crate) fn has_read_back_private_entries(&self) -> bool {
        !self.read_back_entries().is_empty()
    }

    /// Hold this wallet's public contact requests, sent or accepted, until
    /// [`Self::private_payments_restored`]: its shielded store may hold an
    /// unconfirmed private payment from an earlier process whose pair no
    /// reservation holds yet.
    ///
    /// Raising it waits for any held public request's repeat broadcast that
    /// is between its check of this hold and its dispatch
    /// ([`Self::held_request_dispatch`]), so none is decided while the hold
    /// is down and dispatched once it is up. A hold that already stands is
    /// not raised again and waits for nothing. The caller holds no wallet
    /// manager lock: the repeat broadcast takes one under the gate.
    #[cfg(feature = "shielded")]
    pub(crate) async fn hold_until_private_payments_restored(&self) {
        self.hold_until_private_payments_restored_while(|| Some(()))
            .await;
    }

    /// [`Self::hold_until_private_payments_restored`], for a caller whose
    /// reason to hold may be overtaken while the raise waits its turn: once
    /// the raise may go ahead, `still_unrestored` answers with the caller's
    /// lock on what could overtake it, kept until the hold is raised, or
    /// with `None` when it was overtaken and nothing is raised. Whoever
    /// releases the hold ([`Self::private_payments_restored`]) for that
    /// reason does so under the same lock, so a raise and a release never
    /// cross: the raise either sees the release or comes wholly before it.
    #[cfg(feature = "shielded")]
    pub(crate) async fn hold_until_private_payments_restored_while<G>(
        &self,
        still_unrestored: impl FnOnce() -> Option<G>,
    ) {
        if self.private_payments_unrestored() {
            return;
        }
        let _no_repeat_broadcast = self.held_request_dispatch.write().await;
        let Some(_not_overtaken) = still_unrestored() else {
            return;
        };
        self.private_payments_unrestored
            .store(true, Ordering::Release);
    }

    /// Held by a held public request's repeat broadcast from before it
    /// checks [`Self::private_payments_unrestored`] until its bytes are
    /// dispatched: the hold is not raised meanwhile
    /// ([`Self::hold_until_private_payments_restored`]).
    pub(crate) async fn held_request_dispatch(&self) -> tokio::sync::RwLockReadGuard<'_, ()> {
        self.held_request_dispatch.read().await
    }

    /// The reservations of this wallet's unconfirmed private payments are
    /// restored: release the hold [`Self::hold_until_private_payments_restored`]
    /// raised. Called after the restored reservations are in place, so a send
    /// that sees the hold gone sees them.
    #[cfg(feature = "shielded")]
    pub(crate) fn private_payments_restored(&self) {
        self.private_payments_unrestored
            .store(false, Ordering::Release);
    }

    /// Whether [`Self::hold_until_private_payments_restored`] holds this
    /// wallet's public contact requests.
    pub(crate) fn private_payments_unrestored(&self) -> bool {
        self.private_payments_unrestored.load(Ordering::Acquire)
    }

    #[cfg(feature = "shielded")]
    fn read_back_entries(&self) -> std::sync::MutexGuard<'_, BTreeSet<[u8; 32]>> {
        self.read_back_private_entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Keep contact-pool snapshots in reservation order across Core and
    /// withdrawal sends, including a Core broadcast rejection's pool rollback.
    ///
    /// The gate is shared by clones of this handle, not by every handle for
    /// the same `wallet_id`: each [`Self::new`] mints its own. Ordering holds
    /// because the only lockers (the DashPay payment paths) run on the handle
    /// `PlatformWallet::new` builds once per wallet. Any new writer of DashPay
    /// external-account pool snapshots must lock through a clone of that
    /// handle; a separately constructed one (such as the incoming-payment
    /// hook's) would silently bypass the ordering.
    pub(crate) async fn lock_contact_payments(&self) -> tokio::sync::MutexGuard<'_, ()> {
        self.contact_payment_gate.lock().await
    }

    /// Store `changeset` for this wallet.
    ///
    /// On a backend that cannot keep outbound-account markers durably (see
    /// [`persists_outbound_account_markers`](crate::changeset::persists_outbound_account_markers)),
    /// every established-contact row is written without its marker: a marker
    /// that reached disk without its account — through a buffered round that
    /// was later dropped, or a host that skips account writes — would be
    /// trusted on restart beside the previous account. The in-memory marker is
    /// untouched, so the running process keeps its rotation self-heal.
    pub(crate) fn store(&self, changeset: PlatformWalletChangeSet) -> Result<(), PersistenceError> {
        self.inner
            .store(self.wallet_id, self.as_the_backend_keeps_it(changeset))
    }

    /// `changeset` as it is handed to the backend, by [`Self::store`] and by
    /// a store committed in one step alike: without the outbound-account
    /// markers of its established contacts on a backend that cannot keep
    /// them durably.
    fn as_the_backend_keeps_it(
        &self,
        mut changeset: PlatformWalletChangeSet,
    ) -> PlatformWalletChangeSet {
        if !self.persists_outbound_account_markers() {
            if let Some(contacts) = changeset.contacts.as_mut() {
                for established in contacts.established.values_mut() {
                    established.external_account_reference = None;
                }
            }
        }
        changeset
    }

    /// Whether the backend keeps outbound-account markers durably (see
    /// [`persists_outbound_account_markers`](crate::changeset::persists_outbound_account_markers)).
    pub(crate) fn persists_outbound_account_markers(&self) -> bool {
        crate::changeset::persists_outbound_account_markers(self.inner.as_ref())
    }

    /// Preserve backend failure kinds, granting retry only when the backend
    /// guarantees a failed store retained and committed nothing.
    pub(crate) fn classify_store_failure(&self, error: PersistenceError) -> PlatformWalletError {
        PlatformWalletError::from_store_failure(self.inner.as_ref(), error)
    }

    /// What a caller reports when [`Self::store_durably`] did not answer for
    /// a record: the store's failure as [`Self::classify_store_failure`]
    /// gives it, and anything else as the flush failure it is. A flush that
    /// failed for good, the call's own or another that took the record, is
    /// `PersisterFlush` of kind `Fatal`: the record is gone. Another flush
    /// still in progress, or one that put back what it took, leaves the
    /// record buffered for a later flush, and one that returned while the
    /// call ran may have written an older state after it: `PersisterFlush`
    /// of kind `Transient`, to be tried again, which stores the record anew.
    /// A store committed in one step is reported the same way: refused
    /// before it was staged as the store's failure, and a failed commit as
    /// the flush failure, with the backend's own kind.
    ///
    /// One difference for the store's failure. From a backend that commits a
    /// store in one step it keeps the backend's kind, a transient one
    /// included: that answer says nothing of the changeset was staged or
    /// written, which is all a retry needs, so it does not wait on
    /// [`PlatformWalletPersistence::store_transient_is_reissuable`], which
    /// speaks for a plain `store`. On such a backend every
    /// [`NotDurable::Store`] is that answer, as [`Self::store_durably`]
    /// makes no plain store there.
    pub(crate) fn classify_not_durable(&self, not_durable: NotDurable) -> PlatformWalletError {
        match not_durable {
            NotDurable::Store(error) if self.inner.offers_store_and_commit() => {
                PlatformWalletError::from_unstaged_store_failure(error)
            }
            NotDurable::Store(error) => self.classify_store_failure(error),
            NotDurable::Flush(error) => PlatformWalletError::from_flush_failure(error),
            NotDurable::Lost => PlatformWalletError::from_flush_failure(
                PersistenceError::backend_with_kind(PersistenceErrorKind::Fatal, not_durable),
            ),
            NotDurable::OtherFlushUnsettled => PlatformWalletError::from_flush_failure(
                PersistenceError::backend_with_kind(PersistenceErrorKind::Transient, not_durable),
            ),
        }
    }

    /// Flush this wallet's records, counted in the ledger of its id
    /// ([`FlushLedger::flush`]).
    pub(crate) fn flush(&self) -> Result<(), PersistenceError> {
        self.flush_ledger.flush(self.inner.as_ref(), self.wallet_id)
    }

    /// Record `identity` as the recipient of this wallet's asset lock at
    /// `out_point`, the recipient alone and at once
    /// ([`PlatformWalletPersistence::assign_asset_lock_identity_recipient`]):
    /// `Ok` once committed, and any `Err` leaves nothing to commit later.
    /// Counted in the flush ledger like a flush: the backend writes what was
    /// buffered first, so a failure for good there is a loss the others must
    /// see.
    pub(crate) fn assign_asset_lock_identity_recipient(
        &self,
        out_point: OutPoint,
        identity: Identifier,
    ) -> Result<(), PersistenceError> {
        self.flush_ledger.assign_identity_recipient(
            self.inner.as_ref(),
            self.wallet_id,
            out_point,
            identity,
        )
    }

    /// Store `changeset` and answer `Ok` only once it is durable.
    ///
    /// A backend that stages a changeset and commits the wallet's buffer in
    /// one step ([`PlatformWalletPersistence::store_and_commit`]) is asked
    /// to, and its answer is the answer: it speaks for exactly what this
    /// call staged, whatever else drained the buffer around it, so nothing
    /// is read from the ledger to decide it
    /// ([`Self::store_and_commit`]). The rest of this is about every other
    /// backend.
    ///
    /// Any other backend is stored to and flushed in two calls, and what
    /// came between them is worked out from the ledger: `Ok` at once for a
    /// backend that commits a store inline, and otherwise after a flush
    /// that succeeds with, after it, no flush of the wallet id's records in
    /// progress, and none but it returned since before the store
    /// ([`FlushLedger`]), read in that order.
    ///
    /// The flush succeeding proves nothing by itself. Another flush may take
    /// what the store buffered and fail, dropping it or putting it back, and
    /// the one here then succeeds over a buffer that does not hold it. Or
    /// another flush, which took an older state of the same records before
    /// the store, may write it after the one here wrote the newer, and
    /// succeed: a restart then finds the older state. The persistence
    /// contract requires a backend to write what a flush took before what a
    /// later one takes (`PlatformWalletPersistence::flush`), and a store
    /// made durable does not rest on it. Either flush began before this one
    /// returned. So after this flush it shows as a
    /// flush in progress, or as one more that returned since before the
    /// store than this call's own: a flush is counted as returned, whatever
    /// its outcome, before it stops being counted as in progress. One that
    /// failed for good answers [`NotDurable::Lost`]; any other, in progress
    /// or returned, [`NotDurable::OtherFlushUnsettled`].
    ///
    /// Another flush that returned across the call refuses it whatever it
    /// took, one that held nothing of these records included: the ledger
    /// does not know what a flush took.
    ///
    /// When the flush here fails, the same counts are read before its
    /// failure is the answer. [`NotDurable::Flush`] says the record is
    /// buffered still, or dropped, by that flush, which holds only when no
    /// other flush could have taken the record: with one in progress, or
    /// returned since before the store, or a drop the backend reports, the
    /// answer is [`NotDurable::OtherFlushUnsettled`] or [`NotDurable::Lost`]
    /// as it is after a flush that succeeded.
    ///
    /// It answers for `changeset` alone, which [`Self::flush_durably`] does
    /// not: a flush that failed for good before the store dropped nothing of
    /// it, and does not refuse it.
    pub(crate) fn store_durably(
        &self,
        changeset: PlatformWalletChangeSet,
    ) -> Result<(), NotDurable> {
        if self.inner.offers_store_and_commit() {
            return self.store_and_commit(changeset);
        }
        let before = self.flushes_so_far();
        self.store(changeset).map_err(NotDurable::Store)?;
        self.flush_what_was_stored(before)
    }

    /// [`Self::store_durably`] for a changeset that withdraws a record stored
    /// before it (a row's removal), where `Ok` has to mean more than that
    /// the withdrawal is on disk: that nothing writes the record after it.
    ///
    /// A flush inside the backend while the withdrawal is stored may have
    /// taken the record without it, and the withdrawal does not rest on the
    /// backend writing what that flush took before what a later one takes,
    /// though the persistence contract requires it
    /// (`PlatformWalletPersistence::flush`). Should it write the record
    /// after the withdrawal, the counts of failed flushes say nothing of it:
    /// it succeeded. So such a flush answers
    /// [`NotDurable::OtherFlushUnsettled`], and nothing is flushed, whether
    /// it is still in progress once the withdrawal is stored or has returned
    /// since before the store ([`FlushLedger::returned`]). The second case
    /// is one that returns right after the store, behind another that took
    /// the withdrawal and wrote it first: no flush is in progress then, and
    /// the buffer is empty. With none in progress and none returned across
    /// the store, every flush that could hold the record alone returned
    /// before the store began, and one that begins later takes the
    /// withdrawal with whatever of the record is still buffered, the
    /// withdrawal behind it.
    ///
    /// A flush that returned across the store is refused whatever it took,
    /// one that held nothing of the record included: the ledger does not
    /// know what a flush took.
    ///
    /// None of that is worked out for a backend that commits a store in one
    /// step ([`PlatformWalletPersistence::store_and_commit`]), which is
    /// asked to commit the withdrawal as [`Self::store_durably`] asks it to
    /// commit any changeset. That step's contract is what the above
    /// establishes from outside: a drain that began before it has finished
    /// by the time the withdrawal is staged, what it took written, and the
    /// commit writes whatever of the record is still buffered with the
    /// withdrawal behind it. So nothing writes the record after it.
    pub(crate) fn withdraw_durably(
        &self,
        changeset: PlatformWalletChangeSet,
    ) -> Result<(), NotDurable> {
        if self.inner.offers_store_and_commit() {
            return self.store_and_commit(changeset);
        }
        let before = self.flushes_so_far();
        self.store(changeset).map_err(NotDurable::Store)?;
        // Read in this order: a flush that stops being in progress is counted
        // as returned first (`FlushLedger`).
        if !self.store_commits_inline()
            && (self.flush_ledger.in_flight() != 0
                || self.flush_ledger.returned() != before.returned)
        {
            return Err(NotDurable::OtherFlushUnsettled);
        }
        self.flush_what_was_stored(before)
    }

    /// How many times what was buffered under this wallet's id was dropped:
    /// by a flush counted in the ledger that failed for good, or by a drain
    /// of the backend the ledger does not see, which the backend reports
    /// ([`PlatformWalletPersistence::buffered_writes_dropped`]). Compared,
    /// never read as a number: a flush the wallet made is in both counts.
    fn losses(&self) -> u64 {
        losses_of(&self.flush_ledger, self.inner.as_ref(), self.wallet_id)
    }

    /// What the wallet id's ledger has counted so far, read before a store
    /// that is then made durable ([`Self::flush_what_was_stored`]).
    fn flushes_so_far(&self) -> FlushesSoFar {
        FlushesSoFar {
            lost: self.losses(),
            failed: self.flush_ledger.failed(),
            returned: self.flush_ledger.returned(),
        }
    }

    /// The flush half of [`Self::store_durably`], for a changeset just
    /// stored: `before` is the ledger's counts read before that store.
    fn flush_what_was_stored(&self, before: FlushesSoFar) -> Result<(), NotDurable> {
        if self.store_commits_inline() {
            return Ok(());
        }
        let flushed = self.flush();
        // The count of flushes in progress is read first: one that stops
        // being in progress has recorded its failure, and is counted as
        // returned, before then (`FlushLedger`).
        let another_in_progress = self.flush_ledger.in_flight() != 0;
        if let Err(error) = flushed {
            return Err(self.not_durable_after_its_flush_failed(
                error,
                another_in_progress,
                before,
            ));
        }
        if another_in_progress {
            return Err(NotDurable::OtherFlushUnsettled);
        }
        if self.losses() != before.lost {
            return Err(NotDurable::Lost);
        }
        if self.flush_ledger.failed() != before.failed {
            return Err(NotDurable::OtherFlushUnsettled);
        }
        // The flush above is one that returned since the store. Any other
        // succeeded, as none failed: it may have written an older state of
        // what was stored after that flush wrote the newer.
        if self.flush_ledger.returned() != before.returned + 1 {
            return Err(NotDurable::OtherFlushUnsettled);
        }
        Ok(())
    }

    /// [`Self::store_durably`] and [`Self::withdraw_durably`] on a backend
    /// that stages `changeset` and commits the wallet's buffer in one step
    /// ([`PlatformWalletPersistence::store_and_commit`]).
    ///
    /// The backend's answer is taken as it is. A changeset refused before it
    /// was staged is [`NotDurable::Store`]: nothing of it is buffered or on
    /// disk. A commit that failed is [`NotDurable::Flush`]: buffered still
    /// after a transient failure, dropped after any other. Neither
    /// [`NotDurable::Lost`] nor [`NotDurable::OtherFlushUnsettled`] is ever
    /// the answer, as both stand for what could not be told from outside the
    /// backend.
    ///
    /// The step is counted in the wallet id's ledger as the flush it
    /// contains ([`FlushLedger::store_and_commit`]), for the readers that
    /// still infer: a commit that fails for good drops what other handles
    /// had buffered with it, and [`Self::flush_durably`] and the stored
    /// markers learn of that from the ledger.
    fn store_and_commit(&self, changeset: PlatformWalletChangeSet) -> Result<(), NotDurable> {
        self.flush_ledger
            .store_and_commit(
                self.inner.as_ref(),
                self.wallet_id,
                self.as_the_backend_keeps_it(changeset),
            )
            .map_err(|not_committed| match not_committed {
                StoreAndCommitError::NotStaged(error) => NotDurable::Store(error),
                StoreAndCommitError::NotCommitted(error) => NotDurable::Flush(error),
            })
    }

    /// What [`Self::flush_what_was_stored`] answers when its own flush
    /// failed with `error`: `before` is the ledger's counts read before the
    /// store, and `another_in_progress` whether a flush was in progress once
    /// this one had returned.
    ///
    /// The failure says what became of what was stored only when the flush
    /// that failed is the one that took it: a transient one then kept it
    /// buffered, and any other dropped it. Another flush, in progress or
    /// returned since before the store, may have taken it instead, and
    /// that flush's outcome, not this one's, is then the record's: put back,
    /// written, or dropped later with nothing left buffered. So the failure
    /// is the answer only with no such flush. Otherwise the answer is the
    /// ledger's, as after a flush that succeeded, and a caller that stores
    /// its record again on that answer does so here as well.
    ///
    /// A transient failure adds nothing to the losses, so a change in them
    /// is a drop some other drain made, one the ledger does not see
    /// included ([`Self::losses`]): [`NotDurable::Lost`]. A flush of the
    /// call's own that failed for good is itself a loss, whoever took the
    /// record: with another flush across the call it answers
    /// [`NotDurable::Lost`] and never [`NotDurable::OtherFlushUnsettled`],
    /// and with none its failure says the same.
    fn not_durable_after_its_flush_failed(
        &self,
        error: PersistenceError,
        another_in_progress: bool,
        before: FlushesSoFar,
    ) -> NotDurable {
        // The flush that failed is one that returned since the store.
        let another_returned = self.flush_ledger.returned() != before.returned + 1;
        let answer = if !error.is_transient() {
            (another_in_progress || another_returned).then_some(NotDurable::Lost)
        } else if another_in_progress {
            Some(NotDurable::OtherFlushUnsettled)
        } else if self.losses() != before.lost {
            // A transient failure adds nothing to the losses.
            Some(NotDurable::Lost)
        } else {
            another_returned.then_some(NotDurable::OtherFlushUnsettled)
        };
        match answer {
            Some(answer) => {
                tracing::warn!(
                    %error,
                    %answer,
                    "a flush made to make a store durable failed, and may not be the flush \
                     that took what was stored"
                );
                answer
            }
            None => NotDurable::Flush(error),
        }
    }

    /// Make every record stored for this wallet before the call durable, and
    /// answer `Ok` only once it is: at once for a backend that keeps no
    /// buffer ([`keeps_a_buffer`]), after a flush that succeeds otherwise,
    /// and never once a flush of the wallet id's records failed for good
    /// since this handle was made ([`Self::buffered_writes_lost`]).
    ///
    /// A backend whose store is the commit keeps no buffer, unless it also
    /// commits a store in one step
    /// ([`PlatformWalletPersistence::store_and_commit`]): a commit that
    /// failed transiently there left its changeset buffered, and its caller
    /// was told so. Such a backend is flushed and its ledger read like one
    /// that buffers every store, or this would answer for a record that is
    /// still only in the buffer, or that a later failure dropped from it.
    ///
    /// A flush that succeeds proves nothing by itself: one that failed for
    /// good just before it left nothing buffered, so it succeeds over records
    /// that never reached disk, and one that fails transiently holds what it
    /// took until it puts it back, so a flush made meanwhile succeeds over a
    /// buffer those records are not in. A failure is recorded only after the
    /// backend returns, so until then it shows nowhere but in the count of
    /// flushes in progress. The flush here is therefore trusted only when,
    /// after it, no flush is in progress, none has failed for good, and none
    /// has failed at all since this call began, read in that order: every
    /// flush that began before then has recorded its outcome. All are read
    /// from the ledger of the wallet's id, which counts every flush of its
    /// records, whoever made it: this handle, a handle of a wallet removed or
    /// created under the id since, or the manager. One that fails later
    /// takes only what was buffered after this one wrote, so what this call
    /// answers for stays durable, and what the caller does on the strength of
    /// it needs no lock held with the flush.
    ///
    /// While another flush is in progress, or when one failed during the call
    /// and put back what it took, the answer is
    /// [`NotDurable::OtherFlushUnsettled`], also for a call made from inside
    /// a backend's own flush: the caller asks again later, and that call's
    /// flush writes what was put back.
    ///
    /// Another flush that returned during the call is refused the same way,
    /// though it succeeded ([`FlushLedger::returned`]): it may have taken an
    /// older state of the records before this call's flush took the newer,
    /// and written it afterwards. That covers the call's own flush and no
    /// more. Nothing stores those records again, so a later call finds them
    /// in no buffer, and answers for whatever was written last: there it
    /// rests on the backend writing what flushes took in the order they took
    /// it, which the persistence contract requires
    /// (`PlatformWalletPersistence::flush`).
    ///
    /// This stays worked out from the ledger on every backend that keeps a
    /// buffer, one that commits a store in one step included
    /// ([`PlatformWalletPersistence::store_and_commit`]). That step binds
    /// one changeset to one commit, and this call has no changeset: it asks
    /// whether anything stored earlier, by any handle, was dropped since,
    /// which is a question about the past that no single commit answers. A
    /// step that fails is counted in the ledger, so it is seen here.
    #[cfg(feature = "shielded")]
    pub(crate) fn flush_durably(&self) -> Result<(), NotDurable> {
        // A store that is the commit, on a backend that keeps nothing else,
        // left nothing to flush or to lose: a flush of such a backend only
        // notifies the host, and its failing takes nothing back.
        if !keeps_a_buffer(self.inner.as_ref()) {
            return Ok(());
        }
        if self.buffered_writes_lost() {
            return Err(NotDurable::Lost);
        }
        let failed_before = self.flush_ledger.failed();
        let returned_before = self.flush_ledger.returned();
        self.flush().map_err(NotDurable::Flush)?;
        if self.flush_ledger.in_flight() != 0 {
            return Err(NotDurable::OtherFlushUnsettled);
        }
        if self.buffered_writes_lost() {
            return Err(NotDurable::Lost);
        }
        if self.flush_ledger.failed() != failed_before {
            return Err(NotDurable::OtherFlushUnsettled);
        }
        // The flush above is one that returned since the call began. Any
        // other succeeded, as none failed: it may have written an older
        // state of the records after that flush wrote the newer.
        if self.flush_ledger.returned() != returned_before + 1 {
            return Err(NotDurable::OtherFlushUnsettled);
        }
        Ok(())
    }

    /// Whether a flush of the records under this wallet's id failed for good
    /// since this handle was made (anything but a transient failure, which
    /// keeps the buffer), whoever made the flush: what the wallet tracked
    /// before it may not be on disk, though it is in memory. For the rest of
    /// the handle's life no journal record is discharged on the strength of a
    /// tracked request: no read-back is noted
    /// ([`Self::note_read_back_once_durable`]), and a restoration leaves the
    /// records it finds settled for a later load, which tracks what it reads
    /// from disk.
    #[cfg(feature = "shielded")]
    pub(crate) fn buffered_writes_lost(&self) -> bool {
        self.losses() != self.flush_losses_at_start
    }

    /// Note that a page this wallet read proved DashPay's private contact
    /// request store held `size` entries.
    pub(crate) fn note_private_store_size_proved(&self, size: u64) {
        self.private_store_size_proved
            .fetch_max(size, Ordering::SeqCst);
    }

    /// The largest size of DashPay's private contact request store a page
    /// this wallet read proved, in this process, 0 before any. The store only
    /// grows and every size is read from a proof, so the store holds at least
    /// this many entries from then on: a read reaches its end only once its
    /// cursor reaches it, whatever a later page from a node behind shows.
    pub(crate) fn private_store_size_proved(&self) -> u64 {
        self.private_store_size_proved.load(Ordering::SeqCst)
    }

    pub(crate) fn store_commits_inline(&self) -> bool {
        self.inner.store_commits_inline()
    }

    /// Whether the backend can hold a durable DashPay backfill record (see
    /// [`persists_backfill_coverage`](crate::changeset::persists_backfill_coverage)).
    pub(crate) fn persists_backfill_coverage(&self) -> bool {
        crate::changeset::persists_backfill_coverage(self.inner.as_ref())
    }

    /// Feature-specific persistence contracts exposed by the backend.
    pub(crate) fn persistence_capabilities(&self) -> PersistenceCapabilities {
        self.inner.persistence_capabilities()
    }

    /// Whether the backend stores and restores the private contact request
    /// sync's scan state
    /// ([`PersistenceCapabilities::PRIVATE_CONTACT_REQUEST_SCAN_STATE`]).
    pub(crate) fn persists_private_contact_scan_state(&self) -> bool {
        self.persistence_capabilities()
            .contains(PersistenceCapabilities::PRIVATE_CONTACT_SCAN_STATE_RESTART)
    }

    pub(crate) fn load(&self) -> Result<ClientStartState, PersistenceError> {
        self.inner.load()
    }

    /// Look up a single core transaction record by `txid`. Used by the
    /// asset-lock proof flow to recover chainlocked records that the
    /// in-memory map evicted (see
    /// [`PlatformWalletPersistence::get_core_tx_record`]).
    pub(crate) fn get_core_tx_record(
        &self,
        txid: &Txid,
    ) -> Result<Option<TransactionRecord>, PersistenceError> {
        self.inner.get_core_tx_record(self.wallet_id, txid)
    }

    /// [`Self::get_core_tx_record`] with the shared transient-as-miss policy.
    ///
    /// A busy store is indistinguishable in outcome from "the row is not
    /// readable right now", and every caller here already retries a miss on its
    /// next pass, so a transient failure collapses to `Ok(None)`. A permanent
    /// one stays an `Err`: it will not fix itself, so swallowing it would
    /// repeat the same doomed work forever with no signal. Use
    /// [`Self::get_core_tx_record`] directly to tell the two apart.
    ///
    /// Every caller is a poll loop, a per-txid sweep, or a one-shot read whose
    /// miss leads into such a loop (the ChainLock upgrade), so the collapse is
    /// counted into `tally` and reported once when that wait, sweep or read
    /// ends, rather than logged per call.
    pub(crate) fn get_core_tx_record_or_transient_miss(
        &self,
        txid: &Txid,
        tally: &mut TransientMissTally,
    ) -> Result<Option<TransactionRecord>, PersistenceError> {
        match self.get_core_tx_record(txid) {
            Err(e) if e.is_transient() => {
                tally.misses += 1;
                tracing::trace!(
                    %txid,
                    error = %e,
                    "Core tx-record read hit a transient backend failure; reading as a miss"
                );
                Ok(None)
            }
            other => other,
        }
    }

    /// Enumerate the persisted Core transaction ids scoped to this
    /// wallet, tagged with the host's wallet-funded verdict. Used by
    /// DashPay sent-payment reconstruction to fetch the full records
    /// via [`Self::get_core_tx_record`]. `None` means the backend does
    /// not support wallet-scoped enumeration (never "empty table").
    pub(crate) fn list_wallet_core_txids(
        &self,
    ) -> Result<Option<Vec<crate::changeset::traits::ListedCoreTxid>>, PersistenceError> {
        self.inner.list_wallet_core_txids(self.wallet_id)
    }

    /// Look up the persisted DPNS marketplace row for
    /// `(wallet_identity_id, normalized_label)` within this wallet.
    ///
    /// The durable fallback the DPNS marketplace sync pass uses to
    /// recover a departed name's `document_id` once a process restart
    /// has left the session-scoped in-memory map empty — see
    /// [`PlatformWalletPersistence::get_dpns_name_state`] for the full
    /// contract. `Ok(None)` means the backend does not index DPNS rows
    /// by label (or holds no such row); it is not an error.
    pub(crate) fn get_dpns_name_state(
        &self,
        wallet_identity_id: &Identifier,
        normalized_label: &str,
    ) -> Result<Option<DpnsNameStateEntry>, PersistenceError> {
        self.inner
            .get_dpns_name_state(self.wallet_id, wallet_identity_id, normalized_label)
    }
}

/// No-op platform persistence for standalone wallets.
pub struct NoPlatformPersistence;

impl PlatformWalletPersistence for NoPlatformPersistence {
    /// Nothing is ever written, so nothing survives a restart. (Redundant
    /// with the trait's fail-closed default — kept explicit as documentation.)
    fn persists_durably(&self) -> bool {
        false
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

    fn load(&self) -> Result<ClientStartState, PersistenceError> {
        Ok(ClientStartState::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::changeset::{ContactChangeSet, PersistenceErrorKind, SentContactRequestKey};
    use crate::wallet::identity::types::dashpay::{ContactRequest, EstablishedContact};
    use std::sync::{mpsc, OnceLock};
    use std::time::Duration;

    /// `persists_durably` is a fail-closed security capability: an
    /// implementation that does NOT explicitly attest durability must read as
    /// non-durable, so a backend author who forgets the override gets a loud
    /// "requires durable persistence" refusal from the invitation flow
    /// instead of being silently trusted with a re-exportable bearer key.
    #[test]
    fn durability_attestation_defaults_to_fail_closed() {
        struct BareMinimum;
        impl PlatformWalletPersistence for BareMinimum {
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
            fn load(&self) -> Result<ClientStartState, PersistenceError> {
                Ok(ClientStartState::default())
            }
        }
        assert!(!BareMinimum.persists_durably());
        assert!(!NoPlatformPersistence.persists_durably());
    }

    /// A recipient assignment writes what the backend buffered before it, so
    /// one that fails for good is counted as a lost flush: a caller waiting on
    /// the ledger for records it buffered earlier learns they went with it,
    /// instead of trusting a later flush over an emptied buffer.
    #[test]
    fn should_count_an_assignment_that_failed_for_good_as_a_lost_flush() {
        struct DropsOnAssign;
        impl PlatformWalletPersistence for DropsOnAssign {
            fn assign_asset_lock_identity_recipient(
                &self,
                _wallet_id: WalletId,
                _out_point: OutPoint,
                _identity: Identifier,
            ) -> Result<(), PersistenceError> {
                Err(PersistenceError::backend("the database is corrupt"))
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
            fn load(&self) -> Result<ClientStartState, PersistenceError> {
                Ok(ClientStartState::default())
            }
        }

        let persister = WalletPersister::new([0; 32], Arc::new(DropsOnAssign) as _);
        let lost_before = persister.flush_ledger.lost();

        assert!(persister
            .assign_asset_lock_identity_recipient(OutPoint::null(), Identifier::from([1; 32]))
            .is_err());

        assert_eq!(persister.flush_ledger.lost(), lost_before + 1);
        assert_eq!(persister.flush_ledger.in_flight(), 0);
    }

    /// A store is not answered for on the strength of a flush that succeeded
    /// after another took what the store buffered and failed for good,
    /// whether that one ran between the store and its flush or during the
    /// flush: the record never reached disk. A later store is answered for,
    /// as the flush that failed dropped nothing of it.
    #[test]
    fn should_not_answer_for_a_store_another_flush_dropped() {
        /// Buffers what is stored, and is overtaken once: a flush through the
        /// same persister, as one from another thread, takes the buffer and
        /// fails for good, right after the first store buffers
        /// (`in_store`) or as the first flush begins.
        struct Overtaken {
            persister: OnceLock<WalletPersister>,
            in_store: bool,
            overtaken: AtomicBool,
            fails_for_good: AtomicBool,
            buffered: AtomicU64,
            durable: AtomicU64,
        }
        impl Overtaken {
            fn overtake(&self) {
                if self.overtaken.swap(true, Ordering::SeqCst) {
                    return;
                }
                self.fails_for_good.store(true, Ordering::SeqCst);
                let overtaking = self.persister.get().expect("set").flush();
                self.fails_for_good.store(false, Ordering::SeqCst);
                assert!(overtaking.is_err_and(|error| !error.is_transient()));
            }
        }
        impl PlatformWalletPersistence for Overtaken {
            fn store(
                &self,
                _wallet_id: WalletId,
                _changeset: PlatformWalletChangeSet,
            ) -> Result<(), PersistenceError> {
                self.buffered.fetch_add(1, Ordering::SeqCst);
                if self.in_store {
                    self.overtake();
                }
                Ok(())
            }
            fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
                if self.fails_for_good.load(Ordering::SeqCst) {
                    self.buffered.store(0, Ordering::SeqCst);
                    return Err(PersistenceError::backend("the database is corrupt"));
                }
                self.overtake();
                self.durable
                    .fetch_add(self.buffered.swap(0, Ordering::SeqCst), Ordering::SeqCst);
                Ok(())
            }
            fn load(&self) -> Result<ClientStartState, PersistenceError> {
                Ok(ClientStartState::default())
            }
        }

        for in_store in [true, false] {
            let backend = Arc::new(Overtaken {
                persister: OnceLock::new(),
                in_store,
                overtaken: AtomicBool::new(false),
                fails_for_good: AtomicBool::new(false),
                buffered: AtomicU64::new(0),
                durable: AtomicU64::new(0),
            });
            let persister = WalletPersister::new([0; 32], Arc::clone(&backend) as _);
            let _ = backend.persister.set(persister.clone());

            let stored = persister.store_durably(PlatformWalletChangeSet::default());
            assert!(
                matches!(stored, Err(NotDurable::Lost)),
                "overtaken in its store: {in_store}, {stored:?}"
            );
            assert_eq!(backend.durable.load(Ordering::SeqCst), 0, "never on disk");

            let later = persister.store_durably(PlatformWalletChangeSet::default());
            assert!(
                later.is_ok(),
                "overtaken in its store: {in_store}, {later:?}"
            );
            assert_eq!(backend.durable.load(Ordering::SeqCst), 1, "on disk");
        }
    }

    /// Buffers what is stored and writes it at a flush. Every plain store is
    /// followed by a commit the host makes on the backend itself, which the
    /// wallet's ledger does not see and which fails for good: the buffer is
    /// dropped. A store committed in one step leaves that commit nothing to
    /// come between: it runs once the step is over, and finds the buffer
    /// empty.
    #[derive(Default)]
    struct HostCommitsBesideTheStep {
        buffered: AtomicU64,
        written: AtomicU64,
        host_commits: AtomicU64,
    }

    impl HostCommitsBesideTheStep {
        /// A commit the host makes on the backend, failing for good.
        fn host_commit_drops_the_buffer(&self) {
            self.buffered.store(0, Ordering::SeqCst);
            self.host_commits.fetch_add(1, Ordering::SeqCst);
        }
    }

    impl PlatformWalletPersistence for HostCommitsBesideTheStep {
        fn store(
            &self,
            _wallet_id: WalletId,
            _changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            self.buffered.fetch_add(1, Ordering::SeqCst);
            self.host_commit_drops_the_buffer();
            Ok(())
        }
        fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
            self.written
                .fetch_add(self.buffered.swap(0, Ordering::SeqCst), Ordering::SeqCst);
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
            // Staged and committed, with nothing in between.
            self.buffered.fetch_add(1, Ordering::SeqCst);
            self.written
                .fetch_add(self.buffered.swap(0, Ordering::SeqCst), Ordering::SeqCst);
            // The host's commit, held out until the step is over.
            self.host_commit_drops_the_buffer();
            Ok(())
        }
        fn load(&self) -> Result<ClientStartState, PersistenceError> {
            Ok(ClientStartState::default())
        }
    }

    /// A store made durable is committed in one step where the backend
    /// offers it, so a commit the host makes on the backend itself cannot
    /// take the record between its store and its flush: stored and flushed
    /// in two calls, the record is dropped in between by a drain the ledger
    /// never sees and this backend does not count, the flush succeeds over
    /// an empty buffer, and the store is answered for with nothing on disk.
    #[test]
    fn should_commit_a_store_in_one_step_so_a_commit_by_the_host_cannot_drop_it() {
        let backend = Arc::new(HostCommitsBesideTheStep::default());
        let persister = WalletPersister::new([0; 32], Arc::clone(&backend) as _);

        let stored = persister.store_durably(PlatformWalletChangeSet::default());

        assert_eq!(
            backend.host_commits.load(Ordering::SeqCst),
            1,
            "the host did commit"
        );
        assert!(stored.is_ok(), "{stored:?}");
        assert_eq!(
            backend.written.load(Ordering::SeqCst),
            1,
            "what was answered for is on disk"
        );
    }

    /// Keeps the last state stored and writes it at a flush, each store
    /// being a newer state of one record. Its flushes do not wait for one
    /// another: the first stalls with what it took, and writes it after the
    /// second has written. A store committed in one step does wait, as one
    /// step with every other drain: the flush in progress writes what it
    /// took and returns before the step stages anything.
    struct StalledFlushHoldsAnOlderState {
        stores: AtomicU64,
        flushes: AtomicU64,
        buffer: std::sync::Mutex<Option<u64>>,
        disk: std::sync::Mutex<Option<u64>>,
        entered: std::sync::Mutex<mpsc::Sender<()>>,
        release: std::sync::Mutex<mpsc::Receiver<()>>,
        let_go: std::sync::Mutex<mpsc::Sender<()>>,
        first_returned: std::sync::Mutex<mpsc::Receiver<()>>,
    }

    impl StalledFlushHoldsAnOlderState {
        fn write(&self, taken: Option<u64>) {
            if let Some(state) = taken {
                *self.disk.lock().expect("disk") = Some(state);
            }
        }

        fn on_disk(&self) -> Option<u64> {
            *self.disk.lock().expect("disk")
        }

        /// Let the stalled flush write what it took, and wait for it to
        /// return.
        fn wait_for_the_stalled_flush(&self) {
            self.let_go
                .lock()
                .expect("let go")
                .send(())
                .expect("the first flush waits");
            self.first_returned
                .lock()
                .expect("first returned")
                .recv_timeout(Duration::from_secs(10))
                .expect("the first flush returns");
        }

        /// A persister over this backend whose first flush, made on the
        /// thread returned, has taken the first state stored and stalled.
        fn with_the_first_state_in_a_stalled_flush() -> (
            Arc<Self>,
            WalletPersister,
            std::thread::JoinHandle<Result<(), PersistenceError>>,
        ) {
            let (entered, has_entered) = mpsc::channel();
            let (let_go, release) = mpsc::channel();
            let (returned, first_returned) = mpsc::channel();
            let backend = Arc::new(Self {
                stores: AtomicU64::new(0),
                flushes: AtomicU64::new(0),
                buffer: std::sync::Mutex::new(None),
                disk: std::sync::Mutex::new(None),
                entered: std::sync::Mutex::new(entered),
                release: std::sync::Mutex::new(release),
                let_go: std::sync::Mutex::new(let_go),
                first_returned: std::sync::Mutex::new(first_returned),
            });
            let persister = WalletPersister::new([0; 32], Arc::clone(&backend) as _);
            persister
                .store(PlatformWalletChangeSet::default())
                .expect("the first state is buffered");
            let stalled = {
                let persister = persister.clone();
                std::thread::spawn(move || {
                    let flushed = persister.flush();
                    returned.send(()).expect("the store made durable waits");
                    flushed
                })
            };
            has_entered
                .recv_timeout(Duration::from_secs(10))
                .expect("the first flush took the first state");
            (backend, persister, stalled)
        }
    }

    impl PlatformWalletPersistence for StalledFlushHoldsAnOlderState {
        fn store(
            &self,
            _wallet_id: WalletId,
            _changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            let state = self.stores.fetch_add(1, Ordering::SeqCst) + 1;
            *self.buffer.lock().expect("buffer") = Some(state);
            Ok(())
        }

        fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
            let taken = self.buffer.lock().expect("buffer").take();
            match self.flushes.fetch_add(1, Ordering::SeqCst) {
                0 => {
                    self.entered
                        .lock()
                        .expect("entered")
                        .send(())
                        .expect("the test waits");
                    self.release
                        .lock()
                        .expect("release")
                        .recv_timeout(Duration::from_secs(10))
                        .expect("released");
                    self.write(taken);
                }
                1 => {
                    self.write(taken);
                    self.wait_for_the_stalled_flush();
                }
                _ => self.write(taken),
            }
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
            // One step with every other drain: the one in progress is over,
            // what it took written, before anything is staged.
            self.wait_for_the_stalled_flush();
            let state = self.stores.fetch_add(1, Ordering::SeqCst) + 1;
            let buffered = self.buffer.lock().expect("buffer").take();
            self.write(buffered);
            self.write(Some(state));
            Ok(())
        }

        fn load(&self) -> Result<ClientStartState, PersistenceError> {
            Ok(ClientStartState::default())
        }
    }

    /// A store made durable is committed in one step where the backend
    /// offers it, so a flush that took an older state of the record before
    /// the store cannot write it over the newer: stored and flushed in two
    /// calls, the older state lands last behind a flush that succeeded and
    /// returned, and all the wallet can do is refuse to answer, for the
    /// record to be stored again. In one step that flush is over before
    /// anything is staged, and the store is answered for with the newer
    /// state on disk.
    #[test]
    fn should_commit_a_store_in_one_step_so_a_flush_in_progress_cannot_write_an_older_state_last() {
        let (backend, persister, stalled) =
            StalledFlushHoldsAnOlderState::with_the_first_state_in_a_stalled_flush();

        let stored = persister.store_durably(PlatformWalletChangeSet::default());

        assert!(stalled.join().expect("the first flush returns").is_ok());
        assert!(stored.is_ok(), "{stored:?}");
        assert_eq!(
            backend.on_disk(),
            Some(2),
            "the state answered for is the one on disk"
        );
    }

    /// A withdrawal made durable is committed in one step too where the
    /// backend offers it. Stored and flushed in two calls it is refused
    /// while a flush that took the record is in progress, as that flush may
    /// write the record after it. The step is one step with every other
    /// drain: that flush is over, what it took written, before the
    /// withdrawal is staged, so the withdrawal is what is written last and
    /// is answered for.
    #[test]
    fn should_commit_a_withdrawal_in_one_step_so_a_flush_in_progress_cannot_write_the_record_after_it(
    ) {
        let (backend, persister, stalled) =
            StalledFlushHoldsAnOlderState::with_the_first_state_in_a_stalled_flush();

        let withdrawn = persister.withdraw_durably(PlatformWalletChangeSet::default());

        assert!(stalled.join().expect("the first flush returns").is_ok());
        assert!(withdrawn.is_ok(), "{withdrawn:?}");
        assert_eq!(
            backend.on_disk(),
            Some(2),
            "the withdrawal is what was written last"
        );
    }

    /// Commits a store in one step, answering with what `answer` holds, and
    /// counts the plain stores and flushes made beside it.
    #[derive(Default)]
    struct InOneStep {
        answer: std::sync::Mutex<Option<StoreAndCommitError>>,
        stored_plainly: AtomicU64,
        flushed: AtomicU64,
        marker_kept: AtomicBool,
    }

    impl InOneStep {
        fn answering(answer: StoreAndCommitError) -> Arc<Self> {
            let backend = Arc::new(Self::default());
            *backend.answer.lock().expect("answer") = Some(answer);
            backend
        }
    }

    impl PlatformWalletPersistence for InOneStep {
        fn store(
            &self,
            _wallet_id: WalletId,
            _changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            self.stored_plainly.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
        fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
            self.flushed.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
        fn offers_store_and_commit(&self) -> bool {
            true
        }
        fn store_and_commit(
            &self,
            _wallet_id: WalletId,
            changeset: PlatformWalletChangeSet,
        ) -> Result<(), StoreAndCommitError> {
            let marker_kept = changeset.contacts.is_some_and(|contacts| {
                contacts
                    .established
                    .values()
                    .any(|established| established.external_account_reference.is_some())
            });
            self.marker_kept.store(marker_kept, Ordering::SeqCst);
            self.answer
                .lock()
                .expect("answer")
                .take()
                .map_or(Ok(()), Err)
        }
        fn load(&self) -> Result<ClientStartState, PersistenceError> {
            Ok(ClientStartState::default())
        }
    }

    fn busy() -> PersistenceError {
        PersistenceError::backend_with_kind(PersistenceErrorKind::Transient, "the disk is busy")
    }

    /// The answer of a store committed in one step is the answer, and the
    /// two ways it fails stay apart for the callers that report them
    /// differently. A changeset refused before it was staged is the store's
    /// failure: nothing of it is buffered or written, so a transient one
    /// stays transient and may be issued again, though the backend does not
    /// attest that for its plain `store`. A commit that failed is the
    /// flush's failure, with the backend's own kind: buffered still when
    /// transient, dropped otherwise. The backend is neither stored to nor
    /// flushed beside the step.
    #[test]
    fn should_report_a_refused_changeset_as_a_failed_store_and_a_failed_commit_as_a_failed_flush() {
        let refused = InOneStep::answering(StoreAndCommitError::NotStaged(busy()));
        let persister = WalletPersister::new([0; 32], Arc::clone(&refused) as _);
        let not_durable = persister
            .store_durably(PlatformWalletChangeSet::default())
            .expect_err("refused");
        assert!(
            matches!(&not_durable, NotDurable::Store(error) if error.is_transient()),
            "{not_durable:?}"
        );
        assert!(
            !refused.store_transient_is_reissuable(),
            "the backend says nothing of its plain store"
        );
        let reported = persister.classify_not_durable(not_durable);
        assert!(
            matches!(&reported, PlatformWalletError::PersisterStore(error) if error.is_transient()),
            "nothing was staged, so it may be issued again: {reported:?}"
        );

        for (failure, kind) in [
            (busy(), PersistenceErrorKind::Transient),
            (
                PersistenceError::backend("the database is corrupt"),
                PersistenceErrorKind::Fatal,
            ),
        ] {
            let failed = InOneStep::answering(StoreAndCommitError::NotCommitted(failure));
            let persister = WalletPersister::new([0; 32], Arc::clone(&failed) as _);
            let not_durable = persister
                .store_durably(PlatformWalletChangeSet::default())
                .expect_err("not committed");
            assert!(
                matches!(&not_durable, NotDurable::Flush(error) if error.kind() == Some(kind)),
                "{not_durable:?}"
            );
            let reported = persister.classify_not_durable(not_durable);
            assert!(
                matches!(&reported, PlatformWalletError::PersisterFlush(error) if error.kind() == Some(kind)),
                "{reported:?}"
            );
            assert_eq!(failed.stored_plainly.load(Ordering::SeqCst), 0);
            assert_eq!(failed.flushed.load(Ordering::SeqCst), 0);
        }
    }

    /// A withdrawal committed in one step is answered for as any store made
    /// durable that way is: with the backend's answer, and with the backend
    /// neither stored to nor flushed beside the step.
    #[test]
    fn should_take_the_backends_answer_for_a_withdrawal_committed_in_one_step() {
        let backend = InOneStep::answering(StoreAndCommitError::NotCommitted(busy()));
        let persister = WalletPersister::new([0; 32], Arc::clone(&backend) as _);

        let not_committed = persister.withdraw_durably(PlatformWalletChangeSet::default());
        assert!(
            matches!(&not_committed, Err(NotDurable::Flush(error)) if error.is_transient()),
            "{not_committed:?}"
        );
        let committed = persister.withdraw_durably(PlatformWalletChangeSet::default());
        assert!(committed.is_ok(), "{committed:?}");

        assert_eq!(backend.stored_plainly.load(Ordering::SeqCst), 0);
        assert_eq!(backend.flushed.load(Ordering::SeqCst), 0);
    }

    /// A plain store that fails transiently on a backend without the step is
    /// still narrowed to a failure for good, unless the backend attests that
    /// it kept nothing of the changeset: only the step's own answer says
    /// that by itself.
    #[test]
    fn should_still_narrow_a_transient_store_failure_of_a_backend_without_the_step() {
        struct RefusesItsStore;
        impl PlatformWalletPersistence for RefusesItsStore {
            fn store(
                &self,
                _wallet_id: WalletId,
                _changeset: PlatformWalletChangeSet,
            ) -> Result<(), PersistenceError> {
                Err(busy())
            }
            fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
                Ok(())
            }
            fn load(&self) -> Result<ClientStartState, PersistenceError> {
                Ok(ClientStartState::default())
            }
        }

        let persister = WalletPersister::new([0; 32], Arc::new(RefusesItsStore) as _);
        let not_durable = persister
            .store_durably(PlatformWalletChangeSet::default())
            .expect_err("refused");
        assert!(
            matches!(&not_durable, NotDurable::Store(error) if error.is_transient()),
            "{not_durable:?}"
        );
        let reported = persister.classify_not_durable(not_durable);
        assert!(
            matches!(&reported, PlatformWalletError::PersisterStore(error) if !error.is_transient()),
            "{reported:?}"
        );
    }

    /// A commit made in one step that fails for good dropped the wallet's
    /// whole buffer, what other handles of the wallet id stored plainly
    /// included. They learn of it as they learn of a flush that failed for
    /// good: the step is counted in the wallet id's ledger, so a marker
    /// noted as stored is forgotten, and nothing is answered for as durable
    /// on the strength of what was tracked before.
    #[test]
    fn should_tell_the_wallet_ids_other_handles_of_a_commit_in_one_step_that_dropped_the_buffer() {
        let backend = InOneStep::answering(StoreAndCommitError::NotCommitted(
            PersistenceError::backend("the database is corrupt"),
        ));
        let wallet_id: WalletId = [4; 32];
        let persister = WalletPersister::new(wallet_id, Arc::clone(&backend) as _);
        let other = WalletPersister::new(wallet_id, Arc::clone(&backend) as _);
        let owner = Identifier::from([9; 32]);
        other.note_private_marker_stored(owner, Some(true));

        let stored = persister.store_durably(PlatformWalletChangeSet::default());

        assert!(
            matches!(&stored, Err(NotDurable::Flush(error)) if !error.is_transient()),
            "{stored:?}"
        );
        assert_eq!(
            other.private_marker_stored(&owner),
            None,
            "its record may have been in the buffer the commit dropped"
        );
        #[cfg(feature = "shielded")]
        assert!(
            matches!(other.flush_durably(), Err(NotDurable::Lost)),
            "what the other handle stored before may not be on disk"
        );
    }

    /// A backend whose store is the commit and that commits a store in one
    /// step, as SQLite does when it commits every store. Each commit it
    /// attempts, in a store, a step or a flush, fails with the next of
    /// `failures` while any is left: a transient one keeps what is buffered
    /// for a later commit, as the step's contract has it, and any other
    /// drops it. `during_the_step` runs inside the next step, once its
    /// changeset is staged. It counts what it drops, and reports the count
    /// only while `reports_its_drops` is set.
    #[cfg(feature = "shielded")]
    #[derive(Default)]
    struct InlineInOneStep {
        buffered: AtomicU64,
        written: AtomicU64,
        dropped: AtomicU64,
        reports_its_drops: AtomicBool,
        failures: std::sync::Mutex<std::collections::VecDeque<PersistenceError>>,
        during_the_step: std::sync::Mutex<Option<Box<dyn FnOnce() + Send>>>,
    }

    #[cfg(feature = "shielded")]
    impl InlineInOneStep {
        fn failing_with(failures: impl IntoIterator<Item = PersistenceError>) -> Arc<Self> {
            let backend = Arc::new(Self::default());
            backend.failures.lock().expect("failures").extend(failures);
            backend
        }

        fn commit(&self) -> Result<(), PersistenceError> {
            let Some(failure) = self.failures.lock().expect("failures").pop_front() else {
                self.written
                    .fetch_add(self.buffered.swap(0, Ordering::SeqCst), Ordering::SeqCst);
                return Ok(());
            };
            if !failure.is_transient() {
                self.buffered.store(0, Ordering::SeqCst);
                self.dropped.fetch_add(1, Ordering::SeqCst);
            }
            Err(failure)
        }

        /// How many records are in the buffer, and how many on disk.
        fn buffered_and_written(&self) -> (u64, u64) {
            (
                self.buffered.load(Ordering::SeqCst),
                self.written.load(Ordering::SeqCst),
            )
        }
    }

    #[cfg(feature = "shielded")]
    impl PlatformWalletPersistence for InlineInOneStep {
        fn store_commits_inline(&self) -> bool {
            true
        }
        fn store(
            &self,
            _wallet_id: WalletId,
            _changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            self.buffered.fetch_add(1, Ordering::SeqCst);
            self.commit()
        }
        fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
            self.commit()
        }
        fn offers_store_and_commit(&self) -> bool {
            true
        }
        fn store_and_commit(
            &self,
            _wallet_id: WalletId,
            _changeset: PlatformWalletChangeSet,
        ) -> Result<(), StoreAndCommitError> {
            self.buffered.fetch_add(1, Ordering::SeqCst);
            let during_the_step = self.during_the_step.lock().expect("during the step").take();
            if let Some(run) = during_the_step {
                run();
            }
            self.commit().map_err(StoreAndCommitError::NotCommitted)
        }
        fn buffered_writes_dropped(&self, _wallet_id: WalletId) -> Option<u64> {
            self.reports_its_drops
                .load(Ordering::SeqCst)
                .then(|| self.dropped.load(Ordering::SeqCst))
        }
        fn load(&self) -> Result<ClientStartState, PersistenceError> {
            Ok(ClientStartState::default())
        }
    }

    /// On a backend whose store is the commit, a store committed in one step
    /// that fails transiently is still buffered, and its caller is told so.
    /// Asked whether what was stored is durable, the wallet flushes such a
    /// backend as it does one that buffers every store: answering at once,
    /// on the strength of its `store` being the commit, would call durable a
    /// record that is in the buffer only.
    #[cfg(feature = "shielded")]
    #[test]
    fn should_flush_an_inline_backend_that_commits_in_one_step_before_calling_its_records_durable()
    {
        let backend = InlineInOneStep::failing_with([busy(), busy()]);
        let wallet_id: WalletId = [7; 32];
        let persister = WalletPersister::new(wallet_id, Arc::clone(&backend) as _);
        let other = WalletPersister::new(wallet_id, Arc::clone(&backend) as _);

        let stored = persister.store_durably(PlatformWalletChangeSet::default());
        assert!(
            matches!(&stored, Err(NotDurable::Flush(error)) if error.is_transient()),
            "{stored:?}"
        );
        assert_eq!(backend.buffered_and_written(), (1, 0), "left in the buffer");

        let while_the_disk_is_busy = other.flush_durably();
        assert!(
            matches!(
                &while_the_disk_is_busy,
                Err(NotDurable::Flush(error)) if error.is_transient()
            ),
            "the record is in the buffer only: {while_the_disk_is_busy:?}"
        );
        assert_eq!(backend.buffered_and_written(), (1, 0), "still buffered");

        let once_it_is_not = other.flush_durably();
        assert!(once_it_is_not.is_ok(), "{once_it_is_not:?}");
        assert_eq!(backend.buffered_and_written(), (0, 1), "on disk");
    }

    /// What such a commit left buffered is gone once a flush of that backend
    /// fails for good, whoever made the flush. The flush is counted as a
    /// loss, though the backend's store is the commit, so no handle of the
    /// wallet id calls durable what was stored before it.
    #[cfg(feature = "shielded")]
    #[test]
    fn should_report_the_loss_of_what_a_commit_in_one_step_left_buffered_on_an_inline_backend() {
        let backend = InlineInOneStep::failing_with([
            busy(),
            PersistenceError::backend("the database is corrupt"),
        ]);
        let wallet_id: WalletId = [8; 32];
        let persister = WalletPersister::new(wallet_id, Arc::clone(&backend) as _);
        let other = WalletPersister::new(wallet_id, Arc::clone(&backend) as _);

        let stored = persister.store_durably(PlatformWalletChangeSet::default());
        assert!(
            matches!(&stored, Err(NotDurable::Flush(error)) if error.is_transient()),
            "{stored:?}"
        );
        let flushed = persister.flush();
        assert!(
            flushed.as_ref().is_err_and(|error| !error.is_transient()),
            "{flushed:?}"
        );
        assert_eq!(backend.buffered_and_written(), (0, 0), "dropped");

        for handle in [&other, &persister] {
            let durable = handle.flush_durably();
            assert!(
                matches!(durable, Err(NotDurable::Lost)),
                "the record never reached the disk: {durable:?}"
            );
        }
    }

    /// What such a commit left buffered also goes with a plain `store` that
    /// fails for good: on a backend whose store is the commit that store
    /// commits the same buffer, and it is counted in no ledger. Only the
    /// backend's own count tells of it
    /// ([`PlatformWalletPersistence::buffered_writes_dropped`]), and it is
    /// read for every backend that keeps a buffer, its store being the
    /// commit or not. So no handle of the wallet id calls the record
    /// durable, and a marker noted as stored before the drop is forgotten.
    #[cfg(feature = "shielded")]
    #[test]
    fn should_read_the_drops_an_inline_backend_that_commits_in_one_step_reports() {
        let backend = InlineInOneStep::failing_with([
            busy(),
            PersistenceError::backend("the database is corrupt"),
        ]);
        backend.reports_its_drops.store(true, Ordering::SeqCst);
        let wallet_id: WalletId = [10; 32];
        let persister = WalletPersister::new(wallet_id, Arc::clone(&backend) as _);
        let other = WalletPersister::new(wallet_id, Arc::clone(&backend) as _);
        let owner = Identifier::from([3; 32]);
        other.note_private_marker_stored(owner, Some(true));

        let stored = persister.store_durably(PlatformWalletChangeSet::default());
        assert!(
            matches!(&stored, Err(NotDurable::Flush(error)) if error.is_transient()),
            "{stored:?}"
        );
        // Made on the backend itself, as every plain store is: no ledger
        // counts it.
        let plain = backend.store(wallet_id, PlatformWalletChangeSet::default());
        assert!(
            plain.as_ref().is_err_and(|error| !error.is_transient()),
            "{plain:?}"
        );
        assert_eq!(backend.buffered_and_written(), (0, 0), "dropped");

        assert_eq!(
            other.private_marker_stored(&owner),
            None,
            "its record may have been in the buffer that store dropped"
        );
        for handle in [&other, &persister] {
            let durable = handle.flush_durably();
            assert!(
                matches!(durable, Err(NotDurable::Lost)),
                "the record never reached the disk: {durable:?}"
            );
        }
    }

    /// While a store is being committed in one step on such a backend, its
    /// outcome is not known, and nothing is answered for: the commit may yet
    /// fail and drop what the wallet stored before.
    #[cfg(feature = "shielded")]
    #[test]
    fn should_not_answer_for_an_inline_backend_while_a_commit_in_one_step_is_in_progress() {
        let backend = Arc::new(InlineInOneStep::default());
        let wallet_id: WalletId = [9; 32];
        let persister = WalletPersister::new(wallet_id, Arc::clone(&backend) as _);
        let other = WalletPersister::new(wallet_id, Arc::clone(&backend) as _);
        let (asked, answered) = mpsc::channel();
        *backend.during_the_step.lock().expect("during the step") = Some(Box::new(move || {
            asked
                .send(other.flush_durably())
                .expect("the test reads the answer");
        }));

        let stored = persister.store_durably(PlatformWalletChangeSet::default());

        assert!(stored.is_ok(), "{stored:?}");
        let during_the_step = answered.try_recv().expect("asked during the step");
        assert!(
            matches!(during_the_step, Err(NotDurable::OtherFlushUnsettled)),
            "{during_the_step:?}"
        );
    }

    /// A store committed in one step is handed to the backend as a plain
    /// store is: on a backend that cannot keep an established contact's
    /// outbound-account marker durably, without it.
    #[test]
    fn should_commit_a_contact_in_one_step_without_a_marker_the_backend_cannot_keep() {
        let backend = Arc::new(InOneStep::default());
        let persister = WalletPersister::new([0; 32], Arc::clone(&backend) as _);
        let (owner, contact) = (Identifier::from([1; 32]), Identifier::from([2; 32]));
        let request = |sender, recipient| {
            ContactRequest::new(sender, recipient, 1, 2, 7, vec![7u8; 96], 100_000, 0)
        };
        let mut established =
            EstablishedContact::new(contact, request(owner, contact), request(contact, owner));
        established.external_account_reference = Some(7);
        let mut contacts = ContactChangeSet::default();
        contacts.established.insert(
            SentContactRequestKey {
                owner_id: owner,
                recipient_id: contact,
            },
            established,
        );

        assert!(!persister.persists_outbound_account_markers());
        persister
            .store_durably(contacts.into())
            .expect("committed in one step");

        assert!(
            !backend.marker_kept.load(Ordering::SeqCst),
            "the marker did not reach the backend"
        );
    }

    /// A marker noted as stored is forgotten once a flush of the wallet id's
    /// records fails for good, whoever made it: the store it was noted for
    /// may still have been in the buffer that flush dropped, so the next
    /// comparison stores the marker again. A flush that fails keeping what it
    /// took forgets nothing, and a marker noted after the loss is kept.
    #[test]
    fn should_forget_stored_markers_once_any_flush_of_the_wallet_id_fails_for_good() {
        /// A buffering backend whose flush fails, for good or not.
        struct Failing {
            for_good: AtomicBool,
        }
        impl PlatformWalletPersistence for Failing {
            fn store(
                &self,
                _wallet_id: WalletId,
                _changeset: PlatformWalletChangeSet,
            ) -> Result<(), PersistenceError> {
                Ok(())
            }
            fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
                Err(if self.for_good.load(Ordering::SeqCst) {
                    PersistenceError::backend("the database is corrupt")
                } else {
                    PersistenceError::backend_with_kind(
                        PersistenceErrorKind::Transient,
                        "the database is locked",
                    )
                })
            }
            fn load(&self) -> Result<ClientStartState, PersistenceError> {
                Ok(ClientStartState::default())
            }
        }

        let backend = Arc::new(Failing {
            for_good: AtomicBool::new(false),
        });
        let wallet_id: WalletId = [3; 32];
        let persister = WalletPersister::new(wallet_id, Arc::clone(&backend) as _);
        // A handle of a wallet removed, or created since, under the id.
        let other = WalletPersister::new(wallet_id, Arc::clone(&backend) as _);
        let owner = Identifier::from([9; 32]);

        persister.note_private_marker_stored(owner, Some(true));
        assert!(other.flush().is_err_and(|error| error.is_transient()));
        assert_eq!(
            persister.private_marker_stored(&owner),
            Some(true),
            "a flush that kept what it took dropped no store"
        );

        backend.for_good.store(true, Ordering::SeqCst);
        assert!(other.flush().is_err_and(|error| !error.is_transient()));
        assert_eq!(
            persister.private_marker_stored(&owner),
            None,
            "not known stored once the buffer was dropped"
        );

        persister.note_private_marker_stored(owner, Some(false));
        assert_eq!(
            persister.private_marker_stored(&owner),
            Some(false),
            "noted after the loss"
        );
    }

    /// A marker is not noted as stored when a flush of the wallet id's
    /// records failed for good between its record's store and the note: the
    /// record may have been in the buffer that flush dropped, so the next
    /// comparison stores it again. One whose store nothing overtook is noted.
    #[test]
    fn should_not_note_a_marker_whose_store_another_flush_dropped() {
        /// Buffers what is stored, and once `other` is set every store is
        /// overtaken: a flush through that handle, as one from another
        /// thread, takes the buffer and fails for good before the store
        /// returns.
        struct Overtaken {
            other: OnceLock<WalletPersister>,
            fails_for_good: AtomicBool,
        }
        impl PlatformWalletPersistence for Overtaken {
            fn store(
                &self,
                _wallet_id: WalletId,
                _changeset: PlatformWalletChangeSet,
            ) -> Result<(), PersistenceError> {
                if let Some(other) = self.other.get() {
                    self.fails_for_good.store(true, Ordering::SeqCst);
                    let overtaking = other.flush();
                    self.fails_for_good.store(false, Ordering::SeqCst);
                    assert!(overtaking.is_err_and(|error| !error.is_transient()));
                }
                Ok(())
            }
            fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
                if self.fails_for_good.load(Ordering::SeqCst) {
                    return Err(PersistenceError::backend("the database is corrupt"));
                }
                Ok(())
            }
            fn load(&self) -> Result<ClientStartState, PersistenceError> {
                Ok(ClientStartState::default())
            }
        }

        let backend = Arc::new(Overtaken {
            other: OnceLock::new(),
            fails_for_good: AtomicBool::new(false),
        });
        let persister = WalletPersister::new([0; 32], Arc::clone(&backend) as _);
        let owner = Identifier::from([9; 32]);

        persister
            .store_private_marker_record(owner, true, PlatformWalletChangeSet::default())
            .expect("stored");
        assert_eq!(
            persister.private_marker_stored(&owner),
            Some(true),
            "nothing overtook its store"
        );

        let _ = backend.other.set(persister.clone());
        persister
            .store_private_marker_record(owner, false, PlatformWalletChangeSet::default())
            .expect("the store itself succeeds");
        assert_eq!(
            persister.private_marker_stored(&owner),
            None,
            "its record may have been in the buffer the other flush dropped"
        );
    }

    /// Buffers what is stored. Its first flush finds the buffer empty:
    /// another flush, made through the same persister as one from another
    /// thread, took it before, and fails transiently while the first runs,
    /// putting back what it took. Later flushes write what is buffered.
    #[derive(Default)]
    struct PutBack {
        persister: OnceLock<WalletPersister>,
        overlapped: AtomicBool,
        other_fails: AtomicBool,
        buffered: AtomicU64,
        durable: AtomicU64,
    }

    impl PutBack {
        /// A persister over a new one, which its first flush overlaps
        /// through.
        fn persister() -> (Arc<Self>, WalletPersister) {
            let backend = Arc::new(Self::default());
            let persister = WalletPersister::new([0; 32], Arc::clone(&backend) as _);
            let _ = backend.persister.set(persister.clone());
            (backend, persister)
        }
    }

    impl PlatformWalletPersistence for PutBack {
        fn store(
            &self,
            _wallet_id: WalletId,
            _changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            self.buffered.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
        fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
            if self.other_fails.load(Ordering::SeqCst) {
                // The other flush: what it took is buffered again.
                return Err(PersistenceError::backend_with_kind(
                    PersistenceErrorKind::Transient,
                    "the disk is busy",
                ));
            }
            if !self.overlapped.swap(true, Ordering::SeqCst) {
                self.other_fails.store(true, Ordering::SeqCst);
                let other = self.persister.get().expect("set").flush();
                self.other_fails.store(false, Ordering::SeqCst);
                assert!(other.is_err_and(|error| error.is_transient()));
                // The buffer was empty when this flush took it: nothing
                // written.
                return Ok(());
            }
            self.durable
                .fetch_add(self.buffered.swap(0, Ordering::SeqCst), Ordering::SeqCst);
            Ok(())
        }
        fn load(&self) -> Result<ClientStartState, PersistenceError> {
            Ok(ClientStartState::default())
        }
    }

    /// A store is not answered for on the strength of a flush that succeeded
    /// while another held what the store buffered, and then failed
    /// transiently and put it back: the record is buffered still. The next
    /// flush writes it, and a store made then is answered for.
    #[test]
    fn should_not_answer_for_a_store_another_flush_took_and_put_back() {
        let (backend, persister) = PutBack::persister();

        let stored = persister.store_durably(PlatformWalletChangeSet::default());
        assert!(
            matches!(stored, Err(NotDurable::OtherFlushUnsettled)),
            "{stored:?}"
        );
        assert_eq!(backend.durable.load(Ordering::SeqCst), 0, "buffered still");

        let later = persister.store_durably(PlatformWalletChangeSet::default());
        assert!(later.is_ok(), "{later:?}");
        assert_eq!(backend.durable.load(Ordering::SeqCst), 2, "both on disk");
    }

    /// A read-back is not noted on the strength of a flush that succeeded
    /// while another held the record it rests on, and then failed transiently
    /// and put it back: the record is buffered still. The notice is kept,
    /// and noted by the next call, whose flush writes the record.
    #[cfg(feature = "shielded")]
    #[test]
    fn should_note_no_read_back_when_a_flush_puts_back_what_it_took_during_its_own() {
        let (backend, persister) = PutBack::persister();
        persister
            .store(PlatformWalletChangeSet::default())
            .expect("buffered");

        persister.note_read_back_once_durable([[1; 32]]);
        assert!(
            !persister.has_read_back_private_entries(),
            "its flush wrote nothing of what the other put back"
        );
        assert!(persister.read_back_awaiting_flush(&[1; 32]), "kept");
        assert!(!persister.buffered_writes_lost(), "nothing was dropped");
        assert_eq!(backend.durable.load(Ordering::SeqCst), 0, "buffered still");

        persister.note_read_back_once_durable([]);
        assert!(persister.has_read_back_private_entries(), "noted");
        assert_eq!(backend.durable.load(Ordering::SeqCst), 1, "on disk");
    }

    /// A backend whose store is the commit answers for what was stored
    /// whatever its flush notification answers: a flush of it that fails took
    /// nothing back, so the handle does not count its records as lost, and a
    /// read-back is still noted.
    #[cfg(feature = "shielded")]
    #[test]
    fn should_answer_for_an_inline_backend_whose_flush_notification_fails() {
        struct InlineFailing;
        impl PlatformWalletPersistence for InlineFailing {
            fn store_commits_inline(&self) -> bool {
                true
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

        let persister = WalletPersister::new([0; 32], Arc::new(InlineFailing) as _);
        assert!(persister.flush().is_err(), "the notification fails");

        assert!(
            !persister.buffered_writes_lost(),
            "nothing was buffered to lose"
        );
        assert!(persister.flush_durably().is_ok());
        persister.note_read_back_once_durable([[1; 32]]);
        assert!(
            persister.has_read_back_private_entries(),
            "the read-back is noted"
        );
    }

    /// A store is not answered for while another flush of the wallet's
    /// records is still inside the backend: that one may have taken what the
    /// store buffered, and its failure is recorded only once it returns.
    #[test]
    fn should_not_answer_for_a_store_while_another_flush_has_not_returned() {
        struct Stalled {
            flushes: AtomicU64,
            entered: std::sync::Mutex<mpsc::Sender<()>>,
            release: std::sync::Mutex<mpsc::Receiver<()>>,
        }
        impl PlatformWalletPersistence for Stalled {
            fn store(
                &self,
                _wallet_id: WalletId,
                _changeset: PlatformWalletChangeSet,
            ) -> Result<(), PersistenceError> {
                Ok(())
            }
            fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
                if self.flushes.fetch_add(1, Ordering::SeqCst) != 0 {
                    return Ok(());
                }
                // The first flush has not returned yet.
                self.entered
                    .lock()
                    .expect("entered")
                    .send(())
                    .expect("the test waits");
                self.release
                    .lock()
                    .expect("release")
                    .recv_timeout(Duration::from_secs(10))
                    .expect("released");
                Ok(())
            }
            fn load(&self) -> Result<ClientStartState, PersistenceError> {
                Ok(ClientStartState::default())
            }
        }

        let (entered, has_entered) = mpsc::channel();
        let (release, released) = mpsc::channel();
        let backend = Arc::new(Stalled {
            flushes: AtomicU64::new(0),
            entered: std::sync::Mutex::new(entered),
            release: std::sync::Mutex::new(released),
        });
        let persister = WalletPersister::new([0; 32], backend as _);
        let stalled = {
            let persister = persister.clone();
            std::thread::spawn(move || persister.flush())
        };
        has_entered
            .recv_timeout(Duration::from_secs(10))
            .expect("the first flush is inside the backend");

        let meanwhile = persister.store_durably(PlatformWalletChangeSet::default());
        release.send(()).expect("the first flush waits");
        assert!(stalled.join().expect("the first flush returns").is_ok());

        assert!(
            matches!(meanwhile, Err(NotDurable::OtherFlushUnsettled)),
            "{meanwhile:?}"
        );
        let after = persister.store_durably(PlatformWalletChangeSet::default());
        assert!(after.is_ok(), "{after:?}");
    }

    /// A store whose own flush fails is not answered for with that failure
    /// while another flush of the wallet's records is inside the backend:
    /// that one may hold what the store buffered, and may drop it later, so
    /// a flush that failed over nothing says nothing of the record. The
    /// answer is the one a caller stores its record again on. With no other
    /// flush across the call, the failure is the answer, as it was.
    #[test]
    fn should_not_answer_with_its_own_flush_failure_while_another_flush_has_not_returned() {
        /// The first flush stalls inside the backend. Every later one
        /// fails, for good or not as set.
        struct StalledThenFailing {
            flushes: AtomicU64,
            fails_for_good: AtomicBool,
            entered: std::sync::Mutex<mpsc::Sender<()>>,
            release: std::sync::Mutex<mpsc::Receiver<()>>,
        }
        impl PlatformWalletPersistence for StalledThenFailing {
            fn store(
                &self,
                _wallet_id: WalletId,
                _changeset: PlatformWalletChangeSet,
            ) -> Result<(), PersistenceError> {
                Ok(())
            }
            fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
                if self.flushes.fetch_add(1, Ordering::SeqCst) != 0 {
                    return Err(if self.fails_for_good.load(Ordering::SeqCst) {
                        PersistenceError::backend("the database is corrupt")
                    } else {
                        PersistenceError::backend_with_kind(
                            PersistenceErrorKind::Transient,
                            "the disk is busy",
                        )
                    });
                }
                // The first flush has not returned yet.
                self.entered
                    .lock()
                    .expect("entered")
                    .send(())
                    .expect("the test waits");
                self.release
                    .lock()
                    .expect("release")
                    .recv_timeout(Duration::from_secs(10))
                    .expect("released");
                Ok(())
            }
            fn load(&self) -> Result<ClientStartState, PersistenceError> {
                Ok(ClientStartState::default())
            }
        }

        let (entered, has_entered) = mpsc::channel();
        let (release, released) = mpsc::channel();
        let backend = Arc::new(StalledThenFailing {
            flushes: AtomicU64::new(0),
            fails_for_good: AtomicBool::new(false),
            entered: std::sync::Mutex::new(entered),
            release: std::sync::Mutex::new(released),
        });
        let persister = WalletPersister::new([0; 32], Arc::clone(&backend) as _);
        let stalled = {
            let persister = persister.clone();
            std::thread::spawn(move || persister.flush())
        };
        has_entered
            .recv_timeout(Duration::from_secs(10))
            .expect("the first flush is inside the backend");

        let kept = persister.store_durably(PlatformWalletChangeSet::default());
        backend.fails_for_good.store(true, Ordering::SeqCst);
        let dropped = persister.store_durably(PlatformWalletChangeSet::default());
        release.send(()).expect("the first flush waits");
        assert!(stalled.join().expect("the first flush returns").is_ok());

        assert!(
            matches!(kept, Err(NotDurable::OtherFlushUnsettled)),
            "its flush failed transiently: {kept:?}"
        );
        assert!(
            matches!(dropped, Err(NotDurable::Lost)),
            "its flush failed for good: {dropped:?}"
        );

        let alone = persister.store_durably(PlatformWalletChangeSet::default());
        assert!(
            matches!(&alone, Err(NotDurable::Flush(error)) if !error.is_transient()),
            "{alone:?}"
        );
        backend.fails_for_good.store(false, Ordering::SeqCst);
        let alone = persister.store_durably(PlatformWalletChangeSet::default());
        assert!(
            matches!(&alone, Err(NotDurable::Flush(error)) if error.is_transient()),
            "{alone:?}"
        );
    }

    /// A store whose own flush fails is not answered for with that failure
    /// when another flush of the wallet's records returned while it ran:
    /// that one may have taken what the store buffered, so its outcome is
    /// the record's. One that failed for good, or a flush of the call's own
    /// that did, is a loss; one that succeeded leaves it unsettled.
    #[test]
    fn should_not_answer_with_its_own_flush_failure_when_another_flush_returned_during_it() {
        /// Every flush fails, for good or not as set, after another, made
        /// through the same persister as one from another thread, returned
        /// inside it: that one succeeds, or fails for good, as set.
        #[derive(Default)]
        struct CrossedThenFailing {
            persister: OnceLock<WalletPersister>,
            crossing: AtomicBool,
            other_fails_for_good: AtomicBool,
            fails_for_good: AtomicBool,
        }
        impl PlatformWalletPersistence for CrossedThenFailing {
            fn store(
                &self,
                _wallet_id: WalletId,
                _changeset: PlatformWalletChangeSet,
            ) -> Result<(), PersistenceError> {
                Ok(())
            }
            fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
                if self.crossing.load(Ordering::SeqCst) {
                    // The other flush.
                    return if self.other_fails_for_good.load(Ordering::SeqCst) {
                        Err(PersistenceError::backend("the database is corrupt"))
                    } else {
                        Ok(())
                    };
                }
                self.crossing.store(true, Ordering::SeqCst);
                let _ = self.persister.get().expect("set").flush();
                self.crossing.store(false, Ordering::SeqCst);
                Err(if self.fails_for_good.load(Ordering::SeqCst) {
                    PersistenceError::backend("the database is corrupt")
                } else {
                    PersistenceError::backend_with_kind(
                        PersistenceErrorKind::Transient,
                        "the disk is busy",
                    )
                })
            }
            fn load(&self) -> Result<ClientStartState, PersistenceError> {
                Ok(ClientStartState::default())
            }
        }

        let backend = Arc::new(CrossedThenFailing::default());
        let persister = WalletPersister::new([0; 32], Arc::clone(&backend) as _);
        let _ = backend.persister.set(persister.clone());

        let other_succeeded = persister.store_durably(PlatformWalletChangeSet::default());
        assert!(
            matches!(other_succeeded, Err(NotDurable::OtherFlushUnsettled)),
            "{other_succeeded:?}"
        );

        backend.other_fails_for_good.store(true, Ordering::SeqCst);
        let other_dropped = persister.store_durably(PlatformWalletChangeSet::default());
        assert!(
            matches!(other_dropped, Err(NotDurable::Lost)),
            "{other_dropped:?}"
        );

        backend.other_fails_for_good.store(false, Ordering::SeqCst);
        backend.fails_for_good.store(true, Ordering::SeqCst);
        let own_dropped = persister.store_durably(PlatformWalletChangeSet::default());
        assert!(
            matches!(own_dropped, Err(NotDurable::Lost)),
            "{own_dropped:?}"
        );
    }

    /// A withdrawal is not answered for while another flush of the wallet's
    /// records is inside the backend when it is stored: that flush may have
    /// taken the record being withdrawn, and may write it after the
    /// withdrawal. Nothing is flushed for it. A plain store made durable in
    /// the same place is flushed, and not answered for either: the other
    /// flush returned, without failing, after this one wrote.
    #[test]
    fn should_not_answer_for_a_withdrawal_stored_while_another_flush_has_not_returned() {
        /// The first flush stalls inside the backend. The second lets it
        /// go once it has written, and returns only after the first has: so
        /// no flush is in progress, and none failed, when the second returns.
        struct LateFirstFlush {
            flushes: AtomicU64,
            entered: std::sync::Mutex<mpsc::Sender<()>>,
            release: std::sync::Mutex<mpsc::Receiver<()>>,
            let_go: std::sync::Mutex<mpsc::Sender<()>>,
            first_returned: std::sync::Mutex<mpsc::Receiver<()>>,
        }
        impl PlatformWalletPersistence for LateFirstFlush {
            fn store(
                &self,
                _wallet_id: WalletId,
                _changeset: PlatformWalletChangeSet,
            ) -> Result<(), PersistenceError> {
                Ok(())
            }
            fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
                if self.flushes.fetch_add(1, Ordering::SeqCst) != 0 {
                    self.let_go
                        .lock()
                        .expect("let go")
                        .send(())
                        .expect("the first flush waits");
                    self.first_returned
                        .lock()
                        .expect("first returned")
                        .recv_timeout(Duration::from_secs(10))
                        .expect("the first flush returns");
                    return Ok(());
                }
                self.entered
                    .lock()
                    .expect("entered")
                    .send(())
                    .expect("the test waits");
                self.release
                    .lock()
                    .expect("release")
                    .recv_timeout(Duration::from_secs(10))
                    .expect("released");
                Ok(())
            }
            fn load(&self) -> Result<ClientStartState, PersistenceError> {
                Ok(ClientStartState::default())
            }
        }

        let (entered, has_entered) = mpsc::channel();
        let (let_go, release) = mpsc::channel();
        let (returned, first_returned) = mpsc::channel();
        let backend = Arc::new(LateFirstFlush {
            flushes: AtomicU64::new(0),
            entered: std::sync::Mutex::new(entered),
            release: std::sync::Mutex::new(release),
            let_go: std::sync::Mutex::new(let_go),
            first_returned: std::sync::Mutex::new(first_returned),
        });
        let persister = WalletPersister::new([0; 32], Arc::clone(&backend) as _);
        let stalled = {
            let persister = persister.clone();
            std::thread::spawn(move || {
                let flushed = persister.flush();
                returned.send(()).expect("the second flush waits");
                flushed
            })
        };
        has_entered
            .recv_timeout(Duration::from_secs(10))
            .expect("the first flush is inside the backend");

        let withdrawn = persister.withdraw_durably(PlatformWalletChangeSet::default());
        assert!(
            matches!(withdrawn, Err(NotDurable::OtherFlushUnsettled)),
            "{withdrawn:?}"
        );
        assert_eq!(
            backend.flushes.load(Ordering::SeqCst),
            1,
            "nothing was flushed for the withdrawal"
        );

        let stored = persister.store_durably(PlatformWalletChangeSet::default());
        assert!(stalled.join().expect("the first flush returns").is_ok());
        assert!(
            matches!(stored, Err(NotDurable::OtherFlushUnsettled)),
            "{stored:?}"
        );
        assert_eq!(
            backend.flushes.load(Ordering::SeqCst),
            2,
            "the store was flushed"
        );
    }

    /// Keeps the last state stored and writes it at a flush, each store being
    /// a newer state of one record. The first flush stalls with what it
    /// took. The second writes, then lets the first write, and returns only
    /// after the first has.
    struct WritesTheOlderStateLast {
        stores: AtomicU64,
        flushes: AtomicU64,
        buffer: std::sync::Mutex<Option<u64>>,
        disk: std::sync::Mutex<Option<u64>>,
        entered: std::sync::Mutex<mpsc::Sender<()>>,
        release: std::sync::Mutex<mpsc::Receiver<()>>,
        let_go: std::sync::Mutex<mpsc::Sender<()>>,
        first_returned: std::sync::Mutex<mpsc::Receiver<()>>,
    }

    impl WritesTheOlderStateLast {
        fn write(&self, taken: Option<u64>) {
            if let Some(state) = taken {
                *self.disk.lock().expect("disk") = Some(state);
            }
        }

        fn on_disk(&self) -> Option<u64> {
            *self.disk.lock().expect("disk")
        }

        /// A persister over this backend whose first flush, made on the
        /// thread returned, has taken the first state stored and stalled.
        fn with_the_first_state_in_a_stalled_flush() -> (
            Arc<Self>,
            WalletPersister,
            std::thread::JoinHandle<Result<(), PersistenceError>>,
        ) {
            let (entered, has_entered) = mpsc::channel();
            let (let_go, release) = mpsc::channel();
            let (returned, first_returned) = mpsc::channel();
            let backend = Arc::new(Self {
                stores: AtomicU64::new(0),
                flushes: AtomicU64::new(0),
                buffer: std::sync::Mutex::new(None),
                disk: std::sync::Mutex::new(None),
                entered: std::sync::Mutex::new(entered),
                release: std::sync::Mutex::new(release),
                let_go: std::sync::Mutex::new(let_go),
                first_returned: std::sync::Mutex::new(first_returned),
            });
            let persister = WalletPersister::new([0; 32], Arc::clone(&backend) as _);
            persister
                .store(PlatformWalletChangeSet::default())
                .expect("the first state is buffered");
            let stalled = {
                let persister = persister.clone();
                std::thread::spawn(move || {
                    let flushed = persister.flush();
                    returned.send(()).expect("the second flush waits");
                    flushed
                })
            };
            has_entered
                .recv_timeout(Duration::from_secs(10))
                .expect("the first flush took the first state");
            (backend, persister, stalled)
        }
    }

    impl PlatformWalletPersistence for WritesTheOlderStateLast {
        fn store(
            &self,
            _wallet_id: WalletId,
            _changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            let state = self.stores.fetch_add(1, Ordering::SeqCst) + 1;
            *self.buffer.lock().expect("buffer") = Some(state);
            Ok(())
        }

        fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
            let taken = self.buffer.lock().expect("buffer").take();
            match self.flushes.fetch_add(1, Ordering::SeqCst) {
                0 => {
                    self.entered
                        .lock()
                        .expect("entered")
                        .send(())
                        .expect("the test waits");
                    self.release
                        .lock()
                        .expect("release")
                        .recv_timeout(Duration::from_secs(10))
                        .expect("released");
                    self.write(taken);
                }
                1 => {
                    self.write(taken);
                    self.let_go
                        .lock()
                        .expect("let go")
                        .send(())
                        .expect("the first flush waits");
                    self.first_returned
                        .lock()
                        .expect("first returned")
                        .recv_timeout(Duration::from_secs(10))
                        .expect("the first flush returns");
                }
                _ => self.write(taken),
            }
            Ok(())
        }

        fn load(&self) -> Result<ClientStartState, PersistenceError> {
            Ok(ClientStartState::default())
        }
    }

    /// A store is not answered for when another flush of the wallet's
    /// records returned while it was made durable, though none is in
    /// progress afterwards and none failed. Here a flush that took an older
    /// state of the record before the store writes it after this call's
    /// flush wrote the newer, and returns first: a restart would find the
    /// older state. A store made durable afterwards, with no flush about, is
    /// answered for, and its state is the one on disk.
    #[test]
    fn should_not_answer_for_a_store_when_a_flush_returned_while_it_was_made_durable() {
        let (backend, persister, stalled) =
            WritesTheOlderStateLast::with_the_first_state_in_a_stalled_flush();

        let stored = persister.store_durably(PlatformWalletChangeSet::default());

        assert!(stalled.join().expect("the first flush returns").is_ok());
        assert_eq!(
            backend.on_disk(),
            Some(1),
            "the older state was written last"
        );
        assert!(
            matches!(stored, Err(NotDurable::OtherFlushUnsettled)),
            "{stored:?}"
        );

        let again = persister.store_durably(PlatformWalletChangeSet::default());
        assert!(again.is_ok(), "{again:?}");
        assert_eq!(backend.on_disk(), Some(3), "the state stored last");
    }

    /// The wallet's records are not answered for as durable when another
    /// flush returned during the call, though none is in progress afterwards
    /// and none failed: the same flush as above, holding the older state,
    /// writes it after this call's flush wrote the newer.
    #[cfg(feature = "shielded")]
    #[test]
    fn should_not_answer_for_the_wallets_records_when_a_flush_returned_during_the_call() {
        let (backend, persister, stalled) =
            WritesTheOlderStateLast::with_the_first_state_in_a_stalled_flush();
        persister
            .store(PlatformWalletChangeSet::default())
            .expect("the newer state is buffered");

        let flushed = persister.flush_durably();

        assert!(stalled.join().expect("the first flush returns").is_ok());
        assert_eq!(
            backend.on_disk(),
            Some(1),
            "the older state was written last"
        );
        assert!(
            matches!(flushed, Err(NotDurable::OtherFlushUnsettled)),
            "{flushed:?}"
        );
    }

    /// Buffers what is stored and writes it at a flush. While
    /// `host_commit_fails` is set, each store is followed by a commit the
    /// host makes on the backend, which fails for good: the buffer is
    /// dropped, and the drop counted. No flush the wallet makes fails.
    struct DrainedByItsHost {
        buffered: AtomicU64,
        written: AtomicU64,
        dropped: AtomicU64,
        host_commit_fails: AtomicBool,
    }

    impl DrainedByItsHost {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                buffered: AtomicU64::new(0),
                written: AtomicU64::new(0),
                dropped: AtomicU64::new(0),
                host_commit_fails: AtomicBool::new(true),
            })
        }

        /// A commit the host makes on the backend, failing for good.
        fn host_commit_drops_the_buffer(&self) {
            self.buffered.store(0, Ordering::SeqCst);
            self.dropped.fetch_add(1, Ordering::SeqCst);
        }
    }

    impl PlatformWalletPersistence for DrainedByItsHost {
        fn store(
            &self,
            _wallet_id: WalletId,
            _changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            self.buffered.fetch_add(1, Ordering::SeqCst);
            if self.host_commit_fails.load(Ordering::SeqCst) {
                self.host_commit_drops_the_buffer();
            }
            Ok(())
        }

        fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
            let taken = self.buffered.swap(0, Ordering::SeqCst);
            self.written.fetch_add(taken, Ordering::SeqCst);
            Ok(())
        }

        fn buffered_writes_dropped(&self, _wallet_id: WalletId) -> Option<u64> {
            Some(self.dropped.load(Ordering::SeqCst))
        }

        fn load(&self) -> Result<ClientStartState, PersistenceError> {
            Ok(ClientStartState::default())
        }
    }

    /// A store is not answered for when the backend dropped what it had
    /// buffered in a drain the ledger does not see, here a commit the host
    /// made on the backend itself: the wallet's flush then succeeds over an
    /// empty buffer, no flush the wallet made failed, and only the backend's
    /// own count says what happened
    /// ([`PlatformWalletPersistence::buffered_writes_dropped`]). A store
    /// made durable once the host's commits succeed is answered for.
    #[test]
    fn should_not_answer_for_a_store_the_backend_dropped_in_a_drain_of_its_own() {
        let backend = DrainedByItsHost::new();
        let persister = WalletPersister::new([0; 32], Arc::clone(&backend) as _);

        let stored = persister.store_durably(PlatformWalletChangeSet::default());

        assert!(matches!(stored, Err(NotDurable::Lost)), "{stored:?}");
        assert_eq!(
            backend.written.load(Ordering::SeqCst),
            0,
            "nothing reached the disk"
        );
        #[cfg(feature = "shielded")]
        assert!(
            matches!(persister.flush_durably(), Err(NotDurable::Lost)),
            "what the handle stored before may not be on disk either"
        );

        backend.host_commit_fails.store(false, Ordering::SeqCst);
        let again = persister.store_durably(PlatformWalletChangeSet::default());
        assert!(again.is_ok(), "{again:?}");
        assert_eq!(backend.written.load(Ordering::SeqCst), 1);
    }

    /// A marker noted as stored is forgotten once the backend reports a drop
    /// the ledger did not see: not noted at all when the drop came between
    /// its store and the note, and forgotten at the next read when it came
    /// later. So the note never keeps a record from being stored again after
    /// a drain that may have dropped it.
    #[test]
    fn should_forget_a_stored_marker_once_the_backend_reports_a_drop() {
        let backend = DrainedByItsHost::new();
        let persister = WalletPersister::new([0; 32], Arc::clone(&backend) as _);
        let owner = Identifier::from([7; 32]);

        persister
            .store_private_marker_record(owner, true, PlatformWalletChangeSet::default())
            .expect("the store itself succeeds");
        assert_eq!(
            persister.private_marker_stored(&owner),
            None,
            "its record was in the buffer the host's commit dropped"
        );

        backend.host_commit_fails.store(false, Ordering::SeqCst);
        persister
            .store_private_marker_record(owner, true, PlatformWalletChangeSet::default())
            .expect("stored");
        assert_eq!(persister.private_marker_stored(&owner), Some(true));

        backend.host_commit_drops_the_buffer();
        assert_eq!(
            persister.private_marker_stored(&owner),
            None,
            "forgotten at the next read, so the next comparison stores it again"
        );
    }

    /// A withdrawal is not answered for when a flush of the wallet's records
    /// returned while it was stored, though none is in progress afterwards
    /// and none failed. Here a flush that took the record before the
    /// withdrawal was stored writes it after another flush wrote the
    /// withdrawal, and both return before the ledger is read: the record is
    /// on disk, and the buffer is empty. Nothing is flushed for the
    /// withdrawal. One stored afterwards, with no flush about, is answered
    /// for, and is behind the record on disk.
    #[test]
    fn should_not_answer_for_a_withdrawal_when_a_flush_returned_while_it_was_stored() {
        #[derive(Clone, Copy, Debug, PartialEq)]
        enum Written {
            Record,
            Withdrawal,
        }
        fn signal(sender: &std::sync::Mutex<mpsc::Sender<()>>) {
            sender
                .lock()
                .expect("sender")
                .send(())
                .expect("the other side waits");
        }
        fn wait(receiver: &std::sync::Mutex<mpsc::Receiver<()>>) {
            receiver
                .lock()
                .expect("receiver")
                .recv_timeout(Duration::from_secs(10))
                .expect("the other side signals");
        }
        /// Buffers what is stored and writes it at a flush, the first store
        /// being the record and every later one a withdrawal of it. The
        /// first flush stalls with what it took. The second store, before it
        /// returns, has another thread flush what it buffered, then lets the
        /// first flush write, and waits for each to return.
        struct WritesTheRecordLast {
            stores: AtomicU64,
            flushes: AtomicU64,
            buffer: std::sync::Mutex<Vec<Written>>,
            disk: std::sync::Mutex<Vec<Written>>,
            first_entered: std::sync::Mutex<mpsc::Sender<()>>,
            first_released: std::sync::Mutex<mpsc::Receiver<()>>,
            release_first: std::sync::Mutex<mpsc::Sender<()>>,
            first_returned: std::sync::Mutex<mpsc::Receiver<()>>,
            start_second: std::sync::Mutex<mpsc::Sender<()>>,
            second_returned: std::sync::Mutex<mpsc::Receiver<()>>,
        }
        impl WritesTheRecordLast {
            /// Whether a restart would find the record: it was written, and
            /// no withdrawal after it.
            fn record_on_disk(&self) -> bool {
                self.disk.lock().expect("disk").last() == Some(&Written::Record)
            }
        }
        impl PlatformWalletPersistence for WritesTheRecordLast {
            fn store(
                &self,
                _wallet_id: WalletId,
                _changeset: PlatformWalletChangeSet,
            ) -> Result<(), PersistenceError> {
                let nth = self.stores.fetch_add(1, Ordering::SeqCst);
                let stored = if nth == 0 {
                    Written::Record
                } else {
                    Written::Withdrawal
                };
                self.buffer.lock().expect("buffer").push(stored);
                if nth == 1 {
                    signal(&self.start_second);
                    wait(&self.second_returned);
                    signal(&self.release_first);
                    wait(&self.first_returned);
                }
                Ok(())
            }
            fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
                let taken = std::mem::take(&mut *self.buffer.lock().expect("buffer"));
                if self.flushes.fetch_add(1, Ordering::SeqCst) == 0 {
                    signal(&self.first_entered);
                    wait(&self.first_released);
                }
                self.disk.lock().expect("disk").extend(taken);
                Ok(())
            }
            fn load(&self) -> Result<ClientStartState, PersistenceError> {
                Ok(ClientStartState::default())
            }
        }

        let (first_entered, has_entered) = mpsc::channel();
        let (release_first, first_released) = mpsc::channel();
        let (first_done, first_returned) = mpsc::channel();
        let (start_second, second_started) = mpsc::channel();
        let (second_done, second_returned) = mpsc::channel();
        let backend = Arc::new(WritesTheRecordLast {
            stores: AtomicU64::new(0),
            flushes: AtomicU64::new(0),
            buffer: std::sync::Mutex::new(Vec::new()),
            disk: std::sync::Mutex::new(Vec::new()),
            first_entered: std::sync::Mutex::new(first_entered),
            first_released: std::sync::Mutex::new(first_released),
            release_first: std::sync::Mutex::new(release_first),
            first_returned: std::sync::Mutex::new(first_returned),
            start_second: std::sync::Mutex::new(start_second),
            second_returned: std::sync::Mutex::new(second_returned),
        });
        let persister = WalletPersister::new([0; 32], Arc::clone(&backend) as _);
        persister
            .store(PlatformWalletChangeSet::default())
            .expect("the record is buffered");
        let first = {
            let persister = persister.clone();
            std::thread::spawn(move || {
                let flushed = persister.flush();
                first_done.send(()).expect("the store waits");
                flushed
            })
        };
        has_entered
            .recv_timeout(Duration::from_secs(10))
            .expect("the first flush took the record");
        let second = {
            let persister = persister.clone();
            std::thread::spawn(move || {
                second_started
                    .recv_timeout(Duration::from_secs(10))
                    .expect("the withdrawal is stored");
                let flushed = persister.flush();
                second_done.send(()).expect("the store waits");
                flushed
            })
        };

        let withdrawn = persister.withdraw_durably(PlatformWalletChangeSet::default());

        assert!(first.join().expect("the first flush returns").is_ok());
        assert!(second.join().expect("the second flush returns").is_ok());
        assert_eq!(
            *backend.disk.lock().expect("disk"),
            [Written::Withdrawal, Written::Record],
            "the record was written after its withdrawal"
        );
        assert!(backend.record_on_disk());
        assert!(
            matches!(withdrawn, Err(NotDurable::OtherFlushUnsettled)),
            "{withdrawn:?}"
        );
        assert_eq!(
            backend.flushes.load(Ordering::SeqCst),
            2,
            "nothing was flushed for the withdrawal"
        );

        let again = persister.withdraw_durably(PlatformWalletChangeSet::default());
        assert!(again.is_ok(), "{again:?}");
        assert!(
            !backend.record_on_disk(),
            "the later withdrawal is behind the record"
        );
    }

    /// The host's flush runs with no lock of the persister held: a flush that
    /// notes another read-back commitment through the same persister, as a
    /// reentrant backend or another thread would, completes, and nothing a
    /// failed flush was given is lost: the next flush that succeeds notes
    /// them all.
    #[cfg(feature = "shielded")]
    #[test]
    fn should_flush_read_back_notices_without_holding_their_lock() {
        struct Reentrant {
            persister: OnceLock<WalletPersister>,
            flush_fails: AtomicBool,
            reentered: AtomicBool,
        }
        impl PlatformWalletPersistence for Reentrant {
            fn store(
                &self,
                _wallet_id: WalletId,
                _changeset: PlatformWalletChangeSet,
            ) -> Result<(), PersistenceError> {
                Ok(())
            }
            fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
                // The first flush notes another commitment before it ends.
                if !self.reentered.swap(true, Ordering::SeqCst) {
                    if let Some(persister) = self.persister.get() {
                        persister.note_read_back_once_durable([[2; 32]]);
                    }
                }
                if self.flush_fails.load(Ordering::SeqCst) {
                    return Err(PersistenceError::backend_with_kind(
                        PersistenceErrorKind::Transient,
                        "the disk is busy",
                    ));
                }
                Ok(())
            }
            fn load(&self) -> Result<ClientStartState, PersistenceError> {
                Ok(ClientStartState::default())
            }
        }

        let (done, finished) = mpsc::channel();
        std::thread::spawn(move || {
            let backend = Arc::new(Reentrant {
                persister: OnceLock::new(),
                flush_fails: AtomicBool::new(true),
                reentered: AtomicBool::new(false),
            });
            let persister = WalletPersister::new([0; 32], Arc::clone(&backend) as _);
            let _ = backend.persister.set(persister.clone());
            persister.note_read_back_once_durable([[1; 32]]);
            let noted_on_failure = persister.has_read_back_private_entries();
            backend.flush_fails.store(false, Ordering::SeqCst);
            persister.note_read_back_once_durable([]);
            let _ = done.send((noted_on_failure, persister.take_read_back_private_entries()));
        });
        let (noted_on_failure, noted) = finished
            .recv_timeout(Duration::from_secs(10))
            .expect("the flushes complete");
        assert!(!noted_on_failure, "a failed flush notes nothing");
        assert_eq!(noted, BTreeSet::from([[1; 32], [2; 32]]), "nothing lost");
    }

    /// A read-back is not noted on the strength of a flush that succeeded
    /// after another failed for good, also when that failure came while this
    /// call's own flush ran, after it read that nothing was lost: the one
    /// that failed dropped the buffer, so the one here succeeded over records
    /// that never reached disk.
    #[cfg(feature = "shielded")]
    #[test]
    fn should_note_no_read_back_when_a_flush_fails_for_good_during_its_own() {
        struct Overtaken {
            persister: OnceLock<WalletPersister>,
            flushes: AtomicU64,
        }
        impl PlatformWalletPersistence for Overtaken {
            fn store(
                &self,
                _wallet_id: WalletId,
                _changeset: PlatformWalletChangeSet,
            ) -> Result<(), PersistenceError> {
                Ok(())
            }
            fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
                match self.flushes.fetch_add(1, Ordering::SeqCst) {
                    // The flush under test: before it ends, another, as from
                    // another thread, fails for good.
                    0 => {
                        let overtaking = self.persister.get().expect("set").flush();
                        assert!(overtaking.is_err_and(|error| !error.is_transient()));
                        Ok(())
                    }
                    1 => Err(PersistenceError::backend("the database is corrupt")),
                    _ => Ok(()),
                }
            }
            fn load(&self) -> Result<ClientStartState, PersistenceError> {
                Ok(ClientStartState::default())
            }
        }

        let backend = Arc::new(Overtaken {
            persister: OnceLock::new(),
            flushes: AtomicU64::new(0),
        });
        let persister = WalletPersister::new([0; 32], Arc::clone(&backend) as _);
        let _ = backend.persister.set(persister.clone());
        persister.note_read_back_once_durable([[1; 32]]);
        assert!(persister.buffered_writes_lost());
        assert!(
            !persister.has_read_back_private_entries(),
            "its flush succeeded over a buffer the other one dropped"
        );
    }

    /// A read-back is not noted while another flush of the wallet's records
    /// is still inside the backend: that one may have dropped what was
    /// buffered already, and its failure is recorded only once it returns.
    /// The notice is kept, and dropped once the failure shows.
    #[cfg(feature = "shielded")]
    #[test]
    fn should_note_no_read_back_while_another_flush_has_not_returned() {
        struct Stalled {
            flushes: AtomicU64,
            entered: std::sync::Mutex<mpsc::Sender<()>>,
            release: std::sync::Mutex<mpsc::Receiver<()>>,
        }
        impl PlatformWalletPersistence for Stalled {
            fn store(
                &self,
                _wallet_id: WalletId,
                _changeset: PlatformWalletChangeSet,
            ) -> Result<(), PersistenceError> {
                Ok(())
            }
            fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
                if self.flushes.fetch_add(1, Ordering::SeqCst) != 0 {
                    return Ok(());
                }
                // The first flush dropped what was buffered, and has not
                // returned yet.
                self.entered
                    .lock()
                    .expect("entered")
                    .send(())
                    .expect("the test waits");
                self.release
                    .lock()
                    .expect("release")
                    .recv_timeout(Duration::from_secs(10))
                    .expect("released");
                Err(PersistenceError::backend("the database is corrupt"))
            }
            fn load(&self) -> Result<ClientStartState, PersistenceError> {
                Ok(ClientStartState::default())
            }
        }

        let (entered, has_entered) = mpsc::channel();
        let (release, released) = mpsc::channel();
        let backend = Arc::new(Stalled {
            flushes: AtomicU64::new(0),
            entered: std::sync::Mutex::new(entered),
            release: std::sync::Mutex::new(released),
        });
        let persister = WalletPersister::new([0; 32], backend as _);
        let failing = {
            let persister = persister.clone();
            std::thread::spawn(move || persister.flush())
        };
        has_entered
            .recv_timeout(Duration::from_secs(10))
            .expect("the first flush is inside the backend");

        persister.note_read_back_once_durable([[1; 32]]);
        let noted_meanwhile = persister.has_read_back_private_entries();
        let kept = persister.read_back_awaiting_flush(&[1; 32]);
        release.send(()).expect("the first flush waits");
        let failed = failing.join().expect("the first flush returns");

        assert!(failed.is_err_and(|error| !error.is_transient()));
        assert!(!noted_meanwhile, "its own flush proves nothing yet");
        assert!(kept, "kept for a later call");
        persister.note_read_back_once_durable([]);
        assert!(persister.buffered_writes_lost());
        assert!(
            !persister.has_read_back_private_entries(),
            "the buffer was lost"
        );
        assert!(!persister.read_back_awaiting_flush(&[1; 32]));
    }
}
