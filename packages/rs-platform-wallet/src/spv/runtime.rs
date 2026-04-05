//! SPV client runtime — manages DashSpvClient lifecycle and finality tracking.
//!
//! Extracted from `PlatformWalletManager` so the same SPV coordination can be
//! used both with a multi-wallet manager and with a standalone `PlatformWallet`.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use dashcore::Txid;
use tokio::sync::{broadcast, Mutex, RwLock};

use dash_spv::network::PeerNetworkManager;
use dash_spv::storage::DiskStorageManager;
use dash_spv::{ClientConfig, DashSpvClient};

use crate::error::PlatformWalletError;
use crate::events::{PlatformWalletEvent, SpvEvent};
use crate::spv::event_forwarder::SpvEventForwarder;
use crate::spv::wallet_adapter::SpvWalletAdapter;
use crate::wallet::platform_wallet::WalletId;
use crate::wallet::PlatformWallet;

type SpvClient =
    DashSpvClient<SpvWalletAdapter, PeerNetworkManager, DiskStorageManager, SpvEventForwarder>;

/// SPV client runtime — owns the `DashSpvClient`, tracks sync height, and
/// manages asset-lock finality proof waiting.
///
/// Holds references to the wallets collection and event channel at construction
/// time, so callers just need `start(config)` / `stop()`.
pub struct SpvRuntime {
    wallets: Arc<RwLock<BTreeMap<WalletId, Arc<PlatformWallet>>>>,
    event_tx: broadcast::Sender<PlatformWalletEvent>,
    synced_height: AtomicU32,
    /// Shared with `SpvWalletAdapter` — bump to signal bloom filter rebuild.
    monitor_revision: Arc<AtomicU64>,
    finality_waiters: Mutex<BTreeMap<Txid, Option<dpp::prelude::AssetLockProof>>>,
    client: RwLock<Option<SpvClient>>,
}

impl SpvRuntime {
    /// Create a new SPV runtime bound to a wallets collection and event channel.
    pub fn new(
        wallets: Arc<RwLock<BTreeMap<WalletId, Arc<PlatformWallet>>>>,
        event_tx: broadcast::Sender<PlatformWalletEvent>,
    ) -> Self {
        Self {
            wallets,
            event_tx,
            synced_height: AtomicU32::new(0),
            monitor_revision: Arc::new(AtomicU64::new(0)),
            finality_waiters: Mutex::new(BTreeMap::new()),
            client: RwLock::new(None),
        }
    }

    /// Current synced height.
    pub fn synced_height(&self) -> u32 {
        self.synced_height.load(Ordering::Relaxed)
    }

    /// Signal that the wallet set changed (added/removed).
    /// SPV will rebuild the bloom filter on the next tick.
    pub fn notify_wallets_changed(&self) {
        self.monitor_revision.fetch_add(1, Ordering::Relaxed);
    }

    /// Start SPV sync.
    pub async fn start(&self, config: ClientConfig) -> Result<(), PlatformWalletError> {
        {
            let running = self.client.read().await;
            if running.is_some() {
                return Err(PlatformWalletError::SpvAlreadyRunning);
            }
        }

        let adapter = SpvWalletAdapter::new(
            Arc::clone(&self.wallets),
            self.event_tx.clone(),
            Arc::clone(&self.monitor_revision),
        );
        let forwarder = SpvEventForwarder::new(self.event_tx.clone());

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
    pub async fn wait_for_finality(
        &self,
        txid: &Txid,
        timeout: Duration,
    ) -> Result<dpp::prelude::AssetLockProof, PlatformWalletError> {
        let deadline = tokio::time::Instant::now() + timeout;
        let mut rx = self.event_tx.subscribe();

        loop {
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
                        Ok(PlatformWalletEvent::Spv(SpvEvent::Sync(dash_spv::sync::SyncEvent::InstantLockReceived { instant_lock, .. }))) => {
                            if instant_lock.txid == *txid {
                                // TODO: Build proper InstantAssetLockProof from instant_lock data
                                let mut waiters = self.finality_waiters.lock().await;
                                if let Some(entry) = waiters.get_mut(txid) {
                                    *entry = Some(dpp::prelude::AssetLockProof::default());
                                }
                            }
                        }
                        Ok(PlatformWalletEvent::Spv(SpvEvent::Sync(dash_spv::sync::SyncEvent::ChainLockReceived { .. }))) => {
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

impl std::fmt::Debug for SpvRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpvRuntime")
            .field("synced_height", &self.synced_height.load(Ordering::Relaxed))
            .finish()
    }
}
