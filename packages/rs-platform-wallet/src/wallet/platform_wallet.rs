//! The main PlatformWallet struct combining core, identity, dashpay, and platform sub-wallets.

use std::collections::{BTreeMap, BTreeSet};
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use dashcore::OutPoint;
use dpp::address_funds::PlatformAddress;
use dpp::balances::credits::TokenAmount;
use dpp::fee::Credits;
use dpp::prelude::Identifier;
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;
use key_wallet_manager::WalletManager;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use super::asset_lock::manager::AssetLockManager;
use super::asset_lock::tracked::TrackedAssetLock;
use super::core::{CoreWallet, WalletBalance};
use super::dashpay::DashPayWallet;
use super::identity::{IdentityManager, IdentityWallet};
use super::persister::WalletPersister;
use super::platform_addresses::PlatformAddressWallet;
use super::tokens::TokenWallet;
use crate::changeset::{PlatformWalletChangeSet, PlatformWalletPersistence};

/// Unique identifier for a wallet (32-byte hash).
pub type WalletId = [u8; 32];

/// Consolidated mutable state for a platform wallet.
///
/// Lives inside `WalletManager<PlatformWalletInfo>.wallet_infos`. The `Wallet`
/// key material is in `WalletManager.wallets` — NOT inside this struct.
///
/// `WalletBalance` is stored as `Arc<WalletBalance>` for lock-free UI reads.
pub struct PlatformWalletInfo {
    /// Core wallet metadata, accounts, UTXOs, balances.
    /// Delegates `WalletInfoInterface` methods.
    pub core_wallet: ManagedWalletInfo,
    /// Lock-free balance for UI reads. Updated from `ManagedWalletInfo` after
    /// each SPV block/mempool processing and RPC refresh.
    pub balance: Arc<WalletBalance>,
    pub identity_manager: IdentityManager,
    pub tracked_asset_locks: BTreeMap<OutPoint, TrackedAssetLock>,
    pub platform_address_balances: BTreeMap<PlatformAddress, Credits>,
    pub token_watched: BTreeMap<Identifier, BTreeSet<Identifier>>,
    pub token_balances: BTreeMap<(Identifier, Identifier), TokenAmount>,
}

/// A platform wallet that combines core UTXO functionality with identity management.
///
/// This is SPV-free. It needs only key material and an `Sdk`.
/// For SPV support, use [`PlatformWalletManager`](crate::manager::PlatformWalletManager).
///
/// # Cloning
///
/// `PlatformWallet` is cheaply cloneable (a few atomic increments). A clone is a
/// **shared handle** to the same mutable state — not an independent copy. All
/// clones see the same UTXOs, balances, and identities through the shared
/// `WalletManager` lock.
pub struct PlatformWallet {
    wallet_id: WalletId,
    pub(crate) sdk: Arc<dash_sdk::Sdk>,
    pub(crate) wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    pub(crate) core: CoreWallet,
    pub(crate) identity: IdentityWallet,
    pub(crate) dashpay: DashPayWallet,
    pub(crate) platform: PlatformAddressWallet,
    pub(crate) tokens: TokenWallet,
    /// Shared asset lock manager.
    pub(crate) asset_locks: Arc<AssetLockManager>,
    /// Per-wallet persistence handle.
    persister: WalletPersister,
    /// Lock-free balance for UI reads, cloned from `PlatformWalletInfo.balance`.
    pub(crate) balance: Arc<WalletBalance>,
}

impl PlatformWallet {
    /// Access the core wallet (balance, UTXOs, addresses).
    pub fn core(&self) -> &CoreWallet {
        &self.core
    }

    /// Access the identity wallet.
    pub fn identity(&self) -> &IdentityWallet {
        &self.identity
    }

    /// Access the DashPay wallet.
    pub fn dashpay(&self) -> &DashPayWallet {
        &self.dashpay
    }

    /// Access the platform address wallet.
    pub fn platform(&self) -> &PlatformAddressWallet {
        &self.platform
    }

    /// Access the token wallet.
    pub fn tokens(&self) -> &TokenWallet {
        &self.tokens
    }

    /// Access the shared asset lock manager.
    pub fn asset_locks(&self) -> &AssetLockManager {
        &self.asset_locks
    }

    /// Get the wallet ID.
    pub fn wallet_id(&self) -> WalletId {
        self.wallet_id
    }

