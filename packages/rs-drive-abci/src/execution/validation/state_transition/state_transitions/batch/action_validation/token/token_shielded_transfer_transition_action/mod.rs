use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::batch::action_validation::token::token_shielded_transfer_transition_action::state_v0::TokenShieldedTransferTransitionActionStateValidationV0;
use crate::execution::validation::state_transition::ValidationMode;
use crate::platform_types::platform::PlatformStateRef;
use dpp::block::block_info::BlockInfo;
use dpp::identifier::Identifier;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::batch::batched_transition::token_transition::token_shielded_transfer_transition_action::TokenShieldedTransferTransitionAction;

mod state_v0;

pub trait TokenShieldedTransferTransitionActionValidation {
    /// Stateful validation. `validation_mode` decides whether the Orchard proof is verified
    /// here (block processing) or left to CheckTx's admission-limited path.
    #[allow(clippy::too_many_arguments)]
    fn validate_state(
        &self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        validation_mode: ValidationMode,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl TokenShieldedTransferTransitionActionValidation for TokenShieldedTransferTransitionAction {
    fn validate_state(
        &self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        validation_mode: ValidationMode,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .batch_state_transition
            .token_shielded_transfer_transition_state_validation
        {
            0 => self.validate_state_v0(
                platform,
                owner_id,
                block_info,
                execution_context,
                validation_mode,
                transaction,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "TokenShieldedTransferTransitionAction::validate_state".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
