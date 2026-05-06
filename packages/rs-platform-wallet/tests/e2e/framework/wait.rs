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
use dash_sdk::query_types::AddressInfo;
use dash_sdk::Sdk;
use dash_spv::sync::ProgressPercentage;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::Identity;
use dpp::prelude::Identifier;
use platform_wallet::SpvRuntime;

use super::bank::BankWallet;
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
/// `expected`, syncing on every wake AND independently verifying the
/// chain-confirmed view via a proof-verified `AddressInfo::fetch`.
///
/// Event-driven on [`TestWallet::wait_hub`]; a
/// [`BACKSTOP_WAKE_INTERVAL`] cap keeps idle-chain / no-peer
/// scenarios making progress. Sync errors are logged at `debug` and
/// treated as transient — the next event (or backstop wake) retries.
/// The `Notified` future is captured BEFORE the sync to avoid
/// dropping a notification that fires mid-sync.
///
/// **Chain-confirmed gate (Marvin QA — three-tests sync race):**
/// once the wallet's local-view balance reaches `expected`, the
/// helper does NOT return immediately. It then polls
/// [`wait_for_address_balance_chain_confirmed`] within the same
/// overall budget so the address is also visible at `>= expected`
/// from the SDK's proof-verified view. The local view's `sync_balances`
/// can return early when one DAPI node has applied the funding block
/// while a sibling node serving the next request hasn't; without the
/// proof-verified gate, the immediately-following
/// `register_identity_from_addresses` lands on the lagging node and
/// the chain returns "Address does not exist" (ID-007 / TK-007) or
/// "Insufficient combined address balances" (DPNS-001) despite the
/// observed local balance. The chain-confirmed gate retries across
/// nodes until a fresh proof actually shows the funded balance,
/// which empirically tracks block replication closely enough that
/// the follow-up state transition's nonce/balance fetch lands on a
/// caught-up node.
///
/// Returns [`FrameworkError::Cleanup`] on `timeout`.
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
                        "balance reached target (local view); confirming on chain"
                    );
                    // Hand off the remaining budget to the
                    // proof-verified gate. If the cross-node
                    // replication lag is real, this is where it
                    // surfaces; if both views already agree, this
                    // returns on the very first poll.
                    let remaining = deadline.saturating_duration_since(Instant::now());
                    return wait_for_address_balance_chain_confirmed(
                        test_wallet.platform_wallet().sdk(),
                        addr,
                        expected,
                        remaining,
                    )
                    .await
                    .map(|_| ());
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

/// Wait for `addr`'s chain-confirmed balance (queried via the SDK's
/// proof-verified [`AddressInfo::fetch`] path) to reach at least
/// `expected`.
///
/// Mirrors [`wait_for_core_balance`]'s "wait on chain-confirmed
/// state" precedent on the Platform side. Where `wait_for_balance`
/// polls the wallet's local cache (which reflects whichever DAPI
/// node `sync_balances` happened to talk to), this helper independently
/// verifies the address's balance via a proof-verified Fetch — the
/// same path the chain itself walks when validating a state
/// transition's input balances. Polls every
/// [`BACKSTOP_WAKE_INTERVAL`] until the threshold is met or `timeout`
/// elapses.
///
/// Returns the observed proof-verified balance on success,
/// [`FrameworkError::Cleanup`] on timeout. Network / proof errors
/// during polling are treated as transient (logged at `debug`); a
/// missing address (Fetch returns `None`) is treated as
/// "not yet visible" and re-polled.
pub async fn wait_for_address_balance_chain_confirmed(
    sdk: &Sdk,
    addr: &PlatformAddress,
    expected: Credits,
    timeout: Duration,
) -> FrameworkResult<Credits> {
    let start = Instant::now();
    let deadline = Instant::now() + timeout;

    loop {
        match AddressInfo::fetch(sdk, *addr).await {
            Ok(Some(info)) => {
                if info.balance >= expected {
                    tracing::info!(
                        target: "platform_wallet::e2e::wait",
                        addr = ?addr,
                        observed = info.balance,
                        expected,
                        elapsed = ?start.elapsed(),
                        "address balance chain-confirmed"
                    );
                    return Ok(info.balance);
                }
                tracing::debug!(
                    target: "platform_wallet::e2e::wait",
                    addr = ?addr,
                    current = info.balance,
                    expected,
                    "chain-confirmed balance below target"
                );
            }
            Ok(None) => tracing::debug!(
                target: "platform_wallet::e2e::wait",
                addr = ?addr,
                "address not yet visible on chain"
            ),
            Err(err) => tracing::debug!(
                target: "platform_wallet::e2e::wait",
                error = %err,
                addr = ?addr,
                "AddressInfo::fetch failed during \
                 wait_for_address_balance_chain_confirmed"
            ),
        }

        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(FrameworkError::Cleanup(format!(
                "wait_for_address_balance_chain_confirmed timed out \
                 after {timeout:?} (addr={addr:?} expected={expected})"
            )));
        }
        // Cap the sleep against the remaining budget so a sub-2s
        // `timeout` doesn't overshoot by up to `BACKSTOP_WAKE_INTERVAL`.
        tokio::time::sleep(std::cmp::min(remaining, BACKSTOP_WAKE_INTERVAL)).await;
    }
}

