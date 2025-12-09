mod tests;
mod transform_into_action;

use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::prelude::AddressNonce;
use dpp::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition;
use dpp::validation::ConsensusValidationResult;
use drive::state_transition_action::StateTransitionAction;
use std::collections::BTreeMap;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::validation::state_transition::address_funds_transfer::transform_into_action::v0::AddressFundsTransferStateTransitionTransformIntoActionValidationV0;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;

use crate::platform_types::platform_state::v0::PlatformStateV0Methods;

/// A trait to transform into an action for address funds transfer
pub trait StateTransitionAddressFundsTransferTransitionActionTransformer {
    /// Transform into an action for address funds transfer
    fn transform_into_action_for_address_funds_transfer_transition<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        inputs_with_remaining_balance: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl StateTransitionAddressFundsTransferTransitionActionTransformer
    for AddressFundsTransferTransition
{
    fn transform_into_action_for_address_funds_transfer_transition<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        inputs_with_remaining_balance: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version = platform.state.current_platform_version()?;

        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .address_funds_transfer
            .transform_into_action
        {
            0 => self.transform_into_action_v0(inputs_with_remaining_balance, platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "address funds transfer transition: transform_into_action".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
