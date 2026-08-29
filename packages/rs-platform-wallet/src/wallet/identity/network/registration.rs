//! Identity registration and top-up flows.
//!
//! Two public entry points — one to register, one to top up:
//!
//! - [`register_identity_with_funding`](IdentityWallet::register_identity_with_funding)
//! - [`top_up_identity_with_funding`](IdentityWallet::top_up_identity_with_funding)
//!
//! Each handles pre-flight, funding resolution, submission with
//! Platform-side retries, IS→CL fallback, and IdentityManager
//! bookkeeping. The SDK's `_with_signer` calls are issued inline
//! at the submission site — no thin "primitive" wrappers, since
//! a primitive that bypasses the recovery layers has no caller in
//! this codebase. If a single-shot use case ever materialises
//! (e.g. an external tool managing its own submission policy),
//! factor it out then; cheaper to add a method than to maintain a
//! dead one.
//!
//! ## Platform-side recovery layers
//!
//! Both methods wrap the SDK submission in two retry layers, in
//! this order:
//!
//! 1. **CL-height-too-low** (`InvalidAssetLockProofCoreChainHeightError`,
//!    consensus code 10506) — Platform's observed Core tip is briefly
//!    behind the wallet's SPV-verified CL. Bump
//!    `PutSettings::user_fee_increase` (changes ST signable bytes →
//!    different ST hash → bypasses Tenderdash's 24h invalid-tx cache)
//!    and resubmit the same proof. See
//!    [`submit_with_cl_height_retry`].
//!
//! 2. **IS-lock rejection** (`InvalidInstantAssetLockProofSignatureError`)
//!    — Drive rejected because the IS-lock signing quorum has rotated
//!    out. Detected via [`crate::error::is_instant_lock_proof_invalid`].
//!    Upgrade IS→CL via `upgrade_to_chain_lock_proof` and retry. The
//!    CL retry is itself wrapped in the CL-height-too-low loop.
//!
//! On the funding-build side, a third recovery handles the
//! Core-side IS timeout:
//!
//! 3. **IS-lock build-time timeout** —
//!    [`create_funded_asset_lock_proof`](crate::wallet::asset_lock::manager::AssetLockManager::create_funded_asset_lock_proof)
//!    returns `PlatformWalletError::FinalityTimeout` because the
//!    IS-lock didn't propagate within 300s. Resolved by re-entering
//!    `resume_asset_lock` (which will fall through to the
//!    `metadata.last_applied_chain_lock` fallback in `proof.rs`).
//!
//! All recovery paths share the same outpoint-keyed cleanup —
//! [`consume_asset_lock`](crate::wallet::asset_lock::manager::AssetLockManager::consume_asset_lock)
//! — once Platform finally accepts the submission.

use std::collections::BTreeMap;

use dpp::identity::accessors::{IdentityGettersV0, IdentitySettersV0};
use dpp::identity::signer::Signer;
use dpp::identity::v0::IdentityV0;
use dpp::identity::Identity;
use dpp::identity::IdentityPublicKey;
use dpp::identity::KeyID;
use dpp::identity::Purpose;
use dpp::identity::SecurityLevel;
use dpp::prelude::Identifier;
use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;

use dash_sdk::platform::transition::put_identity::PutIdentity;
use dash_sdk::platform::transition::put_settings::PutSettings;
use dash_sdk::platform::transition::top_up_identity::TopUpIdentity;

use crate::error::{is_instant_lock_proof_invalid, PlatformWalletError};
use crate::wallet::asset_lock::orchestration::{
    out_point_from_proof, submit_with_cl_height_retry, FundingResolution, ResolvedFunding,
};
use crate::wallet::asset_lock::AssetLockFunding;

use super::*;

// ---------------------------------------------------------------------------
// register
// ---------------------------------------------------------------------------

