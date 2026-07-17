//! Core wallet functionality: balance, UTXOs, addresses, transaction history.

use std::sync::Arc;

use super::balance::WalletBalance;

use dashcore::Address as DashAddress;
use tokio::sync::RwLock;

use key_wallet::managed_account::address_pool::KeySource;
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
use key_wallet::wallet::managed_wallet_info::transaction_building::AccountTypePreference;
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
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

    /// Wallet id this `CoreWallet` operates on. Exposed so FFI
    /// callers that need to construct a per-call `Signer` (e.g.
    /// `MnemonicResolverCoreSigner`) can thread the same wallet id
    /// the resolver callback will receive.
    pub fn wallet_id(&self) -> WalletId {
        self.wallet_id
    }

    pub async fn set_gap_limit(
        &self,
        account_type: AccountTypePreference,
        account_index: u32,
        gap_limit: u32,
    ) -> Result<(), PlatformWalletError> {
        let mut wm = self.wallet_manager.write().await;
        let (wallet, info) = wm.get_wallet_and_info_mut(&self.wallet_id).ok_or_else(|| {
            PlatformWalletError::WalletNotFound("Wallet not found in wallet manager".to_string())
        })?;

        let xpub = match account_type {
            AccountTypePreference::BIP44 => wallet.get_bip44_account(account_index),
            AccountTypePreference::BIP32 => wallet.get_bip32_account(account_index),
            AccountTypePreference::CoinJoin => wallet.get_coinjoin_account(account_index),
        }
        .map(|a| a.account_xpub)
        .ok_or_else(|| {
            PlatformWalletError::WalletNotFound(format!(
                "wallet account {account_type:?} #{account_index} not found"
            ))
        })?;

        let account = match account_type {
            AccountTypePreference::BIP44 => info
                .core_wallet
                .accounts
                .standard_bip44_accounts
                .get_mut(&account_index),
            AccountTypePreference::BIP32 => info
                .core_wallet
                .accounts
                .standard_bip32_accounts
                .get_mut(&account_index),
            AccountTypePreference::CoinJoin => info
                .core_wallet
                .accounts
                .coinjoin_accounts
                .get_mut(&account_index),
        }
        .ok_or_else(|| {
            PlatformWalletError::WalletNotFound(format!(
                "managed account {account_type:?} #{account_index} not found"
            ))
        })?;

        account
            .set_gap_limit(gap_limit, &KeySource::Public(xpub))
            .map(|_| ())
            .map_err(|e| PlatformWalletError::AddressOperation(e.to_string()))
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

    /// Current last-processed block height for this wallet, or `None` if the
    /// wallet is no longer present in the manager.
    ///
    /// This is the clock the funding reservation is actually stamped with:
    /// `finalize_transaction` / `build_signed` reserve the selected inputs at
    /// `set_current_height(last_processed_height())`, and key-wallet's
    /// `ReservationSet` TTL sweeps entries relative to a later build's
    /// `last_processed_height`. It is therefore the correct — and monotonic —
    /// clock for the deferred-payment
    /// [`SignedPaymentRegistry`](crate::SignedPaymentRegistry) to bound a token's
    /// lifetime against that TTL. `synced_height` is a different clock that can
    /// regress during a rescan, so measuring the reservation's age against it
    /// could let a token outlive its reservation.
    pub(crate) async fn last_processed_height(&self) -> Option<u32> {
        let wm = self.wallet_manager.read().await;
        wm.get_wallet_and_info(&self.wallet_id)
            .map(|(_, info)| info.core_wallet.last_processed_height())
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
