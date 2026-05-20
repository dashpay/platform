//! Network-scoped shielded coordinator.
//!
//! The Orchard commitment tree is chain-wide: every wallet and
//! every account on the same network sees the same `cmx` stream
//! in the same order, backs the same frontier, and shares the
//! same anchor set. The current per-`PlatformWallet` shielded
//! shape (each wallet owning its own [`ShieldedWallet`] and its
//! own [`FileBackedShieldedStore`] handle) duplicates the
//! fetch + trial-decrypt + tree-append work N times for N
//! wallets, and opens N concurrent SQLite handles into a single
//! `shielded_tree_<network>.sqlite` file.
//!
//! [`NetworkShieldedCoordinator`] is the single object that owns
//! everything chain-wide about shielded sync — the SQLite-backed
//! commitment tree, the per-`SubwalletId` notes/sync state, the
//! flat registry of every bound account's **viewing keys** (no
//! spend authority), the caught-up cooldown stamp, and the
//! persister handle. One instance per [`PlatformWalletManager`];
//! lazily constructed on the first `bind_shielded` call so
//! networks where no wallet uses shielded never open a SQLite
//! file.
//!
//! Privilege separation: the coordinator's account registry
//! holds [`AccountViewingKeys`] only — FVK / IVK / OVK / default
//! address. The `SpendAuthorizingKey` lives only on the
//! per-wallet side (in [`OrchardKeySet`]) and is passed into the
//! coordinator's spend methods at call time, never stored at
//! coordinator scope.
//!
//! # Phase 0 status
//!
//! This module currently contains the type skeleton only — the
//! coordinator is declared, its fields wired up, and its
//! lifecycle helpers documented. None of the existing sync /
//! spend code paths consume it yet; that wiring lands in
//! Phase 1+.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::RwLock;

use super::file_store::FileBackedShieldedStore;
use super::keys::AccountViewingKeys;
use super::store::SubwalletId;
use crate::wallet::persister::WalletPersister;

/// Network-scoped shielded coordinator.
///
/// See module docs for the architectural rationale.
///
/// `#[allow(dead_code)]` on the fields is the Phase-0 marker —
/// the type compiles and is exported, but nothing reads it yet.
/// The annotations come off as each phase wires its respective
/// field into the real code path.
#[allow(dead_code)]
pub struct NetworkShieldedCoordinator {
    /// Dash Platform SDK handle. The coordinator runs sync /
    /// nullifier-scan / broadcast against this SDK on behalf of
    /// every bound wallet.
    sdk: Arc<dash_sdk::Sdk>,

    /// Network this coordinator operates on. Pinned at
    /// construction and never mutated — networks each get their
    /// own coordinator instance.
    network: dashcore::Network,

    /// On-disk path to `shielded_tree_<network>.sqlite`. Stored so
    /// subsequent `configure_shielded` calls on the same manager
    /// can fail loudly if a caller passes a mismatched path
    /// (design-doc choice (c): explicit error rather than
    /// silently honor first or silently honor second).
    db_path: PathBuf,

    /// The single SQLite handle into `shielded_tree_<network>.sqlite`.
    /// Both the commitment tree (frontier, checkpoints, marked
    /// auth paths) and the per-[`SubwalletId`] notes / sync
    /// watermarks / nullifier checkpoints live here.
    ///
    /// Every wallet and every account on this network shares
    /// this `Arc<RwLock<_>>` — the single-handle property is
    /// what closes the SQLite-WAL contention and the
    /// delete-while-open race the prior architecture had.
    store: Arc<RwLock<FileBackedShieldedStore>>,

    /// Flat registry of every bound `(walletId, accountIndex)`
    /// pair's viewing keys, populated by
    /// [`register_wallet`](Self::register_wallet). The sync loop
    /// iterates this map once per pass to enumerate every IVK
    /// across every wallet, then trial-decrypts each fetched
    /// note against the union.
    ///
    /// Viewing-grade only: no [`SpendAuthorizingKey`] is ever
    /// stored at coordinator scope. Spend operations re-attach
    /// the ASK by accepting an [`OrchardKeySet`] parameter from
    /// the per-wallet caller.
    accounts: Arc<RwLock<BTreeMap<SubwalletId, AccountViewingKeys>>>,

    /// Persister handle attached when shielded support is first
    /// configured on the manager. The coordinator emits a single
    /// consolidated [`ShieldedChangeSet`] per sync pass — the
    /// changeset is already `SubwalletId`-keyed so per-wallet
    /// fan-out happens naturally on the host side.
    persister: Option<WalletPersister>,

    /// Timestamp of the last sync pass that observed no new
    /// commitments or newly-spent nullifiers — the caught-up
    /// cooldown stamp moves from per-`ShieldedWallet` scope to
    /// per-coordinator scope, so the cooldown applies once per
    /// network instead of once per wallet. Cleared on any
    /// activity; bypassed by `force` syncs.
    last_caught_up_at: std::sync::Mutex<Option<std::time::Instant>>,
}

#[allow(dead_code)]
impl NetworkShieldedCoordinator {
    /// Build a new coordinator. Called by
    /// `PlatformWalletManager::configure_shielded` (Phase 1) on
    /// the first shielded use on this network manager. The
    /// `db_path` is opened immediately; subsequent
    /// `configure_shielded` calls on the same manager verify the
    /// path matches and error otherwise (open question (a) from
    /// the design doc).
    pub fn new(
        sdk: Arc<dash_sdk::Sdk>,
        network: dashcore::Network,
        db_path: PathBuf,
        store: FileBackedShieldedStore,
        persister: Option<WalletPersister>,
    ) -> Self {
        Self {
            sdk,
            network,
            db_path,
            store: Arc::new(RwLock::new(store)),
            accounts: Arc::new(RwLock::new(BTreeMap::new())),
            persister,
            last_caught_up_at: std::sync::Mutex::new(None),
        }
    }

    /// Network this coordinator is pinned to. Used by hosts that
    /// need to assert their `PlatformWalletManager` and the
    /// coordinator agree on the network.
    pub fn network(&self) -> dashcore::Network {
        self.network
    }

    /// The on-disk SQLite path the coordinator opened. Used by
    /// `PlatformWalletManager::configure_shielded` to verify
    /// subsequent calls pass the same path.
    pub fn db_path(&self) -> &std::path::Path {
        &self.db_path
    }

    /// Reference to the shared store. The full sync / spend
    /// surfaces land on the coordinator itself; this accessor
    /// exists for tests and for migration scaffolding in Phase 1.
    pub fn store(&self) -> &Arc<RwLock<FileBackedShieldedStore>> {
        &self.store
    }
}
