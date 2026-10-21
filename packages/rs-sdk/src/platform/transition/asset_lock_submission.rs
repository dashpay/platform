//! Submitting asset-lock-funded state transitions: the Platform rejections
//! a caller reacts to, and the retry on a ChainLock proof that is ahead of
//! Platform's view of Core.
//!
//! - [`is_instant_lock_proof_invalid`] — Platform rejected a (stale) islock;
//!   the caller can resubmit a ChainLock proof for the same outpoint.
//! - [`is_asset_lock_already_consumed`] — the exact outpoint was already
//!   spent on Platform.
//! - [`as_asset_lock_proof_cl_height_too_low`] — Platform's consensus 10506
//!   rejection (the proof's ChainLock is ahead of Platform's Core view).
//! - [`submit_with_cl_height_retry`] — retry-on-10506 wrapper that bumps
//!   `user_fee_increase` between attempts so Tenderdash's invalid-tx hash
//!   cache (24h on mainnet/testnet) can't silently drop resubmits.

use std::time::Duration;

use dpp::consensus::basic::identity::InvalidAssetLockProofCoreChainHeightError;
use dpp::consensus::basic::BasicError;
use dpp::consensus::ConsensusError;
use dpp::dashcore::OutPoint;

use super::put_settings::PutSettings;
use crate::Error;

/// Delay between retries when Platform rejected with CL-height-too-low.
/// Each retry bumps `PutSettings::user_fee_increase` so the ST hash
/// changes (Tenderdash caches rejected ST hashes for ~24h on
/// mainnet/testnet — `keep-invalid-txs-in-cache = true` in dashmate's
/// tenderdash template, hardcoded at
/// `packages/dashmate/templates/platform/drive/tenderdash/config.toml.dot:355`).
const CL_HEIGHT_RETRY_DELAY: Duration = Duration::from_secs(15);

/// Total time we'll keep retrying before surfacing the error. Sized to
/// cover Platform's `create-empty-blocks-interval` (3m on mainnet)
/// plus a 30s safety margin: if Platform hasn't observed the wallet's
/// ChainLock by then, the lag is no longer routine and we need
/// operator visibility instead of further silent retries. The
/// rejection error carries Platform's `current_core_chain_locked_height`
/// each round so logs name the laggard node's reported tip explicitly.
const CL_HEIGHT_RETRY_BUDGET: Duration = Duration::from_secs(210);

/// The most submissions one retry run makes: the initial attempt plus one per
/// `CL_HEIGHT_RETRY_DELAY` in the budget. This bounds the fee bumps even when
/// the elapsed-time clock stalls, as a wall clock moved backward does on wasm32.
const CL_HEIGHT_RETRY_MAX_ATTEMPTS: u32 =
    (CL_HEIGHT_RETRY_BUDGET.as_secs() / CL_HEIGHT_RETRY_DELAY.as_secs()) as u32 + 1;

/// Check whether an SDK error indicates that an InstantSend lock proof was
/// rejected by Platform (e.g. the IS lock has expired).
///
/// This matches the `InvalidInstantAssetLockProofSignatureError` consensus
/// error returned by Drive when the instant lock signature cannot be verified
/// (typically because the quorum that signed it has rotated out).
pub fn is_instant_lock_proof_invalid(error: &Error) -> bool {
    matches!(
        error.consensus_error(),
        Some(ConsensusError::BasicError(
            BasicError::InvalidInstantAssetLockProofSignatureError(_),
        ))
    )
}

/// Whether Platform rejected a transition because the exact asset-lock
/// outpoint it submitted has already been consumed.
///
/// Matches the structured consensus error carried by both CheckTx
/// (`Protocol(ConsensusError)`) and wait-stream
/// (`StateTransitionBroadcastError`) failures. The outpoint comparison is
/// deliberate: callers may only recognize a report for the tracked lock they
/// actually submitted, never an unrelated outpoint mentioned by a malformed
/// error. This signal alone does not authenticate terminal consumption.
pub fn is_asset_lock_already_consumed(error: &Error, out_point: &OutPoint) -> bool {
    matches!(
        error.consensus_error(),
        Some(ConsensusError::BasicError(
            BasicError::IdentityAssetLockTransactionOutPointAlreadyConsumedError(e),
        )) if e.transaction_id() == out_point.txid
            && e.output_index() == out_point.vout as usize
    )
}

