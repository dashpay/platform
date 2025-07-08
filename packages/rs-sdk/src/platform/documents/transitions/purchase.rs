use crate::platform::transition::broadcast::BroadcastStateTransition;
use crate::platform::transition::put_settings::PutSettings;
use crate::platform::Identifier;
use crate::{Error, Sdk};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::DataContract;
use dpp::document::Document;
use dpp::fee::Credits;
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

/// A builder to configure and broadcast document purchase transitions
pub struct DocumentPurchaseTransitionBuilder {
    pub data_contract: Arc<DataContract>,
    pub document_type_name: String,
    pub document: Document,
    pub purchaser_id: Identifier,
    pub price: Credits,
    pub token_payment_info: Option<TokenPaymentInfo>,
    pub settings: Option<PutSettings>,
    pub user_fee_increase: Option<UserFeeIncrease>,
    pub state_transition_creation_options: Option<StateTransitionCreationOptions>,
}

impl DocumentPurchaseTransitionBuilder {
    /// Start building a purchase document request for the provided DataContract.
    ///
    /// # Arguments
    ///
    /// * `data_contract` - The data contract
    /// * `document_type_name` - The name of the document type
    /// * `document` - The document to purchase
    /// * `purchaser_id` - The identifier of the purchaser
    /// * `price` - The price to pay for the document
    ///
    /// # Returns
    ///
    /// * `Self` - The new builder instance
    pub fn new(
        data_contract: Arc<DataContract>,
        document_type_name: String,
        document: Document,
        purchaser_id: Identifier,
        price: Credits,
    ) -> Self {
        Self {
            data_contract,
            document_type_name,
            document,
            purchaser_id,
            price,
            token_payment_info: None,
            settings: None,
            user_fee_increase: None,
            state_transition_creation_options: None,
        }
    }

    /// Creates a new builder from document ID
    ///
    /// # Arguments
    ///
    /// * `data_contract` - The data contract
    /// * `document_type_name` - The name of the document type
    /// * `document_id` - The ID of the document
    /// * `current_owner_id` - The current owner ID of the document
    /// * `purchaser_id` - The identifier of the purchaser
    /// * `price` - The price to pay for the document
    ///
    /// # Returns
    ///
    /// * `Self` - The new builder instance
    pub fn from_document_info(
        data_contract: Arc<DataContract>,
        document_type_name: String,
        document_id: Identifier,
        current_owner_id: Identifier,
        purchaser_id: Identifier,
        price: Credits,
    ) -> Self {
        // Create a minimal document with just the required fields
        // The actual document will be fetched during the transition
        let document = Document::V0(dpp::document::DocumentV0 {
            id: document_id,
            owner_id: current_owner_id,
            properties: Default::default(),
            revision: Some(1), // Will be updated during transition
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

        Self::new(
            data_contract,
            document_type_name,
            document,
            purchaser_id,
            price,
        )
    }

    /// Adds token payment info to the document purchase transition
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

    /// Adds a user fee increase to the document purchase transition
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

    /// Adds settings to the document purchase transition
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

    /// Adds creation_options to the document purchase transition
    ///
    /// # Arguments
    ///
    /// * `creation_options` - The creation options to add
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

    /// Signs the document purchase transition
    ///
    /// # Arguments
    ///
    /// * `sdk` - The SDK instance
    /// * `identity_public_key` - The public key of the identity
    /// * `signer` - The signer instance
    /// * `platform_version` - The platform version
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
                self.purchaser_id,
                self.data_contract.id(),
                true,
                self.settings,
            )
            .await?;

        let document_type = self
            .data_contract
            .document_type_for_name(&self.document_type_name)
            .map_err(|e| Error::Protocol(e.into()))?;

        let state_transition = BatchTransition::new_document_purchase_transition_from_document(
            self.document.clone(),
            document_type,
            self.purchaser_id,
            self.price,
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

/// Result types returned from document purchase operations.
#[derive(Debug)]
pub enum DocumentPurchaseResult {
    /// Document purchased successfully, containing the document with new owner.
    Document(Document),
}

impl Sdk {
    /// Purchases a document from its current owner.
    ///
    /// This method broadcasts a document purchase transition to buy a document
    /// at its set price and transfer ownership to the purchaser. The result
    /// contains the purchased document with updated ownership.
    ///
    /// # Arguments
    ///
    /// * `purchase_document_transition_builder` - Builder containing purchase parameters
    /// * `signing_key` - The identity public key for signing the transition
    /// * `signer` - Implementation of the Signer trait for cryptographic signing
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a `DocumentPurchaseResult` on success, or an `Error` on failure.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The transition signing fails
    /// - Broadcasting the transition fails
    /// - The proof verification returns an unexpected result type
    /// - Document not found or not for sale
    /// - Insufficient funds to complete the purchase
    /// - Price mismatch (document price changed)
    /// - Invalid purchaser identity
    pub async fn document_purchase<S: Signer>(
        &self,
        purchase_document_transition_builder: DocumentPurchaseTransitionBuilder,
        signing_key: &IdentityPublicKey,
        signer: &S,
    ) -> Result<DocumentPurchaseResult, Error> {
        let platform_version = self.version();

        let put_settings = purchase_document_transition_builder.settings;

        let state_transition = purchase_document_transition_builder
            .sign(self, signing_key, signer, platform_version)
            .await?;

        let proof_result = state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(self, put_settings)
            .await?;

        match proof_result {
            StateTransitionProofResult::VerifiedDocuments(documents) => {
                if let Some((_, Some(document))) = documents.into_iter().next() {
                    Ok(DocumentPurchaseResult::Document(document))
                } else {
                    Err(Error::DriveProofError(
                        drive::error::proof::ProofError::UnexpectedResultProof(
                            "Expected document in VerifiedDocuments result for purchase transition"
                                .to_string(),
                        ),
                        vec![],
                        Default::default(),
                    ))
                }
            }
            _ => Err(Error::DriveProofError(
                drive::error::proof::ProofError::UnexpectedResultProof(
                    "Expected VerifiedDocuments for document purchase transition".to_string(),
                ),
                vec![],
                Default::default(),
            )),
        }
    }
}
