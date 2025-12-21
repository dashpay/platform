use crate::error::Error;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::prelude::{AddressNonce, ConsensusValidationResult};
use dpp::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition;
use dpp::version::PlatformVersion;
use drive::state_transition_action::address_funds::address_funds_transfer::AddressFundsTransferTransitionAction;
use drive::state_transition_action::StateTransitionAction;
use std::collections::BTreeMap;

pub(in crate::execution::validation::state_transition::state_transitions::address_funds_transfer) trait AddressFundsTransferStateTransitionTransformIntoActionValidationV0
{
    fn transform_into_action_v0(
        &self,
        inputs_with_remaining_balance: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl AddressFundsTransferStateTransitionTransformIntoActionValidationV0
    for AddressFundsTransferTransition
{
    fn transform_into_action_v0(
        &self,
        inputs_with_remaining_balance: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        _platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let result = AddressFundsTransferTransitionAction::try_from_transition(
            self,
            inputs_with_remaining_balance,
        );

        Ok(result.map(|action| action.into()))
    }
}
