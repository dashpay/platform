//! Proof lifecycle: waiting for proofs, validation, and IS-lock to ChainLock upgrade.

use crate::broadcaster::TransactionBroadcaster;
use std::time::Duration;

use dashcore::{OutPoint, Txid};
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
use key_wallet::managed_account::transaction_record::TransactionRecord;

use crate::error::PlatformWalletError;

use super::super::manager::AssetLockManager;

/// Fall back to the persister if the in-memory `transactions()` map
/// didn't have the record.
///
/// With upstream's `keep-finalized-transactions` Cargo feature OFF
/// (the default), chainlocked records are evicted from the in-memory
/// map and only their txids retained in `finalized_txids`. The
/// persister received the full record on the chainlock-transition
/// `store` call before eviction, so it can answer the lookup. A
/// persister that doesn't index records by txid (the trait's default
/// impl,
/// [`NoPlatformPersistence`](crate::wallet::persister::NoPlatformPersistence))
/// returns `Ok(None)` here — callers must still handle the absence.
///
/// Persister errors are surfaced as `Err(PersistenceError)` so call
/// sites can choose their own policy:
///
/// - **Poll loops** (`wait_for_chain_lock`, `wait_for_proof`) typically
///   downgrade to `None` for the current iteration so the next tick
///   retries — see [`record_or_persister_or_log`] for that policy.
/// - **One-shot recovery / fast-fail call sites** want the error
///   visible so a transient backend failure isn't silently classified
///   as "tx not found" — they handle the `Err` arm explicitly.
pub(super) fn record_or_persister(
    in_memory: Option<TransactionRecord>,
    persister: &crate::wallet::persister::WalletPersister,
    txid: &Txid,
) -> Result<Option<TransactionRecord>, crate::changeset::PersistenceError> {
    if let Some(record) = in_memory {
        return Ok(Some(record));
    }
    persister.get_core_tx_record(txid)
}

/// Variant of [`record_or_persister`] that swallows persister errors
/// as `None` after a `warn`-level log. Use this from poll loops where
/// the next iteration retries — a hard error from a single tick would
/// abort the whole poll prematurely.
pub(super) fn record_or_persister_or_log(
    in_memory: Option<TransactionRecord>,
    persister: &crate::wallet::persister::WalletPersister,
    txid: &Txid,
) -> Option<TransactionRecord> {
    match record_or_persister(in_memory, persister, txid) {
        Ok(opt) => opt,
        Err(e) => {
            tracing::warn!(
                txid = %txid,
                error = %e,
                "Persister fallback for core tx record failed; \
                 treating as miss for this poll iteration"
            );
            None
        }
    }
}

