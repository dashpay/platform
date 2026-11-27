//! Periodic per-identity token state sync coordinator.
//!
//! Self-contained — does not reach into [`PlatformWallet`] or
//! [`WalletManager`]. The manager owns its own identity → token
//! registry (the in-memory `state` cache *is* the registry; there is
//! no second source of truth). Callers add and remove identities and
//! the watched-token sets through the lifecycle API
//! ([`register_identity`](Self::register_identity),
//! [`unregister_identity`](Self::unregister_identity),
//! [`update_watched_tokens`](Self::update_watched_tokens)).
//!
//! The watch list is required, not an optimization: Platform's token
//! balance and info queries take explicit token ids
//! (`GetIdentityTokenBalancesRequestV0.token_ids`), and no query lists the
//! tokens an identity holds. The manager can only sync tokens a caller
//! registered through `update_watched_tokens`.
//!
//! Each pass walks every registered identity, snapshots its watched
//! token list, then sequentially:
//!
//! 1. Chunks the token list into batches of at most
//!    [`MAX_TOKENS_PER_BALANCE_BATCH`] and queries Platform once per
//!    batch via `IdentityTokenBalancesQuery` /
//!    [`TokenAmount::fetch_many`]. Sequential (no parallelism across
//!    batches or identities) — see crate-level note on SDK `!Send`
//!    futures.
//! 2. Builds a [`TokenBalanceChangeSet`] from the batch result and
//!    forwards it to the persister.
//! 3. Updates the manager's own per-identity cache row in lockstep,
//!    so callers reading [`state_for_identity`](Self::state_for_identity)
//!    after [`sync_now`](Self::sync_now) returns see fresh values.
//! 4. Stamps `last_sync_unix` for the identity.
//!
//! Per-(identity, contract) nonce fetching: the registry is keyed by
//! `(identity_id, token_id)` — it doesn't carry `contract_id` today,
//! and Platform requires a separate per-token data-contract fetch to
//! resolve it. Per the design brief, the cache field is plumbed
//! through (`IdentityTokenSyncInfo::contract_id` /
//! `identity_contract_nonce`) but the actual nonce fetch is a
//! follow-up — see the TODO inside [`IdentitySyncManager::sync_now`]
//! and the matching note on [`IdentityTokenSyncInfo::contract_id`].
//!
//! Persister wiring caveat: the manager is identity-scoped, but
//! [`PlatformWalletPersistence::store`] takes a `WalletId`. The
//! changesets written here use [`WalletId::default()`] (`[0u8; 32]`)
//! as a sentinel — token-balance persistence on the FFI / SQLite side
//! is keyed by `(identity_id, token_id)`, so the wallet id is unused
//! on that callback path.
//!
//! Not auto-started. Call [`IdentitySyncManager::start`] once
//! identities are registered and the SDK is connected.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use dpp::balances::credits::TokenAmount;
use dpp::prelude::Identifier;
use tokio::sync::RwLock;

use dash_sdk::platform::tokens::identity_token_balances::{
    IdentityTokenBalances, IdentityTokenBalancesQuery,
};
use dash_sdk::platform::FetchMany;

use dash_async::ThreadRegistry;

use crate::changeset::{PlatformWalletPersistence, TokenBalanceChangeSet};
use crate::manager::sync_loop::{start_sync_loop, Admission, SyncCoordinator, SyncLoop};
use crate::manager::{QuiesceGuard, WalletWorker, COORDINATOR_DRAIN_BUDGET};
use crate::wallet::identity::BlockTime;
use crate::wallet::platform_wallet::WalletId;

/// Default cadence for the identity-token sync loop.
///
/// Token state moves more slowly than UTXO balance — picking 60s here
/// keeps the loop quiet on idle wallets while still catching transfers
/// inside a minute. Tunable at runtime via
/// [`IdentitySyncManager::set_interval`]; this value is just the
/// startup default.
pub const DEFAULT_SYNC_INTERVAL_SECS: u64 = 60;

/// Maximum number of token ids fetched in a single
/// `IdentityTokenBalancesQuery`.
///
/// Platform tolerates larger batches but DAPI rate-limits and proof
/// sizes start to bite past ~100 entries. Keeping this conservative
/// also keeps each round-trip's wall time bounded so the sequential
/// pass doesn't stall behind one slow request.
pub const MAX_TOKENS_PER_BALANCE_BATCH: usize = 100;

/// One row of the per-identity token cache held by
/// [`IdentitySyncManager`].
///
/// Carries the canonical balance for a `(identity_id, token_id)` pair
/// plus the contract context needed to drive token state transitions
/// originating from this identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IdentityTokenSyncInfo {
    /// The token id this row tracks.
    pub token_id: Identifier,
    /// Data contract that owns this token. Multiple tokens issued
    /// from the same data contract share a single per-identity nonce
    /// — see [`Self::identity_contract_nonce`].
    ///
    /// Currently a placeholder (`Identifier::default()`) until token →
    /// contract resolution lands. The watch registry is keyed by
    /// token id only, and resolving the contract id requires a per-
    /// token data-contract fetch — deferred until the registry
    /// carries the field directly. The cache shape includes it now
    /// so consumers don't have to re-plumb later.
    pub contract_id: Identifier,
    /// Latest balance reported by Platform.
    pub balance: TokenAmount,
    /// `IdentityContractNonce` this identity would use for the next
    /// state transition against `contract_id`. Same value across
    /// every row in the cache that shares an `(identity_id,
    /// contract_id)`. Replicated onto each token row so the FFI
    /// mirror can stay a flat array.
    ///
    /// `0` means "not fetched yet" until contract id resolution is
    /// wired up.
    pub identity_contract_nonce: u64,
}

/// Per-identity token sync state held by [`IdentitySyncManager`].
///
/// Identity-scoped, not wallet-scoped — the manager has no wallet
/// linkage of its own; per-wallet aggregation, if needed, lives a
/// layer above.
#[derive(Debug, Clone)]
pub struct IdentityTokenSyncState {
    /// The identity these token rows belong to.
    pub identity_id: Identifier,
    /// Unix seconds at which the most recent pass for this identity
    /// completed. `0` means "no pass has completed".
    pub last_sync_unix: u64,
    /// One row per watched token for this identity.
    pub tokens: Vec<IdentityTokenSyncInfo>,
}

/// Why reading one batch of an identity's token balances failed.
#[derive(Debug, thiserror::Error)]
pub enum TokenBalanceReadError {
    /// Platform could not be queried, or its proof did not verify.
    #[error("could not read token balances from Platform")]
    Fetch(#[source] dash_sdk::Error),
}

/// Outcome of reading one batch of an identity's watched token balances.
#[derive(Debug)]
pub enum TokenBalanceBatchOutcome {
    /// Platform answered with proved balances.
    Ok {
        /// Balance per requested token; `None` means Platform proved there is none.
        balances: BTreeMap<Identifier, Option<TokenAmount>>,
        /// Platform block at which `balances` were proved.
        proved_at: BlockTime,
    },
    /// The read failed; the cached balances of `token_ids` were left unchanged.
    Err {
        /// Tokens this batch requested.
        token_ids: Vec<Identifier>,
        /// Why the read failed.
        error: TokenBalanceReadError,
    },
}

impl TokenBalanceBatchOutcome {
    /// Whether this batch was read successfully.
    pub fn is_ok(&self) -> bool {
        matches!(self, TokenBalanceBatchOutcome::Ok { .. })
    }
}

/// Outcome of syncing a single identity's watched tokens in a pass.
#[derive(Debug, Default)]
pub struct IdentityTokenSyncOutcome {
    /// One entry per balance batch, in the order the batches were read.
    pub batches: Vec<TokenBalanceBatchOutcome>,
}

impl IdentityTokenSyncOutcome {
    /// Whether every batch of this identity was read successfully.
    pub fn is_ok(&self) -> bool {
        self.batches.iter().all(TokenBalanceBatchOutcome::is_ok)
    }
}

/// Summary of one full identity-token sync pass across every registered
/// identity.
#[derive(Debug, Default)]
pub struct IdentitySyncPassSummary {
    /// Per-identity outcomes keyed by identity id. Identities with no
    /// watched tokens are not read and do not appear.
    pub identity_results: BTreeMap<Identifier, IdentityTokenSyncOutcome>,
    /// Unix seconds at which the pass completed. `0` means "no pass ran"
    /// (a concurrent pass was in flight, or sync admission was closed).
    pub sync_unix_seconds: u64,
}

impl IdentitySyncPassSummary {
    /// Whether no identity was read this pass.
    pub fn is_empty(&self) -> bool {
        self.identity_results.is_empty()
    }

    /// Whether this pass ran rather than being skipped.
    pub fn pass_ran(&self) -> bool {
        self.sync_unix_seconds != 0
    }

    /// Number of identities whose every batch was read successfully.
    pub fn success_count(&self) -> usize {
        self.identity_results.values().filter(|o| o.is_ok()).count()
    }

