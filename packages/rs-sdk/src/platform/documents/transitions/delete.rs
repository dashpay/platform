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
    pub data_contract: Arc<DataContract>,
    pub document_type_name: String,
    pub document_id: Identifier,
    pub owner_id: Identifier,
    /// The full document, when the builder was constructed from one.
    /// Required for indexOnly document types: their delete transition
    /// carries the document's values (there is no stored row to fetch
    /// them from), so an id-only builder cannot delete them.
    pub document: Option<Document>,
    pub token_payment_info: Option<TokenPaymentInfo>,
    pub settings: Option<PutSettings>,
    pub user_fee_increase: Option<UserFeeIncrease>,
    pub state_transition_creation_options: Option<StateTransitionCreationOptions>,
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
            document: None,
            token_payment_info: None,
            settings: None,
            user_fee_increase: None,
            state_transition_creation_options: None,
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
        let mut builder = Self::new(
            data_contract,
            document_type_name,
            document.id(),
            document.owner_id(),
        );
        // Keep the full document: an indexOnly delete carries its values.
        builder.document = Some(document.clone());
        builder
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

    /// Adds creation_options to the document delete transition
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

    /// Signs the document delete transition
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
    ///
    /// Resolve the document type and the document this delete will be
    /// built from, validating the builder's target. Runs BEFORE any nonce
    /// is reserved (an invalid builder must not advance the SDK's cached
    /// nonce — enough poisoned increments would push the next valid
    /// transition beyond the protocol's missing-revision window):
    /// an id-only builder is refused for indexOnly types, and a stored
    /// full document must agree with the builder's public id/owner fields.
    fn resolve_document_for_deletion(
        &self,
    ) -> Result<
        (
            dpp::data_contract::document_type::DocumentTypeRef<'_>,
            Document,
        ),
        Error,
    > {
        use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
        use dpp::document::DocumentV0Getters;

        let document_type = self
            .data_contract
            .document_type_for_name(&self.document_type_name)
            .map_err(|e| Error::Protocol(e.into()))?;

        if let Some(document) = &self.document {
            // The id/owner fields stay public alongside the stored
            // document; refuse a contradictory target instead of
            // reserving a nonce for one identity while signing a
            // transition derived from another.
            if document.id() != self.document_id || document.owner_id() != self.owner_id {
                return Err(Error::Generic(
                    "the builder's document_id/owner_id do not match the stored document: \
                     the transition is derived from the document, so a mismatched target \
                     would sign for a different identity or document than the fields claim"
                        .to_string(),
                ));
            }
            return Ok((document_type, document.clone()));
        }

        // indexOnly deletes carry the document's values — an id-only
        // builder has nothing to carry, so demand the full document.
        if document_type.index_only() {
            return Err(Error::Generic(
                "deleting an indexOnly document requires the full document (its values \
                 are what identify the entries): construct the builder with \
                 DocumentDeleteTransitionBuilder::from_document"
                    .to_string(),
            ));
        }

        // A minimal id-only document is all a stored-document (by-id)
        // delete needs.
        let document = Document::V0(dpp::document::DocumentV0 {
            contract_version: None,
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
            creator_id: None,
        });
        Ok((document_type, document))
    }

    pub async fn sign(
        &self,
        sdk: &Sdk,
        identity_public_key: &IdentityPublicKey,
        signer: &impl Signer<IdentityPublicKey>,
        platform_version: &PlatformVersion,
    ) -> Result<StateTransition, Error> {
        // Validate the target FIRST: the nonce fetch below bumps the
        // SDK's cached contract nonce, and no transition is broadcast on
        // an error path, so a rejection after it would leak an increment
        // per failed call.
        let (document_type, document) = self.resolve_document_for_deletion()?;

        let identity_contract_nonce = sdk
            .get_identity_contract_nonce(
                self.owner_id,
                self.data_contract.id(),
                true,
                self.settings,
            )
            .await?;

        let state_transition = BatchTransition::new_document_deletion_transition_from_document(
            document,
            document_type,
            identity_public_key,
            identity_contract_nonce,
            self.user_fee_increase.unwrap_or_default(),
            self.token_payment_info,
            signer,
            platform_version,
            self.state_transition_creation_options,
        )
        .await?;

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
    pub async fn document_delete<S: Signer<IdentityPublicKey>>(
        &self,
        delete_document_transition_builder: DocumentDeleteTransitionBuilder,
        signing_key: &IdentityPublicKey,
        signer: &S,
    ) -> Result<DocumentDeleteResult, Error> {
        let platform_version = self.version();

        let put_settings = delete_document_transition_builder.settings;

        let state_transition = delete_document_transition_builder
            .sign(self, signing_key, signer, platform_version)
            .await?;

        let proof_result = state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(self, put_settings)
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

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::document::{DocumentV0Getters, DocumentV0Setters};
    use dpp::platform_value::Value;
    use dpp::tests::json_document::json_document_to_contract;

    const YAPPR_LIKES_CONTRACT: &str =
        "../rs-drive/tests/supporting_files/contract/yappr-likes/yappr-likes-contract.json";

    fn likes_contract() -> Arc<DataContract> {
        Arc::new(
            json_document_to_contract(YAPPR_LIKES_CONTRACT, true, PlatformVersion::latest())
                .expect("expected to parse the yappr-likes contract"),
        )
    }

    fn like_document(contract: &DataContract) -> Document {
        use dpp::data_contract::document_type::random_document::CreateRandomDocument;
        let like_type = contract
            .document_type_for_name("like")
            .expect("like doctype exists");
        let mut document = like_type
            .random_document(Some(1337), PlatformVersion::latest())
            .expect("expected a random like");
        document.set("hashtag", "dash".into());
        document.set(
            "postId",
            Value::Identifier(Identifier::new([0xe3; 32]).to_buffer()),
        );
        document
    }

    /// The refusal must come out of the pure resolution step — before
    /// `sign()` ever reaches the nonce cache, so a rejected call cannot
    /// advance the cached contract nonce.
    #[test]
    fn should_refuse_an_id_only_delete_of_an_index_only_type_before_any_nonce_work() {
        let contract = likes_contract();
        let builder = DocumentDeleteTransitionBuilder::new(
            Arc::clone(&contract),
            "like".to_string(),
            Identifier::new([1u8; 32]),
            Identifier::new([2u8; 32]),
        );
        let error = builder
            .resolve_document_for_deletion()
            .expect_err("an id-only builder must be refused for an indexOnly type");
        assert!(
            error.to_string().contains("requires the full document"),
            "unexpected error: {error}"
        );
    }

    /// A stored full document must agree with the builder's public
    /// id/owner fields — a contradictory target must not resolve.
    #[test]
    fn should_refuse_a_document_target_that_contradicts_the_builder_fields() {
        let contract = likes_contract();
        let like = like_document(&contract);
        let mut builder = DocumentDeleteTransitionBuilder::from_document(
            Arc::clone(&contract),
            "like".to_string(),
            &like,
        );
        builder.document_id = Identifier::new([9u8; 32]);
        let error = builder
            .resolve_document_for_deletion()
            .expect_err("a mismatched target must be refused");
        assert!(
            error.to_string().contains("do not match"),
            "unexpected error: {error}"
        );
    }

    /// `from_document` keeps the full value tuple: the resolved document
    /// is the one the batch factory derives the indexOnlyDelete kind's
    /// payload from.
    #[test]
    fn should_keep_the_full_document_for_index_only_deletes() {
        let contract = likes_contract();
        let like = like_document(&contract);
        let builder = DocumentDeleteTransitionBuilder::from_document(
            Arc::clone(&contract),
            "like".to_string(),
            &like,
        );
        let (document_type, resolved) = builder
            .resolve_document_for_deletion()
            .expect("a full-document builder must resolve");
        use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
        assert_eq!(document_type.name(), "like");
        assert_eq!(resolved.properties(), like.properties());
        assert_eq!(resolved.id(), like.id());
        assert_eq!(resolved.owner_id(), like.owner_id());
    }

    /// Stored doctypes keep the id-only path: the resolved document is the
    /// minimal by-id shell.
    #[test]
    fn should_resolve_a_minimal_document_for_stored_type_id_only_deletes() {
        let contract = likes_contract();
        let builder = DocumentDeleteTransitionBuilder::new(
            Arc::clone(&contract),
            "post".to_string(),
            Identifier::new([1u8; 32]),
            Identifier::new([2u8; 32]),
        );
        let (_, resolved) = builder
            .resolve_document_for_deletion()
            .expect("an id-only builder must resolve for a stored type");
        assert!(resolved.properties().is_empty());
        assert_eq!(resolved.id(), Identifier::new([1u8; 32]));
    }
}
