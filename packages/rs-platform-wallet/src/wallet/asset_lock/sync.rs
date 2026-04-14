//! Asset lock tracking and synchronization.
//!
//! Contains methods for tracking asset locks, advancing their lifecycle status,
//! recovering locks, waiting for proofs and chain locks, resuming interrupted
//! locks, and re-deriving private keys.

use std::time::Duration;

use dashcore::Address as DashAddress;
use dashcore::{OutPoint, PrivateKey};
use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;

use crate::changeset::changeset::AssetLockChangeSet;
use crate::error::PlatformWalletError;

use super::manager::AssetLockManager;
use super::tracked::{AssetLockStatus, TrackedAssetLock};

// ---------------------------------------------------------------------------
// Asset lock tracking
// ---------------------------------------------------------------------------

impl AssetLockManager {
    /// Track a new asset lock in memory, returning a changeset describing
    /// the inserted entry.
    ///
    /// If an entry already exists at `out_point`, it is overwritten.
    pub(crate) async fn track_asset_lock(&self, lock: TrackedAssetLock) -> AssetLockChangeSet {
        let mut wm = self.wallet_manager.write().await;
        let mut cs = AssetLockChangeSet::default();
        if let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) {
            let out_point = lock.out_point;
            cs.asset_locks.insert(out_point, (&lock).into());
            info.tracked_asset_locks.insert(out_point, lock);
        }
        cs
    }

    /// Remove an asset lock after successful consumption (registration or top-up).
    ///
    /// Returns an [`AssetLockChangeSet`] tombstoning the removed entry
    /// (empty if the lock was not tracked).
    pub(crate) async fn remove_asset_lock(&self, out_point: &OutPoint) -> AssetLockChangeSet {
        let mut wm = self.wallet_manager.write().await;
        let mut cs = AssetLockChangeSet::default();
        if let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) {
            if info.tracked_asset_locks.remove(out_point).is_some() {
                cs.removed.insert(*out_point);
            }
        }
        cs
    }

    /// Advance the status of a tracked asset lock and optionally attach the proof.
    ///
    /// Returns an [`AssetLockChangeSet`] carrying a full snapshot of the
    /// updated entry.
    pub(crate) async fn advance_asset_lock_status(
        &self,
        out_point: &OutPoint,
        new_status: AssetLockStatus,
        proof: Option<dpp::prelude::AssetLockProof>,
    ) -> Result<AssetLockChangeSet, PlatformWalletError> {
        let mut wm = self.wallet_manager.write().await;
        let info = wm
            .get_wallet_info_mut(&self.wallet_id)
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
        let entry = info.tracked_asset_locks.get_mut(out_point).ok_or_else(|| {
            PlatformWalletError::AssetLockProofWait(format!(
                "Asset lock {} is not tracked",
                out_point
            ))
        })?;
        entry.status = new_status;
        if proof.is_some() {
            entry.proof = proof;
        }

        let mut cs = AssetLockChangeSet::default();
        cs.asset_locks.insert(*out_point, (&*entry).into());
        Ok(cs)
    }
}

// ---------------------------------------------------------------------------
// Blocking accessor (for synchronous / evo-tool contexts)
// ---------------------------------------------------------------------------

impl AssetLockManager {
    /// Blocking version of [`recover_asset_lock`](Self::recover_asset_lock).
    ///
    /// Uses `tokio::sync::RwLock::blocking_write` / `blocking_read` — must NOT
    /// be called from within a tokio async context.
    ///
    /// When `proof` is `None`, the method looks up the transaction's actual
    /// on-chain context from `ManagedWalletInfo` to determine the correct
    /// status (and constructs a `ChainAssetLockProof` if the TX is in a
    /// chain-locked block).
    pub fn recover_asset_lock_blocking(
        &self,
        tx: dashcore::Transaction,
        amount: u64,
        account_index: u32,
        funding_type: AssetLockFundingType,
        identity_index: u32,
        out_point: OutPoint,
        proof: Option<dpp::prelude::AssetLockProof>,
    ) {
        let mut wm = self.wallet_manager.blocking_write();
        let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) else {
            return;
        };
        if info.tracked_asset_locks.contains_key(&out_point) {
            return;
        }

        let (status, proof) = match proof {
            Some(ref p) => {
                let status = match p {
                    dpp::prelude::AssetLockProof::Instant(_) => AssetLockStatus::InstantSendLocked,
                    dpp::prelude::AssetLockProof::Chain(_) => AssetLockStatus::ChainLocked,
                };
                (status, proof)
            }
            None => {
                // Need to resolve from wallet info - drop the write guard and use a read.
                // Actually we already have mutable access to info, so we can read from it.
                Self::resolve_status_from_info(&info.core_wallet, account_index, &out_point)
            }
        };

