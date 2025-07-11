use dpp::dashcore::Network;
use dpp::block::block_info::BlockInfo;
use dpp::identifier::Identifier;
use dpp::validation::SimpleConsensusValidationResult;
use drive::state_transition_action::batch::batched_transition::document_transition::document_create_transition_action::DocumentCreateTransitionAction;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;
use crate::error::Error;
use crate::error::execution::ExecutionError;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::batch::action_validation::document::document_create_transition_action::state_v0::DocumentCreateTransitionActionStateValidationV0;
use crate::execution::validation::state_transition::batch::action_validation::document::document_create_transition_action::state_v1::DocumentCreateTransitionActionStateValidationV1;
use crate::execution::validation::state_transition::batch::action_validation::document::document_create_transition_action::advanced_structure_v0::DocumentCreateTransitionActionStructureValidationV0;
use crate::platform_types::platform::PlatformStateRef;

mod advanced_structure_v0;
mod state_v0;
mod state_v1;

pub trait DocumentCreateTransitionActionValidation {
    fn validate_structure(
        &self,
        owner_id: Identifier,
        block_info: &BlockInfo,
        network: Network,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;

    fn validate_state(
        &self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl DocumentCreateTransitionActionValidation for DocumentCreateTransitionAction {
    fn validate_structure(
        &self,
        owner_id: Identifier,
        block_info: &BlockInfo,
        network: Network,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .batch_state_transition
            .document_create_transition_structure_validation
        {
            0 => self.validate_structure_v0(owner_id, block_info, network, platform_version),
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
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .batch_state_transition
            .document_create_transition_state_validation
        {
            0 => self.validate_state_v0(
                platform,
                owner_id,
                block_info,
                execution_context,
                transaction,
                platform_version,
            ),
            // V1 introduces a validation that a contested document does not yet exist (and the
            //  cost for this operation)
            1 => self.validate_state_v1(
                platform,
                owner_id,
                block_info,
                execution_context,
                transaction,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "DocumentCreateTransitionAction::validate_state".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}
