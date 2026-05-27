//! Orchestrated shielded funding from a Core asset lock.
//!
//! Mirrors `wallet/platform_addresses/fund_from_asset_lock.rs` but
//! credits the *shielded* pool (Type 18 `ShieldFromAssetLock`) instead
//! of platform addresses (Type 14 `AddressFundingFromAssetLock`).
//!
//! ## Pipeline
//!
//! 1. **Pre-flight** — exactly-one recipient today (the multi-shape
//!    `Vec<(OrchardAddress, Option<Credits>)>` API is in place so
//!    the caller signature doesn't change when DPP grows multi-output
//!    Orchard bundles for Type 18; see [`validate_shielded_recipients`]).
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
use dpp::address_funds::OrchardAddress;
use dpp::balances::credits::CREDITS_PER_DUFF;
use dpp::fee::Credits;
use dpp::prelude::AssetLockProof;
use dpp::shielded::builder::{build_shield_from_asset_lock_transition_with_signer, OrchardProver};
use dpp::state_transition::proof_result::StateTransitionProofResult;
use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;

use crate::error::is_instant_lock_proof_invalid;
use crate::wallet::asset_lock::orchestration::{
    out_point_from_proof, submit_with_cl_height_retry, AssetLockFunding, FundingResolution,
    ResolvedFunding, CL_FALLBACK_TIMEOUT,
};
use crate::wallet::PlatformWallet;
use crate::PlatformWalletError;

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
    /// * `recipients` — Map from recipient `OrchardAddress` to optional
    ///   credit amount. The shape mirrors the platform-address API for
    ///   future-compatibility with multi-output Orchard bundles, but
    ///   today the pre-flight enforces exactly one recipient and the
    ///   `Option<Credits>` value is ignored — the single recipient
    ///   always receives the full asset-lock value minus the protocol
    ///   minimum fee.
    /// * `asset_lock_signer` — External signer for the outer ECDSA
    ///   signature on the state transition. The raw key never crosses
    ///   the FFI boundary.
    /// * `prover` — Orchard prover (holds the Halo 2 proving key).
    /// * `settings` — Optional `PutSettings`; `user_fee_increase` is
    ///   bumped by the CL-height retry wrapper on consensus 10506.
    #[cfg(feature = "shielded")]
    pub async fn shielded_fund_from_asset_lock<AS, P>(
        &self,
        funding: AssetLockFunding,
        recipients: Vec<(OrchardAddress, Option<Credits>)>,
        asset_lock_signer: &AS,
        prover: P,
        settings: Option<PutSettings>,
    ) -> Result<(), PlatformWalletError>
    where
        AS: ::key_wallet::signer::Signer + Send + Sync,
        P: OrchardProver,
    {
        // Step 1: pre-flight. Failing fast here avoids broadcasting
        // an unfundable asset-lock tx (or paying for an Orchard proof
        // build, ~30s, only to reject downstream).
        validate_shielded_recipients(&recipients)?;

        // Single-flight: serialise shield-class operations on this
        // wallet so two concurrent calls can't race the asset-lock
        // tracker into a half-consumed state.
        let _shield_guard = self.shield_guard.lock().await;

        // Step 2: resolve funding. `AssetLockAddressTopUp` selects the
        // BIP44 funding family for the Core asset-lock tx;
        // `destination_index = 0` is unused for this funding type.
        let ResolvedFunding {
            proof,
            path,
            tracked_out_point,
        } = match self
            .asset_locks
            .resolve_funding_with_is_timeout_fallback(
                funding,
                AssetLockFundingType::AssetLockAddressTopUp,
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
                    .upgrade_to_chain_lock_proof(&out_point, CL_FALLBACK_TIMEOUT)
                    .await?;
                let (_, path) = self
                    .asset_locks
                    .resume_asset_lock(&out_point, CL_FALLBACK_TIMEOUT)
                    .await?;
                ResolvedFunding {
                    proof: chain_proof,
                    path,
                    tracked_out_point: Some(out_point),
                }
            }
        };

        // Step 3: compute the shield amount.
        //
        // For Type 18, the protocol does not deduct fees inside the
        // transition (unlike address-funding's
        // `AddressFundsFeeStrategy`) — the Orchard `value_balance`
        // baked into the Halo 2 proof at build time *is* what enters
        // the shielded pool, and the asset-lock value minus that
        // covers the fee.
        //
        // Today (single recipient + value=None), the recipient gets
        // `asset_lock_value − min_required_fee`. When DPP grows
        // multi-output Orchard bundles for Type 18, this branches:
        // explicit `Some(_)` amounts pass through; the `None` bucket
        // receives the residual.
        let asset_lock_value_credits =
            lookup_asset_lock_value_credits(self, &proof, tracked_out_point.as_ref()).await?;
        let min_fee = self.shield_from_asset_lock_min_fee()?;
        let shield_amount = asset_lock_value_credits
            .checked_sub(min_fee)
            .ok_or_else(|| {
                PlatformWalletError::ShieldedBuildError(format!(
                    "asset lock value ({asset_lock_value_credits} credits) is below the \
                     minimum required fee ({min_fee} credits) for ShieldFromAssetLock"
                ))
            })?;
        if shield_amount == 0 {
            return Err(PlatformWalletError::ShieldedBuildError(
                "shield amount after fee is zero".to_string(),
            ));
        }

        let recipient = recipients.first().expect("preflight enforces len() == 1").0;

        // Step 4: submit. Two Platform-side fallback layers — matching
        // the address-funding sibling: CL-height-too-low retries bump
        // `user_fee_increase` (bypasses Tenderdash's invalid-tx hash
        // cache) and IS-lock rejection triggers an IS→CL upgrade on
        // the same outpoint.
        let proof_out_point = out_point_from_proof(&proof);
        let sdk = self.sdk.clone();
        match submit_with_cl_height_retry(settings, |s| {
            build_and_broadcast_shielded(
                sdk.clone(),
                recipient,
                shield_amount,
                proof.clone(),
                path.clone(),
                asset_lock_signer,
                &prover,
                s,
            )
        })
        .await
        {
            Ok(()) => {}
            Err(e) if is_instant_lock_proof_invalid(&e) => {
                let out_point = proof_out_point;
                tracing::warn!(
                    "IS-lock proof rejected by Platform for shielded fund-from-asset-lock \
                     (tx {}), retrying with ChainLock proof",
                    out_point.txid
                );
                let chain_proof = self
                    .asset_locks
                    .upgrade_to_chain_lock_proof(&out_point, CL_FALLBACK_TIMEOUT)
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
                        s,
                    )
                })
                .await
                .map_err(PlatformWalletError::Sdk)?;
            }
            Err(e) => return Err(PlatformWalletError::Sdk(e)),
        }

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
            min_fee,
            "Shielded fund-from-asset-lock succeeded"
        );

        Ok(())
    }

    /// Minimum fee for a `ShieldFromAssetLock` (Type 18) state
    /// transition, in credits. Read from
    /// `dpp.state_transitions.identities.asset_locks` —
    /// the same fee field Type 14 (address funding) reads.
    fn shield_from_asset_lock_min_fee(&self) -> Result<Credits, PlatformWalletError> {
        let pv = self.sdk.version();
        let asset_lock_base_cost_duffs = pv
            .dpp
            .state_transitions
            .identities
            .asset_locks
            .required_asset_lock_duff_balance_for_processing_start_for_address_funding;
        asset_lock_base_cost_duffs
            .checked_mul(CREDITS_PER_DUFF)
            .ok_or_else(|| {
                PlatformWalletError::ShieldedBuildError(
                    "platform version min-fee constant overflowed credits conversion".to_string(),
                )
            })
    }
}

