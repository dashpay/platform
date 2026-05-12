use crate::platform::transition::broadcast::BroadcastStateTransition;
use crate::platform::transition::put_settings::PutSettings;
use crate::platform::transition::validation::ensure_valid_state_transition_structure;
use crate::platform::Identifier;
use crate::{Error, Sdk};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::DataContract;
use dpp::document::{Document, INITIAL_REVISION};
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::prelude::IdentityNonce;
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

    /// Adds settings to the document delete transition.
    ///
    /// Explicit setters always win regardless of call order: if
    /// [`Self::with_user_fee_increase`] or
    /// [`Self::with_state_transition_creation_options`] has already been
    /// called on this builder, the corresponding field in `settings` is
    /// only used as a fallback when the dedicated builder field is still
    /// `None`. This makes the builder order-independent for these two
    /// fields and avoids silently clobbering a deliberate caller choice.
    ///
    /// # Arguments
    ///
    /// * `settings` - The settings to add
    ///
    /// # Returns
    ///
    /// * `Self` - The updated builder
    pub fn with_settings(mut self, settings: PutSettings) -> Self {
        if self.user_fee_increase.is_none() {
            if let Some(user_fee_increase) = settings.user_fee_increase {
                self.user_fee_increase = Some(user_fee_increase);
            }
        }
        if self.state_transition_creation_options.is_none() {
            if let Some(state_transition_creation_options) =
                settings.state_transition_creation_options
            {
                self.state_transition_creation_options = Some(state_transition_creation_options);
            }
        }
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

    /// Signs the document delete transition.
    ///
    /// Allocates a fresh identity-contract nonce from `sdk` and delegates to
    /// [`Self::sign_with_nonce`]. Use [`Self::sign_with_nonce`] directly if
    /// you need to pre-allocate the nonce so a pre-broadcast failure can roll
    /// it back via
    /// [`Sdk::rollback_identity_contract_nonce`](crate::Sdk::rollback_identity_contract_nonce).
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
        let identity_contract_nonce = sdk
            .get_identity_contract_nonce(
                self.owner_id,
                self.data_contract.id(),
                true,
                self.settings,
            )
            .await?;

        self.sign_with_nonce(
            identity_contract_nonce,
            identity_public_key,
            signer,
            platform_version,
        )
        .await
    }

    /// Signs the document delete transition using a pre-allocated
    /// identity-contract nonce.
    ///
    /// This variant lets the caller separate nonce allocation from signing so
    /// pre-broadcast failures can be rolled back by calling
    /// [`Sdk::rollback_identity_contract_nonce`](crate::Sdk::rollback_identity_contract_nonce)
    /// with the same `identity_contract_nonce`. The caller is responsible for
    /// having obtained the nonce via
    /// [`Sdk::get_identity_contract_nonce`](crate::Sdk::get_identity_contract_nonce)
    /// with `bump_first = true` for the same `(owner_id, contract_id)` pair.
    pub async fn sign_with_nonce(
        &self,
        identity_contract_nonce: IdentityNonce,
        identity_public_key: &IdentityPublicKey,
        signer: &impl Signer<IdentityPublicKey>,
        platform_version: &PlatformVersion,
    ) -> Result<StateTransition, Error> {
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
            creator_id: None,
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

/// Build, sign, and structurally validate a document **delete**
/// [`StateTransition`] without broadcasting it.
///
/// This is the shared pre-broadcast core used by [`Sdk::document_delete`]
/// and the wasm-sdk / FFI prepare-document-delete paths so the
/// nonce-allocate / sign / validate / rollback sequence is implemented in
/// exactly one place.
///
/// Concretely, this helper:
///
/// - allocates a fresh identity-contract nonce via
///   [`Sdk::get_identity_contract_nonce`] with `bump_first = true`,
/// - signs the transition by delegating to
///   [`DocumentDeleteTransitionBuilder::sign_with_nonce`],
/// - runs structure validation via
///   [`ensure_valid_state_transition_structure`], and
/// - on any **pre-broadcast** error (sign/build or local structure
///   validation) conditionally rolls the bumped identity-contract nonce
///   back via [`Sdk::rollback_identity_contract_nonce`], so the local
///   cache does not advance past a nonce the network never observed.
///
/// Errors that occur **after** this helper returns successfully (e.g.
/// serialization failures in callers) are not rolled back here.
pub async fn build_signed_document_delete_transition<S: Signer<IdentityPublicKey>>(
    sdk: &Sdk,
    builder: &DocumentDeleteTransitionBuilder,
    identity_public_key: &IdentityPublicKey,
    signer: &S,
) -> Result<StateTransition, Error> {
    let owner_id = builder.owner_id;
    let contract_id = builder.data_contract.id();

    let identity_contract_nonce = sdk
        .get_identity_contract_nonce(owner_id, contract_id, true, builder.settings)
        .await?;

    let state_transition = match builder
        .sign_with_nonce(
            identity_contract_nonce,
            identity_public_key,
            signer,
            sdk.version(),
        )
        .await
    {
        Ok(transition) => transition,
        Err(err) => {
            sdk.rollback_identity_contract_nonce(owner_id, contract_id, identity_contract_nonce)
                .await;
            return Err(err);
        }
    };

    if let Err(err) = ensure_valid_state_transition_structure(&state_transition, sdk.version()) {
        sdk.rollback_identity_contract_nonce(owner_id, contract_id, identity_contract_nonce)
            .await;
        return Err(err);
    }

    Ok(state_transition)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::transition::put_settings::PutSettings;
    use dpp::state_transition::batch_transition::methods::StateTransitionCreationOptions;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use dpp::version::PlatformVersion;

    #[test]
    fn with_settings_propagates_signed_transition_fields() {
        let settings = PutSettings {
            user_fee_increase: Some(42),
            state_transition_creation_options: Some(StateTransitionCreationOptions::default()),
            ..Default::default()
        };
        let data_contract = get_data_contract_fixture(
            None,
            Default::default(),
            PlatformVersion::latest().protocol_version,
        )
        .data_contract_owned();

        let builder = DocumentDeleteTransitionBuilder::new(
            Arc::new(data_contract),
            "niceDocument".to_string(),
            Identifier::default(),
            Identifier::default(),
        )
        .with_settings(settings);

        assert_eq!(
            builder.settings.map(|settings| settings.user_fee_increase),
            Some(settings.user_fee_increase)
        );
        assert_eq!(builder.user_fee_increase, settings.user_fee_increase);
        assert_eq!(
            builder.state_transition_creation_options,
            settings.state_transition_creation_options
        );
    }

    #[test]
    fn with_settings_does_not_overwrite_explicit_user_fee_increase() {
        let settings_with_seven = PutSettings {
            user_fee_increase: Some(7),
            ..Default::default()
        };
        let data_contract = Arc::new(
            get_data_contract_fixture(
                None,
                Default::default(),
                PlatformVersion::latest().protocol_version,
            )
            .data_contract_owned(),
        );

        let builder = DocumentDeleteTransitionBuilder::new(
            data_contract,
            "niceDocument".to_string(),
            Identifier::default(),
            Identifier::default(),
        )
        .with_user_fee_increase(42)
        .with_settings(settings_with_seven);

        assert_eq!(
            builder.user_fee_increase,
            Some(42),
            "explicit with_user_fee_increase(42) must win over settings.user_fee_increase = Some(7)"
        );
    }

    #[test]
    fn with_settings_does_not_overwrite_explicit_state_transition_creation_options() {
        let explicit_options = StateTransitionCreationOptions {
            batch_feature_version: Some(2),
            ..Default::default()
        };
        let settings_options = StateTransitionCreationOptions {
            batch_feature_version: Some(7),
            ..Default::default()
        };
        assert_ne!(
            explicit_options, settings_options,
            "test precondition: explicit and settings options must differ to prove which one wins"
        );
        let settings_with_options = PutSettings {
            state_transition_creation_options: Some(settings_options),
            ..Default::default()
        };
        let data_contract = Arc::new(
            get_data_contract_fixture(
                None,
                Default::default(),
                PlatformVersion::latest().protocol_version,
            )
            .data_contract_owned(),
        );

        let builder = DocumentDeleteTransitionBuilder::new(
            data_contract,
            "niceDocument".to_string(),
            Identifier::default(),
            Identifier::default(),
        )
        .with_state_transition_creation_options(explicit_options)
        .with_settings(settings_with_options);

        assert_eq!(
            builder.state_transition_creation_options,
            Some(explicit_options),
            "explicit with_state_transition_creation_options must win over settings value"
        );
    }

    #[test]
    fn with_settings_preserves_explicit_fields_when_settings_values_are_none() {
        let explicit_creation_options = StateTransitionCreationOptions::default();
        let builder = DocumentDeleteTransitionBuilder::new(
            Arc::new(
                get_data_contract_fixture(
                    None,
                    Default::default(),
                    PlatformVersion::latest().protocol_version,
                )
                .data_contract_owned(),
            ),
            "niceDocument".to_string(),
            Identifier::default(),
            Identifier::default(),
        )
        .with_user_fee_increase(42)
        .with_state_transition_creation_options(explicit_creation_options)
        .with_settings(PutSettings {
            user_fee_increase: None,
            state_transition_creation_options: None,
            ..Default::default()
        });

        assert_eq!(builder.user_fee_increase, Some(42));
        assert_eq!(
            builder.state_transition_creation_options,
            Some(explicit_creation_options)
        );
        assert!(builder.settings.is_some());
        assert_eq!(
            builder
                .settings
                .as_ref()
                .and_then(|settings| settings.user_fee_increase),
            None
        );
        assert_eq!(
            builder
                .settings
                .as_ref()
                .and_then(|settings| settings.state_transition_creation_options),
            None
        );
    }

    /// Failing-signer used by the rollback test below to deterministically
    /// fail signing **after** nonce allocation. Mirrors the pattern in
    /// `put_document.rs` so a future reader can map the two.
    #[derive(Debug)]
    struct AlwaysFailingSigner;

    #[async_trait::async_trait]
    impl dpp::identity::signer::Signer<IdentityPublicKey> for AlwaysFailingSigner {
        async fn sign(
            &self,
            _key: &IdentityPublicKey,
            _data: &[u8],
        ) -> Result<dpp::platform_value::BinaryData, dpp::ProtocolError> {
            Err(dpp::ProtocolError::Generic(
                "deliberate signing failure for delete rollback test".to_string(),
            ))
        }

        async fn sign_create_witness(
            &self,
            _key: &IdentityPublicKey,
            _data: &[u8],
        ) -> Result<dpp::address_funds::AddressWitness, dpp::ProtocolError> {
            unreachable!("not used by document delete transition signing")
        }

        fn can_sign_with(&self, _key: &IdentityPublicKey) -> bool {
            true
        }
    }

    /// Pre-broadcast signing failure inside `Sdk::document_delete` must
    /// roll the identity-contract nonce back so the cache does not advance
    /// past a nonce the network never observed. Asserting via "next
    /// allocation reuses the rolled-back value" mirrors the put_document
    /// rollback test pattern.
    #[tokio::test]
    async fn document_delete_rolls_back_nonce_on_signing_failure() {
        use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
        use dpp::identity::identity_public_key::{KeyType, Purpose, SecurityLevel};
        use dpp::platform_value::BinaryData;
        use drive_proof_verifier::types::IdentityContractNonceFetcher;

        let data_contract = Arc::new(
            get_data_contract_fixture(
                None,
                Default::default(),
                PlatformVersion::latest().protocol_version,
            )
            .data_contract_owned(),
        );
        let contract_id = data_contract.id();
        let owner_id = Identifier::from([7u8; 32]);
        let document_id = Identifier::from([3u8; 32]);

        // Build a key whose purpose / security level / enabled flag pass the
        // BatchTransition pre-sign verification, so the failure happens
        // inside `signer.sign` (i.e. *after* nonce allocation), not earlier.
        let identity_key = IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: 0,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: BinaryData::new(vec![0u8; 33]),
            disabled_at: None,
        });

        let mut sdk = crate::Sdk::new_mock();
        sdk.mock()
            .expect_fetch::<IdentityContractNonceFetcher, _>(
                (owner_id, contract_id),
                Some(IdentityContractNonceFetcher(10u64)),
            )
            .await
            .expect("set IdentityContractNonceFetcher mock expectation");

        let builder = DocumentDeleteTransitionBuilder::new(
            data_contract.clone(),
            "niceDocument".to_string(),
            document_id,
            owner_id,
        );

        let signer = AlwaysFailingSigner;

        let err = sdk
            .document_delete(builder, &identity_key, &signer)
            .await
            .expect_err(
                "signer failure must surface so document_delete can roll back the allocated nonce",
            );

        assert!(
            err.to_string().contains("deliberate signing failure"),
            "expected the signer's failure to surface, got: {err}"
        );

        // Cache was bumped from platform=10 to 11 during the failed attempt
        // and then rolled back to 10. Re-allocating with bump_first=true
        // must yield 11 again — proving the rolled-back nonce is reusable.
        let next = sdk
            .get_identity_contract_nonce(owner_id, contract_id, true, None)
            .await
            .expect("nonce allocation must succeed after rollback");
        assert_eq!(
            next, 11,
            "rolled-back nonce should be reused by the next allocation"
        );
    }
}

