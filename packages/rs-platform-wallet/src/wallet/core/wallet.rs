//! Core wallet functionality: balance, UTXOs, addresses, transaction history.

use std::sync::Arc;

use super::balance::WalletBalance;

use dashcore::{Address as DashAddress, Transaction};
use tokio::sync::RwLock;

use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use key_wallet_manager::WalletManager;

use crate::error::PlatformWalletError;
use crate::wallet::platform_wallet::{PlatformWalletInfo, WalletId};

/// Core wallet providing UTXO, balance, and address functionality.
#[derive(Clone)]
pub struct CoreWallet {
    // TODO: Are we using SDK here?
    pub(crate) sdk: Arc<dash_sdk::Sdk>,
    /// Shared wallet manager holding all wallets' key material and info.
    pub(crate) wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    /// Identifies which wallet in the manager this sub-wallet operates on.
    pub(crate) wallet_id: WalletId,
    // TODO: Rename to cache
    /// Lock-free balance — updated from `ManagedWalletInfo` on every
    /// SPV block/mempool processing and RPC refresh. Read without any lock.
    pub(crate) balance: Arc<WalletBalance>,
    /// Injected broadcaster — delegates to SPV or DAPI depending on how
    /// the wallet was constructed by `PlatformWalletManager`.
    broadcaster: Arc<dyn crate::broadcaster::TransactionBroadcaster>,
}

impl CoreWallet {
    /// Create a new CoreWallet.
    pub(crate) fn new(
        sdk: Arc<dash_sdk::Sdk>,
        wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
        wallet_id: WalletId,
        broadcaster: Arc<dyn crate::broadcaster::TransactionBroadcaster>,
        balance: Arc<WalletBalance>,
    ) -> Self {
        Self {
            sdk,
            wallet_manager,
            wallet_id,
            balance,
            broadcaster,
        }
    }

    /// Lock-free balance — read from any context without locking.
    /// Updated automatically after SPV/RPC balance changes.
    pub fn balance(&self) -> &WalletBalance {
        &self.balance
    }
    // TODO: We need to accept account index everywhere here. for what are we using these methods?
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
        let mut wm = self.wallet_manager.write().await;
        let (wallet, info) = wm
            .get_wallet_and_info_mut(&self.wallet_id)
            .expect("wallet exists");
        let xpub = Self::derive_account_xpub(wallet, account_index)?;
        let account = info
            .core_wallet
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
            .next_receive_address(Some(&xpub))
            .map_err(|e| crate::error::PlatformWalletError::WalletCreation(e.to_string()))
    }

    /// Blocking version of `next_receive_address` for sync contexts.
    pub fn next_receive_address_blocking(
        &self,
    ) -> Result<DashAddress, crate::error::PlatformWalletError> {
        let account_index = 0u32;
        let mut wm = self.wallet_manager.blocking_write();
        let (wallet, info) = wm
            .get_wallet_and_info_mut(&self.wallet_id)
            .expect("wallet exists");
        let xpub = Self::derive_account_xpub(wallet, account_index)?;
        let account = info
            .core_wallet
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
            .next_receive_address(Some(&xpub))
            .map_err(|e| crate::error::PlatformWalletError::WalletCreation(e.to_string()))
    }

    /// Get the next unused change address for the default account.
    pub(crate) async fn next_change_address(
        &self,
    ) -> Result<DashAddress, crate::error::PlatformWalletError> {
        self.next_change_address_for_account(0).await
    }

    /// Blocking version of `next_change_address` for sync contexts.
    pub fn next_change_address_blocking(
        &self,
    ) -> Result<DashAddress, crate::error::PlatformWalletError> {
        let account_index = 0u32;
        let mut wm = self.wallet_manager.blocking_write();
        let (wallet, info) = wm
            .get_wallet_and_info_mut(&self.wallet_id)
            .expect("wallet exists");
        let xpub = Self::derive_account_xpub(wallet, account_index)?;
        let account = info
            .core_wallet
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
            .next_change_address(Some(&xpub))
            .map_err(|e| crate::error::PlatformWalletError::WalletCreation(e.to_string()))
    }

    /// Get the next unused BIP-44 internal (change) address for a specific account.
    pub async fn next_change_address_for_account(
        &self,
        account_index: u32,
    ) -> Result<DashAddress, crate::error::PlatformWalletError> {
        let mut wm = self.wallet_manager.write().await;
        let (wallet, info) = wm
            .get_wallet_and_info_mut(&self.wallet_id)
            .expect("wallet exists");
        let xpub = Self::derive_account_xpub(wallet, account_index)?;
        let account = info
            .core_wallet
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
            .next_change_address(Some(&xpub))
            .map_err(|e| crate::error::PlatformWalletError::WalletCreation(e.to_string()))
    }
    // TODO: Why we need this?
    /// Get the network from the SDK.
    pub fn network(&self) -> key_wallet::Network {
        self.sdk.network
    }
    // TODO: Why is it static
    /// Derive the BIP-44 account-level extended public key from the wallet
    /// key material.
    fn derive_account_xpub(
        wallet: &key_wallet::wallet::Wallet,
        account_index: u32,
    ) -> Result<key_wallet::bip32::ExtendedPubKey, crate::error::PlatformWalletError> {
        let path = key_wallet::account::AccountType::Standard {
            index: account_index,
            standard_account_type: key_wallet::account::StandardAccountType::BIP44Account,
        }
        .derivation_path(wallet.network)
        .map_err(|e| {
            crate::error::PlatformWalletError::WalletCreation(format!(
                "Invalid account index: {}",
                e
            ))
        })?;
        wallet.derive_extended_public_key(&path).map_err(|e| {
            crate::error::PlatformWalletError::WalletCreation(format!(
                "Failed to derive account xpub: {}",
                e
            ))
        })
    }
}

// Transaction status is tracked natively in key-wallet's TransactionRecord.context.

// ---------------------------------------------------------------------------
// Transaction broadcasting
// ---------------------------------------------------------------------------

impl CoreWallet {
    /// Broadcast a signed transaction to the network.
    ///
    /// Delegates to the injected [`TransactionBroadcaster`] which may use
    /// SPV (P2P) or DAPI (gRPC) depending on how the wallet was constructed.
    ///
    /// Returns the transaction ID on success.
    pub async fn broadcast_transaction(
        &self,
        transaction: &Transaction,
    ) -> Result<dashcore::Txid, PlatformWalletError> {
        self.broadcaster.broadcast(transaction).await
    }
}

impl std::fmt::Debug for CoreWallet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CoreWallet")
            .field("network", &self.sdk.network)
            .finish()
    }
}