/// Extract the `InvalidAssetLockProofCoreChainHeightError` (DPP
/// consensus code 10506) from an SDK error if Platform rejected a
/// ChainLock asset-lock proof because Platform's
/// `last_committed_core_height` is still behind the proof's
/// `core_chain_locked_height`.
///
/// Returns `Some(&error)` whenever the rejection matches, exposing
/// `proof_core_chain_locked_height()` (what the wallet claimed) and
/// `current_core_chain_locked_height()` (Platform's currently observed
/// tip). The latter is what the wallet can log to attribute the lag
/// to: small means routine race against Platform's
/// `create-empty-blocks-interval` (3m on mainnet); large or stuck
/// means the DAPI node we hit is genuinely behind / misbehaving.
///
/// Returns `None` for everything else. The check is stateless and
/// re-evaluated on every CheckTx, so a resubmit after Platform
/// catches up will pass — but Tenderdash's mempool caches rejected-tx
/// hashes for ~24h on mainnet/testnet (`keep-invalid-txs-in-cache =
/// true` in dashmate's tenderdash template), so the resubmit must
/// carry a *different* signable-bytes hash to bypass the cache. The
/// submission layer handles that by bumping
/// `PutSettings::user_fee_increase` before re-issuing.
///
/// Companion to [`is_instant_lock_proof_invalid`]; both feed the
/// CL-proof retry path in the identity registration / top-up flow.
///
/// **Coverage caveat:** the consensus error comes from
/// [`Error::consensus_error`], which sees `StateTransitionBroadcastError`
/// (from `broadcast_and_wait`), `Protocol(ProtocolError::ConsensusError)`
/// (from validation) and either of them inside the
/// `NoAvailableAddressesToRetry` envelope. If a future SDK version surfaces
/// the same `InvalidAssetLockProofCoreChainHeightError` through a different
/// variant, the retry helper silently falls through to the passthrough.
/// Re-audit [`Error::consensus_error`] whenever `Error` gains new variants
/// that can carry consensus errors.
pub fn as_asset_lock_proof_cl_height_too_low(
    error: &Error,
) -> Option<&InvalidAssetLockProofCoreChainHeightError> {
    match error.consensus_error() {
        Some(ConsensusError::BasicError(
            BasicError::InvalidAssetLockProofCoreChainHeightError(e),
        )) => Some(e),
        _ => None,
    }
}

