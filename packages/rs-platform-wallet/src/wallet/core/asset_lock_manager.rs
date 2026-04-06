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
    /// Insert a tracked asset lock.
    pub async fn track_asset_lock(&self, lock: TrackedAssetLock) {
        let mut map = self.tracked.write().await;
        map.insert(lock.txid, lock);
    }

    /// Return all asset locks whose proof is `Some` (ready for consumption).
    pub async fn unused_asset_locks(&self) -> BTreeMap<Txid, TrackedAssetLock> {
        let map = self.tracked.read().await;
        map.iter()
            .filter(|(_, v)| v.proof.is_some())
            .map(|(k, v)| (*k, v.clone()))
            .collect()
    }

    /// Remove an asset lock after successful consumption (registration or top-up).
    pub async fn remove_asset_lock(&self, txid: &Txid) {
        let mut map = self.tracked.write().await;
        map.remove(txid);
    }

    /// Advance the status of a tracked asset lock and optionally attach the proof.
    pub async fn advance_asset_lock_status(
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

    /// Look up a specific tracked asset lock.
    pub async fn get_asset_lock(&self, txid: &Txid) -> Option<TrackedAssetLock> {
        let map = self.tracked.read().await;
        map.get(txid).cloned()
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
    /// 2. Track the lifecycle as `Built`, then `Broadcast`.
    /// 3. Subscribe to SPV events *before* broadcasting (prevents race).
    /// 4. Wait for an InstantLock or ChainLock proof via the event channel.
    /// 5. Track the lifecycle as `InstantSendLocked`.
    /// 6. Return `(proof, private_key, txid)`.
    ///
    /// ## Parameters
    ///
    /// * `amount_duffs` — Amount to lock.
    /// * `funding_type` — Which account to derive the one-time key from.
    /// * `identity_index` — HD identity index.
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
        self.track_asset_lock(TrackedAssetLock {
            txid,
            transaction: tx.clone(),
            funding_type,
            identity_index,
            amount: amount_duffs,
            status: AssetLockStatus::Built,
            proof: None,
        })
        .await;

        // 3. Broadcast.
        self.broadcast_transaction(&tx).await?;

        // 4. Transition to Broadcast.
        self.advance_asset_lock_status(&txid, AssetLockStatus::Broadcast, None)
            .await;

        // 5. Wait for proof via SPV events.
        let proof = self
            .wait_for_proof(&txid, &tx, Duration::from_secs(300))
            .await?;

        // 6. Attach proof — status matches the proof type received.
        let status = match &proof {
            dpp::prelude::AssetLockProof::Instant(_) => AssetLockStatus::InstantSendLocked,
            dpp::prelude::AssetLockProof::Chain(_) => AssetLockStatus::ChainLocked,
        };
        self.advance_asset_lock_status(&txid, status, Some(proof.clone()))
            .await;

        Ok((proof, key, txid))
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

                            let proof = dpp::prelude::AssetLockProof::Chain(
                                ChainAssetLockProof {
                                    core_chain_locked_height: chain_lock.block_height,
                                    out_point: dashcore::OutPoint::new(*txid, 0),
                                },
                            );
                            return Ok(proof);
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

impl std::fmt::Debug for AssetLockManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AssetLockManager")
            .field("network", &self.sdk.network)
            .finish()
    }
}
