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

use dashcore::OutPoint;
use dpp::consensus::basic::BasicError;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::ConsensusError;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::signer::Signer;
use dpp::identity::v0::IdentityV0;
use dpp::identity::Identity;
use dpp::identity::IdentityPublicKey;
use dpp::identity::Purpose;
use dpp::identity::SecurityLevel;
use dpp::prelude::Identifier;
use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;

use dash_sdk::platform::dashpay::identity_id_for_out_point;
use dash_sdk::platform::transition::put_identity::PutIdentity;
use dash_sdk::platform::transition::put_settings::PutSettings;
use dash_sdk::platform::transition::top_up_identity::TopUpIdentityWithMetadata;

use crate::changeset::KeyWithBreadcrumb;
use crate::error::PlatformWalletError;
use crate::wallet::asset_lock::orchestration::{
    out_point_from_proof, FundingResolution, ResolvedFunding,
};
use crate::wallet::asset_lock::AssetLockFunding;
use crate::wallet::identity::state::SlotHolder;
use crate::wallet::platform_wallet::WalletId;
use crate::BlockTime;

use super::*;

/// Every key of an identity this wallet registered (or claimed) at
/// `identity_index`, each with its derivation breadcrumb: the host derived
/// them there, key id = key index.
pub(super) fn derived_keys(
    identity: &Identity,
    wallet_id: WalletId,
    identity_index: u32,
) -> Vec<KeyWithBreadcrumb> {
    identity
        .public_keys()
        .iter()
        .map(|(key_id, key)| KeyWithBreadcrumb {
            key: key.clone(),
            breadcrumb: Some((wallet_id, identity_index, *key_id)),
        })
        .collect()
}

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
    #[allow(clippy::too_many_arguments)]
    pub async fn register_identity_with_funding<S, AS>(
        &self,
        funding: AssetLockFunding,
        identity_index: u32,
        holder: Option<SlotHolder>,
        keys_map: BTreeMap<u32, IdentityPublicKey>,
        identity_signer: &S,
        asset_lock_signer: &AS,
        settings: Option<PutSettings>,
    ) -> Result<Identity, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
        AS: ::key_wallet::signer::ExtendedPubKeySigner + Send + Sync,
    {
        // A new funding runs on its registration's lease, one operation at
        // a time; a resumed lock holds its index through the lock.
        let resume = matches!(
            funding,
            AssetLockFunding::FromExistingAssetLock {
                consume_invitation_voucher: false,
                ..
            }
        );
        let in_flight = match holder {
            Some(holder @ SlotHolder::Registration { .. }) if !resume => {
                self.begin_registration(identity_index, &holder).await?;
                Some(holder)
            }
            _ => None,
        };
        let result = self
            .register_identity_with_funding_leased(
                funding,
                identity_index,
                holder,
                keys_map,
                identity_signer,
                asset_lock_signer,
                settings,
            )
            .await;
        if let Some(holder) = in_flight {
            self.end_registration(identity_index, &holder, &result)
                .await;
        }
        result
    }

    /// [`Self::register_identity_with_funding`] once a new funding's lease
    /// is taken for it.
    #[allow(clippy::too_many_arguments)]
    async fn register_identity_with_funding_leased<S, AS>(
        &self,
        funding: AssetLockFunding,
        identity_index: u32,
        holder: Option<SlotHolder>,
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

        // Step 1b: the slot. A new funding, and an invitation voucher this
        // wallet reclaims as a new identity, need the registration's lease on
        // `identity_index` (from `acquire_identity_slot`); a tracked
        // registration lock being resumed already holds its index, which the
        // resolver checks against `identity_index`.
        if let AssetLockFunding::FromExistingAssetLock {
            out_point,
            consume_invitation_voucher: false,
        } = funding
        {
            // A resumed registration lock: before anything is resolved or
            // submitted (a refusal for keys already registered costs a
            // penalty from the lock each time), see whether the keys derived
            // at its index are an identity's already: this lock's own (an
            // earlier submit landed) or another of this wallet's. Only a
            // tracked registration lock of this index may settle the slot.
            self.asset_locks
                .authorize_registration_lock(out_point, identity_index)
                .await?;
            if let Some(settled) = self
                .settle_registration_lock(identity_index, &keys_map, out_point)
                .await?
            {
                return settled;
            }
        } else {
            match holder {
                Some(holder @ SlotHolder::Registration { .. }) => {
                    self.require_slot_lease(identity_index, &holder).await?;
                    // An identity already registered with these keys would
                    // make Platform refuse this one after its funding is
                    // spent: record it in the slot and send the host to
                    // another index instead.
                    if let Some(taken) = self
                        .slot_taken_on_chain(identity_index, &keys_map, None)
                        .await?
                    {
                        return Err(taken);
                    }
                }
                Some(SlotHolder::InvitationClaim { .. }) | None => {
                    return Err(PlatformWalletError::IdentitySlotNotHeld { identity_index });
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
        let identity = match self
            .asset_locks
            .reconcile_asset_lock_submit_result(
                submit_result,
                &proof_out_point,
                &effective_proof,
                None,
            )
            .await
        {
            Ok(identity) => identity,
            // Platform answered that this lock is spent, that its identity
            // exists, or that a key is registered already: what holds the
            // keys derived at this index says which. Anything that does not
            // settle it is reported as Platform's refusal.
            Err(refusal) if settles_by_the_slot(&refusal, &proof_out_point) => {
                let settled = self
                    .settle_registration_lock(
                        identity_index,
                        placeholder.public_keys(),
                        proof_out_point,
                    )
                    .await;
                return settled_after_funding(settled, refusal);
            }
            Err(refusal) => return Err(refusal),
        };

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
        // The identity, its keys with their breadcrumbs, in one durable
        // write, into this wallet generation only: a wallet replaced during
        // the submit keeps its own slots and leases.
        if let Err(e) = self
            .record_registered_identity(identity.clone(), identity_index)
            .await
        {
            tracing::warn!(
                error = %e,
                identity_id = %identity.id(),
                "register_identity_with_funding: identity registered on \
                 Platform but not recorded in its slot; continuing so \
                 the spent asset lock is still consumed"
            );
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

impl IdentityWallet {
    /// What holds the identity keys derived at `identity_index` (`keys_map`,
    /// derived there), for the registration lock at `out_point`:
    /// - nothing: `None`;
    /// - the identity this lock created (its id is the one Platform derives
    ///   from `out_point`): an earlier submit landed. It is recorded in the
    ///   slot, the lock is marked consumed, and it is returned as the
    ///   registration's result;
    /// - another identity, this wallet's at that index: it is recorded in the
    ///   slot and [`PlatformWalletError::IdentitySlotTakenOnChain`] returned.
    ///   The lock stays, for a top-up of that identity or another use.
    ///
    /// A lookup that fails is `Err`.
    async fn settle_registration_lock(
        &self,
        identity_index: u32,
        keys_map: &BTreeMap<u32, IdentityPublicKey>,
        out_point: OutPoint,
    ) -> Result<Option<Result<Identity, PlatformWalletError>>, PlatformWalletError> {
        let Some(taken) = self
            .slot_taken_on_chain(identity_index, keys_map, None)
            .await?
        else {
            return Ok(None);
        };
        let PlatformWalletError::IdentitySlotTakenOnChain { identity_id, .. } = &taken else {
            return Ok(Some(Err(taken)));
        };
        if *identity_id != identity_id_for_out_point(out_point) {
            return Ok(Some(Err(taken)));
        }
        if let Err(e) = self.asset_locks.consume_asset_lock(&out_point).await {
            tracing::warn!(
                outpoint = %out_point,
                error = %e,
                "a registration lock's identity was found registered, but the lock could \
                 not be marked consumed"
            );
        }
        let identity = {
            let wm = self.wallet_manager.read().await;
            self.current_wallet_info(&wm)?
                .identity_manager
                .identity(identity_id)
                .map(|managed| managed.identity.clone())
        };
        Ok(Some(identity.ok_or(PlatformWalletError::IdentityNotFound(
            *identity_id,
        ))))
    }
}

/// Whether Platform's `refusal` of a registration with the lock at
/// `out_point` is one the keys at the registration's index settle: the lock
/// already spent, the identity it creates already existing, or a key
/// already registered (which also charged the lock a penalty). Platform
/// reports a registered key as `DuplicatedIdentityPublicKeyIdStateError`
/// (40207), or `DuplicatedIdentityPublicKeyIdBasicError` (10501) on
/// protocol versions before 7.
fn settles_by_the_slot(refusal: &PlatformWalletError, out_point: &OutPoint) -> bool {
    match refusal {
        PlatformWalletError::AssetLockAlreadyConsumed(consumed) => consumed == out_point,
        PlatformWalletError::Sdk(error) => matches!(
            error.consensus_error(),
            Some(
                ConsensusError::StateError(
                    StateError::IdentityAlreadyExistsError(_)
                        | StateError::DuplicatedIdentityPublicKeyStateError(_)
                        | StateError::DuplicatedIdentityPublicKeyIdStateError(_)
                ) | ConsensusError::BasicError(
                    BasicError::DuplicatedIdentityPublicKeyIdBasicError(_)
                )
            )
        ),
        _ => false,
    }
}

/// The outcome of a funded registration that Platform refused with
/// `refusal`, given what the keys at its index were found to hold
/// (`settled`): the lock's own identity is the registration's result; any
/// other outcome is Platform's refusal. Never
/// [`PlatformWalletError::IdentitySlotTakenOnChain`]: after funding, that
/// pre-funding answer would let a host fund the next slot again while this
/// lock stays tracked for a resume.
fn settled_after_funding(
    settled: Result<Option<Result<Identity, PlatformWalletError>>, PlatformWalletError>,
    refusal: PlatformWalletError,
) -> Result<Identity, PlatformWalletError> {
    match settled {
        Ok(Some(Ok(identity))) => Ok(identity),
        Ok(Some(Err(_))) | Ok(None) | Err(_) => Err(refusal),
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
                None,
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
        let (mut new_balance, metadata) = self
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
    use dpp::consensus::basic::identity::DuplicatedIdentityPublicKeyIdBasicError;
    use dpp::consensus::state::identity::duplicated_identity_public_key_id_state_error::DuplicatedIdentityPublicKeyIdStateError;
    use dpp::consensus::state::identity::duplicated_identity_public_key_state_error::DuplicatedIdentityPublicKeyStateError;
    use dpp::consensus::state::identity::IdentityAlreadyExistsError;
    use dpp::consensus::state::identity::IdentityInsufficientBalanceError;
    use dpp::ProtocolError;

    /// After funding, only the lock's own identity settles a refusal: an
    /// identity found from another outpoint, nothing found, or a failed
    /// lookup each leave Platform's refusal, never the pre-funding
    /// slot-taken code a host would fund the next slot on.
    #[test]
    fn should_return_the_refusal_not_slot_taken_after_funding() {
        let refusal = || PlatformWalletError::Sdk(dash_sdk::Error::Generic("refused".to_string()));
        let identity = Identity::V0(IdentityV0 {
            id: Identifier::from([4; 32]),
            public_keys: BTreeMap::new(),
            balance: 0,
            revision: 0,
        });
        assert_eq!(
            settled_after_funding(Ok(Some(Ok(identity.clone()))), refusal())
                .expect("the lock's own identity")
                .id(),
            identity.id()
        );
        for settled in [
            Ok(Some(Err(PlatformWalletError::IdentitySlotTakenOnChain {
                identity_index: 0,
                identity_id: Identifier::from([5; 32]),
            }))),
            Ok(None),
            Err(PlatformWalletError::Sdk(dash_sdk::Error::Generic(
                "lookup".to_string(),
            ))),
        ] {
            let outcome = settled_after_funding(settled, refusal());
            assert!(
                matches!(outcome, Err(PlatformWalletError::Sdk(_))),
                "{outcome:?}"
            );
        }
    }

    /// Only the refusals what holds the slot's keys can settle go to it.
    #[test]
    fn should_settle_by_the_slot_only_a_spent_lock_an_existing_identity_or_a_taken_key() {
        let ours = OutPoint::new(Txid::from_byte_array([1; 32]), 0);
        let other = OutPoint::new(Txid::from_byte_array([2; 32]), 0);
        let consensus = |error: ConsensusError| {
            PlatformWalletError::Sdk(dash_sdk::Error::Protocol(ProtocolError::ConsensusError(
                Box::new(error),
            )))
        };
        for (refusal, settles) in [
            (PlatformWalletError::AssetLockAlreadyConsumed(ours), true),
            (PlatformWalletError::AssetLockAlreadyConsumed(other), false),
            (
                consensus(
                    StateError::IdentityAlreadyExistsError(IdentityAlreadyExistsError::new(
                        Identifier::from([3; 32]),
                    ))
                    .into(),
                ),
                true,
            ),
            (
                consensus(
                    StateError::DuplicatedIdentityPublicKeyStateError(
                        DuplicatedIdentityPublicKeyStateError::new(vec![0]),
                    )
                    .into(),
                ),
                true,
            ),
            (
                consensus(
                    StateError::DuplicatedIdentityPublicKeyIdStateError(
                        DuplicatedIdentityPublicKeyIdStateError::new(vec![0]),
                    )
                    .into(),
                ),
                true,
            ),
            (
                consensus(DuplicatedIdentityPublicKeyIdBasicError::new(vec![0]).into()),
                true,
            ),
            (
                consensus(
                    StateError::IdentityInsufficientBalanceError(
                        IdentityInsufficientBalanceError::new(Identifier::from([3; 32]), 0, 1),
                    )
                    .into(),
                ),
                false,
            ),
            (
                PlatformWalletError::Sdk(dash_sdk::Error::Generic("node".to_string())),
                false,
            ),
        ] {
            assert_eq!(settles_by_the_slot(&refusal, &ours), settles, "{refusal}");
        }
    }
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
