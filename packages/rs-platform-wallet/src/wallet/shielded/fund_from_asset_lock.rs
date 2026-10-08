//! Orchestrated shielded funding from a Core asset lock.
//!
//! Mirrors `wallet/platform_addresses/fund_from_asset_lock.rs` but
//! credits the *shielded* pool (Type 18 `ShieldFromAssetLock`) instead
//! of platform addresses (Type 14 `AddressFundingFromAssetLock`).
//!
//! ## Pipeline
//!
//! 1. **Pre-flight** — exactly-one recipient today (the multi-shape
//!    `Vec<(OrchardAddress, Credits)>` API is in place so the caller
//!    signature doesn't change when DPP grows multi-output Orchard
//!    bundles for Type 18; see [`validate_shielded_recipients`]).
//! 2. **Resolve funding** — delegate to the shared
//!    [`AssetLockManager::resolve_funding_with_is_timeout_fallback`].
//!    The Orchard bundle is proved concurrently, on the blocking pool:
//!    it commits to the locked outpoint and value, not to the lock proof,
//!    so the proof starts once the asset-lock transaction is broadcast
//!    instead of after its InstantSend lock arrives.
//! 3. **Submit** — wrap the build-and-broadcast in
//!    `submit_with_cl_height_retry`; each attempt wraps the proved bundle
//!    with [`ProvedShieldFromAssetLockBundle::build_transition_with_signer`]
//!    so the asset-lock-proof signature is routed through the external
//!    `key_wallet::signer::Signer` (the host never sees the raw key).
//!    IS→CL fallback fires on Platform-side IS rejection
//!    (`is_instant_lock_proof_invalid`); it and the CL-height retries reuse
//!    the proved bundle rather than proving again.
//! 4. **Finalize lock state** — a verified submit consumes the tracked
//!    outpoint. An unauthenticated "already consumed" rejection is recorded
//!    only as nonterminal consumption-unknown state after Core ChainLock
//!    finality; it never creates a terminal `Consumed` tombstone.

use dash_sdk::platform::transition::broadcast::BroadcastStateTransition;
use dash_sdk::platform::transition::put_settings::PutSettings;
use dpp::address_funds::{OrchardAddress, PlatformAddress};
use dpp::balances::credits::CREDITS_PER_DUFF;
use dpp::fee::Credits;
use dpp::prelude::AssetLockProof;
use dpp::shielded::builder::{OrchardProver, ProvedShieldFromAssetLockBundle};
use dpp::shielded::compute_minimum_shielded_fee;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::ProtocolError;
use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;

use crate::wallet::asset_lock::tracked::TrackedAssetLock;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::error::is_instant_lock_proof_invalid;
use crate::wallet::asset_lock::orchestration::{
    out_point_from_proof, submit_with_cl_height_retry, AssetLockFunding, FundingResolution,
    ResolvedFunding,
};
use crate::wallet::PlatformWallet;
use crate::PlatformWalletError;

/// On-wire Orchard action count for a `ShieldFromAssetLock` bundle with
/// `dummy_outputs` anonymity-set fillers appended after the single real
/// output.
///
/// `build_output_only_bundle` configures Orchard's
/// `BundleType::Transactional { flags: SPENDS_DISABLED, bundle_required: false }`.
/// For `1 + dummy_outputs` outputs and zero spends, Orchard's `num_actions`
/// is `max(1 + dummy_outputs, MIN_ACTIONS)` where `MIN_ACTIONS == 2`. Consensus
/// prices the flat shielded fee from the on-wire `actions.len()`
/// (`transform_into_action` Step 3b), so the wallet's fee reservation MUST be
/// computed from this exact count or the transition is rejected — see the
/// `validate_structure` / `transform_into_action` checks in rs-dpp / rs-drive-abci.
///
/// With `dummy_outputs == 0` this returns `2`, the historical single-output count.
pub(crate) fn shield_from_asset_lock_num_actions(dummy_outputs: usize) -> usize {
    (1 + dummy_outputs).max(2)
}