/// Submit a state transition with automatic retry on
/// `InvalidAssetLockProofCoreChainHeightError` (consensus code 10506).
///
/// Each retry bumps `settings.user_fee_increase` so the resubmitted ST
/// hashes differently — Tenderdash caches rejected ST hashes for ~24h
/// on mainnet/testnet (`keep-invalid-txs-in-cache = true`), so an
/// identical-bytes resubmit would be silently dropped before reaching
/// Platform's CheckTx.
///
/// **Retry scope.** This wrapper retries ONLY consensus code 10506
/// (CL-height-too-low). Every other `Error` — including
/// transient gRPC `UNAVAILABLE`, DAPI 502/503, RST_STREAM, TLS
/// resets, DNS hiccups, mempool-full bounces — falls through
/// immediately on the first attempt. The rationale: the DAPI client
/// layer (`rs-dapi-client`) below the SDK already implements its own
/// per-request retry + endpoint rotation for transport-level
/// failures, so a second layer of generic retries here would
/// over-retry (or worse, retry an ST submission that the lower
/// layer already retried, against a different validator, leaving
/// two in-flight copies). 10506 is uniquely retried at THIS layer
/// because the fix requires a different `user_fee_increase` value —
/// the lower layer can't know that.
///
/// If the underlying SDK starts surfacing a transient error class
/// that the DAPI client doesn't already retry, widen this match
/// rather than wrapping `submit_with_cl_height_retry` in a second
/// generic-retry loop at the caller.
///
/// We don't pre-flight Platform's chain-lock tip — that's an unproven
/// self-report and a malicious DAPI node could stall us indefinitely.
/// Submit optimistically and react to Platform's deterministic CheckTx
/// rejection. The cryptographic finality guarantee on the wallet side
/// comes from the wallet's own verified ChainLock that promoted the
/// asset-lock tx to chain-locked before the proof was constructed.
///
/// **Trust model.** This function treats the 10506 response as
/// authoritative — there's no client-side cryptographic proof or
/// DAPI-quorum check on the consensus error. That trust boundary
/// lives one layer up: a node that fabricates rejections is a
/// malicious DAPI node, and the right defense is to stop submitting
/// to it (DAPI client rotation / blacklisting), not to engineer
/// around fabricated responses here. Bumping `user_fee_increase` in
/// response to a forged 10506 can grief a user (wasted credits,
/// slowed registration) but cannot extract value — identity fees
/// flow to Platform validators, not DAPI nodes — so the attack is
/// unprofitable. The bounded retry budget further caps the grief
/// impact: at most `CL_HEIGHT_RETRY_BUDGET / CL_HEIGHT_RETRY_DELAY`
/// bumps (~14 with the current 210s/15s pair) before the loop
/// surfaces the error.
///
/// Non-CL-height errors are passed through unchanged. Every rejection
/// is logged with both the proof's claimed height and Platform's
/// currently observed Core tip so persistent lag (>3.5min) attributes
/// to the specific DAPI node we hit and not to a generic timeout.
///
/// **Cancellation:** not cancellation-safe. If the caller drops the
/// returned future mid-sleep, the bumped `user_fee_increase` is lost
/// and any in-flight submission whose response we never consume
/// remains queued in Tenderdash's mempool until it commits or expires.
/// Callers wrapping this in `tokio::select!` with a short timeout
/// should be prepared to either retry (settings reset to original)
/// or accept that Platform may still execute the dropped attempt.
pub async fn submit_with_cl_height_retry<F, Fut, R>(
    mut settings: Option<PutSettings>,
    submit: F,
) -> Result<R, Error>
where
    F: Fn(Option<PutSettings>) -> Fut,
    Fut: std::future::Future<Output = Result<R, Error>>,
{
    let elapsed_since_start = retry_clock();
    let mut attempt: u32 = 0;
    loop {
        attempt += 1;
        match submit(settings).await {
            Ok(r) => return Ok(r),
            Err(e) => {
                let Some(detail) = as_asset_lock_proof_cl_height_too_low(&e) else {
                    return Err(e);
                };
                let elapsed = elapsed_since_start();
                let remaining = CL_HEIGHT_RETRY_BUDGET.saturating_sub(elapsed);
                if remaining.is_zero() || attempt >= CL_HEIGHT_RETRY_MAX_ATTEMPTS {
                    tracing::error!(
                        "Platform rejected ChainLock proof: CL height too low \
                         (proof claimed height={}, Platform tip={}, attempt {}, \
                         elapsed {}s) — retry budget of {}s exhausted; surfacing \
                         error. Platform's reported tip is stuck — likely a lagging \
                         or misbehaving DAPI node.",
                        detail.proof_core_chain_locked_height(),
                        detail.current_core_chain_locked_height(),
                        attempt,
                        elapsed.as_secs(),
                        CL_HEIGHT_RETRY_BUDGET.as_secs(),
                    );
                    return Err(e);
                }
                let sleep_for = remaining.min(CL_HEIGHT_RETRY_DELAY);
                tracing::warn!(
                    "Platform rejected ChainLock proof: CL height too low \
                     (proof claimed height={}, Platform tip={}, attempt {}, \
                     elapsed {}s); bumping user_fee_increase and waiting {}s \
                     before retry",
                    detail.proof_core_chain_locked_height(),
                    detail.current_core_chain_locked_height(),
                    attempt,
                    elapsed.as_secs(),
                    sleep_for.as_secs(),
                );
                settings = Some(bump_user_fee_increase(settings.unwrap_or_default()));
                rs_dapi_client::transport::sleep(sleep_for).await;
            }
        }
    }
}

/// The time since the retry started. Natively this is tokio's clock, which a
/// paused-time test drives. `std::time::Instant` panics in the browser, so on
/// wasm32 it is JavaScript's `Date.now()`, as `internal_cache` reads it.
#[cfg(not(target_arch = "wasm32"))]
fn retry_clock() -> impl Fn() -> Duration {
    let started = tokio::time::Instant::now();
    move || started.elapsed()
}

#[cfg(target_arch = "wasm32")]
fn retry_clock() -> impl Fn() -> Duration {
    let started = js_sys::Date::now();
    move || Duration::from_secs_f64((js_sys::Date::now() - started).max(0.0) / 1000.0)
}

