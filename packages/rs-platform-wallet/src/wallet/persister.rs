//! Per-wallet persistence handles.
//!
//! Contains:
//! - [`WalletPersister`] — wraps the shared [`PlatformWalletPersistence`] with
//!   a fixed `wallet_id` so callers don't need to pass the ID on every call.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use dashcore::Txid;
use key_wallet::managed_account::transaction_record::TransactionRecord;

use crate::changeset::{
    ClientStartState, DpnsNameStateEntry, PersistenceCapabilities, PersistenceError,
    PlatformWalletChangeSet, PlatformWalletPersistence,
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
    /// differs from it is stored again. Absent when none was stored, or the
    /// last store or its flush failed. Shared by the clones the wallet and
    /// the coordinator hold. In memory only.
    stored_private_markers: Arc<std::sync::Mutex<BTreeMap<Identifier, bool>>>,
}

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

impl WalletPersister {
    pub fn new(wallet_id: WalletId, inner: Arc<dyn PlatformWalletPersistence>) -> Self {
        Self {
            wallet_id,
            inner,
            contact_payment_gate: Arc::new(tokio::sync::Mutex::new(())),
            #[cfg(feature = "shielded")]
            read_back_private_entries: Arc::default(),
            discharged_private_entries: Arc::default(),
            private_payments_unrestored: Arc::default(),
            stored_private_markers: Arc::default(),
        }
    }

    /// Note that `owner`'s position record was stored with the marker
    /// `marker`, or, with `None`, that whether it was is not known: its last
    /// store or flush failed ([`Self::private_marker_stored`]).
    pub(crate) fn note_private_marker_stored(&self, owner: Identifier, marker: Option<bool>) {
        let mut stored = self
            .stored_private_markers
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match marker {
            Some(marker) => stored.insert(owner, marker),
            None => stored.remove(&owner),
        };
    }

    /// The marker `owner`'s position record was last stored with in this
    /// session, if one was and its store did not fail since
    /// ([`Self::note_private_marker_stored`]).
    pub(crate) fn private_marker_stored(&self, owner: &Identifier) -> Option<bool> {
        self.stored_private_markers
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(owner)
            .copied()
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

    pub(crate) fn flush(&self) -> Result<(), PersistenceError> {
        self.inner.flush(self.wallet_id)
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
}
