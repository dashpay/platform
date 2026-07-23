//! Submission-side orchestration shared across asset-lock-funded
//! flows (identity registration, identity top-up, platform-address
//! funding).
//!
//! The asset-lock acquisition pipeline (build tx → wait IS/CL) lives
//! in [`crate::wallet::asset_lock::build`] /
//! [`crate::wallet::asset_lock::sync`]. This module holds the *next*
//! layer: turning a funding-source choice into a usable
//! `(AssetLockProof, DerivationPath, OutPoint)` triple, and the
//! Platform-side retry policy applied to whatever ST consumes that
//! triple.
//!
//! Two pieces here:
//!
//! - [`submit_with_cl_height_retry`] — retry-on-10506 wrapper that
//!   bumps `user_fee_increase` between attempts so Tenderdash's
//!   invalid-tx hash cache (24h on mainnet/testnet) can't silently
//!   drop resubmits.
//! - [`AssetLockManager::resolve_funding_with_is_timeout_fallback`] —
//!   maps an [`AssetLockFunding`] choice to a [`FundingResolution`]
//!   that the caller can drive into an IS→CL retry when the IS-lock
//!   timed out.
//!
//! Both are funding-target-agnostic: the caller passes the
//! `AssetLockFundingType` + destination index (identity_index for
//! identity flows, address index for address-funding flows) into
//! the resolver, and supplies its own ST submission closure to the
//! retry helper. The constants here pin the same timeouts across
//! every flow so register / top-up / address-fund can't drift apart
//! on their CL fallback or retry-budget windows.

use std::time::Duration;

use dashcore::OutPoint;
use dpp::prelude::AssetLockProof;
use key_wallet::bip32::DerivationPath;
use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;

use dash_sdk::platform::transition::put_settings::PutSettings;

use crate::broadcaster::TransactionBroadcaster;
use crate::error::{as_asset_lock_proof_cl_height_too_low, PlatformWalletError};
use crate::wallet::asset_lock::manager::AssetLockManager;

// ---------------------------------------------------------------------------
// Timeout policy
// ---------------------------------------------------------------------------

/// Bounded ChainLock wait used *only* by the shielded seed pool, where a
/// `FinalityTimeout` is a deliberate pacing signal — rapid back-to-back
/// batches chain unconfirmed L1 change outputs, and around core's
/// unconfirmed-ancestor depth limit IS/CL proofs stop arriving until a
/// block lands; the seed pool catches the timeout, pauses, and resumes
/// the tracked lock (see `shielded/seed_pool.rs`).
///
/// The user-facing funding flows (identity registration / top-up,
/// platform-address top-up, and user-initiated shielded funding) do NOT
/// use this: they wait for a ChainLock **indefinitely**
/// (`upgrade_to_chain_lock_proof(None)`), because a ChainLock is
/// deterministic finality that will eventually cover any broadcast
/// asset-lock tx — so a broadcast lock is *pending*, never *failed*.
///
/// Only the shielded seed pool consumes this, so it is `shielded`-gated
/// to avoid a dead-code warning in builds without that feature.
#[cfg(feature = "shielded")]
pub(crate) const CL_FALLBACK_TIMEOUT: Duration = Duration::from_secs(180);

/// Delay between retries when Platform rejected with CL-height-too-low.
/// Each retry bumps `PutSettings::user_fee_increase` so the ST hash
/// changes (Tenderdash caches rejected ST hashes for ~24h on
/// mainnet/testnet — `keep-invalid-txs-in-cache = true` in dashmate's
/// tenderdash template, hardcoded at
/// `packages/dashmate/templates/platform/drive/tenderdash/config.toml.dot:355`).
pub(crate) const CL_HEIGHT_RETRY_DELAY: Duration = Duration::from_secs(15);