/// Bump `user_fee_increase` by 1 (saturating at `u16::MAX`).
fn bump_user_fee_increase(mut settings: PutSettings) -> PutSettings {
    settings.user_fee_increase = Some(settings.user_fee_increase.unwrap_or(0).saturating_add(1));
    settings
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::ProtocolError;

    /// Fabricate the SDK-side 10506 error shape exactly as
    /// `as_asset_lock_proof_cl_height_too_low` recognizes it. Both the
    /// matcher and the constructor are pinned here so a future SDK refactor
    /// that changes the variant path can't silently desynchronize the retry
    /// helper from its test surface.
    fn fabricate_cl_height_too_low_error() -> Error {
        let consensus =
            ConsensusError::BasicError(BasicError::InvalidAssetLockProofCoreChainHeightError(
                InvalidAssetLockProofCoreChainHeightError::new(
                    /* proof_core_chain_locked_height */ 100,
                    /* current_core_chain_locked_height */ 99,
                ),
            ));
        Error::Protocol(ProtocolError::ConsensusError(Box::new(consensus)))
    }

    /// Pins two load-bearing invariants of `submit_with_cl_height_retry`:
    ///
    /// 1. Every retry under repeated `InvalidAssetLockProofCoreChainHeightError`
    ///    (consensus 10506) receives a `PutSettings::user_fee_increase`
    ///    strictly greater than the previous attempt. The retry's purpose
    ///    is to bypass Tenderdash's 24h invalid-tx hash cache by changing
    ///    the ST signable bytes; if `user_fee_increase` weren't bumped,
    ///    every resubmit would hash identically and be silently dropped.
    ///    This invariant regressed silently once in the earlier
    ///    swift-funding-with-asset-lock series — the test exists so it
    ///    can't regress quietly again.
    ///
    /// 2. After `CL_HEIGHT_RETRY_BUDGET` elapses without a non-10506
    ///    outcome, the helper surfaces the original 10506 error rather
    ///    than looping forever or swallowing it.
    ///
    /// Driven by `#[tokio::test(start_paused = true)]` + manual
    /// `tokio::time::advance` so the retry's `CL_HEIGHT_RETRY_DELAY`
    /// sleeps fire instantly and total test wall time is sub-millisecond.
    #[tokio::test(start_paused = true)]
    async fn should_bump_the_fee_on_each_retry_and_surface_the_error_after_the_budget() {
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;
        use tokio::sync::Mutex;

        // Capture each invocation's `user_fee_increase` (None on the
        // first call, then Some(N) for each retry). Shared `Mutex<Vec>`
        // because the closure is `Fn` and each future is independent.
        let captured: Arc<Mutex<Vec<Option<u16>>>> = Arc::new(Mutex::new(Vec::new()));
        let call_count = Arc::new(AtomicU32::new(0));
        let captured_clone = captured.clone();
        let call_count_clone = call_count.clone();

        // Stub `submit` closure: always returns 10506 so the retry loop
        // exhausts its budget. The helper's return type is generic over
        // `R`; pin `R = ()` for this test (we never reach the success
        // path).
        let submit = move |settings: Option<PutSettings>| {
            let captured = captured_clone.clone();
            let call_count = call_count_clone.clone();
            async move {
                call_count.fetch_add(1, Ordering::SeqCst);
                captured
                    .lock()
                    .await
                    .push(settings.and_then(|s| s.user_fee_increase));
                Err::<(), _>(fabricate_cl_height_too_low_error())
            }
        };

        let result = submit_with_cl_height_retry(None, submit).await;

        // Surfaced error must be the original 10506 — not a wrapper, not
        // a "timeout" type, not None.
        assert!(
            result.is_err(),
            "retry must surface the underlying error on budget exhaust"
        );
        let surfaced_err = result.unwrap_err();
        assert!(
            as_asset_lock_proof_cl_height_too_low(&surfaced_err).is_some(),
            "surfaced error must still be the InvalidAssetLockProofCoreChainHeightError"
        );

        let captured = captured.lock().await;
        let call_n = call_count.load(Ordering::SeqCst);

        // At least 2 attempts (initial + at least one retry); upper
        // bound is `budget / delay` + 1 with a small slack for the
        // boundary check.
        let max_expected =
            (CL_HEIGHT_RETRY_BUDGET.as_secs() / CL_HEIGHT_RETRY_DELAY.as_secs()) as u32 + 2;
        assert!(
            call_n >= 2 && call_n <= max_expected,
            "expected 2..={max_expected} attempts (initial + retries up to budget), got {call_n}"
        );
        assert_eq!(
            captured.len() as u32,
            call_n,
            "every closure invocation should have recorded a fee value"
        );

        // First attempt: caller-supplied `None` settings → user_fee_increase = None.
        assert_eq!(
            captured[0], None,
            "first attempt must use the caller-supplied `None` settings (no bump yet)"
        );

        // Subsequent attempts: strictly increasing `user_fee_increase`,
        // starting from Some(1) and bumping by 1 each retry. The exact
        // values are load-bearing: Tenderdash hashes the full ST bytes
        // including this field, so consecutive identical values would
        // hit the 24h invalid-tx cache.
        for (i, val) in captured.iter().enumerate().skip(1) {
            let expected = Some(i as u16);
            assert_eq!(
                *val, expected,
                "attempt #{i} (1-indexed retry) must carry user_fee_increase = {expected:?}, got {val:?}"
            );
        }
    }

    /// A 10506 that arrives inside the exhausted-address-retry envelope is
    /// still a 10506: the retry bumps the fee and resubmits instead of
    /// surfacing the error on the first attempt.
    #[tokio::test(start_paused = true)]
    async fn should_retry_cl_height_rejection_wrapped_in_exhausted_address_retry() {
        use std::sync::{Arc, Mutex};

        let seen: Arc<Mutex<Vec<Option<u16>>>> = Arc::new(Mutex::new(Vec::new()));
        let seen_in_submit = seen.clone();

        let result = submit_with_cl_height_retry(None, move |settings| {
            let seen = seen_in_submit.clone();
            async move {
                let mut seen = seen.lock().unwrap();
                seen.push(settings.and_then(|s| s.user_fee_increase));
                if seen.len() == 1 {
                    Err(Error::NoAvailableAddressesToRetry(Box::new(
                        fabricate_cl_height_too_low_error(),
                    )))
                } else {
                    Ok(())
                }
            }
        })
        .await;

        assert!(result.is_ok(), "the retry must resubmit and succeed");
        assert_eq!(
            *seen.lock().unwrap(),
            vec![None, Some(1)],
            "the second attempt must carry a bumped user_fee_increase"
        );
    }
}

