use crate::platform::block_info_from_metadata::block_info_from_metadata;
use crate::platform::transition::broadcast_identity::BroadcastRequestForNewIdentity;
use crate::{Error, Sdk};

use super::broadcast::BroadcastStateTransition;
use super::put_settings::PutSettings;
use super::waitable::Waitable;
use dpp::dashcore::PrivateKey;
use dpp::identity::signer::Signer;
use dpp::prelude::{AssetLockProof, Identity};
use dpp::state_transition::StateTransition;

/// A trait for putting an identity to platform
#[async_trait::async_trait]
pub trait PutIdentity<S: Signer>: Waitable {
    /// Puts an identity on platform.
    ///
    /// TODO: Discuss if it should not actually consume self, since it is no longer valid (eg. identity id is changed)
    async fn put_to_platform(
        &self,
        sdk: &Sdk,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &PrivateKey,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<StateTransition, Error>;

    /// Puts an identity on platform and waits for the confirmation proof.
    async fn put_to_platform_and_wait_for_response(
        &self,
        sdk: &Sdk,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &PrivateKey,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<Self, Error>;
}
#[async_trait::async_trait]
impl<S: Signer> PutIdentity<S> for Identity {
    async fn put_to_platform(
        &self,
        sdk: &Sdk,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &PrivateKey,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<StateTransition, Error> {
        let (state_transition, _) = self.broadcast_request_for_new_identity(
            asset_lock_proof,
            asset_lock_proof_private_key,
            signer,
            sdk.version(),
        )?;

        // response is empty for a broadcast, result comes from the stream wait for state transition result
        state_transition.broadcast(sdk, settings).await?;
        Ok(state_transition)
    }

    async fn put_to_platform_and_wait_for_response(
        &self,
        sdk: &Sdk,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &PrivateKey,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<Identity, Error> {
        let state_transition = self
            .put_to_platform(
                sdk,
                asset_lock_proof,
                asset_lock_proof_private_key,
                signer,
                settings,
            )
            .await?;

        Self::wait_for_response(sdk, state_transition, settings).await
    }
}
