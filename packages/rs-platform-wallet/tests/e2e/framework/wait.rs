//! Polling helpers for asynchronous conditions.
//!
//! [`wait_for`] is the generic poller — supply a closure that
//! returns `Some(T)` when the condition is satisfied. The most
//! common specialisation is [`wait_for_balance`]: poll a wallet's
//! address balance until it reaches the expected value or the
//! deadline elapses.
//!
//! Wave 2 stub. Wave 3 wires the real poll-with-timeout loop and
//! replaces the wallet/address placeholder types.

use std::future::Future;
use std::time::Duration;

use super::{FrameworkError, FrameworkResult};

/// Poll a closure until it returns `Some(T)` or `timeout` elapses.
///
/// The closure is invoked synchronously; each call returns a
/// future that resolves to `Option<T>`. The loop sleeps a small
/// fixed interval between calls (Wave 3 picks the constant — DET's
/// 500 ms is the working baseline).
///
/// Wave 2 stub: returns `NotImplemented` immediately.
pub async fn wait_for<F, Fut, T>(_poll: F, _timeout: Duration) -> FrameworkResult<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Option<T>>,
{
    Err(FrameworkError::NotImplemented(
        "wait::wait_for — wired in Wave 3",
    ))
}

/// Poll a wallet's address balance until it reaches `expected`.
///
/// Wave 2 stub: takes placeholder unit types for the wallet and
/// address slots. Wave 3 replaces them with the real
/// `&framework::wallet_factory::TestWallet` and
/// `&dpp::address_funds::PlatformAddress`.
pub async fn wait_for_balance(
    _test_wallet: &(),
    _addr: &(),
    _expected: u64,
    _timeout: Duration,
) -> FrameworkResult<()> {
    Err(FrameworkError::NotImplemented(
        "wait::wait_for_balance — wired in Wave 3",
    ))
}
