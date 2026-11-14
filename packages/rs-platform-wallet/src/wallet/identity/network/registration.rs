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
//! 2. **Unusable IS proof** — Drive rejected the IS lock because its
//!    signing quorum has rotated out
//!    (`InvalidInstantAssetLockProofSignatureError`), or the IS proof made
//!    the transition larger than `max_state_transition_size`
//!    (`StateTransitionMaxSizeExceededError`, refused by the SDK before
//!    broadcast), or its funding transaction has more inputs than an IS
//!    proof may carry (`IdentityAssetLockTransactionTooManyInputsError`;
//!    from protocol 15 a ChainLock proof is not held to that cap). Detected via
//!    [`needs_chain_lock_proof`]. Upgrade IS→CL via
//!    `upgrade_to_persisted_chain_lock_proof`, which attaches the proof to
//!    the tracked row first, and retry. The CL retry is itself wrapped in
//!    the CL-height-too-low loop, and a ChainLock proof Platform rejects
//!    for its height comes off the row (`invalidate_rejected_chain_proof`).
//!
//! On the funding-build side, a third recovery handles the
//! Core-side IS timeout:
//!
//! 3. **IS-lock build-time timeout** —
//!    [`create_funded_asset_lock_proof`](crate::wallet::asset_lock::manager::AssetLockManager::create_funded_asset_lock_proof)
//!    returns `PlatformWalletError::FinalityTimeout` because the
//!    IS-lock didn't propagate within 300s. Resolved by
//!    `resolve_chain_proof_after_is_timeout`: build the ChainLock proof,
//!    attach it to the row, then resume for the credit-output path.
//!
//! All recovery paths share the same outpoint-keyed cleanup —
//! [`consume_asset_lock`](crate::wallet::asset_lock::manager::AssetLockManager::consume_asset_lock)
//! — once Platform finally accepts the submission.

use std::collections::BTreeMap;

use dash_sdk::dapi_grpc::platform::v0::ResponseMetadata;
use dpp::identity::accessors::IdentityGettersV0;
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
use dash_sdk::platform::transition::top_up_identity::TopUpIdentityWithMetadata;
use dash_sdk::platform::Fetch;

