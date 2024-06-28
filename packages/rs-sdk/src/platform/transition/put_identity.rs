use crate::platform::transition::broadcast_identity::BroadcastRequestForNewIdentity;
use crate::platform::transition::broadcast_request::BroadcastRequestForStateTransition;
use crate::platform::Fetch;
use crate::{Error, Sdk};

use dapi_grpc::platform::VersionedGrpcResponse;
use dapi_grpc::tonic::Code;
use dpp::dashcore::PrivateKey;
use dpp::identity::signer::Signer;
use dpp::prelude::{AssetLockProof, Identity};
use drive_proof_verifier::error::ContextProviderError;
use drive_proof_verifier::DataContractProvider;

use crate::platform::block_info_from_metadata::block_info_from_metadata;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use drive::drive::Drive;
use rs_dapi_client::{DapiClientError, DapiRequest, RequestSettings};

#[async_trait::async_trait]
/// A trait for putting an identity to platform
pub trait PutIdentity<S: Signer> {
    /// Puts an identity on platform
    async fn put_to_platform(
        &self,
        sdk: &Sdk,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &PrivateKey,
        signer: &S,
    ) -> Result<(), Error>;
    /// Puts an identity on platform and waits for the confirmation proof
    async fn put_to_platform_and_wait_for_response(
        &self,
        sdk: &Sdk,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &PrivateKey,
        signer: &S,
    ) -> Result<Identity, Error>;
}

#[async_trait::async_trait]
impl<S: Signer> PutIdentity<S> for Identity {
    async fn put_to_platform(
        &self,
        sdk: &Sdk,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &PrivateKey,
        signer: &S,
    ) -> Result<(), Error> {
        let (_, request) = self.broadcast_request_for_new_identity(
            asset_lock_proof,
            asset_lock_proof_private_key,
            signer,
            sdk.version(),
        )?;

        request
            .clone()
            .execute(sdk, RequestSettings::default())
            .await?;

        // response is empty for a broadcast, result comes from the stream wait for state transition result

        Ok(())
    }

    async fn put_to_platform_and_wait_for_response(
        &self,
        sdk: &Sdk,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &PrivateKey,
        signer: &S,
    ) -> Result<Identity, Error> {
        let identity_id = asset_lock_proof.create_identifier()?;
        let (state_transition, request) = self.broadcast_request_for_new_identity(
            asset_lock_proof,
            asset_lock_proof_private_key,
            signer,
            sdk.version(),
        )?;

        let response_result = request
            .clone()
            .execute(sdk, RequestSettings::default())
            .await;

        match response_result {
            Ok(_) => {}
            //todo make this more reliable
            Err(DapiClientError::Transport(te, _)) if te.code() == Code::AlreadyExists => {
                tracing::debug!(
                    ?identity_id,
                    "attempt to create identity that already exists"
                );
                let identity = Identity::fetch(sdk, identity_id).await?;
                return identity.ok_or(Error::DapiClientError(
                    "identity was proved to not exist but was said to exist".to_string(),
                ));
            }
            Err(e) => return Err(e.into()),
        }

        let request = state_transition.wait_for_state_transition_result_request()?;

        let response = request.execute(sdk, RequestSettings::default()).await?;

        let block_info = block_info_from_metadata(response.metadata()?)?;
        let proof = response.proof_owned()?;
        let context_provider =
            sdk.context_provider()
                .ok_or(Error::from(ContextProviderError::Config(
                    "Context provider not initialized".to_string(),
                )))?;

        let (_, result) = Drive::verify_state_transition_was_executed_with_proof(
            &state_transition,
            &block_info,
            proof.grovedb_proof.as_slice(),
            &context_provider.as_contract_lookup_fn(),
            sdk.version(),
        )?;

        //todo verify

        match result {
            StateTransitionProofResult::VerifiedIdentity(identity) => Ok(identity),
            _ => Err(Error::DapiClientError("proved a non identity".to_string())),
        }
    }
}
