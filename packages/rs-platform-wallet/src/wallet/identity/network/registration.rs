//! Identity registration and top-up flows.
//!
//! ## Two-layer factoring
//!
//! Both registration and top-up are factored as a thin **L1 primitive**
//! wrapping the SDK's `_with_signer` calls, and a **L2 orchestration
//! method** that does pre-flight, funding resolution, IS→CL fallback,
//! and IdentityManager bookkeeping.
//!
//! | Layer | Registration                       | Top-up                            |
//! |-------|------------------------------------|-----------------------------------|
//! | L1    | [`register_identity_with_signer`]  | [`top_up_identity_with_signer`]   |
//! | L2    | [`register_identity_with_funding`] | [`top_up_identity_with_funding`]  |
//!
//! [`register_identity_with_signer`]: IdentityWallet::register_identity_with_signer
//! [`top_up_identity_with_signer`]: IdentityWallet::top_up_identity_with_signer
//! [`register_identity_with_funding`]: IdentityWallet::register_identity_with_funding
//! [`top_up_identity_with_funding`]: IdentityWallet::top_up_identity_with_funding
//!
//! The L2 methods are the canonical entry points. The L1 primitives are
//! `pub` so callers that manage funding outside this crate (evo-tool's
//! tasks, integration tests) can submit a pre-built proof directly.
//!
//! ## IS→CL fallback (the "stuck asset-lock" bug it fixes)
//!
//! L2 covers two distinct surfaces where an IS-lock can fail:
//!
//! 1. **Core-side timeout** — `create_funded_asset_lock_proof` returns
//!    `PlatformWalletError::FinalityTimeout` because the IS-lock
//!    didn't propagate within 300s. Detected via
//!    [`crate::error::is_instant_lock_timeout`]. L2 calls
//!    `upgrade_to_chain_lock_proof` to wait for a ChainLock, then
//!    re-enters submission with the CL proof.
//!
//! 2. **Platform-side rejection** — `put_to_platform_and_wait_for_response_with_signer`
//!    returns `InvalidInstantAssetLockProofSignatureError` (the
//!    consensus error Drive emits when the IS-lock signing quorum has
//!    rotated out). Detected via
//!    [`crate::error::is_instant_lock_proof_invalid`]. Same recovery:
//!    upgrade to ChainLock and retry.
//!
//! Both paths share the same outpoint-keyed cleanup (`consume_asset_lock`)
//! once Platform finally accepts the registration / top-up.

use std::collections::BTreeMap;
use std::time::Duration;

use dashcore::OutPoint;
use dpp::identity::accessors::IdentitySettersV0;
use dpp::identity::signer::Signer;
use dpp::identity::v0::IdentityV0;
use dpp::identity::Identity;
use dpp::identity::IdentityPublicKey;
use dpp::identity::KeyID;
use dpp::identity::Purpose;
use dpp::identity::SecurityLevel;
use dpp::prelude::AssetLockProof;
use dpp::prelude::Identifier;
use key_wallet::bip32::DerivationPath;
use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;

use dash_sdk::platform::transition::put_identity::PutIdentity;
use dash_sdk::platform::transition::put_settings::PutSettings;
use dash_sdk::platform::transition::top_up_identity::TopUpIdentity;

use crate::error::{is_instant_lock_proof_invalid, is_instant_lock_timeout, PlatformWalletError};
use crate::wallet::identity::types::funding::IdentityFunding;

use super::*;

// ---------------------------------------------------------------------------
// Timeout policy
// ---------------------------------------------------------------------------

/// Time we will wait for a ChainLock to materialise after an IS-lock
/// fallback is triggered. 180s mirrors the existing fallback shape and
/// is roughly the worst-case ChainLock latency we've observed in
/// testnet operation. Promoted to a constant so the registration and
/// top-up flows can't drift apart on this number.
const CL_FALLBACK_TIMEOUT: Duration = Duration::from_secs(180);

