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
//! 3. **Submit** — wrap the build-and-broadcast in
//!    `submit_with_cl_height_retry`; the build uses the new
//!    [`build_shield_from_asset_lock_transition_with_signer`] so the
//!    asset-lock-proof signature is routed through the external
//!    `key_wallet::signer::Signer` (the host never sees the raw key).
//!    IS→CL fallback fires on Platform-side IS rejection
//!    (`is_instant_lock_proof_invalid`).
//! 4. **Consume lock** — terminal `consume_asset_lock` on the tracked
//!    outpoint. Notes themselves arrive via the next shielded sync;
//!    the shielded changeset doesn't materialise post-submit the way
//!    the address-funding `AddressInfos` does.

use dash_sdk::platform::transition::broadcast::BroadcastStateTransition;
use dash_sdk::platform::transition::put_settings::PutSettings;
use dpp::address_funds::{OrchardAddress, PlatformAddress};
use dpp::balances::credits::CREDITS_PER_DUFF;
use dpp::fee::Credits;
use dpp::prelude::AssetLockProof;
use dpp::shielded::builder::{build_shield_from_asset_lock_transition_with_signer, OrchardProver};
use dpp::shielded::compute_minimum_shielded_fee;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;

use crate::wallet::asset_lock::tracked::TrackedAssetLock;

use std::time::Duration;

