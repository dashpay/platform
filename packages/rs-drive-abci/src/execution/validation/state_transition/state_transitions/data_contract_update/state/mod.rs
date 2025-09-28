use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::data_contract_update::state::v0::DataContractUpdateStateTransitionStateValidationV0;
use crate::execution::validation::state_transition::processor::v0::StateTransitionStateValidationV0;
use crate::execution::validation::state_transition::ValidationMode;
use crate::platform_types::platform::PlatformRef;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::validation::ConsensusValidationResult;
use drive::drive::subscriptions::DriveSubscriptionFilter;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::transform_to_state_transition_action_result::TransformToStateTransitionActionResult;
use drive::state_transition_action::StateTransitionAction;

pub(crate) mod v0;

impl StateTransitionStateValidationV0 for DataContractUpdateTransition {
    fn validate_state<'a, C: CoreRPCLike>(
        &self,
        action: Option<StateTransitionAction>,
        platform: &PlatformRef<C>,
        validation_mode: ValidationMode,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        passing_filters_for_transition: &[&'a DriveSubscriptionFilter],
        // These are the filters that might still pass, if the original passes
        requiring_original_filters_for_transition: &[&'a DriveSubscriptionFilter],
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<TransformToStateTransitionActionResult<'a>>, Error> {
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
                let action = self.validate_state_v0(
                    platform,
                    block_info,
                    validation_mode,
                    execution_context,
                    tx,
                    platform_version,
                )?.map(|a| a.into());

                Ok(action)
            }
            1 => {
                if action.is_some() {
                    return Err(Error::Execution(ExecutionError::CorruptedCodeExecution("data contract update is calling validate state, and the action is already known. It should not be known at this point")));
                }
                self.validate_state_v1(
                    platform,
                    block_info,
                    validation_mode,
                    execution_context,
                    passing_filters_for_transition,
                    requiring_original_filters_for_transition,
                    tx,
                    platform_version,
                )
            }
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "data contract update transition: validate_state".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}
