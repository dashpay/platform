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
//! # Status (Phase 2b)
//!
//! - **Phase 0** (landed): type skeleton + viewing-key split.
//! - **Phase 1** (landed): the coordinator owns the single
//!   `Arc<RwLock<FileBackedShieldedStore>>` for the network;
//!   every `PlatformWallet::bind_shielded` reuses it.
//! - **Phase 2a** (landed): the coordinator owns the
//!   network-wide caught-up cooldown and the
//!   [`sync`](NetworkShieldedCoordinator::sync) entry point that
//!   `ShieldedSyncManager::sync_now` routes through.
//! - **Phase 2b** (this module): the coordinator drives sync
//!   itself via [`sync_notes_across`] — a single network-wide
//!   SDK fetch + multi-IVK trial-decrypt against the union of
//!   every registered subwallet, collapsing N per-wallet SDK
//!   calls per pass to 1. The consolidated changeset is split
//!   per-`WalletId` and queued through each registered
//!   [`WalletPersister`].
//! - **Phase 4** (later): delete `ShieldedWallet`, flatten
//!   `PlatformWallet`'s shielded surface, and have the
//!   coordinator own the spend path too (accepting an
//!   `OrchardKeySet` at call time for the ASK).
//!
//! [`sync_notes_across`]: super::sync::sync_notes_across

use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

/// Callback fired once per chunk during a coordinator sync pass —
/// the **"downloaded"** progress signal.
/// Arguments: `(cumulative_scanned, latest_block_height)`. Forwarded
/// straight from the SDK stream's per-chunk download completion.
pub type ShieldedProgressCallback = Arc<dyn Fn(u64, u64) + Send + Sync>;

/// Callback fired as commitments are committed to the coordinator's
/// local Merkle tree — the **"checked / committed-to-tree"** progress
/// signal, distinct from [`ShieldedProgressCallback`] (network
/// download). Fired once per appended batch during the interleaved
/// stream consume in [`sync_notes_across`].
///
/// Arguments: `(cumulative_leaves_committed, total_leaves_target)`.
/// `total_leaves_target` is the on-chain MMR leaf count fetched once at
/// the start of the pass; it is `0` when that progress-only RPC failed,
/// which the UI should treat as an indeterminate total.
///
/// [`sync_notes_across`]: super::sync::sync_notes_across
pub type ShieldedTreeProgressCallback = Arc<dyn Fn(u64, u64) + Send + Sync>;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;

use super::file_store::FileBackedShieldedStore;
use super::keys::AccountViewingKeys;
use super::store::{ShieldedStore, StalePendingSpend, SubwalletId};
use super::CAUGHT_UP_COOLDOWN;
use crate::manager::shielded_sync::{ShieldedSyncPassSummary, WalletShieldedOutcome};
use crate::wallet::persister::WalletPersister;
use crate::wallet::platform_wallet::WalletId;

/// Network-scoped shielded coordinator.
///
/// See module docs for the architectural rationale.
///
/// As of Phase 2b, the coordinator owns the entire sync path
/// for the network: a single SDK fetch + multi-IVK
/// trial-decrypt against the union of every registered
/// subwallet, with the consolidated changeset split per-`WalletId`
/// and queued through each registered persister.
pub struct NetworkShieldedCoordinator {
    /// Dash Platform SDK handle. The coordinator runs the note
    /// scan (which also detects spends) and broadcast against this
    /// SDK on behalf of every bound wallet.
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

    /// Per-wallet persister handles, populated by
    /// [`register_wallet`](Self::register_wallet) alongside the
    /// account registry. The Phase-2b sync builds a single
    /// consolidated [`ShieldedChangeSet`] spanning every
    /// touched subwallet, then
    /// [`ShieldedChangeSet::split_by_wallet_id`] fans it back
    /// out so each per-wallet `WalletPersister.store(...)` only
    /// sees its own `wallet_id`'s deltas. (The wire format
    /// requires this — `WalletPersister` is wallet-scoped and
    /// `inner.store(wallet_id, ...)` always passes its bound
    /// wallet_id to the durable layer.)
    persisters: Arc<RwLock<BTreeMap<WalletId, WalletPersister>>>,

    /// Timestamp of the last sync pass that observed no new
    /// commitments or newly-spent nullifiers — the caught-up
    /// cooldown stamp moves from per-`ShieldedWallet` scope to
    /// per-coordinator scope, so the cooldown applies once per
    /// network instead of once per wallet. Cleared on any
    /// activity; bypassed by `force` syncs.
    last_caught_up_at: std::sync::Mutex<Option<Instant>>,

    /// Optional progress callback fired once per chunk inside
    /// `sync_shielded_notes`. Lets the manager translate chunk-level
    /// progress into `PlatformEventHandler::on_shielded_sync_progress`
    /// events without sync_notes_across knowing about the event
    /// manager. Installed by the manager via
    /// [`install_progress_handler`](Self::install_progress_handler);
    /// `None` (default) disables progress reporting (the test path).
    ///
    /// `std::sync::Mutex` rather than `ArcSwap` because `arc_swap`
    /// requires `T: Sized` and we need to hold a `dyn Fn`. The lock
    /// is taken once per sync pass to read the snapshot — no hot-path
    /// contention.
    progress_handler: std::sync::Mutex<Option<ShieldedProgressCallback>>,

