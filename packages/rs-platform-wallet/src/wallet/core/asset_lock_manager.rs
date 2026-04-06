//! Asset lock lifecycle manager.
//!
//! Encapsulates all asset lock operations: building transactions, broadcasting,
//! waiting for proofs, and tracking lifecycle status. Shared across sub-wallets
//! via `Arc<AssetLockManager>`.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use dashcore::Address as DashAddress;
use dashcore::{PrivateKey, Transaction, TxOut, Txid};
use key_wallet::wallet::managed_wallet_info::asset_lock_builder::{
    AssetLockFundingType, CreditOutputFunding,
};
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;
use tokio::sync::{broadcast, RwLock};

use crate::error::PlatformWalletError;
use crate::events::PlatformWalletEvent;

use super::asset_lock::{AssetLockStatus, TrackedAssetLock};

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
    /// Tracked asset locks, keyed by transaction ID.
    ///
    /// Tracks each asset lock from build through broadcast and finality.
    /// Removed once consumed by a successful identity operation.
    tracked: Arc<RwLock<BTreeMap<Txid, TrackedAssetLock>>>,
}

impl AssetLockManager {
    /// Create a new `AssetLockManager`.
    pub(crate) fn new(
        sdk: Arc<dash_sdk::Sdk>,
        wallet: Arc<RwLock<Wallet>>,
        wallet_info: Arc<RwLock<ManagedWalletInfo>>,
        event_tx: broadcast::Sender<PlatformWalletEvent>,
    ) -> Self {
        Self {
            sdk,
            wallet,
            wallet_info,
            event_tx,
            tracked: Arc::new(RwLock::new(BTreeMap::new())),
        }
    }
}

// ---------------------------------------------------------------------------
// Asset lock tracking
// ---------------------------------------------------------------------------

impl AssetLockManager {
    /// Remove an asset lock after successful consumption (registration or top-up).
    pub(crate) async fn remove_asset_lock(&self, txid: &Txid) {
        let mut map = self.tracked.write().await;
        map.remove(txid);
    }

