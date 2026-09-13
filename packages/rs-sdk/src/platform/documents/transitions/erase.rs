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
use dpp::ProtocolError;
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
        let (user_fee_increase, creation_options) = self.signing_parameters();
        Self::check_erase_is_constructible(creation_options.as_ref(), platform_version)?;

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
            user_fee_increase,
            signer,
            platform_version,
            creation_options,
        )
        .await?;

        Ok(state_transition)
    }

    /// The fee increase and creation options the transition is signed with.
    ///
    /// An explicitly set value wins; otherwise the ones carried by the put
    /// settings apply, so a caller that only hands over settings (the wasm and
    /// FFI wrappers do) still signs with what it asked for.
    fn signing_parameters(&self) -> (UserFeeIncrease, Option<StateTransitionCreationOptions>) {
        let settings = self.settings.as_ref();
        (
            self.user_fee_increase
                .or(settings.and_then(|settings| settings.user_fee_increase))
                .unwrap_or_default(),
            self.state_transition_creation_options
                .or(settings.and_then(|settings| settings.state_transition_creation_options)),
        )
    }

    /// Refuses, before any nonce is reserved, an erase the platform version
    /// cannot construct: the transition kind joined the wire at protocol
    /// version 14, and the batch may only be built at a version it knows.
    ///
    /// The same rejection happens inside the transition constructor, but by
    /// then the SDK has already advanced its cached contract nonce for a
    /// transition that is never broadcast.
    fn check_erase_is_constructible(
        creation_options: Option<&StateTransitionCreationOptions>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let serialization = &platform_version.dpp.state_transition_serialization_versions;
        let Some(erase_bounds) = serialization.document_erase_state_transition.as_ref() else {
            return Err(Error::Protocol(ProtocolError::Generic(
                "erase transitions do not exist at this platform version".to_string(),
            )));
        };
        let batch_feature_version = creation_options
            .and_then(|options| options.batch_feature_version)
            .unwrap_or(serialization.batch_state_transition.default_current_version);
        if !matches!(batch_feature_version, 0 | 1) {
            return Err(Error::Protocol(ProtocolError::UnknownVersionMismatch {
                method: "DocumentEraseTransitionBuilder::sign".to_string(),
                known_versions: vec![0, 1],
                received: batch_feature_version,
            }));
        }
        let method_feature_version = creation_options
            .and_then(|options| options.method_feature_version)
            .unwrap_or(erase_bounds.bounds.default_current_version);
        if method_feature_version != 0 {
            return Err(Error::Protocol(ProtocolError::UnknownVersionMismatch {
                method: "DocumentEraseTransitionBuilder::sign".to_string(),
                known_versions: vec![0],
                received: method_feature_version,
            }));
        }
        Ok(())
    }
}

/// What one erase transition left behind.
#[derive(Debug)]
pub enum DocumentEraseResult {
    /// The document is absent by id as of the proof's block. It already was
    /// before the erase ran, so this is an observation of the state the erase
    /// affected, not evidence that this erase executed — read the current
    /// lifecycle with [`Sdk::document_current_lifecycle`] to find out how much
    /// history is left.
    AbsentAsOfProof(Identifier),
}

