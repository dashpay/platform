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

use crate::error::{
    as_asset_lock_proof_cl_height_too_low, is_instant_lock_proof_invalid, PlatformWalletError,
};
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

/// Delay between retries when Platform rejected with CL-height-too-low.
/// Each retry bumps `PutSettings::user_fee_increase` so the ST hash
/// changes (Tenderdash caches rejected ST hashes for ~24h on
/// mainnet/testnet — `keep-invalid-txs-in-cache = true` in dashmate's
/// tenderdash template, hardcoded at
/// `packages/dashmate/templates/platform/drive/tenderdash/config.toml.dot:355`).
const CL_HEIGHT_RETRY_DELAY: Duration = Duration::from_secs(15);

/// Total time we'll keep retrying before surfacing the error. Sized to
/// cover Platform's `create-empty-blocks-interval` (3m on mainnet)
/// plus a 30s safety margin: if Platform hasn't observed the wallet's
/// ChainLock by then, the lag is no longer routine and we need
/// operator visibility instead of further silent retries. The
/// rejection error carries Platform's `current_core_chain_locked_height`
/// each round so logs name the laggard node's reported tip explicitly.
const CL_HEIGHT_RETRY_BUDGET: Duration = Duration::from_secs(210);

/// Submit a state transition with automatic retry on
/// `InvalidAssetLockProofCoreChainHeightError` (consensus code 10506).
///
/// Each retry bumps `settings.user_fee_increase` so the resubmitted ST
/// hashes differently — Tenderdash caches rejected ST hashes for ~24h
/// on mainnet/testnet (`keep-invalid-txs-in-cache = true`), so an
/// identical-bytes resubmit would be silently dropped before reaching
/// Platform's CheckTx.
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
async fn submit_with_cl_height_retry<F, Fut, R>(
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
// Funding resolution (shared between register and top-up)
// ---------------------------------------------------------------------------

/// Outcome of resolving an [`IdentityFunding`] to a concrete asset-lock
/// proof + derivation path.
///
/// `tracked_out_point` is always `Some` — every `IdentityFunding`
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
    /// `PlatformWalletError::FinalityTimeout(out_point)`. The variant
    /// carries the *exact* outpoint that timed out (no
    /// `find_tracked_unproven_lock` BTreeMap walk needed), so the
    /// `IsTimeout` outcome is built directly from the error payload.
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
            IdentityFunding::FromWalletBalance {
                amount_duffs,
                account_index,
            } => {
                match self
                    .asset_locks
                    .create_funded_asset_lock_proof(
                        amount_duffs,
                        account_index,
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
    /// 2. Resolve the [`IdentityFunding`] to an asset-lock proof +
    ///    derivation path.
    /// 3. Submit via
    ///    `Identity::put_to_platform_and_wait_for_response_with_signer`
    ///    inside `submit_with_cl_height_retry`, with IS→CL fallback
    ///    on **both** Core-side timeout (`FinalityTimeout`) and
    ///    Platform-side rejection
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
        let proof_out_point = Self::out_point_from_proof(&proof);
        let identity = match submit_with_cl_height_retry(settings, |s| {
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
            Ok(identity) => identity,
            Err(e) if is_instant_lock_proof_invalid(&e) => {
                let out_point = proof_out_point;
                tracing::warn!(
                    "IS-lock proof rejected by Platform for identity registration (tx {}), \
                     retrying with ChainLock proof",
                    out_point.txid
                );
                let chain_proof = self
                    .asset_locks
                    .upgrade_to_chain_lock_proof(&out_point, CL_FALLBACK_TIMEOUT)
                    .await?;
                submit_with_cl_height_retry(settings, |s| {
                    placeholder.put_to_platform_and_wait_for_response_with_signer(
                        &self.sdk,
                        chain_proof.clone(),
                        &path,
                        asset_lock_signer,
                        identity_signer,
                        s,
                    )
                })
                .await
                .map_err(PlatformWalletError::Sdk)?
            }
            Err(e) => return Err(PlatformWalletError::Sdk(e)),
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
        // consumed. Both `IdentityFunding` variants produce a tracked
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
    /// 2. Resolve the [`IdentityFunding`] to an asset-lock proof.
    /// 3. Submit via `Identity::top_up_identity_with_signer` inside
    ///    `submit_with_cl_height_retry`, with IS→CL fallback on
    ///    Core-side timeout and Platform-side rejection (same as
    ///    register).
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

        // Step 3: submit. Two Platform-side fallback layers (matches
        // `register_identity_with_funding`): CL-height-too-low retries
        // bump `user_fee_increase` to bypass Tenderdash's invalid-tx
        // cache, and IS-lock rejection triggers an IS→CL upgrade on the
        // same outpoint.
        let proof_out_point = Self::out_point_from_proof(&proof);
        let new_balance = match submit_with_cl_height_retry(settings, |s| {
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
            Ok(balance) => balance,
            Err(e) if is_instant_lock_proof_invalid(&e) => {
                let out_point = proof_out_point;
                tracing::warn!(
                    "IS-lock proof rejected by Platform for identity top-up (tx {}), \
                     retrying with ChainLock proof",
                    out_point.txid
                );
                let chain_proof = self
                    .asset_locks
                    .upgrade_to_chain_lock_proof(&out_point, CL_FALLBACK_TIMEOUT)
                    .await?;
                submit_with_cl_height_retry(settings, |s| {
                    identity.top_up_identity_with_signer(
                        &self.sdk,
                        chain_proof.clone(),
                        &path,
                        asset_lock_signer,
                        s,
                    )
                })
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

        let consensus = ConsensusError::BasicError(
            BasicError::InvalidAssetLockProofCoreChainHeightError(
                InvalidAssetLockProofCoreChainHeightError::new(
                    /* proof_core_chain_locked_height */ 100,
                    /* current_core_chain_locked_height */ 99,
                ),
            ),
        );
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
    ///    This invariant regressed silently once in this PR series — the
    ///    test exists so it can't regress quietly again.
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
        assert!(result.is_err(), "retry must surface the underlying error on budget exhaust");
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
        let max_expected = (CL_HEIGHT_RETRY_BUDGET.as_secs() / CL_HEIGHT_RETRY_DELAY.as_secs()) as u32 + 2;
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
            captured[0],
            None,
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
