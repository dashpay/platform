//! Crash recovery and resume for asset locks.
//!
//! Contains methods for recovering asset locks from persisted state,
//! resolving status from wallet info, resuming interrupted locks,
//! and re-deriving private keys.

use crate::broadcaster::TransactionBroadcaster;
use std::time::Duration;

use dashcore::Address as DashAddress;
use dashcore::{OutPoint, PrivateKey};
use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;

use crate::changeset::changeset::AssetLockChangeSet;
use crate::error::PlatformWalletError;

use super::super::manager::AssetLockManager;
use super::super::tracked::{AssetLockStatus, TrackedAssetLock};

// ---------------------------------------------------------------------------
// Blocking accessor (for synchronous / evo-tool contexts)
// ---------------------------------------------------------------------------

impl<B: TransactionBroadcaster + ?Sized> AssetLockManager<B> {
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
// Resumable asset lock
// ---------------------------------------------------------------------------

impl<B: TransactionBroadcaster + ?Sized> AssetLockManager<B> {
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
