use dpp::identifier::Identifier;
use dpp::validation::SimpleConsensusValidationResult;
use drive::state_transition_action::document::documents_batch::document_transition::document_create_transition_action::DocumentCreateTransitionAction;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;
use crate::error::Error;
use crate::error::execution::ExecutionError;
use crate::execution::validation::state_transition::documents_batch::action_validation::document_create_transition_action::state_v0::DocumentCreateTransitionActionStateValidationV0;
use crate::execution::validation::state_transition::documents_batch::action_validation::document_create_transition_action::structure_v0::DocumentCreateTransitionActionStructureValidationV0;
use crate::platform_types::platform::PlatformStateRef;

mod state_v0;
mod structure_v0;

pub trait DocumentCreateTransitionActionValidation {
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;

    fn validate_state(
        &self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        transaction: TransactionArg,
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
                method: "DocumentCreateTransitionAction::validate_structure".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    fn validate_state(
        &self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .documents_batch_state_transition
            .document_create_transition_state_validation
        {
            0 => self.validate_state_v0(platform, owner_id, transaction, platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "DocumentCreateTransitionAction::validate_state".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
