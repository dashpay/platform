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
//! # Status (Phase 2a)
//!
//! - **Phase 0** (landed): type skeleton + viewing-key split.
//! - **Phase 1** (landed): the coordinator owns the single
//!   `Arc<RwLock<FileBackedShieldedStore>>` for the network;
//!   every `PlatformWallet::bind_shielded` reuses it.
//! - **Phase 2a** (this module): the coordinator now owns the
//!   network-wide caught-up cooldown and the
//!   [`sync`](NetworkShieldedCoordinator::sync) entry point that
//!   `ShieldedSyncManager::sync_now` routes through. Per-wallet
//!   iteration still calls into
//!   [`PlatformWallet::shielded_sync(force=true)`] under the
//!   coordinator's cooldown gate.
//! - **Phase 2b** (next): replace the per-wallet iteration with a
//!   single network-wide fetch + multi-IVK trial-decrypt against
//!   the union of every registered subwallet — collapses N SDK
//!   calls per pass to 1.
//! - **Phase 4** (later): delete `ShieldedWallet`, flatten
//!   `PlatformWallet`'s shielded surface, and have the coordinator
//!   own the spend path too (accepting an `OrchardKeySet` at
//!   call time for the ASK).

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;

use super::file_store::FileBackedShieldedStore;
use super::keys::AccountViewingKeys;
use super::store::SubwalletId;
use super::CAUGHT_UP_COOLDOWN;
use crate::manager::shielded_sync::{ShieldedSyncPassSummary, WalletShieldedOutcome};
use crate::wallet::persister::WalletPersister;
use crate::wallet::platform_wallet::WalletId;
use crate::wallet::PlatformWallet;

/// Network-scoped shielded coordinator.
///
/// See module docs for the architectural rationale.
///
/// As of Phase 2a, the coordinator owns the network-wide
/// caught-up cooldown and the sync entry point that
/// [`ShieldedSyncManager`](crate::manager::shielded_sync::ShieldedSyncManager)
/// drives — wallet iteration still delegates to
/// [`PlatformWallet::shielded_sync`] under the hood until Phase 2b
/// collapses the N per-wallet SDK fetches into one
/// network-wide fetch + multi-IVK trial-decrypt pass.
pub struct NetworkShieldedCoordinator {
    /// Dash Platform SDK handle. The coordinator runs sync /
    /// nullifier-scan / broadcast against this SDK on behalf of
    /// every bound wallet. Held but unused in Phase 2a — the
    /// per-wallet `ShieldedWallet` still owns the SDK call in
    /// its own `sync_notes`. Phase 2b lifts that call up here.
    #[allow(dead_code)]
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
    /// fan-out happens naturally on the host side. Held but
    /// unused in Phase 2a — per-wallet `ShieldedWallet`s still
    /// queue their own changesets through their own persister
    /// clones. Phase 2b moves the queueing here.
    #[allow(dead_code)]
    persister: Option<WalletPersister>,

    /// Timestamp of the last sync pass that observed no new
    /// commitments or newly-spent nullifiers — the caught-up
    /// cooldown stamp moves from per-`ShieldedWallet` scope to
    /// per-coordinator scope, so the cooldown applies once per
    /// network instead of once per wallet. Cleared on any
    /// activity; bypassed by `force` syncs.
    last_caught_up_at: std::sync::Mutex<Option<Instant>>,

