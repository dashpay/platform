use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::data_contract_update::state::v0::DataContractUpdateStateTransitionStateValidationV0;
use crate::execution::validation::state_transition::data_contract_update::state::v1::DataContractUpdateStateTransitionStateValidationV1;
use crate::execution::validation::state_transition::processor::state::StateTransitionStateValidation;
use crate::execution::validation::state_transition::ValidationMode;
use crate::platform_types::platform::PlatformRef;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::validation::ConsensusValidationResult;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::StateTransitionAction;

pub(crate) mod v0;
pub(crate) mod v1;

impl StateTransitionStateValidation for DataContractUpdateTransition {
    fn validate_state<C: CoreRPCLike>(
        &self,
        action: Option<StateTransitionAction>,
        platform: &PlatformRef<C>,
        validation_mode: ValidationMode,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version = platform.state.current_platform_version()?;

        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .contract_update_state_transition
            .state
        {
            0 => {
                if action.is_some() {
                    return Err(Error::Execution(ExecutionError::CorruptedCodeExecution("data contract update is calling validate state, and the action is already known. It should not be known at this point")));
                }
                self.validate_state_v0(
                    platform,
                    block_info,
                    validation_mode,
                    execution_context,
                    tx,
                    platform_version,
                )
            }
            1 => {
                if action.is_some() {
                    return Err(Error::Execution(ExecutionError::CorruptedCodeExecution("data contract update is calling validate state, and the action is already known. It should not be known at this point")));
                }
                // V0 transitions use the V0 validator, V1 transitions use the V1 validator
                match self {
                    DataContractUpdateTransition::V0(_) => self.validate_state_v0(
                        platform,
                        block_info,
                        validation_mode,
                        execution_context,
                        tx,
                        platform_version,
                    ),
                    DataContractUpdateTransition::V1(_) => self.validate_state_v1(
                        platform,
                        block_info,
                        validation_mode,
                        execution_context,
                        tx,
                        platform_version,
                    ),
                }
            }
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "data contract update transition: validate_state".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}