// ---------------------------------------------------------------------------
// Funding resolution (shared between register and top-up)
// ---------------------------------------------------------------------------

/// Outcome of resolving an [`IdentityFunding`] to a concrete asset-lock
/// proof + derivation path.
///
/// `tracked_out_point` is `Some` whenever this wallet's
/// `AssetLockManager` owns the lifecycle of the underlying asset lock
/// — i.e. for `FromWalletBalance` (we just built and tracked it) and
/// `FromExistingAssetLock` (caller is resuming a tracked entry). It's
/// `None` for `UseAssetLock` where the caller has externally-managed
/// proofs and we shouldn't touch the tracked-asset-lock map. The
/// outpoint also drives both IS→CL fallback (look up the lock by
/// outpoint) and cleanup (remove the lock on Platform success).
struct ResolvedFunding {
    proof: AssetLockProof,
    path: DerivationPath,
    tracked_out_point: Option<OutPoint>,
}

/// Outcome of [`IdentityWallet::resolve_funding_with_is_timeout_fallback`]:
/// either a fully-resolved funding triple, or an IS-timeout that the
/// caller can convert to a ChainLock retry using the recovered
/// outpoint.
enum FundingResolution {
    Resolved(ResolvedFunding),
    /// IS-lock didn't propagate within the asset-lock manager's wait
    /// window. The outpoint of the tracked-but-unproven lock is
    /// surfaced so the caller can drive an `upgrade_to_chain_lock_proof`
    /// retry without re-walking the tracked-asset-lock map.
    IsTimeout {
        out_point: OutPoint,
    },
}

impl IdentityWallet {
    /// Resolve an [`IdentityFunding`] to a concrete proof + path +
    /// (optional) tracked outpoint, capturing the IS-lock timeout case
    /// as a structured outcome so the caller can drive a CL retry.
    ///
    /// `funding_type` selects the BIP44 account the
    /// `FromWalletBalance` variant pulls UTXOs from
    /// (`IdentityRegistration` for register, `IdentityTopUp` for top
    /// up). The other variants ignore it — they don't build new
    /// asset locks.
    ///
    /// # IS-lock timeout handling
    ///
    /// For the two variants that internally invoke `wait_for_proof`
    /// (`FromWalletBalance` and `FromExistingAssetLock`), an IS-lock
    /// that never propagates within the 300s window surfaces as
    /// `PlatformWalletError::FinalityTimeout`. We catch that here and
    /// return `FundingResolution::IsTimeout` with the outpoint of the
    /// tracked-but-unproven asset lock — for `FromExistingAssetLock`
    /// we already know it (the variant carries it directly), for
    /// `FromWalletBalance` we recover it via
    /// [`find_tracked_unproven_lock`](Self::find_tracked_unproven_lock).
    async fn resolve_funding_with_is_timeout_fallback<AS>(
        &self,
        funding: IdentityFunding,
        funding_type: AssetLockFundingType,
        identity_index: u32,
        asset_lock_signer: &AS,
    ) -> Result<FundingResolution, PlatformWalletError>
    where
        AS: ::key_wallet::signer::Signer + Send + Sync,
    {
        match funding {
            IdentityFunding::FromWalletBalance { amount_duffs } => {
                match self
                    .asset_locks
                    .create_funded_asset_lock_proof(
                        amount_duffs,
                        0,
                        funding_type,
                        identity_index,
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
                    Err(e) if is_instant_lock_timeout(&e) => {
                        // We don't have the outpoint directly because
                        // create_funded_asset_lock_proof consumes the
                        // result. The asset-lock manager tracked the
                        // lock before broadcast — find it back via
                        // (funding_type, identity_index).
                        let out_point = self
                            .find_tracked_unproven_lock(funding_type, identity_index)
                            .await?;
                        Ok(FundingResolution::IsTimeout { out_point })
                    }
                    Err(e) => Err(e),
                }
            }
            IdentityFunding::FromExistingAssetLock { out_point } => {
                match self
                    .asset_locks
                    .resume_asset_lock(&out_point, Duration::from_secs(300))
                    .await
                {
                    Ok((proof, path)) => Ok(FundingResolution::Resolved(ResolvedFunding {
                        proof,
                        path,
                        tracked_out_point: Some(out_point),
                    })),
                    Err(e) if is_instant_lock_timeout(&e) => {
                        // We already know the outpoint from the
                        // variant — no need to walk the tracked map.
                        Ok(FundingResolution::IsTimeout { out_point })
                    }
                    Err(e) => Err(e),
                }
            }
            IdentityFunding::UseAssetLock {
                proof,
                derivation_path,
            } => Ok(FundingResolution::Resolved(ResolvedFunding {
                proof,
                path: derivation_path,
                // Caller owns the lock lifecycle — don't touch the
                // tracked-asset-lock map.
                tracked_out_point: None,
            })),
        }
    }
}