    /// Advance the status of a tracked asset lock and optionally attach the proof.
    async fn advance_asset_lock_status(
        &self,
        txid: &Txid,
        new_status: AssetLockStatus,
        proof: Option<dpp::prelude::AssetLockProof>,
    ) {
        let mut map = self.tracked.write().await;
        if let Some(entry) = map.get_mut(txid) {
            entry.status = new_status;
            if proof.is_some() {
                entry.proof = proof;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Blocking accessor (for synchronous / evo-tool contexts)
// ---------------------------------------------------------------------------

impl AssetLockManager {
    /// Blocking version of [`recover_asset_lock`](Self::recover_asset_lock).
    ///
    /// Uses `tokio::sync::RwLock::blocking_write` -- must NOT be called from
    /// within a tokio async context.
    pub fn blocking_recover_asset_lock(
        &self,
        tx: Transaction,
        amount: u64,
        funding_type: AssetLockFundingType,
        identity_index: u32,
        proof: Option<dpp::prelude::AssetLockProof>,
    ) {
        let txid = tx.txid();

        let mut map = self.tracked.blocking_write();
        if map.contains_key(&txid) {
            return;
        }

        let status = match &proof {
            Some(dpp::prelude::AssetLockProof::Instant(_)) => AssetLockStatus::InstantSendLocked,
            Some(dpp::prelude::AssetLockProof::Chain(_)) => AssetLockStatus::ChainLocked,
            None => AssetLockStatus::Broadcast,
        };

        let lock = TrackedAssetLock {
            txid,
            transaction: tx,
            funding_type,
            identity_index,
            amount,
            status,
            proof,
        };
        map.insert(txid, lock);
    }
}

// ---------------------------------------------------------------------------
// Transaction broadcasting (asset-lock-specific)
// ---------------------------------------------------------------------------

impl AssetLockManager {
    /// Broadcast a signed transaction to the network via DAPI.
    ///
    /// Serializes the transaction using consensus encoding and sends it
    /// through the SDK's DAPI client using the `BroadcastTransactionRequest`
    /// gRPC call.
    ///
    /// Returns the transaction ID on success.
    pub async fn broadcast_transaction(
        &self,
        transaction: &Transaction,
    ) -> Result<dashcore::Txid, PlatformWalletError> {
        use dash_sdk::dapi_client::{DapiRequestExecutor, IntoInner, RequestSettings};
        use dash_sdk::dapi_grpc::core::v0::BroadcastTransactionRequest;
        use dashcore::consensus;

        let tx_bytes = consensus::serialize(transaction);

        let request = BroadcastTransactionRequest {
            transaction: tx_bytes,
            allow_high_fees: false,
            bypass_limits: false,
        };

        let _response = self
            .sdk
            .execute(request, RequestSettings::default())
            .await
            .into_inner()
            .map_err(|e| {
                PlatformWalletError::TransactionBroadcast(format!("DAPI broadcast failed: {}", e))
            })?;

        Ok(transaction.txid())
    }
}

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
    /// * `funding_type` — Which account to derive the one-time key from
    ///   (e.g., `IdentityRegistration`, `IdentityTopUp`).
    /// * `identity_index` — Identity index (used by `IdentityTopUp`, ignored by others).
    pub async fn build_asset_lock_transaction(
        &self,
        amount_duffs: u64,
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

        // 3. Delegate to the key-wallet builder (account 0 for UTXOs).
        let result = wallet_info
            .build_asset_lock(&wallet, 0, vec![funding], DEFAULT_FEE_PER_KB)
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
            other => {
                return Err(PlatformWalletError::AssetLockTransaction(format!(
                    "Unsupported funding type for asset lock: {:?}",
                    other
                )));
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
    /// * `funding_type` — Which account to derive the one-time key from.
    /// * `identity_index` — HD identity index (for `IdentityTopUp`, this is
    ///   the registration index identifying which identity is being topped up).
    pub async fn create_funded_asset_lock_proof(
        &self,
        amount_duffs: u64,
        funding_type: AssetLockFundingType,
        identity_index: u32,
    ) -> Result<(dpp::prelude::AssetLockProof, PrivateKey, Txid), PlatformWalletError> {
        // 1. Build the asset lock transaction.
        let (tx, key) = self
            .build_asset_lock_transaction(amount_duffs, funding_type, identity_index)
            .await?;

        let txid = tx.txid();

        // 2. Track as Built.
        {
            let mut map = self.tracked.write().await;
            map.insert(
                txid,
                TrackedAssetLock {
                    txid,
                    transaction: tx.clone(),
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
        self.broadcast_transaction(&tx).await?;

        // 4. Transition to Broadcast.
        self.advance_asset_lock_status(&txid, AssetLockStatus::Broadcast, None)
            .await;

        // 5. Wait for proof via SPV events.
        let proof = self
            .wait_for_proof(&txid, &tx, Duration::from_secs(300))
            .await?;

        // 5b. If we got an IS-lock proof, check whether the transaction is
        // old enough that Platform might reject it. If so, upgrade to a
        // ChainLock proof proactively.
        let proof = self.validate_or_upgrade_proof(proof, &txid).await?;

        // 6. Attach proof — status matches the proof type received.
        let status = match &proof {
            dpp::prelude::AssetLockProof::Instant(_) => AssetLockStatus::InstantSendLocked,
            dpp::prelude::AssetLockProof::Chain(_) => AssetLockStatus::ChainLocked,
        };
        self.advance_asset_lock_status(&txid, status, Some(proof.clone()))
            .await;

        Ok((proof, key, txid))
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
        txid: &Txid,
    ) -> Result<dpp::prelude::AssetLockProof, PlatformWalletError> {
        use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;

        if !matches!(&proof, dpp::prelude::AssetLockProof::Instant(_)) {
            return Ok(proof);
        }

        // Fetch transaction info from DAPI to check confirmation depth.
        let tx_info = self.get_transaction_info(txid).await?;

        if tx_info.is_chain_locked && tx_info.height > 0 && tx_info.confirmations > 8 {
            // Transaction is old enough that the IS-lock quorum may have
            // rotated. Check if Platform has verified this Core block.
            let platform_height = self.get_platform_core_chain_locked_height().await?;

            if tx_info.height <= platform_height {
                tracing::info!(
                    "Upgrading IS-lock proof to ChainLock proof for tx {} \
                     (height={}, confirmations={}, platform_cl_height={})",
                    txid,
                    tx_info.height,
                    tx_info.confirmations,
                    platform_height,
                );

                return Ok(dpp::prelude::AssetLockProof::Chain(ChainAssetLockProof {
                    core_chain_locked_height: tx_info.height,
                    out_point: dashcore::OutPoint::new(*txid, 0),
                }));
            }
        }

        Ok(proof)
    }

    /// Get transaction info from key-wallet's ManagedWalletInfo (local, no DAPI call).
    ///
    /// Asset lock transactions spend from the standard BIP44 account, so the
    /// transaction record lives there. Falls back to scanning all accounts.
    async fn get_transaction_info(
        &self,
        txid: &Txid,
    ) -> Result<TransactionInfo, PlatformWalletError> {
        use key_wallet::transaction_checking::TransactionContext;

        let info = self.wallet_info.read().await;
        let synced_height = info.metadata.synced_height;

        // Check standard BIP44 account 0 first (most likely location).
        let record = info
            .accounts
            .standard_bip44_accounts
            .get(&0)
            .and_then(|a| a.transactions.get(txid))
            .or_else(|| {
                // Fallback: scan all accounts.
                info.accounts
                    .all_accounts()
                    .iter()
                    .find_map(|a| a.transactions.get(txid))
            });

        match record {
            Some(record) => Ok(TransactionInfo {
                is_chain_locked: matches!(
                    record.context,
                    TransactionContext::InChainLockedBlock(_)
                ),
                height: record.height().unwrap_or(0),
                confirmations: record.confirmations(synced_height),
            }),
            None => Err(PlatformWalletError::AssetLockProofWait(format!(
                "Transaction {} not found in wallet",
                txid
            ))),
        }
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

    /// Attempt to upgrade an IS-lock proof to a ChainLock proof after a
    /// Platform rejection.
    ///
    /// This is called from the recovery layer (Layer 2) when
    /// `put_to_platform` fails with an `InvalidInstantAssetLockProofSignature`
    /// error. It fetches the transaction info and constructs a ChainLock proof
    /// if the transaction is chain-locked and Platform has verified the block.
    pub(crate) async fn upgrade_to_chain_lock_proof(
        &self,
        txid: &Txid,
    ) -> Result<dpp::prelude::AssetLockProof, PlatformWalletError> {
        use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;

        let tx_info = self.get_transaction_info(txid).await?;

        if !tx_info.is_chain_locked || tx_info.height == 0 {
            return Err(PlatformWalletError::AssetLockNotChainLocked(format!(
                "Transaction {} is not chain-locked (is_chain_locked={}, height={})",
                txid, tx_info.is_chain_locked, tx_info.height
            )));
        }

        let platform_height = self.get_platform_core_chain_locked_height().await?;

        if tx_info.height > platform_height {
            return Err(PlatformWalletError::AssetLockExpired(format!(
                "Transaction {} is at height {} but Platform has only verified up to height {}",
                txid, tx_info.height, platform_height
            )));
        }

        tracing::info!(
            "Building ChainLock proof for tx {} after IS-lock rejection \
             (height={}, platform_cl_height={})",
            txid,
            tx_info.height,
            platform_height,
        );

        Ok(dpp::prelude::AssetLockProof::Chain(ChainAssetLockProof {
            core_chain_locked_height: tx_info.height,
            out_point: dashcore::OutPoint::new(*txid, 0),
        }))
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
        txid: &Txid,
        tx: &Transaction,
        timeout: Duration,
    ) -> Result<dpp::prelude::AssetLockProof, PlatformWalletError> {
        use dpp::identity::state_transition::asset_lock_proof::InstantAssetLockProof;

        let deadline = tokio::time::Instant::now() + timeout;
        let mut rx = self.event_tx.subscribe();

        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return Err(PlatformWalletError::FinalityTimeout(*txid));
            }

            tokio::select! {
                event = rx.recv() => {
                    match event {
                        #[cfg(feature = "manager")]
                        Ok(PlatformWalletEvent::Spv(crate::events::SpvEvent::Sync(
                            dash_spv::sync::SyncEvent::InstantLockReceived { instant_lock, .. },
                        ))) => {
                            if instant_lock.txid == *txid {
                                let proof = dpp::prelude::AssetLockProof::Instant(
                                    InstantAssetLockProof::new(
                                        instant_lock,
                                        tx.clone(),
                                        0,
                                    ),
                                );
                                return Ok(proof);
                            }
                        }
                        #[cfg(feature = "manager")]
                        Ok(PlatformWalletEvent::Spv(crate::events::SpvEvent::Sync(
                            dash_spv::sync::SyncEvent::ChainLockReceived { chain_lock, .. },
                        ))) => {
                            use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;

                            // Verify that our asset lock transaction is actually
                            // confirmed at a height <= the chain-locked height.
                            // A ChainLock on block N guarantees finality for all
                            // blocks up to and including N, but we must confirm
                            // our TX is actually in one of those blocks.
                            let info = self.wallet_info.read().await;
                            let record = info
                                .accounts
                                .standard_bip44_accounts
                                .get(&0)
                                .and_then(|a| a.transactions.get(txid))
                                .or_else(|| {
                                    info.accounts
                                        .all_accounts()
                                        .iter()
                                        .find_map(|a| a.transactions.get(txid))
                                });

                            if let Some(record) = record {
                                if let Some(tx_height) = record.height() {
                                    if tx_height <= chain_lock.block_height {
                                        let proof = dpp::prelude::AssetLockProof::Chain(
                                            ChainAssetLockProof {
                                                core_chain_locked_height: tx_height,
                                                out_point: dashcore::OutPoint::new(*txid, 0),
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
                    return Err(PlatformWalletError::FinalityTimeout(*txid));
                }
            }
        }
    }
}

/// Transaction info returned by DAPI's Core gRPC endpoint.
struct TransactionInfo {
    is_chain_locked: bool,
    height: u32,
    confirmations: u32,
}

impl std::fmt::Debug for AssetLockManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AssetLockManager")
            .field("network", &self.sdk.network)
            .finish()
    }
}