impl<B: TransactionBroadcaster + ?Sized> AssetLockManager<B> {
    /// Validate an IS-lock proof and upgrade it to a ChainLock proof if the
    /// transaction is old enough that the IS-lock may have expired.
    ///
    /// When the asset lock transaction has been chain-locked and has enough
    /// confirmations (> 8), the InstantSend lock quorum may have rotated,
    /// causing Platform to reject the IS proof. In that case, if the
    /// transaction's block height is within Platform's verified range
    /// (`core_chain_locked_height`), we can safely switch to a ChainLock
    /// proof.
    ///
    /// If the proof is already a ChainLock proof, or the IS proof is still
    /// fresh, it is returned unchanged.
    pub(crate) async fn validate_or_upgrade_proof(
        &self,
        proof: dpp::prelude::AssetLockProof,
        account_index: u32,
        out_point: &OutPoint,
    ) -> Result<dpp::prelude::AssetLockProof, PlatformWalletError> {
        use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
        use key_wallet::transaction_checking::TransactionContext;

        if !matches!(&proof, dpp::prelude::AssetLockProof::Instant(_)) {
            return Ok(proof);
        }

        let in_memory = {
            let wm = self.wallet_manager.read().await;
            let info = wm
                .get_wallet_info(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
            info.core_wallet
                .accounts
                .standard_bip44_accounts
                .get(&account_index)
                .and_then(|a| a.transactions().get(&out_point.txid).cloned())
            // wm dropped at end of block — release before persister + DAPI calls.
        };

        let record = record_or_persister(in_memory, &self.persister, &out_point.txid)
            .map_err(|e| {
                PlatformWalletError::AssetLockProofWait(format!(
                    "Persister lookup for tx {} failed: {}",
                    out_point.txid, e
                ))
            })?
            .ok_or_else(|| {
                PlatformWalletError::AssetLockProofWait(format!(
                    "Transaction {} not found in account {} (in-memory or persister)",
                    out_point.txid, account_index
                ))
            })?;

        let is_chain_locked = matches!(record.context, TransactionContext::InChainLockedBlock(_));
        let height = record.height().unwrap_or(0);

        if is_chain_locked && height > 0 {
            let platform_height = self.get_platform_core_chain_locked_height().await?;

            if height <= platform_height {
                tracing::debug!(
                    "Upgrading IS-lock proof to ChainLock proof for tx {} \
                     (height={}, platform_cl_height={})",
                    out_point.txid,
                    height,
                    platform_height,
                );

                return Ok(dpp::prelude::AssetLockProof::Chain(ChainAssetLockProof {
                    core_chain_locked_height: height,
                    out_point: *out_point,
                }));
            }
        }

        Ok(proof)
    }

    /// Fetch Platform's current `core_chain_locked_height` by querying the
    /// latest epoch info with metadata.
    async fn get_platform_core_chain_locked_height(&self) -> Result<u32, PlatformWalletError> {
        use dash_sdk::platform::fetch_current_no_parameters::FetchCurrent;
        use dpp::block::extended_epoch_info::ExtendedEpochInfo;

        let (_epoch, metadata) = ExtendedEpochInfo::fetch_current_with_metadata(&self.sdk)
            .await
            .map_err(PlatformWalletError::Sdk)?;

        Ok(metadata.core_chain_locked_height)
    }

    /// Upgrade an IS-lock proof to a ChainLock proof after a Platform
    /// rejection.
    ///
    /// Called from the recovery layer when `put_to_platform` fails with
    /// `InvalidInstantAssetLockProofSignature`. If the TX is already
    /// chain-locked, constructs the proof immediately. Otherwise, **waits**
    /// for a ChainLock via SPV events (up to 10 minutes) so the caller
    /// doesn't see a failure — just a longer wait.
    pub(crate) async fn upgrade_to_chain_lock_proof(
        &self,
        out_point: &OutPoint,
        timeout: Duration,
    ) -> Result<dpp::prelude::AssetLockProof, PlatformWalletError> {
        use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
        use key_wallet::transaction_checking::TransactionContext;

        let txid = out_point.txid;

        let account_index = {
            let wm = self.wallet_manager.read().await;
            let info = wm
                .get_wallet_info(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
            info.tracked_asset_locks
                .get(out_point)
                .map(|lock| lock.account_index)
                .ok_or_else(|| {
                    PlatformWalletError::AssetLockProofWait(format!(
                        "Asset lock {} is not tracked",
                        out_point
                    ))
                })?
        };

        // Check if already chain-locked. Falls back to the persister if
        // the in-memory map already evicted the record (default config).
        let in_memory = {
            let wm = self.wallet_manager.read().await;
            let info = wm
                .get_wallet_info(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
            info.core_wallet
                .accounts
                .standard_bip44_accounts
                .get(&account_index)
                .and_then(|a| a.transactions().get(&txid).cloned())
        };

        let record = record_or_persister(in_memory, &self.persister, &txid).map_err(|e| {
            PlatformWalletError::AssetLockProofWait(format!(
                "Persister lookup for tx {} failed: {}",
                txid, e
            ))
        })?;
        let record = record.ok_or_else(|| {
            // Both lookups missed. The asset lock is known-tracked
            // (we validated `tracked_asset_locks.get` above), so this
            // is a wallet-state mismatch / post-wipe race rather than
            // a "not chain-locked yet" case. Fast-fail rather than
            // dispatching to `wait_for_chain_lock` and burning the
            // full timeout.
            PlatformWalletError::AssetLockProofWait(format!(
                "Transaction {} not found in account {} (in-memory or persister)",
                txid, account_index
            ))
        })?;
        let height = if matches!(record.context, TransactionContext::InChainLockedBlock(_)) {
            record.height()
        } else {
            None
        };

        let height = match height {
            Some(h) => h,
            None => {
                // Not chain-locked yet — wait for a ChainLock via SPV events.
                tracing::info!(
                    "Transaction {} not yet chain-locked, waiting for ChainLock...",
                    txid
                );
                self.wait_for_chain_lock(account_index, out_point, timeout)
                    .await?
            }
        };

        // Wait for Platform to verify the block height.
        let platform_height = self.get_platform_core_chain_locked_height().await?;

        if height > platform_height {
            // Platform hasn't verified this block yet. Poll until it does
            // (ChainLock propagation to Platform is typically fast).
            tracing::info!(
                "TX {} at height {} but Platform at height {}, waiting...",
                txid,
                height,
                platform_height
            );
            // TODO: Poll Platform height until it catches up, for now return error.
            return Err(PlatformWalletError::AssetLockExpired(format!(
                "Transaction {} is at height {} but Platform has only verified up to height {}",
                txid, height, platform_height
            )));
        }

        tracing::info!(
            "Building ChainLock proof for tx {} (height={}, platform_cl_height={})",
            txid,
            height,
            platform_height,
        );

        Ok(dpp::prelude::AssetLockProof::Chain(ChainAssetLockProof {
            core_chain_locked_height: height,
            out_point: *out_point,
        }))
    }

    /// Wait for a ChainLock that covers the given transaction.
    ///
    /// Subscribes to SPV events and waits until the transaction's block
    /// is chain-locked.
    async fn wait_for_chain_lock(
        &self,
        account_index: u32,
        out_point: &OutPoint,
        timeout: Duration,
    ) -> Result<u32, PlatformWalletError> {
        use key_wallet::transaction_checking::TransactionContext;

        let deadline = tokio::time::Instant::now() + timeout;

        loop {
            // Check — might have been updated by SPV sync. Falls back
            // to the persister so a chainlock that arrived between
            // polls (and was evicted from the in-memory map) is still
            // observed.
            let in_memory = {
                let wm = self.wallet_manager.read().await;
                wm.get_wallet_info(&self.wallet_id).and_then(|info| {
                    info.core_wallet
                        .accounts
                        .standard_bip44_accounts
                        .get(&account_index)
                        .and_then(|a| a.transactions().get(&out_point.txid).cloned())
                })
            };
            if let Some(record) =
                record_or_persister_or_log(in_memory, &self.persister, &out_point.txid)
            {
                if matches!(record.context, TransactionContext::InChainLockedBlock(_)) {
                    if let Some(h) = record.height() {
                        return Ok(h);
                    }
                }
            }

            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return Err(PlatformWalletError::FinalityTimeout(out_point.txid));
            }

            // Wait for a lock event notification or timeout.
            tokio::select! {
                _ = self.lock_notify.notified() => continue,
                _ = tokio::time::sleep(remaining) => {
                    return Err(PlatformWalletError::FinalityTimeout(out_point.txid));
                }
            }
        }
    }

    /// Wait for an asset lock proof by subscribing to SPV events.
    ///
    /// Wait for an asset lock proof by checking transaction context state.
    ///
    /// Wakes on `lock_notify` (fired by `SpvEventForwarder` on InstantLock /
    /// ChainLock events) and re-checks the transaction record context.
    ///
    /// Returns a properly-constructed `AssetLockProof` on success, or
    /// `FinalityTimeout` if the timeout elapses first.
    pub(in crate::wallet::asset_lock) async fn wait_for_proof(
        &self,
        out_point: &OutPoint,
        timeout: Duration,
    ) -> Result<dpp::prelude::AssetLockProof, PlatformWalletError> {
        use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
        use dpp::identity::state_transition::asset_lock_proof::InstantAssetLockProof;
        use key_wallet::transaction_checking::TransactionContext;

        let deadline = tokio::time::Instant::now() + timeout;

        // Read account_index and transaction from the tracked lock.
        let (account_index, tracked_tx) = {
            let wm = self.wallet_manager.read().await;
            let info = wm
                .get_wallet_info(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
            let lock = info.tracked_asset_locks.get(out_point).ok_or_else(|| {
                PlatformWalletError::AssetLockProofWait(format!(
                    "Asset lock {} is not tracked",
                    out_point.txid
                ))
            })?;
            (lock.account_index, lock.transaction.clone())
        };

        loop {
            // Check the transaction record context for finality. Falls
            // back to the persister so a chainlocked record evicted
            // from the in-memory map is still observed.
            let in_memory = {
                let wm = self.wallet_manager.read().await;
                wm.get_wallet_info(&self.wallet_id).and_then(|info| {
                    info.core_wallet
                        .accounts
                        .standard_bip44_accounts
                        .get(&account_index)
                        .and_then(|a| a.transactions().get(&out_point.txid).cloned())
                })
            };
            if let Some(record) =
                record_or_persister_or_log(in_memory, &self.persister, &out_point.txid)
            {
                match &record.context {
                    TransactionContext::InstantSend(instant_lock) => {
                        return Ok(dpp::prelude::AssetLockProof::Instant(
                            InstantAssetLockProof::new(
                                instant_lock.clone(),
                                tracked_tx,
                                out_point.vout,
                            ),
                        ));
                    }
                    TransactionContext::InChainLockedBlock(_) => {
                        if let Some(height) = record.height() {
                            return Ok(dpp::prelude::AssetLockProof::Chain(ChainAssetLockProof {
                                core_chain_locked_height: height,
                                out_point: *out_point,
                            }));
                        }
                    }
                    _ => {}
                }
            }

            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return Err(PlatformWalletError::FinalityTimeout(out_point.txid));
            }

            // Wait for a lock event notification or timeout.
            tokio::select! {
                _ = self.lock_notify.notified() => continue,
                _ = tokio::time::sleep(remaining) => {
                    return Err(PlatformWalletError::FinalityTimeout(out_point.txid));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use dashcore::blockdata::transaction::Transaction;
    use dashcore::TxIn;
    use key_wallet::account::account_type::StandardAccountType;
    use key_wallet::account::AccountType;
    use key_wallet::managed_account::transaction_record::TransactionDirection;
    use key_wallet::transaction_checking::{TransactionContext, TransactionType};

    use crate::changeset::{
        ClientStartState, PersistenceError, PlatformWalletChangeSet, PlatformWalletPersistence,
    };
    use crate::wallet::persister::{NoPlatformPersistence, WalletPersister};
    use crate::wallet::platform_wallet::WalletId;

    fn record_with_txid(seed: u8) -> TransactionRecord {
        // A unique txid per `seed` falls out of the (different) input
        // outpoint; the actual transaction body doesn't matter for the
        // helper-under-test's purposes.
        let tx = Transaction {
            version: 1,
            lock_time: 0,
            input: vec![TxIn {
                previous_output: dashcore::OutPoint::new(Txid::from([seed; 32]), 0),
                ..Default::default()
            }],
            output: Vec::new(),
            special_transaction_payload: None,
        };
        TransactionRecord::new(
            tx,
            AccountType::Standard {
                index: 0,
                standard_account_type: StandardAccountType::BIP44Account,
            },
            TransactionContext::Mempool,
            TransactionType::Standard,
            TransactionDirection::Incoming,
            Vec::new(),
            Vec::new(),
            0,
        )
    }

    /// Test persister that answers `get_core_tx_record` from a
    /// configurable in-memory map. `store` / `flush` are no-ops; `load`
    /// returns the default state.
    struct FakeRecordStore {
        records: Mutex<HashMap<Txid, TransactionRecord>>,
    }

    impl FakeRecordStore {
        fn with_records<I: IntoIterator<Item = TransactionRecord>>(records: I) -> Self {
            let map = records.into_iter().map(|r| (r.txid, r)).collect();
            Self {
                records: Mutex::new(map),
            }
        }
    }

    impl PlatformWalletPersistence for FakeRecordStore {
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
        fn get_core_tx_record(
            &self,
            _wallet_id: WalletId,
            txid: &Txid,
        ) -> Result<Option<TransactionRecord>, PersistenceError> {
            Ok(self.records.lock().unwrap().get(txid).cloned())
        }
    }

    /// Test persister that always errors out on `get_core_tx_record`,
    /// to exercise the error-swallowing branch in `record_or_persister`.
    struct ErroringStore;

    impl PlatformWalletPersistence for ErroringStore {
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
        fn get_core_tx_record(
            &self,
            _wallet_id: WalletId,
            _txid: &Txid,
        ) -> Result<Option<TransactionRecord>, PersistenceError> {
            Err(PersistenceError::Backend(
                "simulated backend failure".into(),
            ))
        }
    }

    fn wallet_persister(inner: Arc<dyn PlatformWalletPersistence>) -> WalletPersister {
        WalletPersister::new([0u8; 32], inner)
    }

    #[test]
    fn record_or_persister_prefers_in_memory_when_present() {
        // The in-memory record wins; the persister's record is never
        // consulted (so even a record stored under a *different* txid
        // would be ignored — this verifies the helper short-circuits on
        // a hit).
        let in_memory = record_with_txid(0xAA);
        let in_memory_txid = in_memory.txid;
        let other = record_with_txid(0xBB);
        let persister = wallet_persister(Arc::new(FakeRecordStore::with_records([other])));

        let resolved = record_or_persister(Some(in_memory.clone()), &persister, &in_memory_txid)
            .expect("in-memory hit cannot fail");
        assert_eq!(resolved.map(|r| r.txid), Some(in_memory_txid));
    }

    #[test]
    fn record_or_persister_falls_back_to_persister_on_miss() {
        // The in-memory map evicted (None); the persister still has it.
        // This is the chain-locked-eviction recovery path.
        let stored = record_with_txid(0xCC);
        let stored_txid = stored.txid;
        let persister = wallet_persister(Arc::new(FakeRecordStore::with_records([stored])));

        let resolved =
            record_or_persister(None, &persister, &stored_txid).expect("persister succeeded");
        assert_eq!(resolved.map(|r| r.txid), Some(stored_txid));
    }

    #[test]
    fn record_or_persister_returns_none_when_neither_has_it() {
        // Both miss → Ok(None); callers handle this as "tx not found
        // locally" (proof flow returns its own AssetLockProofWait
        // error, poll loops continue waiting).
        let unknown_txid = Txid::from([0xDD; 32]);
        let persister = wallet_persister(Arc::new(FakeRecordStore::with_records([])));

        let resolved =
            record_or_persister(None, &persister, &unknown_txid).expect("persister succeeded");
        assert!(resolved.is_none());
    }

    #[test]
    fn record_or_persister_default_persister_returns_none() {
        // The trait default impl on `NoPlatformPersistence` returns
        // `Ok(None)` — confirms backends that don't override the new
        // method gracefully no-op (the proof flow still works for
        // mempool/InBlock txs; only the chainlock-eviction recovery is
        // unavailable).
        let unknown_txid = Txid::from([0xEE; 32]);
        let persister = wallet_persister(Arc::new(NoPlatformPersistence));

        let resolved =
            record_or_persister(None, &persister, &unknown_txid).expect("persister succeeded");
        assert!(resolved.is_none());
    }

    #[test]
    fn record_or_persister_propagates_backend_errors() {
        // Backend errors surface as `Err` so call sites can choose
        // their own policy (one-shot recovery logs at error and
        // degrades; poll loops downgrade to None for one tick via
        // `record_or_persister_or_log`).
        let unknown_txid = Txid::from([0xFF; 32]);
        let persister = wallet_persister(Arc::new(ErroringStore));

        let resolved = record_or_persister(None, &persister, &unknown_txid);
        assert!(resolved.is_err());
    }

    #[test]
    fn record_or_persister_or_log_swallows_backend_errors_as_none() {
        // The poll-loop variant downgrades errors to `None` (after a
        // `warn` log) so a transient backend failure on one tick
        // doesn't abort the whole poll.
        let unknown_txid = Txid::from([0xFF; 32]);
        let persister = wallet_persister(Arc::new(ErroringStore));

        let resolved = record_or_persister_or_log(None, &persister, &unknown_txid);
        assert!(resolved.is_none());
    }
}