/// Total time we'll keep retrying before surfacing the error. Sized to
/// cover Platform's `create-empty-blocks-interval` (3m on mainnet)
/// plus a 30s safety margin: if Platform hasn't observed the wallet's
/// ChainLock by then, the lag is no longer routine and we need
/// operator visibility instead of further silent retries. The
/// rejection error carries Platform's `current_core_chain_locked_height`
/// each round so logs name the laggard node's reported tip explicitly.
pub(crate) const CL_HEIGHT_RETRY_BUDGET: Duration = Duration::from_secs(210);

// ---------------------------------------------------------------------------
// Funding choice
// ---------------------------------------------------------------------------

/// How to source the asset lock funding for an asset-lock-funded
/// Platform operation (identity registration, identity top-up,
/// platform-address funding).
///
/// Resolved by [`AssetLockManager::resolve_funding_with_is_timeout_fallback`]
/// into an `(AssetLockProof, DerivationPath, OutPoint)` triple that
/// the `_with_signer` SDK methods can consume. The `OutPoint` is
/// retained for cleanup (so the tracked-asset-lock row can be removed
/// on success) and for IS→CL fallback (so the consumed lock can be
/// looked up by outpoint when the IS proof times out or is rejected).
///
/// Every variant produces a lock tracked by this wallet's
/// [`AssetLockManager`]. The IS→CL fallback paths (300s IS-timeout in
/// the resolver, Platform IS-rejection retry in the submission layer)
/// require the lock to be tracked so they can look it up by outpoint
/// and drive the wait. An earlier variant (`UseAssetLock`) accepted
/// an externally-built proof and skipped tracking — it broke the
/// IS→CL fallback unrecoverably because the lock was invisible to
/// `upgrade_to_chain_lock_proof` (which short-circuits with
/// `Asset lock {} is not tracked`). The variant was removed; future
/// callers that hold an external proof should register it through
/// `AssetLockManager` first, then use `FromExistingAssetLock`.
#[derive(Debug, Clone)]
pub enum AssetLockFunding {
    /// Build an asset lock from wallet UTXOs for the given amount.
    ///
    /// The caller passes the `AssetLockFundingType` into the resolver
    /// to select which BIP44 derivation family is used for the
    /// credit-output key:
    ///
    /// - `IdentityRegistration` — for identity register flows
    /// - `IdentityTopUp` — for identity top-up flows
    /// - `AssetLockAddressTopUp` — for platform-address funding flows
    /// - others — see [`AssetLockFundingType`]
    ///
    /// `account_index` selects which BIP44 *standard* account (by
    /// BIP44 account index) supplies the UTXOs. This exact-amount form
    /// is BIP44-only; CoinJoin funding exists solely as the
    /// whole-balance [`AssetLockFunding::DrainAccountBalance`] form
    /// (CoinJoin accounts have no change semantics). BIP32 funding
    /// remains unsupported.
    FromWalletBalance {
        /// Amount to lock (in duffs).
        amount_duffs: u64,
        /// BIP44 standard-account index to draw the funding UTXOs from.
        account_index: u32,
    },

    /// Build an asset lock that drains a funding account's whole balance:
    /// every final UTXO of `account` is consumed and the lock value is
    /// `Σ inputs − fee`, computed by the key-wallet builder.
    ///
    /// This is the CoinJoin → Shielded path: mixed coins fund the asset
    /// lock directly (CoinJoin funding is drain-only — those accounts have
    /// no change semantics), so they never hop through a transparent BIP44
    /// address. A `Bip44` account is also accepted for a whole-balance
    /// BIP44 lock.
    DrainAccountBalance {
        /// The account family + index to drain.
        account: key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingAccount,
    },