// ---------------------------------------------------------------------------
// L1 primitives — pure submit, no funding/bookkeeping/fallback
// ---------------------------------------------------------------------------

impl IdentityWallet {
    /// **L1 primitive**: submit an identity-create state transition using a
    /// caller-supplied asset-lock proof + derivation path + signer pair.
    ///
    /// Builds a placeholder `Identity` from `keys_map` internally
    /// (caller doesn't need to materialise the DPP type). The first key
    /// (id=0) MUST be a MASTER + AUTHENTICATION key — this is enforced
    /// here defensively so a malformed map fails fast.
    ///
    /// No funding resolution, no bookkeeping, no fallback. The L2
    /// orchestrator [`register_identity_with_funding`](Self::register_identity_with_funding)
    /// owns those concerns and calls this primitive twice (once with
    /// the IS proof, once with the CL proof on IS→CL fallback) so the
    /// retry is byte-identical to the first attempt.
    ///
    /// # Send + Sync bounds
    ///
    /// Both `S` and `AS` carry `Send + Sync` bounds even though this
    /// method's body doesn't `tokio::spawn`. The bounds match the L2
    /// orchestrator's so callers don't have to think about which layer
    /// imposes which constraint. This unblocks future `tokio::spawn`-
    /// driven refactors at the L2 site without a backwards-incompatible
    /// trait-bound change here.
    pub async fn register_identity_with_signer<S, AS>(
        &self,
        keys_map: BTreeMap<u32, IdentityPublicKey>,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_path: &DerivationPath,
        asset_lock_signer: &AS,
        identity_signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<Identity, dash_sdk::Error>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
        AS: ::key_wallet::signer::Signer + Send + Sync,
    {
        let identity = Identity::V0(IdentityV0 {
            id: Identifier::default(),
            public_keys: keys_map,
            balance: 0,
            revision: 0,
        });

        identity
            .put_to_platform_and_wait_for_response_with_signer(
                &self.sdk,
                asset_lock_proof,
                asset_lock_proof_path,
                asset_lock_signer,
                identity_signer,
                settings,
            )
            .await
    }

    /// **L1 primitive**: submit an identity-top-up state transition
    /// using a caller-supplied identity + asset-lock proof + derivation
    /// path + signer.
    ///
    /// No funding resolution, no bookkeeping, no fallback. The L2
    /// orchestrator [`top_up_identity_with_funding`](Self::top_up_identity_with_funding)
    /// owns those concerns.
    ///
    /// Returns the post-transition credit balance.
    ///
    /// `Send + Sync` rationale: same as
    /// [`register_identity_with_signer`](Self::register_identity_with_signer).
    pub async fn top_up_identity_with_signer<AS>(
        &self,
        identity: &Identity,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_path: &DerivationPath,
        asset_lock_signer: &AS,
        settings: Option<PutSettings>,
    ) -> Result<u64, dash_sdk::Error>
    where
        AS: ::key_wallet::signer::Signer + Send + Sync,
    {
        identity
            .top_up_identity_with_signer(
                &self.sdk,
                asset_lock_proof,
                asset_lock_proof_path,
                asset_lock_signer,
                settings.and_then(|s| s.user_fee_increase),
                settings,
            )
            .await
    }
}

// ---------------------------------------------------------------------------
// L2 orchestrator — register
// ---------------------------------------------------------------------------

impl IdentityWallet {
    /// **L2 orchestrator**: register a new asset-lock-funded identity
    /// on Platform.
    ///
    /// Single entry point for every register-with-asset-lock case:
    ///
    /// 1. Pre-flight — validate `keys_map[0]` is a MASTER +
    ///    AUTHENTICATION key (the IdentityCreate transition itself
    ///    must be signed by a MASTER-level identity key, and we pin
    ///    that role on id=0 by convention).
    /// 2. Resolve the [`IdentityFunding`] to an asset-lock proof +
    ///    derivation path.
    /// 3. Submit via the [L1 primitive](Self::register_identity_with_signer),
    ///    with IS→CL fallback on **both** Core-side timeout
    ///    (`FinalityTimeout`) and Platform-side rejection
    ///    (`InvalidInstantAssetLockProofSignatureError`).
    /// 4. On success, add the confirmed identity to the local
    ///    `IdentityManager` and record each key's derivation breadcrumb.
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
        funding: IdentityFunding,
        identity_index: u32,
        keys_map: BTreeMap<u32, IdentityPublicKey>,
        identity_signer: &S,
        asset_lock_signer: &AS,
        settings: Option<PutSettings>,
    ) -> Result<Identity, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
        AS: ::key_wallet::signer::Signer + Send + Sync,
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
                        "keys_map must include key id=0 with MASTER security level"
                            .to_string(),
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
                    .upgrade_to_chain_lock_proof(&out_point, CL_FALLBACK_TIMEOUT)
                    .await?;
                // Recover the credit-output derivation path. The
                // asset lock is now CL-attached (status advanced by
                // `upgrade_to_chain_lock_proof`'s caller path), so
                // `resume_asset_lock` short-circuits to the existing-
                // proof branch and just re-derives the path. This is
                // cheap (no SPV wait) and avoids duplicating the
                // path-derivation logic here.
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

