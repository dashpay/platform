//! Core wallet functionality: balance, UTXOs, addresses, transaction history.

use std::collections::BTreeSet;
use std::sync::Arc;

use dashcore::Address as DashAddress;
use dashcore::Transaction;
use dpp::prelude::CoreBlockHeight;
use key_wallet::account::TransactionRecord;
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;
use key_wallet::{Network, Utxo, WalletCoreBalance};
use tokio::sync::RwLock;

/// Core wallet providing UTXO, balance, and address functionality.
#[derive(Clone)]
pub struct CoreWallet {
    pub(crate) sdk: dash_sdk::Sdk,
    pub(crate) wallet: Arc<RwLock<Wallet>>,
    pub(crate) wallet_info: Arc<RwLock<ManagedWalletInfo>>,
    pub(crate) network: Network,
}

impl CoreWallet {
    /// Get the wallet balance (spendable, unconfirmed, total).
    pub async fn balance(&self) -> WalletCoreBalance {
        let info = self.wallet_info.read().await;
        info.balance()
    }

    /// Get all UTXOs.
    pub async fn utxos(&self) -> BTreeSet<Utxo> {
        let info = self.wallet_info.read().await;
        info.utxos().into_iter().cloned().collect()
    }

    /// Get spendable UTXOs (confirmed, non-dust, unlocked).
    pub async fn spendable_utxos(&self) -> BTreeSet<Utxo> {
        let info = self.wallet_info.read().await;
        info.get_spendable_utxos().into_iter().cloned().collect()
    }

    /// Get the next unused receive address for the default account.
    pub async fn next_receive_address(
        &self,
    ) -> Result<DashAddress, crate::error::PlatformWalletError> {
        self.next_receive_address_for_account(0).await
    }

    /// Get the next unused BIP-44 external (receive) address for a specific account.
    pub async fn next_receive_address_for_account(
        &self,
        account_index: u32,
    ) -> Result<DashAddress, crate::error::PlatformWalletError> {
        let xpub = self.account_xpub(account_index).await?;
        let mut info = self.wallet_info.write().await;
        let account = info
            .accounts
            .standard_bip44_accounts
            .get_mut(&account_index)
            .ok_or_else(|| {
                crate::error::PlatformWalletError::WalletCreation(format!(
                    "BIP-44 account {} not found",
                    account_index
                ))
            })?;
        account
            .next_receive_address(Some(&xpub), true)
            .map_err(|e| crate::error::PlatformWalletError::WalletCreation(e.to_string()))
    }

    /// Get the next unused change address for the default account.
    pub async fn next_change_address(
        &self,
    ) -> Result<DashAddress, crate::error::PlatformWalletError> {
        self.next_change_address_for_account(0).await
    }

    /// Get the next unused BIP-44 internal (change) address for a specific account.
    pub async fn next_change_address_for_account(
        &self,
        account_index: u32,
    ) -> Result<DashAddress, crate::error::PlatformWalletError> {
        let xpub = self.account_xpub(account_index).await?;
        let mut info = self.wallet_info.write().await;
        let account = info
            .accounts
            .standard_bip44_accounts
            .get_mut(&account_index)
            .ok_or_else(|| {
                crate::error::PlatformWalletError::WalletCreation(format!(
                    "BIP-44 account {} not found",
                    account_index
                ))
            })?;
        account
            .next_change_address(Some(&xpub), true)
            .map_err(|e| crate::error::PlatformWalletError::WalletCreation(e.to_string()))
    }

    /// Get all monitored addresses across all account types.
    pub async fn monitored_addresses(&self) -> Vec<DashAddress> {
        let info = self.wallet_info.read().await;
        info.monitored_addresses()
    }

    /// Get the current synced height.
    pub async fn synced_height(&self) -> CoreBlockHeight {
        let info = self.wallet_info.read().await;
        info.synced_height()
    }

    /// Get the wallet birth height.
    pub async fn birth_height(&self) -> CoreBlockHeight {
        let info = self.wallet_info.read().await;
        info.birth_height()
    }

    /// Get the cached network (sync, no lock needed).
    pub fn network(&self) -> Network {
        self.network
    }

    /// Get the transaction history.
    pub async fn transaction_history(&self) -> Vec<TransactionRecord> {
        let info = self.wallet_info.read().await;
        info.transaction_history().into_iter().cloned().collect()
    }

    /// Get immature transactions (coinbase outputs not yet mature).
    pub async fn immature_transactions(&self) -> Vec<Transaction> {
        let info = self.wallet_info.read().await;
        info.immature_transactions()
    }

    /// Get the extended public key for a specific account index.
    ///
    /// Derives the BIP-44 account-level key at `m/44'/coin_type'/account_index'`.
    pub async fn account_xpub(
        &self,
        account_index: u32,
    ) -> Result<key_wallet::bip32::ExtendedPubKey, crate::error::PlatformWalletError> {
        use key_wallet::bip32::{ChildNumber, DerivationPath};

        let coin_type = if self.network == Network::Mainnet {
            5u32 // DASH mainnet
        } else {
            1u32 // testnet/devnet/regtest all use coin_type 1
        };

        let path = DerivationPath::from(vec![
            ChildNumber::from_hardened_idx(44).expect("valid"),
            ChildNumber::from_hardened_idx(coin_type).expect("valid"),
            ChildNumber::from_hardened_idx(account_index).map_err(|e| {
                crate::error::PlatformWalletError::WalletCreation(format!(
                    "Invalid account index: {}",
                    e
                ))
            })?,
        ]);

        let wallet = self.wallet.read().await;
        wallet.derive_extended_public_key(&path).map_err(|e| {
            crate::error::PlatformWalletError::WalletCreation(format!(
                "Failed to derive account xpub: {}",
                e
            ))
        })
    }
}

impl std::fmt::Debug for CoreWallet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CoreWallet")
            .field("network", &self.network)
            .finish()
    }
}
