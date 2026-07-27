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

    /// Serializes every shielded lifecycle TRANSACTION on this
    /// coordinator: a wallet's whole bind install (see
    /// [`begin_install`](Self::begin_install)),
    /// [`unregister_wallet`](Self::unregister_wallet), and
    /// [`clear`](Self::clear).
    ///
    /// Two properties depend on it.
    ///
    /// *Registry atomicity.* `accounts` / `persisters` sit behind
    /// separate `RwLock`s (readers like `sync()` take them
    /// independently), so without this outer mutex a concurrent
    /// register + unregister could interleave as *insert persister →
    /// remove accounts → insert accounts → remove persister*, leaving
    /// visible accounts with no persister — a state in which `sync()`
    /// silently drops that wallet's changeset. With the mutex, the
    /// only accounts-without-persister window a `sync()` pass can
    /// observe is a wallet being genuinely removed mid-pass, where
    /// dropping its changeset is the intended outcome.
    ///
    /// *Install atomicity.* A bind is not one map write but a
    /// sequence — compare the incoming registration, replace it,
    /// restore the host snapshot, commit the hydration flag — and each
    /// step reads state the previous one established. Serializing only
    /// the individual steps would let two binds of the same wallet
    /// commit their registrations in the opposite order to their
    /// key-slot writes (leaving the coordinator decrypting under one
    /// key while addresses and spends use another), and would let an
    /// `unregister_wallet` / `clear` purge state between a restore's
    /// registration check and its store write (repopulating a purged
    /// wallet and re-arming its hydration flag). Holding the mutex
    /// across the whole transaction is what makes "the registration a
    /// restore commits against is the one it checked" hold.
    ///
    /// Lock order is `lifecycle` → `store`; nothing takes them the
    /// other way round.
    lifecycle: tokio::sync::Mutex<()>,

    /// Wallets whose per-subwallet store state has been successfully
    /// hydrated from the host's persisted snapshot this session
    /// (`restore_for_wallet` completed without error after the
    /// wallet's current registration was installed). A matching
    /// registration alone does NOT imply hydration: the first bind
    /// registers before restoring, and a transient persister
    /// load/restore failure is logged rather than surfaced — without
    /// this flag a later re-bind would see matching keys, take the
    /// idempotent fast path, and silently skip the restore that could
    /// now succeed, leaving notes and the watermark absent until a
    /// full rescan or restart. Cleared on unregister / clear, and
    /// whenever a re-bind replaces the registration with a different
    /// account set.
    hydrated: RwLock<std::collections::BTreeSet<WalletId>>,
}

/// Exclusive handle on one wallet's shielded install transaction,
/// returned by
/// [`begin_install`](NetworkShieldedCoordinator::begin_install).
///
/// Holds the coordinator's lifecycle mutex for as long as it lives, so
/// every step taken through it — registration comparison, registration
/// replacement, snapshot restore, hydration commit — observes the same
/// registration, and no `register` / `unregister_wallet` / `clear` from
/// another task can slip between two of them. Callers that need only a
/// single step can use the equivalent method on the coordinator, which
/// wraps it in a one-step transaction.
///
/// Drop it as soon as the install commits: it blocks wallet removal and
/// Clear for the whole scope.
pub struct ShieldedInstall<'a> {
    coordinator: &'a NetworkShieldedCoordinator,
    wallet_id: WalletId,
    _lifecycle: tokio::sync::MutexGuard<'a, ()>,
}

