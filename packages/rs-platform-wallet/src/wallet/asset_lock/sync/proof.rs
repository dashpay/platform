//! Proof lifecycle: waiting for proofs, validation, and IS-lock to ChainLock upgrade.

use std::time::Duration;

use dashcore::OutPoint;

use crate::error::PlatformWalletError;

use super::super::manager::AssetLockManager;

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

        // Drop the read lock before making the DAPI call.
        drop(wm);

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
