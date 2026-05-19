//! The main PlatformWallet struct combining core, identity (+DashPay), and platform sub-wallets.

use std::collections::BTreeMap;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use dashcore::OutPoint;
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;
use key_wallet_manager::WalletManager;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use super::asset_lock::manager::AssetLockManager;
use super::asset_lock::tracked::TrackedAssetLock;
use super::core::{CoreWallet, WalletBalance};
use super::identity::{IdentityManager, IdentityWallet};
use super::persister::WalletPersister;
use super::platform_addresses::PlatformAddressWallet;
#[cfg(feature = "shielded")]
use super::shielded::{FileBackedShieldedStore, ShieldedSyncSummary, ShieldedWallet};
use crate::broadcaster::SpvBroadcaster;
use crate::changeset::{
    ClientStartState, PersistenceError, PlatformWalletChangeSet, PlatformWalletPersistence,
};
#[cfg(feature = "shielded")]
use crate::error::PlatformWalletError;
#[cfg(feature = "shielded")]
use std::path::Path;

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
    // Sub-wallets that hold a broadcaster are monomorphized with
    // `SpvBroadcaster` — the only production broadcaster in use.
    // Swapping this out to another broadcaster is a three-line flip
    // right here plus the `new()` signature below; the sub-wallet
    // definitions themselves stay untouched.
    pub(crate) core: CoreWallet<SpvBroadcaster>,
    pub(crate) identity: IdentityWallet<SpvBroadcaster>,
    pub(crate) platform: PlatformAddressWallet,
    /// Shared asset lock manager.
    pub(crate) asset_locks: Arc<AssetLockManager<SpvBroadcaster>>,
    /// Per-wallet persistence handle.
    persister: WalletPersister,
    /// Lock-free balance for UI reads, cloned from `PlatformWalletInfo.balance`.
    pub(crate) balance: Arc<WalletBalance>,
    /// Shielded (Orchard / ZK) sub-wallet. `None` until [`bind_shielded`]
    /// has run; remains `None` for `WatchOnly` / `ExternalSignable`
    /// wallets that have never had a resolver-driven bind. The
    /// `RwLock` lets the shielded sync coordinator read the bound
    /// state without serializing against unrelated wallet writes.
    ///
    /// [`bind_shielded`]: Self::bind_shielded
    #[cfg(feature = "shielded")]
    pub(crate) shielded: Arc<RwLock<Option<ShieldedWallet<FileBackedShieldedStore>>>>,
}

impl PlatformWallet {
    /// Access the core wallet (balance, UTXOs, addresses).
    pub fn core(&self) -> &CoreWallet<SpvBroadcaster> {
        &self.core
    }

    /// Access the identity wallet.
    ///
    /// Covers both identity-lifecycle and DashPay-contract operations —
    /// these used to be split across `identity()` / `dashpay()`, but the
    /// two facades were merged (the underlying `ManagedIdentity` state
    /// was already shared between them). Keeps the single `SpvBroadcaster`
    /// specialization the rest of this wallet uses.
    pub fn identity(&self) -> &IdentityWallet<SpvBroadcaster> {
        &self.identity
    }

    /// Access the platform address wallet.
    pub fn platform(&self) -> &PlatformAddressWallet {
        &self.platform
    }