        // Build the placeholder identity ONCE so both the primary
        // attempt and the IS→CL retry submit the same key set. This
        // bypasses the L1 primitive — which takes `keys_map` by value
        // — so the retry doesn't need a deep clone of the BTreeMap.
        // The L1 helper exists for *single-shot* callers; L2 owns the
        // fallback shape and inlines the SDK call to avoid the by-value
        // ergonomics issue.
        let placeholder = Identity::V0(IdentityV0 {
            id: Identifier::default(),
            public_keys: keys_map,
            balance: 0,
            revision: 0,
        });

        // Step 3: submit, with Platform-side IS→CL fallback on
        // InstantSend rejection. The retry path uses the SAME
        // credit-output outpoint — no new asset lock built, no new
        // funding-tx broadcast.
        let proof_out_point = Self::out_point_from_proof(&proof);
        let identity = match placeholder
            .put_to_platform_and_wait_for_response_with_signer(
                &self.sdk,
                proof,
                &path,
                asset_lock_signer,
                identity_signer,
                settings,
            )
            .await
        {
            Ok(identity) => identity,
            Err(e) if is_instant_lock_proof_invalid(&e) => {
                let out_point = proof_out_point.ok_or_else(|| {
                    PlatformWalletError::InvalidIdentityData(format!(
                        "IS-lock rejected by Platform but proof carried no \
                         outpoint we could upgrade: {}",
                        e
                    ))
                })?;
                tracing::warn!(
                    "IS-lock proof rejected by Platform for identity registration (tx {}), \
                     retrying with ChainLock proof",
                    out_point.txid
                );
                let chain_proof = self
                    .asset_locks
                    .upgrade_to_chain_lock_proof(&out_point, CL_FALLBACK_TIMEOUT)
                    .await?;
                placeholder
                    .put_to_platform_and_wait_for_response_with_signer(
                        &self.sdk,
                        chain_proof,
                        &path,
                        asset_lock_signer,
                        identity_signer,
                        settings,
                    )
                    .await
                    .map_err(|e| {
                        PlatformWalletError::InvalidIdentityData(format!(
                            "Failed to register identity on Platform (ChainLock retry): {}",
                            e
                        ))
                    })?
            }
            Err(e) => {
                return Err(PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to register identity on Platform: {}",
                    e
                )));
            }
        };

        // Step 4: bookkeeping — add to local IdentityManager + record
        // key derivation breadcrumbs.
        {
            use dpp::identity::accessors::IdentityGettersV0;

            let mut wm = self.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&self.wallet_id).ok_or_else(|| {
                PlatformWalletError::WalletNotFound(
                    "Wallet info not found in wallet manager".to_string(),
                )
            })?;
            info.identity_manager.add_identity(
                identity.clone(),
                identity_index,
                self.wallet_id,
                &self.persister,
            )?;

            let wallet_id = self.wallet_id;
            let identity_id = identity.id();
            let public_keys: Vec<(KeyID, IdentityPublicKey)> = identity
                .public_keys()
                .iter()
                .map(|(k, v)| (*k, v.clone()))
                .collect();

            if let Some(managed) = info.identity_manager.managed_identity_mut(&identity_id) {
                managed.wallet_id = Some(wallet_id);
                for (key_id, pub_key) in public_keys {
                    let key_index = key_id;
                    managed.add_key(
                        pub_key,
                        Some((wallet_id, identity_index, key_index)),
                        &self.persister,
                    );
                }
            }
        }

        // Step 5: clean up the tracked asset lock — Platform has
        // accepted the registration and the credit output is now
        // consumed. Only fires for the variants where we own the
        // lifecycle (`FromWalletBalance` / `FromExistingAssetLock`);
        // `UseAssetLock` is `None` and skipped.
        if let Some(out_point) = tracked_out_point {
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
// L2 orchestrator — top-up
// ---------------------------------------------------------------------------

impl IdentityWallet {
    /// **L2 orchestrator**: top up an existing identity's credit balance.
    ///
    /// Mirror of [`register_identity_with_funding`](Self::register_identity_with_funding)
    /// for top-ups:
    ///
    /// 1. Look up the identity by `identity_id` in the local
    ///    `IdentityManager`. Return `IdentityNotFound` if missing.
    /// 2. Resolve the [`IdentityFunding`] to an asset-lock proof.
    /// 3. Submit via the [L1 primitive](Self::top_up_identity_with_signer),
    ///    with IS→CL fallback on Core-side timeout and Platform-side
    ///    rejection (same as register).
    /// 4. Persist the new credit balance + remove the tracked asset
    ///    lock.
    pub async fn top_up_identity_with_funding<AS>(
        &self,
        identity_id: &Identifier,
        funding: IdentityFunding,
        asset_lock_signer: &AS,
        settings: Option<PutSettings>,
    ) -> Result<u64, PlatformWalletError>
    where
        AS: ::key_wallet::signer::Signer + Send + Sync,
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

        // Step 3: submit. Platform-side IS→CL fallback on rejection.
        let proof_out_point = Self::out_point_from_proof(&proof);
        let new_balance = match self
            .top_up_identity_with_signer(
                &identity,
                proof,
                &path,
                asset_lock_signer,
                settings,
            )
            .await
        {
            Ok(balance) => balance,
            Err(e) if is_instant_lock_proof_invalid(&e) => {
                let out_point = proof_out_point.ok_or_else(|| {
                    PlatformWalletError::InvalidIdentityData(format!(
                        "IS-lock rejected by Platform but proof carried no \
                         outpoint we could upgrade: {}",
                        e
                    ))
                })?;
                tracing::warn!(
                    "IS-lock proof rejected by Platform for identity top-up (tx {}), \
                     retrying with ChainLock proof",
                    out_point.txid
                );
                let chain_proof = self
                    .asset_locks
                    .upgrade_to_chain_lock_proof(&out_point, CL_FALLBACK_TIMEOUT)
                    .await?;
                self.top_up_identity_with_signer(
                    &identity,
                    chain_proof,
                    &path,
                    asset_lock_signer,
                    settings,
                )
                .await
                .map_err(PlatformWalletError::Sdk)?
            }
            Err(e) => return Err(PlatformWalletError::Sdk(e)),
        };

        // Step 4: persist the new balance + clean up the tracked lock.
        {
            let mut wm = self.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&self.wallet_id).ok_or_else(|| {
                PlatformWalletError::WalletNotFound(
                    "Wallet info not found in wallet manager".to_string(),
                )
            })?;
            if let Some(managed) = info.identity_manager.managed_identity_mut(identity_id) {
                managed.identity.set_balance(new_balance);
                if let Err(e) = self.persister.store(managed.snapshot_changeset().into()) {
                    tracing::error!(
                        identity = %identity_id,
                        error = %e,
                        "Failed to persist identity balance update after top_up"
                    );
                }
            }
        }
        if let Some(out_point) = tracked_out_point {
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

impl IdentityWallet {
    /// Look up the most-recently-tracked asset lock for
    /// `(funding_type, identity_index)` that has no attached proof
    /// (status `Built` or `Broadcast`).
    ///
    /// Used by the IS→CL Core-side timeout fallback path: when
    /// `wait_for_proof` times out, the asset-lock manager has already
    /// tracked the lock under its outpoint, but we lost the outpoint
    /// along with the result. This helper recovers it from the
    /// tracked-asset-lock map.
    ///
    /// Returns the outpoint of the matching lock, or an error if no
    /// matching lock is found (which would indicate a wallet-state
    /// mismatch — `wait_for_proof` shouldn't have returned a timeout
    /// without first tracking the lock).
    async fn find_tracked_unproven_lock(
        &self,
        funding_type: AssetLockFundingType,
        identity_index: u32,
    ) -> Result<OutPoint, PlatformWalletError> {
        use crate::wallet::asset_lock::tracked::AssetLockStatus;

        let wm = self.wallet_manager.read().await;
        let info = wm.get_wallet_info(&self.wallet_id).ok_or_else(|| {
            PlatformWalletError::WalletNotFound(
                "Wallet info not found in wallet manager".to_string(),
            )
        })?;
        info.tracked_asset_locks
            .iter()
            .find(|(_, lock)| {
                lock.funding_type == funding_type
                    && lock.identity_index == identity_index
                    && matches!(
                        lock.status,
                        AssetLockStatus::Built | AssetLockStatus::Broadcast
                    )
                    && lock.proof.is_none()
            })
            .map(|(out_point, _)| *out_point)
            .ok_or_else(|| {
                PlatformWalletError::AssetLockProofWait(format!(
                    "IS-lock timeout fallback: no tracked unproven asset lock found \
                     for funding_type={:?} identity_index={}",
                    funding_type, identity_index
                ))
            })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::is_instant_lock_timeout;
    use dashcore::Txid;

    /// Pins the IS-timeout discriminator: only `FinalityTimeout`
    /// matches, so the L2 fallback arms route exactly the cases we
    /// expect. Companion to `is_instant_lock_proof_invalid` (which
    /// discriminates SDK errors at the Platform-rejection boundary).
    #[test]
    fn is_instant_lock_timeout_only_matches_finality_timeout() {
        let timeout = PlatformWalletError::FinalityTimeout(Txid::from([0u8; 32]));
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
