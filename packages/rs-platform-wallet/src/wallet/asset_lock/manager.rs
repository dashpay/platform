//! Asset lock lifecycle manager.
//!
//! Encapsulates all asset lock operations: building transactions, broadcasting,
//! waiting for proofs, and tracking lifecycle status. Shared across sub-wallets
//! via `Arc<AssetLockManager>`.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use dashcore::Address as DashAddress;
use dashcore::{OutPoint, PrivateKey, Transaction, TxOut, Txid};
use key_wallet::wallet::managed_wallet_info::asset_lock_builder::{
    AssetLockFundingType, CreditOutputFunding,
};
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;
use tokio::sync::{broadcast, RwLock};

use crate::changeset::changeset::AssetLockChangeSet;
use crate::error::PlatformWalletError;
use crate::events::PlatformWalletEvent;

use super::tracked::{AssetLockStatus, TrackedAssetLock};

/// Default fee rate in duffs per kilobyte for asset lock transactions.
const DEFAULT_FEE_PER_KB: u64 = 1000;

/// Manages the full asset lock lifecycle: build, broadcast, proof, and tracking.
///
/// Shared across sub-wallets via `Arc<AssetLockManager>` so that any sub-wallet
/// (identity, platform-address, shielded) can create and consume asset locks
/// without going through `CoreWallet`.
#[derive(Clone)]
pub struct AssetLockManager {
    sdk: Arc<dash_sdk::Sdk>,
    wallet: Arc<RwLock<Wallet>>,
    wallet_info: Arc<RwLock<ManagedWalletInfo>>,
    /// Broadcast channel for platform wallet events (SPV sync, locks, etc.).
    ///
    /// Used by `wait_for_proof()` to subscribe to InstantLock / ChainLock
    /// events from the SPV layer.
    event_tx: broadcast::Sender<PlatformWalletEvent>,
    /// Tracked asset locks, keyed by outpoint (txid + output index).
    ///
    /// Each credit output in an asset lock transaction is tracked
    /// independently because a single transaction can have up to 255
    /// credit outputs (DIP-0027), each consumable separately.
    /// Removed once consumed by a successful identity operation.
    tracked: Arc<RwLock<BTreeMap<OutPoint, TrackedAssetLock>>>,
    /// Transaction broadcaster — pluggable so the same `AssetLockManager`
    /// works with different broadcast backends:
    ///
    /// - [`DapiBroadcaster`](crate::broadcaster::DapiBroadcaster) — gRPC via
    ///   Platform DAPI (default for standalone wallets without SPV).
    /// - [`SpvBroadcaster`](crate::broadcaster::SpvBroadcaster) — P2P via SPV
    ///   peers (used when managed by `PlatformWalletManager` with SPV enabled).
    ///
    /// Injected at construction by `PlatformWallet::new()`. The caller
    /// (typically `PlatformWalletManager`) decides which implementation to use.
    broadcaster: Arc<dyn crate::broadcaster::TransactionBroadcaster>,
}

impl AssetLockManager {
    /// Create a new `AssetLockManager`.
    pub(crate) fn new(
        sdk: Arc<dash_sdk::Sdk>,
        wallet: Arc<RwLock<Wallet>>,
        wallet_info: Arc<RwLock<ManagedWalletInfo>>,
        event_tx: broadcast::Sender<PlatformWalletEvent>,
        broadcaster: Arc<dyn crate::broadcaster::TransactionBroadcaster>,
    ) -> Self {
        Self {
            sdk,
            wallet,
            wallet_info,
            event_tx,
            tracked: Arc::new(RwLock::new(BTreeMap::new())),
            broadcaster,
        }
    }
}

// ---------------------------------------------------------------------------
// Changeset support
// ---------------------------------------------------------------------------

impl AssetLockManager {
    /// Snapshot the current tracked asset locks into a changeset for persistence.
    pub(crate) async fn to_changeset(&self) -> AssetLockChangeSet {
        use crate::changeset::changeset::AssetLockEntry;

        let map = self.tracked.read().await;
        let entries = map
            .iter()
            .map(|(out_point, lock)| {
                (
                    *out_point,
                    AssetLockEntry {
                        out_point: lock.out_point,
                        transaction: lock.transaction.clone(),
                        account_index: lock.account_index,
                        funding_type: lock.funding_type,
                        identity_index: lock.identity_index,
                        amount_duffs: lock.amount,
                        status: lock.status.clone(),
                        proof: lock.proof.clone(),
                    },
                )
            })
            .collect();
        AssetLockChangeSet {
            asset_locks: entries,
        }
    }

