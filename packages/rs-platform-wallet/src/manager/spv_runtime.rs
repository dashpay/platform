//! SPV client runtime — manages DashSpvClient lifecycle and finality tracking.
//!
//! Extracted from `PlatformWalletManager` so the same SPV coordination can be
//! used both with a multi-wallet manager and with a standalone `PlatformWallet`.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use dashcore::Txid;
use tokio::sync::{broadcast, Mutex, RwLock};

use dash_spv::network::PeerNetworkManager;
use dash_spv::storage::DiskStorageManager;
use dash_spv::{ClientConfig, DashSpvClient};

use crate::error::PlatformWalletError;
use crate::events::PlatformWalletEvent;
use crate::manager::spv_event_forwarder::SpvEventForwarder;
use crate::manager::spv_wallet_adapter::SpvWalletAdapter;
use crate::wallet::PlatformWallet;

type SpvClient = DashSpvClient<SpvWalletAdapter, PeerNetworkManager, DiskStorageManager, SpvEventForwarder>;

/// SPV client runtime — owns the `DashSpvClient`, tracks sync height, and
/// manages asset-lock finality proof waiting.
///
/// Can be used inside [`PlatformWalletManager`](super::PlatformWalletManager)
/// or attached to a standalone [`PlatformWallet`].
pub struct SpvRuntime {
    /// Current synced block height.
    synced_height: AtomicU32,
    /// Transactions waiting for finality proof (InstantLock or ChainLock).
    /// Registered BEFORE broadcast, updated when SPV event arrives.
    finality_waiters: Mutex<BTreeMap<Txid, Option<dpp::prelude::AssetLockProof>>>,
    /// The running SPV client, if started.
    client: RwLock<Option<SpvClient>>,
}

impl SpvRuntime {
    /// Create a new SPV runtime (not yet started).
    pub fn new() -> Self {
        Self {
            synced_height: AtomicU32::new(0),
            finality_waiters: Mutex::new(BTreeMap::new()),
            client: RwLock::new(None),
        }
    }

    /// Current synced height.
    pub fn synced_height(&self) -> u32 {
        self.synced_height.load(Ordering::Relaxed)
    }

    /// Start SPV sync with the given configuration.
    ///
    /// Creates a `DashSpvClient` that connects to the Dash P2P network,
    /// syncs block headers and compact block filters, and processes
    /// matching blocks through the wallet adapter.
    pub async fn start(
        &self,
        config: ClientConfig,
        wallet: PlatformWallet,
        event_tx: broadcast::Sender<PlatformWalletEvent>,
    ) -> Result<(), PlatformWalletError> {
        {
            let running = self.client.read().await;
            if running.is_some() {
                return Err(PlatformWalletError::SpvAlreadyRunning);
            }
        }

        let adapter = SpvWalletAdapter::new(wallet, event_tx.clone());
        let forwarder = SpvEventForwarder::new(event_tx);

        let network_manager = PeerNetworkManager::new(&config)
            .await
            .map_err(|e| PlatformWalletError::SpvError(e.to_string()))?;
        let storage_manager = DiskStorageManager::new(&config)
            .await
            .map_err(|e| PlatformWalletError::SpvError(e.to_string()))?;

        let spv_client = DashSpvClient::new(
            config,
            network_manager,
            storage_manager,
            Arc::new(RwLock::new(adapter)),
            Arc::new(forwarder),
        )
        .await
        .map_err(|e| PlatformWalletError::SpvError(e.to_string()))?;

        let mut client = self.client.write().await;
        *client = Some(spv_client);

        Ok(())
    }

    /// Stop SPV sync gracefully.
    pub async fn stop(&self) -> Result<(), PlatformWalletError> {
        let mut client = self.client.write().await;
        if let Some(c) = client.take() {
            c.stop()
                .await
                .map_err(|e| PlatformWalletError::SpvError(e.to_string()))?;
        }
        Ok(())
    }

    // ── Finality tracking ──────────────────────────────────────────────

    /// Register a transaction to wait for finality proof.
    /// Call BEFORE broadcasting to prevent race where proof arrives first.
    pub async fn register_for_finality(&self, txid: Txid) {
        let mut waiters = self.finality_waiters.lock().await;
        waiters.insert(txid, None);
    }

    /// Wait for a finality proof (InstantLock or ChainLock) for a registered
    /// transaction.
    ///
    /// Listens on the provided event receiver until the matching proof arrives
    /// or the timeout expires.
    pub async fn wait_for_finality(
        &self,
        txid: &Txid,
        timeout: Duration,
        event_tx: &broadcast::Sender<PlatformWalletEvent>,
    ) -> Result<dpp::prelude::AssetLockProof, PlatformWalletError> {
        let deadline = tokio::time::Instant::now() + timeout;
        let mut rx = event_tx.subscribe();

        loop {
            // Check if proof already arrived
            {
                let waiters = self.finality_waiters.lock().await;
                if let Some(Some(proof)) = waiters.get(txid) {
                    let proof = proof.clone();
                    drop(waiters);
                    self.finality_waiters.lock().await.remove(txid);
                    return Ok(proof);
                }
            }

            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                self.finality_waiters.lock().await.remove(txid);
                return Err(PlatformWalletError::FinalityTimeout(*txid));
            }

            tokio::select! {
                event = rx.recv() => {
                    match event {
                        Ok(PlatformWalletEvent::Sync(dash_spv::sync::SyncEvent::InstantLockReceived { instant_lock, .. })) => {
                            if instant_lock.txid == *txid {
                                // TODO: Build proper InstantAssetLockProof from instant_lock data
                                let mut waiters = self.finality_waiters.lock().await;
                                if let Some(entry) = waiters.get_mut(txid) {
                                    *entry = Some(dpp::prelude::AssetLockProof::default());
                                }
                            }
                        }
                        Ok(PlatformWalletEvent::Sync(dash_spv::sync::SyncEvent::ChainLockReceived { .. })) => {
                            // TODO: Build proper ChainAssetLockProof with height + outpoint
                            let mut waiters = self.finality_waiters.lock().await;
                            if let Some(entry) = waiters.get_mut(txid) {
                                if entry.is_none() {
                                    *entry = Some(dpp::prelude::AssetLockProof::default());
                                }
                            }
                        }
                        Ok(_) => {}
                        Err(broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(broadcast::error::RecvError::Closed) => {
                            self.finality_waiters.lock().await.remove(txid);
                            return Err(PlatformWalletError::SpvError(
                                "Event channel closed".to_string(),
                            ));
                        }
                    }
                }
                _ = tokio::time::sleep(remaining) => {
                    self.finality_waiters.lock().await.remove(txid);
                    return Err(PlatformWalletError::FinalityTimeout(*txid));
                }
            }
        }
    }
}

impl Default for SpvRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for SpvRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpvRuntime")
            .field("synced_height", &self.synced_height.load(Ordering::Relaxed))
            .finish()
    }
}
