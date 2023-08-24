use dpp::ProtocolError;
use dpp::validation::SimpleConsensusValidationResult;
use drive::state_transition_action::document::documents_batch::document_transition::document_replace_transition_action::DocumentReplaceTransitionAction;
use dpp::version::PlatformVersion;
use crate::error::Error;
use crate::error::execution::ExecutionError;
use crate::execution::validation::state_transition::documents_batch::action_validation::document_replace_transition_action::v0::DocumentReplaceTransitionActionValidationV0;

mod v0;
pub trait DocumentReplaceTransitionActionValidation {
    fn validate(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl DocumentReplaceTransitionActionValidation for DocumentReplaceTransitionAction {
    fn validate(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .documents_batch_state_transition
            .document_replace_transition
        {
            0 => self.validate_v0(platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "DocumentReplaceTransitionAction::validate".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
