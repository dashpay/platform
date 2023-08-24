use dpp::ProtocolError;
use dpp::validation::SimpleConsensusValidationResult;
use drive::state_transition_action::document::documents_batch::document_transition::document_delete_transition_action::DocumentDeleteTransitionAction;
use dpp::version::PlatformVersion;
use crate::error::Error;
use crate::error::execution::ExecutionError;
use crate::execution::validation::state_transition::documents_batch::action_validation::document_delete_transition_action::v0::DocumentDeleteTransitionActionValidationV0;

mod v0;
pub trait DocumentDeleteTransitionActionValidation {
    fn validate(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl DocumentDeleteTransitionActionValidation for DocumentDeleteTransitionAction {
    fn validate(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .documents_batch_state_transition
            .document_delete_transition
        {
            0 => self.validate_v0(platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "DocumentDeleteTransitionAction::validate".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
