//! Identity registration flows.

use std::collections::BTreeMap;
use std::time::Duration;

use dpp::identity::signer::Signer;
use dpp::identity::v0::IdentityV0;
use dpp::identity::Identity;
use dpp::identity::IdentityPublicKey;
use dpp::identity::KeyID;
use dpp::identity::Purpose;
use dpp::identity::SecurityLevel;
use dpp::prelude::AssetLockProof;
use dpp::prelude::Identifier;

use dash_sdk::platform::transition::put_identity::PutIdentity;
use dash_sdk::platform::transition::put_settings::PutSettings;
use dash_sdk::platform::transition::top_up_identity::TopUpIdentity;

use crate::error::PlatformWalletError;
// PrivateKeyData no longer needed at the registration call sites —
// `add_key` takes a flat `Option<(wallet_id, identity_index, key_index)>`
// breadcrumb directly.

use crate::wallet::identity::types::funding::IdentityFunding;

use super::*;
use crate::wallet::identity::types::funding::IdentityFundingMethod;

// ---------------------------------------------------------------------------
// Identity registration
// ---------------------------------------------------------------------------

impl IdentityWallet {
    /// Register a new asset-lock-funded identity on Platform using an
    /// externally-supplied signer + caller-derived authentication keys.
    ///
    /// The caller must provide:
    ///
    /// - `funding`: an `IdentityFundingMethod`.
    /// - `identity_index`: BIP-9 identity index.
    /// - `keys_map`: the auth pubkeys the new identity will be created
    ///   with. Caller must derive these from the wallet seed (or from
    ///   iOS Keychain via `dash_sdk_derive_identity_keys_from_mnemonic`)
    ///   and persist the matching private keys to whatever store the
    ///   `signer` reads from. The first key (id=0) MUST be a MASTER /
    ///   AUTHENTICATION key — DPP's IdentityCreate state transition
    ///   itself must be signed by a MASTER-level identity key, and we
    ///   pin that role on id=0 by convention so callers don't need
    ///   protocol knowledge to assemble the map. The asset-lock-spend
    ///   signature on the same transition is a separate signature
    ///   keyed off `asset_lock_private_key`, supplied via `funding`.
    /// - `signer`: external signer for the IdentityCreate transition's
    ///   per-key signatures.
    ///
    /// On success the new identity is added to the local manager and
    /// each key is recorded with its derivation breadcrumb for the
    /// persister callback. IS->CL fallback is retained.
    pub async fn register_identity_with_funding_external_signer<S>(
        &self,
        funding: IdentityFundingMethod,
        identity_index: u32,
        keys_map: BTreeMap<u32, IdentityPublicKey>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<Identity, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        if keys_map.is_empty() {
            return Err(PlatformWalletError::InvalidIdentityData(
                "keys_map must contain at least one identity public key".to_string(),
            ));
        }
        // Defensive: pin id=0 to MASTER+AUTHENTICATION at the FFI
        // boundary so a malformed map fails fast here instead of
        // surfacing as an opaque protocol-side rejection from
        // `put_to_platform_and_wait_for_response`.
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

        // Step 1: obtain asset lock proof + private key.
        let (asset_lock_proof, asset_lock_private_key) = match funding {
            IdentityFundingMethod::UseAssetLock { proof, private_key } => (proof, private_key),
            IdentityFundingMethod::FundWithWallet { amount_duffs } => {
                use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
                let (proof, key, _out_point) = self
                    .asset_locks
                    .create_funded_asset_lock_proof(
                        amount_duffs,
                        0,
                        AssetLockFundingType::IdentityRegistration,
                        identity_index,
                    )
                    .await?;
                (proof, key)
            }
        };

        // Step 2: build the placeholder identity from caller-supplied keys.
        let identity = Identity::V0(IdentityV0 {
            id: Identifier::default(),
            public_keys: keys_map,
            balance: 0,
            revision: 0,
        });

        // Step 3: submit, with IS->CL fallback on InstantSend rejection.
        let proof_out_point = Self::out_point_from_proof(&asset_lock_proof);

        let identity = match identity
            .put_to_platform_and_wait_for_response(
                &self.sdk,
                asset_lock_proof,
                &asset_lock_private_key,
                signer,
                settings,
            )
            .await
        {
            Ok(identity) => identity,
            Err(e) if crate::error::is_instant_lock_proof_invalid(&e) => {
                if let Some(out_point) = proof_out_point {
                    tracing::warn!(
                        "IS-lock proof rejected for identity registration (tx {}, external signer), \
                         retrying with ChainLock proof",
                        out_point.txid
                    );
                    let chain_proof = self
                        .asset_locks
                        .upgrade_to_chain_lock_proof(&out_point, Duration::from_secs(180))
                        .await?;
                    identity
                        .put_to_platform_and_wait_for_response(
                            &self.sdk,
                            chain_proof,
                            &asset_lock_private_key,
                            signer,
                            settings,
                        )
                        .await
                        .map_err(|e| {
                            PlatformWalletError::InvalidIdentityData(format!(
                                "Failed to register identity on Platform (ChainLock retry): {}",
                                e
                            ))
                        })?
                } else {
                    return Err(PlatformWalletError::InvalidIdentityData(format!(
                        "Failed to register identity on Platform: {}",
                        e
                    )));
                }
            }
            Err(e) => {
                return Err(PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to register identity on Platform: {}",
                    e
                )));
            }
        };

        // Step 4: add to local manager + record key derivation
        // breadcrumbs (mirrors the legacy variant exactly so the
        // persister callback fires the same way regardless of which
        // path produced the identity).
        {
            use dpp::identity::accessors::IdentityGettersV0;

            let mut wm = self.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&self.wallet_id).ok_or_else(|| {
                crate::error::PlatformWalletError::WalletNotFound(
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

        Ok(identity)
    }

    /// Register a new identity using an externally-provided identity, asset
    /// lock proof, and signer.
    ///
    /// Unlike
    /// [`register_identity_with_funding_external_signer`](Self::register_identity_with_funding_external_signer),
    /// this method does **not** derive keys or manage the internal
    /// `IdentityManager`. The caller supplies a fully-constructed `Identity`
    /// object, the asset lock proof + private key, and a `Signer`
    /// implementation directly.
    ///
    /// This is useful when the caller manages identities outside of the
    /// platform-wallet `IdentityManager` (e.g. evo-tool's
    /// `QualifiedIdentity`).
    ///
    /// Returns the confirmed `Identity` from Platform.
    pub async fn register_identity_with_signer<S: Signer<IdentityPublicKey>>(
        &self,
        identity: &Identity,
        asset_lock_proof: AssetLockProof,
        asset_lock_private_key: &dashcore::PrivateKey,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<Identity, dash_sdk::Error> {
        identity
            .put_to_platform_and_wait_for_response(
                &self.sdk,
                asset_lock_proof,
                asset_lock_private_key,
                signer,
                settings,
            )
            .await
    }

    /// Top up an identity's credit balance using an externally-provided
    /// identity and asset lock proof.
    ///
    /// Unlike [`top_up_identity_with_funding`](Self::top_up_identity_with_funding),
    /// this method does **not** look up the identity in the internal
    /// `IdentityManager`. The caller supplies the `Identity` object and the
    /// asset lock proof + private key directly.
    ///
    /// This is useful when the caller manages identities outside of the
    /// platform-wallet `IdentityManager` (e.g. evo-tool's
    /// `QualifiedIdentity`).
    ///
    /// Returns the new credit balance.
    pub async fn top_up_identity_with_signer(
        &self,
        identity: &Identity,
        asset_lock_proof: AssetLockProof,
        asset_lock_private_key: &dashcore::PrivateKey,
        settings: Option<PutSettings>,
    ) -> Result<u64, dash_sdk::Error> {
        identity
            .top_up_identity(
                &self.sdk,
                asset_lock_proof,
                asset_lock_private_key,
                settings.and_then(|s| s.user_fee_increase),
                settings,
            )
            .await
    }

    /// Register a new identity using an [`IdentityFunding`] variant and an
    /// externally-provided identity + signer.
    ///
    /// This method unifies funding resolution and Platform submission in a
    /// single call:
    ///
    /// * **`FromWalletBalance`** — builds an asset lock from wallet UTXOs via
    ///   [`AssetLockManager::create_funded_asset_lock_proof`], then submits the
    ///   identity registration to Platform.
    /// * **`FromExistingAssetLock`** — resumes a tracked asset lock by outpoint,
    ///   re-deriving the proof and private key from whatever stage the lock
    ///   is at.
    ///
    /// Unlike
    /// [`register_identity_with_funding_external_signer`](Self::register_identity_with_funding_external_signer),
    /// this method does **not** derive keys or manage the internal
    /// `IdentityManager`. The caller supplies a fully-constructed `Identity`
    /// and a `Signer` implementation, making it suitable for callers that
    /// manage identities externally (e.g. evo-tool's `QualifiedIdentity`).
    ///
    /// Returns the confirmed `Identity` from Platform.
    pub async fn funded_register_identity<S: Signer<IdentityPublicKey>>(
        &self,
        identity: &Identity,
        funding: IdentityFunding,
        identity_index: u32,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<Identity, PlatformWalletError> {
        use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;

        let (asset_lock_proof, asset_lock_private_key, tracked_out_point) = match funding {
            IdentityFunding::FromWalletBalance { amount_duffs } => {
                let (proof, key, out_point) = self
                    .asset_locks
                    .create_funded_asset_lock_proof(
                        amount_duffs,
                        0,
                        AssetLockFundingType::IdentityRegistration,
                        identity_index,
                    )
                    .await?;
                (proof, key, Some(out_point))
            }
            IdentityFunding::FromExistingAssetLock { out_point } => {
                let (proof, key) = self
                    .asset_locks
                    .resume_asset_lock(&out_point, Duration::from_secs(300))
                    .await?;
                (proof, key, Some(out_point))
            }
        };

        // Extract the outpoint before consuming the proof, in case we need to
        // build a ChainLock proof for recovery.
        let proof_out_point = Self::out_point_from_proof(&asset_lock_proof);

        let result = match self
            .register_identity_with_signer(
                identity,
                asset_lock_proof,
                &asset_lock_private_key,
                signer,
                settings,
            )
            .await
        {
            Ok(identity) => identity,
            Err(e) if crate::error::is_instant_lock_proof_invalid(&e) => {
                if let Some(out_point) = proof_out_point {
                    tracing::warn!(
                        "IS-lock proof rejected for funded identity registration (tx {}), \
                         retrying with ChainLock proof",
                        out_point.txid
                    );
                    let chain_proof = self
                        .asset_locks
                        .upgrade_to_chain_lock_proof(&out_point, Duration::from_secs(180))
                        .await?;
                    self.register_identity_with_signer(
                        identity,
                        chain_proof,
                        &asset_lock_private_key,
                        signer,
                        settings,
                    )
                    .await
                    .map_err(PlatformWalletError::Sdk)?
                } else {
                    return Err(PlatformWalletError::Sdk(e));
                }
            }
            Err(e) => return Err(PlatformWalletError::Sdk(e)),
        };

        // Clean up the tracked asset lock after successful consumption.
        if let Some(out_point) = tracked_out_point {
            self.asset_locks.remove_asset_lock(&out_point).await;
        }

        Ok(result)
    }

    /// Top up an identity using an [`IdentityFunding`] variant and an
    /// externally-provided identity.
    ///
    /// This method unifies funding resolution and Platform submission in a
    /// single call:
    ///
    /// * **`FromWalletBalance`** — builds an asset lock from wallet UTXOs via
    ///   [`AssetLockManager::create_funded_asset_lock_proof`], then submits the
    ///   top-up to Platform.
    /// * **`FromExistingAssetLock`** — resumes a tracked asset lock by outpoint,
    ///   re-deriving the proof and private key from whatever stage the lock
    ///   is at.
    ///
    /// Unlike [`top_up_identity_with_funding`](Self::top_up_identity_with_funding),
    /// this method does **not** look up the identity in the internal
    /// `IdentityManager`. The caller supplies the `Identity` object directly,
    /// making it suitable for callers that manage identities externally
    /// (e.g. evo-tool's `QualifiedIdentity`).
    ///
    /// Returns the new credit balance.
    pub async fn funded_top_up_identity(
        &self,
        identity: &Identity,
        funding: IdentityFunding,
        identity_index: u32,
        settings: Option<PutSettings>,
    ) -> Result<u64, PlatformWalletError> {
        use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;

        let (asset_lock_proof, asset_lock_private_key, tracked_out_point) = match funding {
            IdentityFunding::FromWalletBalance { amount_duffs } => {
                let (proof, key, out_point) = self
                    .asset_locks
                    .create_funded_asset_lock_proof(
                        amount_duffs,
                        0,
                        AssetLockFundingType::IdentityTopUp,
                        identity_index,
                    )
                    .await?;
                (proof, key, Some(out_point))
            }
            IdentityFunding::FromExistingAssetLock { out_point } => {
                let (proof, key) = self
                    .asset_locks
                    .resume_asset_lock(&out_point, Duration::from_secs(300))
                    .await?;
                (proof, key, Some(out_point))
            }
        };

        // Extract the outpoint before consuming the proof, in case we need to
        // build a ChainLock proof for recovery.
        let proof_out_point = Self::out_point_from_proof(&asset_lock_proof);

        let new_balance = match self
            .top_up_identity_with_signer(
                identity,
                asset_lock_proof,
                &asset_lock_private_key,
                settings,
            )
            .await
        {
            Ok(balance) => balance,
            Err(e) if crate::error::is_instant_lock_proof_invalid(&e) => {
                if let Some(out_point) = proof_out_point {
                    tracing::warn!(
                        "IS-lock proof rejected for funded identity top-up (tx {}), \
                         retrying with ChainLock proof",
                        out_point.txid
                    );
                    let chain_proof = self
                        .asset_locks
                        .upgrade_to_chain_lock_proof(&out_point, Duration::from_secs(180))
                        .await?;
                    self.top_up_identity_with_signer(
                        identity,
                        chain_proof,
                        &asset_lock_private_key,
                        settings,
                    )
                    .await
                    .map_err(PlatformWalletError::Sdk)?
                } else {
                    return Err(PlatformWalletError::Sdk(e));
                }
            }
            Err(e) => return Err(PlatformWalletError::Sdk(e)),
        };

        // Clean up the tracked asset lock after successful consumption.
        if let Some(out_point) = tracked_out_point {
            self.asset_locks.remove_asset_lock(&out_point).await;
        }

        Ok(new_balance)
    }
}