/// Look up the asset-lock value in credits.
///
/// Preference order:
/// 1. If the proof is `Instant`, read directly from
///    `InstantAssetLockProof::output().value` — no manager lookup
///    needed; this also keeps the function callable in tests that
///    inject a synthetic IS proof.
/// 2. Otherwise (the IS-timeout-fallback path produced a CL proof
///    that doesn't carry the tx output), look up the tracked asset
///    lock by outpoint.
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
            let locks = wallet.asset_locks.list_tracked_locks().await;
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
            "asset lock value ({duffs} duffs) overflowed credits conversion"
        ))
    })
}

/// Build the Type 18 transition and broadcast-and-wait.
///
/// Extracted so `submit_with_cl_height_retry`'s closure stays compact
/// and the IS→CL fallback path can re-call it with the upgraded proof.
#[allow(clippy::too_many_arguments)]
async fn build_and_broadcast_shielded<AS, P>(
    sdk: std::sync::Arc<dash_sdk::Sdk>,
    recipient: OrchardAddress,
    shield_amount: Credits,
    proof: AssetLockProof,
    path: ::key_wallet::bip32::DerivationPath,
    asset_lock_signer: &AS,
    prover: &P,
    settings: Option<PutSettings>,
) -> Result<(), dash_sdk::Error>
where
    AS: ::key_wallet::signer::Signer,
    P: OrchardProver,
{
    let st = build_shield_from_asset_lock_transition_with_signer(
        &recipient,
        shield_amount,
        proof,
        &path,
        asset_lock_signer,
        prover,
        [0u8; 36],
        sdk.version(),
    )
    .await?;

    // Wait for proven execution rather than relay-ACK. Single-use
    // asset-lock proof: a false-positive on a transition Platform
    // later rejects would strand the L1 outpoint with no in-app
    // signal. The proven result is discarded; we only need the
    // confirmation that drive-abci committed.
    st.broadcast_and_wait::<StateTransitionProofResult>(&sdk, settings)
        .await?;
    Ok(())
}

/// Pre-flight check for the recipient list.
///
/// Today: non-empty, exactly one recipient. The multi-shape
/// `Vec<(OrchardAddress, Option<Credits>)>` API is exposed so the
/// caller signature is future-compatible — when DPP grows multi-output
/// Orchard bundles for Type 18, lifting this restriction is a
/// preflight-only change; no FFI / Swift / caller migration needed.
///
/// Generic over `T` so unit tests can pass `(u8, Option<Credits>)`
/// instead of constructing a curve-valid `OrchardAddress` for what
/// is really a length/cardinality check.
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
    // semantics will become: explicit `Some(_)` amounts pass through
    // unchanged; the (exactly one) `None` bucket receives the
    // residual `asset_lock_value − sum(explicit) − fee`.
    if recipients.len() != 1 {
        return Err(PlatformWalletError::AddressOperation(format!(
            "shielded_fund_from_asset_lock currently supports exactly one recipient \
             (multi-output Orchard bundles for Type 18 not yet wired through DPP); got {}",
            recipients.len()
        )));
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
    fn validate_accepts_single_recipient() {
        let v: Vec<(u8, Option<Credits>)> = vec![(0, None)];
        validate_shielded_recipients(&v).expect("single recipient must pass");
    }

    #[test]
    fn validate_accepts_single_recipient_with_some_amount() {
        // The Some(_) value is ignored today (single-recipient case
        // always receives the residual), but the shape stays valid
        // so the caller signature is future-compatible.
        let v: Vec<(u8, Option<Credits>)> = vec![(0, Some(500_000))];
        validate_shielded_recipients(&v).expect("single recipient with amount must pass");
    }
}
