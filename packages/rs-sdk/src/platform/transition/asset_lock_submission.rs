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
//! - [`as_state_transition_max_size_exceeded`] — the transition is over
//!   `max_state_transition_size`.
//! - [`needs_chain_lock_proof`] — whether a failed submit should be retried
//!   with a ChainLock proof for the same outpoint: a stale islock, or an
//!   InstantSend proof that made the transition too large.
//! - [`submit_with_cl_height_retry`] — retry-on-10506 wrapper that bumps
//!   `user_fee_increase` between attempts, so no attempt repeats the bytes
//!   of one Platform refused (see "Refused transitions" below).
//! - [`submit_funding_with_fee_floor`] — the same retry for an identity
//!   funding transition, from the caller's floor and never above
//!   [`MAX_FUNDING_FEE_INCREASE`], telling the caller each attempt's fee
//!   before it is built and what to do next with the lock
//!   ([`FundingFailureKind`]).
//! - [`next_chain_lock_proof`] — the fresh ChainLock proof for a lock whose
//!   fee steps are used up.
//! - [`Sdk::asset_lock_outpoint_state`] — Platform's record of an outpoint,
//!   read with a proof: whether a transition used the lock, and what is left.
//!
//! # Refused transitions
//!
//! Tenderdash keeps the hash of every transition its CheckTx refused
//! (`keep-invalid-txs-in-cache = true` in dashmate's template) with no time
//! limit. The hash leaves a node's cache only when the cache (15000 entries
//! as dashmate configures it) evicts it as the least recently used, and a
//! resubmission refreshes it, or when the node restarts. The mempool's
//! `ttl-duration` (24 h on mainnet and testnet) covers admitted transactions
//! only. A refused transition is never gossiped, so only the node that
//! checked it holds its hash, and that node never admits the same bytes
//! again: Tenderdash 1.8 drops them before CheckTx (DAPI answers that the
//! transition must be created anew), and later releases answer the cached
//! rejection again.
//!
//! So a submission is the pair (proof, `user_fee_increase`), and no pair is
//! sent twice for one lock. Within one proof the fee only rises. Once its
//! steps are used up ([`FundingFailureKind::FeeCapReached`]), the next proof
//! is a ChainLock proof for the same outpoint at a height none carried
//! before ([`next_chain_lock_proof`]). The height is part of the signed
//! bytes, the identity a lock creates derives from the outpoint alone, and
//! Platform accepts any height from the transaction's own up to the Core
//! height it has seen.

use std::future::Future;
use std::time::Duration;

use dpp::asset_lock::reduced_asset_lock_value::{AssetLockValue, AssetLockValueGettersV0};
use dpp::asset_lock::StoredAssetLockInfo;
use dpp::consensus::basic::identity::InvalidAssetLockProofCoreChainHeightError;
use dpp::consensus::basic::state_transition::StateTransitionMaxSizeExceededError;
use dpp::consensus::basic::BasicError;
use dpp::consensus::ConsensusError;
use dpp::dashcore::OutPoint;
use dpp::fee::Credits;
use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
use dpp::prelude::{AssetLockProof, CoreBlockHeight, UserFeeIncrease};
use dpp::serialization::PlatformDeserializableUntrusted;
use dpp::ProtocolError;
use drive::drive::RootTree;
use drive::grovedb::Element;
use drive_proof_verifier::types::{Elements, KeysInPath};

use super::put_settings::PutSettings;
use crate::platform::FetchMany;
use crate::{Error, Sdk};

/// Delay between retries when Platform rejected with CL-height-too-low.
/// Each retry bumps `PutSettings::user_fee_increase` so the ST hash
/// changes: the node that refused the transition keeps its hash
/// (`keep-invalid-txs-in-cache = true` in dashmate's tenderdash template,
/// hardcoded at
/// `packages/dashmate/templates/platform/drive/tenderdash/config.toml.dot:355`;
/// see the module doc).
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

impl Sdk {
    /// Platform's record of the asset lock `out_point` spends, read with a
    /// proof: [`StoredAssetLockInfo::FullyConsumed`] once a transition used
    /// it up, `PartiallyConsumed` with what is left after a transition that
    /// failed past its asset-lock signature check, `NotPresent` before any.
    ///
    /// Drive keeps these records under `SpentAssetLockTransactions` and never
    /// removes one, so `FullyConsumed` is final: no transition can spend the
    /// lock again. It does not say which transition used the lock, or which
    /// identity its credits went to. `NotPresent` holds only as of the
    /// proof's block: a transition still in flight may use the lock later.
    ///
    /// A top-up's own wait cannot tell either: it proves a snapshot of the
    /// identity's balance, which no particular top-up is bound to.
    pub async fn asset_lock_outpoint_state(
        &self,
        out_point: &OutPoint,
    ) -> Result<StoredAssetLockInfo, Error> {
        let key = <[u8; 36]>::from(*out_point).to_vec();
        let query = KeysInPath {
            path: vec![vec![RootTree::SpentAssetLockTransactions as u8]],
            keys: vec![key.clone()],
        };
        let elements = Element::fetch_many(self, query).await?;
        stored_asset_lock_info(elements, &key)
    }
}

