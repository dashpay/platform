//! Multi-wallet manager with SPV coordination.

use std::sync::Arc;

use tokio::sync::{broadcast, RwLock};

use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::Network;
use key_wallet_manager::WalletManager;

use crate::changeset::{Merge, PlatformWalletPersistence};

/// Options for creating a wallet via [`PlatformWalletManager::create_wallet_from_seed_bytes`].
#[derive(Debug, Clone, Default)]
pub struct WalletCreationOptions {
    /// Which accounts to create (BIP44, CoinJoin, identity, etc.).
    pub accounts: WalletAccountCreationOptions,
    /// Block height at which the wallet was created. SPV filter scanning
    /// starts from this height instead of genesis. `None` means scan from
    /// genesis (appropriate for wallets with unknown creation time).
    pub birth_height: Option<u32>,
}
use crate::error::PlatformWalletError;
use crate::events::PlatformWalletEvent;
use crate::spv::SpvRuntime;
use crate::wallet::core::WalletBalance;
use crate::wallet::persister::PlatformWalletPersisterBridge;
use crate::wallet::platform_wallet::{PlatformWalletInfo, WalletId};
use crate::wallet::PlatformWallet;

/// Multi-wallet coordinator with SPV sync and event broadcasting.
///
/// Mirrors the role of `key-wallet-manager`'s `WalletManager` for the Core
/// layer, but at the Platform level: manages multiple [`PlatformWallet`]
/// instances, coordinates SPV block/filter sync via [`SpvRuntime`], and
/// broadcasts unified [`PlatformWalletEvent`]s (sync progress, network
/// changes, wallet updates, finality proofs) to subscribers.
///
/// Internally holds a `WalletManager<PlatformWalletInfo>` that implements
/// `WalletInterface` for DashSpvClient, and a separate map of
/// `PlatformWallet` handles. Both share the same `Arc<RwLock<PlatformWalletInfo>>`
/// per wallet, so balance and UTXO updates from SPV are immediately visible
/// to all wallet operations.
pub struct PlatformWalletManager {
    sdk: Arc<dash_sdk::Sdk>,
    /// Core-layer wallet manager implementing `WalletInterface`.
    /// Shared with `SpvRuntime` so DashSpvClient drives block/mempool
    /// processing directly through it.
    wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    /// Platform-level wallet handles (sub-wallets, identity, dashpay, etc.).
    /// Interior mutability via `RwLock` so methods take `&self`.
    wallets: RwLock<std::collections::BTreeMap<WalletId, Arc<PlatformWallet>>>,
    event_tx: broadcast::Sender<PlatformWalletEvent>,
    spv: Arc<SpvRuntime>,
    persister: Arc<dyn PlatformWalletPersistence>,
}

impl PlatformWalletManager {
    /// Create a new PlatformWalletManager.
    pub fn new(sdk: Arc<dash_sdk::Sdk>, persister: Arc<dyn PlatformWalletPersistence>) -> Self {
        let (event_tx, _) = broadcast::channel(dash_spv::sync::DEFAULT_SYNC_EVENT_CAPACITY);
        let wallet_manager = Arc::new(RwLock::new(WalletManager::new(sdk.network)));
        let spv = Arc::new(SpvRuntime::new(
            Arc::clone(&wallet_manager),
            event_tx.clone(),
        ));
        Self {
            sdk,
            wallet_manager,
            wallets: RwLock::new(std::collections::BTreeMap::new()),
            event_tx,
            spv,
            persister,
        }
    }

    /// The SDK instance.
    pub fn sdk(&self) -> &dash_sdk::Sdk {
        &self.sdk
    }

    /// Access the SPV runtime for sync control.
    pub fn spv(&self) -> &SpvRuntime {
        &self.spv
    }

    // TODO: We can delete it and use core().broadcast() ? don't delete this todo
    /// Broadcast a transaction via SPV P2P peers.
    pub async fn broadcast_transaction(
        &self,
        tx: &dashcore::Transaction,
    ) -> Result<(), PlatformWalletError> {
        self.spv.broadcast_transaction(tx).await
    }

    /// Subscribe to platform wallet events.
    pub fn subscribe_events(&self) -> broadcast::Receiver<PlatformWalletEvent> {
        self.event_tx.subscribe()
    }

