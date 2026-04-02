//! Multi-wallet manager with SPV coordination.

use std::collections::BTreeMap;
use std::sync::Arc;

use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::Mnemonic;
use tokio::sync::{broadcast, RwLock};

use crate::error::PlatformWalletError;
use crate::events::PlatformWalletEvent;
use crate::spv::SpvRuntime;
use crate::wallet::platform_wallet::WalletId;
use crate::wallet::PlatformWallet;

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
pub struct PlatformWalletManager {
    sdk: dash_sdk::Sdk,
    wallets: Arc<RwLock<BTreeMap<WalletId, PlatformWallet>>>,
    event_tx: broadcast::Sender<PlatformWalletEvent>,
    spv: SpvRuntime,
}

impl PlatformWalletManager {
    /// Create a new PlatformWalletManager.
    pub fn new(sdk: dash_sdk::Sdk) -> Self {
        let (event_tx, _) = broadcast::channel(256);
        let wallets = Arc::new(RwLock::new(BTreeMap::new()));
        let spv = SpvRuntime::new(Arc::clone(&wallets), event_tx.clone());
        Self {
            sdk,
            wallets,
            event_tx,
            spv,
        }
    }

    /// The network this manager operates on.
    pub fn network(&self) -> key_wallet::Network {
        self.sdk.network
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
            self.sdk.network,
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
        let (wallet, mnemonic) =
            PlatformWallet::random(self.sdk.clone(), self.sdk.network, options)?;
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
        let wallet = PlatformWallet::from_xpub(self.sdk.clone(), self.sdk.network, xpub)?;
        self.insert_and_return(wallet).await
    }

    /// Remove a wallet from the manager.
    pub async fn remove_wallet(
        &self,
        wallet_id: &WalletId,
    ) -> Result<PlatformWallet, PlatformWalletError> {
        let mut wallets = self.wallets.write().await;
        let removed = wallets
            .remove(wallet_id)
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(wallet_id)))?;
        self.spv.notify_wallets_changed();
        Ok(removed)
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

    /// Access the SPV runtime for sync control and finality tracking.
    pub fn spv(&self) -> &SpvRuntime {
        &self.spv
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
        self.spv.notify_wallets_changed();
        Ok(cloned)
    }
}
