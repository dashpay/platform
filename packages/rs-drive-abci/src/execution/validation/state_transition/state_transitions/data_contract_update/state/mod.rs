use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::data_contract_update::state::v0::DataContractUpdateStateTransitionStateValidationV0;
use crate::execution::validation::state_transition::processor::v0::StateTransitionStateValidationV0;
use crate::platform_types::platform::PlatformRef;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::rpc::core::CoreRPCLike;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::validation::ConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::StateTransitionAction;

pub(crate) mod v0;

impl StateTransitionStateValidationV0 for DataContractUpdateTransition {
    fn validate_state<C: CoreRPCLike>(
        &self,
        _action: Option<StateTransitionAction>,
        platform: &PlatformRef<C>,
        _execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version =
            PlatformVersion::get(platform.state.current_protocol_version_in_consensus())?;
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .contract_update_state_transition
            .state
        {
            0 => self.validate_state_v0(platform, tx, platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "data contract update transition: validate_state".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