    /// Get a reference to the SDK.
    pub fn sdk(&self) -> &dash_sdk::Sdk {
        &self.sdk
    }

    /// Get a reference to the shared wallet manager lock.
    pub fn wallet_manager(&self) -> &Arc<RwLock<WalletManager<PlatformWalletInfo>>> {
        &self.wallet_manager
    }

    /// Get the lock-free balance for UI reads.
    pub fn balance(&self) -> &Arc<WalletBalance> {
        &self.balance
    }

    /// Read-lock the wallet manager and return a guard that derefs to this
    /// wallet's `PlatformWalletInfo`.
    pub async fn state(&self) -> WalletStateReadGuard<'_> {
        WalletStateReadGuard {
            guard: self.wallet_manager.read().await,
            wallet_id: self.wallet_id,
        }
    }

    /// Write-lock the wallet manager and return a guard that derefs to this
    /// wallet's `PlatformWalletInfo` (with `DerefMut`).
    pub async fn state_mut(&self) -> WalletStateWriteGuard<'_> {
        WalletStateWriteGuard {
            guard: self.wallet_manager.write().await,
            wallet_id: self.wallet_id,
        }
    }

    /// Blocking read-lock.
    pub fn state_blocking(&self) -> WalletStateReadGuard<'_> {
        WalletStateReadGuard {
            guard: self.wallet_manager.blocking_read(),
            wallet_id: self.wallet_id,
        }
    }

    /// Blocking write-lock.
    ///
    /// Uses `tokio::sync::RwLock::blocking_write` — must NOT be
    /// called from within a tokio async context. Exists so sync
    /// callers (e.g. SPV-driven transaction processing) can reach
    /// mutation methods that require the wallet-manager write lock.
    pub fn state_mut_blocking(&self) -> WalletStateWriteGuard<'_> {
        WalletStateWriteGuard {
            guard: self.wallet_manager.blocking_write(),
            wallet_id: self.wallet_id,
        }
    }

    /// Construct a PlatformWallet from a WalletManager that already contains
    /// the wallet. The wallet must have been inserted into the WalletManager
    /// before calling this.
    pub(crate) fn new(
        sdk: Arc<dash_sdk::Sdk>,
        wallet_id: WalletId,
        wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
        balance: Arc<WalletBalance>,
        lock_notify: Arc<tokio::sync::Notify>,
        persister: Arc<dyn PlatformWalletPersistence>,
        broadcaster: Arc<dyn crate::broadcaster::TransactionBroadcaster>,
    ) -> Self {
        let core = CoreWallet::new(
            Arc::clone(&sdk),
            Arc::clone(&wallet_manager),
            wallet_id,
            Arc::clone(&broadcaster),
            Arc::clone(&balance),
        );

        // TODO: Create only one in manager.
        let asset_locks = Arc::new(AssetLockManager::new(
            Arc::clone(&sdk),
            Arc::clone(&wallet_manager),
            wallet_id,
            lock_notify,
            broadcaster,
        ));

        let identity = IdentityWallet {
            sdk: Arc::clone(&sdk),
            wallet_manager: Arc::clone(&wallet_manager),
            wallet_id,
            asset_locks: Arc::clone(&asset_locks),
        };

        let dashpay = DashPayWallet {
            sdk: Arc::clone(&sdk),
            wallet_manager: Arc::clone(&wallet_manager),
            wallet_id,
        };

        let platform =
            PlatformAddressWallet::new(Arc::clone(&sdk), Arc::clone(&wallet_manager), wallet_id);
        let tokens = TokenWallet::new(Arc::clone(&sdk), Arc::clone(&wallet_manager), wallet_id);

        Self {
            wallet_id,
            sdk,
            wallet_manager,
            core,
            identity,
            dashpay,
            platform,
            tokens,
            asset_locks,
            persister: WalletPersister::new(wallet_id, persister),
            balance,
        }
    }
}

impl PlatformWallet {
    // TODO: What these methods for? can we remove? Don't deelete this todo
    /// Queue a changeset for later persistence.
    pub fn queue_persist(&self, changeset: PlatformWalletChangeSet) {
        self.persister.store(changeset);
    }

    /// Flush all queued changesets to the storage backend.
    pub fn flush_persist(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.persister.flush()
    }

