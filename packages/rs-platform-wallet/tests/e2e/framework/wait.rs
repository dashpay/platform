//! Async waiters for e2e test conditions.
//!
//! [`wait_for_balance`] is now event-driven: it awaits on the per-test
//! [`super::wait_hub::WaitEventHub`] (installed as the harness's
//! `PlatformEventHandler`) and only re-runs the BLAST sync when a real
//! SPV / wallet / platform-address-sync event fires. A
//! [`BACKSTOP_WAKE_INTERVAL`] safety timeout still bounds the await so
//! idle-chain / no-peer cases (where no events arrive) still make
//! forward progress.
//!
//! [`wait_for`] remains the generic polling fallback for conditions
//! that can't be hooked to the event hub. Use it sparingly — the
//! event-driven path is both faster and easier on the SDK.

use std::future::Future;
use std::time::{Duration, Instant};

use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;

use super::wallet_factory::TestWallet;
use super::{FrameworkError, FrameworkResult};

/// Backstop wake interval for [`wait_for_balance`].
///
/// `wait_for_balance` is event-driven, but on an idle chain (no peers,
/// nothing happening) no events fire — we still want a re-check every
/// `BACKSTOP_WAKE_INTERVAL` so the loop can observe a balance that the
/// last sync produced and detect timeouts in bounded wall-clock time.
pub const BACKSTOP_WAKE_INTERVAL: Duration = Duration::from_secs(2);

/// Default poll interval used by [`wait_for`]. Matches the working
/// baseline used in `dash-evo-tool`'s e2e harness — small enough to
/// keep the test responsive, large enough not to hammer the SDK.
pub const DEFAULT_POLL_INTERVAL: Duration = Duration::from_millis(500);

/// Generic polling helper kept for conditions that aren't tied to the
/// event hub.
///
/// Polls a closure every [`DEFAULT_POLL_INTERVAL`] until it returns
/// `Some(T)` or `timeout` elapses. The closure is invoked once per
/// round; each invocation returns a future. `wait_for` does NOT cancel
/// the in-flight future when the deadline lapses — it waits for the
/// current attempt to resolve and then returns a timeout error if the
/// deadline has been exceeded and the result was still `None`.
pub async fn wait_for<F, Fut, T>(mut poll: F, timeout: Duration) -> FrameworkResult<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Option<T>>,
{
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(value) = poll().await {
            return Ok(value);
        }
        if Instant::now() >= deadline {
            return Err(FrameworkError::Cleanup(format!(
                "wait_for timed out after {timeout:?}"
            )));
        }
        tokio::time::sleep(DEFAULT_POLL_INTERVAL).await;
    }
}

/// Wait for a wallet's address balance to reach at least `expected`.
///
/// Event-driven: awaits on [`TestWallet::wait_hub`] (the harness's
/// shared `WaitEventHub`) and only re-runs `sync_balances` when the
/// hub fires. A [`BACKSTOP_WAKE_INTERVAL`] timeout caps each await so
/// idle-chain / no-peer scenarios still make progress.
///
/// The function captures a [`tokio::sync::futures::Notified`] BEFORE
/// running the sync — that's the contract that prevents losing a
/// notification arriving mid-sync. Sync errors are logged at `debug`
/// and treated as transient: the next event (or backstop wake) retries.
pub async fn wait_for_balance(
    test_wallet: &TestWallet,
    addr: &PlatformAddress,
    expected: Credits,
    timeout: Duration,
) -> FrameworkResult<()> {
    let start = Instant::now();
    let deadline = Instant::now() + timeout;

    loop {
        // Capture a `Notified` BEFORE polling so a notification
        // arriving mid-sync isn't lost. Pinning + `as_mut()` lets us
        // re-await the same future across `tokio::time::timeout`
        // wakeups inside the loop body without rebuilding it.
        let notified = test_wallet.wait_hub().notified();
        tokio::pin!(notified);

        match test_wallet.sync_balances().await {
            Ok(()) => {
                let balances = test_wallet.balances().await;
                let current = balances.get(addr).copied().unwrap_or(0);
                if current >= expected {
                    tracing::info!(
                        target: "platform_wallet::e2e::wait",
                        addr = ?addr,
                        observed = current,
                        elapsed = ?start.elapsed(),
                        "balance reached target"
                    );
                    return Ok(());
                }
                tracing::debug!(
                    target: "platform_wallet::e2e::wait",
                    addr = ?addr,
                    current,
                    expected,
                    "balance below target; waiting on event hub"
                );
            }
            Err(err) => tracing::debug!(
                target: "platform_wallet::e2e::wait",
                error = %err,
                "sync_balances during wait_for_balance failed; retrying"
            ),
        }

        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(FrameworkError::Cleanup(format!(
                "wait_for_balance timed out after {timeout:?} \
                 (addr={addr:?} expected={expected})"
            )));
        }
        // Backstop: wake at most every `BACKSTOP_WAKE_INTERVAL` even if
        // no events arrive (idle chain, no peers, etc.). Real activity
        // wakes us earlier through the `Notified` future.
        let cap = std::cmp::min(remaining, BACKSTOP_WAKE_INTERVAL);
        let _ = tokio::time::timeout(cap, notified.as_mut()).await;
    }
}