    /// Number of identities with at least one failed batch.
    pub fn error_count(&self) -> usize {
        self.identity_results.len() - self.success_count()
    }
}

/// Periodic per-identity token state sync coordinator.
///
/// Self-contained: drives only its own registry / cache and writes
/// balance updates through an injected
/// [`PlatformWalletPersistence`] handle. No coupling to
/// [`PlatformWallet`](crate::wallet::PlatformWallet) or
/// `WalletManager`.
///
/// `sync_now` is re-entrant-safe: if a pass is already running,
/// calling `sync_now` again returns immediately without doing any
/// work (the caller can check `is_syncing()` to distinguish).
pub struct IdentitySyncManager<P>
where
    P: PlatformWalletPersistence + 'static,
{
    /// SDK handle used to issue `IdentityTokenBalancesQuery` /
    /// `TokenAmount::fetch_many` from the sync loop.
    sdk: Arc<dash_sdk::Sdk>,
    /// Persister for [`TokenBalanceChangeSet`] writes. Identity-scoped
    /// changesets travel under [`WalletId::default()`] since this
    /// manager is not wallet-scoped — see crate-level docs. Generic
    /// over `P` so every `persister.store(...)` call on the hot sync
    /// loop dispatches statically.
    persister: Arc<P>,
    /// The lifecycle shared with the sibling coordinators: the
    /// registry-owned loop thread, the `is_syncing` slot, the admission
    /// gate, and the timestamp of the last completed pass across all
    /// identities (identity-level timestamps live on the per-identity
    /// rows in [`IdentitySyncManager::state`]). `sync_now` bails (after
    /// taking the slot) when the gate is closed — by a
    /// [`quiesce`](Self::quiesce) draining an in-flight pass, by a
    /// [`QuiesceGuard`] holder mutating state, or terminally once shutdown
    /// seals it — so once a drain observes `is_syncing == false` no
    /// further pass can start, giving shutdown a real "no more
    /// host-visible persister stores" barrier that cancel-only
    /// [`stop`](Self::stop) does not provide.
    sync: SyncLoop,
    /// Per-identity registry / cache. Keyed by identity id; each row
    /// carries the per-(identity, token) token rows plus the
    /// per-identity last-sync timestamp.
    ///
    /// This is *the* registry — there is no separate "watched tokens"
    /// store. Lifecycle methods
    /// ([`register_identity`](Self::register_identity),
    /// [`update_watched_tokens`](Self::update_watched_tokens),
    /// [`unregister_identity`](Self::unregister_identity)) operate on
    /// this map and the sync loop iterates it.
    state: RwLock<BTreeMap<Identifier, IdentityTokenSyncState>>,
}

impl<P> IdentitySyncManager<P>
where
    P: PlatformWalletPersistence + 'static,
{
    /// Construct a new manager. Pass an SDK handle (for token-balance
    /// fetches) and a persister handle (for `TokenBalanceChangeSet`
    /// writes). The registry starts empty — call
    /// [`register_identity`](Self::register_identity) before
    /// [`start`](Self::start).
    pub fn new(
        sdk: Arc<dash_sdk::Sdk>,
        persister: Arc<P>,
        registry: Arc<ThreadRegistry<WalletWorker>>,
    ) -> Self {
        Self {
            sdk,
            persister,
            sync: SyncLoop::new(
                WalletWorker::IdentitySync,
                registry,
                DEFAULT_SYNC_INTERVAL_SECS,
            ),
            state: RwLock::new(BTreeMap::new()),
        }
    }

    /// Add or replace the registry row for `identity_id`.
    ///
    /// Idempotent — calling with the same identity overwrites the
    /// existing row and resets `last_sync_unix` to `0`. Each distinct
    /// token in `token_ids` (a repeated id is watched once) becomes a
    /// watched row with `balance = 0`,
    /// `contract_id = Identifier::default()`,
    /// `identity_contract_nonce = 0`. The next sync pass populates
    /// real values.
    pub async fn register_identity<I>(&self, identity_id: Identifier, token_ids: I)
    where
        I: IntoIterator<Item = Identifier>,
    {
        let tokens: Vec<IdentityTokenSyncInfo> = distinct_token_ids(token_ids)
            .into_iter()
            .map(|token_id| IdentityTokenSyncInfo {
                token_id,
                contract_id: Identifier::default(),
                balance: 0,
                identity_contract_nonce: 0,
            })
            .collect();
        let mut state = self.state.write().await;
        state.insert(
            identity_id,
            IdentityTokenSyncState {
                identity_id,
                last_sync_unix: 0,
                tokens,
            },
        );
    }

    /// Remove the registry row for `identity_id`.
    ///
    /// Idempotent — a no-op if the identity isn't registered.
    pub async fn unregister_identity(&self, identity_id: &Identifier) {
        let mut state = self.state.write().await;
        state.remove(identity_id);
    }

    /// Whether `identity_id` currently has a registry row. Test-only
    /// observability for the teardown-ordering regression: a row a same-id
    /// recreation registers mid-removal must survive the old generation's
    /// id-keyed unregisters.
    #[cfg(test)]
    pub(crate) async fn is_identity_registered(&self, identity_id: &Identifier) -> bool {
        self.state.read().await.contains_key(identity_id)
    }

    /// Replace the watched-token list for an already-registered
    /// identity.
    ///
    /// Tokens that appear in both the old and new lists keep their
    /// cached balance / contract / nonce. Tokens that drop out of the
    /// new list are removed from the cache. Tokens new to the list
    /// are inserted with `balance = 0` and zero placeholders, just
    /// like [`register_identity`](Self::register_identity).
    ///
    /// If the identity isn't registered, this is a no-op — callers
    /// must call [`register_identity`](Self::register_identity)
    /// first. This is the conservative choice: silently promoting an
    /// unknown identity to a registered one would mask programmer
    /// errors at the caller (e.g. typoed identity ids that today
    /// would surface as "balance never updates", with this method
    /// promoting them they'd surface as "spurious DAPI traffic for an
    /// identity I never meant to track").
    pub async fn update_watched_tokens<I>(&self, identity_id: Identifier, token_ids: I)
    where
        I: IntoIterator<Item = Identifier>,
    {
        let new_tokens = distinct_token_ids(token_ids);
        let mut state = self.state.write().await;
        let Some(row) = state.get_mut(&identity_id) else {
            return;
        };
        // Index existing rows by token id so we can preserve balances
        // for tokens still in the new set.
        let existing: BTreeMap<Identifier, IdentityTokenSyncInfo> = row
            .tokens
            .iter()
            .map(|info| (info.token_id, *info))
            .collect();

        let merged: Vec<IdentityTokenSyncInfo> = new_tokens
            .into_iter()
            .map(|token_id| {
                existing
                    .get(&token_id)
                    .copied()
                    .unwrap_or(IdentityTokenSyncInfo {
                        token_id,
                        contract_id: Identifier::default(),
                        balance: 0,
                        identity_contract_nonce: 0,
                    })
            })
            .collect();

        row.tokens = merged;
    }

    /// Set the polling interval. Clamped to a minimum of 1s.
    ///
    /// The running loop picks this up on its next sleep.
    pub fn set_interval(&self, interval: Duration) {
        self.sync.set_interval(interval);
    }

    /// Current polling interval.
    pub fn interval(&self) -> Duration {
        self.sync.interval()
    }

    /// Whether the background loop is currently running.
    pub fn is_running(&self) -> bool {
        self.sync.is_running()
    }

    /// Whether a sync pass is in flight right now.
    pub fn is_syncing(&self) -> bool {
        self.sync.is_syncing()
    }

    /// Unix seconds of the last completed pass (across all identities),
    /// or `None` if no pass has ever completed.
    pub fn last_sync_unix_seconds(&self) -> Option<u64> {
        self.sync.last_sync_unix_seconds()
    }

    /// Per-identity last-sync timestamp.
    ///
    /// Returns `None` if the identity has never been synced (or isn't
    /// known to this manager yet).
    pub async fn last_sync_unix_for_identity(&self, identity_id: &Identifier) -> Option<u64> {
        let state = self.state.read().await;
        state
            .get(identity_id)
            .map(|s| s.last_sync_unix)
            .filter(|t| *t != 0)
    }

    /// Snapshot the cache row for a single identity, cloned out so
    /// callers don't hold the manager's read lock across their work.
    pub async fn state_for_identity(
        &self,
        identity_id: &Identifier,
    ) -> Option<IdentityTokenSyncState> {
        let state = self.state.read().await;
        state.get(identity_id).cloned()
    }

    /// Snapshot the entire per-identity cache. Cheap clone of a
    /// `BTreeMap<Identifier, IdentityTokenSyncState>` — every value
    /// itself owns a `Vec<IdentityTokenSyncInfo>`, so the clone is
    /// O(total tokens). Used by the FFI snapshot path.
    pub async fn all_state(&self) -> BTreeMap<Identifier, IdentityTokenSyncState> {
        let state = self.state.read().await;
        state.clone()
    }

    /// Best-effort registry depth for the diagnostic snapshot path.
    /// Returns the number of identities currently registered, or
    /// `None` if the registry can't be acquired without parking the
    /// thread (i.e. another writer is in flight). The diagnostic
    /// surface treats `None` as "0" so the caller never blocks.
    pub fn try_queue_depth(&self) -> Option<usize> {
        self.state.try_read().ok().map(|s| s.len())
    }

    /// Start the background sync loop. Idempotent — calling while
    /// already running is a no-op.
    ///
    /// The loop runs on a dedicated OS thread, not on a tokio worker.
    /// This is forced on us by the fact that the SDK token-fetch
    /// futures are `!Send` (the GRPC client state inside the SDK
    /// isn't `Send + Sync`), so they can't ride on `tokio::spawn`,
    /// which demands `Future: Send + 'static`. We use
    /// [`tokio::runtime::Handle::block_on`] so the future still has
    /// access to the main runtime's reactor for network I/O — only
    /// the polling thread is dedicated.
    ///
    /// The first pass runs immediately; subsequent passes fire every
    /// [`interval`](Self::interval).
    ///
    /// **Blocks briefly on restart**: the shared registry synchronously
    /// reaps a still-draining prior-generation thread, spinning up to the
    /// registry reap backstop (default 1 s) before returning. Call it from
    /// the FFI host thread, not an async task.
    pub fn start(self: Arc<Self>) {
        // The registry owns the whole lifecycle: it takes the `closing` /
        // `clearing` latches, installs the cancellation token, spawns the
        // thread, and parks any still-draining prior generation — all under
        // one slot lock, so there is no check-then-spawn gap to race. A
        // no-op if a worker is already live, or if teardown has begun.
        start_sync_loop(self, None, "identity");
    }

    /// Stop the background sync loop. No-op if not running.
    ///
    /// **Cancel-only**: requests cancellation and returns immediately. A
    /// pass already inside `sync_now` keeps running to completion,
    /// including its `persister.store(...)` fan-out. For a real "nothing
    /// is running and nothing more will be persisted" barrier — required
    /// by manager shutdown so the host can free the persister context —
    /// use [`quiesce`](Self::quiesce).
    pub fn stop(&self) {
        self.sync.cancel_loop();
    }

    /// Cancel the background loop **and wait for any in-flight sync pass
    /// to fully drain** before returning — a real quiescence barrier,
    /// unlike cancel-only [`stop`](Self::stop).
    ///
    /// After this returns, no sync pass is running and none can start
    /// until the next [`start`](Self::start) / `sync_now`, so a caller
    /// that immediately tears the manager down (and frees the host-owned
    /// persister context the FFI handed to us) cannot be raced by a pass
    /// that calls `persister.store(...)` through a now-dangling pointer.
    ///
    /// Mechanism: close the `quiescing` gate so any pass that hasn't yet
    /// taken the `is_syncing` slot bails, cancel the loop, then wait for
    /// `is_syncing` to clear. `is_syncing` is held for the whole pass
    /// including the persister fan-out (`sync_now` clears it only after
    /// every `sync_identity` / `apply_fresh_balances` store completes),
    /// so its falling edge (with the gate up) is a sound "fully drained"
    /// signal. The gate is reopened before returning so a later
    /// start/sync works normally.
    ///
    /// **Bounded** by `COORDINATOR_DRAIN_BUDGET`: returns `false` if
    /// the in-flight pass did not drain in time — see
    /// `quiesce_within` for the timeout contract.
    #[must_use = "a false return means the pass did NOT drain; the caller must fail closed"]
    pub async fn quiesce(&self) -> bool {
        self.quiesce_within(COORDINATOR_DRAIN_BUDGET).await
    }

    /// [`quiesce`](Self::quiesce) with an explicit drain budget.
    ///
    /// Returns `true` when the drain completed. Returns `false` when
    /// `is_syncing` was still held at the deadline — a wedged pass. On
    /// that path the `quiescing` gate is deliberately **left closed** so the
    /// wedged pass cannot be followed by a fresh one; the caller must
    /// treat the coordinator as non-clean. A later successful `quiesce`
    /// reopens the gate, as does [`start`](Self::start), the running
    /// loop, or a one-shot `sync_now`, once the wedged pass has released
    /// `is_syncing`.
    pub(crate) async fn quiesce_within(&self, budget: Duration) -> bool {
        // The guard drops here, reopening the gate — this is the
        // "drain only" flavor.
        self.quiesce_held_within(budget).await.is_some()
    }

    /// [`quiesce_within`](Self::quiesce_within) that **keeps sync admission
    /// shut** until the returned guard drops — the barrier a caller needs
    /// when it mutates state a pass touches right after draining.
    ///
    /// `None` means the in-flight pass did not drain within `budget`; the
    /// gate is left closed and the caller must fail closed.
    #[must_use = "None means the pass did NOT drain; the caller must fail closed"]
    pub(crate) async fn quiesce_held_within(&self, budget: Duration) -> Option<QuiesceGuard<'_>> {
        // One deadline for both waits: a store stalled inside the persister
        // holds the slot and the registry lock, and must not use the budget
        // twice.
        let deadline = tokio::time::Instant::now() + budget;
        let guard = self.sync.drain(|| self.stop(), budget).await;
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if !self.persistence_barrier(remaining).await {
            // A store admitted before the gate closed is still running (a
            // stalled persister): report the drain as incomplete and keep
            // admission shut until that pass releases `is_syncing`.
            self.sync.latch_admission_closed();
            return None;
        }
        guard
    }