impl PlatformWallet {
    /// Fund the shielded pool from a Core L1 asset lock, with the
    /// asset-lock proof signed by an external
    /// `key_wallet::signer::Signer` (atomic derive + sign + zeroise
    /// inside the signer's trust boundary).
    ///
    /// # Arguments
    ///
    /// * `funding` — How to source the asset lock. `FromWalletBalance`
    ///   builds a fresh asset lock from Core UTXOs; `FromExistingAssetLock`
    ///   resumes from a tracked outpoint (after relaunch or a stuck
    ///   broadcast).
    /// * `recipients` — Recipient list, shape
    ///   `Vec<(OrchardAddress, Option<Credits>)>` mirroring the
    ///   platform-address Type 14 API. Today the pre-flight enforces
    ///   exactly one recipient with `None` credits — that recipient
    ///   receives the lock value minus the flat `pool_fee`
    ///   (`compute_minimum_shielded_fee(2) + asset_lock_base_cost`).
    ///
    ///   When DPP grows multi-output Orchard bundles for Type 18,
    ///   `Some(_)` values will be honored (explicit credit amounts
    ///   pass through; the single `None` bucket — if any — receives
    ///   the residual). Keeping the multi-shape signature now means
    ///   no caller migration is needed at that point.
    ///
    ///   Unlike Type 14, Type 18 has no protocol-side
    ///   `AddressFundsFeeStrategy` — the Orchard `value_balance`
    ///   (= recipient credits) is baked into the Halo 2 proof at
    ///   build time. The wallet handles the math here so callers
    ///   don't have to know about protocol-level fee constants.
    /// * `asset_lock_signer` — External signer for the outer ECDSA
    ///   signature on the state transition. The raw key never crosses
    ///   the FFI boundary.
    /// * `prover` — Orchard prover (holds the Halo 2 proving key). Owned
    ///   (`'static`) because the proof runs on tokio's blocking pool, and
    ///   starts as soon as the asset-lock transaction is broadcast — while
    ///   the InstantSend lock is awaited — rather than after it. Pass
    ///   `&CachedOrchardProver` (a `&'static` to the zero-sized handle).
    /// * `surplus_output` — Optional platform address that receives the
    ///   asset-lock surplus (`lock_value − shield_amount − pool_fee`).
    ///
    ///   In this orchestrated single-recipient "remainder" flow the
    ///   surplus is structurally **zero**: `shield_amount` is derived as
    ///   `lock_value − pool_fee` (see Step 3), so the consensus surplus
    ///   `lock_value − shield_amount − pool_fee == 0`. With a zero
    ///   surplus, `None` is always consensus-valid (`0 ≤
    ///   shielded_implicit_fee_cap`) and any `surplus_output` the caller
    ///   supplies simply receives 0 credits.
    ///
    ///   It is threaded through to the DPP builder for API completeness
    ///   and forward-compatibility (multi-output / explicit-amount
    ///   bundles, where a real surplus can arise). Because `shield_amount`
    ///   is re-derived deterministically from the on-chain lock value and
    ///   the versioned fee constants, a fresh build and any subsequent
    ///   resume commit to the same `shield_amount` (hence the same zero
    ///   surplus) regardless of the resume call's `surplus_output` — so
    ///   the surplus destination cannot desync the in-flight operation
    ///   even though each attempt re-signs the bundle with fresh randomness.
    /// * `settings` — Optional `PutSettings`; `user_fee_increase` is
    ///   bumped by the CL-height retry wrapper on consensus 10506.
    /// * `cl_wait` — ChainLock-fallback wait policy (`Option<Duration>`).
    ///   User-facing funding passes `None` to wait for the ChainLock
    ///   **indefinitely** (a broadcast lock is pending, never failed). The
    ///   shielded seed pool passes `Some(CL_FALLBACK_TIMEOUT)` so a
    ///   `FinalityTimeout` surfaces as its unconfirmed-ancestor pacing
    ///   signal (see `seed_pool.rs`).
    #[cfg(feature = "shielded")]
    #[allow(clippy::too_many_arguments)]
    pub async fn shielded_fund_from_asset_lock<AS, P>(
        &self,
        coordinator: &std::sync::Arc<crate::wallet::shielded::NetworkShieldedCoordinator>,
        funding: AssetLockFunding,
        recipients: Vec<(OrchardAddress, Option<Credits>)>,
        asset_lock_signer: &AS,
        prover: P,
        surplus_output: Option<PlatformAddress>,
        dummy_outputs: usize,
        settings: Option<PutSettings>,
        cl_wait: Option<Duration>,
    ) -> Result<(), PlatformWalletError>
    where
        AS: ::key_wallet::signer::ExtendedPubKeySigner + Send + Sync,
        P: OrchardProver + Send + Sync + 'static,
    {
        // On-wire Orchard action count = max(1 real + dummy_outputs, 2). Consensus
        // prices the flat shielded fee from this count, so the wallet's fee
        // reservation below is derived from the SAME value (any mismatch is rejected).
        let num_actions = shield_from_asset_lock_num_actions(dummy_outputs);
        // Step 1: pre-flight. Failing fast here avoids broadcasting
        // an unfundable asset-lock tx (or paying for an Orchard proof
        // build, ~30s, only to reject downstream).
        validate_shielded_recipients(&recipients)?;

        // Pre-broadcast sizing guard for the `FromWalletBalance` path:
        // refuse to build an L1 asset-lock that can't even cover the
        // protocol min-fee for Type 18. Without this check the lock
        // gets broadcast in Step 2, then Step 3's `checked_sub`
        // underflows and we return an error with the L1 outpoint
        // already on-chain — a Resume on the orphaned lock
        // deterministically hits the same underflow, so the funds
        // can't be recovered through this code path.
        //
        // The Step 3 check (after `resolve_funding_*`) is still the
        // authoritative safety net for the `FromExistingAssetLock`
        // resume path, where the lock is already on-chain and the
        // sizing decision was made by a prior caller. Here we only
        // protect the fresh-build path.
        if let AssetLockFunding::FromWalletBalance { amount_duffs, .. } = &funding {
            let lock_credits = (*amount_duffs)
                .checked_mul(CREDITS_PER_DUFF)
                .ok_or_else(|| {
                    PlatformWalletError::ShieldedBuildError(format!(
                        "asset lock amount overflows credits conversion ({amount_duffs} duffs * \
                     {CREDITS_PER_DUFF} credits/duff > u64::MAX)"
                    ))
                })?;
            let pool_fee_credits = self.shield_from_asset_lock_pool_fee(num_actions)?;
            if lock_credits <= pool_fee_credits {
                return Err(PlatformWalletError::ShieldedBuildError(format!(
                    "asset lock ({lock_credits} credits, from {amount_duffs} duffs) is at or \
                     below the ShieldFromAssetLock pool fee ({pool_fee_credits} credits = \
                     shielded fee + asset_lock_base_cost) — refusing to broadcast a single-use \
                     L1 outpoint that would be unrecoverable on resume"
                )));
            }
        }

        // Drain path: stamp the authoritative lock-value floor into the
        // funding request. The drained value is only knowable from the BUILT
        // payload (a pre-build balance estimate races concurrent
        // reservations and the builder's own selection filters), so the
        // asset-lock pipeline enforces this floor post-build / pre-broadcast
        // and abandons an undersized build with an owner-guarded reservation
        // release — a single-use L1 outpoint that could never clear the
        // Type 18 pool fee is never created. The floor is the smallest
        // whole-duff value STRICTLY above the pool fee, mirroring the
        // `lock_value − pool_fee > 0` consumability requirement in Step 3.
        let funding = match funding {
            AssetLockFunding::DrainAccountBalance {
                account,
                minimum_lock_duffs: _,
            } => {
                let pool_fee_credits = self.shield_from_asset_lock_pool_fee(num_actions)?;
                let minimum_lock_duffs = pool_fee_credits / CREDITS_PER_DUFF + 1;
                AssetLockFunding::DrainAccountBalance {
                    account,
                    minimum_lock_duffs: Some(minimum_lock_duffs),
                }
            }
            other => other,
        };

        // Single-flight: serialise shield-class operations on this
        // wallet so two concurrent calls can't race the asset-lock
        // tracker into a half-consumed state.
        let _shield_guard = self.shield_guard.lock().await;

        let (recipient, _) = *recipients.first().expect("preflight enforces len() == 1");

        // Encrypt the output under this wallet's OVK so the shielded sync can
        // recover the funding (recipient, value, memo) from chain data alone.
        // Prefer the bound account whose IVK recognizes the recipient address
        // (the sent-note row then lands under that account); fall back to the
        // lowest bound account, or `None` (unrecoverable out_ciphertext) if
        // the shielded sub-wallet isn't bound. Read once, before the funding
        // resolves, so the speculative proof and any re-proof commit to the
        // same OVK.
        let sender_ovk = {
            let guard = self.shielded_keys.read().await;
            guard.as_ref().and_then(|keys| {
                keys.values()
                    .find(|ks| {
                        ks.incoming_viewing_key
                            .diversifier_index(recipient.inner())
                            .is_some()
                    })
                    .or_else(|| keys.values().next())
                    .map(|ks| ks.outgoing_viewing_key.clone())
            })
        };

        // Proves the Orchard bundle for a target (outpoint, amount, protocol
        // version) — on the blocking pool, see `ProofTask`.
        let prove: ProveFn<ProvedShieldFromAssetLockBundle> = Arc::new(move |target| {
            ProvedShieldFromAssetLockBundle::prove(
                &recipient,
                target.shield_amount,
                target.out_point,
                &prover,
                [0u8; 36],
                sender_ovk.clone(),
                dummy_outputs,
                target.platform_version,
            )
        });

        // Step 2: resolve funding. `AssetLockShieldedAddressTopUp`
        // selects the BIP44 funding family dedicated to shielded
        // top-ups (`accounts.asset_lock_shielded_address_topup` —
        // distinct from the platform-address bucket Type 14 uses);
        // see `wallet/asset_lock/build.rs` for the source-account
        // selection, `sync/recovery.rs` for resume-time key re-
        // derivation, and `manager/accessors.rs` for the
        // persistence/UI tag (`fundingTypeRaw == 5`).
        // `destination_index = 0` is unused for this funding type.
        //
        // The Orchard bundle commits to the locked outpoint and value but not
        // to the lock's proof (see `ProvedShieldFromAssetLockBundle`), and
        // both are known once the asset-lock transaction is broadcast. So the
        // proof — seconds of CPU — starts then, on the blocking pool, and runs
        // while the resolver waits for the InstantSend lock; the submit below
        // picks it up if the resolved lock is the one it was proved for. If
        // the resolution fails, dropping the speculative task discards it.
        let (out_point_tx, out_point_rx) = tokio::sync::oneshot::channel();
        let (resolution, speculative) = resolve_while_proving(
            self.asset_locks.resolve_funding_observing_out_point(
                funding,
                AssetLockFundingType::AssetLockShieldedAddressTopUp,
                /* destination_index */ 0,
                asset_lock_signer,
                move |out_point| {
                    let _ = out_point_tx.send(out_point);
                },
            ),
            out_point_rx,
            |out_point| self.start_speculative_shield_proof(out_point, num_actions, &prove),
        )
        .await?;
        let ResolvedFunding {
            proof,
            path,
            tracked_out_point,
        } = match resolution {
            FundingResolution::Resolved(rf) => rf,
            FundingResolution::IsTimeout { out_point } => {
                tracing::warn!(
                    "IS-lock did not propagate within 300s for shielded fund-from-asset-lock \
                     (tx {}), falling back to ChainLock proof",
                    out_point.txid
                );
                // Persists the rebuilt proof before resuming, so a later
                // resume of this lock does not fall back into the
                // record-only proof wait that just timed out.
                let (chain_proof, path) = self
                    .asset_locks
                    .resolve_chain_proof_after_is_timeout(&out_point, cl_wait)
                    .await?;
                ResolvedFunding {
                    proof: chain_proof,
                    path,
                    tracked_out_point: Some(out_point),
                }
            }
        };

        // Step 3: derive `shield_amount` from the asset-lock value.
        //
        // Unlike Type 14 (where Platform deducts the fee inside the
        // transition via `AddressFundsFeeStrategy`), Type 18 bakes
        // the Orchard `value_balance` into the Halo 2 proof at build
        // time — someone *has* to know the precise number before
        // signing. The wallet is the right place: it already has the
        // lock value (from the IS proof's TxOut, or from the asset-
        // lock manager's tracked row for CL-only paths) and the
        // protocol min-fee constant (from `PlatformVersion`).
        //
        // Single-recipient + `None` semantics today: the recipient
        // receives `lock_value - pool_fee`. Future multi-recipient
        // would honor `Some(_)` values explicitly and route the
        // residual to the (sole) `None` bucket; the preflight will
        // change in lockstep with the DPP-side multi-output bundle
        // builder.
        let asset_lock_value_credits =
            lookup_asset_lock_value_credits(self, &proof, tracked_out_point.as_ref()).await?;
        // `pool_fee = compute_minimum_shielded_fee(2) + asset_lock_base_cost` — the SAME flat fee
        // consensus charges (`transform_into_action` Step 3b). Deriving `shield_amount =
        // lock_value − pool_fee` reserves room for the fee and pins the consensus surplus
        // (`lock_value − shield_amount − pool_fee`) to exactly zero.
        let pool_fee_credits = self.shield_from_asset_lock_pool_fee(num_actions)?;
        let shield_amount = shield_amount_after_fee(asset_lock_value_credits, pool_fee_credits)?;

        // Surplus is structurally zero in this remainder flow (`shield_amount == lock_value −
        // pool_fee`), so `None` is always consensus-valid. Defensively assert the cap invariant
        // and surface a clear error rather than building a transition consensus would reject —
        // this guards future code paths that might leave a non-zero residual.
        let surplus = asset_lock_value_credits
            .checked_sub(shield_amount)
            .and_then(|v| v.checked_sub(pool_fee_credits))
            .unwrap_or(0);
        if surplus_output.is_none() {
            let implicit_fee_cap = self
                .sdk
                .version()
                .drive_abci
                .validation_and_processing
                .event_constants
                .shielded_implicit_fee_cap;
            if surplus > implicit_fee_cap {
                return Err(PlatformWalletError::ShieldedBuildError(format!(
                    "ShieldFromAssetLock surplus ({surplus} credits) exceeds the implicit fee cap \
                     ({implicit_fee_cap} credits) and no surplus_output address was supplied — \
                     consensus would reject this transition; pass a surplus_output to receive the \
                     remainder"
                )));
            }
        }

        // Step 4: submit. Two Platform-side fallback layers — matching
        // the address-funding sibling: CL-height-too-low retries bump
        // `user_fee_increase` (bypasses Tenderdash's invalid-tx hash
        // cache) and IS-lock rejection triggers an IS→CL upgrade on
        // the same outpoint.
        //
        // Every attempt — the first, each CL-height retry, the IS→CL
        // fallback — reuses the one proved bundle (`BundleCache`): none of
        // them changes the outpoint or the amount the proof commits to.
        //
        // Subtle: `ShieldFromAssetLockTransition::set_user_fee_increase`
        // is a no-op, so the wrapper's bump cannot diversify the ST hash
        // here the way it does for address-funding. Retries still avoid
        // Tenderdash's invalid-tx cache because every assembly re-signs the
        // bundle with randomized RedPallas signatures. If that signing is
        // ever made deterministic, retries need another diversifier.
        let proof_out_point = out_point_from_proof(&proof);
        let sdk = self.sdk.clone();
        let bundles = tokio::sync::Mutex::new(BundleCache::new(prove, speculative));
        // Serialized actions of the bundle that actually landed — fed to
        // the live activity recorder below. Each attempt re-signs the
        // bundle, so only the landed attempt's actions (signatures
        // included) are the ones on chain; `build_and_broadcast_shielded`
        // returns them on success.
        let (submit_result, effective_proof) = match submit_with_cl_height_retry(settings, |s| {
            build_and_broadcast_shielded(
                sdk.clone(),
                &bundles,
                shield_amount,
                proof.clone(),
                path.clone(),
                asset_lock_signer,
                surplus_output,
                s,
            )
        })
        .await
        {
            Ok(actions) => (Ok(actions), proof.clone()),
            Err(e) if is_instant_lock_proof_invalid(&e) => {
                let out_point = proof_out_point;
                tracing::warn!(
                    "IS-lock proof rejected by Platform for shielded fund-from-asset-lock \
                     (tx {}), retrying with ChainLock proof",
                    out_point.txid
                );
                let chain_proof = self
                    .asset_locks
                    .upgrade_to_chain_lock_proof(&out_point, cl_wait)
                    .await?;
                let cs = self
                    .asset_locks
                    .advance_asset_lock_status(
                        &out_point,
                        crate::wallet::asset_lock::tracked::AssetLockStatus::ChainLocked,
                        Some(chain_proof.clone()),
                    )
                    .await?;
                self.asset_locks.queue_asset_lock_changeset(cs);
                let submit_result = submit_with_cl_height_retry(settings, |s| {
                    build_and_broadcast_shielded(
                        sdk.clone(),
                        &bundles,
                        shield_amount,
                        chain_proof.clone(),
                        path.clone(),
                        asset_lock_signer,
                        surplus_output,
                        s,
                    )
                })
                .await;
                (submit_result, chain_proof)
            }
            Err(e) => (Err(e), proof.clone()),
        };
        // Release the proved bundle (and, if no attempt ever needed it, the
        // speculative proof) before the reconciliation awaits below. The
        // bundle holds its signing secrets (`ProvedShieldFromAssetLockBundle`)
        // from the moment it is proved — possibly through the 300 s
        // InstantSend window and an unbounded ChainLock wait — and this is
        // where they go; every early return above drops them the same way.
        drop(bundles);

        // Whichever proof was submitted, a persisted Chain proof Platform
        // places the transaction outside of must come off the row — including
        // one this resume loaded rather than built, which would otherwise be
        // replayed on every later attempt.
        if let Err(e) = &submit_result {
            self.asset_locks
                .invalidate_rejected_chain_proof(&proof_out_point, &effective_proof, e)
                .await;
        }

        let landed_actions: Vec<dpp::shielded::SerializedAction> = self
            .asset_locks
            .reconcile_asset_lock_submit_result(
                submit_result,
                &proof_out_point,
                &effective_proof,
                cl_wait,
            )
            .await?;

        // Record a live `ShieldFromAssetLock` activity entry over the
        // landed bundle. One entry per call (= one per seed-pool batch),
        // `direction in`, amount = the real shielded note value (dummy
        // fillers contribute no visible output cmx, so they're excluded
        // by construction). Recorded Confirmed directly — `broadcast_and_
        // _wait` already proved inclusion. Best-effort: a recording miss
        // (no bound keyset, no recoverable output) just omits the row.
        self.record_shield_from_asset_lock_activity(coordinator, &landed_actions, shield_amount)
            .await;

        // Step 5: cleanup. Consume the tracked asset lock. The
        // shielded note itself arrives via the next sync — there's
        // no immediate balance changeset to persist (unlike
        // address-funding, which writes proof-attested balances back
        // into `ManagedPlatformAccount`).
        if let Some(out_point) = tracked_out_point {
            // Platform DID accept the shield ST — propagating an Err
            // here would misreport the protocol outcome. The lock row
            // stays non-Consumed and surfaces in the Resumable
            // Funding list; a user Resume on it would be
            // deterministically rejected by Platform with "lock
            // already consumed". Log so it's visible.
            if let Err(e) = self.asset_locks.consume_asset_lock(&out_point).await {
                match &e {
                    PlatformWalletError::WalletNotFound(_) => {
                        tracing::warn!(
                            outpoint = %out_point,
                            error = %e,
                            "consume_asset_lock: wallet handle vanished after successful shielded submit"
                        );
                    }
                    _ => {
                        tracing::error!(
                            outpoint = %out_point,
                            error = %e,
                            "consume_asset_lock failed unexpectedly after successful shielded submit; \
                             the lock row stays non-Consumed and will surface as Resumable. \
                             A user Resume on it will be rejected by Platform with 'lock already consumed'."
                        );
                    }
                }
            }
        }

        tracing::info!(
            shield_amount,
            asset_lock_value_credits,
            pool_fee_credits,
            "Shielded fund-from-asset-lock succeeded"
        );

        Ok(())
    }

