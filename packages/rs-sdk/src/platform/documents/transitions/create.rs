use crate::platform::transition::broadcast::BroadcastStateTransition;
use crate::platform::transition::put_settings::PutSettings;
use crate::{Error, Sdk};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::DataContract;
use dpp::document::{Document, DocumentV0Getters};
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::prelude::UserFeeIncrease;
use dpp::serialization::PlatformSerializable;
use dpp::state_transition::batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
use dpp::state_transition::batch_transition::methods::StateTransitionCreationOptions;
use dpp::state_transition::batch_transition::BatchTransition;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::state_transition::StateTransition;
use dpp::tokens::token_payment_info::TokenPaymentInfo;
use dpp::version::PlatformVersion;
use std::sync::Arc;
use tracing::trace;

/// A builder to configure and broadcast document create transitions
pub struct DocumentCreateTransitionBuilder {
    pub data_contract: Arc<DataContract>,
    pub document_type_name: String,
    pub document: Document,
    pub document_state_transition_entropy: [u8; 32],
    pub token_payment_info: Option<TokenPaymentInfo>,
    pub settings: Option<PutSettings>,
    pub user_fee_increase: Option<UserFeeIncrease>,
    pub state_transition_creation_options: Option<StateTransitionCreationOptions>,
}

impl DocumentCreateTransitionBuilder {
    /// Start building a create document request for the provided DataContract.
    ///
    /// # Arguments
    ///
    /// * `data_contract` - The data contract
    /// * `document_type_name` - The name of the document type to create
    /// * `document` - The document to create
    /// * `document_state_transition_entropy` - Entropy for the state transition
    ///
    /// # Returns
    ///
    /// * `Self` - The new builder instance
    pub fn new(
        data_contract: Arc<DataContract>,
        document_type_name: String,
        document: Document,
        document_state_transition_entropy: [u8; 32],
    ) -> Self {
        Self {
            data_contract,
            document_type_name,
            document,
            document_state_transition_entropy,
            token_payment_info: None,
            settings: None,
            user_fee_increase: None,
            state_transition_creation_options: None,
        }
    }

    /// Adds token payment info to the document create transition
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

    /// Adds a user fee increase to the document create transition
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

    /// Adds settings to the document create transition
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

    /// Adds creation_options to the document create transition
    ///
    /// # Arguments
    ///
    /// * `settings` - The settings to add
    ///
    /// # Returns
    ///
    /// * `Self` - The updated builder
    pub fn with_state_transition_creation_options(
        mut self,
        creation_options: StateTransitionCreationOptions,
    ) -> Self {
        self.state_transition_creation_options = Some(creation_options);
        self
    }

    /// Signs the document create transition
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
    ) -> Result<StateTransition, Error> {
        let identity_contract_nonce = sdk
            .get_identity_contract_nonce(
                self.document.owner_id(),
                self.data_contract.id(),
                true,
                self.settings,
            )
            .await?;

        let document_type = self
            .data_contract
            .document_type_for_name(&self.document_type_name)
            .map_err(|e| Error::Protocol(e.into()))?;

        let state_transition = BatchTransition::new_document_creation_transition_from_document(
            self.document.clone(),
            document_type,
            self.document_state_transition_entropy,
            identity_public_key,
            identity_contract_nonce,
            self.user_fee_increase.unwrap_or_default(),
            self.token_payment_info,
            signer,
            platform_version,
            self.state_transition_creation_options,
        )?;

        Ok(state_transition)
    }
}

/// Result types returned from document creation operations.
#[derive(Debug)]
pub enum DocumentCreateResult {
    /// Document creation result containing the created document.
    Document(Document),
}

impl Sdk {
    /// Creates a new document on the platform.
    ///
    /// This method broadcasts a document creation transition to add a new document
    /// to the specified data contract. The result contains the created document.
    ///
    /// # Arguments
    ///
    /// * `create_document_transition_builder` - Builder containing document creation parameters
    /// * `signing_key` - The identity public key for signing the transition
    /// * `signer` - Implementation of the Signer trait for cryptographic signing
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a `DocumentCreateResult` on success, or an `Error` on failure.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The transition signing fails
    /// - Broadcasting the transition fails
    /// - The proof verification returns an unexpected result type
    /// - Document validation fails
    pub async fn document_create<S: Signer>(
        &self,
        create_document_transition_builder: DocumentCreateTransitionBuilder,
        signing_key: &IdentityPublicKey,
        signer: &S,
    ) -> Result<DocumentCreateResult, Error> {
        let platform_version = self.version();

        let put_settings = create_document_transition_builder.settings;

        let state_transition = create_document_transition_builder
            .sign(self, signing_key, signer, platform_version)
            .await?;

        // Low-level debug logging via tracing
        trace!("document_create: state transition created and signed");
        trace!(hex = %hex::encode(state_transition.serialize_to_bytes()?), "document_create: transition bytes");
        trace!(transition = ?state_transition, "document_create: transition details");

        let proof_result = state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(self, put_settings)
            .await?;

        match proof_result {
            StateTransitionProofResult::VerifiedDocuments(documents) => {
                if let Some((_, Some(document))) = documents.into_iter().next() {
                    Ok(DocumentCreateResult::Document(document))
                } else {
                    Err(Error::DriveProofError(
                        drive::error::proof::ProofError::UnexpectedResultProof(
                            "Expected document in VerifiedDocuments result for create transition"
                                .to_string(),
                        ),
                        vec![],
                        Default::default(),
                    ))
                }
            }
            _ => Err(Error::DriveProofError(
                drive::error::proof::ProofError::UnexpectedResultProof(
                    "Expected VerifiedDocuments for document create transition".to_string(),
                ),
                vec![],
                Default::default(),
            )),
        }
    }
}
