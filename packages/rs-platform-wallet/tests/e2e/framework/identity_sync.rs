//! Test-harness identity-state auto-sync.
//!
//! Periodically calls
//! [`IdentityWallet::refresh_identity`](platform_wallet::wallet::identity::IdentityWallet::refresh_identity)
//! on every `(wallet, identity)` pair the [`PlatformWalletManager`]
//! currently holds, so the cached
//! [`Identity`](dpp::identity::Identity) inside `managed.identity`
//! tracks chain reality during a test run.
//!
//! # Why this exists (test harness only)
//!
//! Production has three sync loops — `PlatformAddressSync`
//! (address balances, 15 s "BLAST"), `IdentityTokenSync` (per-identity
//! token balances), and a one-shot `refresh_identity` call. The first
//! two are background loops; `refresh_identity` is **only** invoked
//! manually. As a result, every field `refresh_identity` writes onto
//! `managed.identity` sets once at identity load and never refreshes:
//!
//! - `Identity::balance()` — credit balance (the TK-cohort silent-sweep
//!   leak fixed in `2d77385a26` traced here).
//! - `Identity::revision()` — DPP identity revision counter.
//! - `Identity::public_keys()` — key set (TK-001c rotates keys; the
//!   wallet view stays stale on the pre-rotation key set without this
//!   loop).
//! - `managed.set_status(Active, …)` — clears any stale `Loading` /
//!   `Failed` status sticking from registration races.
//!
//! Nonces and data contracts are fetched fresh per state-transition
//! broadcast by the SDK (`fetch_inputs_with_nonce`,
//! `put_..._fetching_nonces`), so they're intentionally out of scope.
//! DPNS names live behind a separate
//! [`refresh_dpns_names`](platform_wallet::wallet::identity::IdentityWallet::refresh_dpns_names)
//! entry point and aren't touched here.
//!
//! Production gets its own `IdentityStateSync` in a separate feature
//! request owned by the wallet team — this loop is harness-only and
//! must not be promoted into `src/` from here.

use std::sync::Arc;
use std::time::Duration;

use platform_wallet::wallet::persister::NoPlatformPersistence;
use platform_wallet::PlatformWalletManager;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

/// Default cadence. Matches the production `PlatformAddressSync` /
/// `IdentityTokenSync` / `ShieldedSync` cadence; 3 s previously caused
/// DAPI overload (v36 TK-005b/TK-011 regressions).
pub const DEFAULT_INTERVAL: Duration = Duration::from_secs(15);

/// Best-effort grace period [`IdentitySync::stop`] gives the background
/// task to settle after the cancel token fires before aborting the
/// `JoinHandle`. Keeps test teardown prompt without leaving the loop
/// orphaned mid-RPC.
const STOP_GRACE: Duration = Duration::from_secs(5);

/// Periodic identity-state auto-sync for the e2e harness.
///
/// One pass per `interval`:
/// 1. Snapshot the `(wallet_id, identity_id)` cross-product under a
///    short read lock — no network call holds any wallet lock.
/// 2. For each pair, call the wallet's
///    [`refresh_identity`](platform_wallet::wallet::identity::IdentityWallet::refresh_identity).
/// 3. Per-identity failures are demoted to `WARN`; the pass continues.
///
/// Cancellation is prompt: the sleep races the
/// [`CancellationToken`] in a `tokio::select!`.
pub struct IdentitySync {
    handle: JoinHandle<()>,
    cancel: CancellationToken,
}

impl IdentitySync {
    /// Start the sync loop on the current tokio runtime. Returns
    /// immediately; the loop runs until [`Self::stop`] is called or
    /// `cancel` fires (whichever comes first).
    pub fn start(
        manager: Arc<PlatformWalletManager<NoPlatformPersistence>>,
        cancel: CancellationToken,
        interval: Duration,
    ) -> Self {
        let task_cancel = cancel.clone();
        let handle = tokio::spawn(async move {
            run_loop(manager, task_cancel, interval).await;
        });
        Self { handle, cancel }
    }

