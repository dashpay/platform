//! Per-wallet persistence handles.
//!
//! Contains:
//! - [`WalletPersister`] — wraps the shared [`PlatformWalletPersistence`] with
//!   a fixed `wallet_id` so callers don't need to pass the ID on every call.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use dashcore::Txid;
use key_wallet::managed_account::transaction_record::TransactionRecord;

use crate::changeset::{
    ClientStartState, DpnsNameStateEntry, FlushLedger, PersistenceCapabilities, PersistenceError,
    PersistenceErrorKind, PlatformWalletChangeSet, PlatformWalletPersistence,
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
    /// The store this call made failed, so nothing was flushed.
    #[error("{0}")]
    Store(PersistenceError),
    /// The flush this call made failed: a transient failure keeps what the
    /// backend buffered, any other drops it.
    #[error("{0}")]
    Flush(PersistenceError),
    /// Another flush of the wallet id's records has not returned, or failed
    /// while the call ran with the backend keeping what it had taken from the
    /// buffer: either may hold records the call answers for, which a later
    /// flush writes. A later call answers.
    #[error(
        "another flush of the wallet's records is in progress, or failed keeping what it had \
         taken, so what was flushed may not be everything stored"
    )]
    OtherFlushUnsettled,
}

impl WalletPersister {
    /// A handle whose flushes no other handle or manager shares: for a
    /// backend this handle alone flushes `wallet_id`'s records on.
    pub fn new(wallet_id: WalletId, inner: Arc<dyn PlatformWalletPersistence>) -> Self {
        Self::with_flush_ledger(wallet_id, inner, Arc::default())
    }

    /// A handle that counts its flushes in `flush_ledger`, the one every
    /// flush of `wallet_id`'s records on `inner` is counted in
    /// ([`FlushLedgers`](crate::changeset::FlushLedgers)).
    pub fn with_flush_ledger(
        wallet_id: WalletId,
        inner: Arc<dyn PlatformWalletPersistence>,
        flush_ledger: Arc<FlushLedger>,
    ) -> Self {
        Self {
            wallet_id,
            inner,
            contact_payment_gate: Arc::new(tokio::sync::Mutex::new(())),
            #[cfg(feature = "shielded")]
            read_back_private_entries: Arc::default(),
            #[cfg(feature = "shielded")]
            unflushed_read_back_private_entries: Arc::default(),
            discharged_private_entries: Arc::default(),
            private_payments_unrestored: Arc::default(),
            stored_private_markers: Arc::default(),
            #[cfg(feature = "shielded")]
            flush_losses_at_start: flush_ledger.lost(),
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

    /// The markers noted as stored, forgotten first when a flush of the
    /// wallet id's records failed for good since they were noted: this
    /// handle's flush, another handle's or the manager's, as the ledger
    /// counts them all. The next comparison then stores each marker again.
    fn stored_markers(&self) -> std::sync::MutexGuard<'_, StoredPrivateMarkers> {
        let mut stored = self
            .stored_private_markers
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let losses = self.flush_ledger.lost();
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
    /// durable ([`Self::flush_durably`]): at once for a backend that commits
    /// inline, after a flush otherwise. Those a transient flush failure left
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
                     is in progress, or failed keeping what it took, are not noted yet; a later \
                     call notes them"
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
    #[cfg(feature = "shielded")]
    pub(crate) fn hold_until_private_payments_restored(&self) {
        self.private_payments_unrestored
            .store(true, Ordering::Release);
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
    pub(crate) fn store(
        &self,
        mut changeset: PlatformWalletChangeSet,
    ) -> Result<(), PersistenceError> {
        if !self.persists_outbound_account_markers() {
            if let Some(contacts) = changeset.contacts.as_mut() {
                for established in contacts.established.values_mut() {
                    established.external_account_reference = None;
                }
            }
        }
        self.inner.store(self.wallet_id, changeset)
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
    /// record buffered for a later flush: `PersisterFlush` of kind
    /// `Transient`, to be tried again.
    pub(crate) fn classify_not_durable(&self, not_durable: NotDurable) -> PlatformWalletError {
        match not_durable {
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

    /// Store `changeset` and answer `Ok` only once it is durable: at once for
    /// a backend that commits a store inline, and otherwise after a flush
    /// that succeeds with, after it, no flush of the wallet id's records in
    /// progress and none failed since before the store ([`FlushLedger`]),
    /// read in that order.
    ///
    /// The flush succeeding proves nothing by itself: another flush may take
    /// what the store buffered and fail, dropping it or putting it back, and
    /// the one here then succeeds over a buffer that does not hold it. That
    /// other flush began before this one returned, and it records its failure
    /// only after the backend returns: so after this flush it shows either as
    /// a flush in progress, or as one more that failed than there were before
    /// the store. One that failed for good answers [`NotDurable::Lost`]; one
    /// in progress, or that put back what it took,
    /// [`NotDurable::OtherFlushUnsettled`], as a later flush writes it.
    ///
    /// It answers for `changeset` alone, which [`Self::flush_durably`] does
    /// not: a flush that failed for good before the store dropped nothing of
    /// it, and does not refuse it.
    pub(crate) fn store_durably(
        &self,
        changeset: PlatformWalletChangeSet,
    ) -> Result<(), NotDurable> {
        let (lost_before, failed_before) = (self.flush_ledger.lost(), self.flush_ledger.failed());
        self.store(changeset).map_err(NotDurable::Store)?;
        if self.store_commits_inline() {
            return Ok(());
        }
        self.flush().map_err(NotDurable::Flush)?;
        if self.flush_ledger.in_flight() != 0 {
            return Err(NotDurable::OtherFlushUnsettled);
        }
        if self.flush_ledger.lost() != lost_before {
            return Err(NotDurable::Lost);
        }
        if self.flush_ledger.failed() != failed_before {
            return Err(NotDurable::OtherFlushUnsettled);
        }
        Ok(())
    }

    /// Make every record stored for this wallet before the call durable, and
    /// answer `Ok` only once it is: at once for a backend that commits
    /// inline, after a flush that succeeds otherwise, and never once a flush
    /// of the wallet id's records failed for good since this handle was made
    /// ([`Self::buffered_writes_lost`]).
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
    #[cfg(feature = "shielded")]
    pub(crate) fn flush_durably(&self) -> Result<(), NotDurable> {
        if self.buffered_writes_lost() {
            return Err(NotDurable::Lost);
        }
        if !self.store_commits_inline() {
            let failed_before = self.flush_ledger.failed();
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
        }
        if self.buffered_writes_lost() {
            return Err(NotDurable::Lost);
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
        self.flush_ledger.lost() != self.flush_losses_at_start
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
    use crate::changeset::PersistenceErrorKind;
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
        let ledger = Arc::new(FlushLedger::default());
        let wallet_id: WalletId = [3; 32];
        let persister = WalletPersister::with_flush_ledger(
            wallet_id,
            Arc::clone(&backend) as _,
            Arc::clone(&ledger),
        );
        // A handle of a wallet removed, or created since, under the id.
        let other =
            WalletPersister::with_flush_ledger(wallet_id, Arc::clone(&backend) as _, ledger);
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