    /// Start proving the bundle for the asset lock at `out_point` while its
    /// InstantSend lock is still awaited (see [`ProofTask`]).
    ///
    /// The amount is derived exactly as after the resolution — the lock's
    /// value minus the pool fee — but from the tracked row, as the lock proof
    /// does not exist yet. Returns `None`, starting nothing, when the value is
    /// unknown or cannot fund the shield; the resolved path then reports that
    /// as it always has. A proof started for a target the resolved lock does
    /// not match is discarded, never used.
    async fn start_speculative_shield_proof(
        &self,
        out_point: dashcore::OutPoint,
        num_actions: usize,
        prove: &ProveFn<ProvedShieldFromAssetLockBundle>,
    ) -> Option<ProofTask<ProvedShieldFromAssetLockBundle>> {
        let value_duffs = tracked_asset_lock_value_duffs(self, &out_point)
            .await
            .ok()?;
        let value_credits = value_duffs.checked_mul(CREDITS_PER_DUFF)?;
        let pool_fee_credits = self.shield_from_asset_lock_pool_fee(num_actions).ok()?;
        let shield_amount = shield_amount_after_fee(value_credits, pool_fee_credits).ok()?;
        tracing::debug!(
            %out_point,
            shield_amount,
            "proving the shield-from-asset-lock bundle while the asset lock proof is awaited"
        );
        Some(ProofTask::spawn(
            BundleTarget {
                out_point,
                shield_amount,
                platform_version: self.sdk.version(),
            },
            Arc::clone(prove),
        ))
    }