    /// Optional tree-progress callback fired as commitments are
    /// committed to the local Merkle tree during the interleaved sync
    /// (the "checked" signal, distinct from `progress_handler`'s
    /// "downloaded" signal). Installed by the manager via
    /// [`install_tree_progress_handler`](Self::install_tree_progress_handler);
    /// `None` (default) disables tree-progress reporting.
    ///
    /// Same `std::sync::Mutex` rationale as `progress_handler`.
    tree_progress_handler: std::sync::Mutex<Option<ShieldedTreeProgressCallback>>,
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
    ) -> Self {
        Self {
            sdk,
            network,
            db_path,
            store: Arc::new(RwLock::new(store)),
            accounts: Arc::new(RwLock::new(BTreeMap::new())),
            persisters: Arc::new(RwLock::new(BTreeMap::new())),
            last_caught_up_at: std::sync::Mutex::new(None),
            progress_handler: std::sync::Mutex::new(None),
            tree_progress_handler: std::sync::Mutex::new(None),
        }
    }

    /// Network this coordinator is pinned to. Used by hosts that
    /// need to assert their `PlatformWalletManager` and the
    /// coordinator agree on the network.
    pub fn network(&self) -> dashcore::Network {
        self.network
    }

    /// Install (or replace) the per-chunk progress handler. The
    /// callback runs from inside `sync_shielded_notes`'s chunk loop
    /// — once per ~2048 notes processed — so keep it cheap. Used by
    /// `PlatformWalletManager` to bridge sync-internal progress into
    /// `PlatformEventHandler::on_shielded_sync_progress` events.
    /// Passing `None` removes any installed handler.
    pub fn install_progress_handler(&self, handler: Option<ShieldedProgressCallback>) {
        if let Ok(mut slot) = self.progress_handler.lock() {
            *slot = handler;
        }
    }

    /// Snapshot of the currently installed progress handler.
    pub(super) fn progress_handler(&self) -> Option<ShieldedProgressCallback> {
        self.progress_handler.lock().ok().and_then(|g| g.clone())
    }

    /// Install (or replace) the tree-progress handler — the "checked /
    /// committed-to-tree" signal fired as commitments are appended to
    /// the local Merkle tree during the interleaved sync. Fired once per
    /// appended batch (~8192-note batches), so it's already coarse;
    /// still keep the callback cheap. Used by `PlatformWalletManager`
    /// to bridge tree progress into a second progress bar, distinct
    /// from the download progress handler. Passing `None` removes any
    /// installed handler.
    pub fn install_tree_progress_handler(&self, handler: Option<ShieldedTreeProgressCallback>) {
        if let Ok(mut slot) = self.tree_progress_handler.lock() {
            *slot = handler;
        }
    }

    /// Snapshot of the currently installed tree-progress handler.
    pub(super) fn tree_progress_handler(&self) -> Option<ShieldedTreeProgressCallback> {
        self.tree_progress_handler
            .lock()
            .ok()
            .and_then(|g| g.clone())
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
    /// future [`sync`](Self::sync) passes iterate it, and attach
    /// the wallet's [`WalletPersister`] so the coordinator can
    /// queue per-wallet slices of the consolidated changeset.
    /// Called by [`PlatformWallet::bind_shielded`] after the
    /// per-wallet [`ShieldedWallet`] has been constructed.
    ///
    /// Privilege boundary: only the viewing-key subset
    /// ([`AccountViewingKeys`]) is handed to the coordinator. The
    /// `SpendAuthorizingKey` stays on the per-wallet side
    /// (`OrchardKeySet`) and is re-attached at spend-call time.
    ///
    /// Idempotent: a second call with the same `wallet_id`
    /// replaces every previously-registered account and the
    /// persister handle for that wallet (so a re-bind after a
    /// clear is consistent).
    ///
    /// [`ShieldedWallet`]: super::ShieldedWallet
    /// [`PlatformWallet::bind_shielded`]: crate::wallet::PlatformWallet::bind_shielded
    pub async fn register_wallet(
        &self,
        wallet_id: WalletId,
        account_views: BTreeMap<u32, AccountViewingKeys>,
        persister: WalletPersister,
    ) {
        let mut accounts = self.accounts.write().await;
        // Drop any prior subwallets for this wallet_id before
        // installing the new set so a re-bind with a different
        // account list doesn't leave orphan entries.
        accounts.retain(|id, _| id.wallet_id != wallet_id);
        for (account_index, views) in account_views {
            accounts.insert(SubwalletId::new(wallet_id, account_index), views);
        }
        drop(accounts);
        self.persisters.write().await.insert(wallet_id, persister);
    }

    /// Remove every account belonging to `wallet_id` from the
    /// coordinator's registry, drop the persister handle, and
    /// purge the wallet's per-subwallet store state (decrypted
    /// notes, spent marks, `last_synced_note_index`, nullifier
    /// checkpoints). The shared commitment tree is left intact —
    /// it's a chain-wide structure, not per-wallet. No-op for
    /// parts that weren't present. Called when a wallet is
    /// removed from the manager.
    ///
    /// Purging the store watermark matters: without it, a later
    /// re-bind of the same wallet would resume from the stale
    /// `last_synced_note_index` and silently skip re-emitting its
    /// notes to the host.
    pub async fn unregister_wallet(&self, wallet_id: WalletId) {
        self.accounts
            .write()
            .await
            .retain(|id, _| id.wallet_id != wallet_id);
        self.persisters.write().await.remove(&wallet_id);
        if let Err(e) = self.store.write().await.purge_wallet(wallet_id) {
            tracing::warn!(
                wallet_id = %hex::encode(wallet_id),
                error = %e,
                "Failed to purge per-subwallet store state on unregister"
            );
        }
    }

    /// Currently-registered subwallet ids (snapshot, ascending
    /// `(wallet_id, account_index)` order). Exposed for tests and
    /// for the sync coordinator's pass enumeration.
    pub async fn registered_subwallets(&self) -> Vec<SubwalletId> {
        self.accounts.read().await.keys().copied().collect()
    }

    /// Rehydrate per-subwallet state from a host-persisted
    /// snapshot for the wallet identified by `wallet_id`. Should
    /// be called after [`register_wallet`](Self::register_wallet)
    /// and before the first sync pass so the in-memory store
    /// matches what the host already has on disk (notes, spent
    /// marks, sync watermarks, nullifier checkpoints).
    ///
    /// Filters the supplied [`ShieldedSyncStartState`] in two
    /// ways:
    /// - **By `wallet_id`**: only entries whose `SubwalletId`
    ///   belongs to `wallet_id` are restored. The startup
    ///   snapshot is keyed globally by `SubwalletId` but a single
    ///   `restore_for_wallet` call only owns one wallet's slice;
    ///   the host typically loops over registered wallets and
    ///   calls this once per wallet so each per-wallet `bind`
    ///   flow drops in its own state.
    /// - **By registered account**: subwallets whose
    ///   `account_index` isn't currently registered on this
    ///   coordinator are skipped — they'd accumulate state we
    ///   can never spend (no `OrchardKeySet` for them on the
    ///   per-wallet side).
    ///
    /// No-op on empty snapshots.
    pub async fn restore_for_wallet(
        &self,
        wallet_id: WalletId,
        snapshot: &crate::changeset::ShieldedSyncStartState,
    ) -> Result<(), crate::error::PlatformWalletError> {
        if snapshot.is_empty() {
            return Ok(());
        }
        // Snapshot of registered subwallets for the membership
        // check. Cheaper than holding the accounts read lock
        // across the store write below.
        let registered: std::collections::BTreeSet<SubwalletId> = {
            let accounts = self.accounts.read().await;
            accounts
                .keys()
                .copied()
                .filter(|id| id.wallet_id == wallet_id)
                .collect()
        };
        if registered.is_empty() {
            return Ok(());
        }

        let mut store = self.store.write().await;
        for (id, sub) in &snapshot.per_subwallet {
            // Only restore subwallets that belong to `wallet_id`
            // and are registered on this coordinator.
            if id.wallet_id != wallet_id || !registered.contains(id) {
                continue;
            }
            for note in &sub.notes {
                store.save_note(*id, note).map_err(|e| {
                    crate::error::PlatformWalletError::ShieldedStoreError(e.to_string())
                })?;
                if note.is_spent {
                    store.mark_spent(*id, &note.nullifier).map_err(|e| {
                        crate::error::PlatformWalletError::ShieldedStoreError(e.to_string())
                    })?;
                }
            }
            // Rehydrate recovered outgoing (sent) notes so send history
            // survives a cold start without re-OVK-recovering. Idempotent
            // by `cmx`, so a later re-scan that re-recovers the same note
            // is a no-op.
            for out in &sub.outgoing_notes {
                store.record_outgoing_note(*id, out).map_err(|e| {
                    crate::error::PlatformWalletError::ShieldedStoreError(e.to_string())
                })?;
            }
            // Rehydrate persisted activity entries so the scan deriver's
            // dedupe set (`existing_ids`) includes them this session — a
            // rich live entry restored here is never clobbered by a
            // coarser re-derivation. Idempotent (upsert by `entry.id`).
            for entry in &sub.activity {
                store.save_activity(*id, entry).map_err(|e| {
                    crate::error::PlatformWalletError::ShieldedStoreError(e.to_string())
                })?;
            }
            store
                .set_last_synced_note_index(*id, sub.last_synced_index)
                .map_err(|e| {
                    crate::error::PlatformWalletError::ShieldedStoreError(e.to_string())
                })?;
        }
        Ok(())
    }

    /// Drop every wallet registration, purge all per-subwallet
    /// store state (notes, spent marks, sync watermarks,
    /// nullifier checkpoints), empty the shared commitment tree,
    /// and reset the cooldown stamp. The single SQLite handle stays
    /// open — Clear semantics on the host side are "wipe my
    /// persistence and cold-rebuild from index 0", not "blow away
    /// the SQLite file".
    ///
    /// Purging the in-memory `subwallets` store is what actually
    /// delivers the "re-sync from index 0" contract: the sync
    /// pass derives `already_have` from each subwallet's
    /// `last_synced_note_index`, so leaving stale watermarks
    /// behind would make a same-session re-bind report
    /// caught-up and never re-emit notes to the host (it would
    /// only work after a process restart that drops the
    /// in-memory state). Clearing it here closes that gap.
    ///
    /// Emptying the commitment tree is what keeps the two reset
    /// halves coherent. The watermark rewinds to 0 but the tree's
    /// append gate is `tree_size`, so a tree left at its full
    /// (~1M-leaf) size would gate-skip every re-downloaded position
    /// (`global_pos < tree_size`) — nothing new appends, the
    /// "Checked" progress bar stays pinned at the stale leaf count
    /// while "Downloaded" climbs from 0, and the host pointlessly
    /// re-downloads into an already-complete tree. Resetting the
    /// tree alongside the watermarks makes Clear+resync a true cold
    /// rebuild: `tree_size` returns to 0 and "Checked" climbs 0→N
    /// trailing "Downloaded".
    ///
    /// Used by [`platform_wallet_manager_shielded_clear`] (the
    /// host's Clear button). The host then wipes its own
    /// per-wallet persistence (e.g. SwiftData rows) — Rust can't
    /// reach that layer — and the next `bind_shielded` call
    /// repopulates the registries and resyncs from scratch.
    ///
    /// Resets the cooldown to `None` so the first post-clear
    /// background sync pass runs immediately rather than honoring
    /// a stale "caught up" stamp from before the wipe.
    ///
    /// [`platform_wallet_manager_shielded_clear`]:
    ///     rs-platform-wallet-ffi's FFI entry point
    ///
    /// Returns an error if either store reset (subwallet purge or
    /// commitment-tree reset) fails. The caller **must** surface this:
    /// the host only wipes its own per-wallet persistence (e.g.
    /// SwiftData rows) after `clear()` succeeds. If a reset fails
    /// silently the host could drop its rows while the shared tree
    /// stays populated, and the next cold resync would gate-skip every
    /// re-downloaded position against the stale `tree_size`.
    pub async fn clear(&self) -> Result<(), crate::error::PlatformWalletError> {
        // Reset the persistent store FIRST and bail before mutating any
        // in-memory state if it fails. Clearing `accounts` / `persisters`
        // makes the coordinator forget every bound wallet (no syncs until
        // the host rebinds), so doing that while the store reset failed —
        // and the host therefore keeps its own local state — would leave
        // the two halves inconsistent. Both resets are still attempted
        // even if the first fails, so the store is left as clean as
        // possible, but the first error is captured and propagated.
        let mut first_err: Option<crate::error::PlatformWalletError> = None;
        {
            let mut store = self.store.write().await;
            if let Err(e) = store.purge_all_subwallets() {
                tracing::warn!(error = %e, "Failed to purge subwallet store state on clear");
                first_err.get_or_insert_with(|| {
                    crate::error::PlatformWalletError::ShieldedStoreError(format!(
                        "purge_all_subwallets failed: {e}"
                    ))
                });
            }
            // Reset the shared commitment tree under the same write
            // guard so the watermark (now 0) and the tree size reset
            // together — otherwise the post-clear resync gate-skips
            // every re-downloaded position into the still-full tree.
            if let Err(e) = store.reset_commitment_tree() {
                tracing::warn!(error = %e, "Failed to reset commitment tree on clear");
                first_err.get_or_insert_with(|| {
                    crate::error::PlatformWalletError::ShieldedStoreError(format!(
                        "reset_commitment_tree failed: {e}"
                    ))
                });
            }
        }
        if let Some(e) = first_err {
            return Err(e);
        }

        // Store reset succeeded — now it is safe to drop the in-memory
        // registries and reset the cooldown so the first post-clear
        // background pass runs immediately rather than honoring a stale
        // "caught up" stamp. On the failure path above none of this runs,
        // so a failed clear leaves coordinator state untouched.
        self.accounts.write().await.clear();
        self.persisters.write().await.clear();
        if let Ok(mut g) = self.last_caught_up_at.lock() {
            *g = None;
        }
        Ok(())
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
    /// Phase-2b shape: the union of every registered subwallet's
    /// [`AccountViewingKeys`] drives a **single** SDK fetch via
    /// [`sync_notes_across`]; the SDK's `all_notes` is locally
    /// trial-decrypted against every other subwallet's IVK in
    /// the same pass. The consolidated changeset is then split
    /// per-[`WalletId`] and queued through each registered
    /// [`WalletPersister`].
    ///
    /// [`sync_notes_across`]: super::sync::sync_notes_across
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

        // Snapshot the flat subwallet registry. This Vec is both
        // the IVK fan-out for sync_notes_across and the
        // identity-map for per-wallet summary demux below.
        let subwallets: Vec<(SubwalletId, AccountViewingKeys)> = {
            let accounts = self.accounts.read().await;
            accounts.iter().map(|(id, v)| (*id, v.clone())).collect()
        };

        let mut summary = ShieldedSyncPassSummary::default();
        if subwallets.is_empty() {
            summary.sync_unix_seconds = Self::now_unix();
            return summary;
        }

        // Snapshot armed reservations and prefetch Platform's recorded
        // anchor set BEFORE the note scan. The release pass after the scan
        // may only act on this pre-scan pair: an anchor already absent from
        // a PRE-scan set was pruned before the scan began, so a spend that
        // did execute did so at a height the scan covers — the scan's
        // spent-note reconcile clears its reservation, which the release
        // pass honors via `clear_pending`'s return value. A set fetched
        // AFTER the scan would leave a window (spend executes past the
        // scan's coverage, anchor pruned before the fetch) where a landed
        // spend could be wrongly released. Reservations armed DURING the
        // scan are deliberately absent from the snapshot — their anchors
        // may be newer than this set — and get checked next pass. Skips the
        // network call entirely when nothing is armed (the common case).
        let stranded_release = self.prefetch_stranded_release(&subwallets).await;

        // ONE SDK call covers every registered IVK on the network.
        // Snapshot the optional progress handler installed by the
        // manager; sync_notes_across feeds it into the SDK's chunk
        // loop so callers see live (cumulative_scanned, block_height)
        // updates during long cold syncs instead of one delayed
        // burst at the end.
        let on_progress = self.progress_handler();
        // Second, distinct signal: commitments committed to the local
        // tree as the interleaved consumer drains the SDK stream.
        let on_tree_progress = self.tree_progress_handler();
        let notes = match super::sync::sync_notes_across(
            &self.sdk,
            &self.store,
            &subwallets,
            on_progress.as_ref(),
            on_tree_progress.as_ref(),
        )
        .await
        {
            Ok(r) => r,
            Err(e) => return self.fail_all_wallets(&subwallets, &e),
        };
        // Scan-based spend detection now happens INSIDE
        // `sync_notes_across`: every scanned action's nullifier is
        // replayed against each subwallet's store as part of the note
        // scan (no separate nullifier-sync round-trip). The per-subwallet
        // newly-spent counts and the spend records ride the same
        // `notes` result and `notes.changeset` the receipts do.
        let newly_spent_per_sub = notes.per_subwallet_newly_spent.clone();

        // Residual-spend resolution: `sync_notes_across` above marked every
        // landed spend (clearing its reservation and dropping its redrive
        // record via the store hook). Two passes over what's left, both
        // judged against the PRE-scan recorded-anchor set:
        //
        // 1. Re-drive — for each armed unconfirmed spend whose anchor is
        //    still recorded, re-broadcast the stored byte-identical
        //    transition (bounded by MAX_REDRIVE_ATTEMPTS) to actively
        //    resolve the ambiguity instead of waiting out the retention
        //    window.
        // 2. Prune backstop — release any still-pending pre-scan
        //    reservation whose anchor was already pruned (the spend can
        //    never execute).
        //
        // Runs before the balance read so freed notes are reflected in
        // this pass's balances.
        if let Some((snapshot, recorded)) = stranded_release {
            // Snapshot the per-wallet persisters BEFORE the loop and drop
            // the read guard: `redrive_pending_spends` performs network
            // broadcasts, and holding the persisters lock across those
            // awaits would block wallet register/unregister for the
            // duration of the round trips.
            let subwallet_persisters: Vec<(SubwalletId, Option<WalletPersister>)> = {
                let persisters = self.persisters.read().await;
                subwallets
                    .iter()
                    .map(|(id, _)| (*id, persisters.get(&id.wallet_id).cloned()))
                    .collect()
            };
            for (id, persister) in &subwallet_persisters {
                super::operations::redrive_pending_spends(
                    &self.sdk,
                    &self.store,
                    persister.as_ref(),
                    id.wallet_id,
                    *id,
                    &recorded,
                )
                .await;
            }
            self.release_stranded_spends(snapshot, &recorded).await;
        }

        let balances_per_sub = match super::sync::balances_across(&self.store, &subwallets).await {
            Ok(r) => r,
            Err(e) => return self.fail_all_wallets(&subwallets, &e),
        };

        // Restore-path activity derivation: reconstruct per-operation
        // activity entries best-effort from the notes / outgoing notes
        // this pass (and earlier passes) persisted. Runs every pass so it
        // doubles as the one-time backfill over an already-populated
        // store — `derive_activity_from_scan_data` skips clusters whose
        // id a live entry already owns, and `save_activity` upserts by
        // id, so re-running is idempotent. Failures are logged and
        // swallowed: a derivation miss must never fail a sync pass.
        let mut notes = notes;
        if let Err(e) = self
            .derive_activity_into_changeset(&subwallets, &mut notes.changeset)
            .await
        {
            tracing::warn!(error = %e, "Shielded activity derivation failed; skipping this pass");
        }

        // The note-side changeset already carries saves, synced
        // indices, AND the scan-detected spends, so split it per
        // WalletId directly — each per-wallet `WalletPersister.store`
        // only sees its own wallet's deltas.
        let consolidated = notes.changeset.clone();
        if !crate::changeset::merge::Merge::is_empty(&consolidated) {
            let per_wallet = consolidated.split_by_wallet_id();
            let persisters = self.persisters.read().await;
            for (wallet_id, cs) in per_wallet {
                let Some(persister) = persisters.get(&wallet_id) else {
                    tracing::warn!(
                        wallet_id = %hex::encode(wallet_id),
                        "Shielded sync changeset dropped: no persister registered (host inconsistency)"
                    );
                    continue;
                };
                let full = crate::changeset::PlatformWalletChangeSet {
                    shielded: Some(cs),
                    ..Default::default()
                };
                if let Err(e) = persister.store(full) {
                    tracing::warn!(
                        wallet_id = %hex::encode(wallet_id),
                        error = %e,
                        "Failed to queue shielded changeset from coordinator"
                    );
                }
            }
        }

        // Cooldown decision based on aggregate activity across
        // every subwallet — any new commitment scanned or
        // newly-spent nullifier anywhere on the network clears
        // the stamp so the next pass runs immediately.
        let any_activity = notes.total_scanned > 0 || newly_spent_per_sub.values().any(|&n| n > 0);
        if let Ok(mut guard) = self.last_caught_up_at.lock() {
            if any_activity {
                *guard = None;
            } else {
                *guard = Some(Instant::now());
            }
        }

        // Demux multi-subwallet results into the per-wallet
        // `ShieldedSyncSummary` shape that
        // `PlatformEventManager::on_shielded_sync_completed`
        // already speaks. Same emission shape as Phase 2a.
        summary =
            build_per_wallet_summary(&subwallets, &notes, &newly_spent_per_sub, &balances_per_sub);
        summary.sync_unix_seconds = Self::now_unix();
        summary
    }

    /// Build a `ShieldedSyncPassSummary` where every registered
    /// wallet's outcome is the supplied error string. Used when
    /// the network-wide SDK note scan (sync_notes_across) errors
    /// before any per-wallet result can be produced.
    fn fail_all_wallets(
        &self,
        subwallets: &[(SubwalletId, AccountViewingKeys)],
        e: &crate::error::PlatformWalletError,
    ) -> ShieldedSyncPassSummary {
        let mut wallet_ids: Vec<WalletId> = subwallets.iter().map(|(id, _)| id.wallet_id).collect();
        wallet_ids.sort_unstable();
        wallet_ids.dedup();
        let mut summary = ShieldedSyncPassSummary::default();
        for wallet_id in wallet_ids {
            tracing::warn!(
                wallet_id = %hex::encode(wallet_id),
                error = %e,
                "Network-wide shielded sync failed"
            );
            summary
                .wallet_results
                .insert(wallet_id, WalletShieldedOutcome::Err(e.to_string()));
        }
        summary.sync_unix_seconds = Self::now_unix();
        summary
    }

    fn now_unix() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    /// Pre-scan half of the stranded-reservation release: snapshot the
    /// anchored reservations armed right now and fetch Platform's recorded
    /// anchor set, or `None` when there is nothing to do.
    ///
    /// MUST run before the pass's note scan — the release's fund-safety
    /// argument ([`Self::release_stranded_spends`]) rests on both the
    /// snapshot and the set predating the scan's spent-note reconcile.
    ///
    /// Skips the network round-trip when no anchored reservation exists (the
    /// common case), and treats a recorded-anchor fetch failure as
    /// non-fatal — a sync must not fail because that query hiccupped; the
    /// release simply waits for the next pass.
    async fn prefetch_stranded_release(
        &self,
        subwallets: &[(SubwalletId, AccountViewingKeys)],
    ) -> Option<(Vec<(SubwalletId, StalePendingSpend)>, HashSet<[u8; 32]>)> {
        // Gather anchored reservations across every synced subwallet. The
        // common case is none — then the network round-trip is skipped.
        let stale: Vec<(SubwalletId, StalePendingSpend)> = {
            let store = self.store.read().await;
            let mut acc = Vec::new();
            for (id, _) in subwallets {
                match store.stale_pending_spends(*id) {
                    Ok(entries) => acc.extend(entries.into_iter().map(|entry| (*id, entry))),
                    Err(e) => tracing::warn!(
                        wallet_id = %hex::encode(id.wallet_id),
                        account = id.account_index,
                        error = %e,
                        "shielded reservation release: stale_pending_spends failed; skipping subwallet"
                    ),
                }
            }
            acc
        };
        if stale.is_empty() {
            return None;
        }

        use dash_sdk::platform::fetch_current_no_parameters::FetchCurrent;
        match dash_sdk::query_types::ShieldedAnchors::fetch_current(&self.sdk).await {
            Ok(dash_sdk::query_types::ShieldedAnchors(anchors)) => {
                Some((stale, anchors.into_iter().collect()))
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "shielded reservation release: failed to fetch the recorded anchor set; \
                     skipping this pass"
                );
                None
            }
        }
    }

    /// Release any stranded shielded-spend reservation whose recorded
    /// anchor Platform has pruned.
    ///
    /// A spend that returns broadcast-accepted-but-unconfirmed keeps its
    /// note reservation (so a retry can't double-spend a note that may in
    /// fact have landed), released only when the tx lands or the app
    /// restarts. A spend accepted but that never lands would otherwise
    /// strand its notes for the whole session. Here, after the pass's
    /// spent-note reconcile, any reservation from the PRE-scan `snapshot`
    /// whose anchor is absent from the PRE-scan `recorded` set is provably
    /// dead: `validate_anchor_exists` accepts a spend only while its anchor
    /// is retained (`shielded_anchor_retention_blocks = 1000`), so once the
    /// anchor is pruned the transition can never execute. The notes are
    /// freed and the linked activity row flipped to Failed so the UI shows
    /// a clear, retryable failure instead of "Pending" forever. The
    /// on-chain nullifier set stays the authoritative double-spend guard.
    ///
    /// Fund-safe by construction, resting on two ordering facts:
    ///
    /// 1. Both inputs predate the scan ([`Self::prefetch_stranded_release`]),
    ///    and the scan's spent-note reconcile ran to completion in between.
    ///    An anchor absent from the pre-scan set was pruned before the scan
    ///    began (a pruned root can never re-enter the set — the tree only
    ///    grows), so if that spend nevertheless executed, it executed at a
    ///    height the scan covered — the reconcile already cleared its
    ///    reservation, which fact 2 catches.
    /// 2. Each release is a check-and-clear under a single store write
    ///    guard: the reservation is cleared only while the nullifier is
    ///    still armed with the snapshot's exact anchor + activity. A
    ///    reservation resolved concurrently (the reconcile above, or the
    ///    spend's own result-wait confirming mid-pass) — or cleared and
    ///    re-armed by a retry with a fresh anchor — is skipped, and its
    ///    activity row is left alone.
    ///
    /// A still-recorded (slow-but-landing) spend is never released, and an
    /// anchor-less reservation (reserved but not yet built) is never in the
    /// snapshot (`stale_pending_spends` returns only anchored entries).
    async fn release_stranded_spends(
        &self,
        snapshot: Vec<(SubwalletId, StalePendingSpend)>,
        recorded: &HashSet<[u8; 32]>,
    ) {
        // Persister map is only written on wallet register/unregister —
        // one read acquisition covers the whole loop.
        let persisters = self.persisters.read().await;
        // Every note of a multi-note spend carries the same activity id:
        // flip each linked row once, not once per nullifier.
        let mut flipped: HashSet<[u8; 32]> = HashSet::new();
        for (id, (nullifier, anchor, activity_id)) in snapshot {
            // Fund-safety invariant: release ONLY when the anchor is absent
            // from the recorded set. A still-recorded anchor may yet land.
            if recorded.contains(&anchor) {
                continue;
            }
            // Check-and-clear under ONE write acquisition: the snapshot
            // entry is released only while the nullifier is still armed
            // with the SAME anchor + activity. A reservation cleared
            // mid-scan and re-armed by a retry (same note, fresh anchor
            // and activity) must not be judged against the pre-scan set —
            // its anchor may postdate the fetch; it gets checked next pass.
            {
                let mut store = self.store.write().await;
                let still_same = match store.stale_pending_spends(id) {
                    Ok(current) => current
                        .iter()
                        .any(|(n, a, act)| *n == nullifier && *a == anchor && *act == activity_id),
                    Err(e) => {
                        tracing::warn!(
                            wallet_id = %hex::encode(id.wallet_id),
                            account = id.account_index,
                            error = %e,
                            "shielded reservation release: stale_pending_spends failed; skipping entry"
                        );
                        continue;
                    }
                };
                if !still_same {
                    // Resolved (landed and reconciled, result-wait
                    // confirmed) or re-armed by a retry while this pass
                    // was scanning. Nothing to release — leave the
                    // activity row alone.
                    tracing::debug!(
                        wallet_id = %hex::encode(id.wallet_id),
                        account = id.account_index,
                        nullifier = %hex::encode(nullifier),
                        "shielded reservation release: reservation resolved or re-armed; skipping"
                    );
                    continue;
                }
                match store.clear_pending(id, &nullifier) {
                    // `still_same` held under this same guard, so the clear
                    // removes exactly the snapshot's reservation.
                    Ok(true) => {}
                    Ok(false) => continue,
                    Err(e) => {
                        tracing::warn!(
                            wallet_id = %hex::encode(id.wallet_id),
                            account = id.account_index,
                            error = %e,
                            "shielded reservation release: clear_pending failed"
                        );
                        continue;
                    }
                }
            }
            tracing::info!(
                wallet_id = %hex::encode(id.wallet_id),
                account = id.account_index,
                nullifier = %hex::encode(nullifier),
                anchor = %hex::encode(anchor),
                "shielded reservation released: its recorded anchor was pruned, so the stranded \
                 spend can never execute; freeing the notes"
            );
            // Flip the linked activity row to Failed. Only queue to the
            // wallet's own persister (cloned out — `WalletPersister: Clone`).
            if let Some(entry_id) = activity_id {
                if !flipped.insert(entry_id) {
                    continue;
                }
                let persister = persisters.get(&id.wallet_id).cloned();
                super::operations::record_activity_status_by_id(
                    &self.store,
                    persister.as_ref(),
                    id.wallet_id,
                    id,
                    &entry_id,
                    super::activity::ShieldedActivityStatus::Failed,
                )
                .await;
            }
        }
    }

    /// Derive best-effort activity entries from each subwallet's
    /// persisted scan data and add the new ones to `changeset`.
    ///
    /// For each subwallet: read all notes + OVK-recovered outgoing notes
    /// from the store, classify the recipient of each outgoing note as
    /// own-vs-external by testing it against the subwallet's IVK (the
    /// `diversifier_index` check — Orchard addresses are diversified, so
    /// a fixed address list can't be used), build the
    /// [`super::activity::ScanDeriveInput`], and run
    /// [`super::activity::derive_activity_from_scan_data`] against the
    /// entry ids already on file (live entries win). Newly derived
    /// entries are saved to the store and recorded on `changeset` so they
    /// reach the host persister on this pass's flush.
    ///
    /// All client-side (Option B): no node / DAPI query, only data the
    /// store already holds.
    async fn derive_activity_into_changeset(
        &self,
        subwallets: &[(SubwalletId, AccountViewingKeys)],
        changeset: &mut crate::changeset::ShieldedChangeSet,
    ) -> Result<(), crate::error::PlatformWalletError> {
        use super::activity::{derive_activity_from_scan_data, ScanDeriveInput};

        for (id, views) in subwallets {
            // Read pass: snapshot the inputs under a shared lock, then
            // release it before the CPU-bound classification — holding
            // the WRITE lock across the whole derivation would serialize
            // every other store consumer for the full window. Anything a
            // live recorder lands between the two passes is caught by the
            // overlap re-check under the write lock below.
            let (input, existing_cmxs) = {
                let store = self.store.read().await;
                let notes = store.get_all_notes(*id).map_err(|e| {
                    crate::error::PlatformWalletError::ShieldedStoreError(e.to_string())
                })?;
                let outgoing = store.get_outgoing_notes(*id).map_err(|e| {
                    crate::error::PlatformWalletError::ShieldedStoreError(e.to_string())
                })?;
                if notes.is_empty() && outgoing.is_empty() {
                    continue;
                }

                // Own-recipient set: an outgoing note whose recipient the
                // subwallet's IVK recognizes is self-change, not a payment
                // out.
                let own_addresses: Vec<Vec<u8>> = outgoing
                    .iter()
                    .filter(|o| is_own_orchard_recipient(views, &o.recipient))
                    .map(|o| o.recipient.clone())
                    .collect();

                // Map every stored entry's visible output cmx to the owning
                // entry id, so the deriver can dedupe by cmx OVERLAP (not
                // exact id): a same-block cluster that merges two live ops
                // hashes to an id matching neither, but its cmxs still
                // overlap both.
                let existing_cmxs: BTreeMap<[u8; 32], [u8; 32]> = store
                    .get_activity(*id, 0, usize::MAX)
                    .map_err(|e| {
                        crate::error::PlatformWalletError::ShieldedStoreError(e.to_string())
                    })?
                    .into_iter()
                    .flat_map(|entry| entry.note_cmxs.into_iter().map(move |c| (c, entry.id)))
                    .collect();

                (
                    ScanDeriveInput {
                        notes,
                        outgoing,
                        own_addresses,
                    },
                    existing_cmxs,
                )
            };

            // Lock-free classification.
            let derived = derive_activity_from_scan_data(&input, &existing_cmxs);
            if derived.new_entries.is_empty() && derived.confirmations.is_empty() {
                continue;
            }

            // Write pass: only the upserts hold the write lock.
            let mut store = self.store.write().await;
            // Re-check cmx overlap against the CURRENT activity rows
            // before inserting: a live recorder may have written a richer
            // row (kind / fee / memo / created identity id) for the same
            // cmx set between the read snapshot and here. Saving the
            // scan-derived entry anyway would either clobber that row
            // (id collision) or duplicate it (id mismatch), and scan-only
            // data can never reconstruct the lost live fields. Overlapped
            // clusters degrade to confirmation sightings instead — same
            // treatment the classifier gives overlaps it can see.
            let current_cmxs: BTreeMap<[u8; 32], [u8; 32]> = store
                .get_activity(*id, 0, usize::MAX)
                .map_err(|e| crate::error::PlatformWalletError::ShieldedStoreError(e.to_string()))?
                .into_iter()
                .flat_map(|entry| entry.note_cmxs.into_iter().map(move |c| (c, entry.id)))
                .collect();
            let mut confirmations = derived.confirmations;
            for entry in derived.new_entries {
                let overlapped: std::collections::BTreeSet<[u8; 32]> = entry
                    .note_cmxs
                    .iter()
                    .filter_map(|c| current_cmxs.get(c))
                    .copied()
                    .collect();
                if !overlapped.is_empty() {
                    if let Some(height) = entry.block_height {
                        confirmations.extend(overlapped.into_iter().map(|eid| (eid, height)));
                    }
                    continue;
                }
                store.save_activity(*id, &entry).map_err(|e| {
                    crate::error::PlatformWalletError::ShieldedStoreError(e.to_string())
                })?;
                changeset.record_activity_entry(*id, entry);
            }
            // On-chain sightings of clusters that already have a row:
            // upgrade still-`Pending` (or height-less) rows to Confirmed
            // at the observed height. This is the flip the ambiguous
            // post-broadcast paths (`ShieldedSpendUnconfirmed` /
            // `ShieldedBroadcastUnconfirmed`) leave to the scan. The
            // upgrade rewrites the STORED entry via `with_status`, so the
            // live entry's richer fields (kind / fee / memo /
            // counterparty) survive untouched.
            for (entry_id, height) in confirmations {
                let stored = store
                    .get_activity_by_entry_id(*id, &entry_id)
                    .map_err(|e| {
                        crate::error::PlatformWalletError::ShieldedStoreError(e.to_string())
                    })?;
                let Some(stored) = stored else { continue };
                // Chain truth wins: a row marked Failed by a client-side
                // post-broadcast error whose outputs are later observed
                // on-chain was not actually a failure — upgrade it. (We
                // observed exactly this on devnet: the rc.1 result-proof
                // fetch failure marked an actually-landed identity-create
                // as failed; the cluster's cmxs appearing on-chain is
                // ground truth that the operation executed.) The gate is
                // therefore `Pending || block_height.is_none()`, which
                // also catches those Failed-no-height rows; only a
                // Confirmed-with-height row is final.
                let needs_upgrade = stored.status
                    == super::activity::ShieldedActivityStatus::Pending
                    || stored.block_height.is_none();
                if !needs_upgrade {
                    continue;
                }
                let upgraded = super::activity_recorder::with_status(
                    &stored,
                    super::activity::ShieldedActivityStatus::Confirmed,
                    Some(height),
                );
                store.save_activity(*id, &upgraded).map_err(|e| {
                    crate::error::PlatformWalletError::ShieldedStoreError(e.to_string())
                })?;
                changeset.record_activity_entry(*id, upgraded);
            }
        }
        Ok(())
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

/// Demux the multi-subwallet sync result into the per-wallet
/// [`ShieldedSyncSummary`] shape that
/// [`PlatformEventManager::on_shielded_sync_completed`] already
/// speaks. The coordinator drives sync at SubwalletId-flat
/// granularity but the host event stream is still per-wallet,
/// so this helper folds per-(wallet_id) slices back into the
/// `BTreeMap<u32, _>` per-account shape consumers expect.
///
/// [`PlatformEventManager::on_shielded_sync_completed`]:
///     crate::events::PlatformEventManager::on_shielded_sync_completed
fn build_per_wallet_summary(
    subwallets: &[(SubwalletId, AccountViewingKeys)],
    notes: &super::sync::MultiSyncNotesResult,
    newly_spent_per_sub: &BTreeMap<SubwalletId, usize>,
    balances_per_sub: &BTreeMap<SubwalletId, u64>,
) -> ShieldedSyncPassSummary {
    use super::sync::{ShieldedSyncSummary, SyncNotesResult};

    // Enumerate distinct wallet_ids in ascending order so the
    // BTreeMap iteration in the consumer is deterministic.
    let mut wallet_ids: Vec<WalletId> = subwallets.iter().map(|(id, _)| id.wallet_id).collect();
    wallet_ids.sort_unstable();
    wallet_ids.dedup();

    let mut summary = ShieldedSyncPassSummary::default();
    for wallet_id in wallet_ids {
        let new_notes_per_account: BTreeMap<u32, usize> = notes
            .per_subwallet_new_notes
            .iter()
            .filter(|(id, _)| id.wallet_id == wallet_id)
            .map(|(id, &c)| (id.account_index, c))
            .collect();
        let newly_spent_per_account: BTreeMap<u32, usize> = newly_spent_per_sub
            .iter()
            .filter(|(id, _)| id.wallet_id == wallet_id)
            .map(|(id, &c)| (id.account_index, c))
            .collect();
        let balances: BTreeMap<u32, u64> = balances_per_sub
            .iter()
            .filter(|(id, _)| id.wallet_id == wallet_id)
            .map(|(id, &v)| (id.account_index, v))
            .collect();

        summary.wallet_results.insert(
            wallet_id,
            WalletShieldedOutcome::Ok(ShieldedSyncSummary {
                notes_result: SyncNotesResult {
                    new_notes_per_account,
                    // `total_scanned` is a network property —
                    // every wallet sees the same number of new
                    // positions in the same pass. Surface it on
                    // each per-wallet summary so the host UI can
                    // display it without having to look up the
                    // pass-level value.
                    total_scanned: notes.total_scanned,
                },
                newly_spent_per_account,
                balances,
                is_cooldown_skip: false,
            }),
        );
    }
    summary
}

/// Whether a 43-byte raw Orchard `recipient` belongs to the subwallet's
/// own viewing keys — i.e. it's a self-change output, not a payment to
/// someone else. Orchard addresses are diversified, so this can't be a
/// fixed-address comparison; it tests the recipient against the IVK via
/// `diversifier_index` (mirrors the `sender_ovk` account-selection check
/// in `fund_from_asset_lock.rs`). A malformed recipient (wrong length /
/// off-curve) returns `false` (treated as external) so a corrupt row
/// can't silently mask a real send.
fn is_own_orchard_recipient(views: &AccountViewingKeys, recipient: &[u8]) -> bool {
    let raw: [u8; 43] = match recipient.try_into() {
        Ok(b) => b,
        Err(_) => return false,
    };
    let Some(addr) = Option::<grovedb_commitment_tree::PaymentAddress>::from(
        grovedb_commitment_tree::PaymentAddress::from_raw_address_bytes(&raw),
    ) else {
        return false;
    };
    views
        .incoming_viewing_key
        .diversifier_index(&addr)
        .is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wallet::persister::NoPlatformPersistence;
    use crate::wallet::shielded::keys::OrchardKeySet;

    /// Unique temp directory for a test's SQLite tree (no `tempfile` dev-dep).
    fn temp_dir(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("shielded_coordinator_test_{tag}_{nanos}"))
    }

    /// Build a coordinator backed by a fresh file store under `dir`, with one
    /// wallet registered so `accounts` / `persisters` are both non-empty.
    async fn coordinator_with_one_wallet(dir: &std::path::Path) -> NetworkShieldedCoordinator {
        std::fs::create_dir_all(dir).expect("create temp dir");
        let db_path = dir.join("tree.sqlite");
        let store = FileBackedShieldedStore::open_path(&db_path, 100).expect("open file store");
        let coordinator = NetworkShieldedCoordinator::new(
            Arc::new(dash_sdk::Sdk::new_mock()),
            dashcore::Network::Testnet,
            db_path,
            store,
        );

        let wallet_id: WalletId = [0x11; 32];
        let views = OrchardKeySet::from_seed(&[0x42u8; 64], dashcore::Network::Testnet, 0)
            .expect("derive viewing keys")
            .viewing_keys();
        let mut account_views = BTreeMap::new();
        account_views.insert(0u32, views);
        let persister = WalletPersister::new(wallet_id, Arc::new(NoPlatformPersistence));
        coordinator
            .register_wallet(wallet_id, account_views, persister)
            .await;
        coordinator
    }

    /// Success path: a healthy `clear()` empties the shared commitment tree
    /// AND drops the in-memory account / persister registries, returning `Ok`.
    #[tokio::test]
    async fn clear_success_empties_tree_and_registries() {
        let dir = temp_dir("clear_ok");
        let coordinator = coordinator_with_one_wallet(&dir).await;

        // Put a leaf in the tree so the reset is observable.
        {
            let mut store = coordinator.store().write().await;
            store.append_commitment(&[7u8; 32], true).unwrap();
            assert_eq!(store.tree_size().unwrap(), 1);
        }
        assert!(!coordinator.accounts.read().await.is_empty());
        assert!(!coordinator.persisters.read().await.is_empty());

        coordinator.clear().await.expect("clear should succeed");

        assert_eq!(
            coordinator.store().read().await.tree_size().unwrap(),
            0,
            "tree must be empty after a successful clear"
        );
        assert!(coordinator.accounts.read().await.is_empty());
        assert!(coordinator.persisters.read().await.is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Failure path — regression guard for the partial-clear bug: when the
    /// store reset fails, `clear()` must return `Err` and leave the account /
    /// persister registries populated, so the coordinator does not silently
    /// forget every bound wallet (turning future syncs into no-ops) while the
    /// host is told to keep its own persisted state.
    ///
    /// Unix-only: the failure injection relies on POSIX unlink-while-open
    /// semantics — removing the directory orphans the inode the store's open
    /// SQLite handle keeps using, but a fresh `Connection::open` at the now
    /// missing path fails, which is what drives `reset_commitment_tree` to
    /// error. Windows refuses to remove a directory with open files, so the
    /// injection wouldn't model a reset failure there.
    #[cfg(unix)]
    #[tokio::test]
    async fn clear_failure_preserves_registries() {
        let dir = temp_dir("clear_err");
        let coordinator = coordinator_with_one_wallet(&dir).await;
        assert!(!coordinator.accounts.read().await.is_empty());
        assert!(!coordinator.persisters.read().await.is_empty());

        // Force `reset_commitment_tree` to fail: remove the directory holding
        // the SQLite file so the reset's `Connection::open(path)` can't reopen
        // it. The store's already-open handle keeps working on the unlinked
        // inode, but a fresh open at the now-missing path errors.
        std::fs::remove_dir_all(&dir).expect("remove temp dir to break reopen");

        let result = coordinator.clear().await;
        assert!(
            result.is_err(),
            "clear must surface the store-reset failure rather than swallow it"
        );
        assert!(
            !coordinator.accounts.read().await.is_empty(),
            "accounts must survive a failed clear so sync does not become a no-op"
        );
        assert!(
            !coordinator.persisters.read().await.is_empty(),
            "persisters must survive a failed clear"
        );
    }

    /// Common case: with no anchored reservation in the store, the pre-scan
    /// prefetch short-circuits before any network round-trip — it never
    /// touches the mock SDK (which has no anchor-query expectation), so this
    /// both pins the fast-path skip and proves the pass is wired without
    /// panic (sync() skips the release entirely on `None`).
    #[tokio::test]
    async fn release_stranded_spends_no_op_without_anchored_reservation() {
        let dir = temp_dir("release_noop");
        let coordinator = coordinator_with_one_wallet(&dir).await;

        let subwallets = vec![(
            SubwalletId::new([0x11; 32], 0),
            OrchardKeySet::from_seed(&[0x42u8; 64], dashcore::Network::Testnet, 0)
                .expect("derive viewing keys")
                .viewing_keys(),
        )];

        // No reservation was armed, so `stale_pending_spends` is empty and
        // the prefetch returns None before fetching the recorded anchor set.
        let prefetched = coordinator.prefetch_stranded_release(&subwallets).await;
        assert!(
            prefetched.is_none(),
            "nothing armed ⇒ no anchor fetch, no release pass"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Full release-path coverage through `release_stranded_spends` itself
    /// (not just the store predicate): of two armed reservations, the one
    /// whose anchor is absent from the recorded set is released while the
    /// one whose anchor is still recorded survives untouched.
    #[tokio::test]
    async fn release_stranded_spends_releases_pruned_and_retains_recorded() {
        let dir = temp_dir("release_paths");
        let coordinator = coordinator_with_one_wallet(&dir).await;
        let id = SubwalletId::new([0x11; 32], 0);

        let pruned_nf = [0xAAu8; 32];
        let pruned_anchor = [0xABu8; 32];
        let pruned_act = [0xACu8; 32];
        let live_nf = [0xBAu8; 32];
        let live_anchor = [0xBBu8; 32];
        let live_act = [0xBCu8; 32];
        {
            let mut store = coordinator.store.write().await;
            store.mark_pending(id, &pruned_nf).expect("mark pruned");
            store
                .set_pending_spend(id, &pruned_nf, pruned_anchor, pruned_act)
                .expect("arm pruned");
            store.mark_pending(id, &live_nf).expect("mark live");
            store
                .set_pending_spend(id, &live_nf, live_anchor, live_act)
                .expect("arm live");
        }

        // Snapshot as the pre-scan prefetch would have produced it; the
        // recorded set holds only the live anchor (the other was "pruned").
        let snapshot = vec![
            (id, (pruned_nf, pruned_anchor, Some(pruned_act))),
            (id, (live_nf, live_anchor, Some(live_act))),
        ];
        let recorded: HashSet<[u8; 32]> = [live_anchor].into_iter().collect();

        coordinator
            .release_stranded_spends(snapshot, &recorded)
            .await;

        let remaining = coordinator
            .store
            .read()
            .await
            .stale_pending_spends(id)
            .expect("stale_pending_spends");
        assert_eq!(
            remaining.len(),
            1,
            "pruned-anchor reservation released; recorded-anchor one retained"
        );
        assert_eq!(
            remaining[0].0, live_nf,
            "the still-recorded reservation must survive"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