    /// Shared handle to the manager's wallets map. The coordinator
    /// looks up `Arc<PlatformWallet>` by [`WalletId`] when its
    /// [`sync`](Self::sync) iterates registered subwallets.
    /// Held as a cloned `Arc` of the same `RwLock` the manager
    /// owns, so wallets added after [`configure_shielded`] are
    /// visible to the coordinator on the next sync pass without
    /// any explicit re-registration.
    wallets: Arc<RwLock<BTreeMap<WalletId, Arc<PlatformWallet>>>>,
}

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
        wallets: Arc<RwLock<BTreeMap<WalletId, Arc<PlatformWallet>>>>,
    ) -> Self {
        Self {
            sdk,
            network,
            db_path,
            store: Arc::new(RwLock::new(store)),
            accounts: Arc::new(RwLock::new(BTreeMap::new())),
            persister,
            last_caught_up_at: std::sync::Mutex::new(None),
            wallets,
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

    /// Register every account of a newly-bound shielded wallet so
    /// future [`sync`](Self::sync) passes iterate it. Called by
    /// [`PlatformWallet::bind_shielded`] after the per-wallet
    /// [`ShieldedWallet`] has been constructed.
    ///
    /// Privilege boundary: only the viewing-key subset
    /// ([`AccountViewingKeys`]) is handed to the coordinator. The
    /// `SpendAuthorizingKey` stays on the per-wallet side
    /// (`OrchardKeySet`) and is re-attached at spend-call time.
    ///
    /// Idempotent: a second call with the same `wallet_id`
    /// replaces every previously-registered account for that
    /// wallet (so a re-bind after a clear is consistent).
    ///
    /// [`ShieldedWallet`]: super::ShieldedWallet
    /// [`PlatformWallet::bind_shielded`]: crate::wallet::PlatformWallet::bind_shielded
    pub async fn register_wallet(
        &self,
        wallet_id: WalletId,
        account_views: BTreeMap<u32, AccountViewingKeys>,
    ) {
        let mut accounts = self.accounts.write().await;
        // Drop any prior subwallets for this wallet_id before
        // installing the new set so a re-bind with a different
        // account list doesn't leave orphan entries.
        accounts.retain(|id, _| id.wallet_id != wallet_id);
        for (account_index, views) in account_views {
            accounts.insert(SubwalletId::new(wallet_id, account_index), views);
        }
    }

    /// Remove every account belonging to `wallet_id` from the
    /// coordinator's registry. No-op if the wallet wasn't
    /// registered. Called when a wallet is unregistered from the
    /// manager or when its shielded binding is cleared.
    pub async fn unregister_wallet(&self, wallet_id: WalletId) {
        let mut accounts = self.accounts.write().await;
        accounts.retain(|id, _| id.wallet_id != wallet_id);
    }

    /// Currently-registered subwallet ids (snapshot, ascending
    /// `(wallet_id, account_index)` order). Exposed for tests and
    /// for the sync coordinator's pass enumeration.
    pub async fn registered_subwallets(&self) -> Vec<SubwalletId> {
        self.accounts.read().await.keys().copied().collect()
    }

    /// Run one shielded sync pass for every registered wallet on
    /// this coordinator's network. Returns a per-wallet outcome
    /// summary suitable for emission to UI / persistence layers
    /// via [`PlatformEventManager::on_shielded_sync_completed`].
    ///
    /// `force=false` honors the coordinator-scoped caught-up
    /// cooldown — a sync pass that observed nothing new on any
    /// registered subwallet suppresses subsequent background
    /// passes for [`CAUGHT_UP_COOLDOWN`]. `force=true` (the
    /// user-initiated "Sync Now" path) bypasses the cooldown and
    /// always walks Platform.
    ///
    /// Phase-2a shape: this method iterates the registered
    /// wallets and delegates each one to
    /// [`PlatformWallet::shielded_sync(true)`] under the
    /// coordinator's cooldown gate. Phase 2b collapses the N
    /// per-wallet SDK fetches into a single network-wide fetch +
    /// multi-IVK trial-decrypt pass against the union of every
    /// subwallet's [`AccountViewingKeys`].
    ///
    /// [`PlatformEventManager::on_shielded_sync_completed`]:
    ///     crate::events::PlatformEventManager::on_shielded_sync_completed
    pub async fn sync(&self, force: bool) -> ShieldedSyncPassSummary {
        // Network-wide cooldown gate. Snapshot the remaining
        // window into a local before any await — `std::sync::Mutex`
        // is `!Send` across await points (clippy's
        // `await_holding_lock` lint flags this).
        let cooldown_remaining: Option<Duration> = if force {
            None
        } else {
            self.last_caught_up_at
                .lock()
                .ok()
                .and_then(|guard| *guard)
                .map(|when| CAUGHT_UP_COOLDOWN.saturating_sub(when.elapsed()))
                .filter(|remaining| !remaining.is_zero())
        };

        if let Some(remaining) = cooldown_remaining {
            tracing::debug!(
                cooldown_remaining_secs = remaining.as_secs(),
                cooldown_total_secs = CAUGHT_UP_COOLDOWN.as_secs(),
                "Coordinator sync skipped — within caught-up cooldown"
            );
            return Self::cooldown_skip_summary(self).await;
        }

        // Snapshot the registered wallet ids first, then look up
        // their `Arc<PlatformWallet>` clones from the shared
        // wallets map. The coordinator iterates by `WalletId`
        // (not by `SubwalletId`) because `PlatformWallet::shielded_sync`
        // already runs every bound account in a single chain walk.
        let registered_wallet_ids: Vec<WalletId> = {
            let accounts = self.accounts.read().await;
            let mut ids: Vec<WalletId> = accounts.keys().map(|id| id.wallet_id).collect();
            ids.sort_unstable();
            ids.dedup();
            ids
        };

        let mut summary = ShieldedSyncPassSummary::default();
        if registered_wallet_ids.is_empty() {
            return summary;
        }

        let wallets_snapshot: Vec<(WalletId, Option<Arc<PlatformWallet>>)> = {
            let wallets = self.wallets.read().await;
            registered_wallet_ids
                .iter()
                .map(|id| (*id, wallets.get(id).cloned()))
                .collect()
        };

        let mut any_activity = false;
        for (wallet_id, wallet) in wallets_snapshot {
            let Some(wallet) = wallet else {
                // Registered in coordinator but missing from the
                // wallets map — host inconsistency. Skip with a
                // warn so this surfaces in logs but a single
                // missing wallet doesn't poison the whole pass.
                tracing::warn!(
                    wallet_id = %hex::encode(wallet_id),
                    "Shielded sync skipped: wallet registered on coordinator but not in wallets map"
                );
                summary
                    .wallet_results
                    .insert(wallet_id, WalletShieldedOutcome::Skipped);
                continue;
            };

            // Always force the per-wallet path — the coordinator
            // already gated on the network-wide cooldown above,
            // so the per-wallet `last_caught_up_at` would
            // double-gate and miss new chunks during the
            // cooldown window.
            let outcome = match wallet.shielded_sync(true).await {
                Ok(Some(result)) => {
                    if !result.is_cooldown_skip
                        && (result.notes_result.total_scanned > 0 || result.total_newly_spent() > 0)
                    {
                        any_activity = true;
                    }
                    WalletShieldedOutcome::Ok(result)
                }
                Ok(None) => WalletShieldedOutcome::Skipped,
                Err(e) => {
                    tracing::warn!(
                        wallet_id = %hex::encode(wallet_id),
                        error = %e,
                        "Shielded sync failed via coordinator"
                    );
                    WalletShieldedOutcome::Err(e.to_string())
                }
            };
            summary.wallet_results.insert(wallet_id, outcome);
        }

        // Update the network-wide cooldown stamp based on
        // aggregate activity. Any new commitment or newly-spent
        // nullifier anywhere in the network clears the stamp so
        // the next pass runs immediately.
        if let Ok(mut guard) = self.last_caught_up_at.lock() {
            if any_activity {
                *guard = None;
            } else {
                *guard = Some(Instant::now());
            }
        }

        summary.sync_unix_seconds = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        summary
    }

    /// Build a pass summary in which every registered wallet is
    /// reported as a cooldown skip. Used by [`sync`](Self::sync)
    /// when the network-wide cooldown is in effect.
    async fn cooldown_skip_summary(&self) -> ShieldedSyncPassSummary {
        use super::sync::{ShieldedSyncSummary, SyncNotesResult};

        let registered_wallet_ids: Vec<WalletId> = {
            let accounts = self.accounts.read().await;
            let mut ids: Vec<WalletId> = accounts.keys().map(|id| id.wallet_id).collect();
            ids.sort_unstable();
            ids.dedup();
            ids
        };

        let mut summary = ShieldedSyncPassSummary::default();
        for wallet_id in registered_wallet_ids {
            summary.wallet_results.insert(
                wallet_id,
                WalletShieldedOutcome::Ok(ShieldedSyncSummary {
                    notes_result: SyncNotesResult::default(),
                    newly_spent_per_account: BTreeMap::new(),
                    balances: BTreeMap::new(),
                    is_cooldown_skip: true,
                }),
            );
        }
        summary.sync_unix_seconds = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        summary
    }
}