    /// The flat pool fee for a `ShieldFromAssetLock` (Type 18) state
    /// transition, in credits.
    ///
    /// Mirrors the consensus fee (`transform_into_action` Step 3b):
    ///
    /// ```text
    /// pool_fee = compute_minimum_shielded_fee(num_actions)  [Halo2 proof + per-action]
    ///          + asset_lock_base_cost                        [L1 asset-lock processing]
    /// ```
    ///
    /// `num_actions` is the on-wire Orchard action count of the bundle
    /// (`shield_from_asset_lock_num_actions(dummy_outputs)` — `2` for the
    /// classic single-output bundle, up to `6` for a pool-seeding batch
    /// (the 20 KiB `max_state_transition_size` cap, see
    /// `MAX_ACTIONS_PER_BATCH` in `seed_pool.rs`; not the 16-action
    /// consensus cap)).
    /// `asset_lock_base_cost` (`albc`) is the same constant Type 14 (address
    /// funding) uses, read from `dpp.state_transitions.identities.asset_locks`
    /// and converted duffs→credits.
    pub(crate) fn shield_from_asset_lock_pool_fee(
        &self,
        num_actions: usize,
    ) -> Result<Credits, PlatformWalletError> {
        let pv = self.sdk.version();
        let albc_duffs = pv
            .dpp
            .state_transitions
            .identities
            .asset_locks
            .required_asset_lock_duff_balance_for_processing_start_for_address_funding;
        let albc = albc_duffs.checked_mul(CREDITS_PER_DUFF).ok_or_else(|| {
            PlatformWalletError::ShieldedBuildError(format!(
                "asset_lock_base_cost constant overflowed credits conversion \
                 ({albc_duffs} duffs * {CREDITS_PER_DUFF} credits/duff > u64::MAX)"
            ))
        })?;
        let shielded_fee = compute_minimum_shielded_fee(num_actions, pv).map_err(|e| {
            PlatformWalletError::ShieldedBuildError(format!(
                "failed to compute minimum shielded fee for ShieldFromAssetLock: {e}"
            ))
        })?;
        shielded_fee.checked_add(albc).ok_or_else(|| {
            PlatformWalletError::ShieldedBuildError(format!(
                "ShieldFromAssetLock pool fee overflowed credits conversion \
                 (shielded_fee {shielded_fee} + asset_lock_base_cost {albc} > u64::MAX)"
            ))
        })
    }

    /// Record a confirmed `ShieldFromAssetLock` (Type 18) activity entry
    /// over the landed bundle's `actions`.
    ///
    /// Deliberately records ONLY the landed bundle (no Pending row before
    /// broadcast, no Failed row after): the attempts usually share one
    /// proved bundle, but an attempt may need a fresh proof (a protocol
    /// upgrade between attempts changes the binding), and a fresh proof has
    /// different output cmxs — and the activity id is keyed to those cmxs.
    /// A pre-broadcast Pending row would then orphan (unconfirmable
    /// forever, its cmxs never on-chain) whenever the re-proved attempt is
    /// the one that lands. In-flight and failed Type 18s are
    /// surfaced through the tracked asset-lock lifecycle instead
    /// (Built/Broadcast/Locked/Consumed + the resumable-funding UI),
    /// which tracks the L1 lock — the artifact that actually carries the
    /// recoverable value on failure.
    ///
    /// Best-effort and non-fatal: the broadcast already succeeded, so a
    /// recording miss (no bound shielded keyset, or no wallet-visible
    /// output cmx in the bundle) must not turn the funding into a
    /// failure — it just omits the activity row (a later scan still
    /// surfaces the note via OVK recovery). Finds the keyset whose IVK
    /// recognizes the funded note's recipient (the row then lands under
    /// that account), falling back to the lowest bound account — mirrors
    /// the `sender_ovk` selection above.
    #[cfg(feature = "shielded")]
    async fn record_shield_from_asset_lock_activity(
        &self,
        coordinator: &std::sync::Arc<crate::wallet::shielded::NetworkShieldedCoordinator>,
        actions: &[dpp::shielded::SerializedAction],
        shield_amount: Credits,
    ) {
        use crate::wallet::shielded::activity::{
            ShieldedActivityKind, ShieldedActivityStatus, ShieldedDirection,
        };
        use crate::wallet::shielded::activity_recorder::{build_pending_entry, with_status};

        let guard = self.shielded_keys.read().await;
        let Some(keys_map) = guard.as_ref() else {
            return;
        };
        // Prefer the account whose keyset actually recognizes a visible
        // output in the landed bundle (the funded note decrypts under its
        // IVK / recovers under its OVK) — the row must land under THAT
        // account or the recorder builds with the wrong keys, recovers no
        // cmx, and silently drops the entry; it would also break the
        // shared-id natural key against the eventual scan-derived row.
        // Fall back to the lowest bound account only when nothing
        // matches (a shield to a fully external recipient).
        let matched = keys_map.iter().find(|(_, ks)| {
            !crate::wallet::shielded::activity_recorder::visible_output_cmxs(actions, ks).is_empty()
        });
        let Some((&account, keyset)) = matched.or_else(|| keys_map.iter().next()) else {
            return;
        };

        let Some(pending) = build_pending_entry(
            keyset,
            crate::wallet::shielded::activity_recorder::LiveEntryParams {
                kind: ShieldedActivityKind::ShieldFromAssetLock,
                direction: ShieldedDirection::In,
                amount: shield_amount,
                // The flat pool fee is charged on the L1 side (asset-lock
                // value − shield_amount); the note value is exactly
                // `shield_amount`, so no shielded-pool fee is derivable
                // from the bundle here.
                fee: None,
                counterparty: None,
                memo: None,
                actions,
                spent_notes: &[],
            },
        ) else {
            return;
        };

        let confirmed = with_status(&pending, ShieldedActivityStatus::Confirmed, None);
        let id = crate::wallet::shielded::SubwalletId::new(self.wallet_id(), account);
        crate::wallet::shielded::operations::queue_shielded_activity(
            coordinator.store(),
            Some(self.persister()),
            self.wallet_id(),
            id,
            confirmed,
        )
        .await;
    }
}

/// Look up the asset-lock value in credits.
///
/// Preference order:
/// 1. If the proof is `Instant`, read the credit output from the
///    transaction the proof carries — no manager lookup needed.
/// 2. Otherwise (the IS-timeout-fallback path produced a CL proof
///    that doesn't carry the tx output), look up the tracked
///    asset-lock row by outpoint and read it from its transaction.
///
/// Both go through [`credit_output_duffs`], as does the speculative
/// proof's amount (`start_speculative_shield_proof`), so a bundle proved
/// before the lock proof existed is proved for the amount computed here.
async fn lookup_asset_lock_value_credits(
    wallet: &PlatformWallet,
    proof: &AssetLockProof,
    tracked_out_point: Option<&dashcore::OutPoint>,
) -> Result<Credits, PlatformWalletError> {
    let duffs = match proof {
        AssetLockProof::Instant(is) => credit_output_duffs(is.transaction(), is.output_index())
            .ok_or_else(|| {
                PlatformWalletError::AddressSync(
                    "InstantAssetLockProof has no output at the indicated index".to_string(),
                )
            })?,
        AssetLockProof::Chain(_) => {
            let op = tracked_out_point.ok_or_else(|| {
                PlatformWalletError::AddressSync(
                    "ChainAssetLockProof but no tracked outpoint to look up value".to_string(),
                )
            })?;
            tracked_asset_lock_value_duffs(wallet, op).await?
        }
    };
    duffs.checked_mul(CREDITS_PER_DUFF).ok_or_else(|| {
        PlatformWalletError::ShieldedBuildError(format!(
            "asset lock value ({duffs} duffs * {CREDITS_PER_DUFF} credits/duff > u64::MAX)"
        ))
    })
}

/// The value of the tracked asset lock at `out_point`, in duffs: its
/// transaction's credit output at the outpoint's index (see
/// [`credit_output_duffs`]).
async fn tracked_asset_lock_value_duffs(
    wallet: &PlatformWallet,
    out_point: &dashcore::OutPoint,
) -> Result<u64, PlatformWalletError> {
    let locks: Vec<TrackedAssetLock> = wallet.asset_locks.list_tracked_locks().await;
    let lock = locks
        .iter()
        .find(|l| l.out_point == *out_point)
        .ok_or_else(|| {
            PlatformWalletError::AddressSync(format!(
                "tracked asset lock {} not found in manager",
                out_point
            ))
        })?;
    credit_output_duffs(&lock.transaction, out_point.vout).ok_or_else(|| {
        PlatformWalletError::AddressSync(format!(
            "tracked asset lock {} has no credit output at index {}",
            out_point, out_point.vout
        ))
    })
}