/// Wait for the wallet's Layer-1 Core "confirmed" balance (in duffs)
/// to reach at least `expected_min`.
///
/// Polls [`TestWallet::core_balance_confirmed`] — the lock-free atomic
/// fed by the SPV path's `WalletBalance::confirmed` — every
/// [`BACKSTOP_WAKE_INTERVAL`] until the threshold is met.
///
/// **Caveat on "confirmed":** at the pinned `key-wallet` revision,
/// `WalletCoreBalance::confirmed` counts mature UTXOs that are *either*
/// in a block *or* InstantSend-locked (per the upstream rustdoc). It
/// excludes pure-mempool UTXOs (those land in `unconfirmed`), but it
/// does NOT distinguish IS-locked-but-unconfirmed from
/// block-confirmed. Mempool-eager returns are still avoided — that's
/// enough to gate `setup_with_core_funded_test_wallet` on a
/// proof-strength UTXO usable for asset-lock construction (CR-003 +).
/// If a future test needs a strictly block-confirmed UTXO (e.g.
/// confirmation-count assertions), that will require either an
/// upstream API change or a sibling helper that consults raw UTXO
/// metadata directly. The SPV feed updates the atomic asynchronously,
/// so polling is sufficient — there's no `Notified` future on the
/// Core side analogous to [`wait_for_balance`]'s wait hub. Returns
/// [`FrameworkError::Cleanup`] on `timeout`.
///
/// On success the success-log line includes a `path` field naming the
/// branch that satisfied the threshold:
/// - `confirmed_or_is_locked` — the confirmed atomic reached the
///   target after at least one poll observed it below. Cannot
///   distinguish in-block vs IS-lock at this layer; see caveat above.
/// - `pre_funded_workdir_cache` — the threshold was already met on the
///   very first poll, before any new SPV activity. Indicates a
///   pre-existing UTXO from a prior run's persisted workdir; if the
///   test relies on a *fresh* funding event this is a false-positive
///   signal and the caller should consider clearing the workdir.
///
/// Used by [`super::setup_with_core_funded_test_wallet`] (positive
/// arrival on the test wallet's BIP-44 account 0) and by `ID-007`
/// (negative pin: identity-auth addresses are NOT in
/// `monitored_addresses()`, so a Core send to one MUST time out
/// here at the pinned `key-wallet` revision).
pub async fn wait_for_core_balance(
    test_wallet: &TestWallet,
    expected_min: u64,
    timeout: Duration,
) -> FrameworkResult<u64> {
    let start = Instant::now();
    let deadline = Instant::now() + timeout;
    let mut polls = 0u64;

    loop {
        let observed = test_wallet.core_balance_confirmed();
        if observed >= expected_min {
            // First-poll success means the threshold was already met
            // before this helper saw any new event — pre-funded
            // workdir cache, not freshly arriving funds. Surface the
            // distinction so post-mortems on suspiciously fast returns
            // (Marvin's QA-002 on CR-003) can tell the two paths apart
            // at a glance.
            let path = if polls == 0 {
                "pre_funded_workdir_cache"
            } else {
                "confirmed_or_is_locked"
            };
            tracing::info!(
                target: "platform_wallet::e2e::wait",
                observed,
                expected_min,
                elapsed = ?start.elapsed(),
                path,
                "core balance reached target"
            );
            return Ok(observed);
        }
        polls += 1;
        tracing::debug!(
            target: "platform_wallet::e2e::wait",
            observed,
            expected_min,
            "core balance below target"
        );

        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(FrameworkError::Cleanup(format!(
                "wait_for_core_balance timed out after {timeout:?} \
                 (expected_min={expected_min})"
            )));
        }
        tokio::time::sleep(std::cmp::min(remaining, BACKSTOP_WAKE_INTERVAL)).await;
    }
}

