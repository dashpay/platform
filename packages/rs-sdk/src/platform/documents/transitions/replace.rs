use crate::platform::transition::broadcast::BroadcastStateTransition;
use crate::platform::transition::put_settings::PutSettings;
use crate::{Error, Sdk};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::DocumentType;
use dpp::data_contract::DataContract;
use dpp::document::{Document, DocumentV0Getters};
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

/// A builder to configure and broadcast document replace transitions
pub struct DocumentReplaceTransitionBuilder<'a> {
    data_contract: &'a DataContract,
    document_type: DocumentType,
    document: Document,
    token_payment_info: Option<TokenPaymentInfo>,
    settings: Option<PutSettings>,
    user_fee_increase: Option<UserFeeIncrease>,
}

impl<'a> DocumentReplaceTransitionBuilder<'a> {
    /// Start building a replace document request for the provided DataContract.
    ///
    /// # Arguments
    ///
    /// * `data_contract` - A reference to the data contract
    /// * `document_type` - The document type to replace
    /// * `document` - The document with updated values
    ///
    /// # Returns
    ///
    /// * `Self` - The new builder instance
    pub fn new(
        data_contract: &'a DataContract,
        document_type: DocumentType,
        document: Document,
    ) -> Self {
        Self {
            data_contract,
            document_type,
            document,
            token_payment_info: None,
            settings: None,
            user_fee_increase: None,
        }
    }

    /// Adds token payment info to the document replace transition
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

    /// Adds a user fee increase to the document replace transition
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

    /// Adds settings to the document replace transition
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

    /// Signs the document replace transition
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
                self.document.owner_id(),
                self.data_contract.id(),
                true,
                self.settings,
            )
            .await?;

        let state_transition = BatchTransition::new_document_replacement_transition_from_document(
            self.document.clone(),
            self.document_type.as_ref(),
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

/// Result types returned from document replace operations.
#[derive(Debug)]
pub enum DocumentReplaceResult {
    /// Document replace result containing the updated document.
    Document(Document),
}

impl Sdk {
    /// Replaces an existing document on the platform.
    ///
    /// This method broadcasts a document replacement transition to update an existing
    /// document with new data. The result contains the updated document.
    ///
    /// # Arguments
    ///
    /// * `replace_document_transition_builder` - Builder containing document replacement parameters
    /// * `signing_key` - The identity public key for signing the transition
    /// * `signer` - Implementation of the Signer trait for cryptographic signing
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a `DocumentReplaceResult` on success, or an `Error` on failure.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The transition signing fails
    /// - Broadcasting the transition fails
    /// - The proof verification returns an unexpected result type
    /// - Document validation fails
    /// - Document not found or revision mismatch
    pub async fn document_replace<S: Signer>(
        &self,
        replace_document_transition_builder: DocumentReplaceTransitionBuilder<'_>,
        signing_key: &IdentityPublicKey,
        signer: &S,
    ) -> Result<DocumentReplaceResult, Error> {
        let platform_version = self.version();

        let state_transition = replace_document_transition_builder
            .sign(self, signing_key, signer, platform_version, None)
            .await?;

        let proof_result = state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(self, None)
            .await?;

        match proof_result {
            StateTransitionProofResult::VerifiedDocuments(documents) => {
                if let Some((_, Some(document))) = documents.into_iter().next() {
                    Ok(DocumentReplaceResult::Document(document))
                } else {
                    Err(Error::DriveProofError(
                        drive::error::proof::ProofError::UnexpectedResultProof(
                            "Expected document in VerifiedDocuments result for replace transition"
                                .to_string(),
                        ),
                        vec![],
                        Default::default(),
                    ))
                }
            }
            _ => Err(Error::DriveProofError(
                drive::error::proof::ProofError::UnexpectedResultProof(
                    "Expected VerifiedDocuments for document replace transition".to_string(),
                ),
                vec![],
                Default::default(),
            )),
        }
    }
}