    /// [`quiesce_within`](Self::quiesce_within) that **seals** the gate:
    /// admission never reopens on this coordinator instance.
    ///
    /// Used by manager shutdown. Reopening there would let a direct
    /// `sync_now` that was already dispatched on a host thread — the FFI
    /// resolves the manager under a shared read guard, so it can be
    /// mid-flight while `destroy` runs — start a fresh pass *after* the
    /// drain concluded and fire persister / completion callbacks through
    /// a context the host has since freed.
    pub(crate) async fn quiesce_sealed_within(&self, budget: Duration) -> bool {
        // Through `quiesce_held_within`, so the sealed drain waits out the
        // persistence barrier too.
        let guard = self.quiesce_held_within(budget).await;
        let drained = guard.is_some();
        // Seal before the guard drops so its Drop cannot reopen.
        self.sync.seal_admission();
        drop(guard);
        drained
    }

    /// Wait, at most `budget` (the quiesce's remaining time), for a result application already past its
    /// gate check. Results are applied under the `state` write lock after
    /// checking the gate, so once the gate is closed and this lock has been
    /// taken once, no pass (including one whose drain timed out) stores
    /// anything more. `false` means a store admitted earlier is still running
    /// at the deadline; the caller reports the quiesce as incomplete instead
    /// of waiting on the persister without limit.
    async fn persistence_barrier(&self, budget: Duration) -> bool {
        tokio::time::timeout(budget, self.state.write())
            .await
            .is_ok()
    }

    /// Test-only: claim the `is_syncing` slot and never release it,
    /// standing in for a pass wedged in a network / persister await.
    /// Returns `false` if the slot was already taken.
    #[cfg(test)]
    pub(crate) fn wedge_sync_slot_for_test(&self) -> bool {
        self.sync.wedge_slot_for_test()
    }

    /// Test-only: release a slot taken by
    /// [`wedge_sync_slot_for_test`](Self::wedge_sync_slot_for_test), as the
    /// wedged pass finally draining would.
    #[cfg(test)]
    pub(crate) fn release_sync_slot_for_test(&self) {
        self.sync.release_slot_for_test();
    }

    /// Test-only: seal admission as manager shutdown does, without a drain.
    #[cfg(test)]
    pub(crate) fn seal_sync_admission_for_test(&self) {
        self.sync.seal_admission();
    }

    /// Test-only: whether new sync passes are currently barred — a drain in
    /// flight, a latched timeout, or the terminal seal `shutdown` applies.
    #[cfg(test)]
    pub(crate) fn sync_admission_closed(&self) -> bool {
        self.sync.admission_closed()
    }

    /// Run one sync pass across every registered identity.
    ///
    /// If a pass is already in flight, or sync admission is closed,
    /// returns an empty summary with `sync_unix_seconds == 0` without
    /// doing any work (re-entrant safe). Per-batch read failures are
    /// logged and recorded in the summary but never abort the sweep.
    ///
    /// Iteration order: identities in `Identifier` order, batches in
    /// token-id order. Sequential — no parallelism across batches or
    /// identities — both because the SDK token-fetch futures are
    /// `!Send` (no `tokio::spawn`) and because the design brief
    /// explicitly forbids it.
    pub async fn sync_now(&self) -> IdentitySyncPassSummary {
        match self.try_sync_pass().await {
            SyncAttempt::Ran(summary) => summary,
            SyncAttempt::Busy | SyncAttempt::Closed => IdentitySyncPassSummary::default(),
        }
    }

    /// One attempt at a pass: [`SyncAttempt::Busy`] when another pass holds
    /// the slot, [`SyncAttempt::Closed`] when the quiesce gate is shut,
    /// [`SyncAttempt::Ran`], with the pass's summary, after this call ran the
    /// pass itself.
    async fn try_sync_pass(&self) -> SyncAttempt {
        // Bail without running a pass when another one holds the slot, or
        // when a `quiesce()` has closed the gate — so the drain can complete
        // and shutdown gets a true barrier (no further `persister.store(...)`
        // after quiesce returns). The permit holds `is_syncing` until it
        // drops at the end of this function, on panic unwind too, so a
        // failed pass can never wedge `quiesce()`'s drain.
        let _permit = match self.sync.admit() {
            Admission::Admitted(permit) => permit,
            Admission::Busy => return SyncAttempt::Busy,
            Admission::Closed => return SyncAttempt::Closed,
        };

        // Snapshot the per-identity watch list under a short read
        // lock and release it before any network call. We keep
        // `Vec<Identifier>` in token-id order so each batch chunk is
        // deterministic.
        let per_identity: BTreeMap<Identifier, Vec<Identifier>> = {
            let state = self.state.read().await;
            state
                .iter()
                .filter(|(_, row)| !row.tokens.is_empty())
                .map(|(iid, row)| (*iid, row.tokens.iter().map(|t| t.token_id).collect()))
                .collect()
        };

        let mut summary = IdentitySyncPassSummary::default();
        for (identity_id, token_ids) in per_identity {
            let outcome = self.sync_identity(identity_id, &token_ids).await;
            summary.identity_results.insert(identity_id, outcome);
        }

        let now = SyncLoop::unix_now();
        summary.sync_unix_seconds = now;
        self.sync.record_completed(now);
        SyncAttempt::Ran(summary)
    }