#[cfg(test)]
mod asset_lock_already_consumed_tests {
    use super::*;
    use crate::error::StateTransitionBroadcastError;
    use dpp::consensus::basic::identity::IdentityAssetLockTransactionOutPointAlreadyConsumedError;
    use dpp::consensus::basic::UnsupportedProtocolVersionError;
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::Txid;
    use dpp::ProtocolError;

    fn out_point() -> OutPoint {
        OutPoint::new(Txid::all_zeros(), 7)
    }

    fn consensus_error() -> ConsensusError {
        let out_point = out_point();
        IdentityAssetLockTransactionOutPointAlreadyConsumedError::new(
            out_point.txid,
            out_point.vout as usize,
        )
        .into()
    }

    fn unrelated_consensus_error() -> ConsensusError {
        UnsupportedProtocolVersionError::new(2, 1).into()
    }

    #[test]
    fn should_recognize_a_protocol_consensus_error_for_the_exact_outpoint() {
        let error = Error::Protocol(ProtocolError::ConsensusError(Box::new(consensus_error())));

        assert!(is_asset_lock_already_consumed(&error, &out_point()));
    }

    #[test]
    fn should_recognize_a_broadcast_consensus_error_for_the_exact_outpoint() {
        let error = Error::StateTransitionBroadcastError(StateTransitionBroadcastError {
            code: 10504,
            message: "asset lock already consumed".to_string(),
            cause: Some(consensus_error()),
        });

        assert!(is_asset_lock_already_consumed(&error, &out_point()));
    }

    #[test]
    fn should_ignore_unrelated_errors_and_different_outpoints() {
        let error = Error::Protocol(ProtocolError::ConsensusError(Box::new(consensus_error())));
        let different_out_point = OutPoint::new(Txid::all_zeros(), 8);

        assert!(!is_asset_lock_already_consumed(
            &error,
            &different_out_point
        ));
        assert!(!is_asset_lock_already_consumed(
            &Error::Generic("boom".to_string()),
            &out_point()
        ));

        let unrelated_protocol = Error::Protocol(ProtocolError::ConsensusError(Box::new(
            unrelated_consensus_error(),
        )));
        assert!(!is_asset_lock_already_consumed(
            &unrelated_protocol,
            &out_point()
        ));

        let unrelated_broadcast =
            Error::StateTransitionBroadcastError(StateTransitionBroadcastError {
                code: 10504,
                // Deliberately resembles the target message: matching must
                // depend on the structured cause, never this display text.
                message: "asset lock output already completely used".to_string(),
                cause: Some(unrelated_consensus_error()),
            });
        assert!(!is_asset_lock_already_consumed(
            &unrelated_broadcast,
            &out_point()
        ));
    }
}
