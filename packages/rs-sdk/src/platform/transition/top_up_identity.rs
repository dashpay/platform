use super::broadcast::BroadcastStateTransition;
use super::put_settings::PutSettings;
use super::validation::ensure_valid_state_transition_structure;
use super::waitable::Waitable;
use crate::{Error, Sdk};
use dpp::dashcore::PrivateKey;
use dpp::identity::{Identity, PartialIdentity};
use dpp::prelude::AssetLockProof;
use dpp::state_transition::identity_topup_transition::methods::IdentityTopUpTransitionMethodsV0;
use dpp::state_transition::identity_topup_transition::IdentityTopUpTransition;

#[async_trait::async_trait]
pub trait TopUpIdentity: Waitable {
    /// Tops up an existing identity using an asset lock proof whose
    /// private key is held in-process.
    ///
    /// Prefer [`Self::top_up_identity_with_signer`] when the asset-lock
    /// private key lives outside Rust (Swift / hardware wallet / HSM):
    /// the `_with_signer` variant routes asset-lock signing through an
    /// external [`key_wallet::signer::Signer`] so the private key never
    /// crosses the FFI boundary as raw bytes.
    async fn top_up_identity_with_private_key(
        &self,
        sdk: &Sdk,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &PrivateKey,
        settings: Option<PutSettings>,
    ) -> Result<u64, Error>;

    /// Tops up an existing identity using an asset-lock signer.
    ///
    /// Signer-driven counterpart to
    /// [`Self::top_up_identity_with_private_key`]. `asset_lock_signer`
    /// produces the outer state-transition ECDSA signature for the key
    /// at `asset_lock_proof_path` — atomically deriving, signing, and
    /// zeroising inside the signer's trust boundary.
    #[cfg(feature = "core_key_wallet")]
    async fn top_up_identity_with_signer<AS>(
        &self,
        sdk: &Sdk,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_path: &dpp::key_wallet::bip32::DerivationPath,
        asset_lock_signer: &AS,
        settings: Option<PutSettings>,
    ) -> Result<u64, Error>
    where
        AS: dpp::key_wallet::signer::Signer + Send + Sync;
}

#[async_trait::async_trait]
impl TopUpIdentity for Identity {
    async fn top_up_identity_with_private_key(
        &self,
        sdk: &Sdk,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &PrivateKey,
        settings: Option<PutSettings>,
    ) -> Result<u64, Error> {
        let user_fee_increase = settings
            .and_then(|s| s.user_fee_increase)
            .unwrap_or_default();
        let state_transition = IdentityTopUpTransition::try_from_identity_with_private_key(
            self,
            asset_lock_proof,
            asset_lock_proof_private_key.inner.as_ref(),
            user_fee_increase,
            sdk.version(),
            None,
        )?;
        ensure_valid_state_transition_structure(&state_transition, sdk.version())?;
        let identity: PartialIdentity = state_transition
            .broadcast_and_wait_for_affected_state(sdk, settings)
            .await?;

        identity
            .balance
            .ok_or(Error::Generic("expected an identity balance".to_string()))
    }

    #[cfg(feature = "core_key_wallet")]
    async fn top_up_identity_with_signer<AS>(
        &self,
        sdk: &Sdk,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_path: &dpp::key_wallet::bip32::DerivationPath,
        asset_lock_signer: &AS,
        settings: Option<PutSettings>,
    ) -> Result<u64, Error>
    where
        AS: dpp::key_wallet::signer::Signer + Send + Sync,
    {
        let user_fee_increase = settings
            .and_then(|s| s.user_fee_increase)
            .unwrap_or_default();
        let state_transition = IdentityTopUpTransition::try_from_identity_with_signer(
            self,
            asset_lock_proof,
            asset_lock_proof_path,
            asset_lock_signer,
            user_fee_increase,
            sdk.version(),
            None,
        )
        .await?;
        ensure_valid_state_transition_structure(&state_transition, sdk.version())?;
        let identity: PartialIdentity = state_transition
            .broadcast_and_wait_for_affected_state(sdk, settings)
            .await?;

        identity
            .balance
            .ok_or(Error::Generic("expected an identity balance".to_string()))
    }
}
