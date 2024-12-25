use dashcore_rpc::dashcore::Network;
use dpp::block::block_info::BlockInfo;
use dpp::identifier::Identifier;
use dpp::validation::SimpleConsensusValidationResult;
use drive::state_transition_action::document::documents_batch::document_transition::token_transfer_transition_action::TokenTransferTransitionAction;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;
use crate::error::Error;
use crate::error::execution::ExecutionError;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::batch::action_validation::token_transfer_transition_action::state_v0::TokenTransferTransitionActionStateValidationV0;
use crate::execution::validation::state_transition::batch::action_validation::token_transfer_transition_action::structure_v0::TokenTransferTransitionActionStructureValidationV0;
use crate::platform_types::platform::PlatformStateRef;

mod state_v0;
mod structure_v0;

pub trait TokenTransferTransitionActionValidation {
    fn validate_structure(
        &self,
        owner_id: Identifier,
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

impl TokenTransferTransitionActionValidation for TokenTransferTransitionAction {
    fn validate_structure(
        &self,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .batch_state_transition
            .token_transfer_transition_structure_validation
        {
            0 => self.validate_structure_v0(owner_id, platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "TokenTransferTransitionAction::validate_structure".to_string(),
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
            .token_issuance_transition_state_validation
        {
            0 => self.validate_state_v0(
                platform,
                owner_id,
                block_info,
                execution_context,
                transaction,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "TokenTransferTransitionAction::validate_state".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
