//! SPV client runtime — manages the DashSpvClient lifecycle.
//!
//! Extracted from `PlatformWalletManager` so the same SPV coordination can be
//! used both with a multi-wallet manager and with a standalone `PlatformWallet`.
//!
//! Asset-lock finality tracking (IS/CL proof waiting) is handled by
//! `AssetLockManager` directly — it subscribes to the shared event channel.

use std::collections::BTreeMap;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use tokio::sync::{broadcast, RwLock};

use dashcore::sml::llmq_type::LLMQType;
use dashcore::{QuorumHash, Transaction};
use tokio_util::sync::CancellationToken;

use dash_spv::network::PeerNetworkManager;
use dash_spv::storage::DiskStorageManager;
use dash_spv::{ClientConfig, DashSpvClient, Hash};

use key_wallet_manager::WalletInterface;

use crate::error::PlatformWalletError;
use crate::events::PlatformWalletEvent;
use crate::spv::event_forwarder::SpvEventForwarder;
use crate::spv::wallet_adapter::SpvWalletAdapter;
use crate::wallet::platform_wallet::WalletId;
use crate::wallet::PlatformWallet;

type SpvClient =
    DashSpvClient<SpvWalletAdapter, PeerNetworkManager, DiskStorageManager, SpvEventForwarder>;

/// SPV client runtime — owns the `DashSpvClient` and tracks sync height.
///
/// Holds references to the wallets collection and event channel at construction
/// time, so callers just need `start(config)` / `stop()`.
///
/// Asset-lock finality tracking (InstantLock / ChainLock waiting) is handled
/// directly by `AssetLockManager` via SPV event subscriptions — the runtime
/// only drives SPV sync and forwards events.
pub struct SpvRuntime {
    event_tx: broadcast::Sender<PlatformWalletEvent>,
    adapter: Arc<RwLock<SpvWalletAdapter>>,
    client: RwLock<Option<SpvClient>>,
}

impl SpvRuntime {
    /// Create a new SPV runtime bound to a wallets collection and event channel.
    pub fn new(
        wallets: Arc<RwLock<BTreeMap<WalletId, Arc<PlatformWallet>>>>,
        event_tx: broadcast::Sender<PlatformWalletEvent>,
    ) -> Self {
        let adapter = Arc::new(RwLock::new(SpvWalletAdapter::new(wallets)));
        Self {
            event_tx,
            adapter,
            client: RwLock::new(None),
        }
    }

    // TODO: Not sure blocking method is good idea here
    /// Current synced height.
    pub fn synced_height(&self) -> u32 {
        self.adapter
            .try_read()
            .map(|a| a.synced_height())
            .unwrap_or(0)
    }

    // TODO: it needs to be public? not sure blocking is good.
    /// Signal that the wallet set changed (added/removed).
    /// SPV will rebuild the bloom filter on the next tick.
    pub fn notify_wallets_changed(&self) {
        if let Ok(adapter) = self.adapter.try_read() {
            adapter.monitor_revision.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Start SPV sync.
    pub async fn start(&self, config: ClientConfig) -> Result<(), PlatformWalletError> {
        {
            let running = self.client.read().await;
            if running.is_some() {
                return Err(PlatformWalletError::SpvAlreadyRunning);
            }
        }

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
            Arc::clone(&self.adapter),
            Arc::new(forwarder),
        )
        .await
        .map_err(|e| PlatformWalletError::SpvError(e.to_string()))?;

        let mut client = self.client.write().await;
        *client = Some(spv_client);

        Ok(())
    }

    /// Check whether the SPV client has been started (i.e. `start()` was called
    /// and the client exists).
    pub fn is_started(&self) -> bool {
        self.client.try_read().map(|c| c.is_some()).unwrap_or(false)
    }

    /// Broadcast a transaction to all connected SPV peers.
    ///
    /// After a successful broadcast the transaction is also fed into the local
    /// wallet adapter so that balances update immediately without waiting for
    /// SPV to relay it back.
    pub async fn broadcast_transaction(
        &self,
        tx: &Transaction,
    ) -> Result<(), PlatformWalletError> {
        let client_guard = self.client.read().await;
        let client = client_guard
            .as_ref()
            .ok_or(PlatformWalletError::SpvNotRunning)?;

        client
            .broadcast_transaction(tx)
            .await
            .map_err(|e| PlatformWalletError::SpvError(e.to_string()))?;

        // Process the transaction locally so the wallet sees it immediately.
        let mut adapter = self.adapter.write().await;
        let _ = adapter.process_mempool_transaction(tx, false).await;

        Ok(())
    }

    /// Look up a quorum public key via the SPV masternode state.
    ///
    /// Returns the 48-byte BLS public key for the quorum identified by
    /// `(quorum_type, quorum_hash)` at the given chain-locked `height`.
    pub async fn get_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        height: u32,
    ) -> Result<[u8; 48], PlatformWalletError> {
        let client_guard = self.client.read().await;
        let client = client_guard
            .as_ref()
            .ok_or(PlatformWalletError::SpvNotRunning)?;

        let llmq_type = LLMQType::from(quorum_type as u8);
        let qh = QuorumHash::from_byte_array(quorum_hash).reverse();

        let quorum = client
            .get_quorum_at_height(height, llmq_type, qh)
            .await
            .map_err(|e| PlatformWalletError::SpvError(e.to_string()))?;

        Ok(*quorum.quorum_entry.quorum_public_key.as_ref())
    }

    /// Run the SPV sync loop.
    ///
    /// Creates the client via [`start`](Self::start), then drives
    /// `client.run(cancel)` until the cancellation token fires.  On exit the
    /// client is stopped via [`stop`](Self::stop).
    pub async fn run(
        &self,
        config: ClientConfig,
        cancel_token: CancellationToken,
    ) -> Result<(), PlatformWalletError> {
        self.start(config).await?;

        let result = {
            let client_guard = self.client.read().await;
            let client = client_guard
                .as_ref()
                .ok_or(PlatformWalletError::SpvNotRunning)?;

            let run_cancel = CancellationToken::new();
            let run_future = client.run(run_cancel.clone());
            tokio::pin!(run_future);

            tokio::select! {
                res = &mut run_future => {
                    res.map_err(|e| PlatformWalletError::SpvError(e.to_string()))
                }
                _ = cancel_token.cancelled() => {
                    run_cancel.cancel();
                    Ok(())
                }
            }
        };

        self.stop().await?;
        result
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
}

impl std::fmt::Debug for SpvRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpvRuntime")
            .field("synced_height", &self.synced_height())
            .finish()
    }
}