/// Reads the observation an erase leaves behind out of the verified result.
///
/// Its own function so a test can drive it with the outcome an erase actually
/// produces: the proof classifier reports an erase as affected state, which the
/// strict wait refuses, so the erase has to take the affected-state wait and
/// this has to accept what that wait returns.
fn erase_observation(result: StateTransitionProofResult) -> Result<DocumentEraseResult, Error> {
    match result {
        StateTransitionProofResult::VerifiedDocuments(documents) => {
            if let Some((erased_id, None)) = documents.into_iter().next() {
                Ok(DocumentEraseResult::AbsentAsOfProof(erased_id))
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

impl Sdk {
    /// Erases a chunk of the retained revisions of an already deleted document.
    ///
    /// The first erase must be signed by the document's owner; any identity may
    /// submit the ones after it. A document with more retained revisions than
    /// one transition may remove needs several, and this method broadcasts one.
    ///
    /// The proof this returns authenticates that the document is absent by id,
    /// which it already was before the erase ran, so it is an observation of
    /// the state the erase affected rather than evidence that this erase
    /// executed. That is why it takes the affected-state wait: the strict wait
    /// refuses exactly this classification. Use
    /// [`Sdk::document_current_lifecycle`] to observe how much history is left.
    pub async fn document_erase<S: Signer<IdentityPublicKey>>(
        &self,
        erase_document_transition_builder: DocumentEraseTransitionBuilder,
        signing_key: &IdentityPublicKey,
        signer: &S,
    ) -> Result<DocumentEraseResult, Error> {
        let platform_version = self.version();
        let put_settings = erase_document_transition_builder.settings;

        let state_transition = erase_document_transition_builder
            .sign(self, signing_key, signer, platform_version)
            .await?;

        let proof_result = state_transition
            .broadcast_and_wait_for_affected_state::<StateTransitionProofResult>(self, put_settings)
            .await?;

        erase_observation(proof_result)
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    /// The erase reads its observation out of exactly the result the
    /// affected-state wait hands back, and refuses anything else.
    #[test]
    fn should_read_an_absent_document_as_the_erases_observation() {
        let id = Identifier::from([9u8; 32]);
        let observation = erase_observation(StateTransitionProofResult::VerifiedDocuments(
            BTreeMap::from([(id, None)]),
        ))
        .expect("an absent document is what an erase leaves behind");
        assert!(matches!(
            observation,
            DocumentEraseResult::AbsentAsOfProof(seen) if seen == id
        ));

        let document = dpp::document::Document::V0(Default::default());
        erase_observation(StateTransitionProofResult::VerifiedDocuments(
            BTreeMap::from([(id, Some(document))]),
        ))
        .expect_err("a document that is still there did not survive an erase");

        erase_observation(StateTransitionProofResult::VerifiedTokenBalanceAbsence(id))
            .expect_err("only a document result can describe an erase");
    }

    #[cfg(feature = "mocks")]
    fn builder_with(settings: Option<PutSettings>) -> DocumentEraseTransitionBuilder {
        let contract = dpp::tests::fixtures::get_dashpay_contract_fixture(
            None,
            0,
            PlatformVersion::latest().protocol_version,
        )
        .data_contract_owned();
        let builder = DocumentEraseTransitionBuilder::new(
            Arc::new(contract),
            "profile".to_string(),
            Identifier::from([1u8; 32]),
            Identifier::from([2u8; 32]),
        );
        match settings {
            Some(settings) => builder.with_settings(settings),
            None => builder,
        }
    }

    /// A fee increase or creation options handed over inside the put settings
    /// reach the signing step, and an explicit builder value still wins.
    #[cfg(feature = "mocks")]
    #[test]
    fn should_sign_with_the_settings_fee_increase_unless_set_explicitly() {
        let options = StateTransitionCreationOptions {
            batch_feature_version: Some(1),
            ..Default::default()
        };
        let settings = PutSettings {
            user_fee_increase: Some(250),
            state_transition_creation_options: Some(options),
            ..Default::default()
        };
        assert_eq!(builder_with(None).signing_parameters(), (0, None));
        assert_eq!(
            builder_with(Some(settings)).signing_parameters(),
            (250, Some(options))
        );
        assert_eq!(
            builder_with(Some(settings))
                .with_user_fee_increase(7)
                .with_state_transition_creation_options(Default::default())
                .signing_parameters(),
            (7, Some(Default::default()))
        );
    }

    /// The version gate runs before the nonce fetch, so a platform version
    /// that cannot construct an erase is refused without reserving a nonce.
    #[test]
    fn should_refuse_an_erase_the_platform_version_cannot_construct() {
        let too_old = PlatformVersion::get(13).unwrap();
        let error = DocumentEraseTransitionBuilder::check_erase_is_constructible(None, too_old)
            .expect_err("protocol 13 has no erase transition");
        assert!(error
            .to_string()
            .contains("erase transitions do not exist at this platform version"));
        let current = PlatformVersion::get(14).unwrap();
        DocumentEraseTransitionBuilder::check_erase_is_constructible(None, current)
            .expect("protocol 14 constructs erases");
        let unknown_batch = StateTransitionCreationOptions {
            batch_feature_version: Some(9),
            ..Default::default()
        };
        DocumentEraseTransitionBuilder::check_erase_is_constructible(Some(&unknown_batch), current)
            .expect_err("an unknown batch version is refused before any nonce is reserved");
        let unknown_method = StateTransitionCreationOptions {
            method_feature_version: Some(1),
            ..Default::default()
        };
        DocumentEraseTransitionBuilder::check_erase_is_constructible(Some(&unknown_method), current)
            .expect_err("an unknown erase method version is refused before any nonce is reserved");
    }
}