use crate::error::{is_asset_lock_already_consumed, is_instant_lock_proof_invalid};
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
    /// * `prover` — Orchard prover (holds the Halo 2 proving key).
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
    ///   even though each attempt re-signs a freshly-randomized bundle.
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
        P: OrchardProver,
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

        // Step 2: resolve funding. `AssetLockShieldedAddressTopUp`
        // selects the BIP44 funding family dedicated to shielded
        // top-ups (`accounts.asset_lock_shielded_address_topup` —
        // distinct from the platform-address bucket Type 14 uses);
        // see `wallet/asset_lock/build.rs` for the source-account
        // selection, `sync/recovery.rs` for resume-time key re-
        // derivation, and `manager/accessors.rs` for the
        // persistence/UI tag (`fundingTypeRaw == 5`).
        // `destination_index = 0` is unused for this funding type.
        let ResolvedFunding {
            proof,
            path,
            tracked_out_point,
        } = match self
            .asset_locks
            .resolve_funding_with_is_timeout_fallback(
                funding,
                AssetLockFundingType::AssetLockShieldedAddressTopUp,
                /* destination_index */ 0,
                asset_lock_signer,
            )
            .await?
        {
            FundingResolution::Resolved(rf) => rf,
            FundingResolution::IsTimeout { out_point } => {
                tracing::warn!(
                    "IS-lock did not propagate within 300s for shielded fund-from-asset-lock \
                     (tx {}), falling back to ChainLock proof",
                    out_point.txid
                );
                let chain_proof = self
                    .asset_locks
                    .upgrade_to_chain_lock_proof(&out_point, cl_wait)
                    .await?;
                let (_, path) = self
                    .asset_locks
                    .resume_asset_lock(&out_point, cl_wait)
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
        let (recipient, _) = *recipients.first().expect("preflight enforces len() == 1");

        // Encrypt the output under this wallet's OVK so the shielded sync can
        // recover the funding (recipient, value, memo) from chain data alone.
        // Prefer the bound account whose IVK recognizes the recipient address
        // (the sent-note row then lands under that account); fall back to the
        // lowest bound account, or `None` (unrecoverable out_ciphertext) if
        // the shielded sub-wallet isn't bound.
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

        // Step 4: submit. Two Platform-side fallback layers — matching
        // the address-funding sibling: CL-height-too-low retries bump
        // `user_fee_increase` (bypasses Tenderdash's invalid-tx hash
        // cache) and IS-lock rejection triggers an IS→CL upgrade on
        // the same outpoint.
        //
        // Subtle: `ShieldFromAssetLockTransition::set_user_fee_increase`
        // is a no-op (pinned at `state_transition::mod`'s
        // `test_shield_from_asset_lock_user_fee_increase_is_zero_and_setter_noop`),
        // so the wrapper's bump cannot directly diversify the ST hash
        // here the way it does for address-funding. Retries still avoid
        // Tenderdash's invalid-tx cache because `build_output_only_bundle`
        // draws fresh randomness from `OsRng` on every call
        // (`packages/rs-dpp/src/shielded/builder/mod.rs`), so a re-built
        // bundle has a different Halo 2 proof and therefore a different
        // signable hash. If the prover is ever made deterministic for
        // reproducibility, this orchestration would need an explicit
        // diversifier (e.g. a memo-derived bump) to keep CL-height
        // retries from silently degrading into duplicate-hash submits.
        let proof_out_point = out_point_from_proof(&proof);
        let sdk = self.sdk.clone();
        // Serialized actions of the bundle that actually landed — fed to
        // the live activity recorder below. Each retry re-randomizes the
        // bundle, so only the landed attempt's actions are the ones a
        // later scan will recover; `build_and_broadcast_shielded` returns
        // them on success.
        let submit_result = match submit_with_cl_height_retry(settings, |s| {
            build_and_broadcast_shielded(
                sdk.clone(),
                recipient,
                shield_amount,
                proof.clone(),
                path.clone(),
                asset_lock_signer,
                &prover,
                sender_ovk.clone(),
                surplus_output,
                dummy_outputs,
                s,
            )
        })
        .await
        {
            Ok(actions) => Ok(actions),
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
                submit_with_cl_height_retry(settings, |s| {
                    build_and_broadcast_shielded(
                        sdk.clone(),
                        recipient,
                        shield_amount,
                        chain_proof.clone(),
                        path.clone(),
                        asset_lock_signer,
                        &prover,
                        sender_ovk.clone(),
                        surplus_output,
                        dummy_outputs,
                        s,
                    )
                })
                .await
            }
            Err(e) => Err(e),
        };

        let landed_actions: Vec<dpp::shielded::SerializedAction> = match submit_result {
            Ok(actions) => actions,
            Err(e) if is_asset_lock_already_consumed(&e, &proof_out_point) => {
                // The exact outpoint was accepted by an earlier attempt, but
                // that attempt did not reach the local cleanup below. Close
                // the lifecycle now so persistence and recovery UIs stop
                // advertising an impossible retry, then preserve the typed
                // already-consumed outcome across FFI for idempotent hosts.
                self.asset_locks
                    .consume_asset_lock(&proof_out_point)
                    .await?;
                tracing::info!(
                    outpoint = %proof_out_point,
                    "reconciled already-consumed shielded asset lock"
                );
                return Err(PlatformWalletError::AssetLockAlreadyConsumed(
                    proof_out_point,
                ));
            }
            Err(e) => return Err(PlatformWalletError::Sdk(e)),
        };

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
    /// broadcast, no Failed row after): `submit_with_cl_height_retry`
    /// re-builds and re-randomizes the Orchard bundle on every attempt,
    /// so each attempt has different output cmxs — and the activity id is
    /// keyed to those cmxs. A pre-broadcast Pending row would orphan
    /// (unconfirmable forever, its cmxs never on-chain) whenever a retry
    /// is the attempt that lands. In-flight and failed Type 18s are
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
/// 1. If the proof is `Instant`, read directly from
///    `InstantAssetLockProof::output().value` — no manager lookup
///    needed.
/// 2. Otherwise (the IS-timeout-fallback path produced a CL proof
///    that doesn't carry the tx output), look up the tracked
///    asset-lock row by outpoint.
async fn lookup_asset_lock_value_credits(
    wallet: &PlatformWallet,
    proof: &AssetLockProof,
    tracked_out_point: Option<&dashcore::OutPoint>,
) -> Result<Credits, PlatformWalletError> {
    let duffs = match proof {
        AssetLockProof::Instant(is) => {
            let out = is.output().ok_or_else(|| {
                PlatformWalletError::AddressSync(
                    "InstantAssetLockProof has no output at the indicated index".to_string(),
                )
            })?;
            out.value
        }
        AssetLockProof::Chain(_) => {
            let op = tracked_out_point.ok_or_else(|| {
                PlatformWalletError::AddressSync(
                    "ChainAssetLockProof but no tracked outpoint to look up value".to_string(),
                )
            })?;
            let locks: Vec<TrackedAssetLock> = wallet.asset_locks.list_tracked_locks().await;
            locks
                .iter()
                .find(|l| l.out_point == *op)
                .map(|l| l.amount)
                .ok_or_else(|| {
                    PlatformWalletError::AddressSync(format!(
                        "tracked asset lock {} not found in manager",
                        op
                    ))
                })?
        }
    };
    duffs.checked_mul(CREDITS_PER_DUFF).ok_or_else(|| {
        PlatformWalletError::ShieldedBuildError(format!(
            "asset lock value ({duffs} duffs * {CREDITS_PER_DUFF} credits/duff > u64::MAX)"
        ))
    })
}

/// Build the Type 18 transition and broadcast-and-wait.
///
/// Extracted so `submit_with_cl_height_retry`'s closure stays compact
/// and the IS→CL fallback path can re-call it with the upgraded proof.
///
/// On success returns the **landed** bundle's serialized Orchard actions
/// so the orchestrator can record a live `ShieldFromAssetLock` activity
/// entry over the exact bundle that committed (each retry re-randomizes
/// the bundle, so only the landed attempt's cmxs are the ones a later
/// scan will recover).
#[allow(clippy::too_many_arguments)]
async fn build_and_broadcast_shielded<AS, P>(
    sdk: std::sync::Arc<dash_sdk::Sdk>,
    recipient: OrchardAddress,
    shield_amount: Credits,
    proof: AssetLockProof,
    path: ::key_wallet::bip32::DerivationPath,
    asset_lock_signer: &AS,
    prover: &P,
    sender_ovk: Option<grovedb_commitment_tree::OutgoingViewingKey>,
    surplus_output: Option<PlatformAddress>,
    dummy_outputs: usize,
    settings: Option<PutSettings>,
) -> Result<Vec<dpp::shielded::SerializedAction>, dash_sdk::Error>
where
    AS: ::key_wallet::signer::Signer,
    P: OrchardProver,
{
    use dpp::state_transition::shield_from_asset_lock_transition::ShieldFromAssetLockTransition;
    use dpp::state_transition::StateTransition;

    let st = build_shield_from_asset_lock_transition_with_signer(
        &recipient,
        shield_amount,
        proof,
        &path,
        asset_lock_signer,
        prover,
        [0u8; 36],
        sender_ovk,
        surplus_output,
        dummy_outputs,
        sdk.version(),
    )
    .await?;

    // `actions` is a public field on the V0 struct (no accessor trait for
    // Shield / ShieldFromAssetLock).
    let actions = match &st {
        StateTransition::ShieldFromAssetLock(ShieldFromAssetLockTransition::V0(v0)) => {
            v0.actions.clone()
        }
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
    use super::*;

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
}