/// The value, in duffs, that an asset lock on `transaction` at `output_index`
/// locks: the payload's `credit_outputs[output_index]` — the output
/// `InstantAssetLockProof::output` names and consensus credits. `None` when
/// the transaction carries no such credit output.
///
/// Not the tracked row's `amount`, which sums all credit outputs (the same
/// number for the single-output locks this wallet builds). Reading the output
/// on both paths keeps the speculative and the resolved amounts equal.
fn credit_output_duffs(transaction: &dashcore::Transaction, output_index: u32) -> Option<u64> {
    use dashcore::blockdata::transaction::special_transaction::TransactionPayload;

    match &transaction.special_transaction_payload {
        Some(TransactionPayload::AssetLockPayloadType(payload)) => payload
            .credit_outputs
            .get(output_index as usize)
            .map(|output| output.value),
        _ => None,
    }
}

/// Build the Type 18 transition and broadcast-and-wait.
///
/// Extracted so `submit_with_cl_height_retry`'s closure stays compact
/// and the IS→CL fallback path can re-call it with the upgraded proof.
/// The Orchard bundle comes from `bundles`: the one already proved for
/// this lock (reused across attempts), else a fresh proof.
///
/// On success returns the **landed** bundle's serialized Orchard actions
/// so the orchestrator can record a live `ShieldFromAssetLock` activity
/// entry over the exact bundle that committed.
#[allow(clippy::too_many_arguments)]
async fn build_and_broadcast_shielded<AS>(
    sdk: std::sync::Arc<dash_sdk::Sdk>,
    bundles: &tokio::sync::Mutex<BundleCache<ProvedShieldFromAssetLockBundle>>,
    shield_amount: Credits,
    proof: AssetLockProof,
    path: ::key_wallet::bip32::DerivationPath,
    asset_lock_signer: &AS,
    surplus_output: Option<PlatformAddress>,
    settings: Option<PutSettings>,
) -> Result<Vec<dpp::shielded::SerializedAction>, dash_sdk::Error>
where
    AS: ::key_wallet::signer::Signer,
{
    use dpp::state_transition::shield_from_asset_lock_transition::accessors::ShieldFromAssetLockTransitionAccessorsV0;
    use dpp::state_transition::StateTransition;

    // One version for the whole attempt: the bundle's binding is checked
    // against the same rules the transition is assembled under.
    let platform_version = sdk.version();
    let target = BundleTarget {
        out_point: out_point_from_proof(&proof),
        shield_amount,
        platform_version,
    };
    let bundle = bundles.lock().await.bundle_for(target).await?;
    let st = bundle
        .build_transition_with_signer(
            proof,
            &path,
            asset_lock_signer,
            surplus_output,
            platform_version,
        )
        .await?;

    let actions = match &st {
        StateTransition::ShieldFromAssetLock(transition) => transition.actions().to_vec(),
        _ => Vec::new(),
    };

    // Wait for the verified result rather than relay-ACK. Single-use
    // asset-lock proof: a false-positive on a transition Platform
    // later rejects would strand the L1 outpoint with no in-app
    // signal. A shield-from-asset-lock proof authenticates the consumed
    // outpoint (and surplus-address state) at the committed block but
    // cannot bind this exact Orchard request, so the outcome is an
    // affected-state snapshot; a consensus rejection still surfaces as
    // an error on this wait.
    st.broadcast_and_wait_for_affected_state::<StateTransitionProofResult>(&sdk, settings)
        .await?;
    Ok(actions)
}

/// `lock_value − pool_fee`, the amount a single-recipient
/// `ShieldFromAssetLock` moves into the pool; errors when the lock cannot
/// cover the fee with something left to shield.
fn shield_amount_after_fee(
    asset_lock_value_credits: Credits,
    pool_fee_credits: Credits,
) -> Result<Credits, PlatformWalletError> {
    let shield_amount = asset_lock_value_credits
        .checked_sub(pool_fee_credits)
        .ok_or_else(|| {
            PlatformWalletError::ShieldedBuildError(format!(
                "asset lock value ({asset_lock_value_credits} credits) is below the \
                 ShieldFromAssetLock pool fee ({pool_fee_credits} credits = shielded fee + \
                 asset_lock_base_cost)"
            ))
        })?;
    if shield_amount == 0 {
        return Err(PlatformWalletError::ShieldedBuildError(
            "shield amount after fee is zero".to_string(),
        ));
    }
    Ok(shield_amount)
}

// ---------------------------------------------------------------------------
// Proving the Orchard bundle off the async runtime, ahead of the lock proof
// ---------------------------------------------------------------------------

/// Everything a `ShieldFromAssetLock` bundle is proved for that can differ
/// between the speculative proof and a submission attempt. The recipient,
/// sender OVK, memo and filler count are fixed for the whole call and live in
/// the [`ProveFn`].
#[derive(Clone, Copy)]
struct BundleTarget {
    /// The locked outpoint — the bundle's sighash binds it.
    out_point: dashcore::OutPoint,
    /// The value the bundle moves into the pool.
    shield_amount: Credits,
    /// The protocol version whose binding rules the bundle is proved under.
    /// Compared by number: an SDK protocol-version change between the
    /// speculative proof and an attempt re-proves even when the binding is
    /// unchanged — conservative, and rare (it takes a protocol upgrade during
    /// the call).
    platform_version: &'static dpp::version::PlatformVersion,
}

impl PartialEq for BundleTarget {
    fn eq(&self, other: &Self) -> bool {
        self.out_point == other.out_point
            && self.shield_amount == other.shield_amount
            && self.platform_version.protocol_version == other.platform_version.protocol_version
    }
}

/// Proves a bundle for a target. Blocking and CPU-heavy: only ever run on
/// tokio's blocking pool, through [`ProofTask`].
type ProveFn<B> = Arc<dyn Fn(BundleTarget) -> Result<B, ProtocolError> + Send + Sync>;

/// A bundle proof running on tokio's blocking pool, so it never occupies an
/// async worker.
///
/// Dropping the task cancels it: a proof that has not started yet never runs
/// (the task is aborted, and the job checks a cancellation flag before
/// proving), and the result of one already running — Halo 2 proving cannot be
/// interrupted — is dropped, together with the inputs the job holds, as soon
/// as it finishes. Nothing waits for a cancelled proof and nothing is
/// persisted from it.
struct ProofTask<B> {
    target: BundleTarget,
    cancelled: Arc<AtomicBool>,
    /// `Some` until the proof has been joined.
    handle: Option<tokio::task::JoinHandle<Option<Result<B, ProtocolError>>>>,
}

impl<B: Send + 'static> ProofTask<B> {
    fn spawn(target: BundleTarget, prove: ProveFn<B>) -> Self {
        let cancelled = Arc::new(AtomicBool::new(false));
        let job_cancelled = Arc::clone(&cancelled);
        let handle = tokio::task::spawn_blocking(move || {
            (!job_cancelled.load(Ordering::Acquire)).then(|| prove(target))
        });
        Self {
            target,
            cancelled,
            handle: Some(handle),
        }
    }

    /// Wait for the proof. Cancellation-safe: dropping this future drops the
    /// task, which cancels the proof.
    async fn join(mut self) -> Result<B, ProtocolError> {
        let joined = self
            .handle
            .as_mut()
            .expect("the handle is present until the proof is joined")
            .await;
        self.handle = None;
        match joined {
            Ok(Some(result)) => result,
            // Re-raise a proving panic exactly as if it had run inline.
            Err(e) if e.is_panic() => std::panic::resume_unwind(e.into_panic()),
            // The flag is only set on drop, so `Ok(None)` is unreachable
            // while this task is alive; a cancelled join is the runtime
            // shutting down.
            Ok(None) | Err(_) => Err(ProtocolError::ShieldedBuildError(
                "Orchard proof generation was cancelled".to_string(),
            )),
        }
    }
}

impl<B> Drop for ProofTask<B> {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            self.cancelled.store(true, Ordering::Release);
            handle.abort();
        }
    }
}

/// The proved bundle the submission attempts of one funding call share.
///
/// Holds the bundle's signing secrets (see `ProvedShieldFromAssetLockBundle`)
/// in memory only, until the submission attempts end; they are dropped with
/// the cache.
///
/// Holds the speculative proof started while the lock proof was awaited
/// and, once joined or freshly proved, the bundle itself. Every attempt asks
/// for the bundle of its target; a bundle is reused only for an identical
/// target (same outpoint, amount and protocol version), so a reused bundle
/// commits to exactly what a fresh proof would. Anything else — a
/// speculative proof for another outpoint or amount, a protocol upgrade
/// between attempts — is discarded and proved afresh.
struct BundleCache<B> {
    prove: ProveFn<B>,
    speculative: Option<ProofTask<B>>,
    proved: Option<(BundleTarget, Arc<B>)>,
}