        let lock = TrackedAssetLock {
            out_point,
            transaction: tx,
            account_index,
            funding_type,
            identity_index,
            amount,
            status,
            proof,
        };
        let mut cs = AssetLockChangeSet::default();
        cs.asset_locks.insert(out_point, (&lock).into());
        info.tracked_asset_locks.insert(out_point, lock);

        // Must drop the write guard before queuing — the persister's
        // flush (if strategy is Immediate) may need the wallet manager
        // lock for other sub-changesets.
        drop(wm);
        self.queue_asset_lock_changeset(cs);
    }

    /// Determine asset lock status by looking up the transaction in
    /// `ManagedWalletInfo`.
    ///
    /// If the TX is in a chain-locked block, returns `ChainLocked` with a
    /// constructed `ChainAssetLockProof`. If the TX has an InstantSend
    /// context, returns `InstantSendLocked` (without a proof, since we lack
    /// the IS-lock data). Otherwise defaults to `Broadcast`.
    fn resolve_status_from_info(
        wallet_info: &ManagedWalletInfo,
        account_index: u32,
        out_point: &OutPoint,
    ) -> (AssetLockStatus, Option<dpp::prelude::AssetLockProof>) {
        use key_wallet::transaction_checking::TransactionContext;

        let record = wallet_info
            .accounts
            .standard_bip44_accounts
            .get(&account_index)
            .and_then(|a| a.transactions.get(&out_point.txid));

        match record {
            Some(record) => match &record.context {
                TransactionContext::InChainLockedBlock(_) => {
                    if let Some(height) = record.height() {
                        use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
                        let proof = dpp::prelude::AssetLockProof::Chain(ChainAssetLockProof {
                            core_chain_locked_height: height,
                            out_point: *out_point,
                        });
                        (AssetLockStatus::ChainLocked, Some(proof))
                    } else {
                        (AssetLockStatus::ChainLocked, None)
                    }
                }
                TransactionContext::InstantSend(_) => (AssetLockStatus::InstantSendLocked, None),
                _ => (AssetLockStatus::Broadcast, None),
            },
            None => (AssetLockStatus::Broadcast, None),
        }
    }
}

// ---------------------------------------------------------------------------
// Proof validation and upgrade
// ---------------------------------------------------------------------------

impl AssetLockManager {
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

        let wm = self.wallet_manager.read().await;
        let info = wm
            .get_wallet_info(&self.wallet_id)
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
        let synced_height = info.core_wallet.metadata.synced_height;

        let record = info
            .core_wallet
            .accounts
            .standard_bip44_accounts
            .get(&account_index)
            .and_then(|a| a.transactions.get(&out_point.txid))
            .ok_or_else(|| {
                PlatformWalletError::AssetLockProofWait(format!(
                    "Transaction {} not found in account {}",
                    out_point.txid, account_index
                ))
            })?;

        let is_chain_locked = matches!(record.context, TransactionContext::InChainLockedBlock(_));
        let height = record.height().unwrap_or(0);
        let confirmations = record.confirmations(synced_height);

        // Drop the read lock before making the DAPI call.
        drop(wm);