/// Decodes a spent-asset-lock record as Drive writes it: an empty item once
/// the lock is used up, its remaining [`AssetLockValue`] otherwise.
fn stored_asset_lock_info(
    mut elements: Elements,
    key: &[u8],
) -> Result<StoredAssetLockInfo, Error> {
    let Some(element) = elements.swap_remove(key).flatten() else {
        return Ok(StoredAssetLockInfo::NotPresent);
    };
    let bytes = element.as_item_bytes().map_err(|e| {
        Error::InvalidProvedResponse(format!(
            "a spent asset lock record that is not an item: {e}"
        ))
    })?;
    if bytes.is_empty() {
        return Ok(StoredAssetLockInfo::FullyConsumed);
    }
    AssetLockValue::deserialize_from_bytes_untrusted_exact(bytes)
        .map(StoredAssetLockInfo::PartiallyConsumed)
        .map_err(|e| {
            Error::InvalidProvedResponse(format!("an undecodable spent asset lock record: {e}"))
        })
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
/// catches up passes, but only with *different* bytes: the node that
/// refused a transition keeps its hash with no time limit
/// (`keep-invalid-txs-in-cache = true` in dashmate's tenderdash template;
/// see the module doc). The submission layer handles that by bumping
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

/// The `StateTransitionMaxSizeExceededError` an SDK error carries, if any:
/// the transition was larger than the protocol's `max_state_transition_size`.
///
/// For an asset-lock-funded transition this is usually the InstantSend proof:
/// it embeds the whole funding transaction plus its InstantLock, about 184
/// bytes per funding input, while a ChainLock proof is a fixed ~40 bytes.
pub fn as_state_transition_max_size_exceeded(
    error: &Error,
) -> Option<&StateTransitionMaxSizeExceededError> {
    match error.consensus_error() {
        Some(ConsensusError::BasicError(BasicError::StateTransitionMaxSizeExceededError(e))) => {
            Some(e)
        }
        _ => None,
    }
}

/// Whether a failed submit with `proof` should be retried with a ChainLock
/// proof for the same outpoint: Platform could not verify the IS lock, or
/// the IS proof made the transition too large and a ChainLock proof in its
/// place brings it under the limit.
///
/// For the size case: an InstantSend proof embeds the whole funding
/// transaction and its InstantLock, so it grows with every funding input,
/// while a ChainLock proof names only the outpoint and a height, so a lock
/// funded by many small inputs can only be consumed once it is chainlocked. A transition that is too
/// large for another reason (its own content) would still be too large
/// with a ChainLock proof, so it is not retried: the retry waits for a
/// ChainLock first, which would only delay the same error.
///
/// Only an InstantSend `proof` qualifies: a ChainLock proof has no smaller
/// proof to give way to, whatever the error says.
pub fn needs_chain_lock_proof(error: &Error, proof: &AssetLockProof) -> bool {
    matches!(proof, AssetLockProof::Instant(_))
        && (is_instant_lock_proof_invalid(error) || chain_lock_proof_fits(error, proof))
}

/// Bytes a transition sheds when it carries a ChainLock proof for the same
/// outpoint instead of `proof`; `None` for a ChainLock proof, which has
/// nothing smaller to give way to.
fn chain_lock_proof_saving(proof: &AssetLockProof) -> Option<u64> {
    let AssetLockProof::Instant(instant) = proof else {
        return None;
    };
    // The ChainLock proof for the same outpoint, at the height whose varint
    // encoding is the longest, since the height it will carry is not known
    // yet.
    let chain_proof = AssetLockProof::Chain(ChainAssetLockProof {
        core_chain_locked_height: u32::MAX,
        out_point: OutPoint::new(instant.transaction().txid(), instant.output_index()),
    });
    Some(encoded_len(proof)?.saturating_sub(encoded_len(&chain_proof)?))
}

/// Whether `error` is a size refusal that the same transition passes with a
/// ChainLock proof in place of `proof`. The proof is only measured for a
/// size refusal.
fn chain_lock_proof_fits(error: &Error, proof: &AssetLockProof) -> bool {
    let Some(exceeded) = as_state_transition_max_size_exceeded(error) else {
        return false;
    };
    chain_lock_proof_saving(proof).is_some_and(|saving| {
        exceeded.actual_size_bytes().saturating_sub(saving) <= exceeded.max_size_bytes()
    })
}

/// The proof's length in a serialized state transition, as DPP measures it.
fn encoded_len(proof: &AssetLockProof) -> Option<u64> {
    proof
        .serialized_len_in_state_transition()
        .ok()
        .map(|len| len as u64)
}

/// Submit a state transition with automatic retry on
/// `InvalidAssetLockProofCoreChainHeightError` (consensus code 10506).
///
/// Each retry bumps `settings.user_fee_increase` so the resubmitted ST
/// hashes differently: the node that refused a transition keeps its hash
/// with no time limit (`keep-invalid-txs-in-cache = true`; see the module
/// doc), so an identical-bytes resubmit would be dropped before reaching
/// Platform's CheckTx. Retries stop once `user_fee_increase` reaches
/// `u16::MAX`, where a bump would repeat it.
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
    settings: Option<PutSettings>,
    submit: F,
) -> Result<R, Error>
where
    F: Fn(Option<PutSettings>) -> Fut,
    Fut: Future<Output = Result<R, Error>>,
{
    retry_cl_height_too_low(settings, UserFeeIncrease::MAX, submit)
        .await
        .map_err(|(error, _)| error)
}