impl ShieldedInstall<'_> {
    /// See
    /// [`wallet_registration_matches`](NetworkShieldedCoordinator::wallet_registration_matches).
    pub async fn registration_matches(
        &self,
        account_views: &BTreeMap<u32, AccountViewingKeys>,
    ) -> bool {
        self.coordinator
            .registration_matches_locked(self.wallet_id, account_views)
            .await
    }

    /// The first account of `account_views` that is already registered
    /// on this wallet under a DIFFERENT full viewing key, if any.
    ///
    /// Per-subwallet durable state (notes, activity, watermark) is keyed
    /// by `(wallet_id, account_index)` alone — nothing records which key
    /// decrypted it — so a re-key of a bound account cannot be applied
    /// coherently: the surviving durable rows belong to the old key
    /// while the registration claims the new one. Bind paths use this to
    /// refuse the change instead of silently mixing the two.
    pub async fn conflicting_account(
        &self,
        account_views: &BTreeMap<u32, AccountViewingKeys>,
    ) -> Option<u32> {
        let accounts = self.coordinator.accounts.read().await;
        account_views.iter().find_map(|(account, views)| {
            let id = SubwalletId::new(self.wallet_id, *account);
            accounts
                .get(&id)
                .filter(|registered| registered.to_fvk_bytes() != views.to_fvk_bytes())
                .map(|_| *account)
        })
    }

    /// See [`register_wallet`](NetworkShieldedCoordinator::register_wallet).
    pub async fn register(
        &self,
        account_views: BTreeMap<u32, AccountViewingKeys>,
        persister: WalletPersister,
    ) {
        self.coordinator
            .register_locked(self.wallet_id, account_views, persister)
            .await
    }

    /// See [`restore_for_wallet`](NetworkShieldedCoordinator::restore_for_wallet).
    pub async fn restore(
        &self,
        snapshot: &crate::changeset::ShieldedSyncStartState,
    ) -> Result<(), crate::error::PlatformWalletError> {
        self.coordinator
            .restore_locked(self.wallet_id, snapshot)
            .await
    }

    /// See [`is_hydrated`](NetworkShieldedCoordinator::is_hydrated).
    pub async fn is_hydrated(&self) -> bool {
        self.coordinator.is_hydrated_locked(self.wallet_id).await
    }

    /// See [`mark_hydrated`](NetworkShieldedCoordinator::mark_hydrated).
    pub async fn mark_hydrated(&self, hydrated: bool) {
        self.coordinator
            .mark_hydrated_locked(self.wallet_id, hydrated)
            .await
    }
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
            lifecycle: tokio::sync::Mutex::new(()),
            hydrated: RwLock::new(std::collections::BTreeSet::new()),
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
    /// clear is consistent). A re-bind never removes the persister
    /// (it only ever replaces it), so a sync pass finishing during a
    /// re-bind always finds a persister for its changeset. Store
    /// state is purged ONLY for subwallets the new registration
    /// drops or re-keys: an account that was registered before but
    /// is absent from `account_views`, or whose full viewing key
    /// changed (its stored notes were decrypted under — and belong
    /// to — the old key). Accounts that remain bound with the same
    /// key keep their in-memory notes and watermark across the
    /// re-bind, so a registration replacement racing an in-flight
    /// sync pass can no longer wipe the pass's results.
    ///
    /// [`ShieldedWallet`]: super::ShieldedWallet
    /// [`PlatformWallet::bind_shielded`]: crate::wallet::PlatformWallet::bind_shielded
    pub async fn register_wallet(
        &self,
        wallet_id: WalletId,
        account_views: BTreeMap<u32, AccountViewingKeys>,
        persister: WalletPersister,
    ) {
        self.begin_install(wallet_id)
            .await
            .register(account_views, persister)
            .await
    }

    /// Open an install transaction for `wallet_id`, taking the
    /// coordinator's lifecycle mutex for the returned guard's lifetime.
    ///
    /// A bind's steps — compare the incoming registration, replace it,
    /// restore the host snapshot, commit hydration — only compose into
    /// the intended "replace this wallet's shielded state" operation if
    /// nothing else mutates the registries in between; see the
    /// `lifecycle` field doc for the interleavings this forbids. Every
    /// single-step public method below opens (and immediately closes)
    /// one of these transactions, so a caller holding a guard must go
    /// through the guard rather than calling them — they would deadlock
    /// on the same non-reentrant mutex.
    pub async fn begin_install(&self, wallet_id: WalletId) -> ShieldedInstall<'_> {
        ShieldedInstall {
            coordinator: self,
            wallet_id,
            _lifecycle: self.lifecycle.lock().await,
        }
    }

    /// [`register_wallet`](Self::register_wallet)'s body, with the
    /// lifecycle mutex already held by the caller's transaction.
    async fn register_locked(
        &self,
        wallet_id: WalletId,
        account_views: BTreeMap<u32, AccountViewingKeys>,
        persister: WalletPersister,
    ) {
        // Subwallets the new registration drops or re-keys. Their
        // store state must go: a dropped account would otherwise
        // keep unspendable notes and a stale watermark alive, and a
        // re-keyed account's stored notes belong to the OLD viewing
        // key. Computed before the maps mutate.
        let stale: Vec<SubwalletId> = {
            let accounts = self.accounts.read().await;
            accounts
                .iter()
                .filter(|(id, _)| id.wallet_id == wallet_id)
                .filter(|(id, views)| {
                    account_views
                        .get(&id.account_index)
                        .map(|new_views| new_views.to_fvk_bytes() != views.to_fvk_bytes())
                        .unwrap_or(true)
                })
                .map(|(id, _)| *id)
                .collect()
        };
        // Whether the registration shape changed AT ALL — a dropped or
        // re-keyed subwallet (`stale`), or an ADDED account (present in
        // `account_views` but not registered before). An added account
        // needs its own hydration (the host may hold persisted notes /
        // a watermark for it from an earlier session), so any shape
        // change invalidates the wallet's hydrated flag even when
        // nothing is purged.
        let shape_changed = !stale.is_empty() || {
            let accounts = self.accounts.read().await;
            let registered_count = accounts
                .keys()
                .filter(|id| id.wallet_id == wallet_id)
                .count();
            registered_count != account_views.len()
        };

        // Persister FIRST, accounts second. `sync()` snapshots
        // `accounts` at pass start and looks the persister up only
        // when it queues the pass's changeset — with the reverse
        // order a pass starting between the two inserts would see
        // the subwallets but find no persister and silently drop
        // the whole changeset ("no persister registered", observed
        // on device as a scan pass whose discovered notes never
        // reached the host). Registering the persister before the
        // accounts makes "accounts visible ⇒ persister visible"
        // hold at every interleaving.
        self.persisters.write().await.insert(wallet_id, persister);
        {
            let mut accounts = self.accounts.write().await;
            // Drop any prior subwallets for this wallet_id before
            // installing the new set so a re-bind with a different
            // account list doesn't leave orphan entries.
            accounts.retain(|id, _| id.wallet_id != wallet_id);
            for (account_index, views) in account_views {
                accounts.insert(SubwalletId::new(wallet_id, account_index), views);
            }
        }

        // Any shape change invalidates prior hydration — the restore
        // that ran for the old shape doesn't cover the new one (an
        // added account may have host-persisted state waiting).
        if shape_changed {
            self.hydrated.write().await.remove(&wallet_id);
        }

        // Purge ONLY the dropped / re-keyed subwallets' store state.
        // Skipped entirely on the identical-registration path (empty
        // `stale`), which keeps that path free of the store lock — it
        // must never queue behind an in-flight sync pass (that
        // blocking, followed by a purge, was the original
        // wipe-the-pass bug). A changed-set re-bind may block here,
        // but it only ever touches subwallets the caller explicitly
        // dropped; retained accounts are left untouched.
        if !stale.is_empty() {
            let mut store = self.store.write().await;
            for id in stale {
                if let Err(e) = store.purge_subwallet(id) {
                    tracing::warn!(
                        wallet_id = %hex::encode(id.wallet_id),
                        account = id.account_index,
                        error = %e,
                        "Failed to purge dropped subwallet store state on re-register"
                    );
                }
            }
        }
    }

    /// Whether `wallet_id`'s per-subwallet store state has been
    /// successfully hydrated from the host snapshot this session
    /// (see [`mark_hydrated`](Self::mark_hydrated)).
    pub async fn is_hydrated(&self, wallet_id: WalletId) -> bool {
        self.begin_install(wallet_id).await.is_hydrated().await
    }

    async fn is_hydrated_locked(&self, wallet_id: WalletId) -> bool {
        self.hydrated.read().await.contains(&wallet_id)
    }

    /// Record whether `wallet_id`'s hydration
    /// ([`restore_for_wallet`](Self::restore_for_wallet) after the
    /// current registration was installed) succeeded. The bind path
    /// sets `true` only after a successful load + restore; a
    /// transient persistence failure leaves it `false` so the next
    /// re-bind retries the restore instead of taking the idempotent
    /// fast path on top of an unhydrated store.
    pub async fn mark_hydrated(&self, wallet_id: WalletId, hydrated: bool) {
        self.begin_install(wallet_id)
            .await
            .mark_hydrated(hydrated)
            .await
    }

    async fn mark_hydrated_locked(&self, wallet_id: WalletId, hydrated: bool) {
        let mut set = self.hydrated.write().await;
        if hydrated {
            set.insert(wallet_id);
        } else {
            set.remove(&wallet_id);
        }
    }

    /// Whether `wallet_id` is currently registered with exactly
    /// `account_views` — the same account indices, each bound to the
    /// same full viewing key (compared by the canonical 96-byte FVK
    /// encoding; IVK / OVK / default address are pure functions of
    /// the FVK, so FVK equality covers the whole viewing set).
    ///
    /// Used by `PlatformWallet::install_shielded_views` to detect an
    /// idempotent re-bind (same wallet, same accounts, same keys):
    /// together with [`is_hydrated`](Self::is_hydrated) it gates the
    /// fast path that skips reloading and re-applying the persister
    /// snapshot. On a matching-and-hydrated re-bind the in-memory
    /// store is strictly fresher than any snapshot, and skipping the
    /// restore keeps the bind free of the store lock — it must never
    /// queue behind an in-flight sync pass (the pre-fix full-purge
    /// re-bind did exactly that and then wiped the pass's
    /// freshly-saved notes and watermark; observed on iOS as "note
    /// discovered by sync is not spendable until app restart" plus a
    /// full rescan on every subsequent pass).
    pub async fn wallet_registration_matches(
        &self,
        wallet_id: WalletId,
        account_views: &BTreeMap<u32, AccountViewingKeys>,
    ) -> bool {
        self.begin_install(wallet_id)
            .await
            .registration_matches(account_views)
            .await
    }

    async fn registration_matches_locked(
        &self,
        wallet_id: WalletId,
        account_views: &BTreeMap<u32, AccountViewingKeys>,
    ) -> bool {
        let accounts = self.accounts.read().await;
        let registered: BTreeMap<u32, [u8; 96]> = accounts
            .iter()
            .filter(|(id, _)| id.wallet_id == wallet_id)
            .map(|(id, views)| (id.account_index, views.to_fvk_bytes()))
            .collect();
        registered.len() == account_views.len()
            && account_views
                .iter()
                .all(|(account, views)| registered.get(account) == Some(&views.to_fvk_bytes()))
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
        // Same lifecycle serialization as an install transaction —
        // without it a concurrent register could interleave between the
        // two map mutations and end up with visible accounts whose
        // persister this removal just deleted, and an in-flight bind's
        // restore could repopulate (and re-mark hydrated) the state
        // purged below after the purge ran.
        let _lifecycle = self.lifecycle.lock().await;
        self.accounts
            .write()
            .await
            .retain(|id, _| id.wallet_id != wallet_id);
        self.persisters.write().await.remove(&wallet_id);
        self.hydrated.write().await.remove(&wallet_id);
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
    /// - **By viewing key**: a subwallet whose snapshot carries a
    ///   viewing key that differs from the registered one is skipped.
    ///   Durable notes, activity and watermarks are keyed by
    ///   `(wallet_id, account_index)` only, so nothing else
    ///   distinguishes rows produced under a since-replaced key; they
    ///   describe funds the current key cannot spend, and their
    ///   watermark would make the pass that should find the current
    ///   key's history skip straight past it. Snapshots with no viewing
    ///   key for a subwallet (persistence predating those rows) are
    ///   restored as before.
    ///
    /// No-op on empty snapshots.
    pub async fn restore_for_wallet(
        &self,
        wallet_id: WalletId,
        snapshot: &crate::changeset::ShieldedSyncStartState,
    ) -> Result<(), crate::error::PlatformWalletError> {
        self.begin_install(wallet_id).await.restore(snapshot).await
    }

    /// [`restore_for_wallet`](Self::restore_for_wallet)'s body, with the
    /// lifecycle mutex already held by the caller's transaction — which
    /// is what makes the registration this reads still be the one in
    /// force when the store write below lands.
    async fn restore_locked(
        &self,
        wallet_id: WalletId,
        snapshot: &crate::changeset::ShieldedSyncStartState,
    ) -> Result<(), crate::error::PlatformWalletError> {
        if snapshot.is_empty() {
            return Ok(());
        }
        // Snapshot of this wallet's registered subwallets and their
        // viewing keys. Cheaper than holding the accounts read lock
        // across the store write below.
        let registered: BTreeMap<SubwalletId, [u8; 96]> = {
            let accounts = self.accounts.read().await;
            accounts
                .iter()
                .filter(|(id, _)| id.wallet_id == wallet_id)
                .map(|(id, views)| (*id, views.to_fvk_bytes()))
                .collect()
        };
        if registered.is_empty() {
            return Ok(());
        }

        let mut store = self.store.write().await;
        for (id, sub) in &snapshot.per_subwallet {
            // Only restore subwallets that are registered on this
            // coordinator — `registered` holds `wallet_id`'s alone, so
            // this covers the by-wallet filter too.
            let Some(registered_fvk) = registered.get(id) else {
                continue;
            };
            // ...and whose snapshot was produced under the key that is
            // registered now. A mismatch means these rows belong to a
            // key this account no longer holds: restoring them would
            // surface unspendable notes and carry over a watermark that
            // hides the current key's own history.
            if let Some(snapshot_fvk) = snapshot.viewing_keys.get(id) {
                if snapshot_fvk.as_slice() != registered_fvk.as_slice() {
                    tracing::warn!(
                        wallet_id = %hex::encode(id.wallet_id),
                        account = id.account_index,
                        "Skipping shielded snapshot restore: it was produced under a \
                         different viewing key than the one registered"
                    );
                    continue;
                }
            }
            // Nullifiers the store already tracks for this subwallet.
            // The in-memory state is always at least as fresh as the
            // host snapshot (the snapshot's rows were produced from
            // it via the persister changesets), so a note the store
            // already knows must NOT be overwritten from the
            // snapshot: `save_note` is overwrite-by-nullifier, and a
            // stale snapshot copy could flip a spent note back to
            // unspent (re-offering it to selection = double-spend
            // attempt) or roll back a fresher block height. Restore
            // is additive-only: it fills in notes the store has never
            // seen (the cold-start case, where this set is empty).
            let known_nullifiers: std::collections::BTreeSet<[u8; 32]> = store
                .get_all_notes(*id)
                .map_err(|e| crate::error::PlatformWalletError::ShieldedStoreError(e.to_string()))?
                .into_iter()
                .map(|n| n.nullifier)
                .collect();
            for note in &sub.notes {
                if known_nullifiers.contains(&note.nullifier) {
                    continue;
                }
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
            // Watermark restore is advance-only. A snapshot loaded
            // before an in-flight sync pass (a mid-session re-bind
            // queues behind the pass's store write lock) carries the
            // PRE-pass watermark; applying it unconditionally rewinds
            // the store below what the pass just scanned and forces a
            // full rescan on every subsequent pass until restart.
            // The store's own value only ever comes from a completed
            // scan or a prior restore, so taking the max is always
            // safe; genuine rewinds (chain rollback handling, Clear)
            // write the store directly rather than through restore.
            let current = store.last_synced_note_index(*id).map_err(|e| {
                crate::error::PlatformWalletError::ShieldedStoreError(e.to_string())
            })?;
            if sub.last_synced_index > current {
                store
                    .set_last_synced_note_index(*id, sub.last_synced_index)
                    .map_err(|e| {
                        crate::error::PlatformWalletError::ShieldedStoreError(e.to_string())
                    })?;
            }
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
        // Held across the whole Clear, store reset included, so no bind
        // install can be halfway through its transaction here: one that
        // already registered would otherwise restore its pre-Clear
        // snapshot into the store this just purged, which is the
        // "Clear did nothing" symptom seen from the other side. Taken
        // before the store lock to keep the `lifecycle` → `store` order.
        let _lifecycle = self.lifecycle.lock().await;

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
            // Verify the reset actually landed to 0 leaves under the SAME
            // write guard. `reset_commitment_tree` can theoretically report
            // Ok while leaving the on-disk tree populated (e.g. a checkpoint
            // that didn't take, or a shardtree reopen that re-materialized a
            // cached frontier). Without this check `clear()` would return Ok,
            // the host would wipe its own Room/SwiftData rows, and the next
            // cold resync would gate-skip every re-downloaded position against
            // the still-full tree — the exact "Clear did nothing, tree frozen
            // at N/N, zero notes scanned" symptom. Turning a silent no-op into
            // a hard error makes the host fail closed (keep its rows) and
            // surfaces the failure instead of masking it.
            if first_err.is_none() {
                match store.tree_size() {
                    Ok(0) => {}
                    Ok(n) => {
                        tracing::error!(
                            remaining_leaves = n,
                            "commitment tree still populated after reset_commitment_tree"
                        );
                        first_err = Some(crate::error::PlatformWalletError::ShieldedStoreError(
                            format!("commitment tree still has {n} leaves after reset"),
                        ));
                    }
                    Err(e) => {
                        first_err = Some(crate::error::PlatformWalletError::ShieldedStoreError(
                            format!("tree_size check after reset failed: {e}"),
                        ));
                    }
                }
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
        self.hydrated.write().await.clear();
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

    /// ON-DISK durability of `clear()` — the regression guard for the
    /// "Clear resets the in-memory tree but not the persisted SQLite" bug.
    ///
    /// The prior test asserts `tree_size() == 0` on the SAME store handle,
    /// which passes even if `clear()` only reset an in-memory shardtree /
    /// frontier and left the on-disk `commitment_tree_*` rows intact. This
    /// test instead reopens the persisted file through a COMPLETELY FRESH
    /// `FileBackedShieldedStore` (independent connection, cold frontier) after
    /// `clear()` and asserts both the tree AND the per-subwallet watermark
    /// read back empty — proving the reset reached disk. Without the on-disk
    /// reset + WAL checkpoint, this fresh handle would reload the full tree
    /// (the on-device "tap Clear, relaunch, still 867/867" symptom).
    #[tokio::test]
    async fn clear_resets_the_persisted_on_disk_store_not_just_memory() {
        let dir = temp_dir("clear_ondisk");
        let db_path = dir.join("tree.sqlite");
        let coordinator = coordinator_with_one_wallet(&dir).await;
        let wallet_id: WalletId = [0x11; 32];
        let id = SubwalletId::new(wallet_id, 0);

        // Build a non-trivial tree AND advance the watermark, then checkpoint
        // — the exact durable state a real sync leaves behind.
        {
            let mut store = coordinator.store().write().await;
            for i in 0..8u8 {
                store.append_commitment(&[i + 1; 32], true).unwrap();
            }
            store.checkpoint_tree(8).unwrap();
            store.set_last_synced_note_index(id, 8).unwrap();
            assert_eq!(store.tree_size().unwrap(), 8);
            assert_eq!(store.last_synced_note_index(id).unwrap(), 8);
        }

        coordinator.clear().await.expect("clear should succeed");

        // Reopen the persisted file with a FRESH handle: this reads the
        // on-disk tables cold (no shared in-memory frontier), so a non-zero
        // size here would mean clear() never wrote the disk.
        let reopened = FileBackedShieldedStore::open_path(&db_path, 100)
            .expect("reopen persisted store after clear");
        assert_eq!(
            reopened.tree_size().unwrap(),
            0,
            "on-disk commitment tree must read 0 through a fresh handle after clear — \
             a non-zero size means clear() only reset the in-memory tree"
        );
        // The watermark is in-memory per store handle (rebuilt from the host's
        // Room/SwiftData on bind, which the host wipes separately), so a fresh
        // handle starts it at 0 by construction; assert it to pin the contract
        // that a cold reopen is caught up to nothing.
        assert_eq!(
            reopened.last_synced_note_index(id).unwrap(),
            0,
            "a freshly reopened store must have no per-subwallet watermark"
        );
        drop(reopened);

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

    /// Build a minimal unspent [`super::super::store::ShieldedNote`]
    /// carrying `nullifier` at `position`.
    fn test_note(
        nullifier: [u8; 32],
        position: u64,
        is_spent: bool,
    ) -> crate::wallet::shielded::ShieldedNote {
        crate::wallet::shielded::ShieldedNote {
            position,
            cmx: [0x2A; 32],
            nullifier,
            block_height: 100,
            is_spent,
            value: 1_000,
            note_data: vec![0u8; 115],
        }
    }

    /// Regression guard for the mid-session re-bind stomp: a
    /// `restore_for_wallet` call whose snapshot predates a completed
    /// sync pass (the exact state a re-bind queued behind the pass's
    /// store write lock applies) must not rewind the watermark and
    /// must not overwrite notes the store already tracks.
    ///
    /// Observed on iOS (rc.2, testnet): pass discovers a note and
    /// advances the watermark 0 → 2248; a second launch-time bind then
    /// applies its pre-pass snapshot, wiping the note from selection
    /// ("No unspent shielded notes available" until app restart) and
    /// rewinding the watermark so every subsequent pass rescanned the
    /// full tree.
    #[tokio::test]
    async fn restore_with_stale_snapshot_does_not_rewind_watermark_or_clobber_notes() {
        use crate::changeset::{ShieldedSubwalletStartState, ShieldedSyncStartState};

        let dir = temp_dir("stale_restore");
        let coordinator = coordinator_with_one_wallet(&dir).await;
        let wallet_id: WalletId = [0x11; 32];
        let id = SubwalletId::new(wallet_id, 0);

        // State a completed sync pass left behind: one fresh unspent
        // note, watermark advanced to 2248.
        let fresh_nf = [0xF1; 32];
        {
            let mut store = coordinator.store().write().await;
            store
                .save_note(id, &test_note(fresh_nf, 2246, false))
                .unwrap();
            store.set_last_synced_note_index(id, 2248).unwrap();
        }

        // A stale snapshot loaded BEFORE that pass: no notes yet,
        // watermark still at the pre-pass value.
        let mut snapshot = ShieldedSyncStartState::default();
        snapshot.per_subwallet.insert(
            id,
            ShieldedSubwalletStartState {
                last_synced_index: 2046,
                ..Default::default()
            },
        );

        coordinator
            .restore_for_wallet(wallet_id, &snapshot)
            .await
            .expect("restore should succeed");

        let store = coordinator.store().read().await;
        assert_eq!(
            store.last_synced_note_index(id).unwrap(),
            2248,
            "a stale snapshot must not rewind the watermark below a completed pass"
        );
        let unspent = store.get_unspent_notes(id).unwrap();
        assert_eq!(unspent.len(), 1, "the pass's note must survive the restore");
        assert_eq!(unspent[0].nullifier, fresh_nf);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A snapshot may never resurrect a note the store already marked
    /// spent: `save_note` is overwrite-by-nullifier, so an
    /// unconditional restore of a stale `is_spent = false` row would
    /// re-offer a spent note to selection (a double-spend attempt at
    /// broadcast). Restore must be additive-only over nullifiers.
    #[tokio::test]
    async fn restore_does_not_resurrect_a_spent_note() {
        use crate::changeset::{ShieldedSubwalletStartState, ShieldedSyncStartState};

        let dir = temp_dir("no_resurrect");
        let coordinator = coordinator_with_one_wallet(&dir).await;
        let wallet_id: WalletId = [0x11; 32];
        let id = SubwalletId::new(wallet_id, 0);

        let nf = [0xD0; 32];
        {
            let mut store = coordinator.store().write().await;
            store.save_note(id, &test_note(nf, 5, false)).unwrap();
            assert!(store.mark_spent(id, &nf).unwrap());
        }

        // Stale snapshot still carries the note as unspent, plus one
        // genuinely new note the store has never seen.
        let new_nf = [0xD1; 32];
        let mut snapshot = ShieldedSyncStartState::default();
        snapshot.per_subwallet.insert(
            id,
            ShieldedSubwalletStartState {
                notes: vec![test_note(nf, 5, false), test_note(new_nf, 6, false)],
                ..Default::default()
            },
        );

        coordinator
            .restore_for_wallet(wallet_id, &snapshot)
            .await
            .expect("restore should succeed");

        let store = coordinator.store().read().await;
        let unspent = store.get_unspent_notes(id).unwrap();
        assert_eq!(
            unspent.len(),
            1,
            "only the genuinely new note is restored; the spent one stays spent"
        );
        assert_eq!(unspent[0].nullifier, new_nf);
        assert_eq!(store.get_all_notes(id).unwrap().len(), 2);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A changed-set re-register purges ONLY the dropped subwallet's
    /// store state: accounts that remain bound with the same viewing
    /// key keep their in-memory notes and watermark across the
    /// re-bind, and the persister handle survives throughout. This is
    /// the account-set-change analog of the identical-re-bind fast
    /// path — a registration replacement racing an in-flight sync
    /// pass can no longer wipe results for accounts the caller kept.
    #[tokio::test]
    async fn changed_set_reregister_keeps_retained_account_state_and_purges_dropped() {
        let dir = temp_dir("reg_partial_purge");
        let coordinator = coordinator_with_one_wallet(&dir).await;
        let wallet_id: WalletId = [0x11; 32];
        let keep = SubwalletId::new(wallet_id, 0);
        let drop_id = SubwalletId::new(wallet_id, 1);

        // Register accounts {0, 1} and give both live store state.
        let views0 = OrchardKeySet::from_seed(&[0x42u8; 64], dashcore::Network::Testnet, 0)
            .expect("derive viewing keys")
            .viewing_keys();
        let views1 = OrchardKeySet::from_seed(&[0x42u8; 64], dashcore::Network::Testnet, 1)
            .expect("derive viewing keys")
            .viewing_keys();
        let persister = WalletPersister::new(wallet_id, Arc::new(NoPlatformPersistence));
        let mut both = BTreeMap::new();
        both.insert(0u32, views0.clone());
        both.insert(1u32, views1);
        coordinator
            .register_wallet(wallet_id, both, persister.clone())
            .await;
        {
            let mut store = coordinator.store().write().await;
            store
                .save_note(keep, &test_note([0xA0; 32], 10, false))
                .unwrap();
            store.set_last_synced_note_index(keep, 500).unwrap();
            store
                .save_note(drop_id, &test_note([0xB0; 32], 11, false))
                .unwrap();
            store.set_last_synced_note_index(drop_id, 500).unwrap();
        }

        // Re-register with only account 0.
        let mut only0 = BTreeMap::new();
        only0.insert(0u32, views0);
        coordinator
            .register_wallet(wallet_id, only0, persister)
            .await;

        let store = coordinator.store().read().await;
        assert_eq!(
            store.get_unspent_notes(keep).unwrap().len(),
            1,
            "retained account keeps its notes across a changed-set re-bind"
        );
        assert_eq!(
            store.last_synced_note_index(keep).unwrap(),
            500,
            "retained account keeps its watermark across a changed-set re-bind"
        );
        assert!(
            store.get_all_notes(drop_id).unwrap().is_empty(),
            "dropped account's notes are purged"
        );
        assert_eq!(
            store.last_synced_note_index(drop_id).unwrap(),
            0,
            "dropped account's watermark is purged"
        );
        drop(store);
        assert!(
            coordinator.persisters.read().await.contains_key(&wallet_id),
            "the persister handle survives the whole re-bind"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Re-registering the same account index under a DIFFERENT viewing
    /// key purges its store state: the stored notes were decrypted
    /// under — and belong to — the old key, and must not survive into
    /// the new registration.
    #[tokio::test]
    async fn rekeyed_account_state_is_purged_on_reregister() {
        let dir = temp_dir("reg_rekey_purge");
        let coordinator = coordinator_with_one_wallet(&dir).await;
        let wallet_id: WalletId = [0x11; 32];
        let id = SubwalletId::new(wallet_id, 0);

        {
            let mut store = coordinator.store().write().await;
            store
                .save_note(id, &test_note([0xC0; 32], 3, false))
                .unwrap();
            store.set_last_synced_note_index(id, 42).unwrap();
        }

        // Same account index, different seed ⇒ different FVK.
        let rekeyed = OrchardKeySet::from_seed(&[0x99u8; 64], dashcore::Network::Testnet, 0)
            .expect("derive viewing keys")
            .viewing_keys();
        let mut views = BTreeMap::new();
        views.insert(0u32, rekeyed);
        let persister = WalletPersister::new(wallet_id, Arc::new(NoPlatformPersistence));
        coordinator
            .register_wallet(wallet_id, views, persister)
            .await;

        let store = coordinator.store().read().await;
        assert!(
            store.get_all_notes(id).unwrap().is_empty(),
            "a re-keyed account's old-key notes must not survive"
        );
        assert_eq!(store.last_synced_note_index(id).unwrap(), 0);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Hydration-flag mechanics: unset by default, set/cleared via
    /// `mark_hydrated`, cleared by a changed-set re-register (the new
    /// shape hasn't been hydrated) but preserved by an identical
    /// re-register (the fast path's precondition), and cleared by
    /// unregister.
    #[tokio::test]
    async fn hydration_flag_follows_registration_lifecycle() {
        let dir = temp_dir("hydration_flag");
        let coordinator = coordinator_with_one_wallet(&dir).await;
        let wallet_id: WalletId = [0x11; 32];

        assert!(!coordinator.is_hydrated(wallet_id).await, "unset initially");
        coordinator.mark_hydrated(wallet_id, true).await;
        assert!(coordinator.is_hydrated(wallet_id).await);

        // Identical re-register preserves the flag.
        let views0 = OrchardKeySet::from_seed(&[0x42u8; 64], dashcore::Network::Testnet, 0)
            .expect("derive viewing keys")
            .viewing_keys();
        let persister = WalletPersister::new(wallet_id, Arc::new(NoPlatformPersistence));
        let mut same = BTreeMap::new();
        same.insert(0u32, views0.clone());
        coordinator
            .register_wallet(wallet_id, same.clone(), persister.clone())
            .await;
        assert!(
            coordinator.is_hydrated(wallet_id).await,
            "identical re-register must not clear hydration"
        );

        // Changed-set re-register clears it.
        let views1 = OrchardKeySet::from_seed(&[0x42u8; 64], dashcore::Network::Testnet, 1)
            .expect("derive viewing keys")
            .viewing_keys();
        let mut expanded = same.clone();
        expanded.insert(1u32, views1);
        coordinator
            .register_wallet(wallet_id, expanded, persister.clone())
            .await;
        assert!(
            !coordinator.is_hydrated(wallet_id).await,
            "a changed account set invalidates prior hydration"
        );

        // And unregister clears it too.
        coordinator.mark_hydrated(wallet_id, true).await;
        coordinator.unregister_wallet(wallet_id).await;
        assert!(!coordinator.is_hydrated(wallet_id).await);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A snapshot produced under a viewing key the account no longer
    /// holds must not be restored: its notes are unspendable under the
    /// registered key and its watermark would hide that key's own
    /// history from the scan. Durable rows are keyed by
    /// `(wallet_id, account_index)` only, so the snapshot's own viewing
    /// key is the only thing that distinguishes them.
    ///
    /// The second half pins that the filter is key-based, not a blanket
    /// skip: the same snapshot restores once its key matches.
    #[tokio::test]
    async fn restore_skips_a_snapshot_produced_under_a_different_viewing_key() {
        use crate::changeset::{ShieldedSubwalletStartState, ShieldedSyncStartState};

        let dir = temp_dir("restore_rekeyed");
        let coordinator = coordinator_with_one_wallet(&dir).await;
        let wallet_id: WalletId = [0x11; 32];
        let id = SubwalletId::new(wallet_id, 0);

        // `coordinator_with_one_wallet` registers account 0 under the
        // 0x42 seed's key; stage a snapshot from a different one.
        let old_key = OrchardKeySet::from_seed(&[0x99u8; 64], dashcore::Network::Testnet, 0)
            .expect("derive viewing keys")
            .viewing_keys();
        let mut snapshot = ShieldedSyncStartState::default();
        snapshot.per_subwallet.insert(
            id,
            ShieldedSubwalletStartState {
                notes: vec![test_note([0xD0; 32], 7, false)],
                last_synced_index: 5_000,
                ..Default::default()
            },
        );
        snapshot
            .viewing_keys
            .insert(id, old_key.to_fvk_bytes().to_vec());

        coordinator
            .restore_for_wallet(wallet_id, &snapshot)
            .await
            .expect("restore reports success, having skipped the stale subwallet");

        {
            let store = coordinator.store().read().await;
            assert!(
                store.get_all_notes(id).unwrap().is_empty(),
                "notes decrypted under a replaced viewing key must not be restored"
            );
            assert_eq!(
                store.last_synced_note_index(id).unwrap(),
                0,
                "the replaced key's watermark must not carry over — it would make the \
                 scan skip the range holding the registered key's own notes"
            );
        }

        // Same snapshot, now carrying the registered key: restored.
        let current_key = OrchardKeySet::from_seed(&[0x42u8; 64], dashcore::Network::Testnet, 0)
            .expect("derive viewing keys")
            .viewing_keys();
        snapshot
            .viewing_keys
            .insert(id, current_key.to_fvk_bytes().to_vec());
        coordinator
            .restore_for_wallet(wallet_id, &snapshot)
            .await
            .expect("restore");

        let store = coordinator.store().read().await;
        assert_eq!(
            store.get_all_notes(id).unwrap().len(),
            1,
            "a snapshot matching the registered key restores as before"
        );
        assert_eq!(store.last_synced_note_index(id).unwrap(), 5_000);
        drop(store);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// An install transaction excludes wallet removal for its whole
    /// scope. Without that, an `unregister_wallet` could purge between a
    /// bind's registration check and its restore, and the restore would
    /// repopulate the state the removal just dropped — then mark the
    /// removed wallet hydrated.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn install_transaction_blocks_unregister_until_it_commits() {
        let dir = temp_dir("install_excludes_unregister");
        let coordinator = Arc::new(coordinator_with_one_wallet(&dir).await);
        let wallet_id: WalletId = [0x11; 32];

        let install = coordinator.begin_install(wallet_id).await;
        let remover = {
            let coordinator = Arc::clone(&coordinator);
            tokio::spawn(async move { coordinator.unregister_wallet(wallet_id).await })
        };

        // Long enough that an unserialized unregister (a couple of
        // in-memory map writes) would certainly have landed.
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(
            !coordinator.registered_subwallets().await.is_empty(),
            "unregister must wait for the in-flight install transaction to commit"
        );

        drop(install);
        remover.await.expect("unregister task");
        assert!(
            coordinator.registered_subwallets().await.is_empty(),
            "the queued unregister runs as soon as the transaction commits"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// `wallet_registration_matches` — the idempotent-re-bind gate:
    /// true only for the exact same wallet + account → FVK map; false
    /// for an unknown wallet, a different account set, or a different
    /// key on the same account index.
    #[tokio::test]
    async fn wallet_registration_matches_detects_identical_rebind() {
        let dir = temp_dir("reg_match");
        let coordinator = coordinator_with_one_wallet(&dir).await;
        let wallet_id: WalletId = [0x11; 32];

        let same_views = OrchardKeySet::from_seed(&[0x42u8; 64], dashcore::Network::Testnet, 0)
            .expect("derive viewing keys")
            .viewing_keys();
        let mut same = BTreeMap::new();
        same.insert(0u32, same_views.clone());
        assert!(
            coordinator
                .wallet_registration_matches(wallet_id, &same)
                .await,
            "identical wallet + account + FVK must match"
        );

        // Unknown wallet id.
        assert!(
            !coordinator
                .wallet_registration_matches([0x99; 32], &same)
                .await
        );

        // Extra account on the request side.
        let extra_views = OrchardKeySet::from_seed(&[0x42u8; 64], dashcore::Network::Testnet, 1)
            .expect("derive viewing keys")
            .viewing_keys();
        let mut extra = same.clone();
        extra.insert(1u32, extra_views.clone());
        assert!(
            !coordinator
                .wallet_registration_matches(wallet_id, &extra)
                .await,
            "a changed account set must not match"
        );

        // Same account index, different key material.
        let mut different_key = BTreeMap::new();
        different_key.insert(0u32, extra_views);
        assert!(
            !coordinator
                .wallet_registration_matches(wallet_id, &different_key)
                .await,
            "a different FVK on the same account must not match"
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
