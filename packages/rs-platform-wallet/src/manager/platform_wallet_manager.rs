//! Multi-wallet manager with SPV coordination.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::{Mnemonic, Network};
use tokio::sync::{broadcast, RwLock};

use crate::error::PlatformWalletError;
use crate::events::PlatformWalletEvent;
use crate::wallet::platform_wallet::WalletId;
use crate::wallet::PlatformWallet;

/// Manages multiple platform wallets and coordinates SPV sync.
pub struct PlatformWalletManager {
    sdk: dash_sdk::Sdk,
    network: Network,
    wallets: RwLock<BTreeMap<WalletId, PlatformWallet>>,
    event_tx: broadcast::Sender<PlatformWalletEvent>,
    synced_height: AtomicU32,
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
            synced_height: AtomicU32::new(0),
        }
    }

    /// Create a wallet from a BIP-39 mnemonic and add it to the manager.
    pub async fn create_wallet_from_mnemonic(
        &self,
        mnemonic: &str,
        passphrase: &str,
        options: WalletAccountCreationOptions,
    ) -> Result<PlatformWallet, PlatformWalletError> {
        let wallet =
            PlatformWallet::from_mnemonic(self.sdk.clone(), self.network, mnemonic, passphrase, options)?;
        self.insert_and_return(wallet).await
    }

    /// Create a wallet with a randomly generated mnemonic.
    /// Returns the wallet and the generated mnemonic.
    pub async fn create_wallet_with_random_mnemonic(
        &self,
        options: WalletAccountCreationOptions,
    ) -> Result<(PlatformWallet, Mnemonic), PlatformWalletError> {
        let (wallet, mnemonic) =
            PlatformWallet::random(self.sdk.clone(), self.network, options)?;
        let wallet = self.insert_and_return(wallet).await?;
        Ok((wallet, mnemonic))
    }

    /// Import a wallet from an extended private key string.
    pub async fn import_wallet_from_extended_key(
        &self,
        xprv: &str,
        options: WalletAccountCreationOptions,
    ) -> Result<PlatformWallet, PlatformWalletError> {
        let wallet =
            PlatformWallet::from_extended_key(self.sdk.clone(), self.network, xprv, options)?;
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

    /// Get the current synced height across all wallets.
    pub fn synced_height(&self) -> u32 {
        self.synced_height.load(Ordering::Relaxed)
    }

    /// Start SPV sync (stub — to be implemented with dash-spv integration).
    pub async fn start_spv(&self) -> Result<(), PlatformWalletError> {
        // TODO: Integrate with dash-spv DashSpvClient
        Ok(())
    }

    /// Stop SPV sync (stub — to be implemented with dash-spv integration).
    pub async fn stop_spv(&self) -> Result<(), PlatformWalletError> {
        // TODO: Integrate with dash-spv DashSpvClient
        Ok(())
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
            .field("synced_height", &self.synced_height.load(Ordering::Relaxed))
            .finish()
    }
}