    /// Access the shared asset lock manager.
    pub fn asset_locks(&self) -> &Arc<AssetLockManager<SpvBroadcaster>> {
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

    /// Clone the underlying `Arc<dash_sdk::Sdk>` so callers (e.g. FFI
    /// async blocks moved onto a worker runtime) can hold an
    /// independently-owned SDK handle without keeping the
    /// `PlatformWallet` borrow alive.
    pub fn sdk_arc(&self) -> Arc<dash_sdk::Sdk> {
        Arc::clone(&self.sdk)
    }

    /// Get a reference to the shared wallet manager lock.
    pub fn wallet_manager(&self) -> &Arc<RwLock<WalletManager<PlatformWalletInfo>>> {
        &self.wallet_manager
    }

    /// Get the lock-free balance for UI reads.
    pub fn balance(&self) -> &Arc<WalletBalance> {
        &self.balance
    }

    /// Get a reference to the per-wallet persistence handle.
    ///
    /// Callers that hold a `&PlatformWallet` and need to invoke mutation
    /// methods on [`ManagedIdentity`] (e.g. `set_dashpay_profile`,
    /// `record_dashpay_payment`, `add_identity`) must pass this persister
    /// so those methods can persist the resulting changeset immediately.
    pub fn persister(&self) -> &WalletPersister {
        &self.persister
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

    /// Non-blocking read-lock. Returns `None` if the lock is currently
    /// held by a writer, or cannot be acquired without parking the
    /// thread. Safe to call from any context — never panics, never
    /// blocks. Intended for sync callers that run on a tokio runtime
    /// thread (e.g. egui UI render callbacks) where blocking variants
    /// would panic and async variants cannot be awaited.
    pub fn try_state(&self) -> Option<WalletStateReadGuard<'_>> {
        self.wallet_manager
            .try_read()
            .ok()
            .map(|guard| WalletStateReadGuard {
                guard,
                wallet_id: self.wallet_id,
            })
    }

    /// Non-blocking write-lock. Returns `None` if the lock is currently
    /// held by any reader or writer. Same safety properties as
    /// [`try_state`]: never panics, never blocks.
    pub fn try_state_mut(&self) -> Option<WalletStateWriteGuard<'_>> {
        self.wallet_manager
            .try_write()
            .ok()
            .map(|guard| WalletStateWriteGuard {
                guard,
                wallet_id: self.wallet_id,
            })
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
        broadcaster: Arc<SpvBroadcaster>,
    ) -> Self {
        // Build the per-wallet persister handle once and share it with
        // the sub-wallets that need to queue their own changesets
        // (currently just `AssetLockManager` — see Item 8 sub-step 1a).
        let wallet_persister = WalletPersister::new(wallet_id, persister);

        let core = CoreWallet::new(
            Arc::clone(&sdk),
            Arc::clone(&wallet_manager),
            wallet_id,
            Arc::clone(&broadcaster),
            Arc::clone(&balance),
        );

        // Asset-lock broadcaster is pinned to `SpvBroadcaster`; the
        // identity wallet's DashPay payment broadcaster uses a clone
        // of the same Arc since production currently runs one
        // broadcaster type across the stack.
        let dashpay_broadcaster = Arc::clone(&broadcaster);

        let asset_locks = Arc::new(AssetLockManager::new(
            Arc::clone(&sdk),
            Arc::clone(&wallet_manager),
            wallet_id,
            lock_notify,
            broadcaster,
            wallet_persister.clone(),
        ));

        let identity: IdentityWallet<SpvBroadcaster> = IdentityWallet {
            sdk: Arc::clone(&sdk),
            wallet_manager: Arc::clone(&wallet_manager),
            wallet_id,
            asset_locks: Arc::clone(&asset_locks),
            persister: wallet_persister.clone(),
            broadcaster: dashpay_broadcaster,
        };

        let platform = PlatformAddressWallet::new(
            Arc::clone(&sdk),
            Arc::clone(&wallet_manager),
            wallet_id,
            Arc::clone(&asset_locks),
            wallet_persister.clone(),
        );

        Self {
            wallet_id,
            sdk,
            wallet_manager,
            core,
            identity,
            platform,
            asset_locks,
            persister: wallet_persister,
            balance,
            #[cfg(feature = "shielded")]
            shielded: Arc::new(RwLock::new(None)),
        }
    }

    /// Bind a shielded (Orchard) sub-wallet to this `PlatformWallet`.
    ///
    /// Derives ZIP-32 Orchard keys from `seed` (a 32-252 byte BIP-39
    /// seed; see [`SpendingKey::from_zip32_seed`]), opens or creates
    /// the per-network commitment tree at `db_path`, and stores the
    /// resulting [`ShieldedWallet`] on this handle. The caller is
    /// responsible for sourcing the seed (e.g. via the host
    /// `MnemonicResolverHandle`) and for zeroizing it once this call
    /// returns. The seed is not retained — only the FVK / IVK / OVK
    /// / default address derived from it survive on the wallet.
    ///
    /// Idempotent: a second call replaces the previously-bound
    /// shielded wallet (e.g. after a network switch).
    ///
    /// [`SpendingKey::from_zip32_seed`]: grovedb_commitment_tree::SpendingKey::from_zip32_seed
    #[cfg(feature = "shielded")]
    pub async fn bind_shielded(
        &self,
        seed: &[u8],
        account: u32,
        db_path: impl AsRef<Path>,
    ) -> Result<(), PlatformWalletError> {
        // Open / create the SQLite-backed commitment tree first so
        // any I/O failure surfaces before we touch the wallet's
        // existing shielded slot.
        let store = FileBackedShieldedStore::open_path(db_path, 100)
            .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))?;
        let network = self.sdk.network;
        let wallet =
            ShieldedWallet::from_seed(Arc::clone(&self.sdk), seed, network, account, store)?;

        let mut slot = self.shielded.write().await;
        *slot = Some(wallet);
        Ok(())
    }

    /// Whether the shielded sub-wallet has been bound via
    /// [`bind_shielded`](Self::bind_shielded).
    #[cfg(feature = "shielded")]
    pub async fn is_shielded_bound(&self) -> bool {
        self.shielded.read().await.is_some()
    }

    /// Run one shielded sync pass on this wallet.
    ///
    /// Returns `Ok(None)` if the shielded sub-wallet hasn't been
    /// bound (the sync coordinator skips unbound wallets without
    /// surfacing an error). Returns `Ok(Some(summary))` after a
    /// successful pass, or `Err(_)` if the underlying sync failed.
    #[cfg(feature = "shielded")]
    pub async fn shielded_sync(&self) -> Result<Option<ShieldedSyncSummary>, PlatformWalletError> {
        let guard = self.shielded.read().await;
        match guard.as_ref() {
            Some(wallet) => Ok(Some(wallet.sync().await?)),
            None => Ok(None),
        }
    }

    /// The default Orchard payment address for this wallet, as the
    /// raw 43-byte representation. Returns `None` if the shielded
    /// sub-wallet hasn't been bound. Hosts apply their own bech32m
    /// encoding (HRP + 0x10 type byte) on top.
    #[cfg(feature = "shielded")]
    pub async fn shielded_default_address(&self) -> Option<[u8; 43]> {
        let guard = self.shielded.read().await;
        guard
            .as_ref()
            .map(|w| w.default_address().to_raw_address_bytes())
    }
}

