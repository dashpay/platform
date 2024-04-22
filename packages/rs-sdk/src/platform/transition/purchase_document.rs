use crate::platform::transition::broadcast_request::BroadcastRequestForStateTransition;
use std::sync::Arc;

use crate::{Error, Sdk};

use crate::platform::block_info_from_metadata::block_info_from_metadata;
use crate::platform::transition::put_settings::PutSettings;
use dapi_grpc::platform::VersionedGrpcResponse;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentType;
use dpp::data_contract::DataContract;
use dpp::document::{Document, DocumentV0Getters};
use dpp::fee::Credits;
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::prelude::Identifier;
use dpp::state_transition::documents_batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
use dpp::state_transition::documents_batch_transition::DocumentsBatchTransition;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::state_transition::StateTransition;
use drive::drive::Drive;
use rs_dapi_client::{DapiRequest, RequestSettings};

#[async_trait::async_trait]
/// A trait for purchasing a document on Platform
pub trait PurchaseDocument<S: Signer> {
    /// Tries to purchase a document on platform
    /// Setting settings to `None` sets default connection behavior
    async fn purchase_document(
        &self,
        price: Credits,
        sdk: &Sdk,
        document_type: DocumentType,
        purchaser_id: Identifier,
        identity_public_key: IdentityPublicKey,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<StateTransition, Error>;

    /// Waits for the response of a state transition after it has been broadcast
    async fn wait_for_response(
        &self,
        sdk: &Sdk,
        state_transition: StateTransition,
        data_contract: Arc<DataContract>,
    ) -> Result<Document, Error>;

    /// Tries to purchase a document on platform and waits for the response
    async fn purchase_document_and_wait_for_response(
        &self,
        price: Credits,
        sdk: &Sdk,
        document_type: DocumentType,
        purchaser_id: Identifier,
        identity_public_key: IdentityPublicKey,
        data_contract: Arc<DataContract>,
        signer: &S,
    ) -> Result<Document, Error>;
}

#[async_trait::async_trait]
impl<S: Signer> PurchaseDocument<S> for Document {
    async fn purchase_document(
        &self,
        price: Credits,
        sdk: &Sdk,
        document_type: DocumentType,
        purchaser_id: Identifier,
        identity_public_key: IdentityPublicKey,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<StateTransition, Error> {
        let new_identity_contract_nonce = sdk
            .get_identity_contract_nonce(
                purchaser_id,
                document_type.data_contract_id(),
                true,
                settings,
            )
            .await?;

        let settings = settings.unwrap_or_default();

        let transition = DocumentsBatchTransition::new_document_purchase_transition_from_document(
            self.clone(),
            document_type.as_ref(),
            purchaser_id,
            price,
            &identity_public_key,
            new_identity_contract_nonce,
            settings.user_fee_increase.unwrap_or_default(),
            signer,
            sdk.version(),
            None,
            None,
            None,
        )?;

        let request = transition.broadcast_request_for_state_transition()?;

        request
            .clone()
            .execute(sdk, settings.request_settings)
            .await?;

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

        let block_info = block_info_from_metadata(response.metadata()?)?;

        let proof = response.proof_owned()?;

        let (_, result) = Drive::verify_state_transition_was_executed_with_proof(
            &state_transition,
            &block_info,
            proof.grovedb_proof.as_slice(),
            &|_| Ok(Some(data_contract.clone())),
            sdk.version(),
        )?;

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

    async fn purchase_document_and_wait_for_response(
        &self,
        price: Credits,
        sdk: &Sdk,
        document_type: DocumentType,
        purchaser_id: Identifier,
        identity_public_key: IdentityPublicKey,
        data_contract: Arc<DataContract>,
        signer: &S,
    ) -> Result<Document, Error> {
        let state_transition = self
            .purchase_document(
                price,
                sdk,
                document_type,
                purchaser_id,
                identity_public_key,
                signer,
                None,
            )
            .await?;

        let document = <Self as PurchaseDocument<S>>::wait_for_response(
            self,
            sdk,
            state_transition,
            data_contract,
        )
        .await?;

        Ok(document)
    }
}
