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
    /// * `recipients` — Recipient `OrchardAddress` + explicit credit
    ///   amount per entry. The shape mirrors the platform-address
    ///   `BTreeMap<PlatformAddress, _>` API for future-compatibility
    ///   with multi-output Orchard bundles, but today the pre-flight
    ///   enforces exactly one recipient (Type 18's bundle builder is
    ///   single-output).
    ///
    ///   Unlike platform-address funding, the caller passes the
    ///   credit amount explicitly. For Type 18 there is no
    ///   protocol-side `AddressFundsFeeStrategy`; the Orchard
    ///   `value_balance` (= `sum(recipient credits)`) is baked into
    ///   the Halo 2 proof at build time, and the asset-lock value
    ///   minus that covers the Platform fee. The caller is
    ///   responsible for sizing the L1 lock to cover both.
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
        recipients: Vec<(OrchardAddress, Credits)>,
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

        // Caller specifies shield_amount per recipient (Type 18's
        // Orchard `value_balance` is baked into the Halo 2 proof at
        // build time, unlike address-funding's protocol-level fee
        // strategy). Identities and platform-addresses take
        // `amount_duffs` from the caller for the same reason —
        // Platform handles their fee math inside the transition,
        // shielded can't.
        let (recipient, shield_amount) =
            *recipients.first().expect("preflight enforces len() == 1");

        // Sizing sanity check on the FromWalletBalance path: refuse
        // obviously-undersized configurations BEFORE we broadcast
        // the asset-lock tx (single-use L1 funds) or spend ~30s
        // building a Halo 2 proof Platform would reject.
        //
        // Best-effort, not authoritative — Platform's real fee
        // depends on state we don't track. We only check the lower
        // bound: lock_credits >= shield_amount + min_required_fee.
        // The resume path takes an existing tracked outpoint; the
        // lock value is already pinned on-chain, and checking it
        // here would re-introduce the asset-lock-value lookup we
        // deliberately removed in favour of caller-supplied
        // `shield_amount`.
        if let AssetLockFunding::FromWalletBalance { amount_duffs, .. } = &funding {
            let lock_credits = (*amount_duffs).saturating_mul(CREDITS_PER_DUFF);
            let min_fee_duffs = self
                .sdk
                .version()
                .dpp
                .state_transitions
                .identities
                .asset_locks
                .required_asset_lock_duff_balance_for_processing_start_for_address_funding;
            let min_fee_credits = min_fee_duffs.saturating_mul(CREDITS_PER_DUFF);
            let required = shield_amount.saturating_add(min_fee_credits);
            if lock_credits < required {
                return Err(PlatformWalletError::ShieldedBuildError(format!(
                    "asset lock ({lock_credits} credits, from {amount_duffs} duffs) cannot cover \
                     shield_amount ({shield_amount}) + protocol min fee ({min_fee_credits}); \
                     refusing to broadcast a single-use asset lock and build a ~30s Halo 2 proof \
                     for a submission Platform would reject"
                )));
            }
        }

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

        tracing::info!(shield_amount, "Shielded fund-from-asset-lock succeeded");

        Ok(())
    }
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
/// Today: non-empty, exactly one recipient, non-zero amount. The
/// multi-shape `Vec<(OrchardAddress, Credits)>` API is exposed so
/// the caller signature is future-compatible — when DPP grows
/// multi-output Orchard bundles for Type 18, lifting the
/// single-recipient restriction is a preflight-only change; no FFI
/// / Swift / caller migration needed.
///
/// Generic over `T` so unit tests can pass `(u8, Credits)` instead
/// of constructing a curve-valid `OrchardAddress` for what is
/// really a length / cardinality check.
pub(super) fn validate_shielded_recipients<T>(
    recipients: &[(T, Credits)],
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
    // semantics will become: each recipient's `Credits` flows into
    // its Orchard output, and the bundle's `value_balance` becomes
    // `sum(credits)`.
    if recipients.len() != 1 {
        return Err(PlatformWalletError::AddressOperation(format!(
            "shielded_fund_from_asset_lock currently supports exactly one recipient \
             (multi-output Orchard bundles for Type 18 not yet wired through DPP); got {}",
            recipients.len()
        )));
    }
    if recipients[0].1 == 0 {
        return Err(PlatformWalletError::ShieldedBuildError(
            "shield amount must be > 0".to_string(),
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
        let v: Vec<(u8, Credits)> = Vec::new();
        let err = validate_shielded_recipients(&v).expect_err("empty must reject");
        assert!(format!("{err}").contains("at least one recipient"));
    }

    #[test]
    fn validate_rejects_multi_recipient_for_now() {
        let v: Vec<(u8, Credits)> = vec![(1, 100), (2, 200)];
        let err = validate_shielded_recipients(&v).expect_err("multi-recipient must reject (TODO)");
        let msg = format!("{err}");
        assert!(
            msg.contains("exactly one recipient"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn validate_rejects_zero_amount() {
        let v: Vec<(u8, Credits)> = vec![(0, 0)];
        let err = validate_shielded_recipients(&v).expect_err("zero amount must reject");
        assert!(
            format!("{err}").contains("must be > 0"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn validate_accepts_single_recipient_with_amount() {
        let v: Vec<(u8, Credits)> = vec![(0, 500_000)];
        validate_shielded_recipients(&v).expect("single recipient with amount must pass");
    }
}