    /// Resume from a tracked asset lock identified by its outpoint
    /// (txid + output index).
    ///
    /// The asset lock must already be tracked by the
    /// [`AssetLockManager`]. The manager resumes from whatever stage
    /// the lock is at (built, broadcast, IS-locked, or chain-locked)
    /// and re-derives the credit-output derivation path; the
    /// signer-driven submission path then passes that path back to
    /// the same signer when constructing the consuming state
    /// transition.
    FromExistingAssetLock {
        /// The outpoint identifying the tracked asset lock (txid + output index).
        out_point: OutPoint,
        /// Explicit authorization to consume an
        /// [`AssetLockFundingType::IdentityInvitation`]-typed lock — a
        /// DashPay invitation **bearer voucher** whose key was exported
        /// into a shared link. Only the invitation reclaim flow sets this;
        /// every generic resume/top-up path leaves it `false` and is
        /// refused invitation locks by the resolver, so a voucher can
        /// never be silently consumed into an unrelated local identity
        /// (which would invalidate the invitee's already-shared claim).
        consume_invitation_voucher: bool,
    },
}

// ---------------------------------------------------------------------------
// Funding resolution outcome
// ---------------------------------------------------------------------------

/// Outcome of resolving an [`AssetLockFunding`] to a concrete
/// asset-lock proof + derivation path.
///
/// `tracked_out_point` is always `Some` — every `AssetLockFunding`
/// variant produces a lock tracked by this wallet's `AssetLockManager`
/// (the now-removed `UseAssetLock` variant was the only one that set
/// it to `None`, and its absence broke both the IS-timeout and the
/// IS-rejection fallback paths because they need the tracked entry to
/// drive `upgrade_to_chain_lock_proof`). The outpoint drives IS→CL
/// fallback (look up the lock by outpoint) and cleanup (remove the
/// lock on Platform success). Kept as `Option<OutPoint>` for now so
/// future variants without lifecycle tracking can be added without
/// reshaping `FundingResolution`; today every code path that
/// constructs it passes `Some`.
pub(crate) struct ResolvedFunding {
    pub(crate) proof: AssetLockProof,
    pub(crate) path: DerivationPath,
    pub(crate) tracked_out_point: Option<OutPoint>,
}

/// Outcome of [`AssetLockManager::resolve_funding_with_is_timeout_fallback`]:
/// either a fully-resolved funding triple, or an IS-timeout that the
/// caller can convert to a ChainLock retry using the recovered
/// outpoint.
pub(crate) enum FundingResolution {
    Resolved(ResolvedFunding),
    /// IS-lock didn't propagate within the asset-lock manager's wait
    /// window. The outpoint of the tracked-but-unproven lock is
    /// surfaced so the caller can drive an `upgrade_to_chain_lock_proof`
    /// retry without re-walking the tracked-asset-lock map.
    IsTimeout {
        out_point: OutPoint,
    },
}

