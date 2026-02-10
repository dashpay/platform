mod transform_into_action;

use dpp::state_transition::unshield_transition::UnshieldTransition;
use dpp::validation::ConsensusValidationResult;
use drive::state_transition_action::StateTransitionAction;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::validation::state_transition::unshield::transform_into_action::v0::UnshieldStateTransitionTransformIntoActionValidationV0;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;

use crate::platform_types::platform_state::PlatformStateV0Methods;

/// A trait to transform into an action for unshield transition
pub trait StateTransitionUnshieldTransitionActionTransformer {
    /// Transform into an action for unshield transition
    fn transform_into_action_for_unshield_transition<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl StateTransitionUnshieldTransitionActionTransformer for UnshieldTransition {
    fn transform_into_action_for_unshield_transition<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version = platform.state.current_platform_version()?;

        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .unshield_state_transition
            .transform_into_action
        {
            0 => self.transform_into_action_v0(platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "unshield transition: transform_into_action".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