impl<B: Send + Sync + 'static> BundleCache<B> {
    fn new(prove: ProveFn<B>, speculative: Option<ProofTask<B>>) -> Self {
        Self {
            prove,
            speculative,
            proved: None,
        }
    }

    async fn bundle_for(&mut self, target: BundleTarget) -> Result<Arc<B>, ProtocolError> {
        if let Some((proved_for, bundle)) = &self.proved {
            if *proved_for == target {
                tracing::debug!(
                    out_point = %target.out_point,
                    "reusing the proved shield-from-asset-lock bundle for this attempt"
                );
                return Ok(Arc::clone(bundle));
            }
        }
        let task = match self.speculative.take() {
            Some(task) if task.target == target => task,
            Some(stale) => {
                // Rare (the lock's value or the protocol version changed under
                // the call), but it forfeits the overlap: make it visible.
                tracing::info!(
                    speculative_out_point = %stale.target.out_point,
                    speculative_amount = stale.target.shield_amount,
                    out_point = %target.out_point,
                    shield_amount = target.shield_amount,
                    "discarding a speculative shield-from-asset-lock proof made for other inputs"
                );
                ProofTask::spawn(target, Arc::clone(&self.prove))
            }
            None => ProofTask::spawn(target, Arc::clone(&self.prove)),
        };
        let bundle = Arc::new(task.join().await?);
        self.proved = Some((target, Arc::clone(&bundle)));
        Ok(bundle)
    }
}

/// Drive `resolve` (funding resolution, whose proof wait is the slow part) to
/// completion while `speculate` starts a proof as soon as `out_point` delivers
/// the lock's outpoint. `speculate` must only *start* the proof (return its
/// [`ProofTask`]), not wait for it, so this returns as soon as `resolve` does.
///
/// If `resolve` fails, the speculative task is dropped here — cancelling the
/// proof — and the error is returned. If `resolve` never reports an outpoint
/// (its sender is dropped), no proof is started.
async fn resolve_while_proving<R, E, B, S, SF>(
    resolve: impl std::future::Future<Output = Result<R, E>>,
    out_point: tokio::sync::oneshot::Receiver<dashcore::OutPoint>,
    speculate: S,
) -> Result<(R, Option<ProofTask<B>>), E>
where
    S: FnOnce(dashcore::OutPoint) -> SF,
    SF: std::future::Future<Output = Option<ProofTask<B>>>,
{
    let speculation = async move {
        let out_point = out_point.await.ok()?;
        speculate(out_point).await
    };
    let (resolved, speculative) = tokio::join!(resolve, speculation);
    Ok((resolved?, speculative))
}

