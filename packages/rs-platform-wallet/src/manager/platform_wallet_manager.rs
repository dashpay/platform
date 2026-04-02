//! Multi-wallet manager with SPV coordination.

use std::collections::BTreeMap;
use std::time::Duration;

use dashcore::Txid;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::{Mnemonic, Network};
use tokio::sync::{broadcast, RwLock};

use crate::error::PlatformWalletError;
use crate::events::PlatformWalletEvent;
use crate::manager::spv_runtime::SpvRuntime;
use crate::wallet::platform_wallet::WalletId;
use crate::wallet::PlatformWallet;

use dash_spv::ClientConfig;

/// Multi-wallet coordinator with SPV sync and event broadcasting.
///
/// Mirrors the role of `key-wallet-manager`'s `WalletManager` for the Core
/// layer, but at the Platform level: manages multiple [`PlatformWallet`]
/// instances, coordinates SPV block/filter sync via [`SpvRuntime`], and
/// broadcasts unified [`PlatformWalletEvent`]s (sync progress, network
/// changes, wallet updates, finality proofs) to subscribers.
///
/// Each managed [`PlatformWallet`] shares its underlying `Wallet` and
/// `ManagedWalletInfo` with the SPV adapter through `Arc<RwLock<…>>`,
/// so balance and UTXO updates from SPV are immediately visible to all
/// wallet operations.
///
/// # SPV lifecycle
///
/// - [`start_spv`](Self::start_spv) — starts SPV sync via [`SpvRuntime`].
/// - [`stop_spv`](Self::stop_spv) — graceful shutdown.
///
/// # Finality tracking
///
/// - [`register_for_finality`](Self::register_for_finality) — register a
///   txid *before* broadcasting to prevent proof-arrival races.
/// - [`wait_for_finality`](Self::wait_for_finality) — async wait for an
///   InstantLock or ChainLock event for the registered txid.
pub struct PlatformWalletManager {
    sdk: dash_sdk::Sdk,
    network: Network,
    wallets: RwLock<BTreeMap<WalletId, PlatformWallet>>,
    event_tx: broadcast::Sender<PlatformWalletEvent>,
    spv: SpvRuntime,
}

impl PlatformWalletManager {
    /// Create a new PlatformWalletManager.
    pub fn new(sdk: dash_sdk::Sdk, network: Network) -> Self {
        let (event_tx, _) = broadcast::channel(256);
        Self {
            sdk,
            network,
            wallets: RwLock::new(BTreeMap::new()),
            event_tx,
            spv: SpvRuntime::new(),
        }
    }

    /// Create a wallet from a BIP-39 mnemonic and add it to the manager.
    pub async fn create_wallet_from_mnemonic(
        &self,
        mnemonic: &str,
        passphrase: &str,
        options: WalletAccountCreationOptions,
    ) -> Result<PlatformWallet, PlatformWalletError> {
        let wallet = PlatformWallet::from_mnemonic(
            self.sdk.clone(),
            self.network,
            mnemonic,
            passphrase,
            options,
        )?;
        self.insert_and_return(wallet).await
    }

    /// Create a wallet with a randomly generated mnemonic.
    /// Returns the wallet and the generated mnemonic.
    pub async fn create_wallet_with_random_mnemonic(
        &self,
        options: WalletAccountCreationOptions,
    ) -> Result<(PlatformWallet, Mnemonic), PlatformWalletError> {
        let (wallet, mnemonic) = PlatformWallet::random(self.sdk.clone(), self.network, options)?;
        let wallet = self.insert_and_return(wallet).await?;
        Ok((wallet, mnemonic))
    }

    /// Import a wallet from an extended private key string.
    pub async fn import_wallet_from_extended_key(
        &self,
        xprv: &str,
        options: WalletAccountCreationOptions,
    ) -> Result<PlatformWallet, PlatformWalletError> {
        let wallet = PlatformWallet::from_extended_key(self.sdk.clone(), xprv, options)?;
        self.insert_and_return(wallet).await
    }

    /// Import a watch-only wallet from an extended public key string.
    pub async fn import_wallet_from_xpub(
        &self,
        xpub: &str,
    ) -> Result<PlatformWallet, PlatformWalletError> {
        let wallet = PlatformWallet::from_xpub(self.sdk.clone(), self.network, xpub)?;
        self.insert_and_return(wallet).await
    }

    /// Remove a wallet from the manager.
    pub async fn remove_wallet(
        &self,
        wallet_id: &WalletId,
    ) -> Result<PlatformWallet, PlatformWalletError> {
        let mut wallets = self.wallets.write().await;
        wallets
            .remove(wallet_id)
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(wallet_id)))
    }

    /// Get a clone of a wallet by its ID.
    pub async fn get_wallet(&self, wallet_id: &WalletId) -> Option<PlatformWallet> {
        let wallets = self.wallets.read().await;
        wallets.get(wallet_id).cloned()
    }

    /// List all wallet IDs.
    pub async fn list_wallets(&self) -> Vec<WalletId> {
        let wallets = self.wallets.read().await;
        wallets.keys().copied().collect()
    }

    /// Subscribe to platform wallet events.
    pub fn subscribe_events(&self) -> broadcast::Receiver<PlatformWalletEvent> {
        self.event_tx.subscribe()
    }

    /// Get the current SPV synced height.
    pub fn synced_height(&self) -> u32 {
        self.spv.synced_height()
    }

    /// Start SPV sync with the given configuration.
    pub async fn start_spv(&self, config: ClientConfig) -> Result<(), PlatformWalletError> {
        let wallet = {
            let wallets = self.wallets.read().await;
            wallets
                .values()
                .next()
                .cloned()
                .ok_or(PlatformWalletError::NoWalletsConfigured)?
        };
        self.spv.start(config, wallet, self.event_tx.clone()).await
    }

    /// Stop SPV sync.
    pub async fn stop_spv(&self) -> Result<(), PlatformWalletError> {
        self.spv.stop().await
    }

    /// Register a transaction to wait for finality proof.
    /// Call BEFORE broadcasting to prevent race where proof arrives first.
    pub async fn register_for_finality(&self, txid: Txid) {
        self.spv.register_for_finality(txid).await;
    }

    /// Wait for a finality proof (InstantLock or ChainLock) for a registered
    /// transaction.
    pub async fn wait_for_finality(
        &self,
        txid: &Txid,
        timeout: Duration,
    ) -> Result<dpp::prelude::AssetLockProof, PlatformWalletError> {
        self.spv.wait_for_finality(txid, timeout, &self.event_tx).await
    }

    /// Insert a wallet into the manager and return a clone.
    async fn insert_and_return(
        &self,
        wallet: PlatformWallet,
    ) -> Result<PlatformWallet, PlatformWalletError> {
        let wallet_id = wallet.wallet_id();
        let mut wallets = self.wallets.write().await;
        if wallets.contains_key(&wallet_id) {
            return Err(PlatformWalletError::WalletAlreadyExists(hex::encode(
                wallet_id,
            )));
        }
        let cloned = wallet.clone();
        wallets.insert(wallet_id, wallet);
        Ok(cloned)
    }
}

impl std::fmt::Debug for PlatformWalletManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlatformWalletManager")
            .field("network", &self.network)
            .field("spv", &self.spv)
            .finish()
    }
}