    /// [`sync_now`](Self::sync_now) for a manual caller that has just
    /// registered identities or tokens and reads the stored balances once it
    /// returns: it runs a pass of its own, waiting out any pass in flight
    /// (which may have read the watch list before the registration, and
    /// whose results may not be stored yet) and retrying when another caller
    /// takes the slot first. Returns `true` once its own pass ran, and
    /// `false` without a pass when the quiesce gate closes or when other
    /// passes keep the slot past [`COORDINATOR_DRAIN_BUDGET`] (a wedged pass
    /// must not hold the caller, and through the FFI the manager registry,
    /// indefinitely); the caller then reports that no pass ran. Timer ticks
    /// keep using `sync_now`, so they never wait.
    pub async fn sync_now_after_in_flight(&self) -> bool {
        let deadline = tokio::time::Instant::now() + COORDINATOR_DRAIN_BUDGET;
        loop {
            match self.try_sync_pass().await {
                SyncAttempt::Ran(_) => return true,
                SyncAttempt::Closed => return false,
                SyncAttempt::Busy => {
                    while self.sync.is_syncing() {
                        if self.sync.admission_closed() || tokio::time::Instant::now() >= deadline {
                            return false;
                        }
                        tokio::time::sleep(Duration::from_millis(20)).await;
                    }
                    // The slot may have freed only after the deadline passed
                    // (between two polls): do not start a pass past it.
                    if tokio::time::Instant::now() >= deadline {
                        return false;
                    }
                }
            }
        }
    }

    /// Sync a single identity's watched tokens against Platform.
    ///
    /// Splits `token_ids` into batches of at most
    /// [`MAX_TOKENS_PER_BALANCE_BATCH`], issues one
    /// `IdentityTokenBalancesQuery` per batch, builds a
    /// [`TokenBalanceChangeSet`] for the persister, and rewrites this
    /// manager's per-identity cache row.
    ///
    /// Errors are logged, reported per batch in the returned outcome, and
    /// the per-identity row is left at its previous value rather than
    /// being cleared — a transient DAPI error shouldn't drop the cached
    /// balance from under the UI.
    async fn sync_identity(
        &self,
        identity_id: Identifier,
        token_ids: &[Identifier],
    ) -> IdentityTokenSyncOutcome {
        let mut outcome = IdentityTokenSyncOutcome::default();
        if token_ids.is_empty() {
            return outcome;
        }

        // Accumulate balances across batches before touching the
        // cache or the persister — keep batch network calls unlocked.
        let mut fresh_balances: BTreeMap<Identifier, Option<TokenAmount>> = BTreeMap::new();

        for chunk in token_ids.chunks(MAX_TOKENS_PER_BALANCE_BATCH) {
            let query = IdentityTokenBalancesQuery {
                identity_id,
                token_ids: chunk.to_vec(),
            };

            // Type-annotate the call site explicitly: `fetch_many_with_metadata`
            // is generic over the response type, and the inference
            // chain through `RetrievedObjects` doesn't pick a unique
            // implementor without a hint.
            let fetched: Result<(IdentityTokenBalances, _), _> =
                TokenAmount::fetch_many_with_metadata(self.sdk.as_ref(), query, None).await;
            match fetched {
                Ok((result, metadata)) => {
                    let balances: BTreeMap<Identifier, Option<TokenAmount>> =
                        result.iter().map(|(id, b)| (*id, *b)).collect();
                    fresh_balances.extend(&balances);
                    outcome.batches.push(TokenBalanceBatchOutcome::Ok {
                        balances,
                        proved_at: BlockTime::from(metadata),
                    });
                }
                Err(e) => {
                    tracing::warn!(
                        identity_id = %identity_id,
                        chunk_len = chunk.len(),
                        error = %e,
                        "identity-sync: token balance batch failed; leaving cache untouched for this batch"
                    );
                    // Skip this batch; do not poison fresh_balances.
                    // Other batches for this identity may still
                    // succeed.
                    outcome.batches.push(TokenBalanceBatchOutcome::Err {
                        token_ids: chunk.to_vec(),
                        error: TokenBalanceReadError::Fetch(e),
                    });
                }
            }
        }

        if !fresh_balances.is_empty() {
            self.apply_fresh_balances(identity_id, fresh_balances).await;
        }
        outcome
    }

    async fn apply_fresh_balances(
        &self,
        identity_id: Identifier,
        fresh_balances: BTreeMap<Identifier, Option<TokenAmount>>,
    ) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let mut state = self.state.write().await;
        // A pass that outlived a quiesce (its drain timed out, or shutdown
        // sealed the gate while it was reading) stores nothing: after
        // `quiesce` returns no `persister.store(...)` may follow, or a
        // retired manager could overwrite balances its replacement wrote.
        // Checked under the write lock the quiesce barrier takes.
        if self.sync.admission_closed() {
            return;
        }
        let Some(existing_row) = state.get(&identity_id).cloned() else {
            return;
        };

        // Persist only tokens still watched now: a registration that
        // replaced the watch list while this pass was reading (for example
        // after the host deleted the token's contract) must not have a
        // stale result written back. Checked under the same write lock
        // the registration takes.
        let mut cs = TokenBalanceChangeSet::default();
        for prior in &existing_row.tokens {
            let key = (identity_id, prior.token_id);
            match fresh_balances.get(&prior.token_id) {
                Some(Some(amount)) => {
                    cs.balances.insert(key, *amount);
                }
                Some(None) => {
                    cs.removed_balances.insert(key);
                }
                None => {}
            }
        }
        if cs.balances.is_empty() && cs.removed_balances.is_empty() {
            return;
        }

        // The persister API is wallet-scoped (`store(wallet_id, ..)`)
        // but this manager is identity-scoped. Use the zero-byte
        // sentinel — the FFI / SQLite token-balance write paths key
        // their rows by `(identity_id, token_id)` and ignore the
        // wallet id on this changeset.
        let sentinel: WalletId = WalletId::default();
        if let Err(e) = self.persister.store(sentinel, cs.into()) {
            tracing::error!(
                identity_id = %identity_id,
                error = %e,
                "identity-sync: failed to persist token balance changeset"
            );
        }

        let mut new_tokens: Vec<IdentityTokenSyncInfo> =
            Vec::with_capacity(existing_row.tokens.len());
        for prior in &existing_row.tokens {
            match fresh_balances.get(&prior.token_id) {
                Some(Some(amount)) => {
                    new_tokens.push(IdentityTokenSyncInfo {
                        balance: *amount,
                        ..*prior
                    });
                }
                // Proved absent: the persisted row was removed above, but
                // the token stays watched so a later pass sees it arrive.
                Some(None) => {
                    new_tokens.push(IdentityTokenSyncInfo {
                        balance: 0,
                        ..*prior
                    });
                }
                None => {
                    new_tokens.push(*prior);
                }
            }
        }

        state.insert(
            identity_id,
            IdentityTokenSyncState {
                identity_id,
                last_sync_unix: now,
                tokens: new_tokens,
            },
        );
    }
}

impl<P> SyncCoordinator for IdentitySyncManager<P>
where
    P: PlatformWalletPersistence + 'static,
{
    fn sync_loop(&self) -> &SyncLoop {
        &self.sync
    }

    async fn background_pass(&self) {
        self.sync_now().await;
    }
}

/// Outcome of one [`IdentitySyncManager`] pass attempt.
#[derive(Debug)]
enum SyncAttempt {
    /// This call held the slot and ran the pass, with what the pass read.
    Ran(IdentitySyncPassSummary),
    /// Another pass held the slot.
    Busy,
    /// The quiesce gate was shut; no pass ran.
    Closed,
}

/// `token_ids` with repeats removed, in first-seen order. A balance batch
/// with a repeated id fails proof verification as a whole, so the watch
/// list never holds one.
fn distinct_token_ids<I>(token_ids: I) -> Vec<Identifier>
where
    I: IntoIterator<Item = Identifier>,
{
    let mut seen = BTreeSet::new();
    token_ids
        .into_iter()
        .filter(|token_id| seen.insert(*token_id))
        .collect()
}