impl Sdk {
    /// Deletes an existing document from the platform.
    ///
    /// This method broadcasts a document deletion transition to permanently remove
    /// a document from the platform. The result confirms the deletion.
    ///
    /// # Nonce handling on local errors
    ///
    /// The identity-contract nonce is allocated explicitly before signing so
    /// that **pre-broadcast** failures (sign/build error or local structure
    /// validation error) can be rolled back via
    /// [`Sdk::rollback_identity_contract_nonce`]. The local cache therefore
    /// does not advance past a nonce the network never observed. Broadcast
    /// failures are not rolled back here; they continue to rely on the
    /// existing [`broadcast_and_wait`](crate::platform::transition::broadcast::BroadcastStateTransition::broadcast_and_wait)
    /// refresh behavior because the network may already have observed the
    /// nonce.
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
        let put_settings = delete_document_transition_builder.settings;

        // Build/sign/validate via the shared helper so the nonce
        // allocate/sign/validate/rollback sequence stays in one place.
        let state_transition = build_signed_document_delete_transition(
            self,
            &delete_document_transition_builder,
            signing_key,
            signer,
        )
        .await?;

        // Broadcast: do NOT roll back on broadcast failure — the network may
        // already have observed the nonce. broadcast_and_wait keeps the
        // existing refresh behavior on its own failures.
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