    /// Create a PlatformWallet from raw seed bytes, initialize persisted
    /// state, register it with the manager and return an `Arc` handle.
    ///
    /// The wallet is created with the manager's shared event channel so
    /// SPV events (InstantLock / ChainLock) reach the `AssetLockManager`.
    /// Persisted state (transactions, UTXOs, balances, identities) is loaded
    /// from the shared persister and applied before the wallet is registered,
    /// so the returned wallet is fully configured and ready for use.
    pub async fn create_wallet_from_seed_bytes(
        &self,
        network: Network,
        seed_bytes: [u8; 64],
        options: WalletCreationOptions,
    ) -> Result<Arc<PlatformWallet>, PlatformWalletError> {
        use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
        use key_wallet::wallet::Wallet;
        use key_wallet_manager::ManagedWalletState;

        let wallet =
            Wallet::from_seed_bytes(seed_bytes, network, options.accounts).map_err(|e| {
                PlatformWalletError::WalletCreation(format!(
                    "Failed to create wallet from seed bytes: {}",
                    e
                ))
            })?;
        let mut wallet_info = ManagedWalletInfo::from_wallet(&wallet);
        if let Some(height) = options.birth_height {
            wallet_info.set_birth_height(height);
        }
        let wallet_id = wallet_info.wallet_id;

        // Build ManagedWalletState with the persister bridge.
        let bridge = PlatformWalletPersisterBridge::new(wallet_id, Arc::clone(&self.persister));
        let managed_state = ManagedWalletState::new(wallet, wallet_info, bridge);
        let balance = Arc::new(WalletBalance::new());

        // Build the shared state Arc.
        let state = Arc::new(RwLock::new(PlatformWalletInfo {
            managed_state,
            balance: Arc::clone(&balance),
            identity_manager: crate::wallet::identity::IdentityManager::new(),
            tracked_asset_locks: std::collections::BTreeMap::new(),
            platform_address_balances: std::collections::BTreeMap::new(),
            token_watched: std::collections::BTreeMap::new(),
            token_balances: std::collections::BTreeMap::new(),
        }));

        // Insert into WalletManager (shares the same Arc).
        {
            let mut wm = self.wallet_manager.write().await;
            wm.insert_wallet_state(wallet_id, Arc::clone(&state))
                .map_err(|e| {
                    PlatformWalletError::WalletCreation(format!(
                        "Failed to register wallet in WalletManager: {}",
                        e
                    ))
                })?;
        }

        // Build the PlatformWallet handle from the shared state.
        let broadcaster = Arc::new(crate::broadcaster::SpvBroadcaster::new(Arc::clone(
            &self.spv,
        )));
        let platform_wallet = PlatformWallet::from_shared_state(
            Arc::clone(&self.sdk),
            wallet_id,
            state,
            balance,
            self.event_tx.clone(),
            Arc::clone(&self.persister),
            broadcaster,
        );

        // Load persisted state and apply it to the in-memory wallet.
        let changeset = platform_wallet.load_persisted().map_err(|e| {
            PlatformWalletError::WalletCreation(format!(
                "Failed to load persisted wallet state: {}",
                e
            ))
        })?;
        if !changeset.is_empty() {
            platform_wallet.apply(&changeset);
        }

        let platform_wallet = Arc::new(platform_wallet);

        // Register the PlatformWallet handle.
        {
            let mut wallets = self.wallets.write().await;
            wallets.insert(wallet_id, Arc::clone(&platform_wallet));
        }

        Ok(platform_wallet)
    }

    /// Remove a wallet from the manager.
    pub async fn remove_wallet(
        &self,
        wallet_id: &WalletId,
    ) -> Result<Arc<PlatformWallet>, PlatformWalletError> {
        // Remove from PlatformWallet handles.
        let removed = {
            let mut wallets = self.wallets.write().await;
            wallets
                .remove(wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(wallet_id)))?
        };
        // Remove from WalletManager.
        {
            let mut wm = self.wallet_manager.write().await;
            let _ = wm.remove_wallet(wallet_id);
        }
        Ok(removed)
    }

    /// Get a clone of a wallet by its ID.
    pub async fn get_wallet(&self, wallet_id: &WalletId) -> Option<Arc<PlatformWallet>> {
        let wallets = self.wallets.read().await;
        wallets.get(wallet_id).cloned()
    }

    /// List all wallet IDs.
    pub async fn wallet_ids(&self) -> Vec<WalletId> {
        let wallets = self.wallets.read().await;
        wallets.keys().copied().collect()
    }
}
