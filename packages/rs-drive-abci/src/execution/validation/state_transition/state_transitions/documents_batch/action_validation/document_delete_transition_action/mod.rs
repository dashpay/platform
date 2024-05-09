use dpp::block::epoch::Epoch;
use dpp::identifier::Identifier;
use dpp::validation::SimpleConsensusValidationResult;
use drive::state_transition_action::document::documents_batch::document_transition::document_delete_transition_action::DocumentDeleteTransitionAction;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;
use crate::error::Error;
use crate::error::execution::ExecutionError;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::documents_batch::action_validation::document_delete_transition_action::state_v0::DocumentDeleteTransitionActionStateValidationV0;
use crate::execution::validation::state_transition::documents_batch::action_validation::document_delete_transition_action::structure_v0::DocumentDeleteTransitionActionStructureValidationV0;
use crate::platform_types::platform::PlatformStateRef;

mod state_v0;
mod structure_v0;

pub trait DocumentDeleteTransitionActionValidation {
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;

    fn validate_state(
        &self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        epoch: &Epoch,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl DocumentDeleteTransitionActionValidation for DocumentDeleteTransitionAction {
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .documents_batch_state_transition
            .document_delete_transition_structure_validation
        {
            0 => self.validate_structure_v0(),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "DocumentDeleteTransitionAction::validate_structure".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    fn validate_state(
        &self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        epoch: &Epoch,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .documents_batch_state_transition
            .document_delete_transition_state_validation
        {
            0 => self.validate_state_v0(
                platform,
                owner_id,
                epoch,
                execution_context,
                transaction,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "DocumentDeleteTransitionAction::validate_state".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