// ---------------------------------------------------------------------------
// Retry helper
// ---------------------------------------------------------------------------

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
/// (CL-height-too-low). Every other `dash_sdk::Error` — including
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
/// comes from the SPV-verified ChainLock BLS signature
/// (`info.core_wallet.metadata.last_applied_chain_lock`) that promoted
/// the asset-lock tx's record context to `InChainLockedBlock` before
/// we constructed the proof.
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
/// surfaces the error. A proper fix would require cryptographically
/// verifiable consensus errors (a quorum signature on rejection, or
/// validator attestation) and is tracked as future work; doing it
/// in-place here would either re-implement DAPI client trust or
/// require an SDK API change neither of which belong in this PR.
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
pub(crate) async fn submit_with_cl_height_retry<F, Fut, R>(
    mut settings: Option<PutSettings>,
    submit: F,
) -> Result<R, dash_sdk::Error>
where
    F: Fn(Option<PutSettings>) -> Fut,
    Fut: std::future::Future<Output = Result<R, dash_sdk::Error>>,
{
    let started = tokio::time::Instant::now();
    let deadline = started + CL_HEIGHT_RETRY_BUDGET;
    let mut attempt: u32 = 0;
    loop {
        attempt += 1;
        match submit(settings).await {
            Ok(r) => return Ok(r),
            Err(e) => {
                let Some(detail) = as_asset_lock_proof_cl_height_too_low(&e) else {
                    return Err(e);
                };
                let elapsed = started.elapsed();
                let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
                if remaining.is_zero() {
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
                tokio::time::sleep(sleep_for).await;
            }
        }
    }
}

/// Bump `user_fee_increase` by 1 (saturating at `u16::MAX`).
fn bump_user_fee_increase(mut settings: PutSettings) -> PutSettings {
    settings.user_fee_increase = Some(settings.user_fee_increase.unwrap_or(0).saturating_add(1));
    settings
}

// ---------------------------------------------------------------------------
// Free helpers
// ---------------------------------------------------------------------------

/// Extract the outpoint from an asset lock proof. Total over the
/// `AssetLockProof` enum — neither variant can fail to produce an
/// outpoint (Instant: derived from embedded tx + output index;
/// Chain: carried directly as `out_point`).
///
/// Free function (not an `AssetLockManager` method) because it has
/// no dependency on the manager's state and the manager is generic
/// over its broadcaster `B`, which would force callers into explicit
/// turbofish.
pub(crate) fn out_point_from_proof(proof: &AssetLockProof) -> OutPoint {
    match proof {
        AssetLockProof::Instant(instant) => {
            OutPoint::new(instant.transaction().txid(), instant.output_index())
        }
        AssetLockProof::Chain(chain) => chain.out_point,
    }
}

// ---------------------------------------------------------------------------
// Resolver on AssetLockManager
// ---------------------------------------------------------------------------

impl<B: TransactionBroadcaster + ?Sized> AssetLockManager<B> {
    /// Resolve an [`AssetLockFunding`] to a concrete proof + path +
    /// (optional) tracked outpoint, capturing the IS-lock timeout case
    /// as a structured outcome so the caller can drive a CL retry.
    ///
    /// `funding_type` selects the BIP44 account the `FromWalletBalance`
    /// variant pulls UTXOs from (`IdentityRegistration` for register,
    /// `IdentityTopUp` for top up, `AssetLockAddressTopUp` for
    /// platform-address funding). The other variants ignore it — they
    /// don't build new asset locks.
    ///
    /// `destination_index` is the within-family HD index — the
    /// identity index for identity flows, the address index for
    /// platform-address funding flows. Routed straight through to
    /// [`Self::create_funded_asset_lock_proof`].
    ///
    /// # IS-lock timeout handling
    ///
    /// For the two variants that internally invoke `wait_for_proof`
    /// (`FromWalletBalance` and `FromExistingAssetLock`), an IS-lock
    /// that never propagates within the 300s window surfaces as
    /// `PlatformWalletError::FinalityTimeout(out_point)`. The variant
    /// carries the *exact* outpoint that timed out (no
    /// `find_tracked_unproven_lock` BTreeMap walk needed), so the
    /// `IsTimeout` outcome is built directly from the error payload.
    pub(crate) async fn resolve_funding_with_is_timeout_fallback<AS>(
        &self,
        funding: AssetLockFunding,
        funding_type: AssetLockFundingType,
        destination_index: u32,
        asset_lock_signer: &AS,
    ) -> Result<FundingResolution, PlatformWalletError>
    where
        AS: ::key_wallet::signer::Signer + Send + Sync,
    {
        match funding {
            AssetLockFunding::FromWalletBalance {
                amount_duffs,
                account_index,
            } => {
                match self
                    .create_funded_asset_lock_proof(
                        amount_duffs,
                        account_index,
                        funding_type,
                        destination_index,
                        asset_lock_signer,
                    )
                    .await
                {
                    Ok((proof, path, out_point)) => {
                        Ok(FundingResolution::Resolved(ResolvedFunding {
                            proof,
                            path,
                            tracked_out_point: Some(out_point),
                        }))
                    }
                    Err(PlatformWalletError::FinalityTimeout(out_point)) => {
                        // The exact outpoint that timed out comes from
                        // the error payload — no `find_tracked_unproven_lock`
                        // walk needed (which would pick BTreeMap-first
                        // on multiple unproven locks for the same key).
                        Ok(FundingResolution::IsTimeout { out_point })
                    }
                    Err(e) => Err(e),
                }
            }
            AssetLockFunding::DrainAccountBalance {
                account,
            } => {
                // Same pipeline as `FromWalletBalance`, with drain amount
                // semantics and the caller-picked funding account family.
                match self
                    .create_funded_asset_lock_proof_with_funding(
                        super::build::AssetLockBuildAmount::DrainAll,
                        account,
                        funding_type,
                        destination_index,
                        asset_lock_signer,
                    )
                    .await
                {
                    Ok((proof, path, out_point)) => {
                        Ok(FundingResolution::Resolved(ResolvedFunding {
                            proof,
                            path,
                            tracked_out_point: Some(out_point),
                        }))
                    }
                    Err(PlatformWalletError::FinalityTimeout(out_point)) => {
                        Ok(FundingResolution::IsTimeout { out_point })
                    }
                    Err(e) => Err(e),
                }
            }
            AssetLockFunding::FromExistingAssetLock {
                out_point,
                consume_invitation_voucher,
            } => {
                // Invitation vouchers are bearer instruments: the credit
                // output's private key was exported into a shared link, so
                // consuming the lock through a generic resume/top-up would
                // both misdirect the funds into a local identity and kill
                // the invitee's claim. Refuse unless the caller carries the
                // reclaim flow's explicit authorization.
                if !consume_invitation_voucher
                    && self.tracked_funding_type(&out_point).await
                        == Some(AssetLockFundingType::IdentityInvitation)
                {
                    return Err(PlatformWalletError::AssetLockTransaction(format!(
                        "asset lock {out_point} is a DashPay invitation voucher; \
                         generic resume/top-up refuses to consume it (its key is \
                         shared in the invitation link, and consuming it would \
                         invalidate the invitee's claim) — use the invitation \
                         reclaim flow, which passes explicit authorization"
                    )));
                }
                // 300s is an InstantSend-preference window, not a finality
                // timeout: on expiry the caller falls back to an unbounded
                // ChainLock wait, so a resumed broadcast lock never fails
                // just because IS was slow.
                match self
                    .resume_asset_lock(&out_point, Some(Duration::from_secs(300)))
                    .await
                {
                    Ok((proof, path)) => Ok(FundingResolution::Resolved(ResolvedFunding {
                        proof,
                        path,
                        tracked_out_point: Some(out_point),
                    })),
                    Err(PlatformWalletError::FinalityTimeout(timed_out)) => {
                        // Outpoint from the error (which equals
                        // `out_point` from the variant in practice —
                        // but we use the error payload for parity
                        // with the FromWalletBalance arm).
                        Ok(FundingResolution::IsTimeout {
                            out_point: timed_out,
                        })
                    }
                    Err(e) => Err(e),
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Fabricate the SDK-side 10506 error shape exactly as
    /// `as_asset_lock_proof_cl_height_too_low` recognizes it
    /// (`error.rs:223-242`). Both the matcher and the constructor are
    /// pinned here so a future SDK refactor that changes the variant
    /// path can't silently desynchronize the retry helper from its
    /// test surface.
    fn fabricate_cl_height_too_low_error() -> dash_sdk::Error {
        use dpp::consensus::basic::identity::InvalidAssetLockProofCoreChainHeightError;
        use dpp::consensus::basic::BasicError;
        use dpp::consensus::ConsensusError;

        let consensus =
            ConsensusError::BasicError(BasicError::InvalidAssetLockProofCoreChainHeightError(
                InvalidAssetLockProofCoreChainHeightError::new(
                    /* proof_core_chain_locked_height */ 100,
                    /* current_core_chain_locked_height */ 99,
                ),
            ));
        dash_sdk::Error::Protocol(dpp::ProtocolError::ConsensusError(Box::new(consensus)))
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
    async fn submit_with_cl_height_retry_bumps_user_fee_and_surfaces_after_budget() {
        use dash_sdk::platform::transition::put_settings::PutSettings;
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
}