impl IdentityWallet {
    /// Register a new asset-lock-funded identity on Platform.
    ///
    /// Single entry point for every register-with-asset-lock case:
    ///
    /// 1. Pre-flight — validate `keys_map[0]` is a MASTER +
    ///    AUTHENTICATION key (the IdentityCreate transition itself
    ///    must be signed by a MASTER-level identity key, and we pin
    ///    that role on id=0 by convention).
    /// 2. Resolve the [`AssetLockFunding`] to an asset-lock proof +
    ///    derivation path.
    /// 3. Submit via
    ///    `Identity::put_to_platform_and_wait_for_response_with_signer`
    ///    inside `submit_with_cl_height_retry`, with IS→CL fallback
    ///    on **both** Core-side timeout (`FinalityTimeout`) and
    ///    Platform-side rejection
    ///    (`InvalidInstantAssetLockProofSignatureError`).
    /// 4. On success, add the confirmed identity to the local
    ///    `IdentityManager` and record each key's derivation breadcrumb.
    ///    Best-effort: Platform has already accepted, so a local
    ///    bookkeeping failure is logged, not propagated.
    /// 5. Remove the tracked asset lock (if any) — the credit output
    ///    has been consumed, so the entry is no longer needed.
    ///
    /// # The IS→CL fallback path
    ///
    /// The Core-side timeout fallback is the architectural fix this
    /// iter introduces. Before, `create_funded_asset_lock_proof`'s
    /// 300s IS-lock timeout was terminal: a chain-locked but
    /// IS-unlocked asset-lock would leave the funded DASH stranded.
    /// The fix uses the **same** asset-lock signer for the CL retry —
    /// no priv-key materialisation Rust-side — so the "no private keys
    /// outside Swift, even briefly between operations" architectural
    /// invariant is preserved.
    ///
    /// # Idempotency note
    ///
    /// The IS→CL retry is bounded (180s waiting for ChainLock). If the
    /// CL retry itself fails, the asset-lock stays tracked (cleanup
    /// only runs on Platform success) so subsequent registration
    /// attempts can resume via `FromExistingAssetLock`.
    pub async fn register_identity_with_funding<S, AS>(
        &self,
        funding: AssetLockFunding,
        identity_index: u32,
        keys_map: BTreeMap<u32, IdentityPublicKey>,
        identity_signer: &S,
        asset_lock_signer: &AS,
        settings: Option<PutSettings>,
    ) -> Result<Identity, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
        AS: ::key_wallet::signer::ExtendedPubKeySigner + Send + Sync,
    {
        // Step 1: pre-flight on the caller-supplied keys map.
        if keys_map.is_empty() {
            return Err(PlatformWalletError::InvalidIdentityData(
                "keys_map must contain at least one identity public key".to_string(),
            ));
        }
        {
            use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
            match keys_map.get(&0) {
                Some(k)
                    if k.security_level() == SecurityLevel::MASTER
                        && k.purpose() == Purpose::AUTHENTICATION => {}
                Some(_) => {
                    return Err(PlatformWalletError::InvalidIdentityData(
                        "keys_map[0] must be a MASTER-level AUTHENTICATION key \
                         (required to sign the IdentityCreate transition)"
                            .to_string(),
                    ));
                }
                None => {
                    return Err(PlatformWalletError::InvalidIdentityData(
                        "keys_map must include key id=0 with MASTER security level".to_string(),
                    ));
                }
            }
        }

        // Step 2: resolve funding to a proof + derivation path. The
        // resolver catches IS-lock timeouts and surfaces them as a
        // structured outcome carrying the tracked outpoint, so the
        // CL retry uses the SAME credit output (no new asset lock).
        let ResolvedFunding {
            proof,
            path,
            tracked_out_point,
        } = match self
            .asset_locks
            .resolve_funding_with_is_timeout_fallback(
                funding,
                AssetLockFundingType::IdentityRegistration,
                identity_index,
                asset_lock_signer,
            )
            .await?
        {
            FundingResolution::Resolved(rf) => rf,
            FundingResolution::IsTimeout { out_point } => {
                tracing::warn!(
                    "IS-lock did not propagate within 300s for funded identity registration \
                     (tx {}), falling back to ChainLock proof",
                    out_point.txid
                );
                let chain_proof = self
                    .asset_locks
                    .upgrade_to_chain_lock_proof(&out_point, None)
                    .await?;
                // Recover the credit-output derivation path. The
                // asset lock is now CL-attached (status advanced by
                // `upgrade_to_chain_lock_proof`'s caller path), so
                // `resume_asset_lock` short-circuits to the existing-
                // proof branch and just re-derives the path. This is
                // cheap (no SPV wait) and avoids duplicating the
                // path-derivation logic here.
                let (_, path) = self.asset_locks.resume_asset_lock(&out_point, None).await?;
                ResolvedFunding {
                    proof: chain_proof,
                    path,
                    tracked_out_point: Some(out_point),
                }
            }
        };

        // Build the placeholder identity ONCE so both the primary
        // attempt and the IS→CL retry submit the same key set
        // without a `keys_map` deep clone on the retry path.
        let placeholder = Identity::V0(IdentityV0 {
            id: Identifier::default(),
            public_keys: keys_map,
            balance: 0,
            revision: 0,
        });

        // Step 3: submit, with two layers of Platform-side fallback:
        //   - **CL-height-too-low** (transient): bump `user_fee_increase`
        //     and retry the same proof. See [`submit_with_cl_height_retry`].
        //   - **IS-lock rejection** (quorum rotated): upgrade IS→CL on
        //     the same credit-output outpoint — no new asset lock built,
        //     no new funding-tx broadcast.
        //
        // Both retries share the original `placeholder` Identity; the
        // CL-height retry also iterates inside the IS→CL fallback branch
        // so a freshly-upgraded CL proof gets the same patience.
        let proof_out_point = out_point_from_proof(&proof);
        let (submit_result, effective_proof) = match submit_with_cl_height_retry(settings, |s| {
            placeholder.put_to_platform_and_wait_for_response_with_signer(
                &self.sdk,
                proof.clone(),
                &path,
                asset_lock_signer,
                identity_signer,
                s,
            )
        })
        .await
        {
            Ok(identity) => (Ok(identity), proof.clone()),
            Err(e) if is_instant_lock_proof_invalid(&e) => {
                let out_point = proof_out_point;
                tracing::warn!(
                    "IS-lock proof rejected by Platform for identity registration (tx {}), \
                     retrying with ChainLock proof",
                    out_point.txid
                );
                let chain_proof = self
                    .asset_locks
                    .upgrade_to_chain_lock_proof(&out_point, None)
                    .await?;
                let submit_result = submit_with_cl_height_retry(settings, |s| {
                    placeholder.put_to_platform_and_wait_for_response_with_signer(
                        &self.sdk,
                        chain_proof.clone(),
                        &path,
                        asset_lock_signer,
                        identity_signer,
                        s,
                    )
                })
                .await;
                (submit_result, chain_proof)
            }
            Err(e) => (Err(e), proof.clone()),
        };
        let identity = self
            .asset_locks
            .reconcile_asset_lock_submit_result(
                submit_result,
                &proof_out_point,
                &effective_proof,
                None,
            )
            .await?;

        // Step 4 (best-effort): bookkeeping — add to local
        // IdentityManager + record key derivation breadcrumbs.
        //
        // Platform has ALREADY accepted the registration, so a local
        // bookkeeping failure must NOT propagate as `Err` — the caller
        // would report failure for an identity that exists on chain,
        // and the early return would skip Step 5's `consume_asset_lock`,
        // leaving the spent lock in the Resumable Funding list where a
        // Resume gets Platform's deterministic "lock already consumed"
        // rejection. A missed local add self-heals on the next identity
        // re-sync. This mirrors `register_from_addresses` Step 3.
        {
            let mut wm = self.wallet_manager.write().await;
            match wm.get_wallet_info_mut(&self.wallet_id) {
                Some(info) => match info.identity_manager.add_identity(
                    identity.clone(),
                    identity_index,
                    self.wallet_id,
                    &self.persister,
                ) {
                    Ok(()) => {
                        let wallet_id = self.wallet_id;
                        let identity_id = identity.id();
                        let public_keys: Vec<(KeyID, IdentityPublicKey)> = identity
                            .public_keys()
                            .iter()
                            .map(|(k, v)| (*k, v.clone()))
                            .collect();

                        if let Some(managed) =
                            info.identity_manager.managed_identity_mut(&identity_id)
                        {
                            managed.wallet_id = Some(wallet_id);
                            for (key_id, pub_key) in public_keys {
                                let key_index = key_id;
                                managed
                                    .add_key(
                                        pub_key,
                                        Some((wallet_id, identity_index, key_index)),
                                        &self.persister,
                                    )
                                    .map_err(|e| {
                                        PlatformWalletError::Persistence(format!(
                                            "identity key not persisted after registration: {e}"
                                        ))
                                    })?;
                            }
                        }
                    }
                    Err(e) => {
                        // Breadcrumbs are skipped too: `IdentityAlreadyExists`
                        // can mean an out-of-wallet entry, and stamping
                        // `wallet_id` on one without moving buckets would
                        // contradict the manager's location index.
                        tracing::warn!(
                            error = %e,
                            identity_id = %identity.id(),
                            "register_identity_with_funding: identity registered on \
                             Platform but local add_identity failed; continuing so \
                             the spent asset lock is still consumed"
                        );
                    }
                },
                None => {
                    tracing::warn!(
                        identity_id = %identity.id(),
                        "register_identity_with_funding: identity registered on \
                         Platform but wallet info was not found locally; skipping \
                         local persistence"
                    );
                }
            }
        }

        // Step 5: clean up the tracked asset lock — Platform has
        // accepted the registration and the credit output is now
        // consumed. Both `AssetLockFunding` variants produce a tracked
        // lock so `tracked_out_point` is always `Some` today; the
        // `Option` is retained for future variants that may not have
        // wallet-owned lifecycle.
        if let Some(out_point) = tracked_out_point {
            // Cleanup failure here can only mean WalletNotFound
            // (the wallet handle that just registered an identity
            // vanished). Surface as a warn — the identity DID
            // register successfully on Platform, so propagating the
            // error to the caller would be misleading.
            if let Err(e) = self.asset_locks.consume_asset_lock(&out_point).await {
                tracing::warn!(
                    outpoint = %out_point,
                    error = %e,
                    "consume_asset_lock failed after successful Platform submit"
                );
            }
        }

        Ok(identity)
    }
}

