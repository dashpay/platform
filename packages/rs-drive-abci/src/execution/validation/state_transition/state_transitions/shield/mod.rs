#[cfg(test)]
mod tests;
mod transform_into_action;

use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::prelude::AddressNonce;
use dpp::state_transition::shield_transition::ShieldTransition;
use dpp::validation::ConsensusValidationResult;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::StateTransitionAction;
use std::collections::BTreeMap;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::validation::state_transition::shield::transform_into_action::v0::ShieldStateTransitionTransformIntoActionValidationV0;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;

use crate::platform_types::platform_state::PlatformStateV0Methods;

/// A trait to transform into an action for shield transition
pub trait StateTransitionShieldTransitionActionTransformer {
    /// Transform into an action for shield transition
    fn transform_into_action_for_shield_transition<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        inputs_with_remaining_balance: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl StateTransitionShieldTransitionActionTransformer for ShieldTransition {
    fn transform_into_action_for_shield_transition<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        inputs_with_remaining_balance: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version = platform.state.current_platform_version()?;

        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .shield_state_transition
            .transform_into_action
        {
            0 => self.transform_into_action_v0(
                platform.drive,
                tx,
                inputs_with_remaining_balance,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "shield transition: transform_into_action".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
