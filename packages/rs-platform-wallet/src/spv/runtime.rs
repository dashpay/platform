//! SPV client runtime — manages the DashSpvClient lifecycle.
//!
//! Extracted from `PlatformWalletManager` so the same SPV coordination can be
//! used both with a multi-wallet manager and with a standalone `PlatformWallet`.
//!
//! Asset-lock finality tracking (IS/CL proof waiting) is handled by
//! `AssetLockManager` directly — it subscribes to the shared event channel.

use std::sync::Arc;

use tokio::sync::{broadcast, RwLock};

use dashcore::sml::llmq_type::LLMQType;
use dashcore::{QuorumHash, Transaction};
use tokio_util::sync::CancellationToken;

use dash_spv::network::PeerNetworkManager;
use dash_spv::storage::DiskStorageManager;
use dash_spv::{ClientConfig, DashSpvClient, Hash};

use key_wallet_manager::{WalletInterface, WalletManager};

use crate::error::PlatformWalletError;
use crate::events::PlatformWalletEvent;
use crate::spv::event_forwarder::SpvEventForwarder;
use crate::wallet::platform_wallet::PlatformWalletInfo;

type SpvClient = DashSpvClient<
    WalletManager<PlatformWalletInfo>,
    PeerNetworkManager,
    DiskStorageManager,
    SpvEventForwarder,
>;

/// SPV client runtime — owns the `DashSpvClient` and tracks sync height.
///
/// Holds a reference to the shared `WalletManager<PlatformWalletInfo>` and
/// event channel at construction time, so callers just need `start(config)` /
/// `stop()`.
///
/// Asset-lock finality tracking (InstantLock / ChainLock waiting) is handled
/// directly by `AssetLockManager` via SPV event subscriptions — the runtime
/// only drives SPV sync and forwards events.
pub struct SpvRuntime {
    event_tx: broadcast::Sender<PlatformWalletEvent>,
    /// Shared `WalletManager<PlatformWalletInfo>` — implements `WalletInterface`,
    /// so DashSpvClient can drive block/mempool processing directly through it.
    /// `WalletManager` bumps its own structural revision when wallets are
    /// added/removed, so no external `notify_wallets_changed()` is needed.
    wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    client: RwLock<Option<SpvClient>>,
}

impl SpvRuntime {
    /// Create a new SPV runtime bound to a wallet manager and event channel.
    pub fn new(
        wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
        event_tx: broadcast::Sender<PlatformWalletEvent>,
    ) -> Self {
        Self {
            event_tx,
            wallet_manager,
            client: RwLock::new(None),
        }
    }

    /// Current synced height. Reads a plain field on WalletManager (sync, no
    /// per-wallet lock).
    pub fn synced_height(&self) -> u32 {
        self.wallet_manager
            .try_read()
            .map(|wm| wm.synced_height())
            .unwrap_or(0)
    }

    // TODO: We shoudln't do it
    /// Reset filter_committed_height to 0, forcing a filter rescan from
    /// birth_height on the next SPV start. Call BEFORE `run()`.
    ///
    /// Useful when wallet state isn't persisted: cached committed height
    /// from a previous run would skip historical blocks, leaving the
    /// wallet with zero balance.
    pub async fn reset_filter_committed_height(&self) {
        let mut wm = self.wallet_manager.write().await;
        wm.update_filter_committed_height(0).await;
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
            Arc::clone(&self.wallet_manager),
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
    /// The transaction will be relayed back to us through SPV's bloom filter
    /// matching, at which point the wallet adapter processes it and updates
    /// balances automatically.
    pub(crate) async fn broadcast_transaction(
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
        tracing::info!("SpvRuntime::run() starting client...");
        self.start(config).await?;
        tracing::info!("SpvRuntime::run() client started, entering sync loop");

        let is_cancelled = cancel_token.is_cancelled();
        tracing::info!(
            "SpvRuntime::run() cancel_token already cancelled? {}",
            is_cancelled
        );

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
                    tracing::info!("SpvRuntime::run() client.run() completed: {:?}", res.is_ok());
                    res.map_err(|e| PlatformWalletError::SpvError(e.to_string()))
                }
                _ = cancel_token.cancelled() => {
                    tracing::info!("SpvRuntime::run() cancel_token fired, cancelling client");
                    run_cancel.cancel();
                    Ok(())
                }
            }
        };

        tracing::info!(
            "SpvRuntime::run() exiting sync loop, result ok={}",
            result.is_ok()
        );
        // Always attempt cleanup, but don't let a stop() failure mask the
        // actual SPV run result.
        if let Err(e) = self.stop().await {
            tracing::warn!("SPV stop error during cleanup: {}", e);
        }
        tracing::info!("SpvRuntime::run() done");
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
