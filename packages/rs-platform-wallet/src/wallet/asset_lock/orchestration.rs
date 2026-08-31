//! Submission-side orchestration shared across asset-lock-funded
//! flows (identity registration, identity top-up, platform-address
//! funding, and shielded funding).
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
use crate::error::{
    as_asset_lock_proof_cl_height_too_low, is_asset_lock_already_consumed, PlatformWalletError,
};
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

/// Bounded ChainLock wait for the **already-consumed reconciliation** path
/// ([`AssetLockManager::reconcile_asset_lock_submit_result`]).
///
/// Reconciliation is not a funding flow, so the "a ChainLock always
/// eventually arrives, therefore wait forever" reasoning behind
/// `upgrade_to_chain_lock_proof(None)` does not transfer to it. The
/// operation Platform was asked to perform has already terminated with an
/// unauthenticated `already consumed` report; the ChainLock is wanted only
/// as *evidence to durably record alongside that report*, and the caller is
/// blocked the whole time it is being fetched.
///
/// Every production call site reaches this through an FFI entry point that
/// drives the future with `runtime().block_on(...)`, so an unbounded wait
/// here does not merely delay a result — it pins the host thread that made
/// the call. The realistic trigger is routine: an IS-locked lock consumed
/// seconds after broadcast reports `already consumed` while its ChainLock
/// is still ~2.5 minutes out, and never arrives at all when the device is
/// offline or SPV is not connected.
///
/// On expiry the reconciliation still returns the typed
/// [`PlatformWalletError::AssetLockAlreadyConsumed`] — the code-24 signal
/// hosts branch on — having simply failed to attach the chain proof. That
/// matches the pre-#4357 behavior (typed error, no proof retained) while
/// keeping #4357's proof retention whenever the ChainLock is reachable
/// inside the bound.
pub(crate) const RECONCILIATION_CHAIN_LOCK_TIMEOUT: Duration = Duration::from_secs(180);

