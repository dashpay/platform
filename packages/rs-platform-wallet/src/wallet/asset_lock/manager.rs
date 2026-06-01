//! Asset lock lifecycle manager.
//!
//! Encapsulates all asset lock operations: building transactions, broadcasting,
//! waiting for proofs, and tracking lifecycle status. Shared across sub-wallets
//! via `Arc<AssetLockManager>`.

use std::sync::Arc;

use tokio::sync::{Notify, RwLock};

use crate::broadcaster::TransactionBroadcaster;
use crate::changeset::changeset::AssetLockChangeSet;
use crate::wallet::persister::WalletPersister;
use crate::wallet::platform_wallet::{PlatformWalletInfo, WalletId};

use super::tracked::TrackedAssetLock;
use key_wallet_manager::WalletManager;

/// Default fee rate in duffs per kilobyte for asset lock transactions.
pub(super) const DEFAULT_FEE_PER_KB: u64 = 1000;

/// Manages the full asset lock lifecycle: build, broadcast, proof, and tracking.
///
/// Shared across sub-wallets via `Arc<AssetLockManager>` so that any sub-wallet
/// (identity, platform-address, shielded) can create and consume asset locks
/// without going through `CoreWallet`.
///
/// `B` is the concrete broadcaster type; every `broadcast()` call dispatches
/// statically instead of through a `dyn` vtable.
pub struct AssetLockManager<B: TransactionBroadcaster + ?Sized> {
    pub(super) sdk: Arc<dash_sdk::Sdk>,
    /// The shared wallet manager lock for all mutable wallet state.
    pub(super) wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    /// Identifies which wallet within the manager this manager operates on.
    pub(super) wallet_id: WalletId,
    /// Notified on InstantLock / ChainLock events by SpvEventForwarder.
    /// Used by `wait_for_proof()` and `wait_for_chain_lock()`.
    pub(super) lock_notify: Arc<Notify>,
    /// Transaction broadcaster — pluggable so the same `AssetLockManager`
    /// works with different broadcast backends:
    ///
    /// - [`DapiBroadcaster`](crate::broadcaster::DapiBroadcaster) — gRPC via
    ///   Platform DAPI (default for standalone wallets without SPV).
    /// - [`SpvBroadcaster`](crate::broadcaster::SpvBroadcaster) — P2P via SPV
    ///   peers (used when managed by `PlatformWalletManager` with SPV enabled).
    ///
    /// Injected at construction by `PlatformWallet::new()`. The caller
    /// (typically `PlatformWalletManager`) decides which implementation to use.
    pub(super) broadcaster: Arc<B>,
    /// Per-wallet persistence handle. Cloned from the parent
    /// `PlatformWallet` at construction so asset lock mutations can
    /// queue their own `AssetLockChangeSet`s into the changeset flush
    /// boundary without round-tripping through the parent wallet.
    ///
    /// Item 8 sub-step 1a: previously mutations returned
    /// `AssetLockChangeSet` and callers (including
    /// `create_funded_asset_lock_proof` itself) dropped them with
    /// `let _cs = ...`. Every emitted changeset now flows straight
    /// into `queue_persist` here.
    pub(super) persister: WalletPersister,
}

impl<B: TransactionBroadcaster + ?Sized> AssetLockManager<B> {
    /// Create a new `AssetLockManager`.
    pub(crate) fn new(
        sdk: Arc<dash_sdk::Sdk>,
        wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
        wallet_id: WalletId,
        lock_notify: Arc<Notify>,
        broadcaster: Arc<B>,
        persister: WalletPersister,
    ) -> Self {
        Self {
            sdk,
            wallet_manager,
            wallet_id,
            lock_notify,
            broadcaster,
            persister,
        }
    }

    /// Queue an `AssetLockChangeSet` onto the per-wallet persister.
    /// No-op when the changeset is empty.
    ///
    /// `pub(crate)` so the orchestrated funding flows in
    /// `wallet::platform_addresses` and `wallet::identity::network`
    /// can pair an `advance_asset_lock_status` call with a flush
    /// without going through the asset-lock module boundary. The
    /// internal-only flag (no `pub`) keeps the API hidden from
    /// crate consumers.
    pub(crate) fn queue_asset_lock_changeset(&self, cs: AssetLockChangeSet) {
        if <AssetLockChangeSet as crate::changeset::Merge>::is_empty(&cs) {
            return;
        }
        if let Err(e) = self
            .persister
            .store(crate::changeset::PlatformWalletChangeSet {
                asset_locks: Some(cs),
                ..Default::default()
            })
        {
            tracing::error!(
                error = %e,
                "AssetLockManager: failed to queue changeset for persistence"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Public read accessors
// ---------------------------------------------------------------------------

impl<B: TransactionBroadcaster + ?Sized> AssetLockManager<B> {
    /// Wallet id this manager operates on. Exposed so FFI callers that
    /// build a `MnemonicResolverCoreSigner` (or similar) on demand can
    /// thread the wallet id through to the resolver callback without
    /// reaching into private fields.
    pub fn wallet_id(&self) -> WalletId {
        self.wallet_id
    }

    /// Network the SDK was constructed with. Same rationale as
    /// [`Self::wallet_id`] — needed by FFI callers that build a
    /// `key_wallet::signer::Signer` per call.
    pub fn network(&self) -> dashcore::Network {
        self.sdk.network
    }

    /// List all tracked asset locks (blocking version for UI / synchronous contexts).
    ///
    /// Uses `tokio::sync::RwLock::blocking_read` — must NOT be called from
    /// within a tokio async context.
    pub fn list_tracked_locks_blocking(&self) -> Vec<TrackedAssetLock> {
        let wm = self.wallet_manager.blocking_read();
        wm.get_wallet_info(&self.wallet_id)
            .map(|info| info.tracked_asset_locks.values().cloned().collect())
            .unwrap_or_default()
    }

    /// List all tracked asset locks (async version).
    pub async fn list_tracked_locks(&self) -> Vec<TrackedAssetLock> {
        let wm = self.wallet_manager.read().await;
        wm.get_wallet_info(&self.wallet_id)
            .map(|info| info.tracked_asset_locks.values().cloned().collect())
            .unwrap_or_default()
    }
}

impl<B: TransactionBroadcaster + ?Sized> std::fmt::Debug for AssetLockManager<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AssetLockManager")
            .field("network", &self.sdk.network)
            .finish()
    }
}