impl<P> std::fmt::Debug for IdentitySyncManager<P>
where
    P: PlatformWalletPersistence + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IdentitySyncManager")
            .field("is_running", &self.is_running())
            .field("is_syncing", &self.is_syncing())
            .field("interval_secs", &self.interval().as_secs())
            .field("last_sync_unix", &self.last_sync_unix_seconds())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::changeset::{ClientStartState, PersistenceError, PlatformWalletChangeSet};
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

    /// Test-only persister that swallows every `store` call and
    /// records nothing. Lifecycle / registry tests don't need the
    /// real persistence pipeline; they just need a handle that
    /// satisfies the `Arc<dyn PlatformWalletPersistence>` constructor
    /// parameter.
    struct NoopPersister;

    impl PlatformWalletPersistence for NoopPersister {
        fn store(
            &self,
            _wallet_id: WalletId,
            _changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            Ok(())
        }

        fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
            Ok(())
        }

        fn load(&self) -> Result<ClientStartState, PersistenceError> {
            Ok(ClientStartState::default())
        }
    }

    struct RecordingPersister {
        stores: AtomicUsize,
    }

    impl RecordingPersister {
        fn new() -> Self {
            Self {
                stores: AtomicUsize::new(0),
            }
        }
    }

    impl PlatformWalletPersistence for RecordingPersister {
        fn store(
            &self,
            _wallet_id: WalletId,
            _changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            self.stores.fetch_add(1, AtomicOrdering::SeqCst);
            Ok(())
        }

        fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
            Ok(())
        }

        fn load(&self) -> Result<ClientStartState, PersistenceError> {
            Ok(ClientStartState::default())
        }
    }

    /// Build a manager wired to a no-op persister. The SDK is
    /// constructed via `SdkBuilder::new_mock` so we don't need a
    /// running runtime for the registry/lifecycle tests below; none
    /// of them exercise the network path.
    fn make_manager() -> Arc<IdentitySyncManager<NoopPersister>> {
        make_manager_with_registry().0
    }

    /// [`make_manager`] plus the registry that owns the loop thread, for
    /// tests that join it.
    fn make_manager_with_registry() -> (
        Arc<IdentitySyncManager<NoopPersister>>,
        Arc<ThreadRegistry<WalletWorker>>,
    ) {
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let persister = Arc::new(NoopPersister);
        let registry = ThreadRegistry::<WalletWorker>::new();
        (
            Arc::new(IdentitySyncManager::new(
                sdk,
                persister,
                Arc::clone(&registry),
            )),
            registry,
        )
    }

    const PROVED_HEIGHT: u64 = 4_242;
    const PROVED_TIME_MS: u64 = 1_750_000_000_000;

    /// Build a recording manager whose mock SDK answers exactly the given
    /// balance queries, proved at [`PROVED_HEIGHT`] / [`PROVED_TIME_MS`].
    /// Any other query fails, standing in for a DAPI error.
    async fn make_answering_manager(
        answers: Vec<(IdentityTokenBalancesQuery, IdentityTokenBalances)>,
    ) -> (
        Arc<IdentitySyncManager<RecordingPersister>>,
        Arc<RecordingPersister>,
    ) {
        let mut sdk = dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk");
        let mut mock = sdk.mock();
        mock.set_response_height(PROVED_HEIGHT)
            .set_response_time_ms(PROVED_TIME_MS);
        for (query, balances) in answers {
            mock.expect_fetch_many::<Identifier, TokenAmount, _, IdentityTokenBalances>(
                query,
                Some(balances),
            )
            .await
            .expect("token balances expectation");
        }
        drop(mock);
        let persister = Arc::new(RecordingPersister::new());
        (
            Arc::new(IdentitySyncManager::new(
                Arc::new(sdk),
                Arc::clone(&persister),
                ThreadRegistry::<WalletWorker>::new(),
            )),
            persister,
        )
    }

    fn make_recording_manager() -> (
        Arc<IdentitySyncManager<RecordingPersister>>,
        Arc<RecordingPersister>,
    ) {
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let persister = Arc::new(RecordingPersister::new());
        (
            Arc::new(IdentitySyncManager::new(
                sdk,
                Arc::clone(&persister),
                ThreadRegistry::<WalletWorker>::new(),
            )),
            persister,
        )
    }

    /// `register_identity` populates a row with zero-balance
    /// placeholders for each token, and `state_for_identity` returns
    /// the cloned row. Validates the read API the FFI snapshot path
    /// depends on.
    #[tokio::test]
    async fn register_and_read_identity_state() {
        let mgr = make_manager();
        let id_a = Identifier::from([1u8; 32]);
        let id_b = Identifier::from([2u8; 32]);
        let token_x = Identifier::from([10u8; 32]);
        let token_y = Identifier::from([11u8; 32]);

        mgr.register_identity(id_a, [token_x, token_y]).await;
        mgr.register_identity(id_b, [token_x]).await;

        let row_a = mgr.state_for_identity(&id_a).await.unwrap();
        assert_eq!(row_a.identity_id, id_a);
        assert_eq!(row_a.last_sync_unix, 0);
        assert_eq!(row_a.tokens.len(), 2);
        assert!(row_a.tokens.iter().all(|t| t.balance == 0));
        assert!(row_a
            .tokens
            .iter()
            .all(|t| t.contract_id == Identifier::default()));
        assert!(row_a.tokens.iter().all(|t| t.identity_contract_nonce == 0));

        // last_sync_unix_for_identity reports None until a pass
        // completes (placeholder rows have last_sync_unix == 0).
        assert_eq!(mgr.last_sync_unix_for_identity(&id_a).await, None);

        // Unknown identity → None on every read API.
        let unknown = Identifier::from([99u8; 32]);
        assert!(mgr.state_for_identity(&unknown).await.is_none());
        assert_eq!(mgr.last_sync_unix_for_identity(&unknown).await, None);

        // all_state returns both rows.
        let all = mgr.all_state().await;
        assert_eq!(all.len(), 2);
        assert!(all.contains_key(&id_a));
        assert!(all.contains_key(&id_b));
    }

    /// `register_identity` is idempotent: a second call replaces the
    /// row, including its watched-token set, and resets
    /// `last_sync_unix` to 0.
    #[tokio::test]
    async fn register_identity_is_idempotent() {
        let mgr = make_manager();
        let id_a = Identifier::from([1u8; 32]);
        let token_x = Identifier::from([10u8; 32]);
        let token_y = Identifier::from([11u8; 32]);
        let token_z = Identifier::from([12u8; 32]);

        mgr.register_identity(id_a, [token_x, token_y]).await;
        // Re-register with a different token set.
        mgr.register_identity(id_a, [token_z]).await;

        let row = mgr.state_for_identity(&id_a).await.unwrap();
        assert_eq!(row.tokens.len(), 1);
        assert_eq!(row.tokens[0].token_id, token_z);
        assert_eq!(row.last_sync_unix, 0);
    }

    /// `unregister_identity` drops the row; calling it again is a
    /// no-op rather than an error.
    #[tokio::test]
    async fn unregister_identity_is_idempotent() {
        let mgr = make_manager();
        let id_a = Identifier::from([1u8; 32]);
        let token_x = Identifier::from([10u8; 32]);

        mgr.register_identity(id_a, [token_x]).await;
        assert!(mgr.state_for_identity(&id_a).await.is_some());

        mgr.unregister_identity(&id_a).await;
        assert!(mgr.state_for_identity(&id_a).await.is_none());

        // Idempotent: calling again on an unknown identity must not panic.
        mgr.unregister_identity(&id_a).await;
        mgr.unregister_identity(&Identifier::from([99u8; 32])).await;
    }

    #[tokio::test]
    async fn unregistered_identity_does_not_persist_fresh_balances() {
        let (mgr, persister) = make_recording_manager();
        let id_a = Identifier::from([1u8; 32]);
        let token_x = Identifier::from([10u8; 32]);
        let mut fresh = BTreeMap::new();
        fresh.insert(token_x, Some(5u64));

        mgr.register_identity(id_a, [token_x]).await;
        mgr.unregister_identity(&id_a).await;
        mgr.apply_fresh_balances(id_a, fresh).await;

        assert_eq!(persister.stores.load(AtomicOrdering::SeqCst), 0);
    }

    /// `set_interval` clamps to >=1s and is read back via `interval`.
    /// Default interval matches the documented constant. Pinned so
    /// future tuning surfaces in the test suite.
    #[tokio::test]
    async fn interval_round_trip() {
        let mgr = make_manager();

        assert_eq!(
            mgr.interval(),
            Duration::from_secs(DEFAULT_SYNC_INTERVAL_SECS)
        );

        mgr.set_interval(Duration::from_secs(0));
        assert_eq!(mgr.interval(), Duration::from_secs(1));

        mgr.set_interval(Duration::from_secs(120));
        assert_eq!(mgr.interval(), Duration::from_secs(120));
    }

    /// `quiesce()` must not return while a pass is in flight, and must
    /// return promptly once the pass drains.
    ///
    /// Drives the real `is_syncing` lifecycle: the test takes the slot
    /// via the same `compare_exchange` the real `sync_now` uses and holds
    /// it (standing in for the pass body + persister fan-out, which
    /// `sync_now` keeps the flag set across), then clears it. We assert
    /// `quiesce()` is still pending while the flag is held and completes
    /// after it falls — i.e. the falling edge of `is_syncing` is what
    /// unblocks the barrier.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn quiesce_blocks_until_in_flight_pass_drains() {
        let mgr = make_manager();

        // Hold the slot as an in-flight pass would, for exactly as long as
        // the test decides — no timer the test could miss on a loaded runner.
        assert!(
            mgr.wedge_sync_slot_for_test(),
            "test should own the is_syncing slot"
        );

        let quiesce_fut = mgr.quiesce();
        tokio::pin!(quiesce_fut);

        // While the pass holds the slot, quiesce must stay pending, with
        // admission already shut.
        assert!(
            futures::poll!(quiesce_fut.as_mut()).is_pending(),
            "quiesce returned while a pass was in flight"
        );
        assert!(mgr.sync_admission_closed());
        assert!(mgr.is_syncing(), "pass should still be in flight");

        // Once the pass drains, quiesce must return, with admission reopened.
        mgr.release_sync_slot_for_test();
        let drained = tokio::time::timeout(Duration::from_secs(2), quiesce_fut)
            .await
            .expect("quiesce did not return after the pass drained");
        assert!(drained);
        assert!(!mgr.sync_admission_closed());
        assert!(!mgr.is_syncing());
    }

    /// A pass that never drains must NOT hang `quiesce_within` forever:
    /// the drain returns `false` at its deadline and deliberately leaves
    /// the `quiescing` gate closed (so the wedged pass cannot be followed by
    /// a fresh one). A later successful quiesce reopens the gate. Sibling
    /// of the dashpay / platform-address regression tests.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn quiesce_within_times_out_and_leaves_gate_up_when_pass_never_drains() {
        let mgr = make_manager();

        assert!(mgr.wedge_sync_slot_for_test());

        let drained = tokio::time::timeout(
            Duration::from_secs(2),
            mgr.quiesce_within(Duration::from_millis(100)),
        )
        .await
        .expect("bounded quiesce must return at its deadline");
        assert!(!drained, "a wedged pass must be reported as non-drained");
        assert!(
            mgr.sync_admission_closed(),
            "gate must stay up after a timed-out drain"
        );

        mgr.release_sync_slot_for_test();
        assert!(mgr.quiesce().await);
        assert!(!mgr.sync_admission_closed());
    }

    /// A `sync_now()` invoked while `quiescing` is set must bail without
    /// running the pass — in particular, without calling
    /// `persister.store(...)`. This is the gate that prevents a pass
    /// from slipping in between `quiesce`'s `stop()` and its drain.
    #[tokio::test]
    async fn sync_now_bails_when_quiescing() {
        let (mgr, persister) = make_recording_manager();
        let id_a = Identifier::from([1u8; 32]);
        let token_x = Identifier::from([10u8; 32]);
        mgr.register_identity(id_a, [token_x]).await;

        // Raise the gate as an in-flight `quiesce()` would (a drain holds
        // the gate from its first instruction).
        let gate_hold = mgr
            .quiesce_held_within(Duration::from_secs(1))
            .await
            .expect("an idle coordinator must drain immediately");

        let summary = mgr.sync_now().await;

        // No persister store happened, and the slot was released so a
        // later (post-quiesce) pass can still run.
        assert_eq!(persister.stores.load(AtomicOrdering::SeqCst), 0);
        assert!(!summary.pass_ran(), "a skipped pass must not report as ran");
        assert!(summary.is_empty());
        assert!(!mgr.is_syncing());
        drop(gate_hold);
    }

    /// A successful read reports the proved balances and the Platform block
    /// they were proved at, so a caller can order them against local writes.
    #[tokio::test]
    async fn should_report_proved_balances_and_block_for_each_identity_read() {
        let id_a = Identifier::from([1u8; 32]);
        let token_x = Identifier::from([10u8; 32]);
        let token_y = Identifier::from([11u8; 32]);
        let query = IdentityTokenBalancesQuery {
            identity_id: id_a,
            token_ids: vec![token_x, token_y],
        };
        let answer: IdentityTokenBalances = [(token_x, Some(7u64)), (token_y, None)]
            .into_iter()
            .collect();
        let (mgr, _persister) = make_answering_manager(vec![(query, answer)]).await;
        mgr.register_identity(id_a, [token_x, token_y]).await;

        let summary = mgr.sync_now().await;

        assert!(summary.pass_ran());
        assert_eq!(summary.success_count(), 1);
        assert_eq!(summary.error_count(), 0);
        let outcome = summary.identity_results.get(&id_a).expect("identity read");
        assert!(outcome.is_ok());
        match outcome.batches.as_slice() {
            [TokenBalanceBatchOutcome::Ok {
                balances,
                proved_at,
            }] => {
                assert_eq!(balances.get(&token_x), Some(&Some(7)));
                assert_eq!(balances.get(&token_y), Some(&None));
                assert_eq!(*proved_at, BlockTime::new(PROVED_HEIGHT, 0, PROVED_TIME_MS));
            }
            other => panic!("expected one proved batch, got {other:?}"),
        }
    }

    /// A failed read is reported with a typed error for the identity, and the
    /// failed identity's cache row keeps its unchecked placeholder.
    #[tokio::test]
    async fn should_report_typed_failure_for_identity_whose_read_fails() {
        let id_a = Identifier::from([1u8; 32]);
        let id_b = Identifier::from([2u8; 32]);
        let token_x = Identifier::from([10u8; 32]);
        let query_a = IdentityTokenBalancesQuery {
            identity_id: id_a,
            token_ids: vec![token_x],
        };
        let answer: IdentityTokenBalances = [(token_x, Some(3u64))].into_iter().collect();
        let (mgr, _persister) = make_answering_manager(vec![(query_a, answer)]).await;
        mgr.register_identity(id_a, [token_x]).await;
        mgr.register_identity(id_b, [token_x]).await;

        let summary = mgr.sync_now().await;

        assert!(summary.pass_ran());
        assert_eq!(summary.success_count(), 1);
        assert_eq!(summary.error_count(), 1);
        assert!(summary.identity_results[&id_a].is_ok());
        let failed = &summary.identity_results[&id_b];
        assert!(!failed.is_ok());
        match failed.batches.as_slice() {
            [TokenBalanceBatchOutcome::Err {
                token_ids,
                error: TokenBalanceReadError::Fetch(_),
            }] => assert_eq!(token_ids, &vec![token_x]),
            other => panic!("expected one failed batch, got {other:?}"),
        }
        let row_b = mgr.state_for_identity(&id_b).await.expect("row kept");
        assert_eq!(row_b.last_sync_unix, 0);
        assert_eq!(row_b.tokens[0].balance, 0);
    }

    /// When one batch of an identity fails and another succeeds, each batch
    /// reports its own outcome, so the failed batch's placeholders are not
    /// mistaken for checked balances (dashpay/platform#5771).
    #[tokio::test]
    async fn should_report_each_batch_when_one_batch_of_an_identity_fails() {
        let id_a = Identifier::from([1u8; 32]);
        let tokens: Vec<Identifier> = (0..=MAX_TOKENS_PER_BALANCE_BATCH)
            .map(|i| {
                let mut bytes = [0u8; 32];
                bytes[..8].copy_from_slice(&(i as u64).to_be_bytes());
                Identifier::from(bytes)
            })
            .collect();
        let (first, rest) = tokens.split_at(MAX_TOKENS_PER_BALANCE_BATCH);
        let query = IdentityTokenBalancesQuery {
            identity_id: id_a,
            token_ids: first.to_vec(),
        };
        let answer: IdentityTokenBalances = first.iter().map(|t| (*t, Some(1u64))).collect();
        let (mgr, _persister) = make_answering_manager(vec![(query, answer)]).await;
        mgr.register_identity(id_a, tokens.clone()).await;

        let summary = mgr.sync_now().await;

        let outcome = &summary.identity_results[&id_a];
        assert!(!outcome.is_ok());
        assert_eq!(summary.error_count(), 1);
        match outcome.batches.as_slice() {
            [TokenBalanceBatchOutcome::Ok { balances, .. }, TokenBalanceBatchOutcome::Err { token_ids, .. }] =>
            {
                assert_eq!(balances.len(), MAX_TOKENS_PER_BALANCE_BATCH);
                assert_eq!(token_ids.as_slice(), rest);
            }
            other => panic!("expected a proved then a failed batch, got {other:?}"),
        }
    }

    /// A pass that ran with nothing to read still reports as ran, unlike a
    /// pass skipped because admission was closed.
    #[tokio::test]
    async fn should_report_pass_with_no_watched_identities_as_ran() {
        let mgr = make_manager();

        let summary = mgr.sync_now().await;

        assert!(summary.pass_ran());
        assert!(summary.is_empty());
    }

    /// Round-trip: register → read → update_watched_tokens → read.
    /// `update_watched_tokens` preserves the rows for tokens still in
    /// the new set, drops removed ones, and inserts placeholders for
    /// added ones. It is a no-op on an unknown identity.
    #[tokio::test]
    async fn update_watched_tokens_round_trip() {
        let mgr = make_manager();
        let id_a = Identifier::from([1u8; 32]);
        let token_x = Identifier::from([10u8; 32]);
        let token_y = Identifier::from([11u8; 32]);
        let token_z = Identifier::from([12u8; 32]);

        mgr.register_identity(id_a, [token_x, token_y]).await;

        // Mutate the row in place to simulate a populated balance for
        // token_x — we want to verify update_watched_tokens preserves
        // it across the swap.
        {
            let mut state = mgr.state.write().await;
            let row = state.get_mut(&id_a).unwrap();
            for info in row.tokens.iter_mut() {
                if info.token_id == token_x {
                    info.balance = 12_345;
                    info.identity_contract_nonce = 7;
                }
            }
        }

        // New set: keep token_x, drop token_y, add token_z.
        mgr.update_watched_tokens(id_a, [token_x, token_z]).await;

        let row = mgr.state_for_identity(&id_a).await.unwrap();
        assert_eq!(row.tokens.len(), 2);

        let by_id: BTreeMap<Identifier, IdentityTokenSyncInfo> =
            row.tokens.iter().map(|t| (t.token_id, *t)).collect();
        // Preserved.
        let kept = by_id.get(&token_x).unwrap();
        assert_eq!(kept.balance, 12_345);
        assert_eq!(kept.identity_contract_nonce, 7);
        // Dropped.
        assert!(!by_id.contains_key(&token_y));
        // Added with placeholders.
        let added = by_id.get(&token_z).unwrap();
        assert_eq!(added.balance, 0);
        assert_eq!(added.identity_contract_nonce, 0);
        assert_eq!(added.contract_id, Identifier::default());

        // Unknown identity: no-op (does not promote to register).
        let unknown = Identifier::from([99u8; 32]);
        mgr.update_watched_tokens(unknown, [token_x]).await;
        assert!(mgr.state_for_identity(&unknown).await.is_none());
    }

    /// A repeated token id is watched once, by `register_identity` and by
    /// `update_watched_tokens`: one balance batch with a repeat fails
    /// proof verification as a whole and would hide every other token in
    /// it.
    #[tokio::test]
    async fn should_watch_a_repeated_token_id_once() {
        let mgr = make_manager();
        let id_a = Identifier::from([1u8; 32]);
        let token_x = Identifier::from([10u8; 32]);
        let token_y = Identifier::from([11u8; 32]);

        mgr.register_identity(id_a, [token_x, token_y, token_x])
            .await;
        let row = mgr.state_for_identity(&id_a).await.unwrap();
        let ids: Vec<Identifier> = row.tokens.iter().map(|t| t.token_id).collect();
        assert_eq!(ids, vec![token_x, token_y]);

        mgr.update_watched_tokens(id_a, [token_y, token_y]).await;
        let row = mgr.state_for_identity(&id_a).await.unwrap();
        let ids: Vec<Identifier> = row.tokens.iter().map(|t| t.token_id).collect();
        assert_eq!(ids, vec![token_y]);
    }

    /// A result for a token that left the watch list while the pass was
    /// reading is not persisted, so a deleted contract's balance is not
    /// written back after the host replaced the watch list.
    #[tokio::test]
    async fn should_not_persist_a_result_for_a_token_no_longer_watched() {
        let (mgr, persister) = make_recording_manager();
        let id_a = Identifier::from([1u8; 32]);
        let token_x = Identifier::from([10u8; 32]);
        let token_y = Identifier::from([11u8; 32]);
        mgr.register_identity(id_a, [token_x]).await;
        // The watch list changed after the pass snapshotted [token_x].
        mgr.register_identity(id_a, [token_y]).await;

        let mut fresh = BTreeMap::new();
        fresh.insert(token_x, Some(5u64));
        mgr.apply_fresh_balances(id_a, fresh).await;

        assert_eq!(persister.stores.load(AtomicOrdering::SeqCst), 0);
        let row = mgr.state_for_identity(&id_a).await.unwrap();
        assert_eq!(row.tokens.len(), 1);
        assert_eq!(row.tokens[0].token_id, token_y);
        assert_eq!(row.tokens[0].balance, 0);
    }

    /// A pass that resumes after shutdown sealed the gate (its drain timed
    /// out) stores nothing, so a retired manager cannot overwrite balances
    /// its replacement wrote.
    #[tokio::test]
    async fn should_not_persist_a_result_after_the_gate_is_sealed() {
        let (mgr, persister) = make_recording_manager();
        let id_a = Identifier::from([1u8; 32]);
        let token_x = Identifier::from([10u8; 32]);
        mgr.register_identity(id_a, [token_x]).await;
        // A pass is still reading when shutdown gives up on the drain.
        assert!(mgr.wedge_sync_slot_for_test());
        assert!(!mgr.quiesce_sealed_within(Duration::from_millis(50)).await);

        let mut fresh = BTreeMap::new();
        fresh.insert(token_x, Some(5u64));
        mgr.apply_fresh_balances(id_a, fresh).await;

        assert_eq!(persister.stores.load(AtomicOrdering::SeqCst), 0);
        mgr.release_sync_slot_for_test();
    }

    /// A quiesce whose barrier cannot take the registry lock in time (a
    /// store stalled inside the persister holds it) reports an incomplete
    /// drain instead of waiting without limit, and keeps admission shut.
    #[tokio::test]
    async fn should_report_a_quiesce_timeout_while_a_store_is_stalled() {
        let mgr = make_manager();
        let stalled_store = mgr.state.write().await;

        let drained = mgr.quiesce_within(Duration::from_millis(50)).await;

        assert!(!drained);
        assert!(mgr.sync_admission_closed());
        drop(stalled_store);
    }

    /// The manual sync waits for a pass in flight and then runs its own, so
    /// a registration that pass snapshotted past is still synced.
    #[tokio::test]
    async fn should_run_a_manual_pass_after_the_pass_in_flight() {
        let mgr = make_manager();
        assert!(mgr.wedge_sync_slot_for_test());
        let releaser = {
            let mgr = Arc::clone(&mgr);
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(60)).await;
                mgr.release_sync_slot_for_test();
            })
        };

        assert!(mgr.sync_now_after_in_flight().await);

        releaser.await.unwrap();
        assert!(mgr.last_sync_unix_seconds().is_some(), "its own pass ran");
        assert!(!mgr.is_syncing());
    }

    /// A manual sync already waiting returns `false` as soon as the gate
    /// closes, well before its wait budget runs out.
    #[tokio::test(start_paused = true)]
    async fn should_report_no_pass_when_the_gate_closes_while_waiting() {
        let mgr = make_manager();
        assert!(mgr.wedge_sync_slot_for_test());
        let started = tokio::time::Instant::now();
        let closer = {
            let mgr = Arc::clone(&mgr);
            tokio::spawn(async move {
                // Close the gate while the manual sync is already waiting.
                tokio::time::sleep(Duration::from_millis(100)).await;
                mgr.seal_sync_admission_for_test();
            })
        };

        assert!(!mgr.sync_now_after_in_flight().await);

        closer.await.unwrap();
        assert!(
            started.elapsed() < COORDINATOR_DRAIN_BUDGET / 2,
            "returned on the gate, not the deadline"
        );
        mgr.release_sync_slot_for_test();
    }

    /// A slot the waiter first sees free only once the wait budget has run
    /// out does not start a pass.
    #[tokio::test(start_paused = true)]
    async fn should_not_start_a_pass_after_the_wait_budget() {
        let mgr = make_manager();
        assert!(mgr.wedge_sync_slot_for_test());
        let releaser = {
            let mgr = Arc::clone(&mgr);
            tokio::spawn(async move {
                // Released between the waiter's last poll before the
                // deadline and its first poll at it, so the waiter first sees
                // the slot free once the budget has run out.
                tokio::time::sleep(COORDINATOR_DRAIN_BUDGET - Duration::from_millis(10)).await;
                mgr.release_sync_slot_for_test();
            })
        };

        assert!(!mgr.sync_now_after_in_flight().await);

        releaser.await.unwrap();
        assert!(mgr.last_sync_unix_seconds().is_none(), "no pass ran");
    }

    /// When another caller takes the slot the moment it frees, the manual
    /// sync waits again and runs its own pass instead of returning while the
    /// other pass is still under way.
    #[tokio::test]
    async fn should_retry_when_another_caller_takes_the_slot_first() {
        let mgr = make_manager();
        assert!(mgr.wedge_sync_slot_for_test());
        let other = {
            let mgr = Arc::clone(&mgr);
            tokio::spawn(async move {
                // Release, then immediately hold the slot again as a second
                // caller would, for another stretch.
                tokio::time::sleep(Duration::from_millis(40)).await;
                mgr.release_sync_slot_for_test();
                assert!(mgr.wedge_sync_slot_for_test());
                tokio::time::sleep(Duration::from_millis(60)).await;
                mgr.release_sync_slot_for_test();
            })
        };

        assert!(matches!(mgr.try_sync_pass().await, SyncAttempt::Busy));
        assert!(mgr.sync_now_after_in_flight().await);

        other.await.unwrap();
        assert!(mgr.last_sync_unix_seconds().is_some(), "its own pass ran");
    }

    /// A store stalled inside the persister holds both the slot and the
    /// registry lock; the quiesce still reports within one budget. Time is
    /// paused, so the bound measures the deadline, not the runner.
    #[tokio::test(start_paused = true)]
    async fn should_report_within_one_budget_while_a_store_holds_slot_and_lock() {
        let mgr = make_manager();
        assert!(mgr.wedge_sync_slot_for_test());
        let stalled_store = mgr.state.write().await;

        let started = tokio::time::Instant::now();
        let drained = mgr.quiesce_within(Duration::from_millis(100)).await;

        assert!(!drained);
        assert!(started.elapsed() < Duration::from_millis(180));
        drop(stalled_store);
        mgr.release_sync_slot_for_test();
    }

    /// A token proved absent (the identity holds none yet) has its stored
    /// row removed but stays watched, so a later pass sees it arrive
    /// without the caller registering it again.
    #[tokio::test]
    async fn should_keep_watching_a_token_proved_absent() {
        let (mgr, persister) = make_recording_manager();
        let id_a = Identifier::from([1u8; 32]);
        let token_x = Identifier::from([10u8; 32]);
        let token_y = Identifier::from([11u8; 32]);
        mgr.register_identity(id_a, [token_x, token_y]).await;

        let mut fresh = BTreeMap::new();
        fresh.insert(token_x, None);
        fresh.insert(token_y, Some(5u64));
        mgr.apply_fresh_balances(id_a, fresh).await;
        assert_eq!(persister.stores.load(AtomicOrdering::SeqCst), 1);

        let row = mgr.state_for_identity(&id_a).await.unwrap();
        let by_id: BTreeMap<Identifier, IdentityTokenSyncInfo> =
            row.tokens.iter().map(|t| (t.token_id, *t)).collect();
        assert_eq!(by_id.get(&token_x).map(|t| t.balance), Some(0));
        assert_eq!(by_id.get(&token_y).map(|t| t.balance), Some(5));

        // The next balance arrives for the still-watched token.
        let mut later = BTreeMap::new();
        later.insert(token_x, Some(7u64));
        mgr.apply_fresh_balances(id_a, later).await;
        let row = mgr.state_for_identity(&id_a).await.unwrap();
        let x = row.tokens.iter().find(|t| t.token_id == token_x).unwrap();
        assert_eq!(x.balance, 7);
    }

    /// Poll until the coordinator has stamped a completed pass.
    async fn wait_for_a_pass(stamped: impl Fn() -> bool, what: &str) {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while !stamped() {
            assert!(tokio::time::Instant::now() < deadline, "{what}");
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    /// Admission stays shut for as long as the drain's guard is held and
    /// is restored when it drops.
    #[tokio::test]
    async fn should_bar_passes_until_the_quiesce_guard_drops() {
        let mgr = make_manager();

        let guard = mgr
            .quiesce_held_within(Duration::from_secs(1))
            .await
            .expect("an idle coordinator must drain immediately");
        assert!(mgr.sync_admission_closed());

        mgr.sync_now().await;
        assert_eq!(
            mgr.last_sync_unix_seconds(),
            None,
            "no pass may run while the guard is held"
        );
        assert!(!mgr.is_syncing(), "the slot must be released on the bail");

        drop(guard);

        assert!(!mgr.sync_admission_closed());
        mgr.sync_now().await;
        assert!(mgr.last_sync_unix_seconds().is_some());
    }

    /// Overlapping holders compose: the gate reopens only when the last
    /// guard drops.
    #[tokio::test]
    async fn should_reopen_admission_only_after_the_last_quiesce_guard_drops() {
        let mgr = make_manager();

        let outer = mgr
            .quiesce_held_within(Duration::from_secs(1))
            .await
            .expect("drain");
        let inner = mgr
            .quiesce_held_within(Duration::from_secs(1))
            .await
            .expect("drain");

        drop(inner);
        assert!(
            mgr.sync_admission_closed(),
            "the outer holder is still mutating; admission must stay shut"
        );

        drop(outer);
        assert!(!mgr.sync_admission_closed());
    }

    /// The shutdown drain seals the gate: no later pass or drain reopens
    /// it.
    #[tokio::test]
    async fn should_never_reopen_admission_after_a_sealed_quiesce() {
        let mgr = make_manager();

        assert!(mgr.quiesce_sealed_within(Duration::from_secs(1)).await);
        assert!(mgr.sync_admission_closed(), "seal must leave the gate shut");

        mgr.sync_now().await;
        assert_eq!(mgr.last_sync_unix_seconds(), None);

        assert!(mgr.quiesce().await);
        assert!(mgr.sync_admission_closed());
        mgr.sync_now().await;
        assert_eq!(mgr.last_sync_unix_seconds(), None);
        assert!(!mgr.is_syncing());
    }

    /// A sealed drain that times out still seals, and reports non-drained.
    #[tokio::test]
    async fn should_seal_admission_even_when_the_sealed_quiesce_times_out() {
        let mgr = make_manager();
        assert!(mgr.wedge_sync_slot_for_test());

        assert!(!mgr.quiesce_sealed_within(Duration::from_millis(50)).await);

        mgr.release_sync_slot_for_test();
        mgr.sync_now().await;
        assert!(
            mgr.sync_admission_closed(),
            "a sealed gate survives the drained-latch reopen"
        );
        assert_eq!(mgr.last_sync_unix_seconds(), None);
    }

    /// A loop started behind a pass a timed-out drain gave up on runs no
    /// pass while that one is wedged, and resumes once it drains.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn should_resume_passes_once_the_wedged_pass_drains_after_start() {
        let mgr = make_manager();

        assert!(mgr.wedge_sync_slot_for_test());
        assert!(!mgr.quiesce_within(Duration::from_millis(50)).await);
        assert!(
            mgr.sync_admission_closed(),
            "timed-out drain latches the gate"
        );

        // The loop's first attempt loses the slot CAS to the wedge; it then
        // polls at `LATCHED_RETRY_INTERVAL`, not the 60 s default.
        Arc::clone(&mgr).start();
        assert!(mgr.is_running());
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(
            mgr.sync_admission_closed(),
            "gate stays latched while wedged"
        );
        assert_eq!(
            mgr.last_sync_unix_seconds(),
            None,
            "no pass may run behind a wedged one"
        );

        mgr.release_sync_slot_for_test();
        wait_for_a_pass(
            || mgr.last_sync_unix_seconds().is_some(),
            "loop never resumed passes after the wedged pass drained",
        )
        .await;
        assert!(!mgr.sync_admission_closed(), "latch is cleared");

        mgr.stop();
        assert!(mgr.quiesce().await);
    }

    /// The wedged pass is already gone when `start()` runs: the start
    /// itself clears the latch.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn should_reopen_a_latched_gate_on_start_when_the_wedged_pass_is_gone() {
        let mgr = make_manager();

        assert!(mgr.wedge_sync_slot_for_test());
        assert!(!mgr.quiesce_within(Duration::from_millis(50)).await);
        mgr.release_sync_slot_for_test();
        assert!(
            mgr.sync_admission_closed(),
            "latch outlives the wedged pass"
        );

        Arc::clone(&mgr).start();
        assert!(!mgr.sync_admission_closed(), "start clears a stale latch");

        wait_for_a_pass(
            || mgr.last_sync_unix_seconds().is_some(),
            "loop never ran a pass after start cleared the latch",
        )
        .await;

        mgr.stop();
        assert!(mgr.quiesce().await);
    }

    /// A one-shot `sync_now` with no loop running clears a latch whose
    /// wedged pass has drained, and not one whose pass is still wedged.
    #[tokio::test]
    async fn should_reopen_a_drained_latch_on_sync_now() {
        let mgr = make_manager();

        assert!(mgr.wedge_sync_slot_for_test());
        assert!(!mgr.quiesce_within(Duration::from_millis(50)).await);
        mgr.sync_now().await;
        assert_eq!(mgr.last_sync_unix_seconds(), None, "bails behind the wedge");
        assert!(mgr.sync_admission_closed(), "latch stays while wedged");

        mgr.release_sync_slot_for_test();
        mgr.sync_now().await;
        assert!(
            mgr.last_sync_unix_seconds().is_some(),
            "a direct pass runs once the wedged pass is gone"
        );
        assert!(
            !mgr.sync_admission_closed(),
            "the direct pass cleared the latch"
        );
    }

    /// The first pass runs as soon as the loop starts, not one interval
    /// later (the default interval is far longer than the wait here).
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn should_run_first_pass_immediately_after_start() {
        let mgr = make_manager();

        Arc::clone(&mgr).start();
        wait_for_a_pass(
            || mgr.last_sync_unix_seconds().is_some(),
            "the loop never ran its first pass",
        )
        .await;

        mgr.stop();
        assert!(mgr.quiesce().await);
    }

    /// A pass over an empty registry still completes and stamps the
    /// coordinator.
    #[tokio::test]
    async fn should_stamp_a_completed_pass() {
        let mgr = make_manager();
        assert_eq!(mgr.last_sync_unix_seconds(), None);

        mgr.sync_now().await;

        assert!(mgr.last_sync_unix_seconds().is_some());
        assert!(!mgr.is_syncing());
    }

    /// A second `sync_now` while a pass holds the slot does nothing: no
    /// timestamp, and the slot it lost stays with its owner.
    #[tokio::test]
    async fn should_skip_sync_now_while_slot_is_held() {
        let mgr = make_manager();
        assert!(mgr.wedge_sync_slot_for_test());

        mgr.sync_now().await;

        assert_eq!(mgr.last_sync_unix_seconds(), None);
        assert!(
            mgr.is_syncing(),
            "the losing call must not release the in-flight pass's slot"
        );
        assert!(!mgr.sync_admission_closed());

        mgr.release_sync_slot_for_test();
        mgr.sync_now().await;
        assert!(mgr.last_sync_unix_seconds().is_some());
    }

    /// A refused `sync_now` behind a closed gate leaves no timestamp.
    #[tokio::test]
    async fn should_not_stamp_sync_now_refused_by_a_shut_gate() {
        let mgr = make_manager();
        let _guard = mgr
            .quiesce_held_within(Duration::from_secs(1))
            .await
            .expect("drain");

        mgr.sync_now().await;

        assert_eq!(mgr.last_sync_unix_seconds(), None);
    }

    /// `start()` on a running loop is a no-op: the loop keeps running, one
    /// `stop()` cancels it, and exactly one thread is there to join.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn should_ignore_start_when_loop_is_running() {
        let (mgr, registry) = make_manager_with_registry();

        Arc::clone(&mgr).start();
        assert!(mgr.is_running());

        Arc::clone(&mgr).start();
        assert!(mgr.is_running());

        mgr.stop();
        assert!(!mgr.is_running(), "one stop cancels the only loop");
        let report = registry.shutdown().await;
        assert!(report.all_clean(), "clean join: {report:?}");
        assert_eq!(report.detached, 0, "no second loop thread was left behind");
    }

    /// A `stop()` followed at once by `start()` leaves the new loop
    /// running and cancellable, and both generations join cleanly.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn should_keep_new_loop_cancellable_after_stop_then_quick_start() {
        let (mgr, registry) = make_manager_with_registry();

        Arc::clone(&mgr).start();
        assert!(mgr.is_running());

        mgr.stop();
        assert!(!mgr.is_running(), "stop() clears the running flag at once");

        Arc::clone(&mgr).start();
        assert!(mgr.is_running(), "loop B must be running after the restart");

        mgr.stop();
        assert!(!mgr.is_running());
        let report = registry.shutdown().await;
        assert!(
            report.all_clean(),
            "clean shutdown after restart: {report:?}"
        );
    }
}
