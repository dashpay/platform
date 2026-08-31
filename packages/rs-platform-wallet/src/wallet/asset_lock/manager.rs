//! Asset lock lifecycle manager.
//!
//! Encapsulates all asset lock operations: building transactions, broadcasting,
//! waiting for proofs, and tracking lifecycle status. Shared across sub-wallets
//! via `Arc<AssetLockManager>`.

use std::collections::BTreeMap;
use std::sync::Arc;

use dashcore::OutPoint;
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
    /// Serializes the funding-index-critical section of
    /// [`broadcast_funded_asset_lock`](Self::broadcast_funded_asset_lock) —
    /// build (index allocation + in-memory mark-used) through the address-pool
    /// persist/flush. Without it, two concurrent builds can interleave so that
    /// the FIRST build's pool snapshot (collected before the second build
    /// marked its index) is persisted LAST, rolling the durable snapshot back
    /// to a state where the second index reads unused — after a restart the
    /// next invitation re-selects that index and re-exports the same bearer
    /// voucher key. A dedicated mutex (never held while awaiting
    /// `wallet_manager`'s lock from outside this section, and never acquired
    /// by code that already holds it) avoids the self-deadlock a
    /// `wallet_manager.write()` guard would cause across the build→persist
    /// span. Deliberately NOT held across the broadcast/proof-wait — only the
    /// snapshot ordering needs serialization.
    pub(super) build_persist_serial: tokio::sync::Mutex<()>,
    /// Outpoints whose `Built` row is excluded from rejected-build cleanup,
    /// counted so concurrent resumes of the same lock each hold their own
    /// claim. A live resume releases its claim on pre-dispatch exit. Once a
    /// send may have had side effects or local finality has been observed,
    /// cancellation leaves the claim sticky until the row advances beyond
    /// `Built`.
    ///
    /// The claim is what keeps a parked resume from broadcasting a
    /// transaction whose inputs the rejection cleanup has already released.
    /// A resume snapshots the row, then waits for the broadcast transport,
    /// sends, and only then records the send by advancing the row —
    /// suspension points the whole way. The claim remains after an ambiguous
    /// cancellation in that interval. The initial build's definite
    /// pre-send rejection removes the still-`Built` row and releases both
    /// the funding reservation and the in-broadcast fence, and its
    /// removal-guard (row still `Built`) cannot see a resume that has not
    /// reached its status advance yet. So without a claim the release can
    /// land inside the resume's dispatch window, and the resume then puts
    /// the original transaction on the wire from inputs a rebuild is free
    /// to reselect. [`untrack_asset_lock`](Self::untrack_asset_lock)
    /// therefore refuses the removal while a claim stands, which leaves the
    /// reservation and fence held and downgrades the build's verdict to the
    /// unknown outcome — exactly what it already reports when its guard
    /// fires on an advanced row.
    ///
    /// Atomicity comes from the wallet lock, not from this mutex: the claim
    /// is taken while the resume still holds the read guard it snapshotted
    /// the row under, and read while the cleanup holds the write guard it
    /// removes the row under. The two guards exclude each other, so either
    /// the cleanup sees the claim and keeps the row, or it removed the row
    /// before the resume could snapshot it and the resume finds nothing to
    /// resume. This mutex is only ever held for map arithmetic — never
    /// across an await.
    ///
    /// Per-manager rather than per-wallet state because a registered wallet
    /// has exactly one `AssetLockManager`, shared as `Arc<AssetLockManager>`
    /// across every sub-wallet, so a build and a resume of the same lock
    /// always meet here — the same reasoning that puts
    /// `build_persist_serial` above on the manager. A manager handle that
    /// outlives its registration keeps its own map, which claims nothing
    /// about outpoints: a re-registration allocates a fresh funding index
    /// and therefore a different funding transaction, so no build and resume
    /// of ONE outpoint can end up on two maps.
    pub(super) resume_dispatch_claims: std::sync::Mutex<BTreeMap<OutPoint, usize>>,
    /// Test-only gauge of builds currently at or past the
    /// `build_persist_serial` gate within `broadcast_funded_asset_lock`
    /// (incremented before the `lock().await`, RAII-decremented on every
    /// exit from the call). Lets the concurrency regression test
    /// synchronize on "the second build has reached the serialization
    /// gate" instead of assuming a scheduling delay — while the first
    /// build holds the lock, a gauge of 2 proves the second build cannot
    /// yet have collected its pool snapshot.
    #[cfg(test)]
    pub(super) build_serial_gate: std::sync::atomic::AtomicUsize,
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
            build_persist_serial: tokio::sync::Mutex::new(()),
            resume_dispatch_claims: std::sync::Mutex::new(BTreeMap::new()),
            #[cfg(test)]
            build_serial_gate: std::sync::atomic::AtomicUsize::new(0),
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