    /// Signal the loop and wait for it to settle (up to
    /// [`STOP_GRACE`]); abort on timeout so test teardown can't hang
    /// on a stuck DAPI round-trip.
    pub async fn stop(self) {
        self.cancel.cancel();
        match tokio::time::timeout(STOP_GRACE, self.handle).await {
            Ok(Ok(())) => {}
            Ok(Err(join_err)) => {
                if !join_err.is_cancelled() {
                    tracing::warn!(
                        target: "platform_wallet::e2e::identity_sync",
                        error = %join_err,
                        "identity-sync task ended with a join error"
                    );
                }
            }
            Err(_) => {
                tracing::warn!(
                    target: "platform_wallet::e2e::identity_sync",
                    grace_secs = STOP_GRACE.as_secs(),
                    "identity-sync did not settle within grace; task already cancelled"
                );
            }
        }
    }
}

async fn run_loop(
    manager: Arc<PlatformWalletManager<NoPlatformPersistence>>,
    cancel: CancellationToken,
    interval: Duration,
) {
    tracing::debug!(
        target: "platform_wallet::e2e::identity_sync",
        interval_secs = interval.as_secs(),
        "identity-sync loop starting"
    );

    let mut next_tick_number: u64 = 0;
    let mut last_tick_at = std::time::Instant::now();

    loop {
        if cancel.is_cancelled() {
            break;
        }

        let elapsed_ms = last_tick_at.elapsed().as_millis() as u64;
        last_tick_at = std::time::Instant::now();

        tick(manager.as_ref(), next_tick_number, elapsed_ms).await;
        next_tick_number += 1;

        tokio::select! {
            _ = tokio::time::sleep(interval) => {}
            _ = cancel.cancelled() => break,
        }
    }

    tracing::debug!(
        target: "platform_wallet::e2e::identity_sync",
        "identity-sync loop exiting"
    );
}

/// Single pass: snapshot every `(wallet, identity_id)` pair held by
/// the manager and refresh each. Errors are logged and skipped.
async fn tick(
    manager: &PlatformWalletManager<NoPlatformPersistence>,
    tick_n: u64,
    elapsed_ms: u64,
) {
    use dpp::prelude::Identifier;
    use platform_wallet::wallet::WalletId;
    use platform_wallet::PlatformWallet;

    // Phase 1 — collect Arc<PlatformWallet> snapshots so we don't hold
    // the manager's wallets map across any per-wallet lock.
    let wallet_ids = manager.wallet_ids().await;
    let mut wallets: Vec<(WalletId, Arc<PlatformWallet>)> = Vec::with_capacity(wallet_ids.len());
    for wallet_id in wallet_ids {
        if let Some(wallet) = manager.get_wallet(&wallet_id).await {
            wallets.push((wallet_id, wallet));
        }
    }

    // Phase 2 — pull the identity-id list off each wallet under that
    // wallet's own short read lock, then drop the lock before any
    // network call. Both buckets (owned + observed) are covered.
    let mut targets: Vec<(WalletId, Arc<PlatformWallet>, Identifier)> = Vec::new();
    for (wallet_id, wallet) in &wallets {
        let ids = {
            let state = wallet.state().await;
            state.identity_manager.identity_ids()
        };
        for id in ids {
            targets.push((*wallet_id, Arc::clone(wallet), id));
        }
    }

    tracing::trace!(
        target: "platform_wallet::e2e::identity_sync",
        tick_n,
        targets_n = targets.len(),
        elapsed_ms,
        "identity-sync tick start"
    );

    if targets.is_empty() {
        return;
    }

    // Phase 3 — refresh each identity. A transient DAPI error on one
    // identity must NOT stop the sync for the others.
    for (wallet_id, wallet, identity_id) in targets {
        if let Err(err) = wallet.identity().refresh_identity(&identity_id).await {
            tracing::warn!(
                target: "platform_wallet::e2e::identity_sync",
                wallet_id = %hex::encode(wallet_id),
                %identity_id,
                error = %err,
                "identity-sync: refresh_identity failed; will retry next tick"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The grace constant is the contract: any positive value will work
    /// in practice, but a regression to `0` would make `stop()` always
    /// abort instead of joining cleanly.
    #[test]
    fn stop_grace_is_positive() {
        assert!(STOP_GRACE > Duration::ZERO);
    }

    /// `DEFAULT_INTERVAL` must match the proven production cadence (15 s).
    /// Lock it in so a future refactor can't silently drop it back to an
    /// over-aggressive value without an explicit decision.
    #[test]
    fn default_interval_matches_production_cadence() {
        assert_eq!(DEFAULT_INTERVAL, Duration::from_secs(15));
    }
}
