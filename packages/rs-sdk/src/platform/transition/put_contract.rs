use crate::platform::transition::broadcast_request::BroadcastRequestForStateTransition;
use std::collections::BTreeMap;

use crate::{Error, Sdk};

use crate::platform::transition::put_settings::PutSettings;
use dapi_grpc::platform::VersionedGrpcResponse;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::DataContract;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::signer::Signer;
use dpp::identity::{IdentityPublicKey, PartialIdentity};
use dpp::state_transition::data_contract_create_transition::methods::DataContractCreateTransitionMethodsV0;
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::state_transition::StateTransition;
use drive::drive::Drive;
use rs_dapi_client::{DapiRequest, RequestSettings};

#[async_trait::async_trait]
/// A trait for putting a contract to platform
pub trait PutContract<S: Signer> {
    /// Puts a document on platform
    /// setting settings to `None` sets default connection behavior
    async fn put_to_platform(
        &self,
        sdk: &Sdk,
        identity_public_key: IdentityPublicKey,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<StateTransition, Error>;

    /// Waits for the response of a state transition after it has been broadcast
    async fn wait_for_response(
        &self,
        sdk: &Sdk,
        state_transition: StateTransition,
    ) -> Result<DataContract, Error>;

    /// Puts a contract on platform and waits for the confirmation proof
    async fn put_to_platform_and_wait_for_response(
        &self,
        sdk: &Sdk,
        identity_public_key: IdentityPublicKey,
        signer: &S,
    ) -> Result<DataContract, Error>;
}

#[async_trait::async_trait]
impl<S: Signer> PutContract<S> for DataContract {
    async fn put_to_platform(
        &self,
        sdk: &Sdk,
        identity_public_key: IdentityPublicKey,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<StateTransition, Error> {
        let new_identity_nonce = sdk
            .get_identity_nonce(self.owner_id(), true, settings)
            .await?;

        let key_id = identity_public_key.id();

        let partial_identity = PartialIdentity {
            id: self.owner_id(),
            loaded_public_keys: BTreeMap::from([(key_id, identity_public_key)]),
            balance: None,
            revision: None,
            not_found_public_keys: Default::default(),
        };
        let transition = DataContractCreateTransition::new_from_data_contract(
            self.clone(),
            new_identity_nonce,
            &partial_identity,
            key_id,
            signer,
            sdk.version(),
            None,
        )?;

        let request = transition.broadcast_request_for_state_transition()?;

        request
            .clone()
            .execute(sdk, settings.unwrap_or_default().request_settings)
            .await?;

        // response is empty for a broadcast, result comes from the stream wait for state transition result

        Ok(transition)
    }

    async fn wait_for_response(
        &self,
        sdk: &Sdk,
        state_transition: StateTransition,
    ) -> Result<DataContract, Error> {
        let request = state_transition.wait_for_state_transition_result_request()?;

        let response = request.execute(sdk, RequestSettings::default()).await?;

        let block_time = response.metadata()?.time_ms;

        let proof = response.proof_owned()?;

        let (_, result) = Drive::verify_state_transition_was_executed_with_proof(
            &state_transition,
            block_time,
            proof.grovedb_proof.as_slice(),
            &|_| Ok(None),
            sdk.version(),
        )?;

        //todo verify

        match result {
            StateTransitionProofResult::VerifiedDataContract(data_contract) => Ok(data_contract),
            _ => Err(Error::DapiClientError("proved a non document".to_string())),
        }
    }

    async fn put_to_platform_and_wait_for_response(
        &self,
        sdk: &Sdk,
        identity_public_key: IdentityPublicKey,
        signer: &S,
    ) -> Result<DataContract, Error> {
        let state_transition = self
            .put_to_platform(sdk, identity_public_key, signer, None)
            .await?;

        let data_contract =
            <Self as PutContract<S>>::wait_for_response(self, sdk, state_transition).await?;

        Ok(data_contract)
    }
}