use crate::changeset::AssetLockRecipient;
use crate::error::PlatformWalletError;
use crate::wallet::asset_lock::orchestration::{
    out_point_from_proof, FundingResolution, ResolvedFunding,
};
use crate::wallet::asset_lock::AssetLockFunding;
use crate::BlockTime;

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
    /// The IS→CL retry waits for the ChainLock without a bound (a
    /// broadcast lock is pending, never failed). If the CL retry itself
    /// fails, the asset-lock stays tracked (cleanup only runs on Platform
    /// success), carrying the ChainLock proof, so subsequent registration
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
                None,
                asset_lock_signer,
                None,
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
                let (chain_proof, path) = self
                    .asset_locks
                    .resolve_chain_proof_after_is_timeout(&out_point, None)
                    .await?;
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
        //   - **Unusable IS proof** (quorum rotated, the IS proof made the
        //     transition too large, or its funding transaction is over the
        //     input cap): upgrade IS→CL on the same
        //     credit-output outpoint — no new asset lock built, no new
        //     funding-tx broadcast.
        //
        // Both retries share the original `placeholder` Identity; the
        // CL-height retry also iterates inside the IS→CL fallback branch
        // so a freshly-upgraded CL proof gets the same patience.
        let proof_out_point = out_point_from_proof(&proof);
        let (submit_result, effective_proof) = self
            .asset_locks
            .submit_with_chain_lock_fallback(
                proof,
                settings,
                None,
                "identity registration",
                |proof, s| {
                    placeholder.put_to_platform_and_wait_for_response_with_signer(
                        &self.sdk,
                        proof,
                        &path,
                        asset_lock_signer,
                        identity_signer,
                        s,
                    )
                },
            )
            .await?;
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
            use dpp::identity::accessors::IdentityGettersV0;

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

        // Steps 2–3. A resumed unbound top-up lock pays this identity only if
        // its row recorded it; a new lock is bound and records nothing.
        let (mut new_balance, metadata, tracked_out_point) = self
            .submit_top_up_with_funding(
                &identity,
                funding,
                AssetLockFundingType::IdentityTopUp,
                identity_index,
                asset_lock_signer,
                settings,
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
                        new_balance = managed.persist_confirmed_balance(
                            new_balance,
                            BlockTime::from(metadata),
                            &self.persister,
                        );
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

    /// Top up someone else's identity — one this wallet does not manage —
    /// from the wallet's Core balance.
    ///
    /// 1. Fetch the identity from Platform (proved) instead of the local
    ///    `IdentityManager`. An identity Platform does not know is
    ///    [`PlatformWalletError::IdentityNotFound`]; nothing is built.
    /// 2. Fund the lock from the wallet's unbound top-up account
    ///    (`IdentityTopUpNotBound`, DIP-9), which needs no registration
    ///    index. The lock's row records `identity_id` and is stored before
    ///    the lock is broadcast, so an interrupted top-up resumes to this
    ///    identity and to no other (see
    ///    [`Self::resume_identity_top_up_to_recorded_recipient`]). Requires a
    ///    persister declaring `ASSET_LOCK_IDENTITY_RECIPIENTS`.
    /// 3. Submit the `IdentityTopUp` with the same IS→CL fallbacks as the
    ///    managed top-up, and return the identity's proved new balance.
    /// 4. Consume the tracked lock. Nothing is written to the identity
    ///    manager: the identity is not this wallet's.
    ///
    /// `funding` may also be an existing unbound lock that recorded this
    /// identity; a lock recording another identity is refused.
    pub async fn top_up_external_identity_with_funding<AS>(
        &self,
        identity_id: &Identifier,
        funding: AssetLockFunding,
        asset_lock_signer: &AS,
        settings: Option<PutSettings>,
    ) -> Result<u64, PlatformWalletError>
    where
        AS: ::key_wallet::signer::ExtendedPubKeySigner + Send + Sync,
    {
        // TODO(dashpay/platform#5860): report a failed lookup as
        // `PlatformWalletError::IdentityLookupFailed` (FFI 80) once #5860
        // merges, as its external top-up from Platform addresses does. Until
        // then it surfaces as the plain SDK error; either way nothing is built.
        let identity = Identity::fetch(&self.sdk, *identity_id)
            .await
            .map_err(PlatformWalletError::Sdk)?
            .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;

        let (new_balance, _metadata, tracked_out_point) = self
            .submit_top_up_with_funding(
                &identity,
                funding,
                AssetLockFundingType::IdentityTopUpNotBound,
                0,
                asset_lock_signer,
                settings,
            )
            .await?;

        if let Some(out_point) = tracked_out_point {
            // Platform accepted the top-up: a cleanup failure is logged, not
            // reported, as for the managed top-up.
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

    /// Resume an interrupted unbound identity top-up to the identity its lock
    /// recorded, whoever's it is. The caller names no identity, so a host's
    /// recovery cannot redirect someone else's top-up into one of its own
    /// identities.
    ///
    /// A recorded identity this wallet manages is topped up through
    /// [`Self::top_up_identity_with_funding`] (which also updates its local
    /// balance); any other through
    /// [`Self::top_up_external_identity_with_funding`]. Returns the identity
    /// paid and its proved new balance.
    ///
    /// A lock that records no identity (rebuilt from the chain) is
    /// [`PlatformWalletError::AssetLockRecipientUnknown`] until the user
    /// assigns one; a lock that is not an unbound top-up is
    /// [`PlatformWalletError::AssetLockFundingMismatch`].
    pub async fn resume_identity_top_up_to_recorded_recipient<AS>(
        &self,
        out_point: dashcore::OutPoint,
        asset_lock_signer: &AS,
        settings: Option<PutSettings>,
    ) -> Result<(Identifier, u64), PlatformWalletError>
    where
        AS: ::key_wallet::signer::ExtendedPubKeySigner + Send + Sync,
    {
        let (funding_type, identity_index, _status, recorded) = self
            .asset_locks
            .tracked_resume_metadata(&out_point)
            .await
            .ok_or(PlatformWalletError::AssetLockNotTracked(out_point))?;
        if funding_type != AssetLockFundingType::IdentityTopUpNotBound {
            return Err(PlatformWalletError::AssetLockFundingMismatch {
                out_point,
                expected_funding_type: AssetLockFundingType::IdentityTopUpNotBound,
                expected_identity_index: 0,
                actual_funding_type: funding_type,
                actual_identity_index: identity_index,
            });
        }
        let identity_id =
            recorded.ok_or(PlatformWalletError::AssetLockRecipientUnknown(out_point))?;
        let managed = {
            let wm = self.wallet_manager.read().await;
            wm.get_wallet_info(&self.wallet_id).is_some_and(|info| {
                info.identity_manager.identity(&identity_id).is_some()
                    && info.identity_manager.identity_index(&identity_id).is_some()
            })
        };
        let funding = AssetLockFunding::FromExistingAssetLock {
            out_point,
            consume_invitation_voucher: false,
        };
        let new_balance = if managed {
            self.top_up_identity_with_funding(&identity_id, funding, asset_lock_signer, settings)
                .await?
        } else {
            self.top_up_external_identity_with_funding(
                &identity_id,
                funding,
                asset_lock_signer,
                settings,
            )
            .await?
        };
        Ok((identity_id, new_balance))
    }

    /// Steps 2–3 of a Core-funded top-up, shared by the managed
    /// ([`Self::top_up_identity_with_funding`]) and external
    /// ([`Self::top_up_external_identity_with_funding`]) routes: resolve the
    /// funding to a proof (with the IS→CL fallback), submit the
    /// `IdentityTopUp` for `identity`, and reconcile the result. The lock pays
    /// `identity`: a new unbound lock records it, and an existing unbound
    /// lock must have recorded it. Returns the proved new balance, the
    /// response metadata and the tracked outpoint to consume.
    #[allow(clippy::too_many_arguments)]
    async fn submit_top_up_with_funding<AS>(
        &self,
        identity: &Identity,
        funding: AssetLockFunding,
        funding_type: AssetLockFundingType,
        identity_index: u32,
        asset_lock_signer: &AS,
        settings: Option<PutSettings>,
    ) -> Result<(u64, ResponseMetadata, Option<dashcore::OutPoint>), PlatformWalletError>
    where
        AS: ::key_wallet::signer::ExtendedPubKeySigner + Send + Sync,
    {
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
                funding_type,
                identity_index,
                Some(AssetLockRecipient::Identity(identity.id())),
                asset_lock_signer,
                None,
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
                let (chain_proof, path) = self
                    .asset_locks
                    .resolve_chain_proof_after_is_timeout(&out_point, None)
                    .await?;
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
        // cache, and an unusable IS proof triggers an IS→CL upgrade on
        // the same outpoint.
        let proof_out_point = out_point_from_proof(&proof);
        let (submit_result, effective_proof) = self
            .asset_locks
            .submit_with_chain_lock_fallback(
                proof,
                settings,
                None,
                "identity top-up",
                |proof, s| {
                    identity.top_up_identity_with_signer_with_metadata(
                        &self.sdk,
                        proof,
                        &path,
                        asset_lock_signer,
                        s,
                    )
                },
            )
            .await?;
        let (new_balance, metadata) = self
            .asset_locks
            .reconcile_asset_lock_submit_result(
                submit_result,
                &proof_out_point,
                &effective_proof,
                None,
            )
            .await?;
        Ok((new_balance, metadata, tracked_out_point))
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

// `PlatformWalletError::FinalityTimeout` carries the full `OutPoint`, so
// the IS→CL fallback reads the outpoint directly off the error payload —
// no BTreeMap walk by `(funding_type, identity_index)` is needed, which
// also closes the non-determinism gap when multiple unproven locks share
// that key.

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::is_instant_lock_timeout;
    use dashcore::{OutPoint, Txid};

    use crate::test_support::{funded_wallet_manager, platform_wallet_with_sdk, WalletSigner};
    use crate::wallet::asset_lock::tracked::{AssetLockStatus, TrackedAssetLock};
    use crate::wallet::platform_wallet::PlatformWallet;
    use key_wallet::account::account_type::StandardAccountType;

    /// A funded wallet over a mock SDK that answers "no such identity" for
    /// `unknown`.
    async fn wallet_where_platform_does_not_know(
        unknown: Identifier,
    ) -> (PlatformWallet, WalletSigner) {
        let (wallet_manager, wallet_id, _generation, signer) =
            funded_wallet_manager(StandardAccountType::BIP44Account).await;
        let network = {
            let wm = wallet_manager.read().await;
            wm.get_wallet_and_info(&wallet_id).unwrap().0.network
        };
        let mut sdk = dash_sdk::SdkBuilder::new_mock()
            .with_network(network)
            .with_version(dpp::version::PlatformVersion::latest())
            .build()
            .expect("mock sdk");
        sdk.mock()
            .expect_fetch(unknown, None::<Identity>)
            .await
            .expect("identity expectation");
        (
            platform_wallet_with_sdk(wallet_manager, wallet_id, sdk).await,
            signer,
        )
    }

    /// Track a proof-less lock of `funding_type` recording `recorded`.
    async fn track_lock(
        wallet: &PlatformWallet,
        vout: u32,
        funding_type: AssetLockFundingType,
        recorded: Option<Identifier>,
    ) -> OutPoint {
        let transaction = dashcore::Transaction {
            version: 3,
            lock_time: 0,
            input: vec![],
            output: vec![],
            special_transaction_payload: None,
        };
        let out_point = OutPoint::new(Txid::from([0x61u8; 32]), vout);
        let mut wm = wallet.identity().wallet_manager.write().await;
        wm.get_wallet_info_mut(&wallet.wallet_id())
            .unwrap()
            .tracked_asset_locks
            .insert(
                out_point,
                TrackedAssetLock {
                    operation_id: None,
                    out_point,
                    transaction,
                    account_index: 0,
                    funding_type,
                    identity_index: 0,
                    amount: 1_000_000,
                    status: AssetLockStatus::Broadcast,
                    proof: None,
                    shielded_recipient: None,
                    identity_recipient: recorded,
                },
            );
        out_point
    }

    /// An identity Platform does not know is refused before any lock is
    /// built: nothing is tracked afterwards.
    #[tokio::test]
    async fn should_refuse_an_external_top_up_of_an_identity_platform_does_not_know() {
        let unknown = Identifier::from([0x62u8; 32]);
        let (wallet, signer) = wallet_where_platform_does_not_know(unknown).await;

        let result = wallet
            .identity()
            .top_up_external_identity_with_funding(
                &unknown,
                AssetLockFunding::FromWalletBalance {
                    amount_duffs: 1_000_000,
                    account_index: 0,
                },
                &signer,
                None,
            )
            .await;

        assert!(
            matches!(result, Err(PlatformWalletError::IdentityNotFound(id)) if id == unknown),
            "{result:?}"
        );
        assert!(wallet.asset_locks().list_tracked_locks().await.is_empty());
    }

    /// A known identity is still refused before any lock is built when the
    /// host's persister cannot keep the lock's recipient across a restart.
    #[tokio::test]
    async fn should_refuse_an_external_top_up_a_persister_cannot_record() {
        let known = Identifier::from([0x64u8; 32]);
        let (wallet_manager, wallet_id, _generation, signer) =
            funded_wallet_manager(StandardAccountType::BIP44Account).await;
        let network = {
            let wm = wallet_manager.read().await;
            wm.get_wallet_and_info(&wallet_id).unwrap().0.network
        };
        let mut sdk = dash_sdk::SdkBuilder::new_mock()
            .with_network(network)
            .with_version(dpp::version::PlatformVersion::latest())
            .build()
            .expect("mock sdk");
        let identity = Identity::V0(IdentityV0 {
            id: known,
            public_keys: BTreeMap::new(),
            balance: 5,
            revision: 0,
        });
        sdk.mock()
            .expect_fetch(known, Some(identity))
            .await
            .expect("identity expectation");
        let wallet = platform_wallet_with_sdk(wallet_manager, wallet_id, sdk).await;

        let result = wallet
            .identity()
            .top_up_external_identity_with_funding(
                &known,
                AssetLockFunding::FromWalletBalance {
                    amount_duffs: 1_000_000,
                    account_index: 0,
                },
                &signer,
                None,
            )
            .await;

        assert!(
            matches!(&result, Err(PlatformWalletError::Persistence(message))
                if message.contains("asset_lock_identity_recipients")),
            "{result:?}"
        );
        assert!(wallet.asset_locks().list_tracked_locks().await.is_empty());
    }

    /// The resume names no identity: it pays the one the lock recorded, or
    /// nobody. A lock that is not an unbound top-up, or records no identity,
    /// is refused; a recorded identity this wallet does not manage takes the
    /// external route (here Platform does not know it, so nothing is paid).
    /// Every refused lock stays tracked.
    #[tokio::test]
    async fn should_resume_an_unbound_top_up_only_to_its_recorded_identity() {
        let recorded = Identifier::from([0x63u8; 32]);
        let (wallet, signer) = wallet_where_platform_does_not_know(recorded).await;
        let resume = |out_point| {
            wallet
                .identity()
                .resume_identity_top_up_to_recorded_recipient(out_point, &signer, None)
        };

        let missing = OutPoint::new(Txid::from([0x60u8; 32]), 0);
        assert!(matches!(
            resume(missing).await,
            Err(PlatformWalletError::AssetLockNotTracked(o)) if o == missing
        ));

        let bound = track_lock(&wallet, 0, AssetLockFundingType::IdentityTopUp, None).await;
        assert!(matches!(
            resume(bound).await,
            Err(PlatformWalletError::AssetLockFundingMismatch { .. })
        ));

        let unassigned = track_lock(
            &wallet,
            1,
            AssetLockFundingType::IdentityTopUpNotBound,
            None,
        )
        .await;
        assert!(matches!(
            resume(unassigned).await,
            Err(PlatformWalletError::AssetLockRecipientUnknown(o)) if o == unassigned
        ));

        let external = track_lock(
            &wallet,
            2,
            AssetLockFundingType::IdentityTopUpNotBound,
            Some(recorded),
        )
        .await;
        assert!(matches!(
            resume(external).await,
            Err(PlatformWalletError::IdentityNotFound(id)) if id == recorded
        ));

        assert_eq!(wallet.asset_locks().list_tracked_locks().await.len(), 3);
    }

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