    /// Load persisted state for this wallet.
    pub fn load_persisted(
        &self,
    ) -> Result<PlatformWalletChangeSet, Box<dyn std::error::Error + Send + Sync>> {
        self.persister.load()
    }

    /// Apply a [`PlatformWalletChangeSet`] to this wallet's in-memory
    /// state under the wallet manager write lock.
    ///
    /// Delegates to [`PlatformWalletInfo::apply_changeset`], which is
    /// the canonical restore path. Holds the `WalletManager` write
    /// lock for the duration so the split borrow of `(&mut Wallet,
    /// &mut PlatformWalletInfo)` is safe — `&mut Wallet` is needed so
    /// the core sub-changeset can re-derive HD accounts via
    /// `Wallet::add_account`.
    ///
    /// Returns [`ApplyError::WalletNotFound`](crate::wallet::ApplyError::WalletNotFound)
    /// if the wallet has been removed from the manager between handle
    /// acquisition and this call.
    ///
    /// Consumes the changeset by value — `apply_changeset` drains
    /// every map straight into the wallet maps with no clones.
    pub async fn apply(
        &self,
        changeset: PlatformWalletChangeSet,
    ) -> Result<(), crate::wallet::ApplyError> {
        let mut wm = self.wallet_manager.write().await;
        let (wallet, info) = wm
            .get_wallet_mut_and_info_mut(&self.wallet_id)
            .ok_or(crate::wallet::ApplyError::WalletNotFound(self.wallet_id))?;
        info.apply_changeset(wallet, changeset)
    }
}

impl Clone for PlatformWallet {
    fn clone(&self) -> Self {
        Self {
            wallet_id: self.wallet_id,
            sdk: self.sdk.clone(),
            wallet_manager: self.wallet_manager.clone(),
            core: self.core.clone(),
            identity: self.identity.clone(),
            dashpay: self.dashpay.clone(),
            platform: self.platform.clone(),
            tokens: self.tokens.clone(),
            asset_locks: self.asset_locks.clone(),
            persister: self.persister.clone(),
            balance: self.balance.clone(),
        }
    }
}

impl std::fmt::Debug for PlatformWallet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlatformWallet")
            .field("wallet_id", &hex::encode(self.wallet_id))
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Wallet state guard types — lock WalletManager, deref to PlatformWalletInfo
// ---------------------------------------------------------------------------

/// Read guard that locks `WalletManager` and derefs to this wallet's
/// `PlatformWalletInfo`. Also provides `.wallet()` for key material access.
pub struct WalletStateReadGuard<'a> {
    guard: RwLockReadGuard<'a, WalletManager<PlatformWalletInfo>>,
    wallet_id: WalletId,
}

impl<'a> WalletStateReadGuard<'a> {
    /// Access the immutable `Wallet` (key material).
    pub fn wallet(&self) -> &Wallet {
        self.guard
            .get_wallet(&self.wallet_id)
            .expect("wallet exists")
    }
}

impl Deref for WalletStateReadGuard<'_> {
    type Target = PlatformWalletInfo;
    fn deref(&self) -> &PlatformWalletInfo {
        self.guard
            .get_wallet_info(&self.wallet_id)
            .expect("wallet exists")
    }
}

/// Write guard that locks `WalletManager` and derefs to this wallet's
/// `PlatformWalletInfo` (with `DerefMut`). Also provides `.wallet()`.
pub struct WalletStateWriteGuard<'a> {
    guard: RwLockWriteGuard<'a, WalletManager<PlatformWalletInfo>>,
    wallet_id: WalletId,
}

impl<'a> WalletStateWriteGuard<'a> {
    /// Access the immutable `Wallet` (key material).
    pub fn wallet(&self) -> &Wallet {
        self.guard
            .get_wallet(&self.wallet_id)
            .expect("wallet exists")
    }
}

impl Deref for WalletStateWriteGuard<'_> {
    type Target = PlatformWalletInfo;
    fn deref(&self) -> &PlatformWalletInfo {
        self.guard
            .get_wallet_info(&self.wallet_id)
            .expect("wallet exists")
    }
}

impl DerefMut for WalletStateWriteGuard<'_> {
    fn deref_mut(&mut self) -> &mut PlatformWalletInfo {
        self.guard
            .get_wallet_info_mut(&self.wallet_id)
            .expect("wallet exists")
    }
}