    /// Restore tracked asset locks from a persisted changeset.
    ///
    /// Uses `blocking_write` — must NOT be called from within a tokio async context.
    pub(crate) fn restore_from_changeset_blocking(&self, changeset: &AssetLockChangeSet) {
        let mut map = self.tracked.blocking_write();
        for (out_point, entry) in &changeset.asset_locks {
            map.insert(
                *out_point,
                TrackedAssetLock {
                    out_point: *out_point,
                    transaction: entry.transaction.clone(),
                    account_index: entry.account_index,
                    funding_type: entry.funding_type,
                    identity_index: entry.identity_index,
                    amount: entry.amount_duffs,
                    status: entry.status.clone(),
                    proof: entry.proof.clone(),
                },
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Public read accessors
// ---------------------------------------------------------------------------

impl AssetLockManager {
    /// List all tracked asset locks (blocking version for UI / synchronous contexts).
    ///
    /// Uses `tokio::sync::RwLock::blocking_read` — must NOT be called from
    /// within a tokio async context.
    pub fn list_tracked_locks_blocking(&self) -> Vec<TrackedAssetLock> {
        let map = self.tracked.blocking_read();
        map.values().cloned().collect()
    }

    /// List all tracked asset locks (async version).
    pub async fn list_tracked_locks(&self) -> Vec<TrackedAssetLock> {
        let map = self.tracked.read().await;
        map.values().cloned().collect()
    }
}

// ---------------------------------------------------------------------------
// Asset lock tracking
// ---------------------------------------------------------------------------

impl AssetLockManager {
    /// Remove an asset lock after successful consumption (registration or top-up).
    pub(crate) async fn remove_asset_lock(&self, out_point: &OutPoint) {
        let mut map = self.tracked.write().await;
        map.remove(out_point);
    }

    /// Advance the status of a tracked asset lock and optionally attach the proof.
    async fn advance_asset_lock_status(
        &self,
        out_point: &OutPoint,
        new_status: AssetLockStatus,
        proof: Option<dpp::prelude::AssetLockProof>,
    ) -> Result<(), PlatformWalletError> {
        let mut map = self.tracked.write().await;
        let entry = map.get_mut(out_point).ok_or_else(|| {
            PlatformWalletError::AssetLockProofWait(format!(
                "Asset lock {} is not tracked",
                out_point
            ))
        })?;
        entry.status = new_status;
        if proof.is_some() {
            entry.proof = proof;
        }
        Ok(())
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
        tx: Transaction,
        amount: u64,
        account_index: u32,
        funding_type: AssetLockFundingType,
        identity_index: u32,
        out_point: OutPoint,
        proof: Option<dpp::prelude::AssetLockProof>,
    ) {
        let mut map = self.tracked.blocking_write();
        if map.contains_key(&out_point) {
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
            None => self.resolve_status_from_wallet_info(account_index, &out_point),
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
        map.insert(out_point, lock);
    }

    /// Determine asset lock status by looking up the transaction in
    /// `ManagedWalletInfo`.
    ///
    /// If the TX is in a chain-locked block, returns `ChainLocked` with a
    /// constructed `ChainAssetLockProof`. If the TX has an InstantSend
    /// context, returns `InstantSendLocked` (without a proof, since we lack
    /// the IS-lock data). Otherwise defaults to `Broadcast`.
    fn resolve_status_from_wallet_info(
        &self,
        account_index: u32,
        out_point: &OutPoint,
    ) -> (AssetLockStatus, Option<dpp::prelude::AssetLockProof>) {
        use key_wallet::transaction_checking::TransactionContext;

        let info = self.wallet_info.blocking_read();
        let record = info
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
                TransactionContext::InstantSend => (AssetLockStatus::InstantSendLocked, None),
                _ => (AssetLockStatus::Broadcast, None),
            },
            None => (AssetLockStatus::Broadcast, None),
        }
    }
}

// ---------------------------------------------------------------------------
// Transaction broadcasting (asset-lock-specific)
// ---------------------------------------------------------------------------


// ---------------------------------------------------------------------------
// Asset lock transaction building
// ---------------------------------------------------------------------------

impl AssetLockManager {
    /// Build an asset lock transaction using the key-wallet builder.
    ///
    /// Delegates UTXO selection, fee calculation, change handling, and signing
    /// to `ManagedWalletInfo::build_asset_lock`.
    ///
    /// # Arguments
    ///
    /// * `amount_duffs` — Amount to lock in duffs.
    /// * `account_index` — BIP44 account index to select UTXOs from.
    /// * `funding_type` — Which account to derive the one-time key from
    ///   (e.g., `IdentityRegistration`, `IdentityTopUp`).
    /// * `identity_index` — Identity index (used by `IdentityTopUp`, ignored by others).
    pub async fn build_asset_lock_transaction(
        &self,
        amount_duffs: u64,
        account_index: u32,
        funding_type: AssetLockFundingType,
        identity_index: u32,
    ) -> Result<(Transaction, PrivateKey), PlatformWalletError> {
        if amount_duffs == 0 {
            return Err(PlatformWalletError::AssetLockTransaction(
                "Amount must be greater than zero".to_string(),
            ));
        }

        let wallet = self.wallet.read().await;
        let mut wallet_info = self.wallet_info.write().await;

        // 1. Peek at the next unused address from the funding account to
        //    build the credit output P2PKH script.
        let funding_address = Self::peek_next_funding_address(
            &mut wallet_info,
            &wallet,
            funding_type,
            identity_index,
        )?;

        // 2. Build the credit output for the asset lock payload.
        let credit_output = TxOut {
            value: amount_duffs,
            script_pubkey: funding_address.script_pubkey(),
        };

        let funding = CreditOutputFunding {
            output: credit_output,
            funding_type,
            identity_index,
        };

        // 3. Delegate to the key-wallet builder.
        let result = wallet_info
            .build_asset_lock(&wallet, account_index, vec![funding], DEFAULT_FEE_PER_KB)
            .map_err(|e| {
                PlatformWalletError::AssetLockTransaction(format!(
                    "Asset lock builder failed: {}",
                    e
                ))
            })?;

        // 4. Convert the raw key bytes to a PrivateKey.
        let key_bytes = result.keys.into_iter().next().ok_or_else(|| {
            PlatformWalletError::AssetLockTransaction("Builder returned no keys".to_string())
        })?;
        let one_time_private_key = PrivateKey::from_byte_array(&key_bytes, self.sdk.network)
            .map_err(|e| {
                PlatformWalletError::AssetLockTransaction(format!(
                    "Invalid private key from builder: {}",
                    e
                ))
            })?;

        Ok((result.transaction, one_time_private_key))
    }

    /// Peek at the next unused address from a funding account without
    /// consuming it (i.e. without marking it as used).
    ///
    /// The key-wallet builder's `next_private_key` will later find the same
    /// address, derive the private key, and mark it as used.
    fn peek_next_funding_address(
        wallet_info: &mut ManagedWalletInfo,
        wallet: &Wallet,
        funding_type: AssetLockFundingType,
        identity_index: u32,
    ) -> Result<DashAddress, PlatformWalletError> {
        let (managed_account, account_xpub) = match funding_type {
            AssetLockFundingType::IdentityRegistration => {
                let xpub = wallet
                    .accounts
                    .identity_registration
                    .as_ref()
                    .map(|a| a.account_xpub);
                let account = wallet_info
                    .accounts
                    .identity_registration
                    .as_mut()
                    .ok_or_else(|| {
                        PlatformWalletError::AssetLockTransaction(
                            "Identity registration account not found".to_string(),
                        )
                    })?;
                (account, xpub)
            }
            AssetLockFundingType::IdentityTopUp => {
                let xpub = wallet
                    .accounts
                    .identity_topup
                    .get(&identity_index)
                    .map(|a| a.account_xpub);
                let account = wallet_info
                    .accounts
                    .identity_topup
                    .get_mut(&identity_index)
                    .ok_or_else(|| {
                        PlatformWalletError::AssetLockTransaction(format!(
                            "Identity top-up account for index {} not found",
                            identity_index
                        ))
                    })?;
                (account, xpub)
            }
            AssetLockFundingType::IdentityTopUpNotBound => {
                let xpub = wallet
                    .accounts
                    .identity_topup_not_bound
                    .as_ref()
                    .map(|a| a.account_xpub);
                let account = wallet_info
                    .accounts
                    .identity_topup_not_bound
                    .as_mut()
                    .ok_or_else(|| {
                        PlatformWalletError::AssetLockTransaction(
                            "Identity top-up (unbound) account not found".to_string(),
                        )
                    })?;
                (account, xpub)
            }
            AssetLockFundingType::IdentityInvitation => {
                let xpub = wallet
                    .accounts
                    .identity_invitation
                    .as_ref()
                    .map(|a| a.account_xpub);
                let account = wallet_info
                    .accounts
                    .identity_invitation
                    .as_mut()
                    .ok_or_else(|| {
                        PlatformWalletError::AssetLockTransaction(
                            "Identity invitation account not found".to_string(),
                        )
                    })?;
                (account, xpub)
            }
            AssetLockFundingType::AssetLockAddressTopUp => {
                let xpub = wallet
                    .accounts
                    .asset_lock_address_topup
                    .as_ref()
                    .map(|a| a.account_xpub);
                let account = wallet_info
                    .accounts
                    .asset_lock_address_topup
                    .as_mut()
                    .ok_or_else(|| {
                        PlatformWalletError::AssetLockTransaction(
                            "Asset lock address top-up account not found".to_string(),
                        )
                    })?;
                (account, xpub)
            }
            AssetLockFundingType::AssetLockShieldedAddressTopUp => {
                let xpub = wallet
                    .accounts
                    .asset_lock_shielded_address_topup
                    .as_ref()
                    .map(|a| a.account_xpub);
                let account = wallet_info
                    .accounts
                    .asset_lock_shielded_address_topup
                    .as_mut()
                    .ok_or_else(|| {
                        PlatformWalletError::AssetLockTransaction(
                            "Asset lock shielded address top-up account not found".to_string(),
                        )
                    })?;
                (account, xpub)
            }
        };

        // Get the next unused address from the pool.  We pass
        // `add_to_state: true` so that a newly-generated address is stored
        // in the pool and the builder's `next_private_key` can find it.
        // The address is NOT marked as used yet — that happens inside the
        // builder after a successful transaction build.
        managed_account
            .next_address(account_xpub.as_ref(), true)
            .map_err(|e| {
                PlatformWalletError::AssetLockTransaction(format!(
                    "Failed to get next funding address: {}",
                    e
                ))
            })
    }

    /// Build, broadcast, and wait for an asset lock proof.
    ///
    /// This is the **unified** entry point for obtaining a funded asset lock
    /// proof, replacing the earlier `create_registration_asset_lock_proof` and
    /// `create_topup_asset_lock_proof` methods.
    ///
    /// ## Flow
    ///
    /// 1. Build the asset lock transaction via the key-wallet builder.
    /// 2. Track the lifecycle as `Built` (in-memory).
    /// 3. Broadcast the transaction.
    /// 4. Wait for an InstantLock or ChainLock proof via the event channel.
    /// 5. Track the lifecycle as `InstantSendLocked` or `ChainLocked`.
    /// 6. Return `(proof, private_key, txid)`.
    ///
    /// ## Persistence
    ///
    /// This method tracks the asset lock in memory before broadcasting, so
    /// the lock is recoverable even if the proof wait is interrupted. However,
    /// the `AssetLockManager` does not persist state directly — **callers MUST
    /// persist the wallet state** after this method returns (or after broadcast
    /// if crash-safety before finality is required). The changeset system
    /// (`AssetLockChangeSet`) will capture the tracked lock state when the
    /// persister flushes.
    ///
    /// ## Parameters
    ///
    /// * `amount_duffs` — Amount to lock.
    /// * `account_index` — BIP44 account index to select UTXOs from.
    /// * `funding_type` — Which account to derive the one-time key from.
    /// * `identity_index` — HD identity index (for `IdentityTopUp`, this is
    ///   the registration index identifying which identity is being topped up).
    pub async fn create_funded_asset_lock_proof(
        &self,
        amount_duffs: u64,
        account_index: u32,
        funding_type: AssetLockFundingType,
        identity_index: u32,
    ) -> Result<(dpp::prelude::AssetLockProof, PrivateKey, OutPoint), PlatformWalletError> {
        // 1. Build the asset lock transaction.
        let (tx, key) = self
            .build_asset_lock_transaction(amount_duffs, account_index, funding_type, identity_index)
            .await?;

        let txid = tx.txid();
        let out_point = OutPoint::new(txid, 0);

        // 2. Track as Built.
        {
            let mut map = self.tracked.write().await;
            map.insert(
                out_point,
                TrackedAssetLock {
                    out_point,
                    transaction: tx.clone(),
                    account_index,
                    funding_type,
                    identity_index,
                    amount: amount_duffs,
                    status: AssetLockStatus::Built,
                    proof: None,
                },
            );
        }

        // NOTE: The tracked lock is now in memory but NOT persisted to storage.
        // If the app crashes after the broadcast below but before this method
        // returns, the lock must be recovered from the chain on restart.
        // Callers that need crash-safety should persist the wallet state here.
        tracing::debug!(
            %txid,
            "Asset lock tracked in memory as Built; broadcasting. \
             Caller should persist wallet state after this method returns."
        );

        // 3. Broadcast.
        self.broadcaster.broadcast(&tx).await?;

        // 4. Transition to Broadcast.
        self.advance_asset_lock_status(&out_point, AssetLockStatus::Broadcast, None)
            .await?;

        // 5. Wait for proof via SPV events.
        let proof = self
            .wait_for_proof(&out_point, Duration::from_secs(300))
            .await?;

        // 5b. If we got an IS-lock proof, check whether the transaction is
        // old enough that Platform might reject it. If so, upgrade to a
        // ChainLock proof proactively.
        let proof = self
            .validate_or_upgrade_proof(proof, account_index, &out_point)
            .await?;

        // 6. Attach proof — status matches the proof type received.
        let status = match &proof {
            dpp::prelude::AssetLockProof::Instant(_) => AssetLockStatus::InstantSendLocked,
            dpp::prelude::AssetLockProof::Chain(_) => AssetLockStatus::ChainLocked,
        };
        self.advance_asset_lock_status(&out_point, status, Some(proof.clone()))
            .await?;

        Ok((proof, key, out_point))
    }

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
    async fn validate_or_upgrade_proof(
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

        let info = self.wallet_info.read().await;
        let synced_height = info.metadata.synced_height;

        let record = info
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
        drop(info);

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
            let map = self.tracked.read().await;
            map.get(out_point)
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
            let info = self.wallet_info.read().await;
            let record = info
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
        let mut rx = self.event_tx.subscribe();

        loop {
            // Re-check — might have been updated by SPV sync while we waited.
            {
                let info = self.wallet_info.read().await;
                if let Some(record) = info
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

            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return Err(PlatformWalletError::FinalityTimeout(out_point.txid));
            }

            tokio::select! {
                event = rx.recv() => {
                    match event {
                        Ok(PlatformWalletEvent::Spv(crate::events::SpvEvent::Sync(
                            dash_spv::sync::SyncEvent::ChainLockReceived { .. },
                        ))) => {
                            // ChainLock received — re-check on next loop iteration.
                            continue;
                        }
                        Ok(_) => {}
                        Err(broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(broadcast::error::RecvError::Closed) => {
                            return Err(PlatformWalletError::SpvError(
                                "Event channel closed".to_string(),
                            ));
                        }
                    }
                }
                _ = tokio::time::sleep(remaining) => {
                    return Err(PlatformWalletError::FinalityTimeout(out_point.txid));
                }
            }
        }
    }

    /// Wait for an asset lock proof by subscribing to SPV events.
    ///
    /// Subscribes to the platform wallet event channel and listens for
    /// `InstantLockReceived` (primary) or `ChainLockReceived` (fallback)
    /// events matching the given transaction.
    ///
    /// Returns a properly-constructed `AssetLockProof` on success, or
    /// `FinalityTimeout` if the timeout elapses first.
    async fn wait_for_proof(
        &self,
        out_point: &OutPoint,
        timeout: Duration,
    ) -> Result<dpp::prelude::AssetLockProof, PlatformWalletError> {
        use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
        use dpp::identity::state_transition::asset_lock_proof::InstantAssetLockProof;
        use key_wallet::transaction_checking::TransactionContext;

        let deadline = tokio::time::Instant::now() + timeout;
        let mut rx = self.event_tx.subscribe();

        // Read account_index and transaction from the tracked lock.
        // These don't change during the wait.
        let (account_index, tracked_tx) = {
            let map = self.tracked.read().await;
            let lock = map.get(out_point).ok_or_else(|| {
                PlatformWalletError::AssetLockProofWait(format!(
                    "Asset lock {} is not tracked",
                    out_point.txid
                ))
            })?;
            (lock.account_index, lock.transaction.clone())
        };

        // Check if SPV already synced the proof before we started waiting.
        {
            let info = self.wallet_info.read().await;
            if let Some(record) = info
                .accounts
                .standard_bip44_accounts
                .get(&account_index)
                .and_then(|a| a.transactions.get(&out_point.txid))
            {
                if let TransactionContext::InChainLockedBlock(_) = &record.context {
                    if let Some(height) = record.height() {
                        return Ok(dpp::prelude::AssetLockProof::Chain(ChainAssetLockProof {
                            core_chain_locked_height: height,
                            out_point: *out_point,
                        }));
                    }
                }
            }
        }

        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return Err(PlatformWalletError::FinalityTimeout(out_point.txid));
            }

            tokio::select! {
                event = rx.recv() => {
                    match event {
                        Ok(PlatformWalletEvent::Spv(crate::events::SpvEvent::Sync(
                            dash_spv::sync::SyncEvent::InstantLockReceived { instant_lock, .. },
                        ))) => {
                            if instant_lock.txid == out_point.txid {
                                let proof = dpp::prelude::AssetLockProof::Instant(
                                    InstantAssetLockProof::new(
                                        instant_lock,
                                        tracked_tx,
                                        out_point.vout,
                                    ),
                                );
                                return Ok(proof);
                            }
                        }
                        Ok(PlatformWalletEvent::Spv(crate::events::SpvEvent::Sync(
                            dash_spv::sync::SyncEvent::ChainLockReceived { chain_lock, .. },
                        ))) => {
                            // Verify that our asset lock transaction is actually
                            // confirmed at a height <= the chain-locked height.
                            let info = self.wallet_info.read().await;
                            let record = info
                                .accounts
                                .standard_bip44_accounts
                                .get(&account_index)
                                .and_then(|a| a.transactions.get(&out_point.txid));

                            if let Some(record) = record {
                                if let Some(tx_height) = record.height() {
                                    if tx_height <= chain_lock.block_height {
                                        let proof = dpp::prelude::AssetLockProof::Chain(
                                            ChainAssetLockProof {
                                                core_chain_locked_height: tx_height,
                                                out_point: *out_point,
                                            },
                                        );
                                        return Ok(proof);
                                    }
                                }
                            }
                            // TX not yet confirmed or not in a chain-locked
                            // block — keep waiting for more events.
                        }
                        Ok(_) => {}
                        Err(broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(broadcast::error::RecvError::Closed) => {
                            return Err(PlatformWalletError::SpvError(
                                "Event channel closed".to_string(),
                            ));
                        }
                    }
                }
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
            let map = self.tracked.read().await;
            let lock = map.get(out_point).ok_or_else(|| {
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
                self.advance_asset_lock_status(out_point, AssetLockStatus::Broadcast, None)
                    .await?;
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
        self.advance_asset_lock_status(out_point, new_status, Some(proof.clone()))
            .await?;

        // 4. Re-derive the one-time private key.
        let private_key = {
            let map = self.tracked.read().await;
            let lock = map.get(out_point).ok_or_else(|| {
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

        // 3. Find the derivation path in the funding account.
        let wallet_info = self.wallet_info.read().await;
        let funding_account = match lock.funding_type {
            AssetLockFundingType::IdentityRegistration => {
                wallet_info.accounts.identity_registration.as_ref()
            }
            AssetLockFundingType::IdentityTopUp => wallet_info
                .accounts
                .identity_topup
                .get(&lock.identity_index),
            AssetLockFundingType::IdentityTopUpNotBound => {
                wallet_info.accounts.identity_topup_not_bound.as_ref()
            }
            AssetLockFundingType::IdentityInvitation => {
                wallet_info.accounts.identity_invitation.as_ref()
            }
            AssetLockFundingType::AssetLockAddressTopUp => {
                wallet_info.accounts.asset_lock_address_topup.as_ref()
            }
            AssetLockFundingType::AssetLockShieldedAddressTopUp => wallet_info
                .accounts
                .asset_lock_shielded_address_topup
                .as_ref(),
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

        // Drop the wallet_info lock before acquiring the wallet lock.
        drop(wallet_info);

        // 4. Derive the private key from the wallet's root key.
        let wallet = self.wallet.read().await;
        let secret_key = wallet.derive_private_key(&derivation_path).map_err(|e| {
            PlatformWalletError::AssetLockTransaction(format!(
                "Failed to derive private key for asset lock: {}",
                e
            ))
        })?;

        Ok(PrivateKey::new(secret_key, self.sdk.network))
    }
}

impl std::fmt::Debug for AssetLockManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AssetLockManager")
            .field("network", &self.sdk.network)
            .finish()
    }
}