/// Pre-flight check for the recipient list.
///
/// Today: non-empty, exactly one recipient whose `Credits` value is
/// `None` (= "remainder" semantics — receives `lock_value − min_fee`
/// after Step 2 resolves the asset lock). The multi-shape
/// `Vec<(OrchardAddress, Option<Credits>)>` API is exposed so the
/// caller signature is future-compatible — when DPP grows
/// multi-output Orchard bundles for Type 18, `Some(_)` values will
/// be honored (explicit credit amounts pass through; the single
/// `None` bucket receives the residual). Same shape as Type 14.
///
/// Generic over `T` so unit tests can pass `(u8, Option<Credits>)`
/// instead of constructing a curve-valid `OrchardAddress` for what
/// is really a length / cardinality check.
pub(super) fn validate_shielded_recipients<T>(
    recipients: &[(T, Option<Credits>)],
) -> Result<(), PlatformWalletError> {
    if recipients.is_empty() {
        return Err(PlatformWalletError::AddressOperation(
            "shielded_fund_from_asset_lock requires at least one recipient".to_string(),
        ));
    }
    // TODO(multi-output): when DPP grows multi-output Orchard bundles
    // for Type 18 (`build_output_only_bundle` currently builds a
    // single-output bundle; extending would also affect the Shield
    // Type 15 path that shares it), drop this restriction. The
    // semantics will become: explicit `Some(credits)` flows into
    // its Orchard output; the (exactly one) `None` bucket receives
    // the residual `asset_lock_value − sum(explicit) − fee`.
    if recipients.len() != 1 {
        return Err(PlatformWalletError::AddressOperation(format!(
            "shielded_fund_from_asset_lock currently supports exactly one recipient \
             (multi-output Orchard bundles for Type 18 not yet wired through DPP); got {}",
            recipients.len()
        )));
    }
    if recipients[0].1.is_some() {
        // TODO(multi-output): drop this when the bundle builder honors
        // explicit `Some(_)` values per recipient.
        return Err(PlatformWalletError::AddressOperation(
            "shielded_fund_from_asset_lock currently ignores explicit recipient credits \
             (the single recipient receives lock_value - min_fee). Pass `None` for the \
             remainder semantics; explicit amounts will be honored once DPP grows \
             multi-output Orchard bundles for Type 18."
                .to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};

    use dashcore::{Network, OutPoint};
    use dpp::consensus::basic::identity::IdentityAssetLockTransactionOutPointAlreadyConsumedError;
    use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
    use key_wallet::account::account_type::StandardAccountType;
    use key_wallet_manager::WalletManager;
    use tokio::sync::{Notify, RwLock};

    use super::*;
    use crate::changeset::{
        ClientStartState, PersistenceCapabilities, PersistenceError, PersistenceErrorKind,
        PlatformWalletChangeSet, PlatformWalletPersistence,
    };
    use crate::test_support::{funded_wallet_manager, AlwaysRejectedBroadcaster};
    use crate::wallet::asset_lock::manager::AssetLockManager;
    use crate::wallet::asset_lock::tracked::{AssetLockStatus, TrackedAssetLock};
    use crate::wallet::persister::WalletPersister;
    use crate::wallet::platform_wallet::{PlatformWalletInfo, WalletId};

    // The preflight is a pure length/cardinality check; the
    // recipient type is irrelevant for what we're testing. Using
    // `u8` as the placeholder type avoids needing to construct a
    // curve-valid `OrchardAddress` (which requires the Orchard
    // crate's spend-key plumbing) inside this crate.

    #[test]
    fn validate_rejects_empty_recipients() {
        let v: Vec<(u8, Option<Credits>)> = Vec::new();
        let err = validate_shielded_recipients(&v).expect_err("empty must reject");
        assert!(format!("{err}").contains("at least one recipient"));
    }

    #[test]
    fn validate_rejects_multi_recipient_for_now() {
        let v: Vec<(u8, Option<Credits>)> = vec![(1, None), (2, Some(100))];
        let err = validate_shielded_recipients(&v).expect_err("multi-recipient must reject (TODO)");
        let msg = format!("{err}");
        assert!(
            msg.contains("exactly one recipient"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn validate_rejects_explicit_some_for_now() {
        // Until DPP grows multi-output Orchard bundles for Type 18,
        // we ignore explicit amounts (the single recipient receives
        // lock_value - min_fee). Reject explicit `Some(_)` so the
        // caller's expectation matches the wallet's behaviour
        // instead of silently dropping the value.
        let v: Vec<(u8, Option<Credits>)> = vec![(0, Some(500_000))];
        let err = validate_shielded_recipients(&v)
            .expect_err("explicit Some must reject until multi-output is wired");
        let msg = format!("{err}");
        assert!(
            msg.contains("currently ignores explicit recipient credits"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn validate_accepts_single_none_recipient() {
        let v: Vec<(u8, Option<Credits>)> = vec![(0, None)];
        validate_shielded_recipients(&v).expect("single recipient with None must pass");
    }

    #[derive(Default)]
    struct RecordingPersistence {
        stored: Mutex<Vec<PlatformWalletChangeSet>>,
        fail_next_store: AtomicBool,
        fail_next_flush: Mutex<Option<PersistenceErrorKind>>,
        store_commits_inline: AtomicBool,
        omit_reconciliation_capabilities: AtomicBool,
    }

    impl PlatformWalletPersistence for RecordingPersistence {
        fn store_commits_inline(&self) -> bool {
            self.store_commits_inline.load(Ordering::SeqCst)
        }

        fn persistence_capabilities(&self) -> PersistenceCapabilities {
            if self.omit_reconciliation_capabilities.load(Ordering::SeqCst) {
                PersistenceCapabilities::NONE
            } else {
                PersistenceCapabilities::ASSET_LOCK_RECONCILIATION
            }
        }

        fn store(
            &self,
            _wallet_id: WalletId,
            changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            if self.fail_next_store.swap(false, Ordering::SeqCst) {
                return Err(PersistenceError::backend(
                    "simulated asset-lock store failure",
                ));
            }
            self.stored
                .lock()
                .expect("recording persistence mutex")
                .push(changeset);
            Ok(())
        }

        fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
            let failure = self
                .fail_next_flush
                .lock()
                .expect("recording persistence mutex")
                .take();
            if let Some(kind) = failure {
                Err(PersistenceError::backend_with_kind(
                    kind,
                    "simulated asset-lock flush failure",
                ))
            } else {
                Ok(())
            }
        }

        fn load(&self) -> Result<ClientStartState, PersistenceError> {
            Ok(ClientStartState::default())
        }
    }

    struct ConsumptionReportContext {
        manager: AssetLockManager<AlwaysRejectedBroadcaster>,
        wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
        wallet_id: WalletId,
        out_point: OutPoint,
        proof: AssetLockProof,
        persistence: Arc<RecordingPersistence>,
    }

    async fn consumption_report_context() -> ConsumptionReportContext {
        consumption_report_context_for(AssetLockFundingType::AssetLockShieldedAddressTopUp).await
    }

    async fn consumption_report_context_for(
        funding_type: AssetLockFundingType,
    ) -> ConsumptionReportContext {
        let (wallet_manager, wallet_id, _generation, signer) =
            funded_wallet_manager(StandardAccountType::BIP44Account).await;
        let persistence = Arc::new(RecordingPersistence::default());
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
            WalletPersister::new(wallet_id, Arc::<RecordingPersistence>::clone(&persistence)),
        );
        let (transaction, _path) = manager
            .build_asset_lock_transaction(
                1_000_000,
                0,
                AssetLockFundingType::AssetLockShieldedAddressTopUp,
                0,
                &signer,
            )
            .await
            .expect("build asset lock");
        let out_point = OutPoint::new(transaction.txid(), 0);
        let proof = AssetLockProof::Chain(ChainAssetLockProof {
            core_chain_locked_height: 42,
            out_point,
        });
        {
            let mut wm = wallet_manager.write().await;
            let info = wm
                .get_wallet_info_mut(&wallet_id)
                .expect("wallet must remain registered");
            info.tracked_asset_locks.insert(
                out_point,
                TrackedAssetLock {
                    out_point,
                    transaction,
                    account_index: 0,
                    funding_type,
                    identity_index: 0,
                    amount: 1_000_000,
                    status: AssetLockStatus::ChainLocked,
                    proof: Some(proof.clone()),
                },
            );
        }

        ConsumptionReportContext {
            manager,
            wallet_manager,
            wallet_id,
            out_point,
            proof,
            persistence,
        }
    }

    fn already_consumed_error(out_point: OutPoint) -> dash_sdk::Error {
        dash_sdk::Error::Protocol(dpp::ProtocolError::ConsensusError(Box::new(
            IdentityAssetLockTransactionOutPointAlreadyConsumedError::new(
                out_point.txid,
                out_point.vout as usize,
            )
            .into(),
        )))
    }

    #[tokio::test]
    async fn successful_submit_result_passes_through_without_mutation() {
        let ctx = consumption_report_context().await;
        let stored_before = ctx
            .persistence
            .stored
            .lock()
            .expect("recording persistence mutex")
            .len();

        let value = ctx
            .manager
            .reconcile_asset_lock_submit_result(
                Ok::<_, dash_sdk::Error>(42u64),
                &ctx.out_point,
                &ctx.proof,
                None,
            )
            .await
            .expect("successful submission must pass through");
        assert_eq!(value, 42);

        let wm = ctx.wallet_manager.read().await;
        let lock = wm
            .get_wallet_info(&ctx.wallet_id)
            .expect("wallet")
            .tracked_asset_locks
            .get(&ctx.out_point)
            .expect("tracked lock");
        assert_eq!(lock.status, AssetLockStatus::ChainLocked);
        assert_eq!(lock.proof, Some(ctx.proof));
        assert_eq!(
            ctx.persistence
                .stored
                .lock()
                .expect("recording persistence mutex")
                .len(),
            stored_before
        );
    }

    #[tokio::test]
    async fn consumed_report_reconciliation_is_funding_role_agnostic() {
        for funding_type in [
            AssetLockFundingType::IdentityRegistration,
            AssetLockFundingType::IdentityTopUp,
            AssetLockFundingType::IdentityInvitation,
            AssetLockFundingType::AssetLockAddressTopUp,
            AssetLockFundingType::AssetLockShieldedAddressTopUp,
        ] {
            let ctx = consumption_report_context_for(funding_type).await;
            let error = ctx
                .manager
                .reconcile_asset_lock_submit_result::<()>(
                    Err(already_consumed_error(ctx.out_point)),
                    &ctx.out_point,
                    &ctx.proof,
                    None,
                )
                .await
                .expect_err("already-consumed report must stay a typed error");
            assert!(matches!(
                error,
                PlatformWalletError::AssetLockAlreadyConsumed(actual)
                    if actual == ctx.out_point
            ));
            let wm = ctx.wallet_manager.read().await;
            let lock = wm
                .get_wallet_info(&ctx.wallet_id)
                .expect("wallet")
                .tracked_asset_locks
                .get(&ctx.out_point)
                .expect("tracked lock");
            assert_eq!(lock.funding_type, funding_type);
            assert_eq!(lock.status, AssetLockStatus::RecoveredFromChain);
        }
    }

    #[tokio::test]
    async fn consumed_report_returns_typed_error_and_persists_nonterminal_state() {
        let ctx = consumption_report_context().await;

        let error = ctx
            .manager
            .reconcile_asset_lock_submit_result::<()>(
                Err(already_consumed_error(ctx.out_point)),
                &ctx.out_point,
                &ctx.proof,
                None,
            )
            .await
            .expect_err("already-consumed report remains a typed host signal");
        assert!(matches!(
            error,
            PlatformWalletError::AssetLockAlreadyConsumed(actual) if actual == ctx.out_point
        ));

        let wm = ctx.wallet_manager.read().await;
        let lock = wm
            .get_wallet_info(&ctx.wallet_id)
            .expect("wallet")
            .tracked_asset_locks
            .get(&ctx.out_point)
            .expect("tracked lock is retained");
        assert_eq!(lock.status, AssetLockStatus::RecoveredFromChain);
        assert!(matches!(lock.proof, Some(AssetLockProof::Chain(_))));
        drop(wm);

        let persisted_status = ctx
            .persistence
            .stored
            .lock()
            .expect("recording persistence mutex")
            .iter()
            .filter_map(|cs| cs.asset_locks.as_ref())
            .filter_map(|asset_locks| asset_locks.asset_locks.get(&ctx.out_point))
            .map(|entry| entry.status.clone())
            .next_back();
        assert_eq!(persisted_status, Some(AssetLockStatus::RecoveredFromChain));
    }

    #[tokio::test]
    async fn consumed_report_without_persistence_contract_keeps_pending_state() {
        let ctx = consumption_report_context().await;
        ctx.persistence
            .omit_reconciliation_capabilities
            .store(true, Ordering::SeqCst);

        let error = ctx
            .manager
            .reconcile_asset_lock_submit_result::<()>(
                Err(already_consumed_error(ctx.out_point)),
                &ctx.out_point,
                &ctx.proof,
                None,
            )
            .await
            .expect_err("unsupported persistence must fail before reconciliation");
        assert!(matches!(error, PlatformWalletError::Persistence(_)));

        let wm = ctx.wallet_manager.read().await;
        let lock = wm
            .get_wallet_info(&ctx.wallet_id)
            .expect("wallet")
            .tracked_asset_locks
            .get(&ctx.out_point)
            .expect("tracked lock");
        assert_eq!(lock.status, AssetLockStatus::ChainLocked);
        assert_eq!(lock.proof, Some(ctx.proof));
        assert!(ctx
            .persistence
            .stored
            .lock()
            .expect("recording persistence mutex")
            .is_empty());
    }

    #[tokio::test]
    async fn unrelated_or_mismatched_errors_do_not_mutate_asset_lock() {
        let ctx = consumption_report_context().await;
        let stored_before = ctx
            .persistence
            .stored
            .lock()
            .expect("recording persistence mutex")
            .len();

        for error in [
            dash_sdk::Error::Generic("unrelated".to_string()),
            already_consumed_error(OutPoint::new(ctx.out_point.txid, ctx.out_point.vout + 1)),
        ] {
            let mapped = ctx
                .manager
                .reconcile_asset_lock_submit_result::<()>(
                    Err(error),
                    &ctx.out_point,
                    &ctx.proof,
                    None,
                )
                .await
                .expect_err("submit error must remain an error");
            assert!(matches!(mapped, PlatformWalletError::Sdk(_)));
        }

        let wm = ctx.wallet_manager.read().await;
        let lock = wm
            .get_wallet_info(&ctx.wallet_id)
            .expect("wallet")
            .tracked_asset_locks
            .get(&ctx.out_point)
            .expect("tracked lock");
        assert_eq!(lock.status, AssetLockStatus::ChainLocked);
        assert_eq!(lock.proof, Some(ctx.proof));
        assert_eq!(
            ctx.persistence
                .stored
                .lock()
                .expect("recording persistence mutex")
                .len(),
            stored_before
        );
    }

    #[tokio::test]
    async fn persistence_failure_rolls_back_consumption_unknown_state() {
        let ctx = consumption_report_context().await;
        ctx.persistence
            .fail_next_store
            .store(true, Ordering::SeqCst);

        let error = ctx
            .manager
            .reconcile_asset_lock_submit_result::<()>(
                Err(already_consumed_error(ctx.out_point)),
                &ctx.out_point,
                &ctx.proof,
                None,
            )
            .await
            .expect_err("host persistence rejection must surface");
        assert!(matches!(error, PlatformWalletError::Persistence(_)));

        let wm = ctx.wallet_manager.read().await;
        let lock = wm
            .get_wallet_info(&ctx.wallet_id)
            .expect("wallet")
            .tracked_asset_locks
            .get(&ctx.out_point)
            .expect("tracked lock");
        assert_eq!(lock.status, AssetLockStatus::ChainLocked);
        assert_eq!(lock.proof, Some(ctx.proof));
    }

    #[tokio::test]
    async fn transient_flush_failure_keeps_buffered_candidate_in_memory() {
        let ctx = consumption_report_context().await;
        *ctx.persistence
            .fail_next_flush
            .lock()
            .expect("recording persistence mutex") = Some(PersistenceErrorKind::Transient);

        let error = ctx
            .manager
            .reconcile_asset_lock_submit_result::<()>(
                Err(already_consumed_error(ctx.out_point)),
                &ctx.out_point,
                &ctx.proof,
                None,
            )
            .await
            .expect_err("host flush rejection must surface");
        assert!(matches!(error, PlatformWalletError::Persistence(_)));

        let wm = ctx.wallet_manager.read().await;
        let lock = wm
            .get_wallet_info(&ctx.wallet_id)
            .expect("wallet")
            .tracked_asset_locks
            .get(&ctx.out_point)
            .expect("tracked lock");
        assert_eq!(lock.status, AssetLockStatus::RecoveredFromChain);
    }

    #[tokio::test]
    async fn fatal_post_commit_flush_failure_keeps_durable_candidate_in_memory() {
        let ctx = consumption_report_context().await;
        ctx.persistence
            .store_commits_inline
            .store(true, Ordering::SeqCst);
        *ctx.persistence
            .fail_next_flush
            .lock()
            .expect("recording persistence mutex") = Some(PersistenceErrorKind::Fatal);

        let error = ctx
            .manager
            .reconcile_asset_lock_submit_result::<()>(
                Err(already_consumed_error(ctx.out_point)),
                &ctx.out_point,
                &ctx.proof,
                None,
            )
            .await
            .expect_err("post-commit flush rejection must surface");
        assert!(matches!(error, PlatformWalletError::Persistence(_)));

        let wm = ctx.wallet_manager.read().await;
        let lock = wm
            .get_wallet_info(&ctx.wallet_id)
            .expect("wallet")
            .tracked_asset_locks
            .get(&ctx.out_point)
            .expect("tracked lock");
        assert_eq!(lock.status, AssetLockStatus::RecoveredFromChain);
    }

    // -- Proving the bundle while the InstantSend lock is awaited ---------
    //
    // A fake prover stands in for Halo 2: the "bundle" is the amount it was
    // proved for.

    mod proof_pipeline {
        use std::sync::atomic::AtomicUsize;
        use std::time::Duration;

        use dpp::version::PlatformVersion;
        use tokio::sync::oneshot;

        use super::*;

        const AMOUNT: Credits = 1_000_000;

        fn target(shield_amount: Credits) -> BundleTarget {
            BundleTarget {
                out_point: OutPoint::from([7u8; 36]),
                shield_amount,
                platform_version: PlatformVersion::latest(),
            }
        }

        fn counting_prover(calls: &Arc<AtomicUsize>) -> ProveFn<Credits> {
            let calls = Arc::clone(calls);
            Arc::new(move |target: BundleTarget| {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok(target.shield_amount)
            })
        }

        /// The InstantSend lock is only delivered once the proof is running,
        /// and the proof only finishes once the lock has been delivered, so
        /// this completes only if the two overlap.
        #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
        async fn should_prove_while_the_lock_is_awaited() {
            let (proof_started_tx, proof_started_rx) = oneshot::channel::<()>();
            let (lock_delivered_tx, lock_delivered_rx) = std::sync::mpsc::channel::<()>();
            let proof_started_tx = Mutex::new(Some(proof_started_tx));
            let lock_delivered_rx = Mutex::new(lock_delivered_rx);
            let prove: ProveFn<Credits> = Arc::new(move |target: BundleTarget| {
                if let Some(tx) = proof_started_tx.lock().unwrap().take() {
                    let _ = tx.send(());
                }
                lock_delivered_rx
                    .lock()
                    .unwrap()
                    .recv()
                    .expect("lock delivered");
                Ok(target.shield_amount)
            });

            let (out_point_tx, out_point_rx) = oneshot::channel();
            let resolve = async move {
                let _ = out_point_tx.send(target(AMOUNT).out_point);
                proof_started_rx.await.expect("proof started");
                lock_delivered_tx.send(()).unwrap();
                Ok::<_, PlatformWalletError>(())
            };
            let bundle = tokio::time::timeout(Duration::from_secs(30), async {
                let ((), speculative) = resolve_while_proving(resolve, out_point_rx, |out_point| {
                    let task = ProofTask::spawn(
                        BundleTarget {
                            out_point,
                            ..target(AMOUNT)
                        },
                        Arc::clone(&prove),
                    );
                    std::future::ready(Some(task))
                })
                .await
                .expect("resolution");
                assert!(speculative.is_some());
                BundleCache::new(Arc::clone(&prove), speculative)
                    .bundle_for(target(AMOUNT))
                    .await
                    .expect("proof")
            })
            .await
            .expect("the proof and the lock wait did not overlap");
            assert_eq!(*bundle, AMOUNT);
        }

        /// Every submission attempt of one call — the first, the IS→CL
        /// fallback, the CL-height retries — asks for the same target and gets
        /// the speculative bundle. A changed amount or protocol version is
        /// proved afresh.
        #[tokio::test]
        async fn should_prove_once_per_target() {
            let calls = Arc::new(AtomicUsize::new(0));
            let prove = counting_prover(&calls);
            let speculative = ProofTask::spawn(target(AMOUNT), Arc::clone(&prove));
            let mut cache = BundleCache::new(prove, Some(speculative));

            let first = cache.bundle_for(target(AMOUNT)).await.expect("proof");
            for _ in 0..3 {
                let again = cache.bundle_for(target(AMOUNT)).await.expect("proof");
                assert!(Arc::ptr_eq(&first, &again));
            }
            assert_eq!(calls.load(Ordering::SeqCst), 1);

            let other_amount = cache.bundle_for(target(AMOUNT - 1)).await.expect("proof");
            assert_eq!(*other_amount, AMOUNT - 1);
            assert_eq!(calls.load(Ordering::SeqCst), 2);
            let other_version = BundleTarget {
                platform_version: PlatformVersion::first(),
                ..target(AMOUNT - 1)
            };
            cache.bundle_for(other_version).await.expect("proof");
            assert_eq!(calls.load(Ordering::SeqCst), 3);
        }

        #[tokio::test]
        async fn should_not_use_a_speculative_proof_for_other_inputs() {
            let prove = counting_prover(&Arc::new(AtomicUsize::new(0)));
            let speculative = ProofTask::spawn(target(AMOUNT + 1), Arc::clone(&prove));
            let bundle = BundleCache::new(prove, Some(speculative))
                .bundle_for(target(AMOUNT))
                .await
                .expect("proof");
            assert_eq!(*bundle, AMOUNT);
        }

        /// A proof cancelled before a blocking thread picks it up never runs.
        #[test]
        fn should_never_run_a_proof_cancelled_before_it_starts() {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .max_blocking_threads(1)
                .enable_all()
                .build()
                .expect("runtime");
            runtime.block_on(async {
                let calls = Arc::new(AtomicUsize::new(0));
                // Occupy the only blocking thread so the proof stays queued.
                let (release_tx, release_rx) = std::sync::mpsc::channel::<()>();
                let blocker = tokio::task::spawn_blocking(move || {
                    let _ = release_rx.recv();
                });
                drop(ProofTask::spawn(target(AMOUNT), counting_prover(&calls)));
                release_tx.send(()).unwrap();
                blocker.await.unwrap();
                // The queue is FIFO: once this runs, the cancelled job has had
                // its turn.
                tokio::task::spawn_blocking(|| ()).await.unwrap();
                assert_eq!(calls.load(Ordering::SeqCst), 0);
            });
        }
    }
}
