//! Polling helpers for asynchronous conditions.
//!
//! [`wait_for`] is the generic poller — supply a closure that
//! returns `Some(T)` when the condition is satisfied. The most
//! common specialisation is [`wait_for_balance`]: poll a wallet's
//! address balance until it reaches the expected value or the
//! deadline elapses.

use std::future::Future;
use std::time::{Duration, Instant};

use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;

use super::wallet_factory::TestWallet;
use super::{FrameworkError, FrameworkResult};

/// Default poll interval between attempts. Matches the working
/// baseline used in `dash-evo-tool`'s e2e harness — small enough to
/// keep the test responsive, large enough not to hammer the SDK.
pub const DEFAULT_POLL_INTERVAL: Duration = Duration::from_millis(500);

/// Poll a closure until it returns `Some(T)` or `timeout` elapses.
///
/// The closure is invoked once per round; each invocation returns a
/// future. `wait_for` does NOT cancel the in-flight future when the
/// deadline lapses — it waits for the current attempt to resolve and
/// then returns a timeout error if the deadline has been exceeded
/// and the result was still `None`.
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

/// Poll a wallet's address balance until it reaches at least
/// `expected` or the deadline elapses.
///
/// Each round runs a full `sync_balances` pass and then re-reads the
/// cached balance — the SDK's BLAST sync is the only way to observe
/// new on-chain funds, so polling the cached map without a sync
/// would never see the deposit. Sync errors are logged and treated
/// as transient: the next round retries.
pub async fn wait_for_balance(
    test_wallet: &TestWallet,
    addr: &PlatformAddress,
    expected: Credits,
    timeout: Duration,
) -> FrameworkResult<()> {
    let start = Instant::now();
    let result = wait_for(
        || async {
            if let Err(err) = test_wallet.sync_balances().await {
                tracing::debug!(
                    target: "platform_wallet::e2e::wait",
                    error = %err,
                    "sync_balances during wait_for_balance failed; retrying"
                );
                return None;
            }
            let balances = test_wallet.balances().await;
            let current = balances.get(addr).copied().unwrap_or(0);
            if current >= expected {
                Some(current)
            } else {
                tracing::debug!(
                    target: "platform_wallet::e2e::wait",
                    addr = ?addr,
                    current,
                    expected,
                    "balance below target; polling"
                );
                None
            }
        },
        timeout,
    )
    .await?;

    tracing::info!(
        target: "platform_wallet::e2e::wait",
        addr = ?addr,
        observed = result,
        elapsed = ?start.elapsed(),
        "balance reached target"
    );
    Ok(())
}