        // TODO: This is weird - why would we wait for 8 confirmations if we already know it's chain-locked?
        if is_chain_locked && height > 0 && confirmations > 8 {
            let platform_height = self.get_platform_core_chain_locked_height().await?;

            if height <= platform_height {
                tracing::debug!(
                    "Upgrading IS-lock proof to ChainLock proof for tx {} \
                     (height={}, confirmations={}, platform_cl_height={})",
                    out_point.txid,
                    height,
                    confirmations,
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

        // Check if already chain-locked.
        let height = {
            let wm = self.wallet_manager.read().await;
            let info = wm
                .get_wallet_info(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
            let record = info
                .core_wallet
                .accounts
                .standard_bip44_accounts
                .get(&account_index)
                .and_then(|a| a.transactions.get(&txid))
                .ok_or_else(|| {
                    PlatformWalletError::AssetLockProofWait(format!(
                        "Transaction {} not found in account {}",
                        txid, account_index
                    ))
                })?;

            if matches!(record.context, TransactionContext::InChainLockedBlock(_)) {
                record.height()
            } else {
                None
            }
        };

        let height = match height {
            Some(h) => h,
            None => {
                // Not chain-locked yet — wait for a ChainLock via SPV events.
                tracing::info!(
                    "Transaction {} not yet chain-locked, waiting for ChainLock...",
                    txid
                );
                self.wait_for_chain_lock(account_index, &out_point, timeout)
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
            // Check — might have been updated by SPV sync.
            {
                let wm = self.wallet_manager.read().await;
                if let Some(info) = wm.get_wallet_info(&self.wallet_id) {
                    if let Some(record) = info
                        .core_wallet
                        .accounts
                        .standard_bip44_accounts
                        .get(&account_index)
                        .and_then(|a| a.transactions.get(&out_point.txid))
                    {
                        if matches!(record.context, TransactionContext::InChainLockedBlock(_)) {
                            if let Some(h) = record.height() {
                                return Ok(h);
                            }
                        }
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
    pub(super) async fn wait_for_proof(
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
            // Check the transaction record context for finality.
            {
                let wm = self.wallet_manager.read().await;
                if let Some(info) = wm.get_wallet_info(&self.wallet_id) {
                    if let Some(record) = info
                        .core_wallet
                        .accounts
                        .standard_bip44_accounts
                        .get(&account_index)
                        .and_then(|a| a.transactions.get(&out_point.txid))
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
                                    return Ok(dpp::prelude::AssetLockProof::Chain(
                                        ChainAssetLockProof {
                                            core_chain_locked_height: height,
                                            out_point: *out_point,
                                        },
                                    ));
                                }
                            }
                            _ => {}
                        }
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
}

// ---------------------------------------------------------------------------
// Resumable asset lock
// ---------------------------------------------------------------------------

impl AssetLockManager {
    /// Resume a tracked asset lock from whatever stage it's at.
    ///
    /// Looks up the tracked lock by `txid`, then:
    ///
    /// - **`Built`**: re-broadcasts the transaction and waits for a proof.
    /// - **`Broadcast`**: waits for a proof.
    /// - **`InstantSendLocked` / `ChainLocked`**: uses the existing proof
    ///   (upgrading a stale IS-lock to a ChainLock proof if necessary).
    ///
    /// After obtaining the proof, advances the tracked lock status and
    /// re-derives the one-time private key from the wallet.
    ///
    /// Returns `(proof, private_key)` ready for use in identity registration
    /// or top-up.
    pub async fn resume_asset_lock(
        &self,
        out_point: &OutPoint,
        timeout: Duration,
    ) -> Result<(dpp::prelude::AssetLockProof, PrivateKey), PlatformWalletError> {
        // 1. Look up the tracked lock — snapshot the fields we need.
        let (tx, status, existing_proof, account_index) = {
            let wm = self.wallet_manager.read().await;
            let info = wm
                .get_wallet_info(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
            let lock = info.tracked_asset_locks.get(out_point).ok_or_else(|| {
                PlatformWalletError::AssetLockProofWait(format!(
                    "Asset lock {} is not tracked",
                    out_point
                ))
            })?;
            (
                lock.transaction.clone(),
                lock.status.clone(),
                lock.proof.clone(),
                lock.account_index,
            )
        };

        // 2. Resume from the current status.
        let proof = match status {
            AssetLockStatus::Built => {
                // Re-broadcast and wait for proof.
                self.broadcaster.broadcast(&tx).await?;
                let cs = self
                    .advance_asset_lock_status(out_point, AssetLockStatus::Broadcast, None)
                    .await?;
                self.queue_asset_lock_changeset(cs);
                let proof = self.wait_for_proof(out_point, timeout).await?;
                self.validate_or_upgrade_proof(proof, account_index, out_point)
                    .await?
            }
            AssetLockStatus::Broadcast => {
                // Already broadcast — just wait for proof.
                let proof = self.wait_for_proof(out_point, timeout).await?;
                self.validate_or_upgrade_proof(proof, account_index, out_point)
                    .await?
            }
            AssetLockStatus::InstantSendLocked | AssetLockStatus::ChainLocked => {
                // Already have a proof — validate / upgrade if stale.
                let proof = existing_proof.ok_or_else(|| {
                    PlatformWalletError::AssetLockProofWait(format!(
                        "Asset lock {} is marked as {:?} but has no proof attached",
                        out_point, status
                    ))
                })?;
                self.validate_or_upgrade_proof(proof, account_index, out_point)
                    .await?
            }
        };

        // 3. Advance status and attach proof.
        let new_status = match &proof {
            dpp::prelude::AssetLockProof::Instant(_) => AssetLockStatus::InstantSendLocked,
            dpp::prelude::AssetLockProof::Chain(_) => AssetLockStatus::ChainLocked,
        };
        let cs = self
            .advance_asset_lock_status(out_point, new_status, Some(proof.clone()))
            .await?;
        self.queue_asset_lock_changeset(cs);

        // 4. Re-derive the one-time private key.
        let private_key = {
            let wm = self.wallet_manager.read().await;
            let info = wm
                .get_wallet_info(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
            let lock = info.tracked_asset_locks.get(out_point).ok_or_else(|| {
                PlatformWalletError::AssetLockProofWait(format!(
                    "Asset lock {} disappeared during resume",
                    out_point
                ))
            })?;
            self.rederive_private_key(lock).await?
        };

        Ok((proof, private_key))
    }

    /// Re-derive the one-time private key for a tracked asset lock.
    ///
    /// The credit output address was generated from a funding account
    /// (identity registration, top-up, etc.). This method finds that address
    /// in the funding account's address pool, retrieves its derivation path,
    /// and derives the private key from the wallet's root key.
    async fn rederive_private_key(
        &self,
        lock: &TrackedAssetLock,
    ) -> Result<PrivateKey, PlatformWalletError> {
        use dashcore::blockdata::transaction::special_transaction::TransactionPayload;
        use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;

        // 1. Extract the credit output from the AssetLockPayload.
        let payload = lock
            .transaction
            .special_transaction_payload
            .as_ref()
            .ok_or_else(|| {
                PlatformWalletError::AssetLockTransaction(
                    "Transaction has no special transaction payload".to_string(),
                )
            })?;
        let asset_lock_payload = match payload {
            TransactionPayload::AssetLockPayloadType(p) => p,
            _ => {
                return Err(PlatformWalletError::AssetLockTransaction(
                    "Transaction payload is not an AssetLockPayload".to_string(),
                ));
            }
        };
        let credit_output = asset_lock_payload.credit_outputs.first().ok_or_else(|| {
            PlatformWalletError::AssetLockTransaction(
                "AssetLockPayload has no credit outputs".to_string(),
            )
        })?;

        // 2. Get the address from the credit output's script_pubkey.
        let address = DashAddress::from_script(&credit_output.script_pubkey, self.sdk.network)
            .map_err(|e| {
                PlatformWalletError::AssetLockTransaction(format!(
                    "Failed to derive address from credit output script: {}",
                    e
                ))
            })?;

        // 3. Find the derivation path in the funding account and derive key under a single lock.
        let wm = self.wallet_manager.read().await;
        let info = wm
            .get_wallet_info(&self.wallet_id)
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
        let wi = &info.core_wallet;
        let funding_account = match lock.funding_type {
            AssetLockFundingType::IdentityRegistration => {
                wi.accounts.identity_registration.as_ref()
            }
            AssetLockFundingType::IdentityTopUp => {
                wi.accounts.identity_topup.get(&lock.identity_index)
            }
            AssetLockFundingType::IdentityTopUpNotBound => {
                wi.accounts.identity_topup_not_bound.as_ref()
            }
            AssetLockFundingType::IdentityInvitation => wi.accounts.identity_invitation.as_ref(),
            AssetLockFundingType::AssetLockAddressTopUp => {
                wi.accounts.asset_lock_address_topup.as_ref()
            }
            AssetLockFundingType::AssetLockShieldedAddressTopUp => {
                wi.accounts.asset_lock_shielded_address_topup.as_ref()
            }
        };

        let funding_account = funding_account.ok_or_else(|| {
            PlatformWalletError::AssetLockTransaction(format!(
                "Funding account {:?} not found for re-derivation",
                lock.funding_type
            ))
        })?;

        let derivation_path = funding_account
            .address_derivation_path(&address)
            .ok_or_else(|| {
                PlatformWalletError::AssetLockTransaction(format!(
                    "Address {} not found in funding account {:?}",
                    address, lock.funding_type
                ))
            })?;

        // 4. Derive the private key from the wallet's root key.
        let wallet = wm
            .get_wallet(&self.wallet_id)
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
        let secret_key = wallet.derive_private_key(&derivation_path).map_err(|e| {
            PlatformWalletError::AssetLockTransaction(format!(
                "Failed to derive private key for asset lock: {}",
                e
            ))
        })?;

        Ok(PrivateKey::new(secret_key, self.sdk.network))
    }
}
