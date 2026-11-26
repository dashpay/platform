use super::asset_lock_submission::submit_with_cl_height_retry;
use super::broadcast::{
    broadcast_may_have_been_accepted, convert_proof_result, BroadcastStateTransition,
    WaitForOutcome,
};
use super::put_settings::PutSettings;
use super::validation::ensure_valid_state_transition_structure;
use super::waitable::Waitable;
use crate::{Error, Sdk};
use dapi_grpc::platform::v0::ResponseMetadata;
use dpp::dashcore::PrivateKey;
use dpp::identity::{Identity, PartialIdentity};
use dpp::prelude::AssetLockProof;
use dpp::state_transition::identity_topup_transition::methods::IdentityTopUpTransitionMethodsV0;
use dpp::state_transition::identity_topup_transition::IdentityTopUpTransition;
use dpp::state_transition::StateTransition;

#[async_trait::async_trait]
pub trait TopUpIdentity: Waitable {
    /// Tops up an existing identity using an asset lock proof whose
    /// private key is held in-process. Retries definite ChainLock-height lag within
    /// the bounded SDK retry budget. A duplicate returns [`Error::AlreadyExists`];
    /// uncertain delivery stays [`Error::StateTransitionResultUnconfirmed`].
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

/// Balance operations that also expose the committed proof metadata.
#[async_trait::async_trait]
pub trait TopUpIdentityWithMetadata: Waitable {
    /// Returns the balance snapshot and its proof metadata after an acknowledged submission.
    /// Uses the same bounded height retry and duplicate handling as [`TopUpIdentity`].
    #[cfg(feature = "core_key_wallet")]
    async fn top_up_identity_with_signer_with_metadata<AS>(
        &self,
        sdk: &Sdk,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_path: &dpp::key_wallet::bip32::DerivationPath,
        asset_lock_signer: &AS,
        settings: Option<PutSettings>,
    ) -> Result<(u64, ResponseMetadata), Error>
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
        submit_with_cl_height_retry(settings, |settings| {
            let asset_lock_proof = asset_lock_proof.clone();
            async move {
                let user_fee_increase = settings
                    .and_then(|s| s.user_fee_increase)
                    .unwrap_or_default();
                let state_transition = IdentityTopUpTransition::try_from_identity_with_private_key(
                    self,
                    asset_lock_proof,
                    &asset_lock_proof_private_key.inner.to_bytes()[..],
                    user_fee_increase,
                    sdk.version(),
                    None,
                )?;
                submit_top_up(sdk, state_transition, settings).await
            }
        })
        .await
        .map(|(balance, _)| balance)
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
        self.top_up_identity_with_signer_with_metadata(
            sdk,
            asset_lock_proof,
            asset_lock_proof_path,
            asset_lock_signer,
            settings,
        )
        .await
        .map(|(balance, _)| balance)
    }
}

#[async_trait::async_trait]
impl TopUpIdentityWithMetadata for Identity {
    #[cfg(feature = "core_key_wallet")]
    async fn top_up_identity_with_signer_with_metadata<AS>(
        &self,
        sdk: &Sdk,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_path: &dpp::key_wallet::bip32::DerivationPath,
        asset_lock_signer: &AS,
        settings: Option<PutSettings>,
    ) -> Result<(u64, ResponseMetadata), Error>
    where
        AS: dpp::key_wallet::signer::Signer + Send + Sync,
    {
        submit_with_cl_height_retry(settings, |settings| {
            let asset_lock_proof = asset_lock_proof.clone();
            async move {
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
                submit_top_up(sdk, state_transition, settings).await
            }
        })
        .await
    }
}

/// A balance snapshot cannot resolve a duplicate or uncertain top-up submission.
async fn submit_top_up(
    sdk: &Sdk,
    state_transition: StateTransition,
    settings: Option<PutSettings>,
) -> Result<(u64, ResponseMetadata), Error> {
    ensure_valid_state_transition_structure(&state_transition, sdk.version())?;
    let broadcast_error = match state_transition.broadcast(sdk, settings).await {
        Ok(()) => None,
        Err(error) if broadcast_may_have_been_accepted(&error) => Some(error),
        Err(error) => return Err(error),
    };
    let (outcome, metadata) = state_transition
        .wait_for_outcome_with_metadata(sdk, settings)
        .await
        .map_err(Error::into_unconfirmed)?;
    if !outcome.is_execution_proved() {
        if let Some(error) = broadcast_error {
            return Err(top_up_submission_error(error));
        }
    }
    let identity: PartialIdentity =
        convert_proof_result(outcome.into_result()).map_err(Error::into_unconfirmed)?;
    identity
        .balance
        .ok_or(Error::Generic("expected an identity balance".to_string()))
        .map(|balance| (balance, metadata))
}

fn top_up_submission_error(error: Error) -> Error {
    if !error.is_already_exists() {
        return error.into_unconfirmed();
    }
    match error {
        Error::StateTransitionResultUnconfirmed(inner)
        | Error::NoAvailableAddressesToRetry(inner) => top_up_submission_error(*inner),
        duplicate => duplicate,
    }
}
