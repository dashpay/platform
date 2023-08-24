use dpp::identifier::Identifier;
use dpp::ProtocolError;
use dpp::validation::SimpleConsensusValidationResult;
use drive::state_transition_action::document::documents_batch::document_transition::document_create_transition_action::DocumentCreateTransitionAction;
use dpp::version::PlatformVersion;
use crate::error::Error;
use crate::error::execution::ExecutionError;
use crate::execution::validation::state_transition::documents_batch::action_validation::document_create_transition_action::v0::DocumentCreateTransitionActionValidationV0;

mod v0;

pub trait DocumentCreateTransitionActionValidation {
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl DocumentCreateTransitionActionValidation for DocumentCreateTransitionAction {
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .documents_batch_state_transition
            .document_create_transition_structure_validation
        {
            0 => self.validate_structure_v0(platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "DocumentCreateTransitionAction::validate".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
