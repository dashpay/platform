//! Core wallet functionality: balance, UTXOs, addresses, transaction history.

use std::sync::Arc;

use super::balance::WalletBalance;
use super::reservations::OutpointReservations;

use dashcore::Address as DashAddress;
use tokio::sync::RwLock;

use key_wallet::managed_account::address_pool::AddressPool;
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
use key_wallet::wallet::Wallet;
use key_wallet::KeySource;
use key_wallet_manager::WalletManager;

use crate::broadcaster::TransactionBroadcaster;
use crate::error::PlatformWalletError;
use crate::wallet::platform_addresses::address_reserve::{self, PoolKind};
use crate::wallet::platform_wallet::{PlatformWalletInfo, WalletId};

/// Which pool of a standard BIP-44 account to hand an address from.
#[derive(Clone, Copy)]
enum Bip44Pool {
    /// External (receive) pool — `address_pools_mut()[0]`.
    External,
    /// Internal (change) pool — `address_pools_mut()[1]`.
    Internal,
}

impl Bip44Pool {
    /// Position of this pool in `ManagedAccountType::address_pools_mut`,
    /// which returns `[external, internal]` for standard accounts.
    fn pool_position(self) -> usize {
        match self {
            Bip44Pool::External => 0,
            Bip44Pool::Internal => 1,
        }
    }

    fn reserve_pool_kind(self) -> PoolKind {
        match self {
            Bip44Pool::External => PoolKind::CoreReceive,
            Bip44Pool::Internal => PoolKind::CoreChange,
        }
    }
}

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
    /// Outpoints currently reserved by an in-flight `send_to_addresses`
    /// call on this handle. Closes the same-UTXO concurrent-selection
    /// race — see [`super::reservations`].
    pub(crate) reservations: OutpointReservations,
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
            reservations: OutpointReservations::new(),
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

    /// Pick and atomically reserve the next unused address from a standard
    /// BIP-44 account's external or internal pool.
    ///
    /// BRIDGE: tri-state Unused → Reserved → Used. The upstream pool only
    /// flips `used` on a positive-balance sync, so plain `next_unused`
    /// hands the same index to two concurrent callers. This routes the
    /// pick through [`address_reserve::next_unused_and_reserve`], which
    /// skips reserved-but-unused indices and reserves the chosen one
    /// atomically under the wallet write lock the caller already holds.
    /// Reservations are ephemeral and released by the TTL sweep; once the
    /// address is actually paid, the pool's own `used` flag keeps it out
    /// of future hand-out, making the lingering reservation harmless.
    /// Collapses to `pool.next_unused_and_reserve(..)` once key-wallet
    /// gains a native Reserved state — see rust-dashcore#791.
    ///
    /// The change pool is shared with the broadcast loop, which peeks
    /// change addresses into `OutpointReservations.pending_change` before a
    /// send confirms. A standalone change hand-out must skip those too, so
    /// the internal pool also consults `pending_change` and retries; the
    /// receive pool has no such cross-path sharing and never retries.
    fn reserve_bip44_address(
        &self,
        wallet: &Wallet,
        info: &mut PlatformWalletInfo,
        account_index: u32,
        pool: Bip44Pool,
    ) -> Result<DashAddress, PlatformWalletError> {
        let avoid = match pool {
            Bip44Pool::Internal => self.reservations.pending_change_snapshot(),
            Bip44Pool::External => std::collections::HashSet::new(),
        };

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

        let address_pool: &mut AddressPool = account
            .managed_account_type_mut()
            .address_pools_mut()
            .into_iter()
            .nth(pool.pool_position())
            .ok_or_else(|| {
                PlatformWalletError::AddressOperation(format!(
                    "BIP-44 account {} has no pool at position {}",
                    account_index,
                    pool.pool_position()
                ))
            })?;

        // Each rejected index stays reserved in the bridge's CoreChange
        // set, so the next pick advances past it — the loop converges to
        // the first index that is neither bridge-reserved nor pending from
        // an in-flight send. `avoid` is empty for the receive pool, so that
        // path returns on the first iteration.
        loop {
            let address = address_reserve::next_unused_and_reserve(
                address_pool,
                self.wallet_id,
                pool.reserve_pool_kind(),
                account_index,
                &KeySource::Public(xpub),
                true,
            )
            .map_err(|e| PlatformWalletError::AddressOperation(e.to_string()))?;

            if !avoid.contains(&address) {
                return Ok(address);
            }
        }
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
        self.reserve_bip44_address(wallet, info, account_index, Bip44Pool::External)
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
        self.reserve_bip44_address(wallet, info, account_index, Bip44Pool::External)
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
        self.reserve_bip44_address(wallet, info, account_index, Bip44Pool::Internal)
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
        self.reserve_bip44_address(wallet, info, account_index, Bip44Pool::Internal)
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
            reservations: self.reservations.clone(),
        }
    }
}