// ---------------------------------------------------------------------------
// top-up
// ---------------------------------------------------------------------------

impl IdentityWallet {
    /// Top up an existing identity's credit balance.
    ///
    /// Mirror of [`register_identity_with_funding`](Self::register_identity_with_funding)
    /// for top-ups:
    ///
    /// 1. Look up the identity by `identity_id` in the local
    ///    `IdentityManager`. Return `IdentityNotFound` if missing.
    /// 2. Resolve the [`AssetLockFunding`] to an asset-lock proof.
    /// 3. Submit via `Identity::top_up_identity_with_signer` inside
    ///    `submit_with_cl_height_retry`, with IS→CL fallback on
    ///    Core-side timeout and Platform-side rejection (same as
    ///    register).
    /// 4. Persist the new credit balance + remove the tracked asset
    ///    lock. Best-effort: Platform has already accepted, so a local
    ///    bookkeeping failure is logged, not propagated.
    pub async fn top_up_identity_with_funding<AS>(
        &self,
        identity_id: &Identifier,
        funding: AssetLockFunding,
        asset_lock_signer: &AS,
        settings: Option<PutSettings>,
    ) -> Result<u64, PlatformWalletError>
    where
        AS: ::key_wallet::signer::ExtendedPubKeySigner + Send + Sync,
    {
        // Step 1: retrieve the identity + its HD index.
        let (identity, identity_index) = {
            let wm = self.wallet_manager.read().await;
            let info = wm.get_wallet_info(&self.wallet_id).ok_or_else(|| {
                PlatformWalletError::WalletNotFound(
                    "Wallet info not found in wallet manager".to_string(),
                )
            })?;
            let manager = &info.identity_manager;
            let identity = manager
                .identity(identity_id)
                .map(|m| m.identity.clone())
                .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;
            let index = manager
                .identity_index(identity_id)
                .ok_or(PlatformWalletError::IdentityIndexNotSet(*identity_id))?;
            (identity, index)
        };

        // Step 2: resolve funding. Same IS→CL fallback shape as
        // `register_identity_with_funding` — see that method for the
        // architectural rationale.
        let ResolvedFunding {
            proof,
            path,
            tracked_out_point,
        } = match self
            .asset_locks
            .resolve_funding_with_is_timeout_fallback(
                funding,
                AssetLockFundingType::IdentityTopUp,
                identity_index,
                asset_lock_signer,
            )
            .await?
        {
            FundingResolution::Resolved(rf) => rf,
            FundingResolution::IsTimeout { out_point } => {
                tracing::warn!(
                    "IS-lock did not propagate within 300s for funded identity top-up \
                     (tx {}), falling back to ChainLock proof",
                    out_point.txid
                );
                let chain_proof = self
                    .asset_locks
                    .upgrade_to_chain_lock_proof(&out_point, None)
                    .await?;
                let (_, path) = self.asset_locks.resume_asset_lock(&out_point, None).await?;
                ResolvedFunding {
                    proof: chain_proof,
                    path,
                    tracked_out_point: Some(out_point),
                }
            }
        };

        // Step 3: submit. Two Platform-side fallback layers (matches
        // `register_identity_with_funding`): CL-height-too-low retries
        // bump `user_fee_increase` to bypass Tenderdash's invalid-tx
        // cache, and IS-lock rejection triggers an IS→CL upgrade on the
        // same outpoint.
        let proof_out_point = out_point_from_proof(&proof);
        let (submit_result, effective_proof) = match submit_with_cl_height_retry(settings, |s| {
            identity.top_up_identity_with_signer(
                &self.sdk,
                proof.clone(),
                &path,
                asset_lock_signer,
                s,
            )
        })
        .await
        {
            Ok(balance) => (Ok(balance), proof.clone()),
            Err(e) if is_instant_lock_proof_invalid(&e) => {
                let out_point = proof_out_point;
                tracing::warn!(
                    "IS-lock proof rejected by Platform for identity top-up (tx {}), \
                     retrying with ChainLock proof",
                    out_point.txid
                );
                let chain_proof = self
                    .asset_locks
                    .upgrade_to_chain_lock_proof(&out_point, None)
                    .await?;
                let submit_result = submit_with_cl_height_retry(settings, |s| {
                    identity.top_up_identity_with_signer(
                        &self.sdk,
                        chain_proof.clone(),
                        &path,
                        asset_lock_signer,
                        s,
                    )
                })
                .await;
                (submit_result, chain_proof)
            }
            Err(e) => (Err(e), proof.clone()),
        };
        let new_balance = self
            .asset_locks
            .reconcile_asset_lock_submit_result(
                submit_result,
                &proof_out_point,
                &effective_proof,
                None,
            )
            .await?;

        // Step 4 (best-effort): persist the new balance + clean up the
        // tracked lock.
        //
        // Platform has ALREADY accepted the top-up, so a missing local
        // wallet must NOT propagate as `Err` — the caller would report
        // failure for credits that exist on chain, and the early return
        // would skip the `consume_asset_lock` below, leaving the spent
        // lock in the Resumable Funding list where a Resume gets
        // Platform's deterministic "lock already consumed" rejection.
        // The stale local balance self-heals on the next identity
        // re-sync. Same posture as register's Step 4.
        {
            let mut wm = self.wallet_manager.write().await;
            match wm.get_wallet_info_mut(&self.wallet_id) {
                Some(info) => {
                    if let Some(managed) = info.identity_manager.managed_identity_mut(identity_id) {
                        let prev_balance = managed.identity.balance();
                        managed.identity.set_balance(new_balance);
                        if let Err(source) =
                            self.persister.store(managed.snapshot_changeset().into())
                        {
                            managed.identity.set_balance(prev_balance);
                            return Err(PlatformWalletError::PersistedAfterOnChainSuccess {
                                identity: *identity_id,
                                op: "top_up",
                                source,
                            });
                        }
                    }
                }
                None => {
                    tracing::warn!(
                        identity = %identity_id,
                        "top_up_identity_with_funding: top-up accepted on Platform \
                         but wallet info was not found locally; skipping balance \
                         persistence"
                    );
                }
            }
        }
        if let Some(out_point) = tracked_out_point {
            // Cleanup failure here can only mean WalletNotFound
            // (the wallet handle that just registered an identity
            // vanished). Surface as a warn — the identity DID
            // register successfully on Platform, so propagating the
            // error to the caller would be misleading.
            if let Err(e) = self.asset_locks.consume_asset_lock(&out_point).await {
                tracing::warn!(
                    outpoint = %out_point,
                    error = %e,
                    "consume_asset_lock failed after successful Platform submit"
                );
            }
        }

        Ok(new_balance)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

// `find_tracked_unproven_lock` was removed when
// `PlatformWalletError::FinalityTimeout` was widened to carry the full
// `OutPoint` (previously only the `Txid`). The IS→CL fallback now reads
// the outpoint directly off the error payload — no BTreeMap walk by
// `(funding_type, identity_index)` is needed, which also closes the
// non-determinism gap when multiple unproven locks shared that key.

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::is_instant_lock_timeout;
    use dashcore::{OutPoint, Txid};

    /// Pins the IS-timeout discriminator: only `FinalityTimeout`
    /// matches, so the IS→CL fallback arms route exactly the cases
    /// we expect. Companion to `is_instant_lock_proof_invalid`
    /// (which discriminates SDK errors at the Platform-rejection
    /// boundary).
    #[test]
    fn is_instant_lock_timeout_only_matches_finality_timeout() {
        let timeout = PlatformWalletError::FinalityTimeout(OutPoint {
            txid: Txid::from([0u8; 32]),
            vout: 0,
        });
        assert!(
            is_instant_lock_timeout(&timeout),
            "FinalityTimeout must route to IS→CL fallback"
        );

        // Adjacent error shapes that share the asset-lock domain but
        // are NOT timeouts — must NOT trigger the fallback.
        let expired = PlatformWalletError::AssetLockExpired("CL not yet available".to_string());
        assert!(
            !is_instant_lock_timeout(&expired),
            "AssetLockExpired must NOT trigger IS→CL fallback \
             (the lock is already past the CL grace window)"
        );

        let not_cl = PlatformWalletError::AssetLockNotChainLocked("missing".to_string());
        assert!(
            !is_instant_lock_timeout(&not_cl),
            "AssetLockNotChainLocked must NOT trigger IS→CL fallback"
        );

        let wait_err = PlatformWalletError::AssetLockProofWait("not tracked".to_string());
        assert!(
            !is_instant_lock_timeout(&wait_err),
            "AssetLockProofWait must NOT trigger IS→CL fallback \
             (wallet-state mismatch is a hard failure)"
        );
    }
}