/// The retry behind [`submit_with_cl_height_retry`] and
/// [`submit_funding_with_fee_floor`]: no retry after an attempt whose
/// `user_fee_increase` reached `max_fee`. A failure comes with its attempt's
/// `user_fee_increase`.
async fn retry_cl_height_too_low<F, Fut, R>(
    mut settings: Option<PutSettings>,
    max_fee: UserFeeIncrease,
    submit: F,
) -> Result<R, (Error, UserFeeIncrease)>
where
    F: Fn(Option<PutSettings>) -> Fut,
    Fut: Future<Output = Result<R, Error>>,
{
    let elapsed_since_start = retry_clock();
    let mut attempt: u32 = 0;
    loop {
        attempt += 1;
        let fee = settings
            .and_then(|s| s.user_fee_increase)
            .unwrap_or_default();
        match submit(settings).await {
            Ok(r) => return Ok(r),
            Err(e) => {
                let Some(detail) = as_asset_lock_proof_cl_height_too_low(&e) else {
                    return Err((e, fee));
                };
                let elapsed = elapsed_since_start();
                let remaining = CL_HEIGHT_RETRY_BUDGET.saturating_sub(elapsed);
                if fee >= max_fee {
                    tracing::error!(
                        "Platform rejected ChainLock proof: CL height too low \
                         (proof claimed height={}, Platform tip={}, attempt {}) at \
                         user_fee_increase {}, the most this submission states; \
                         surfacing error",
                        detail.proof_core_chain_locked_height(),
                        detail.current_core_chain_locked_height(),
                        attempt,
                        fee,
                    );
                    return Err((e, fee));
                }
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
                    return Err((e, fee));
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

/// The most `user_fee_increase` an identity funding transition is built with
/// by [`submit_funding_with_fee_floor`]: a percentage added to the processing
/// fee, so at most double it, paid from the asset lock's own credits.
pub const MAX_FUNDING_FEE_INCREASE: UserFeeIncrease = 100;

/// The most transitions one [`submit_funding_with_fee_floor`] call builds:
/// the first attempt and one per retry delay in the retry budget.
pub const FUNDING_ATTEMPTS: u32 = CL_HEIGHT_RETRY_MAX_ATTEMPTS;

/// What the caller does next with the asset lock a funding call submitted.
/// Nothing here proves the lock unconsumed: the caller keeps the lock until
/// a proof settles it, and never assumes that a call it dropped sent nothing.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FundingFailureKind {
    /// Platform could not use the InstantSend proof (a stale islock, or one
    /// that made the transition too large, see [`needs_chain_lock_proof`]):
    /// submit a ChainLock proof for the same outpoint once the lock is
    /// ChainLocked.
    NeedsChainLockProof,
    /// Platform's view of Core stayed below the ChainLock proof's height
    /// (consensus 10506) through every attempt the retry budget allowed, all
    /// below [`MAX_FUNDING_FEE_INCREASE`]. Retry later, one above the last
    /// attempt's fee.
    PlatformBehindCore,
    /// No fee step is left for this proof: the floor is above
    /// [`MAX_FUNDING_FEE_INCREASE`], so nothing was submitted, or the attempt
    /// at it still met 10506. Any pair of this proof and a fee up to the cap
    /// may be held by a node that refused it (see the module doc), so do not
    /// resend this proof at any step: send a ChainLock proof for the same
    /// outpoint at a height it has not carried ([`next_chain_lock_proof`]),
    /// from floor 0. The lock is not settled.
    FeeCapReached,
    /// A node reported this exact outpoint already consumed
    /// ([`is_asset_lock_already_consumed`]). Not authenticated: read
    /// [`Sdk::asset_lock_outpoint_state`], or for a registration the identity
    /// the outpoint derives.
    ReportedConsumed,
    /// Platform refused the transition with another consensus error.
    Rejected,
    /// Nothing settled: a transport failure, a timeout, a response that
    /// failed verification, or bad input. A transition that was sent may
    /// still execute.
    Unavailable,
    /// A top-up whose wait returned a verified snapshot of the identity's
    /// balance, while its lock is not proved used up
    /// ([`Error::AssetLockNotUsedUp`]). The snapshot binds neither the top-up
    /// nor the lock.
    Submitted,
}

/// A funding call that did not settle its lock.
#[derive(Debug)]
pub struct FundingFailure {
    /// What the caller does next with the lock.
    pub kind: FundingFailureKind,
    /// The error the call ended with.
    pub error: Error,
}

/// Why a top-up's asset lock is not proved used up ([`Error::AssetLockNotUsedUp`]).
#[derive(Debug, thiserror::Error)]
pub enum UnsettledLockRecord {
    /// No transition has used the lock yet; one in flight still may.
    #[error("Platform has no record of its use")]
    NotPresent,
    /// A transition that failed after its asset-lock signature checked out
    /// used part of it; these credits can still fund one.
    #[error("{0} of its credits are unspent")]
    PartiallyConsumed(Credits),
    /// The record could not be read.
    #[error("its record could not be read: {0}")]
    Unread(Box<Error>),
}

/// What the caller does next with the lock `proof` spends, after `error`. A
/// 10506 reads as [`FundingFailureKind::PlatformBehindCore`] here: only
/// [`submit_funding_with_fee_floor`], which knows the attempt's fee, reads
/// one at the cap as [`FundingFailureKind::FeeCapReached`].
pub fn funding_failure_kind(error: &Error, proof: &AssetLockProof) -> FundingFailureKind {
    if matches!(error, Error::FeeIncreaseFloorAboveCap { .. }) {
        FundingFailureKind::FeeCapReached
    } else if matches!(error, Error::AssetLockNotUsedUp { .. }) {
        FundingFailureKind::Submitted
    } else if as_asset_lock_proof_cl_height_too_low(error).is_some() {
        FundingFailureKind::PlatformBehindCore
    } else if needs_chain_lock_proof(error, proof) {
        FundingFailureKind::NeedsChainLockProof
    } else if proof
        .out_point()
        .is_some_and(|out_point| is_asset_lock_already_consumed(error, &out_point))
    {
        FundingFailureKind::ReportedConsumed
    } else if is_consensus_refusal(error) {
        FundingFailureKind::Rejected
    } else {
        FundingFailureKind::Unavailable
    }
}

/// Whether Platform refused with a consensus error, CheckTx's or the wait's,
/// decoded or not, looking through the exhausted-retry envelope.
fn is_consensus_refusal(error: &Error) -> bool {
    match error {
        Error::NoAvailableAddressesToRetry(last) => is_consensus_refusal(last),
        Error::Protocol(ProtocolError::ConsensusError(_))
        | Error::StateTransitionBroadcastError(_) => true,
        _ => false,
    }
}

/// Submits the identity funding transition `submit` builds, retrying a 10506
/// as [`submit_with_cl_height_retry`] does: the first attempt at `floor`,
/// each retry one higher, none above [`MAX_FUNDING_FEE_INCREASE`], at most
/// [`FUNDING_ATTEMPTS`]. A floor above the cap submits nothing.
///
/// `on_attempt` gets each attempt's `user_fee_increase` before the attempt is
/// built, so a caller that drops this future still knows the highest one it
/// may have sent. The next call for the same proof starts one above it; once
/// none is left, the next proof is [`next_chain_lock_proof`]'s, from floor 0.
pub async fn submit_funding_with_fee_floor<F, Fut, R>(
    proof: &AssetLockProof,
    floor: UserFeeIncrease,
    on_attempt: impl Fn(UserFeeIncrease),
    submit: F,
) -> Result<R, FundingFailure>
where
    F: Fn(PutSettings) -> Fut,
    Fut: Future<Output = Result<R, Error>>,
{
    if floor > MAX_FUNDING_FEE_INCREASE {
        return Err(FundingFailure {
            kind: FundingFailureKind::FeeCapReached,
            error: Error::FeeIncreaseFloorAboveCap {
                floor,
                cap: MAX_FUNDING_FEE_INCREASE,
            },
        });
    }
    let first = PutSettings {
        user_fee_increase: Some(floor),
        ..Default::default()
    };
    retry_cl_height_too_low(Some(first), MAX_FUNDING_FEE_INCREASE, |settings| {
        let settings = settings.unwrap_or(first);
        on_attempt(settings.user_fee_increase.unwrap_or(floor));
        submit(settings)
    })
    .await
    .map_err(|(error, fee)| {
        let kind = match funding_failure_kind(&error, proof) {
            FundingFailureKind::PlatformBehindCore if fee >= MAX_FUNDING_FEE_INCREASE => {
                FundingFailureKind::FeeCapReached
            }
            kind => kind,
        };
        FundingFailure { kind, error }
    })
}

/// The proof a lock whose fee steps are used up
/// ([`FundingFailureKind::FeeCapReached`]) is submitted with next, from floor
/// 0: a ChainLock proof for `out_point` at the transaction's `mined_height`
/// while no ChainLock proof was sent for it (`last_chain_height` `None`),
/// else one block above the last one sent. Its bytes are then new to every
/// node, and Platform has to reach only one block more than the height it
/// refused. `None` while that height is above `best_chain_locked_height`,
/// the caller's best locally verified ChainLock: wait for a later one.
///
/// The caller has verified that the transaction is mined at `mined_height` on
/// its ChainLocked chain, as for any ChainLock proof, and records the height
/// it sends before sending, so that no height is used twice.
pub fn next_chain_lock_proof(
    out_point: OutPoint,
    mined_height: CoreBlockHeight,
    last_chain_height: Option<CoreBlockHeight>,
    best_chain_locked_height: CoreBlockHeight,
) -> Option<ChainAssetLockProof> {
    let height = match last_chain_height {
        None => mined_height,
        Some(last) => last.checked_add(1)?.max(mined_height),
    };
    (height <= best_chain_locked_height).then_some(ChainAssetLockProof {
        core_chain_locked_height: height,
        out_point,
    })
}

/// A top-up whose wait answered `balance` is settled only by `record`, the
/// proved record of its lock `out_point`, showing the lock used up.
pub(crate) fn settle_top_up(
    out_point: OutPoint,
    balance: Credits,
    record: Result<StoredAssetLockInfo, Error>,
) -> Result<Credits, FundingFailure> {
    let record = match record {
        Ok(StoredAssetLockInfo::FullyConsumed) => return Ok(balance),
        Ok(StoredAssetLockInfo::NotPresent) => UnsettledLockRecord::NotPresent,
        Ok(StoredAssetLockInfo::PartiallyConsumed(value)) => {
            UnsettledLockRecord::PartiallyConsumed(value.remaining_credit_value())
        }
        Err(error) => UnsettledLockRecord::Unread(Box::new(error)),
    };
    Err(FundingFailure {
        kind: FundingFailureKind::Submitted,
        error: Error::AssetLockNotUsedUp { out_point, record },
    })
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
    ///    is to get past Tenderdash's cache of refused hashes by changing
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
        // hit its cache of refused hashes.
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

#[cfg(test)]
mod chain_lock_fallback_tests {
    use dpp::consensus::basic::state_transition::StateTransitionMaxSizeExceededError;
    use dpp::consensus::ConsensusError;
    use dpp::dashcore::{InstantLock, ScriptBuf, Transaction, TxIn};
    use dpp::identity::state_transition::asset_lock_proof::InstantAssetLockProof;

    use super::*;

    /// An IS proof over a funding tx with `n_inputs` signed P2PKH inputs
    /// (148 B each), each outpoint repeated in the InstantLock.
    fn instant_proof(n_inputs: u8) -> AssetLockProof {
        let input: Vec<TxIn> = (0..n_inputs)
            .map(|i| TxIn {
                previous_output: OutPoint::from([i; 36]),
                script_sig: ScriptBuf::from_bytes(vec![0u8; 107]),
                sequence: u32::MAX,
                witness: Default::default(),
            })
            .collect();
        AssetLockProof::Instant(InstantAssetLockProof::new(
            InstantLock {
                inputs: input.iter().map(|i| i.previous_output).collect(),
                ..InstantLock::default()
            },
            Transaction {
                version: 3,
                lock_time: 0,
                input,
                output: Vec::new(),
                special_transaction_payload: None,
            },
            0,
        ))
    }

    fn chain_proof() -> AssetLockProof {
        AssetLockProof::Chain(ChainAssetLockProof {
            core_chain_locked_height: 100,
            out_point: OutPoint::from([11u8; 36]),
        })
    }

    /// The SDK's pre-broadcast refusal of a transition of `actual_size_bytes`.
    fn oversized_error(actual_size_bytes: u64) -> Error {
        ConsensusError::from(StateTransitionMaxSizeExceededError::new(
            actual_size_bytes,
            20_480,
        ))
        .into()
    }

    /// A transition 2.7 KB over the limit with an IS proof over an
    /// 80-input lock fits with a ChainLock proof, so it takes the same
    /// IS→CL path as a rejected IS signature.
    #[test]
    fn should_retry_an_oversized_instant_proof_transition_with_a_chain_lock_proof() {
        assert!(needs_chain_lock_proof(
            &oversized_error(23_168),
            &instant_proof(80)
        ));
    }

    /// The saving is measured against a ChainLock proof for the lock's own
    /// outpoint: output 0's vout takes one varint byte where the null
    /// outpoint's `u32::MAX` takes five, so a transition just over the limit
    /// by the real saving is retried, and one byte more is not.
    #[test]
    fn should_measure_the_chain_lock_proof_with_the_locks_own_outpoint() {
        let proof = instant_proof(1);
        let AssetLockProof::Instant(instant) = &proof else {
            unreachable!("instant_proof builds an InstantSend proof")
        };
        let chain_proof_for = |out_point| {
            AssetLockProof::Chain(ChainAssetLockProof {
                core_chain_locked_height: u32::MAX,
                out_point,
            })
        };
        let own_outpoint = OutPoint::new(instant.transaction().txid(), 0);
        let saving = chain_lock_proof_saving(&proof).unwrap();
        assert_eq!(
            saving,
            encoded_len(&proof).unwrap() - encoded_len(&chain_proof_for(own_outpoint)).unwrap()
        );
        assert!(
            saving
                > encoded_len(&proof).unwrap()
                    - encoded_len(&chain_proof_for(OutPoint::null())).unwrap(),
            "the null outpoint's vout encodes longer than output 0's"
        );

        assert!(needs_chain_lock_proof(
            &oversized_error(20_480 + saving),
            &proof
        ));
        assert!(!needs_chain_lock_proof(
            &oversized_error(20_480 + saving + 1),
            &proof
        ));
    }

    /// A transition too large by its own content stays too large with a
    /// ChainLock proof, so it is not retried after a ChainLock wait.
    #[test]
    fn should_not_retry_when_a_chain_lock_proof_would_still_be_too_large() {
        assert!(!needs_chain_lock_proof(
            &oversized_error(23_168),
            &instant_proof(1)
        ));
    }

    /// A chain proof that is still too large cannot be shrunk by another
    /// chain proof: the error is surfaced, not retried.
    #[test]
    fn should_not_retry_an_oversized_chain_proof_transition() {
        assert!(!needs_chain_lock_proof(
            &oversized_error(20_481),
            &chain_proof()
        ));
    }

    /// A stale-islock rejection reported for a ChainLock proof has nothing
    /// smaller to fall back to.
    #[test]
    fn should_not_retry_a_chain_proof_on_a_stale_islock_error() {
        use dpp::consensus::basic::identity::InvalidInstantAssetLockProofSignatureError;

        let stale_islock: Error = dpp::ProtocolError::ConsensusError(Box::new(
            InvalidInstantAssetLockProofSignatureError::new().into(),
        ))
        .into();

        assert!(needs_chain_lock_proof(&stale_islock, &instant_proof(1)));
        assert!(!needs_chain_lock_proof(&stale_islock, &chain_proof()));
    }

    #[test]
    fn should_not_retry_an_unrelated_error_with_a_chain_lock_proof() {
        let error = Error::Generic("transport failure".to_string());

        assert!(!needs_chain_lock_proof(&error, &instant_proof(80)));
    }
}

#[cfg(test)]
mod asset_lock_outpoint_state_tests {
    use super::*;

    /// A spent-asset-lock record reads as Drive writes it: no record before
    /// any use, an empty item once used up, the remaining value after a
    /// partial use. An element Drive never writes there is refused.
    #[test]
    fn should_read_a_spent_asset_lock_record_as_drive_writes_it() {
        use dpp::serialization::PlatformSerializable;
        use dpp::version::PlatformVersion;

        let key = vec![7u8; 36];
        let read = |records: Vec<(Vec<u8>, Option<Element>)>| {
            stored_asset_lock_info(records.into_iter().collect(), &key)
        };
        let left = AssetLockValue::new(2_000, vec![0x76], 1_500, vec![], PlatformVersion::latest())
            .expect("an asset lock value");

        assert_eq!(read(vec![]).unwrap(), StoredAssetLockInfo::NotPresent);
        assert_eq!(
            read(vec![(key.clone(), None)]).unwrap(),
            StoredAssetLockInfo::NotPresent
        );
        assert_eq!(
            read(vec![(vec![8u8; 36], Some(Element::new_item(vec![])))]).unwrap(),
            StoredAssetLockInfo::NotPresent,
            "another outpoint's record says nothing about this one"
        );
        assert_eq!(
            read(vec![(key.clone(), Some(Element::new_item(vec![])))]).unwrap(),
            StoredAssetLockInfo::FullyConsumed
        );
        assert_eq!(
            read(vec![(
                key.clone(),
                Some(Element::new_item(left.serialize_to_bytes().unwrap()))
            )])
            .unwrap(),
            StoredAssetLockInfo::PartiallyConsumed(left)
        );
        assert!(matches!(
            read(vec![(key.clone(), Some(Element::empty_tree()))]),
            Err(Error::InvalidProvedResponse(_))
        ));
        assert!(matches!(
            read(vec![(key.clone(), Some(Element::new_item(vec![0xff; 3])))]),
            Err(Error::InvalidProvedResponse(_))
        ));
    }
}

// The fakes below answer with the SDK's own `Error`, which is large.
#[cfg(test)]
#[allow(clippy::result_large_err)]
mod funding_tests {
    use std::future::ready;
    use std::sync::Mutex;

    use dpp::consensus::basic::identity::{
        IdentityAssetLockTransactionOutPointAlreadyConsumedError,
        InvalidInstantAssetLockProofSignatureError,
    };
    use dpp::dashcore::{InstantLock, Transaction, Txid};
    use dpp::identity::state_transition::asset_lock_proof::InstantAssetLockProof;
    use dpp::version::PlatformVersion;
    use rs_dapi_client::DapiClientError;
    use tokio::time::Instant;

    use super::*;
    use crate::error::StateTransitionBroadcastError;

    fn consensus(error: ConsensusError) -> Error {
        Error::Protocol(ProtocolError::ConsensusError(Box::new(error)))
    }

    fn cl_height_too_low() -> Error {
        consensus(InvalidAssetLockProofCoreChainHeightError::new(101, 100).into())
    }

    fn chain_proof(vout: u32) -> AssetLockProof {
        AssetLockProof::Chain(ChainAssetLockProof {
            core_chain_locked_height: 100,
            out_point: OutPoint::new(Txid::from_byte_array([4; 32]), vout),
        })
    }

    fn instant_proof() -> AssetLockProof {
        AssetLockProof::Instant(InstantAssetLockProof::new(
            InstantLock::default(),
            Transaction {
                version: 3,
                lock_time: 0,
                input: Vec::new(),
                output: Vec::new(),
                special_transaction_payload: None,
            },
            0,
        ))
    }

    /// Runs [`submit_funding_with_fee_floor`] over `proof` from `floor`, each
    /// attempt answered by `answer` (given its fee): the fee each attempt was
    /// built with, the fees `on_attempt` was told, and the result.
    async fn funded(
        proof: &AssetLockProof,
        floor: UserFeeIncrease,
        answer: impl Fn(UserFeeIncrease) -> Result<u64, Error>,
    ) -> (
        Vec<UserFeeIncrease>,
        Vec<UserFeeIncrease>,
        Result<u64, FundingFailure>,
    ) {
        let built = Mutex::new(Vec::new());
        let told = Mutex::new(Vec::new());
        let result = submit_funding_with_fee_floor(
            proof,
            floor,
            |fee| told.lock().unwrap().push(fee),
            |settings| {
                let fee = settings.user_fee_increase.expect("every attempt states it");
                built.lock().unwrap().push(fee);
                ready(answer(fee))
            },
        )
        .await;
        (
            built.into_inner().unwrap(),
            told.into_inner().unwrap(),
            result,
        )
    }

    /// Each attempt starts at the floor and is one higher than the last, so
    /// none repeats a transition a node refused; the caller is told each fee
    /// before it is built.
    #[tokio::test(start_paused = true)]
    async fn should_raise_the_fee_increase_from_the_floor_on_each_retry() {
        let (built, told, result) = funded(&chain_proof(0), 7, |_| Err(cl_height_too_low())).await;
        assert_eq!(built.len() as u32, FUNDING_ATTEMPTS);
        assert_eq!(built.first(), Some(&7));
        assert!(
            built.windows(2).all(|pair| pair[1] == pair[0] + 1),
            "{built:?}"
        );
        assert_eq!(told, built);
        let failure = result.unwrap_err();
        assert_eq!(failure.kind, FundingFailureKind::PlatformBehindCore);
        assert!(as_asset_lock_proof_cl_height_too_low(&failure.error).is_some());

        // The next call, from one above, never repeats an attempt.
        let last = *built.last().unwrap();
        let (next, _, _) = funded(&chain_proof(0), last + 1, |_| Ok(5)).await;
        assert_eq!(next, vec![last + 1]);
    }

    /// Retries stop at the cap, and a call that leaves no fee step says so
    /// rather than ask for the next floor, which would submit nothing.
    #[tokio::test(start_paused = true)]
    async fn should_stop_funding_retries_at_the_fee_cap() {
        let started = Instant::now();
        let (built, told, result) = funded(&chain_proof(0), MAX_FUNDING_FEE_INCREASE - 3, |_| {
            Err(cl_height_too_low())
        })
        .await;
        assert_eq!(built, vec![97, 98, 99, 100]);
        assert_eq!(told, built);
        let failure = result.unwrap_err();
        assert_eq!(failure.kind, FundingFailureKind::FeeCapReached);
        assert!(as_asset_lock_proof_cl_height_too_low(&failure.error).is_some());
        // Three waits between four attempts, none after the last.
        assert_eq!(started.elapsed(), Duration::from_secs(45));

        // The floor after it submits nothing.
        let (built, told, result) =
            funded(&chain_proof(0), MAX_FUNDING_FEE_INCREASE + 1, |_| Ok(5)).await;
        assert!(built.is_empty() && told.is_empty());
        let failure = result.unwrap_err();
        assert_eq!(failure.kind, FundingFailureKind::FeeCapReached);
        assert!(matches!(
            failure.error,
            Error::FeeIncreaseFloorAboveCap {
                floor: 101,
                cap: 100
            }
        ));

        // Any other answer at the cap is no claim about the fee steps.
        let (built, _, result) = funded(&chain_proof(0), MAX_FUNDING_FEE_INCREASE, |_| {
            Err(Error::TimeoutReached(Duration::from_secs(1), "t".into()))
        })
        .await;
        assert_eq!(built, vec![100]);
        assert_eq!(result.unwrap_err().kind, FundingFailureKind::Unavailable);
    }

    #[tokio::test(start_paused = true)]
    async fn should_report_a_funding_retry_that_lands() {
        let (built, told, result) = funded(&chain_proof(0), 0, |fee| match fee {
            0 => Err(cl_height_too_low()),
            _ => Ok(5),
        })
        .await;
        assert_eq!(built, vec![0, 1]);
        assert_eq!(told, built);
        assert_eq!(result.unwrap(), 5);
    }

    /// The caller learns an attempt's fee before the attempt is built, so a
    /// call it drops while the attempt is in flight still reports it.
    #[tokio::test]
    async fn should_tell_each_attempt_before_it_is_built() {
        let told = Mutex::new(Vec::new());
        let proof = chain_proof(0);
        let call = submit_funding_with_fee_floor(
            &proof,
            4,
            |fee| told.lock().unwrap().push(fee),
            |_| std::future::pending::<Result<u64, Error>>(),
        );
        assert!(
            tokio::time::timeout(Duration::from_millis(10), call)
                .await
                .is_err(),
            "the attempt never answers"
        );
        assert_eq!(told.into_inner().unwrap(), vec![4]);
    }

    /// Only the outpoint-matched report reads as consumed, and nothing but a
    /// proof as done.
    #[test]
    fn should_map_funding_errors_to_what_the_caller_does_next() {
        let consumed = |vout| -> ConsensusError {
            IdentityAssetLockTransactionOutPointAlreadyConsumedError::new(
                Txid::from_byte_array([4; 32]),
                vout,
            )
            .into()
        };
        let broadcast = |cause| {
            Error::StateTransitionBroadcastError(StateTransitionBroadcastError {
                code: 10504,
                message: "asset lock already consumed".to_string(),
                cause,
            })
        };
        let stale_islock = || consensus(InvalidInstantAssetLockProofSignatureError::new().into());
        let out_point = OutPoint::new(Txid::from_byte_array([4; 32]), 1);
        for (error, proof, kind) in [
            (
                cl_height_too_low(),
                chain_proof(1),
                FundingFailureKind::PlatformBehindCore,
            ),
            (
                stale_islock(),
                instant_proof(),
                FundingFailureKind::NeedsChainLockProof,
            ),
            // A ChainLock proof has no smaller proof to fall back to.
            (stale_islock(), chain_proof(1), FundingFailureKind::Rejected),
            (
                consensus(consumed(1)),
                chain_proof(1),
                FundingFailureKind::ReportedConsumed,
            ),
            (
                broadcast(Some(consumed(1))),
                chain_proof(1),
                FundingFailureKind::ReportedConsumed,
            ),
            // Another outpoint, or a bare code, is no report about this lock.
            (
                consensus(consumed(2)),
                chain_proof(1),
                FundingFailureKind::Rejected,
            ),
            (
                broadcast(None),
                chain_proof(1),
                FundingFailureKind::Rejected,
            ),
            (
                Error::NoAvailableAddressesToRetry(Box::new(broadcast(None))),
                chain_proof(1),
                FundingFailureKind::Rejected,
            ),
            (
                Error::DapiClientError(DapiClientError::NoAvailableAddresses),
                chain_proof(1),
                FundingFailureKind::Unavailable,
            ),
            (
                Error::TimeoutReached(Duration::from_secs(1), "t".into()),
                chain_proof(1),
                FundingFailureKind::Unavailable,
            ),
            (
                Error::InvalidProvedResponse("x".into()),
                chain_proof(1),
                FundingFailureKind::Unavailable,
            ),
            (
                Error::Generic("x".into()),
                chain_proof(1),
                FundingFailureKind::Unavailable,
            ),
            (
                Error::FeeIncreaseFloorAboveCap {
                    floor: 101,
                    cap: 100,
                },
                chain_proof(1),
                FundingFailureKind::FeeCapReached,
            ),
            (
                Error::AssetLockNotUsedUp {
                    out_point,
                    record: UnsettledLockRecord::NotPresent,
                },
                chain_proof(1),
                FundingFailureKind::Submitted,
            ),
        ] {
            assert_eq!(funding_failure_kind(&error, &proof), kind, "{error}");
        }
    }

    /// A top-up settles only on a record showing its lock used up. Any other
    /// record, or one that could not be read, leaves it submitted, with the
    /// read's own error kept.
    #[test]
    fn should_settle_a_top_up_only_on_a_record_of_its_lock_used_up() {
        let out_point = OutPoint::new(Txid::from_byte_array([4; 32]), 1);
        let unsettled = |record| {
            let failure = settle_top_up(out_point, 5, record).expect_err("not settled");
            assert_eq!(failure.kind, FundingFailureKind::Submitted);
            failure.error
        };

        assert_eq!(
            settle_top_up(out_point, 5, Ok(StoredAssetLockInfo::FullyConsumed)).unwrap(),
            5
        );
        let error = unsettled(Ok(StoredAssetLockInfo::NotPresent));
        assert!(
            error.to_string().ends_with(&format!(
                "asset lock {out_point} is not proved used up: Platform has no record of its use"
            )),
            "{error}"
        );
        let left = AssetLockValue::new(2_000, vec![0x76], 1_500, vec![], PlatformVersion::latest())
            .expect("an asset lock value");
        let error = unsettled(Ok(StoredAssetLockInfo::PartiallyConsumed(left)));
        assert!(
            matches!(
                error,
                Error::AssetLockNotUsedUp {
                    record: UnsettledLockRecord::PartiallyConsumed(1_500),
                    ..
                }
            ),
            "{error}"
        );
        assert!(error
            .to_string()
            .contains("1500 of its credits are unspent"));
        let error = unsettled(Err(Error::TimeoutReached(
            Duration::from_secs(1),
            "down".into(),
        )));
        assert!(
            matches!(
                &error,
                Error::AssetLockNotUsedUp {
                    record: UnsettledLockRecord::Unread(read),
                    ..
                } if matches!(**read, Error::TimeoutReached(..))
            ),
            "{error}"
        );
        assert!(error.to_string().contains("its record could not be read"));
    }

    /// The next proof is at the mined height until a ChainLock proof was
    /// sent, then one block above the last, never below the mined height,
    /// and only once a ChainLock the caller verified covers it.
    #[test]
    fn should_give_the_next_chain_lock_proof_one_block_above_the_last() {
        let out_point = OutPoint::new(Txid::from_byte_array([4; 32]), 1);
        let height = |last, best| {
            next_chain_lock_proof(out_point, 1_000, last, best).map(|proof| {
                assert_eq!(proof.out_point, out_point);
                proof.core_chain_locked_height
            })
        };
        assert_eq!(height(None, 1_000), Some(1_000));
        assert_eq!(height(None, 999), None);
        assert_eq!(height(Some(1_000), 1_005), Some(1_001));
        assert_eq!(height(Some(1_000), 1_000), None);
        assert_eq!(height(Some(1_003), 1_005), Some(1_004));
        assert_eq!(height(Some(900), 1_005), Some(1_000));
        assert_eq!(height(Some(u32::MAX), u32::MAX), None);
    }
}

#[cfg(test)]
#[allow(clippy::result_large_err)]
mod refused_transition_tests {
    //! The recovery contract against nodes that keep every transition their
    //! CheckTx refused, keyed as Tenderdash keys its cache: by the SHA-256 of
    //! the transition's bytes, here those of a real IdentityTopUp signed by
    //! the deterministic signer the SDK uses.

    use std::collections::HashSet;
    use std::sync::Mutex;

    use dpp::dashcore::hashes::sha256;
    use dpp::dashcore::Txid;
    use dpp::identity::Identity;
    use dpp::serialization::PlatformSerializable;
    use dpp::state_transition::identity_topup_transition::methods::IdentityTopUpTransitionMethodsV0;
    use dpp::state_transition::identity_topup_transition::IdentityTopUpTransition;
    use dpp::version::PlatformVersion;
    use dpp::ProtocolError;
    use rs_dapi_client::transport::TransportError;
    use rs_dapi_client::DapiClientError;

    use super::*;

    /// What a node answers to bytes whose hash it holds.
    #[derive(Clone, Copy, Debug)]
    enum Hit {
        /// Tenderdash 1.8 with rs-dapi: the duplicate's CheckTx passes, and
        /// DAPI answers Internal, which every node holding the hash repeats.
        Internal,
        /// Tenderdash after #1530: the cached rejection again.
        CachedRejection,
    }

    /// Platform's nodes, all holding the hashes of the transitions their
    /// CheckTx refused, with Platform's view of Core at `core_height`.
    struct Network {
        hit: Hit,
        core_height: Mutex<CoreBlockHeight>,
        refused: Mutex<HashSet<[u8; 32]>>,
        /// The (proof height, fee) pairs that reached CheckTx.
        checked: Mutex<Vec<(CoreBlockHeight, UserFeeIncrease)>>,
    }

    impl Network {
        fn new(hit: Hit, core_height: CoreBlockHeight) -> Self {
            Self {
                hit,
                core_height: Mutex::new(core_height),
                refused: Mutex::new(HashSet::new()),
                checked: Mutex::new(Vec::new()),
            }
        }

        /// The top-up of one identity from `proof` at `fee`, as it is sent.
        fn top_up(proof: &ChainAssetLockProof, fee: UserFeeIncrease) -> Vec<u8> {
            let version = PlatformVersion::latest();
            let identity =
                Identity::create_basic_identity([3; 32].into(), version).expect("an identity");
            IdentityTopUpTransition::try_from_identity_with_private_key(
                &identity,
                AssetLockProof::Chain(proof.clone()),
                &[0x42; 32],
                fee,
                version,
                None,
            )
            .expect("the top-up is signed")
            .serialize_to_bytes()
            .expect("the top-up serializes")
        }

        fn submit(&self, proof: &ChainAssetLockProof, fee: UserFeeIncrease) -> Result<(), Error> {
            let key = sha256::Hash::hash(&Self::top_up(proof, fee)).to_byte_array();
            let height = proof.core_chain_locked_height;
            if self.refused.lock().unwrap().contains(&key) {
                return Err(match self.hit {
                    Hit::Internal => Error::from(DapiClientError::NoAvailableAddressesToRetry(
                        Box::new(TransportError::Grpc(dapi_grpc::tonic::Status::internal(
                            "State Transition processing error: the state transition is \
                                 valid, create a new state transition with different hash",
                        ))),
                    )),
                    Hit::CachedRejection => cl_height_too_low(height, height - 1),
                });
            }
            self.checked.lock().unwrap().push((height, fee));
            let core_height = *self.core_height.lock().unwrap();
            if height > core_height {
                self.refused.lock().unwrap().insert(key);
                return Err(cl_height_too_low(height, core_height));
            }
            Ok(())
        }

        /// One funding call from `floor`: the fees it built, and its result.
        async fn fund(
            &self,
            proof: &ChainAssetLockProof,
            floor: UserFeeIncrease,
        ) -> (Vec<UserFeeIncrease>, Result<(), FundingFailure>) {
            let built = Mutex::new(Vec::new());
            let result = submit_funding_with_fee_floor(
                &AssetLockProof::Chain(proof.clone()),
                floor,
                |fee| built.lock().unwrap().push(fee),
                |settings| {
                    let fee = settings.user_fee_increase.expect("every attempt states it");
                    std::future::ready(self.submit(proof, fee))
                },
            )
            .await;
            (built.into_inner().unwrap(), result)
        }

        /// Calls from floor 0 up, each from one above the last fee the one
        /// before built, until one leaves no fee step or lands: the kinds of
        /// the failures on the way, and the last call's result.
        async fn fund_until_capped(
            &self,
            proof: &ChainAssetLockProof,
        ) -> (Vec<FundingFailureKind>, Result<(), FundingFailure>) {
            let (mut kinds, mut floor) = (Vec::new(), 0);
            loop {
                let (built, result) = self.fund(proof, floor).await;
                match &result {
                    Err(failure) if failure.kind != FundingFailureKind::FeeCapReached => {
                        kinds.push(failure.kind);
                        floor = built.last().expect("a call below the cap builds") + 1;
                    }
                    _ => return (kinds, result),
                }
            }
        }
    }

    fn cl_height_too_low(proof: CoreBlockHeight, platform: CoreBlockHeight) -> Error {
        Error::Protocol(ProtocolError::ConsensusError(Box::new(
            InvalidAssetLockProofCoreChainHeightError::new(proof, platform).into(),
        )))
    }

    /// A lock capped at proof height h while Platform was behind is never
    /// funded by that proof again, at any fee, after Platform catches up: the
    /// nodes hold every pair, and none reaches CheckTx. The proof at h+1, from
    /// floor 0, lands at its first attempt.
    #[tokio::test(start_paused = true)]
    async fn should_fund_a_capped_lock_only_with_a_fresh_chain_lock_proof() {
        let out_point = OutPoint::new(Txid::from_byte_array([4; 32]), 1);
        let mined = 1_000;
        for hit in [Hit::Internal, Hit::CachedRejection] {
            let network = Network::new(hit, mined - 1);
            let proof = next_chain_lock_proof(out_point, mined, None, mined)
                .expect("the mined height is ChainLocked");
            assert_eq!(proof.core_chain_locked_height, mined);

            // Platform is behind: every fee step of the proof is refused.
            let (kinds, capped) = network.fund_until_capped(&proof).await;
            assert!(
                kinds
                    .iter()
                    .all(|kind| *kind == FundingFailureKind::PlatformBehindCore),
                "{hit:?}: {kinds:?}"
            );
            assert_eq!(
                capped.unwrap_err().kind,
                FundingFailureKind::FeeCapReached,
                "{hit:?}"
            );
            let checked = network.checked.lock().unwrap().clone();
            assert_eq!(
                checked,
                (0..=MAX_FUNDING_FEE_INCREASE)
                    .map(|fee| (mined, fee))
                    .collect::<Vec<_>>(),
                "{hit:?}: each pair reaches CheckTx once"
            );

            // Platform catches up; the same proof from floor 0 never lands.
            *network.core_height.lock().unwrap() = mined + 5;
            let (_, again) = network.fund_until_capped(&proof).await;
            assert!(again.is_err(), "{hit:?}: a refused pair was admitted");
            assert_eq!(
                network.checked.lock().unwrap().len(),
                checked.len(),
                "{hit:?}: a held pair reached CheckTx"
            );

            // One block above it, from floor 0, lands at the first attempt.
            let next = next_chain_lock_proof(
                out_point,
                mined,
                Some(proof.core_chain_locked_height),
                mined + 5,
            )
            .expect("a ChainLock covers the next height");
            assert_eq!(next.core_chain_locked_height, mined + 1);
            let (built, landed) = network.fund(&next, 0).await;
            assert!(landed.is_ok(), "{hit:?}");
            assert_eq!(built, vec![0], "{hit:?}");
        }
    }

    /// The pair (proof, fee) is the transition's identity: the same pair
    /// gives the same bytes, and the next height gives new ones at fee 0.
    #[test]
    fn should_sign_new_bytes_only_for_a_new_proof_or_fee() {
        let proof = |height| ChainAssetLockProof {
            core_chain_locked_height: height,
            out_point: OutPoint::new(Txid::from_byte_array([4; 32]), 1),
        };
        assert_eq!(
            Network::top_up(&proof(1_000), 0),
            Network::top_up(&proof(1_000), 0)
        );
        assert_ne!(
            Network::top_up(&proof(1_000), 0),
            Network::top_up(&proof(1_000), 1)
        );
        assert_ne!(
            Network::top_up(&proof(1_000), 0),
            Network::top_up(&proof(1_001), 0)
        );
    }
}
