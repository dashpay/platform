use crate::platform::transition::broadcast_identity::BroadcastRequestForNewIdentity;
use crate::{Error, Sdk};

use super::broadcast::BroadcastStateTransition;
use super::put_settings::PutSettings;
use super::waitable::Waitable;
use dpp::dashcore::PrivateKey;
use dpp::identity::signer::Signer;
use dpp::identity::state_transition::asset_lock_proof::AssetLockProof;
use dpp::identity::Identity;
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
        println!("PutIdentity::put_to_platform.1: {:p} {:p}", &asset_lock_proof, &asset_lock_proof_private_key);
        let (state_transition, _) = self.broadcast_request_for_new_identity(
            asset_lock_proof,
            asset_lock_proof_private_key,
            signer,
            sdk.version(),
        )?;
        println!("PutIdentity::put_to_platform.2: {:p}", &state_transition);
        // response is empty for a broadcast, result comes from the stream wait for state transition result
        state_transition.broadcast(sdk, settings).await?;
        println!("PutIdentity::put_to_platform.3: {:p}", &state_transition);
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
        println!("put_to_platform_and_wait_for_response.1: {:p} / {:p} / {:p} / {:p} / {:p}", sdk, &asset_lock_proof, asset_lock_proof_private_key, signer, &settings);
        let state_transition = self
            .put_to_platform(
                sdk,
                asset_lock_proof,
                asset_lock_proof_private_key,
                signer,
                settings,
            )
            .await?;
        println!("put_to_platform_and_wait_for_response.2: {:p}", &state_transition);
        Self::wait_for_response(sdk, state_transition, settings).await
    }
}
