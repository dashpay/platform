use dashcore_rpc::dashcore::Network;
use dpp::block::block_info::BlockInfo;
use dpp::identifier::Identifier;
use dpp::validation::SimpleConsensusValidationResult;
use drive::state_transition_action::document::documents_batch::document_transition::token_issuance_transition_action::TokenIssuanceTransitionAction;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;
use crate::error::Error;
use crate::error::execution::ExecutionError;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::documents_batch::action_validation::token_issuance_transition_action::state_v0::TokenIssuanceTransitionActionStateValidationV0;
use crate::execution::validation::state_transition::documents_batch::action_validation::token_issuance_transition_action::structure_v0::TokenIssuanceTransitionActionStructureValidationV0;
use crate::platform_types::platform::PlatformStateRef;

mod state_v0;
mod structure_v0;

pub trait TokenIssuanceTransitionActionValidation {
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

impl TokenIssuanceTransitionActionValidation for TokenIssuanceTransitionAction {
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
            .token_issuance_transition_structure_validation
        {
            0 => self.validate_structure_v0(owner_id, block_info, network, platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "TokenIssuanceTransitionAction::validate_structure".to_string(),
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
                method: "TokenIssuanceTransitionAction::validate_state".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}
