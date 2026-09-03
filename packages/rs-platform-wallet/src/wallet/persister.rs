//! Per-wallet persistence handles.
//!
//! Contains:
//! - [`WalletPersister`] — wraps the shared [`PlatformWalletPersistence`] with
//!   a fixed `wallet_id` so callers don't need to pass the ID on every call.

use std::sync::Arc;

use dashcore::Txid;
use key_wallet::managed_account::transaction_record::TransactionRecord;

use crate::changeset::{
    ClientStartState, DpnsNameStateEntry, PersistenceCapabilities, PersistenceError,
    PlatformWalletChangeSet, PlatformWalletPersistence,
};
use crate::wallet::platform_wallet::WalletId;
use dpp::prelude::Identifier;

/// Per-wallet persistence handle.
///
/// Thin wrapper around the shared [`PlatformWalletPersistence`] that binds
/// a specific wallet's ID. Created by [`PlatformWallet::new`] and used
/// internally for `queue_persist` / `flush_persist`.
#[derive(Clone)]
pub struct WalletPersister {
    wallet_id: WalletId,
    inner: Arc<dyn PlatformWalletPersistence>,
}

impl WalletPersister {
    pub fn new(wallet_id: WalletId, inner: Arc<dyn PlatformWalletPersistence>) -> Self {
        Self { wallet_id, inner }
    }

    pub(crate) fn store(&self, changeset: PlatformWalletChangeSet) -> Result<(), PersistenceError> {
        self.inner.store(self.wallet_id, changeset)
    }

    pub(crate) fn flush(&self) -> Result<(), PersistenceError> {
        self.inner.flush(self.wallet_id)
    }

    pub(crate) fn store_commits_inline(&self) -> bool {
        self.inner.store_commits_inline()
    }

    /// Feature-specific persistence contracts exposed by the backend.
    pub(crate) fn persistence_capabilities(&self) -> PersistenceCapabilities {
        self.inner.persistence_capabilities()
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

    /// [`Self::get_core_tx_record`] with the shared transient-as-miss
    /// read policy applied.
    ///
    /// A transient backend failure (a busy store) is indistinguishable in
    /// outcome from "the row is not readable right now", and every caller
    /// of this read already handles a miss by retrying on its next pass —
    /// so it collapses to `Ok(None)` and is logged at debug. A permanent
    /// failure stays an `Err`: it will not fix itself, so a caller that
    /// swallowed it would repeat the same doomed work forever with no
    /// signal. Callers that need to tell the two apart use
    /// [`Self::get_core_tx_record`] directly.
    pub(crate) fn get_core_tx_record_or_transient_miss(
        &self,
        txid: &Txid,
    ) -> Result<Option<TransactionRecord>, PersistenceError> {
        match self.get_core_tx_record(txid) {
            Err(e) if e.is_transient() => {
                tracing::debug!(
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
