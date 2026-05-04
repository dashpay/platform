//! Async waiters for e2e test conditions.
//!
//! [`wait_for_balance`] is event-driven on the harness's shared
//! [`super::wait_hub::WaitEventHub`] with a
//! [`BACKSTOP_WAKE_INTERVAL`] safety timeout for idle-chain /
//! no-peer scenarios. [`wait_for`] is the generic polling fallback
//! for conditions that can't hook into the event hub.

use std::future::Future;
use std::time::{Duration, Instant};

use dash_sdk::platform::Fetch;
use dash_sdk::Sdk;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::Identity;
use dpp::prelude::Identifier;

use super::wallet_factory::TestWallet;
use super::{FrameworkError, FrameworkResult};

/// Backstop wake interval for [`wait_for_balance`] — bounds the
/// wall clock when no events arrive (idle chain, no peers).
pub const BACKSTOP_WAKE_INTERVAL: Duration = Duration::from_secs(2);

/// Default poll interval for [`wait_for`].
pub const DEFAULT_POLL_INTERVAL: Duration = Duration::from_millis(500);

/// Generic polling helper for conditions that aren't tied to the
/// event hub.
///
/// Calls `poll` every [`DEFAULT_POLL_INTERVAL`] until it returns
/// `Some(T)` or `timeout` elapses. The current in-flight future is
/// allowed to resolve before the timeout error is returned — no
/// cancellation mid-attempt. Returns
/// [`FrameworkError::Cleanup`] on timeout.
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

/// Wait for `addr`'s balance on `test_wallet` to reach at least
/// `expected`, syncing on every wake.
///
/// Event-driven on [`TestWallet::wait_hub`]; a
/// [`BACKSTOP_WAKE_INTERVAL`] cap keeps idle-chain / no-peer
/// scenarios making progress. Sync errors are logged at `debug` and
/// treated as transient — the next event (or backstop wake) retries.
/// The `Notified` future is captured BEFORE the sync to avoid
/// dropping a notification that fires mid-sync. Returns
/// [`FrameworkError::Cleanup`] on `timeout`.
pub async fn wait_for_balance(
    test_wallet: &TestWallet,
    addr: &PlatformAddress,
    expected: Credits,
    timeout: Duration,
) -> FrameworkResult<()> {
    let start = Instant::now();
    let deadline = Instant::now() + timeout;

    loop {
        // Capture `Notified` BEFORE the sync so a notification
        // arriving mid-sync isn't lost; pin + `as_mut()` lets us
        // re-await the same future across timeouts.
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
        // Backstop wake on idle chains; real activity wakes us
        // earlier via the `Notified` future.
        let cap = std::cmp::min(remaining, BACKSTOP_WAKE_INTERVAL);
        let _ = tokio::time::timeout(cap, notified.as_mut()).await;
    }
}

/// Wait for an on-chain identity balance to reach at least `expected`.
///
/// Polls `Identity::fetch(sdk, identity_id)` every
/// [`BACKSTOP_WAKE_INTERVAL`] and returns the observed balance when
/// it meets the threshold. Network errors during polling are treated
/// as transient (logged at `debug`); a missing identity (the SDK
/// returns `None`) is treated as "not yet visible" and re-polled.
pub async fn wait_for_identity_balance(
    sdk: &Sdk,
    identity_id: Identifier,
    expected: Credits,
    timeout: Duration,
) -> FrameworkResult<Credits> {
    let start = Instant::now();
    let deadline = Instant::now() + timeout;

    loop {
        match Identity::fetch(sdk, identity_id).await {
            Ok(Some(identity)) => {
                let balance = identity.balance();
                if balance >= expected {
                    tracing::info!(
                        target: "platform_wallet::e2e::wait",
                        ?identity_id,
                        observed = balance,
                        expected,
                        elapsed = ?start.elapsed(),
                        "identity balance reached target"
                    );
                    return Ok(balance);
                }
                tracing::debug!(
                    target: "platform_wallet::e2e::wait",
                    ?identity_id,
                    current = balance,
                    expected,
                    "identity balance below target"
                );
            }
            Ok(None) => tracing::debug!(
                target: "platform_wallet::e2e::wait",
                ?identity_id,
                "identity not yet visible on chain"
            ),
            Err(err) => tracing::debug!(
                target: "platform_wallet::e2e::wait",
                error = %err,
                "fetch::<Identity> failed during wait_for_identity_balance"
            ),
        }

        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(FrameworkError::Cleanup(format!(
                "wait_for_identity_balance timed out after {timeout:?} \
                 (identity_id={identity_id:?} expected={expected})"
            )));
        }
        // Cap the sleep against the remaining budget so a sub-2s
        // `timeout` doesn't overshoot by up to `BACKSTOP_WAKE_INTERVAL`.
        tokio::time::sleep(std::cmp::min(remaining, BACKSTOP_WAKE_INTERVAL)).await;
    }
}

/// Wait for a DPNS `<name>.dash` registration to become visible to
/// resolvers.
///
/// Polls [`Sdk::resolve_dpns_name`] every [`BACKSTOP_WAKE_INTERVAL`]
/// until it returns `Some(..)` or the timeout elapses. Returns the
/// resolved owning identity id on success. Test authors typically
/// pair this with the wallet's `register_name_with_external_signer`
/// call so the assertion side of the test waits on observable
/// propagation, not just on the state-transition's broadcast
/// acknowledgement.
pub async fn wait_for_dpns_name_visible(
    sdk: &Sdk,
    name: &str,
    timeout: Duration,
) -> FrameworkResult<Identifier> {
    let start = Instant::now();
    let deadline = Instant::now() + timeout;

    loop {
        match sdk.resolve_dpns_name(name).await {
            Ok(Some(id)) => {
                tracing::info!(
                    target: "platform_wallet::e2e::wait",
                    name,
                    elapsed = ?start.elapsed(),
                    "DPNS name visible"
                );
                return Ok(id);
            }
            Ok(None) => tracing::debug!(
                target: "platform_wallet::e2e::wait",
                name,
                "DPNS name not yet visible"
            ),
            Err(err) => tracing::debug!(
                target: "platform_wallet::e2e::wait",
                name,
                error = %err,
                "DPNS resolve failed during wait_for_dpns_name_visible"
            ),
        }

        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(FrameworkError::Cleanup(format!(
                "wait_for_dpns_name_visible timed out after {timeout:?} (name={name:?})"
            )));
        }
        // Cap the sleep against the remaining budget so a sub-2s
        // `timeout` doesn't overshoot by up to `BACKSTOP_WAKE_INTERVAL`.
        tokio::time::sleep(std::cmp::min(remaining, BACKSTOP_WAKE_INTERVAL)).await;
    }
}
