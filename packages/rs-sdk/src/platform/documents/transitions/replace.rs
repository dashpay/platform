use crate::platform::transition::broadcast::BroadcastStateTransition;
use crate::platform::transition::put_document::{
    build_signed_document_replace_transition, ensure_revision_for_replace,
};
use crate::platform::transition::put_settings::PutSettings;
use crate::platform::transition::validation::ensure_valid_state_transition_structure;
use crate::{Error, Sdk};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::DataContract;
use dpp::document::{Document, DocumentV0Getters};
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
use tracing::trace;

/// A builder to configure and broadcast document replace transitions
pub struct DocumentReplaceTransitionBuilder {
    pub data_contract: Arc<DataContract>,
    pub document_type_name: String,
    pub document: Document,
    pub token_payment_info: Option<TokenPaymentInfo>,
    pub settings: Option<PutSettings>,
    pub user_fee_increase: Option<UserFeeIncrease>,
    pub state_transition_creation_options: Option<StateTransitionCreationOptions>,
}

impl DocumentReplaceTransitionBuilder {
    /// Start building a replace document request for the provided DataContract.
    ///
    /// # Arguments
    ///
    /// * `data_contract` - The data contract
    /// * `document_type_name` - The name of the document type to replace
    /// * `document` - The document with updated values
    ///
    /// # Returns
    ///
    /// * `Self` - The new builder instance
    pub fn new(
        data_contract: Arc<DataContract>,
        document_type_name: String,
        document: Document,
    ) -> Self {
        Self {
            data_contract,
            document_type_name,
            document,
            token_payment_info: None,
            settings: None,
            user_fee_increase: None,
            state_transition_creation_options: None,
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

    /// Adds a user fee increase to the document replace transition.
    ///
    /// The dedicated [`Self::user_fee_increase`] field is the single source
    /// of truth for the effective value applied at sign time. Explicit
    /// setters always win regardless of call order — see
    /// [`Self::with_settings`] for the order-independence contract.
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

    /// Adds settings to the document replace transition.
    ///
    /// `user_fee_increase` and `state_transition_creation_options` are owned
    /// by their dedicated builder fields, which are the single source of
    /// truth for the effective values applied at sign time. This method
    /// extracts those two fields out of the supplied [`PutSettings`] into
    /// the dedicated fields **only if** the dedicated field is still
    /// `None`, and then stores the remainder of `settings` (with those two
    /// fields cleared) on the builder.
    ///
    /// Net effect: explicit setters always win over `with_settings` for
    /// `user_fee_increase` and `state_transition_creation_options`, regardless
    /// of call order. All other [`PutSettings`] fields (timeouts, retry
    /// behavior, nonce stale time, etc.) flow through unchanged to be used
    /// for nonce allocation and broadcast.
    ///
    /// # Arguments
    ///
    /// * `settings` - The settings to add
    ///
    /// # Returns
    ///
    /// * `Self` - The updated builder
    pub fn with_settings(mut self, mut settings: PutSettings) -> Self {
        if self.user_fee_increase.is_none() {
            self.user_fee_increase = settings.user_fee_increase;
        }
        if self.state_transition_creation_options.is_none() {
            self.state_transition_creation_options = settings.state_transition_creation_options;
        }
        // Strip the fee/creation-options fields from the stored settings so
        // the dedicated builder fields are the sole source of truth at
        // sign time. The remainder of `settings` flows through unchanged.
        settings.user_fee_increase = None;
        settings.state_transition_creation_options = None;
        self.settings = Some(settings);
        self
    }

    /// Adds creation_options to the document replace transition.
    ///
    /// The dedicated [`Self::state_transition_creation_options`] field is the
    /// single source of truth for the effective value applied at sign time.
    /// Explicit setters always win regardless of call order — see
    /// [`Self::with_settings`] for the order-independence contract.
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

    /// Signs the document replace transition.
    ///
    /// Allocates a fresh identity-contract nonce from `sdk` and delegates to
    /// [`Self::sign_with_nonce`]. If signing fails *after* the nonce has been
    /// allocated (e.g. the document type lookup or BatchTransition build
    /// fails), the bumped identity-contract nonce is conditionally rolled
    /// back via
    /// [`Sdk::rollback_identity_contract_nonce`](crate::Sdk::rollback_identity_contract_nonce)
    /// so the local cache does not advance past a nonce the network never
    /// observed.
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
        ensure_revision_for_replace(self.document.revision())?;

        let owner_id = self.document.owner_id();
        let contract_id = self.data_contract.id();
        let identity_contract_nonce = sdk
            .get_identity_contract_nonce(owner_id, contract_id, true, self.settings)
            .await?;

        match self
            .sign_with_nonce(
                identity_contract_nonce,
                identity_public_key,
                signer,
                platform_version,
            )
            .await
        {
            Ok(transition) => Ok(transition),
            Err(err) => {
                sdk.rollback_identity_contract_nonce(
                    owner_id,
                    contract_id,
                    identity_contract_nonce,
                )
                .await;
                Err(err)
            }
        }
    }

    /// Signs the document replace transition using a pre-allocated
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

        ensure_revision_for_replace(self.document.revision())?;

        let state_transition = BatchTransition::new_document_replacement_transition_from_document(
            self.document.clone(),
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

        ensure_valid_state_transition_structure(&state_transition, platform_version)?;

        Ok(state_transition)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::transition::put_settings::PutSettings;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::document::{DocumentV0, INITIAL_REVISION};
    use dpp::platform_value::Identifier as PVIdentifier;
    use dpp::state_transition::batch_transition::methods::StateTransitionCreationOptions;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use dpp::version::PlatformVersion;

    fn fixture_data_contract() -> Arc<DataContract> {
        Arc::new(
            get_data_contract_fixture(
                None,
                Default::default(),
                PlatformVersion::latest().protocol_version,
            )
            .data_contract_owned(),
        )
    }

    fn fixture_replace_document(contract: &DataContract) -> Document {
        Document::V0(DocumentV0 {
            id: PVIdentifier::from([1u8; 32]),
            owner_id: contract.owner_id(),
            properties: Default::default(),
            revision: Some(INITIAL_REVISION + 1),
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
        })
    }

    /// `with_settings` must extract `user_fee_increase` and
    /// `state_transition_creation_options` from the supplied `PutSettings`
    /// into the dedicated builder fields so `sign_with_nonce` (which reads
    /// only the dedicated fields) honors them on the wire.
    #[test]
    fn with_settings_extracts_fee_and_options_into_dedicated_fields() {
        let data_contract = fixture_data_contract();
        let document = fixture_replace_document(&data_contract);
        let settings = PutSettings {
            user_fee_increase: Some(42),
            state_transition_creation_options: Some(StateTransitionCreationOptions::default()),
            ..Default::default()
        };

        let builder = DocumentReplaceTransitionBuilder::new(
            data_contract,
            "niceDocument".to_string(),
            document,
        )
        .with_settings(settings);

        assert_eq!(builder.user_fee_increase, settings.user_fee_increase);
        assert_eq!(
            builder.state_transition_creation_options,
            settings.state_transition_creation_options
        );
        let stored = builder.settings.expect("settings must be stored");
        assert_eq!(stored.user_fee_increase, None);
        assert_eq!(stored.state_transition_creation_options, None);
    }

    /// Explicit setters must beat `with_settings` regardless of call order.
    #[test]
    fn explicit_setters_beat_settings_regardless_of_order() {
        let data_contract = fixture_data_contract();
        let document = fixture_replace_document(&data_contract);

        let settings_first = DocumentReplaceTransitionBuilder::new(
            data_contract.clone(),
            "niceDocument".to_string(),
            document.clone(),
        )
        .with_settings(PutSettings {
            user_fee_increase: Some(7),
            ..Default::default()
        })
        .with_user_fee_increase(42);

        let explicit_first = DocumentReplaceTransitionBuilder::new(
            data_contract,
            "niceDocument".to_string(),
            document,
        )
        .with_user_fee_increase(42)
        .with_settings(PutSettings {
            user_fee_increase: Some(7),
            ..Default::default()
        });

        assert_eq!(settings_first.user_fee_increase, Some(42));
        assert_eq!(explicit_first.user_fee_increase, Some(42));
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
    pub async fn document_replace<S: Signer<IdentityPublicKey>>(
        &self,
        replace_document_transition_builder: DocumentReplaceTransitionBuilder,
        signing_key: &IdentityPublicKey,
        signer: &S,
    ) -> Result<DocumentReplaceResult, Error> {
        trace!(
            document_id = %replace_document_transition_builder.document.id(),
            document_revision = replace_document_transition_builder.document.revision().unwrap_or(0),
            "document_replace: start"
        );

        // Destructure so we can move builder-owned fields (notably the
        // `StateTransitionCreationOptions`, which is not necessarily Clone)
        // into the effective settings without an extra copy.
        let DocumentReplaceTransitionBuilder {
            data_contract,
            document_type_name,
            document,
            token_payment_info,
            settings,
            user_fee_increase,
            state_transition_creation_options,
        } = replace_document_transition_builder;

        // Keep original settings for broadcast (request_settings,
        // wait_timeout, etc.) and overlay builder-specific
        // user_fee_increase / state_transition_creation_options onto the
        // effective settings passed to the strict helper.
        let broadcast_settings = settings;
        let mut effective_settings = settings.unwrap_or_default();
        if let Some(ufi) = user_fee_increase {
            effective_settings.user_fee_increase = Some(ufi);
        }
        if state_transition_creation_options.is_some() {
            effective_settings.state_transition_creation_options =
                state_transition_creation_options;
        }

        let document_type = data_contract
            .document_type_cloned_for_name(&document_type_name)
            .map_err(|e| Error::Protocol(e.into()))?;

        trace!("document_replace: signing state transition");
        // Route through the strict replace helper so the one-shot
        // `document_replace` builder API gets the same fail-fast
        // revision-vs-intent validation as the wasm-sdk
        // `prepareDocumentReplace` path. Pre-broadcast errors roll back
        // the allocated identity-contract nonce inside the helper.
        let state_transition = build_signed_document_replace_transition(
            self,
            &document,
            &document_type,
            signing_key,
            token_payment_info,
            signer,
            Some(effective_settings),
        )
        .await?;
        trace!("document_replace: state transition signed");

        trace!("document_replace: broadcasting and awaiting response");
        let proof_result = state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(self, broadcast_settings)
            .await?;
        trace!("document_replace: broadcast completed");

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
