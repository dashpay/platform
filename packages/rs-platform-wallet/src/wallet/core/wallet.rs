//! Core wallet functionality: balance, UTXOs, addresses, transaction history.

use std::sync::Arc;

use super::balance::WalletBalance;

use dashcore::Address as DashAddress;
use tokio::sync::RwLock;

use key_wallet_manager::WalletManager;

use crate::broadcaster::TransactionBroadcaster;
use crate::error::PlatformWalletError;
use crate::wallet::platform_wallet::{PlatformWalletInfo, WalletId};

/// Core wallet providing UTXO, balance, and address functionality.
///
/// This is a lightweight handle — all mutable state lives in the shared
/// `WalletManager<PlatformWalletInfo>` behind an `Arc<RwLock<…>>`.
/// The handle holds `Arc` references and is cheaply `Clone`able.
///
/// `B` is the concrete transaction-broadcaster type. The generic
/// parameter lets broadcast calls dispatch statically instead of
/// through a `dyn` vtable.
pub struct CoreWallet<B: TransactionBroadcaster + ?Sized> {
    pub(crate) sdk: Arc<dash_sdk::Sdk>,
    pub(crate) wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    pub(crate) wallet_id: WalletId,
    /// Injected broadcaster — delegates to SPV or DAPI depending on how
    /// the wallet was constructed by `PlatformWalletManager`.
    pub(crate) broadcaster: Arc<B>,
    /// Lock-free balance for UI reads.
    balance: Arc<WalletBalance>,
}

impl<B: TransactionBroadcaster + ?Sized> CoreWallet<B> {
    pub(crate) fn new(
        sdk: Arc<dash_sdk::Sdk>,
        wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
        wallet_id: WalletId,
        broadcaster: Arc<B>,
        balance: Arc<WalletBalance>,
    ) -> Self {
        Self {
            sdk,
            wallet_manager,
            wallet_id,
            broadcaster,
            balance,
        }
    }

    /// Lock-free balance snapshot for UI reads.
    pub fn balance(&self) -> &WalletBalance {
        &self.balance
    }

    /// Get the next unused BIP-44 external (receive) address for a specific account.
    pub async fn next_receive_address_for_account(
        &self,
        account_index: u32,
    ) -> Result<DashAddress, PlatformWalletError> {
        let mut wm = self.wallet_manager.write().await;
        let (wallet, info) = wm.get_wallet_and_info_mut(&self.wallet_id).ok_or_else(|| {
            crate::error::PlatformWalletError::WalletNotFound(
                "Wallet not found in wallet manager".to_string(),
            )
        })?;

        let xpub = wallet
            .accounts
            .standard_bip44_accounts
            .get(&account_index)
            .map(|a| a.account_xpub)
            .ok_or_else(|| {
                PlatformWalletError::WalletNotFound(format!(
                    "BIP-44 account {} not found",
                    account_index
                ))
            })?;

        let account = info
            .core_wallet
            .accounts
            .standard_bip44_accounts
            .get_mut(&account_index)
            .ok_or_else(|| {
                PlatformWalletError::WalletNotFound(format!(
                    "BIP-44 managed account {} not found",
                    account_index
                ))
            })?;

        account
            .next_receive_address(Some(&xpub), true)
            .map_err(|e| PlatformWalletError::AddressOperation(e.to_string()))
    }

    /// Blocking version of `next_receive_address_for_account`.
    pub fn next_receive_address_for_account_blocking(
        &self,
        account_index: u32,
    ) -> Result<DashAddress, PlatformWalletError> {
        let mut wm = self.wallet_manager.blocking_write();
        let (wallet, info) = wm.get_wallet_and_info_mut(&self.wallet_id).ok_or_else(|| {
            crate::error::PlatformWalletError::WalletNotFound(
                "Wallet not found in wallet manager".to_string(),
            )
        })?;

        let xpub = wallet
            .accounts
            .standard_bip44_accounts
            .get(&account_index)
            .map(|a| a.account_xpub)
            .ok_or_else(|| {
                PlatformWalletError::WalletNotFound(format!(
                    "BIP-44 account {} not found",
                    account_index
                ))
            })?;

        let account = info
            .core_wallet
            .accounts
            .standard_bip44_accounts
            .get_mut(&account_index)
            .ok_or_else(|| {
                PlatformWalletError::WalletNotFound(format!(
                    "BIP-44 managed account {} not found",
                    account_index
                ))
            })?;

        account
            .next_receive_address(Some(&xpub), true)
            .map_err(|e| PlatformWalletError::AddressOperation(e.to_string()))
    }