impl PlatformWallet {
    // TODO: What these methods for? can we remove? Don't deelete this todo
    /// Queue a changeset for later persistence.
    pub fn queue_persist(&self, changeset: PlatformWalletChangeSet) {
        if let Err(e) = self.persister.store(changeset) {
            tracing::error!(
                error = %e,
                wallet_id = %hex::encode(self.wallet_id),
                "Failed to queue changeset for persistence"
            );
        }
    }

    /// Flush all queued changesets to the storage backend.
    pub fn flush_persist(&self) -> Result<(), PersistenceError> {
        self.persister.flush()
    }

    /// Load persisted state for this wallet.
    pub fn load_persisted(&self) -> Result<ClientStartState, PersistenceError> {
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
        // The platform-address sync watermark lives on the provider,
        // not on `PlatformWalletInfo`. Pull it out before handing the
        // changeset to `apply_changeset` (which consumes by value), then
        // feed it to the providers once apply completes.
        let pa_sync_state = changeset.platform_addresses.as_ref().map(|pa| {
            (
                pa.sync_height,
                pa.sync_timestamp,
                pa.last_known_recent_block,
            )
        });

        {
            let mut wm = self.wallet_manager.write().await;
            let (wallet, info) = wm
                .get_wallet_mut_and_info_mut(&self.wallet_id)
                .ok_or(crate::wallet::ApplyError::WalletNotFound(self.wallet_id))?;
            info.apply_changeset(wallet, changeset)?;
        }

        if let Some((height, timestamp, recent_block)) = pa_sync_state {
            self.platform
                .apply_sync_state(height, timestamp, recent_block)
                .await;
        }
        Ok(())
    }

    /// Load persisted state from the persister and apply it to the
    /// in-memory wallet. Convenience wrapper for
    /// `apply(load_persisted()?)`.
    ///
    /// **Idempotent** — safe to call multiple times. The apply path
    /// uses monotonic / OR merges on every field it touches
    /// (`highest_used` is MAX-merged, `utxos_instant_locked` is
    /// set-union), so re-applying the same persisted state is a no-op.
    ///
    /// This is the recommended entry point for startup hydration
    /// *after* late-registered accounts (e.g. DashPay contact
    /// accounts that `bootstrap_dashpay_contact_accounts` adds) have
    /// landed. The initial load_and_apply called during
    /// `PlatformWallet` construction only hydrates state for
    /// accounts that exist at that point; a second call after
    /// account bootstrap picks up the rest without regressing
    /// anything.
    pub async fn load_and_apply_persisted(
        &self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let ClientStartState {
            mut platform_addresses,
            wallets: _,
        } = self.load_persisted()?;

        if let Some(persisted) = platform_addresses.remove(&self.wallet_id) {
            self.platform
                .initialize_from_persisted(persisted)
                .await
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
        }

        Ok(())
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
            platform: self.platform.clone(),
            asset_locks: self.asset_locks.clone(),
            persister: self.persister.clone(),
            balance: self.balance.clone(),
            #[cfg(feature = "shielded")]
            shielded: self.shielded.clone(),
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
            .expect("wallet exists in guard")
    }
}

impl Deref for WalletStateReadGuard<'_> {
    type Target = PlatformWalletInfo;
    fn deref(&self) -> &PlatformWalletInfo {
        self.guard
            .get_wallet_info(&self.wallet_id)
            .expect("wallet exists in guard")
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
            .expect("wallet exists in guard")
    }
}

impl Deref for WalletStateWriteGuard<'_> {
    type Target = PlatformWalletInfo;
    fn deref(&self) -> &PlatformWalletInfo {
        self.guard
            .get_wallet_info(&self.wallet_id)
            .expect("wallet exists in guard")
    }
}

impl DerefMut for WalletStateWriteGuard<'_> {
    fn deref_mut(&mut self) -> &mut PlatformWalletInfo {
        self.guard
            .get_wallet_info_mut(&self.wallet_id)
            .expect("wallet exists in guard")
    }
}
