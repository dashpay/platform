//! Save documents to the platform
use crate::{Error, Sdk};

use dpp::data_contract::document_type::DocumentType;
use dpp::document::{Document, DocumentV0Getters};
use dpp::identity::signer::Signer;
use dpp::state_transition::documents_batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
use dpp::state_transition::documents_batch_transition::DocumentsBatchTransition;
use dpp::state_transition::proof_result::StateTransitionProofResult;

use super::broadcast::BroadcastStateTransition;

#[async_trait::async_trait]
/// A trait for putting an identity to platform
pub trait PutDocument<S: Signer> {
    /// Puts an identity on platform
    async fn put_to_platform(
        &self,
        sdk: &Sdk,
        document_type: DocumentType,
        document_state_transition_entropy: [u8; 32],
    ) -> Result<(), Error>;
    /// Puts an identity on platform and waits for the confirmation proof
    ///
    /// ## Requirements
    ///
    /// Data contract must be available inside the `ContextProvider` configured inside the `Sdk`.
    async fn put_to_platform_and_wait_for_response(
        &self,
        sdk: &Sdk,
        document_type: DocumentType,
        document_state_transition_entropy: [u8; 32], // TODO: ask why we need this as input; maybe rng should be provided via sdk / context provider?
    ) -> Result<Document, Error>;
}

#[async_trait::async_trait]
impl<S: Signer> PutDocument<S> for Document {
    async fn put_to_platform(
        &self,
        sdk: &Sdk,
        document_type: DocumentType,
        document_state_transition_entropy: [u8; 32],
    ) -> Result<(), Error> {
        let wallet = sdk
            .wallet
            .as_ref()
            .ok_or(Error::Config("wallet not configured in sdk".to_string()))?;

        let identity_public_key = wallet
            .identity_public_key(&dpp::identity::Purpose::AUTHENTICATION)
            .ok_or(Error::Config(
                "cannot retrieve identity public key from wallet".to_string(),
            ))?;

        let transition = DocumentsBatchTransition::new_document_creation_transition_from_document(
            self.clone(),
            document_type.as_ref(),
            document_state_transition_entropy,
            &identity_public_key,
            wallet,
            sdk.version(),
            None,
            None,
            None,
        )?;

        transition.broadcast(sdk).await
    }

    async fn put_to_platform_and_wait_for_response(
        &self,
        sdk: &Sdk,
        document_type: DocumentType,
        document_state_transition_entropy: [u8; 32],
    ) -> Result<Document, Error> {
        let wallet = sdk
            .wallet
            .as_ref()
            .ok_or(Error::Config("wallet not configured in sdk".to_string()))?;

        let identity_public_key = wallet
            .identity_public_key(&dpp::identity::Purpose::AUTHENTICATION)
            .ok_or(Error::Config(
                "cannot retrieve identity public key from wallet".to_string(),
            ))?;

        let state_transition =
            DocumentsBatchTransition::new_document_creation_transition_from_document(
                self.clone(),
                document_type.as_ref(),
                document_state_transition_entropy,
                &identity_public_key,
                wallet,
                sdk.version(),
                None,
                None,
                None,
            )?;

        let result = state_transition.broadcast_and_wait(sdk, None).await?;

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
}
