use crate::platform::transition::broadcast_request::BroadcastRequestForStateTransition;
use std::sync::Arc;

use crate::{Error, Sdk};

use dapi_grpc::platform::VersionedGrpcResponse;
use dpp::data_contract::document_type::DocumentType;
use dpp::data_contract::DataContract;
use dpp::document::{Document, DocumentV0Getters};
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::state_transition::documents_batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
use dpp::state_transition::documents_batch_transition::DocumentsBatchTransition;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::state_transition::StateTransition;
use drive::drive::Drive;
use rs_dapi_client::{DapiRequest, RequestSettings};

#[async_trait::async_trait]
/// A trait for putting an identity to platform
pub trait PutDocument<S: Signer> {
    /// Puts an identity on platform
    async fn put_to_platform(
        &self,
        sdk: &Sdk,
        document_type: DocumentType,
        document_state_transition_entropy: [u8; 32],
        identity_public_key: IdentityPublicKey,
        signer: &S,
    ) -> Result<StateTransition, Error>;

    /// Waits for the response of a state transition after it has been broadcast
    async fn wait_for_response(
        &self,
        sdk: &Sdk,
        state_transition: StateTransition,
        data_contract: Arc<DataContract>,
    ) -> Result<Document, Error>;

    /// Puts an identity on platform and waits for the confirmation proof
    async fn put_to_platform_and_wait_for_response(
        &self,
        sdk: &Sdk,
        document_type: DocumentType,
        document_state_transition_entropy: [u8; 32],
        identity_public_key: IdentityPublicKey,
        data_contract: Arc<DataContract>,
        signer: &S,
    ) -> Result<Document, Error>;

    async fn put_to_platform_with_settings(
        &self,
        sdk: &Sdk,
        document_type: DocumentType,
        document_state_transition_entropy: [u8; 32],
        identity_public_key: IdentityPublicKey,
        signer: &S,
        settings: RequestSettings,
    ) -> Result<StateTransition, Error>;
}

#[async_trait::async_trait]
impl<S: Signer> PutDocument<S> for Document {
    async fn put_to_platform(
        &self,
        sdk: &Sdk,
        document_type: DocumentType,
        document_state_transition_entropy: [u8; 32],
        identity_public_key: IdentityPublicKey,
        signer: &S,
    ) -> Result<StateTransition, Error> {
        self.put_to_platform_with_settings(
            sdk,
            document_type,
            document_state_transition_entropy,
            identity_public_key,
            signer,
            RequestSettings::default(),
        )
        .await
    }

    async fn put_to_platform_with_settings(
        &self,
        sdk: &Sdk,
        document_type: DocumentType,
        document_state_transition_entropy: [u8; 32],
        identity_public_key: IdentityPublicKey,
        signer: &S,
        settings: RequestSettings,
    ) -> Result<StateTransition, Error> {
        let transition = DocumentsBatchTransition::new_document_creation_transition_from_document(
            self.clone(),
            document_type.as_ref(),
            document_state_transition_entropy,
            &identity_public_key,
            signer,
            sdk.version(),
            None,
            None,
            None,
        )?;

        let request = transition.broadcast_request_for_state_transition()?;

        request.clone().execute(sdk, settings).await?;

        // response is empty for a broadcast, result comes from the stream wait for state transition result

        Ok(transition)
    }

    async fn wait_for_response(
        &self,
        sdk: &Sdk,
        state_transition: StateTransition,
        data_contract: Arc<DataContract>,
    ) -> Result<Document, Error> {
        let request = state_transition.wait_for_state_transition_result_request()?;

        let response = request.execute(sdk, RequestSettings::default()).await?;

        let proof = response.proof_owned()?;

        let (_, result) = Drive::verify_state_transition_was_executed_with_proof(
            &state_transition,
            proof.grovedb_proof.as_slice(),
            &|_| Ok(Some(data_contract.clone())),
            sdk.version(),
        )?;

        //todo verify

        match result {
            StateTransitionProofResult::VerifiedDocuments(mut documents) => {
                let document = documents
                    .remove(self.id_ref())
                    .ok_or(Error::InvalidProvedResponse(
                        "did not prove the sent document".to_string(),
                    ))?
                    .ok_or(Error::InvalidProvedResponse(
                        "expected there to actually be a document".to_string(),
                    ))?;
                Ok(document)
            }
            _ => Err(Error::DapiClientError("proved a non document".to_string())),
        }
    }

    async fn put_to_platform_and_wait_for_response(
        &self,
        sdk: &Sdk,
        document_type: DocumentType,
        document_state_transition_entropy: [u8; 32],
        identity_public_key: IdentityPublicKey,
        data_contract: Arc<DataContract>,
        signer: &S,
    ) -> Result<Document, Error> {
        let state_transition = self
            .put_to_platform(
                sdk,
                document_type,
                document_state_transition_entropy,
                identity_public_key,
                signer,
            )
            .await?;

        // TODO: Why do we need full type annotation?
        let document =
            <Self as PutDocument<S>>::wait_for_response(self, sdk, state_transition, data_contract)
                .await?;

        Ok(document)
    }
}
