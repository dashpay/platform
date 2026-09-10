use crate::platform::transition::broadcast::BroadcastStateTransition;
use crate::platform::transition::put_settings::PutSettings;
use crate::platform::{Fetch, Identifier};
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
use dpp::version::PlatformVersion;
use drive::drive::document::history::{
    DocumentHistoryLifecycle, DocumentHistorySelector, DocumentHistoryState,
};
use std::sync::Arc;

/// A builder to configure and broadcast document erase transitions.
///
/// Erasing purges the retained revisions of a document that has already been
/// deleted. The first erase must come from the document's owner and commits the
/// document to erasure; every erase after that may come from any identity,
/// because the committed state already carries the authorization.
pub struct DocumentEraseTransitionBuilder {
    /// The data contract.
    pub data_contract: Arc<DataContract>,
    /// The name of the document type whose revisions are being erased.
    pub document_type_name: String,
    /// The document whose revisions are being erased.
    pub document_id: Identifier,
    /// The identity submitting and paying for this erase, which need not own
    /// the document once its erasure has begun.
    pub owner_id: Identifier,
    /// Settings for broadcasting.
    pub settings: Option<PutSettings>,
    /// A user fee increase.
    pub user_fee_increase: Option<UserFeeIncrease>,
    /// State transition creation options.
    pub state_transition_creation_options: Option<StateTransitionCreationOptions>,
}

impl DocumentEraseTransitionBuilder {
    /// Start building an erase request for the provided data contract.
    ///
    /// There is no token payment: the deletion this erase follows was charged
    /// when the document was deleted, and an erase that carried one would let a
    /// continuation, which anyone may submit, move tokens.
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
            settings: None,
            user_fee_increase: None,
            state_transition_creation_options: None,
        }
    }

    /// Adds a user fee increase to the erase transition.
    pub fn with_user_fee_increase(mut self, user_fee_increase: UserFeeIncrease) -> Self {
        self.user_fee_increase = Some(user_fee_increase);
        self
    }

    /// Adds settings to the erase transition.
    pub fn with_settings(mut self, settings: PutSettings) -> Self {
        self.settings = Some(settings);
        self
    }

    /// Adds creation options to the erase transition.
    pub fn with_state_transition_creation_options(
        mut self,
        creation_options: StateTransitionCreationOptions,
    ) -> Self {
        self.state_transition_creation_options = Some(creation_options);
        self
    }

    /// Signs the erase transition.
    pub async fn sign(
        &self,
        sdk: &Sdk,
        identity_public_key: &IdentityPublicKey,
        signer: &impl Signer<IdentityPublicKey>,
        platform_version: &PlatformVersion,
    ) -> Result<StateTransition, Error> {
        // Validate the target before the nonce fetch below bumps the SDK's
        // cached contract nonce: no transition is broadcast on an error path,
        // so a rejection after it would leak an increment per failed call.
        let document_type = self
            .data_contract
            .document_type_for_name(&self.document_type_name)
            .map_err(|e| Error::Protocol(e.into()))?;

        // The transition carries only the base, so an id is all the builder
        // needs; the values of a document that is no longer visible are not
        // available to a client anyway.
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

        let identity_contract_nonce = sdk
            .get_identity_contract_nonce(
                self.owner_id,
                self.data_contract.id(),
                true,
                self.settings,
            )
            .await?;

        let state_transition = BatchTransition::new_document_erase_transition_from_document(
            document,
            document_type,
            identity_public_key,
            identity_contract_nonce,
            self.user_fee_increase.unwrap_or_default(),
            signer,
            platform_version,
            self.state_transition_creation_options,
        )
        .await?;

        Ok(state_transition)
    }
}

/// What one erase transition achieved.
#[derive(Debug)]
pub enum DocumentEraseResult {
    /// The erase was accepted. Whether it removed the last revision is not
    /// something the transition's own proof can show, because the document was
    /// already absent from ordinary reads before it ran — read the current
    /// lifecycle with [`Sdk::document_current_lifecycle`] to find out.
    Accepted(Identifier),
}

impl Sdk {
    /// Erases a chunk of the retained revisions of an already deleted document.
    ///
    /// The first erase must be signed by the document's owner; any identity may
    /// submit the ones after it. A document with more retained revisions than
    /// one transition may remove needs several, and this method broadcasts one.
    ///
    /// The proof this returns authenticates that the document is absent by id,
    /// which it already was before the erase ran, so it is evidence about the
    /// state, not about this transition having executed. Use
    /// [`Sdk::document_current_lifecycle`] to observe how much history is left.
    pub async fn document_erase<S: Signer<IdentityPublicKey>>(
        &self,
        erase_document_transition_builder: DocumentEraseTransitionBuilder,
        signing_key: &IdentityPublicKey,
        signer: &S,
    ) -> Result<DocumentEraseResult, Error> {
        let platform_version = self.version();
        let put_settings = erase_document_transition_builder.settings;
        let document_id = erase_document_transition_builder.document_id;

        let state_transition = erase_document_transition_builder
            .sign(self, signing_key, signer, platform_version)
            .await?;

        let proof_result = state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(self, put_settings)
            .await?;

        match proof_result {
            StateTransitionProofResult::VerifiedDocuments(documents) => {
                if let Some((erased_id, None)) = documents.into_iter().next() {
                    Ok(DocumentEraseResult::Accepted(erased_id))
                } else {
                    Err(Error::DriveProofError(
                        drive::error::proof::ProofError::UnexpectedResultProof(
                            "expected an absent document in the VerifiedDocuments result for an \
                             erase transition"
                                .to_string(),
                        ),
                        vec![],
                        Default::default(),
                    ))
                }
            }
            _ => Err(Error::DriveProofError(
                drive::error::proof::ProofError::UnexpectedResultProof(
                    "expected VerifiedDocuments for a document erase transition".to_string(),
                ),
                vec![],
                Default::default(),
            )),
        }
    }

    /// Reads where a document stands in its lifecycle right now.
    ///
    /// Deliberately named apart from the erase and delete calls: the answer
    /// describes committed state at the moment of the read, not the outcome of
    /// any particular transition. Another erase, or a re-create of the same id,
    /// may land between a transition and this read.
    pub async fn document_current_lifecycle(
        &self,
        data_contract_id: Identifier,
        document_type_name: String,
        document_id: Identifier,
    ) -> Result<DocumentHistoryLifecycle, Error> {
        use dash_platform_queries::documents::document_history_query::DocumentHistoryQuery;
        use drive_proof_verifier::types::DocumentHistory;

        let history = DocumentHistory::fetch(
            self,
            DocumentHistoryQuery {
                data_contract_id,
                document_type_name,
                document_id,
                // The oldest retained revision, if any: the page is not the
                // point, the metadata alongside it is.
                selector: DocumentHistorySelector::StartAtTime(0),
                limit: Some(1),
            },
        )
        .await?;

        Ok(history
            .and_then(|history| history.lifecycle)
            .unwrap_or(DocumentHistoryLifecycle {
                state: DocumentHistoryState::Absent,
                remaining_revisions: 0,
                times: Default::default(),
            }))
    }
}