    /// Get the next unused BIP-44 internal (change) address for a specific account.
    pub async fn next_change_address_for_account(
        &self,
        account_index: u32,
    ) -> Result<DashAddress, PlatformWalletError> {
        let mut wm = self.wallet_manager.write().await;
        let (wallet, info) = wm.get_wallet_and_info_mut(&self.wallet_id).ok_or_else(|| {
            crate::error::PlatformWalletError::WalletNotFound(
                "Wallet not found in wallet manager".to_string(),
            )
        })?;

        let xpub = wallet
            .accounts
            .standard_bip44_accounts
            .get(&account_index)
            .map(|a| a.account_xpub)
            .ok_or_else(|| {
                PlatformWalletError::WalletNotFound(format!(
                    "BIP-44 account {} not found",
                    account_index
                ))
            })?;

        let account = info
            .core_wallet
            .accounts
            .standard_bip44_accounts
            .get_mut(&account_index)
            .ok_or_else(|| {
                PlatformWalletError::WalletNotFound(format!(
                    "BIP-44 managed account {} not found",
                    account_index
                ))
            })?;

        account
            .next_change_address(Some(&xpub), true)
            .map_err(|e| PlatformWalletError::AddressOperation(e.to_string()))
    }

    /// Blocking version of `next_change_address_for_account`.
    pub fn next_change_address_for_account_blocking(
        &self,
        account_index: u32,
    ) -> Result<DashAddress, PlatformWalletError> {
        let mut wm = self.wallet_manager.blocking_write();
        let (wallet, info) = wm.get_wallet_and_info_mut(&self.wallet_id).ok_or_else(|| {
            crate::error::PlatformWalletError::WalletNotFound(
                "Wallet not found in wallet manager".to_string(),
            )
        })?;

        let xpub = wallet
            .accounts
            .standard_bip44_accounts
            .get(&account_index)
            .map(|a| a.account_xpub)
            .ok_or_else(|| {
                PlatformWalletError::WalletNotFound(format!(
                    "BIP-44 account {} not found",
                    account_index
                ))
            })?;

        let account = info
            .core_wallet
            .accounts
            .standard_bip44_accounts
            .get_mut(&account_index)
            .ok_or_else(|| {
                PlatformWalletError::WalletNotFound(format!(
                    "BIP-44 managed account {} not found",
                    account_index
                ))
            })?;

        account
            .next_change_address(Some(&xpub), true)
            .map_err(|e| PlatformWalletError::AddressOperation(e.to_string()))
    }

    /// Get the network from the SDK.
    pub fn network(&self) -> key_wallet::Network {
        self.sdk.network
    }
}

impl<B: TransactionBroadcaster + ?Sized> std::fmt::Debug for CoreWallet<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CoreWallet")
            .field("network", &self.sdk.network)
            .finish()
    }
}

// Manual `Clone` impl: the derive would add a `where B: Clone`
// bound, but `Arc<B>` clones without cloning `B` itself, so we
// don't want that bound. `B: ?Sized` is enough.
impl<B: TransactionBroadcaster + ?Sized> Clone for CoreWallet<B> {
    fn clone(&self) -> Self {
        Self {
            sdk: Arc::clone(&self.sdk),
            wallet_manager: Arc::clone(&self.wallet_manager),
            wallet_id: self.wallet_id,
            broadcaster: Arc::clone(&self.broadcaster),
            balance: Arc::clone(&self.balance),
        }
    }
}
