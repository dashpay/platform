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
//!    forwards it to the persister. The wallet-side write path
//!    (`TokenWallet::sync` mutating `PlatformWalletInfo.token_balances`
//!    directly) is unrelated and untouched.
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

use std::collections::BTreeMap;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use dpp::balances::credits::TokenAmount;
use dpp::prelude::Identifier;
use tokio::sync::RwLock;

use dash_sdk::platform::tokens::identity_token_balances::{
    IdentityTokenBalances, IdentityTokenBalancesQuery,
};
use dash_sdk::platform::FetchMany;

use crate::changeset::{PlatformWalletPersistence, TokenBalanceChangeSet};
use crate::manager::loop_cancel::LoopCancelGuard;
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
    /// Generation-guarded cancel-token slot for the background loop —
    /// see [`LoopCancelGuard`] for the stale-loop shutdown invariant.
    cancel_guard: LoopCancelGuard,
    interval_secs: AtomicU64,
    is_syncing: AtomicBool,
    /// Set by [`quiesce`](Self::quiesce) to gate new passes while it
    /// drains an in-flight one. `sync_now` bails (after taking the
    /// `is_syncing` slot) when this is set, so once `quiesce` observes
    /// `is_syncing == false` no further pass can start — giving shutdown
    /// a real "no more host-visible persister stores" barrier that
    /// cancel-only [`stop`](Self::stop) does not provide.
    quiescing: AtomicBool,
    /// Unix seconds of the last completed pass across all identities.
    /// `0` = never. Identity-level timestamps live on the per-identity
    /// rows in [`IdentitySyncManager::state`].
    last_sync_unix: AtomicU64,
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
    pub fn new(sdk: Arc<dash_sdk::Sdk>, persister: Arc<P>) -> Self {
        Self {
            sdk,
            persister,
            cancel_guard: LoopCancelGuard::new(),
            interval_secs: AtomicU64::new(DEFAULT_SYNC_INTERVAL_SECS),
            is_syncing: AtomicBool::new(false),
            quiescing: AtomicBool::new(false),
            last_sync_unix: AtomicU64::new(0),
            state: RwLock::new(BTreeMap::new()),
        }
    }

    /// Add or replace the registry row for `identity_id`.
    ///
    /// Idempotent — calling with the same identity overwrites the
    /// existing row and resets `last_sync_unix` to `0`. Each token
    /// in `token_ids` becomes a watched row with `balance = 0`,
    /// `contract_id = Identifier::default()`,
    /// `identity_contract_nonce = 0`. The next sync pass populates
    /// real values.
    pub async fn register_identity<I>(&self, identity_id: Identifier, token_ids: I)
    where
        I: IntoIterator<Item = Identifier>,
    {
        let tokens: Vec<IdentityTokenSyncInfo> = token_ids
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
        let new_tokens: Vec<Identifier> = token_ids.into_iter().collect();
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
        let secs = interval.as_secs().max(1);
        self.interval_secs.store(secs, Ordering::Release);
    }

    /// Current polling interval.
    pub fn interval(&self) -> Duration {
        Duration::from_secs(self.interval_secs.load(Ordering::Acquire))
    }

    /// Whether the background loop is currently running.
    pub fn is_running(&self) -> bool {
        self.cancel_guard.is_running()
    }

    /// Whether a sync pass is in flight right now.
    pub fn is_syncing(&self) -> bool {
        self.is_syncing.load(Ordering::Acquire)
    }

    /// Unix seconds of the last completed pass (across all identities),
    /// or `None` if no pass has ever completed.
    pub fn last_sync_unix_seconds(&self) -> Option<u64> {
        match self.last_sync_unix.load(Ordering::Acquire) {
            0 => None,
            n => Some(n),
        }
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
    pub fn start(self: Arc<Self>) {
        let Some((cancel, my_generation)) = self.cancel_guard.install() else {
            return;
        };

        let handle = tokio::runtime::Handle::current();
        let this = self;
        std::thread::Builder::new()
            .name("identity-sync".into())
            .spawn(move || {
                handle.block_on(async move {
                    loop {
                        if cancel.is_cancelled() {
                            break;
                        }

                        this.sync_now().await;

                        let interval = this.interval();
                        tokio::select! {
                            _ = tokio::time::sleep(interval) => {}
                            _ = cancel.cancelled() => break,
                        }
                    }

                    this.cancel_guard.clear_if_current(my_generation);
                });
            })
            .expect("failed to spawn identity-sync thread");
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
        if let Some(token) = self.cancel_guard.take() {
            token.cancel();
        }
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
    /// Mechanism: set the `quiescing` gate so any pass that hasn't yet
    /// taken the `is_syncing` slot bails, cancel the loop, then wait for
    /// `is_syncing` to clear. `is_syncing` is held for the whole pass
    /// including the persister fan-out (`sync_now` clears it only after
    /// every `sync_identity` / `apply_fresh_balances` store completes),
    /// so its falling edge (with the gate up) is a sound "fully drained"
    /// signal. The gate is reopened before returning so a later
    /// start/sync works normally.
    pub async fn quiesce(&self) {
        self.quiescing.store(true, Ordering::Release);
        self.stop();
        while self.is_syncing.load(Ordering::Acquire) {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        self.quiescing.store(false, Ordering::Release);
    }

    /// Run one sync pass across every registered identity.
    ///
    /// If a pass is already in flight, returns immediately without
    /// doing any work (re-entrant safe).
    ///
    /// Iteration order: identities in `Identifier` order, batches in
    /// token-id order. Sequential — no parallelism across batches or
    /// identities — both because the SDK token-fetch futures are
    /// `!Send` (no `tokio::spawn`) and because the design brief
    /// explicitly forbids it.
    pub async fn sync_now(&self) {
        if self
            .is_syncing
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return;
        }

        // A `quiesce()` may have raised the gate between our CAS and
        // here; if so, release the slot and bail without running a pass
        // so the drain can complete and shutdown gets a true barrier
        // (no further `persister.store(...)` after quiesce returns).
        if self.quiescing.load(Ordering::Acquire) {
            self.is_syncing.store(false, Ordering::Release);
            return;
        }

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

        for (identity_id, token_ids) in per_identity {
            self.sync_identity(identity_id, &token_ids).await;
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        self.last_sync_unix.store(now, Ordering::Release);
        self.is_syncing.store(false, Ordering::Release);
    }

    /// Sync a single identity's watched tokens against Platform.
    ///
    /// Splits `token_ids` into batches of at most
    /// [`MAX_TOKENS_PER_BALANCE_BATCH`], issues one
    /// `IdentityTokenBalancesQuery` per batch, builds a
    /// [`TokenBalanceChangeSet`] for the persister, and rewrites this
    /// manager's per-identity cache row.
    ///
    /// Errors are logged and the per-identity row is left at its
    /// previous value rather than being cleared — a transient DAPI
    /// error shouldn't drop the cached balance from under the UI.
    async fn sync_identity(&self, identity_id: Identifier, token_ids: &[Identifier]) {
        if token_ids.is_empty() {
            return;
        }

        // Accumulate balances across batches before touching the
        // cache or the persister — keep batch network calls unlocked.
        let mut fresh_balances: BTreeMap<Identifier, Option<TokenAmount>> = BTreeMap::new();

        for chunk in token_ids.chunks(MAX_TOKENS_PER_BALANCE_BATCH) {
            let query = IdentityTokenBalancesQuery {
                identity_id,
                token_ids: chunk.to_vec(),
            };

            // Type-annotate the call site explicitly: `fetch_many`
            // is generic over the response type, and the inference
            // chain through `RetrievedObjects` doesn't pick a unique
            // implementor without a hint. Same pattern used by
            // `TokenWallet::sync`.
            let fetched: Result<IdentityTokenBalances, _> =
                TokenAmount::fetch_many(self.sdk.as_ref(), query).await;
            match fetched {
                Ok(result) => {
                    for (token_id, maybe_balance) in result.iter() {
                        fresh_balances.insert(*token_id, *maybe_balance);
                    }
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
                }
            }
        }

        if fresh_balances.is_empty() {
            return;
        }

        self.apply_fresh_balances(identity_id, fresh_balances).await;
    }

    async fn apply_fresh_balances(
        &self,
        identity_id: Identifier,
        fresh_balances: BTreeMap<Identifier, Option<TokenAmount>>,
    ) {
        let mut cs = TokenBalanceChangeSet::default();
        for (token_id, maybe_balance) in &fresh_balances {
            let key = (identity_id, *token_id);
            match maybe_balance {
                Some(amount) => {
                    cs.balances.insert(key, *amount);
                }
                None => {
                    cs.removed_balances.insert(key);
                }
            }
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let mut state = self.state.write().await;
        let Some(existing_row) = state.get(&identity_id).cloned() else {
            return;
        };

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
                Some(None) => {}
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

impl<P> std::fmt::Debug for IdentitySyncManager<P>
where
    P: PlatformWalletPersistence + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IdentitySyncManager")
            .field("is_running", &self.is_running())
            .field("is_syncing", &self.is_syncing())
            .field("interval_secs", &self.interval_secs.load(Ordering::Acquire))
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
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let persister = Arc::new(NoopPersister);
        Arc::new(IdentitySyncManager::new(sdk, persister))
    }

    fn make_recording_manager() -> (
        Arc<IdentitySyncManager<RecordingPersister>>,
        Arc<RecordingPersister>,
    ) {
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let persister = Arc::new(RecordingPersister::new());
        (
            Arc::new(IdentitySyncManager::new(sdk, Arc::clone(&persister))),
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
    /// Drives the real `is_syncing` lifecycle: a background task takes
    /// the slot via the same `compare_exchange` the real `sync_now`
    /// uses, holds it across a sleep (standing in for the pass body +
    /// persister fan-out, which `sync_now` keeps the flag set across),
    /// then clears it. We assert `quiesce()` is still pending while the
    /// flag is held and completes after it falls — i.e. the falling edge
    /// of `is_syncing` is what unblocks the barrier.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn quiesce_blocks_until_in_flight_pass_drains() {
        let mgr = make_manager();

        // Stand in for an in-flight `sync_now`: take the `is_syncing`
        // slot exactly as the real pass does, hold it, then release.
        let holder = Arc::clone(&mgr);
        let pass = tokio::spawn(async move {
            assert!(
                holder
                    .is_syncing
                    .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                    .is_ok(),
                "test should own the is_syncing slot"
            );
            tokio::time::sleep(Duration::from_millis(200)).await;
            holder.is_syncing.store(false, Ordering::Release);
        });

        // Give the holder task a chance to take the slot before we
        // start draining.
        while !mgr.is_syncing() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }

        let quiesce_fut = mgr.quiesce();
        tokio::pin!(quiesce_fut);

        // While the pass holds the flag, quiesce must stay pending.
        tokio::select! {
            _ = &mut quiesce_fut => panic!("quiesce returned while a pass was in flight"),
            _ = tokio::time::sleep(Duration::from_millis(50)) => {}
        }
        assert!(mgr.is_syncing(), "pass should still be in flight");

        // Once the pass drains, quiesce must return (well within a
        // generous bound — it polls every 20ms).
        tokio::time::timeout(Duration::from_secs(2), &mut quiesce_fut)
            .await
            .expect("quiesce did not return after the pass drained");

        // The gate is reopened before quiesce returns.
        assert!(!mgr.quiescing.load(Ordering::Acquire));
        assert!(!mgr.is_syncing());
        pass.await.unwrap();
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

        // Raise the gate as `quiesce()` would.
        mgr.quiescing.store(true, Ordering::Release);

        mgr.sync_now().await;

        // No persister store happened, and the slot was released so a
        // later (post-quiesce) pass can still run.
        assert_eq!(persister.stores.load(AtomicOrdering::SeqCst), 0);
        assert!(!mgr.is_syncing());
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
}
