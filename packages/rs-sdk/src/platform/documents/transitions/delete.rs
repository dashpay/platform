use crate::platform::transition::broadcast::BroadcastStateTransition;
use crate::platform::transition::put_settings::PutSettings;
use crate::platform::Identifier;
use crate::{Error, Sdk};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters};
use dpp::data_contract::document_type::action_fees::agreement::DocumentActionFeeAgreement;
use dpp::data_contract::document_type::methods::{DocumentTypeBasicMethods, DocumentTypeV0Methods};
use dpp::data_contract::document_type::{
    index_only_row_commits_created_at, DocumentProperty, DocumentPropertyType, DocumentTypeRef,
};
use dpp::data_contract::DataContract;
use dpp::document::{Document, INITIAL_REVISION};
use dpp::document::{DocumentV0, DocumentV0Getters, DocumentV0Setters};
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::platform_value::Value;
use dpp::prelude::TimestampMillis;
use dpp::prelude::UserFeeIncrease;
use dpp::state_transition::batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
use dpp::state_transition::batch_transition::methods::StateTransitionCreationOptions;
use dpp::state_transition::batch_transition::BatchTransition;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::state_transition::StateTransition;
use dpp::tokens::token_payment_info::TokenPaymentInfo;
use dpp::version::PlatformVersion;
use std::collections::BTreeMap;
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
        let mut builder = Self::new(
            data_contract,
            document_type_name,
            document.id(),
            document.owner_id(),
        );
        // Keep the full document: an indexOnly delete carries its values. A
        // property only a `summableOffCountIndex` index's source fixes sits in
        // no entry-keeping index, so a document read back lacks it: set it
        // from the referenced document before passing the document here.
        builder.document = Some(document.clone());
        builder
    }

    /// Starts a delete of an `indexOnly` document from the values it was
    /// created with: see [`index_only_delete_document`], which builds the
    /// document and refuses what Platform would only refuse after the
    /// transition is broadcast and paid for.
    pub fn from_values(
        data_contract: Arc<DataContract>,
        document_type_name: String,
        document_id: Identifier,
        owner_id: Identifier,
        values: BTreeMap<String, Value>,
        created_at: Option<TimestampMillis>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        let document_type = data_contract
            .document_type_for_name(&document_type_name)
            .map_err(|e| Error::Protocol(e.into()))?;
        let document = index_only_delete_document(
            document_type,
            document_id,
            owner_id,
            values,
            created_at,
            platform_version,
        )?;
        let mut builder = Self::new(data_contract, document_type_name, document_id, owner_id);
        builder.document = Some(document);
        Ok(builder)
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

    /// Adds what the document delete transition agrees to pay in action fees. Required when
    /// the document type charges a fee for the action: build it from the contract the user was
    /// shown with `DocumentActionFeeAgreement::for_document_type_action`, so that a fee changed
    /// since is refused instead of paid.
    ///
    /// Call it after `with_state_transition_creation_options`, which replaces the options this
    /// is kept in.
    ///
    /// # Arguments
    ///
    /// * `action_fee_agreement` - The action fee agreement to add
    ///
    /// # Returns
    ///
    /// * `Self` - The updated builder
    pub fn with_action_fee_agreement(
        mut self,
        action_fee_agreement: DocumentActionFeeAgreement,
    ) -> Self {
        self.state_transition_creation_options
            .get_or_insert_with(Default::default)
            .action_fee_agreement = Some(action_fee_agreement);
        self
    }

    /// Resolve the document type and the document this delete will be
    /// built from, validating the builder's target. Runs BEFORE any nonce
    /// is reserved (an invalid builder must not advance the SDK's cached
    /// nonce — enough poisoned increments would push the next valid
    /// transition beyond the protocol's missing-revision window):
    /// an id-only builder is refused for indexOnly types, and a stored
    /// full document must agree with the builder's public id/owner fields.
    fn resolve_document_for_deletion(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<(DocumentTypeRef<'_>, Document), Error> {
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
            // An indexOnly delete is derived from these values: what
            // Platform would refuse in them is refused here, before the
            // nonce is reserved.
            if document_type.index_only() {
                check_index_only_delete(
                    document_type,
                    document.properties(),
                    document.created_at(),
                    platform_version,
                )?;
            }
            return Ok((document_type, document.clone()));
        }

        // indexOnly deletes carry the document's values — an id-only
        // builder has nothing to carry, so demand the full document.
        if document_type.index_only() {
            return Err(Error::Generic(
                "deleting an indexOnly document requires the full document (its values \
                 are what identify the entries): construct the builder with \
                 DocumentDeleteTransitionBuilder::from_values, or from_document"
                    .to_string(),
            ));
        }

        // A minimal id-only document is all a stored-document (by-id)
        // delete needs.
        let document = Document::V0(DocumentV0 {
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
            moderated_at: None,
            moderated_by: None,
        });
        Ok((document_type, document))
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
        let (document_type, document) = self.resolve_document_for_deletion(platform_version)?;

        // A local failure after the nonce is reserved would leave the cached nonce ahead of
        // Platform's, so what can be refused without it is refused first.
        if let Some(creation_options) = &self.state_transition_creation_options {
            creation_options.validate_base_carries_action_fee_agreement(platform_version)?;
        }

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
            self.token_payment_info.clone(),
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
    /// a document from the platform. The result confirms the deletion. For an
    /// indexOnly document type the proof shows the document's entry gone at the
    /// proof's block, not that this delete removed it: no stronger proof exists
    /// for such a document.
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
        let index_only = delete_document_transition_builder
            .data_contract
            .document_type_for_name(&delete_document_transition_builder.document_type_name)
            .map_err(|e| Error::Protocol(e.into()))?
            .index_only();

        let state_transition = delete_document_transition_builder
            .sign(self, signing_key, signer, platform_version)
            .await?;

        // An indexOnly document keeps no row: its proof shows the entry is gone, which it would
        // also be had it never existed, so it cannot prove that this delete executed. That is the
        // strongest proof such a document has, so it is accepted rather than failing a delete
        // that landed.
        let proof_result = if index_only {
            state_transition
                .broadcast_and_wait_for_affected_state::<StateTransitionProofResult>(
                    self,
                    put_settings,
                )
                .await?
        } else {
            state_transition
                .broadcast_and_wait::<StateTransitionProofResult>(self, put_settings)
                .await?
        };

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

/// The document an `indexOnly` delete carries, built from the values the
/// document was created with: `values` are the document type's own
/// properties, as create takes them (byte arrays and identifiers may still
/// be in their JSON text form; they are converted the way create converts
/// them), `document_id` the id its create returned, and `created_at` its
/// `$createdAt`, given only when it has one — not among `values`, unlike
/// `Drive`'s index-only document helper.
///
/// Refused before anything is signed, instead of after a paid broadcast: a
/// type that is not `indexOnly` (its delete goes by id and does not look at
/// the values) or whose documents can not be deleted, a value the type does
/// not declare (also inside a declared object; a `$` name is never
/// declared), a missing required property (looked up by path, with every
/// `generatedFrom` property generated as the delete will carry it), and a
/// missing `created_at` on a type whose entries commit to it. A value of the
/// wrong shape is left for Platform to refuse.
pub fn index_only_delete_document(
    document_type: DocumentTypeRef,
    document_id: Identifier,
    owner_id: Identifier,
    mut values: BTreeMap<String, Value>,
    created_at: Option<TimestampMillis>,
    platform_version: &PlatformVersion,
) -> Result<Document, Error> {
    if !document_type.index_only() {
        return Err(Error::Generic(format!(
            "Document type {:?} is not indexOnly: its documents are deleted by id",
            document_type.name()
        )));
    }
    document_type.sanitize_document_properties(&mut values);
    check_index_only_delete(document_type, &values, created_at, platform_version)?;
    let mut document = document_type
        .create_document_from_data(values.into(), owner_id, 0, 0, [0u8; 32], platform_version)
        .map_err(|e| Error::Generic(format!("Failed to build document: {e}")))?;
    document.set_id(document_id);
    // `create_document_from_data` stamps the current time on a type that
    // requires one. A delete names the stored entry, so it carries the
    // caller's value or none.
    document.set_created_at(created_at);
    Ok(document)
}

/// What Platform would refuse, after a paid broadcast, in the values an
/// indexOnly delete carries; see [`index_only_delete_document`].
fn check_index_only_delete(
    document_type: DocumentTypeRef,
    values: &BTreeMap<String, Value>,
    created_at: Option<TimestampMillis>,
    platform_version: &PlatformVersion,
) -> Result<(), Error> {
    let name = document_type.name();
    if !document_type.documents_can_be_deleted() {
        return Err(Error::Generic(format!(
            "Documents of type {name:?} can not be deleted"
        )));
    }
    if let Some(path) = undeclared_property(
        &mut values.iter().map(|(name, value)| (name.as_str(), value)),
        &|name| document_type.properties().get(name),
    ) {
        return Err(Error::Generic(format!(
            "Document type {name:?} has no property {path:?} to delete by"
        )));
    }
    if created_at.is_none()
        && index_only_row_commits_created_at(
            document_type.required_fields(),
            document_type.index_structure(),
        )
    {
        return Err(Error::Generic(format!(
            "Document type {name:?} commits to $createdAt: the delete needs the value the \
             document was created with"
        )));
    }
    let mut carried = values.clone();
    document_type
        .regenerate_generated_properties(&mut carried, platform_version)
        .map_err(|e| Error::Generic(format!("Failed to generate document properties: {e}")))?;
    let carried = Value::from(carried);
    if let Some(path) = document_type.required_fields().iter().find(|path| {
        !path.starts_with('$') && matches!(carried.get_optional_value_at_path(path), Ok(None))
    }) {
        return Err(Error::Generic(format!(
            "Document type {name:?} requires {path:?}: the delete needs every value the \
             document was created with"
        )));
    }
    Ok(())
}

/// The path of the first value the schema does not declare, looking inside
/// declared objects. A `$` name is never declared. A value of the wrong
/// kind for its property (a map where the schema has no object, or the
/// other way round) is left for Platform to refuse.
fn undeclared_property<'a>(
    values: &mut dyn Iterator<Item = (&'a str, &'a Value)>,
    declared: &dyn Fn(&str) -> Option<&'a DocumentProperty>,
) -> Option<String> {
    for (name, value) in values {
        let Some(property) = declared(name).filter(|_| !name.starts_with('$')) else {
            return Some(name.to_string());
        };
        let (DocumentPropertyType::Object(members), Value::Map(entries)) =
            (&property.property_type, value)
        else {
            continue;
        };
        let mut nested = entries
            .iter()
            .filter_map(|(key, value)| key.as_text().map(|key| (key, value)));
        if let Some(path) = undeclared_property(&mut nested, &|name| members.get(name)) {
            return Some(format!("{name}.{path}"));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::data_contracts::SystemDataContract;
    use dpp::system_data_contracts::load_system_data_contract;
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

    #[test]
    fn should_build_an_index_only_delete_from_the_values_a_document_was_created_with() {
        let contract = likes_contract();
        let like = like_document(&contract);
        let builder = DocumentDeleteTransitionBuilder::from_values(
            Arc::clone(&contract),
            "like".to_string(),
            like.id(),
            like.owner_id(),
            like.properties().clone(),
            like.created_at(),
            PlatformVersion::latest(),
        )
        .expect("the values a like was created with build its delete");

        let document = builder
            .document
            .as_ref()
            .expect("the delete carries the document");
        assert_eq!(document.id(), like.id());
        assert_eq!(document.owner_id(), like.owner_id());
        assert_eq!(document.properties(), like.properties());
        assert_eq!(document.created_at(), like.created_at());
        builder
            .resolve_document_for_deletion(PlatformVersion::latest())
            .expect("the builder resolves to the document it was made from");
    }

    #[test]
    fn should_refuse_values_an_index_only_delete_could_not_carry() {
        let contract = likes_contract();
        let like = like_document(&contract);
        let like_type = contract
            .document_type_for_name("like")
            .expect("like doctype exists");
        let build = |values: BTreeMap<String, Value>| {
            index_only_delete_document(
                like_type,
                like.id(),
                like.owner_id(),
                values,
                like.created_at(),
                PlatformVersion::latest(),
            )
        };

        let mut undeclared = like.properties().clone();
        undeclared.insert("unknown".to_string(), Value::Bool(true));
        let error = build(undeclared).expect_err("an undeclared value is refused");
        assert!(
            error.to_string().contains("has no property \"unknown\""),
            "{error}"
        );

        let mut system = like.properties().clone();
        system.insert("$ownerId".to_string(), Value::Identifier([7u8; 32]));
        let error = build(system).expect_err("a system field among the values is refused");
        assert!(
            error.to_string().contains("has no property \"$ownerId\""),
            "{error}"
        );

        let required = like_type
            .required_fields()
            .iter()
            .find(|name| !name.starts_with('$') && like.properties().contains_key(*name))
            .expect("the like type has a required property")
            .clone();
        let mut partial = like.properties().clone();
        partial.remove(&required);
        let error = build(partial).expect_err("a missing required property is refused");
        assert!(
            error
                .to_string()
                .contains(&format!("requires {required:?}")),
            "{error}"
        );
    }

    #[test]
    fn should_refuse_values_for_a_type_that_is_not_index_only() {
        let contract =
            load_system_data_contract(SystemDataContract::DPNS, PlatformVersion::latest())
                .expect("dpns contract loads");
        let error = index_only_delete_document(
            contract
                .document_type_for_name("domain")
                .expect("domain type"),
            Identifier::new([1u8; 32]),
            Identifier::new([2u8; 32]),
            BTreeMap::new(),
            None,
            PlatformVersion::latest(),
        )
        .expect_err("a stored type is deleted by id");
        assert!(error.to_string().contains("is not indexOnly"), "{error}");
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
            .resolve_document_for_deletion(PlatformVersion::latest())
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
            .resolve_document_for_deletion(PlatformVersion::latest())
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
            .resolve_document_for_deletion(PlatformVersion::latest())
            .expect("a full-document builder must resolve");
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
            .resolve_document_for_deletion(PlatformVersion::latest())
            .expect("an id-only builder must resolve for a stored type");
        assert!(resolved.properties().is_empty());
        assert_eq!(resolved.id(), Identifier::new([1u8; 32]));
    }
}
