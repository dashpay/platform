use crate::platform::transition::broadcast::BroadcastStateTransition;
use crate::platform::transition::put_settings::PutSettings;
use crate::platform::Identifier;
use crate::{Error, Sdk};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::DataContract;
use dpp::document::{Document, INITIAL_REVISION};
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::prelude::UserFeeIncrease;
use dpp::state_transition::batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
use dpp::state_transition::batch_transition::methods::StateTransitionCreationOptions;
use dpp::state_transition::batch_transition::BatchTransition;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::state_transition::StateTransition;
use dpp::tokens::token_payment_info::TokenPaymentInfo;
use dpp::version::PlatformVersion;
use std::sync::Arc;

/// A builder to configure and broadcast document delete transitions
pub struct DocumentDeleteTransitionBuilder {
    data_contract: Arc<DataContract>,
    document_type_name: String,
    document_id: Identifier,
    owner_id: Identifier,
    token_payment_info: Option<TokenPaymentInfo>,
    settings: Option<PutSettings>,
    user_fee_increase: Option<UserFeeIncrease>,
}

impl DocumentDeleteTransitionBuilder {
    /// Start building a delete document request for the provided DataContract.
    ///
    /// # Arguments
    ///
    /// * `data_contract` - The data contract
    /// * `document_type_name` - The name of the document type to delete
    /// * `document_id` - The ID of the document to delete
    /// * `owner_id` - The owner ID of the document
    ///
    /// # Returns
    ///
    /// * `Self` - The new builder instance
    pub fn new(
        data_contract: Arc<DataContract>,
        document_type_name: String,
        document_id: Identifier,
        owner_id: Identifier,
    ) -> Self {
        Self {
            data_contract,
            document_type_name,
            document_id,
            owner_id,
            token_payment_info: None,
            settings: None,
            user_fee_increase: None,
        }
    }

    /// Creates a new builder from an existing document
    ///
    /// # Arguments
    ///
    /// * `data_contract` - The data contract
    /// * `document_type_name` - The name of the document type to delete
    /// * `document` - The document to delete
    ///
    /// # Returns
    ///
    /// * `Self` - The new builder instance
    pub fn from_document(
        data_contract: Arc<DataContract>,
        document_type_name: String,
        document: &Document,
    ) -> Self {
        use dpp::document::DocumentV0Getters;
        Self::new(
            data_contract,
            document_type_name,
            document.id(),
            document.owner_id(),
        )
    }

    /// Adds token payment info to the document delete transition
    ///
    /// # Arguments
    ///
    /// * `token_payment_info` - The token payment info to add
    ///
    /// # Returns
    ///
    /// * `Self` - The updated builder
    pub fn with_token_payment_info(mut self, token_payment_info: TokenPaymentInfo) -> Self {
        self.token_payment_info = Some(token_payment_info);
        self
    }

    /// Adds a user fee increase to the document delete transition
    ///
    /// # Arguments
    ///
    /// * `user_fee_increase` - The user fee increase to add
    ///
    /// # Returns
    ///
    /// * `Self` - The updated builder
    pub fn with_user_fee_increase(mut self, user_fee_increase: UserFeeIncrease) -> Self {
        self.user_fee_increase = Some(user_fee_increase);
        self
    }

    /// Adds settings to the document delete transition
    ///
    /// # Arguments
    ///
    /// * `settings` - The settings to add
    ///
    /// # Returns
    ///
    /// * `Self` - The updated builder
    pub fn with_settings(mut self, settings: PutSettings) -> Self {
        self.settings = Some(settings);
        self
    }

    /// Signs the document delete transition
    ///
    /// # Arguments
    ///
    /// * `sdk` - The SDK instance
    /// * `identity_public_key` - The public key of the identity
    /// * `signer` - The signer instance
    /// * `platform_version` - The platform version
    /// * `options` - Optional state transition creation options
    ///
    /// # Returns
    ///
    /// * `Result<StateTransition, Error>` - The signed state transition or an error
    pub async fn sign(
        &self,
        sdk: &Sdk,
        identity_public_key: &IdentityPublicKey,
        signer: &impl Signer,
        platform_version: &PlatformVersion,
        options: Option<StateTransitionCreationOptions>,
    ) -> Result<StateTransition, Error> {
        let identity_contract_nonce = sdk
            .get_identity_contract_nonce(
                self.owner_id,
                self.data_contract.id(),
                true,
                self.settings,
            )
            .await?;

        let document_type = self
            .data_contract
            .document_type_for_name(&self.document_type_name)
            .map_err(|e| Error::Protocol(e.into()))?;

        // Create a minimal document for deletion
        let document = Document::V0(dpp::document::DocumentV0 {
            id: self.document_id,
            owner_id: self.owner_id,
            properties: Default::default(),
            revision: Some(INITIAL_REVISION),
            created_at: None,
            updated_at: None,
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
        });

        let state_transition = BatchTransition::new_document_deletion_transition_from_document(
            document,
            document_type,
            identity_public_key,
            identity_contract_nonce,
            self.user_fee_increase.unwrap_or_default(),
            self.token_payment_info,
            signer,
            platform_version,
            options,
        )?;

        Ok(state_transition)
    }
}

/// Result types returned from document delete operations.
#[derive(Debug)]
pub enum DocumentDeleteResult {
    /// Document deletion confirmed (document no longer exists).
    Deleted(Identifier),
}

impl Sdk {
    /// Deletes an existing document from the platform.
    ///
    /// This method broadcasts a document deletion transition to permanently remove
    /// a document from the platform. The result confirms the deletion.
    ///
    /// # Arguments
    ///
    /// * `delete_document_transition_builder` - Builder containing document deletion parameters
    /// * `signing_key` - The identity public key for signing the transition
    /// * `signer` - Implementation of the Signer trait for cryptographic signing
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a `DocumentDeleteResult` on success, or an `Error` on failure.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The transition signing fails
    /// - Broadcasting the transition fails
    /// - The proof verification returns an unexpected result type
    /// - Document not found or already deleted
    /// - Insufficient permissions to delete the document
    pub async fn document_delete<S: Signer>(
        &self,
        delete_document_transition_builder: DocumentDeleteTransitionBuilder,
        signing_key: &IdentityPublicKey,
        signer: &S,
    ) -> Result<DocumentDeleteResult, Error> {
        let platform_version = self.version();

        let state_transition = delete_document_transition_builder
            .sign(self, signing_key, signer, platform_version, None)
            .await?;

        let proof_result = state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(self, None)
            .await?;

        match proof_result {
            StateTransitionProofResult::VerifiedDocuments(documents) => {
                if let Some((document_id, None)) = documents.into_iter().next() {
                    // None indicates the document has been deleted
                    Ok(DocumentDeleteResult::Deleted(document_id))
                } else {
                    Err(Error::DriveProofError(
                        drive::error::proof::ProofError::UnexpectedResultProof(
                            "Expected deleted document (None) in VerifiedDocuments result for delete transition".to_string(),
                        ),
                        vec![],
                        Default::default(),
                    ))
                }
            }
            _ => Err(Error::DriveProofError(
                drive::error::proof::ProofError::UnexpectedResultProof(
                    "Expected VerifiedDocuments for document delete transition".to_string(),
                ),
                vec![],
                Default::default(),
            )),
        }
    }
}