/// Bounded proof wait applied after a resume re-broadcast came back
/// `MaybeSent` (see `sync::recovery::resume_asset_lock`).
///
/// The unbounded `wait_for_proof(None)` used by the funding flows is
/// justified by the transaction being *known* broadcast — finality is then
/// only a matter of time. A `MaybeSent` verdict does not establish that:
/// `DapiBroadcaster` classifies every failure as `MaybeSent`, and the SPV
/// broadcaster reports `Rejected` only for `NotConnected`, so a genuinely
/// rejected transaction is indistinguishable from an accepted one. Waiting
/// without a bound on that signal converts a ~30s broadcast failure into a
/// permanent hang at the `resume_asset_lock(.., None)` call sites.
///
/// Sized to comfortably cover a ChainLock (~2.5 min) so a transaction that
/// really was accepted still resolves inside the bound; on expiry the
/// caller gets `TransactionBroadcastUnconfirmed`, which is what the
/// pre-#4367 code returned immediately.
pub(crate) const UNCONFIRMED_BROADCAST_PROOF_TIMEOUT: Duration = Duration::from_secs(180);

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
/// `upgrade_to_chain_lock_proof` (which short-circuits with the typed
/// [`PlatformWalletError::AssetLockNotTracked`]). The variant was removed; future
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
    /// Funding is POOLED across `ASSET_LOCK_FUNDING_SOURCES`: coin
    /// selection draws from the union of the BIP44 and BIP32 accounts at
    /// `account_index` and every DashPay contact-receiving account, so
    /// the lock does not need its whole amount sitting in one account.
    /// Change returns to BIP44, the first source. Sources this wallet has
    /// nothing for are skipped.
    ///
    /// CoinJoin is deliberately not in that set — spending mixed outputs
    /// alongside transparent ones links them and undoes the mixing — so
    /// CoinJoin funding still exists solely as the whole-balance
    /// [`AssetLockFunding::DrainAccountBalance`] form (those accounts
    /// also have no change semantics).
    FromWalletBalance {
        /// Amount to lock (in duffs).
        amount_duffs: u64,
        /// Index addressing the standard (BIP44/BIP32) families of the
        /// pooled source set. DashPay contact-receiving accounts span
        /// their own indices and are pooled in regardless.
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
        account:
            key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingAccount,
        /// Floor on the drained lock value, enforced against the BUILT
        /// payload before tracking/broadcast (see
        /// [`AssetLockBuildAmount::DrainAll`]); an undersized build is
        /// abandoned with nothing on the wire. `None` skips the check.
        ///
        /// [`AssetLockBuildAmount::DrainAll`]: super::build::AssetLockBuildAmount::DrainAll
        minimum_lock_duffs: Option<u64>,
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
    /// Normalize the final result of an asset-lock-funded Platform submit.
    ///
    /// A matching `already consumed` consensus response is not authenticated,
    /// so it cannot prove that the requested operation completed. Promote the
    /// lock to an SPV-backed ChainLock proof, durably retain it as
    /// consumption-unknown, and preserve the typed host signal. Successful and
    /// unrelated results pass through unchanged.
    ///
    /// `chain_lock_timeout` bounds the IS→CL promotion. `None` does **not**
    /// mean "wait forever" here: it selects
    /// [`RECONCILIATION_CHAIN_LOCK_TIMEOUT`]. Reconciliation always
    /// terminates, because the promotion is a best-effort attempt to attach
    /// evidence to an operation that has *already* finished, and every
    /// production caller reaches it under an FFI `block_on` that would
    /// otherwise pin the host thread for as long as the ChainLock is
    /// missing (offline / unconnected SPV: forever).
    ///
    /// Failing to obtain the proof therefore degrades rather than
    /// propagates: the lock keeps its current status and the typed
    /// [`PlatformWalletError::AssetLockAlreadyConsumed`] is still returned,
    /// so the host's code-24 branch is reached either way and the caller
    /// may retry to pick the proof up later.
    pub(crate) async fn reconcile_asset_lock_submit_result<T>(
        &self,
        result: Result<T, dash_sdk::Error>,
        out_point: &OutPoint,
        effective_proof: &AssetLockProof,
        chain_lock_timeout: Option<Duration>,
    ) -> Result<T, PlatformWalletError> {
        let error = match result {
            Ok(value) => return Ok(value),
            Err(error) => error,
        };
        if !is_asset_lock_already_consumed(&error, out_point) {
            return Err(PlatformWalletError::Sdk(error));
        }

        let chain_proof = match effective_proof {
            AssetLockProof::Chain(_) => Some(effective_proof.clone()),
            AssetLockProof::Instant(_) => {
                let bounded = chain_lock_timeout.or(Some(RECONCILIATION_CHAIN_LOCK_TIMEOUT));
                match self.upgrade_to_chain_lock_proof(out_point, bounded).await {
                    Ok(proof) => Some(proof),
                    // Bounded, so this arm is reachable in normal operation
                    // (an IS-locked lock consumed seconds after broadcast has
                    // no ChainLock yet). Record nothing, keep the code-24
                    // signal, let the caller retry.
                    //
                    // Deliberately a catch-all, not `FinalityTimeout`-only.
                    // The code-24 classification above came from Platform's
                    // outpoint-matched consensus error, not from this local
                    // lookup, so a `WalletNotFound` / `AssetLockProofWait`
                    // (lock untracked after a restore, persister failure,
                    // record-map mismatch) does not invalidate it — the
                    // promotion is best-effort evidence-attachment either
                    // way. Propagating those errors instead would swap the
                    // host's actionable already-consumed branch for a
                    // generic local error in exactly the degraded-state
                    // scenarios where that branch is the only path that can
                    // still resolve the operation from Platform-side
                    // evidence. Failures on the RECORDING path below do
                    // propagate (`mark_asset_lock_consumption_unknown` keeps
                    // its `?`).
                    Err(e) => {
                        tracing::warn!(
                            outpoint = %out_point,
                            error = %e,
                            timeout = ?bounded,
                            "could not obtain a ChainLock proof for an unauthenticated \
                             already-consumed report within the bound; reporting the lock \
                             as already consumed without retaining consumption-unknown state"
                        );
                        None
                    }
                }
            }
        };

        if let Some(chain_proof) = chain_proof {
            self.mark_asset_lock_consumption_unknown(out_point, chain_proof)
                .await?;
            tracing::warn!(
                outpoint = %out_point,
                "recorded unauthenticated already-consumed report as consumption unknown"
            );
        }

        Err(PlatformWalletError::AssetLockAlreadyConsumed(*out_point))
    }

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
        AS: ::key_wallet::signer::ExtendedPubKeySigner + Send + Sync,
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
                minimum_lock_duffs,
            } => {
                // Same pipeline as `FromWalletBalance`, with drain amount
                // semantics and the caller-picked funding account family.
                match self
                    .create_funded_asset_lock_proof_with_funding(
                        super::build::AssetLockBuildAmount::DrainAll { minimum_lock_duffs },
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
                let (actual_funding_type, actual_identity_index, status) = self
                    .tracked_resume_metadata(&out_point)
                    .await
                    .ok_or(PlatformWalletError::AssetLockNotTracked(out_point))?;
                if status == crate::wallet::asset_lock::tracked::AssetLockStatus::Consumed {
                    return Err(PlatformWalletError::AssetLockAlreadyConsumed(out_point));
                }
                // Invitation vouchers are bearer instruments: the credit
                // output's private key was exported into a shared link, so
                // consuming the lock through a generic resume/top-up would
                // both misdirect the funds into a local identity and kill
                // the invitee's claim. Refuse unless the caller carries the
                // reclaim flow's explicit authorization.
                let invitation_reclaim = authorized_invitation_reclaim(
                    actual_funding_type,
                    funding_type,
                    consume_invitation_voucher,
                );
                if !invitation_reclaim {
                    validate_existing_asset_lock_role(
                        out_point,
                        funding_type,
                        destination_index,
                        actual_funding_type,
                        actual_identity_index,
                    )?;
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

/// The two dedicated invitation-reclaim destinations. Explicit authority is
/// still mandatory; generic registration/top-up recovery always passes false.
fn authorized_invitation_reclaim(
    actual_funding_type: AssetLockFundingType,
    expected_funding_type: AssetLockFundingType,
    consume_invitation_voucher: bool,
) -> bool {
    actual_funding_type == AssetLockFundingType::IdentityInvitation
        && consume_invitation_voucher
        && matches!(
            expected_funding_type,
            AssetLockFundingType::IdentityRegistration | AssetLockFundingType::IdentityTopUp
        )
}

/// Authorize a tracked one-shot output for the operation that is about to
/// consume it. Identity registration and bound top-up paths must match their
/// identity index; an unbound top-up is deliberately eligible for any target.
/// Invitation vouchers never pass this generic validator—the explicit reclaim
/// authorization above is the only bypass.
fn validate_existing_asset_lock_role(
    out_point: OutPoint,
    expected_funding_type: AssetLockFundingType,
    expected_identity_index: u32,
    actual_funding_type: AssetLockFundingType,
    actual_identity_index: u32,
) -> Result<(), PlatformWalletError> {
    let eligible = match expected_funding_type {
        AssetLockFundingType::IdentityRegistration => {
            actual_funding_type == AssetLockFundingType::IdentityRegistration
                && actual_identity_index == expected_identity_index
        }
        AssetLockFundingType::IdentityTopUp => {
            (actual_funding_type == AssetLockFundingType::IdentityTopUp
                && actual_identity_index == expected_identity_index)
                || actual_funding_type == AssetLockFundingType::IdentityTopUpNotBound
        }
        _ => actual_funding_type == expected_funding_type,
    };
    if eligible {
        Ok(())
    } else {
        Err(PlatformWalletError::AssetLockFundingMismatch {
            out_point,
            expected_funding_type,
            expected_identity_index,
            actual_funding_type,
            actual_identity_index,
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn existing_lock_role_validation_binds_identity_funding() {
        let out_point = OutPoint::null();

        assert!(validate_existing_asset_lock_role(
            out_point,
            AssetLockFundingType::IdentityRegistration,
            7,
            AssetLockFundingType::IdentityRegistration,
            7,
        )
        .is_ok());
        for error in [
            validate_existing_asset_lock_role(
                out_point,
                AssetLockFundingType::IdentityRegistration,
                7,
                AssetLockFundingType::IdentityRegistration,
                8,
            ),
            validate_existing_asset_lock_role(
                out_point,
                AssetLockFundingType::IdentityRegistration,
                7,
                AssetLockFundingType::IdentityTopUp,
                7,
            ),
            validate_existing_asset_lock_role(
                out_point,
                AssetLockFundingType::IdentityRegistration,
                0,
                AssetLockFundingType::IdentityInvitation,
                0,
            ),
        ] {
            assert!(matches!(
                error,
                Err(PlatformWalletError::AssetLockFundingMismatch { .. })
            ));
        }

        assert!(validate_existing_asset_lock_role(
            out_point,
            AssetLockFundingType::IdentityTopUp,
            11,
            AssetLockFundingType::IdentityTopUp,
            11,
        )
        .is_ok());
        assert!(validate_existing_asset_lock_role(
            out_point,
            AssetLockFundingType::IdentityTopUp,
            11,
            AssetLockFundingType::IdentityTopUpNotBound,
            999,
        )
        .is_ok());
        assert!(matches!(
            validate_existing_asset_lock_role(
                out_point,
                AssetLockFundingType::IdentityTopUp,
                11,
                AssetLockFundingType::IdentityTopUp,
                12,
            ),
            Err(PlatformWalletError::AssetLockFundingMismatch { .. })
        ));
    }

    #[test]
    fn explicit_invitation_reclaim_allows_registration_and_topup_only() {
        for expected in [
            AssetLockFundingType::IdentityRegistration,
            AssetLockFundingType::IdentityTopUp,
        ] {
            assert!(authorized_invitation_reclaim(
                AssetLockFundingType::IdentityInvitation,
                expected,
                true,
            ));
            assert!(!authorized_invitation_reclaim(
                AssetLockFundingType::IdentityInvitation,
                expected,
                false,
            ));
        }
        assert!(!authorized_invitation_reclaim(
            AssetLockFundingType::IdentityInvitation,
            AssetLockFundingType::AssetLockAddressTopUp,
            true,
        ));
        assert!(!authorized_invitation_reclaim(
            AssetLockFundingType::IdentityRegistration,
            AssetLockFundingType::IdentityRegistration,
            true,
        ));
    }

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

    // -----------------------------------------------------------------
    // Reconciliation promotion-failure regressions
    //
    // Two tests pin the two failure shapes of the IS→CL promotion inside
    // `reconcile_asset_lock_submit_result`, one per arm of the deliberate
    // catch-all on its `Err` branch:
    //   - `already_consumed_reconciliation_terminates_without_a_chainlock`
    //     pins the BOUNDED-WAIT shape (`FinalityTimeout`);
    //   - `already_consumed_reconciliation_downgrades_non_timeout_promotion_failure`
    //     pins the DEGRADED-LOCAL-STATE shape (`AssetLockProofWait`).
    // Each first asserts the promotion error variant DIRECTLY, so the two
    // scenarios cannot silently collapse onto the same path, then asserts
    // the shared downgrade outcome.
    // -----------------------------------------------------------------

    use crate::test_support::{
        funded_wallet_manager, AlwaysRejectedBroadcaster, NoopTestPersister,
    };
    use crate::wallet::asset_lock::manager::AssetLockManager;
    use crate::wallet::asset_lock::tracked::{AssetLockStatus, TrackedAssetLock};
    use crate::wallet::persister::WalletPersister;
    use crate::wallet::platform_wallet::{PlatformWalletInfo, WalletId};
    use dpp::consensus::basic::identity::IdentityAssetLockTransactionOutPointAlreadyConsumedError;
    use dpp::identity::state_transition::asset_lock_proof::InstantAssetLockProof;
    use std::sync::Arc;

    /// Shared fixture: a funded wallet with ONE tracked, IS-locked asset
    /// lock whose effective proof is an Instant proof — the shape that
    /// routes `reconcile_asset_lock_submit_result` through the
    /// `upgrade_to_chain_lock_proof` promotion (a Chain proof would
    /// short-circuit past it).
    ///
    /// The built funding transaction is deliberately NOT registered as a
    /// `TransactionRecord` anywhere: `build_asset_lock_transaction` only
    /// reserves inputs, nothing is broadcast, and `NoopTestPersister`
    /// keeps the persistence trait's `Ok(None)` record lookup. Out of the
    /// box the promotion therefore fast-fails with `AssetLockProofWait`
    /// ("transaction not found"); a test that wants the bounded-wait
    /// `FinalityTimeout` shape instead must register a (non-chain-locked)
    /// record for `transaction` first.
    struct InstantReconciliationContext {
        manager: AssetLockManager<AlwaysRejectedBroadcaster>,
        wallet_manager:
            Arc<tokio::sync::RwLock<key_wallet_manager::WalletManager<PlatformWalletInfo>>>,
        wallet_id: WalletId,
        transaction: dashcore::Transaction,
        out_point: OutPoint,
        instant_proof: AssetLockProof,
    }

    async fn instant_reconciliation_context() -> InstantReconciliationContext {
        use dashcore::{InstantLock, Network};
        use key_wallet::account::account_type::StandardAccountType;
        use tokio::sync::Notify;

        let (wallet_manager, wallet_id, _generation, signer) =
            funded_wallet_manager(StandardAccountType::BIP44Account).await;
        let sdk = Arc::new(
            dash_sdk::SdkBuilder::new_mock()
                .with_network(Network::Testnet)
                .build()
                .expect("mock sdk"),
        );
        let manager = AssetLockManager::new(
            sdk,
            Arc::clone(&wallet_manager),
            wallet_id,
            Arc::new(Notify::new()),
            Arc::new(AlwaysRejectedBroadcaster),
            WalletPersister::new(wallet_id, Arc::new(NoopTestPersister)),
        );
        let (transaction, _path) = manager
            .build_asset_lock_transaction(
                1_000_000,
                0,
                AssetLockFundingType::IdentityRegistration,
                0,
                &signer,
            )
            .await
            .expect("build asset lock");
        let out_point = OutPoint::new(transaction.txid(), 0);

        // An INSTANT proof: this is what selects the `upgrade_to_chain_lock_proof`
        // arm. A Chain proof would short-circuit and never exercise the wait.
        let instant_proof = AssetLockProof::Instant(InstantAssetLockProof::new(
            InstantLock::default(),
            transaction.clone(),
            0,
        ));
        {
            let mut wm = wallet_manager.write().await;
            wm.get_wallet_info_mut(&wallet_id)
                .expect("wallet must remain registered")
                .tracked_asset_locks
                .insert(
                    out_point,
                    TrackedAssetLock {
                        out_point,
                        transaction: transaction.clone(),
                        account_index: 0,
                        funding_type: AssetLockFundingType::IdentityRegistration,
                        identity_index: 0,
                        amount: 1_000_000,
                        status: AssetLockStatus::InstantSendLocked,
                        proof: Some(instant_proof.clone()),
                    },
                );
        }

        InstantReconciliationContext {
            manager,
            wallet_manager,
            wallet_id,
            transaction,
            out_point,
            instant_proof,
        }
    }

    /// The unauthenticated code-24 consensus response that puts
    /// `reconcile_asset_lock_submit_result` on the reconciliation path.
    fn already_consumed_error(out_point: OutPoint) -> dash_sdk::Error {
        dash_sdk::Error::Protocol(dpp::ProtocolError::ConsensusError(Box::new(
            IdentityAssetLockTransactionOutPointAlreadyConsumedError::new(
                out_point.txid,
                out_point.vout as usize,
            )
            .into(),
        )))
    }

    /// The downgrade outcome both promotion-failure shapes must share:
    /// `reconcile_asset_lock_submit_result` (called with `None`, exactly
    /// what every production call site passes) still returns the typed
    /// code-24 `AssetLockAlreadyConsumed`, and the tracked row keeps what
    /// it had — no ChainLock proof was obtainable, so nothing may claim
    /// consumption-unknown state, and a later retry can still pick the
    /// proof up.
    async fn assert_downgraded_to_already_consumed(ctx: &InstantReconciliationContext) {
        let error = ctx
            .manager
            .reconcile_asset_lock_submit_result::<()>(
                Err(already_consumed_error(ctx.out_point)),
                &ctx.out_point,
                &ctx.instant_proof,
                None,
            )
            .await
            .expect_err("an already-consumed report always ends as a typed error");

        assert!(
            matches!(
                error,
                PlatformWalletError::AssetLockAlreadyConsumed(actual) if actual == ctx.out_point
            ),
            "a failed IS→CL promotion must DOWNGRADE to the code-24 signal the hosts \
             branch on — not propagate the promotion's own error, got {error:?}"
        );

        let wm = ctx.wallet_manager.read().await;
        let lock = wm
            .get_wallet_info(&ctx.wallet_id)
            .expect("wallet")
            .tracked_asset_locks
            .get(&ctx.out_point)
            .expect("lock stays tracked");
        assert_eq!(
            lock.status,
            AssetLockStatus::InstantSendLocked,
            "without a chain proof the lock must NOT be promoted to RecoveredFromChain"
        );
        assert_eq!(
            lock.proof,
            Some(ctx.instant_proof.clone()),
            "the tracked proof must be untouched by a failed promotion"
        );
    }

    /// Regression: the already-consumed reconciliation must TERMINATE when
    /// the ChainLock it wants never arrives.
    ///
    /// Shape: the funding transaction is present and tracked and its record
    /// is registered but not in a chain-locked block, the effective proof
    /// is an InstantSend proof (so the IS→CL promotion runs and dispatches
    /// to `wait_for_chain_lock`), and no SPV chainlock is ever delivered.
    ///
    /// Before the fix a `None` reconciliation timeout meant "wait forever"
    /// and this future never resolved. Under FFI that is a permanently
    /// pinned host thread, since every production call site is reached
    /// through `runtime().block_on(...)`. The realistic trigger is
    /// ordinary: a lock consumed seconds after broadcast is IS-locked but
    /// not yet chain-locked (~2.5 min away), and never chain-locked at all
    /// when the device is offline.
    ///
    /// `start_paused` lets the runtime auto-advance the bounded sleep, so
    /// the assertion is that the call resolves at all — and resolves as the
    /// typed code-24 `AssetLockAlreadyConsumed` the hosts branch on, not as
    /// the `FinalityTimeout` of the failed promotion.
    #[tokio::test(start_paused = true)]
    async fn already_consumed_reconciliation_terminates_without_a_chainlock() {
        use key_wallet::account::account_type::StandardAccountType;
        use key_wallet::account::AccountType;
        use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
        use key_wallet::managed_account::transaction_record::{
            TransactionDirection, TransactionRecord,
        };
        use key_wallet::transaction_checking::{TransactionContext, TransactionType};

        let ctx = instant_reconciliation_context().await;

        // Register the funding tx's record (mempool context, NOT chain-
        // locked) under BIP44 account 0, so the promotion's record lookup
        // succeeds and it genuinely dispatches to `wait_for_chain_lock`.
        // Without this the lookup misses and the promotion fast-fails with
        // `AssetLockProofWait` before any waiting — the OTHER regression's
        // scenario, which must stay distinct from this one.
        {
            let record = TransactionRecord::new(
                ctx.transaction.clone(),
                AccountType::Standard {
                    index: 0,
                    standard_account_type: StandardAccountType::BIP44Account,
                },
                TransactionContext::Mempool,
                TransactionType::Standard,
                TransactionDirection::Outgoing,
                Vec::new(),
                Vec::new(),
                0,
            );
            let mut wm = ctx.wallet_manager.write().await;
            wm.get_wallet_info_mut(&ctx.wallet_id)
                .expect("wallet must remain registered")
                .core_wallet
                .accounts
                .standard_bip44_accounts
                .get_mut(&0)
                .expect("funded fixture has BIP44 account 0")
                .transactions_mut()
                .insert(ctx.out_point.txid, record);
        }

        // Pin the SCENARIO, not just the outcome: the promotion itself must
        // burn the bound and report `FinalityTimeout` (auto-advanced under
        // `start_paused`), proving this test exercises the timeout arm of
        // the reconciliation catch-all and not the fast-fail one.
        let promotion_err = ctx
            .manager
            .upgrade_to_chain_lock_proof(&ctx.out_point, Some(RECONCILIATION_CHAIN_LOCK_TIMEOUT))
            .await
            .expect_err("no ChainLock ever arrives: the promotion must fail");
        assert!(
            matches!(
                promotion_err,
                PlatformWalletError::FinalityTimeout(actual) if actual == ctx.out_point
            ),
            "expected the promotion to time out waiting for a ChainLock, got {promotion_err:?}"
        );

        assert_downgraded_to_already_consumed(&ctx).await;
    }

    /// Companion regression pinning the DELIBERATE breadth of the
    /// promotion's `Err` catch-all in `reconcile_asset_lock_submit_result`
    /// (see the comment on that arm): a NON-timeout promotion failure must
    /// be downgraded to the code-24 signal exactly like a timeout, because
    /// the already-consumed classification came from Platform's
    /// outpoint-matched consensus error — a failed local lookup does not
    /// invalidate it, and the non-timeout failures occur precisely in the
    /// degraded-local-state scenarios where the host's code-24 branch is
    /// the only path that can still resolve the operation.
    ///
    /// Shape: the lock is tracked, but its transaction record is
    /// unavailable — never registered in any account's in-memory map (the
    /// fixture never broadcasts) and unknown to the persister
    /// (`NoopTestPersister` keeps the trait's `Ok(None)` default). That is
    /// the post-restore / wallet-state-mismatch shape, and the promotion
    /// fast-fails with `AssetLockProofWait` instead of waiting.
    ///
    /// "Fixing" the catch-all to propagate everything but
    /// `FinalityTimeout` turns the reconcile result below into
    /// `AssetLockProofWait` and fails this test — that narrowing was
    /// proposed and declined in review (finding 9237664c50df); this test
    /// keeps the decision from silently regressing.
    #[tokio::test(start_paused = true)]
    async fn already_consumed_reconciliation_downgrades_non_timeout_promotion_failure() {
        let ctx = instant_reconciliation_context().await;

        // Pin the SCENARIO first: with the record unavailable, the
        // promotion must fail with the NON-timeout `AssetLockProofWait`
        // fast-fail. If a future fixture change made the record findable,
        // this assertion — not a silently green downgrade check — fails.
        let promotion_err = ctx
            .manager
            .upgrade_to_chain_lock_proof(&ctx.out_point, Some(RECONCILIATION_CHAIN_LOCK_TIMEOUT))
            .await
            .expect_err("record unavailable: the promotion must fail");
        assert!(
            matches!(promotion_err, PlatformWalletError::AssetLockProofWait(_)),
            "expected the non-timeout AssetLockProofWait fast-fail, got {promotion_err:?}"
        );

        assert_downgraded_to_already_consumed(&ctx).await;
    }
}