/// Wait for the bank wallet's confirmed Core (Layer-1) balance to
/// reach at least `min_duffs`.
///
/// Used by the harness right after [`BankWallet::load`] to gate the
/// "ready to issue Core sends" milestone on the SPV compact-filter
/// scan having actually walked far enough to observe the bank's
/// pre-funded UTXOs (Marvin's QA-001 — without this gate, a cold-cache
/// run samples the balance while SPV is still ~52 s into a ~15 min
/// scan and reports `confirmed=0` for an address that's been funded
/// since last week).
///
/// Polls [`BankWallet::core_balance_confirmed`] every
/// [`BACKSTOP_WAKE_INTERVAL`] until the threshold is met. Emits a
/// progress log every [`BANK_FUNDED_PROGRESS_INTERVAL`] including the
/// SPV filter-scan height vs the chain tip — operators can tell
/// "scan at 1.2M of 1.47M, still walking" (alive) from "scan at tip,
/// balance still 0" (real funding problem). Returns the observed
/// balance on success, [`FrameworkError::Cleanup`] on timeout.
pub async fn wait_for_bank_funded(
    bank: &BankWallet,
    spv: Option<&SpvRuntime>,
    min_duffs: u64,
    timeout: Duration,
) -> FrameworkResult<u64> {
    let start = Instant::now();
    let deadline = start + timeout;
    let mut next_progress_log = start + BANK_FUNDED_PROGRESS_INTERVAL;

    loop {
        let observed = bank.core_balance_confirmed();
        if observed >= min_duffs {
            tracing::info!(
                target: "platform_wallet::e2e::wait",
                observed,
                min_duffs,
                elapsed = ?start.elapsed(),
                "bank Core funding gate cleared"
            );
            return Ok(observed);
        }

        let now = Instant::now();
        if now >= next_progress_log {
            log_bank_funded_progress(spv, observed, min_duffs, start.elapsed()).await;
            next_progress_log = now + BANK_FUNDED_PROGRESS_INTERVAL;
        }

        let remaining = deadline.saturating_duration_since(now);
        if remaining.is_zero() {
            log_bank_funded_progress(spv, observed, min_duffs, start.elapsed()).await;
            return Err(FrameworkError::Cleanup(format!(
                "wait_for_bank_funded timed out after {timeout:?} \
                 (observed={observed} duffs, min_duffs={min_duffs})"
            )));
        }
        tokio::time::sleep(std::cmp::min(remaining, BACKSTOP_WAKE_INTERVAL)).await;
    }
}

/// Period between info-level progress lines emitted by
/// [`wait_for_bank_funded`].
pub const BANK_FUNDED_PROGRESS_INTERVAL: Duration = Duration::from_secs(30);

/// One info-level progress line for [`wait_for_bank_funded`]. Pulls
/// the SPV filter-scan height + tip when the runtime is available so
/// the operator can distinguish "scan still walking" from "scan at
/// tip, balance genuinely zero".
async fn log_bank_funded_progress(
    spv: Option<&SpvRuntime>,
    observed: u64,
    target: u64,
    elapsed: Duration,
) {
    let snapshot = match spv {
        Some(rt) => rt.sync_progress().await,
        None => None,
    };
    let filters = snapshot
        .as_ref()
        .and_then(|p| p.filters().ok())
        .map(|f| (f.current_height(), f.target_height()));
    let headers = snapshot
        .as_ref()
        .and_then(|p| p.headers().ok())
        .map(|h| (h.current_height(), h.target_height()));

    match (filters, headers) {
        (Some((scan_height, scan_tip)), _) => tracing::info!(
            target: "platform_wallet::e2e::wait",
            observed,
            target,
            scan_height,
            scan_tip,
            ?elapsed,
            "waiting for bank Core funding (SPV compact-filter scan in progress)"
        ),
        (None, Some((tip, target_tip))) => tracing::info!(
            target: "platform_wallet::e2e::wait",
            observed,
            target,
            header_height = tip,
            header_tip = target_tip,
            ?elapsed,
            "waiting for bank Core funding (filters not yet reporting; headers shown)"
        ),
        (None, None) => tracing::info!(
            target: "platform_wallet::e2e::wait",
            observed,
            target,
            ?elapsed,
            "waiting for bank Core funding (no SPV progress snapshot yet)"
        ),
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
