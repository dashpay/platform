use crate::error::Error;
use dpp::address_funds::PlatformAddress;
use dpp::block::block_info::BlockInfo;
use dpp::fee::Credits;
use dpp::prelude::{AddressNonce, ConsensusValidationResult};
use dpp::state_transition::address_credit_withdrawal_transition::AddressCreditWithdrawalTransition;
use dpp::version::PlatformVersion;
use drive::state_transition_action::address_funds::address_credit_withdrawal::AddressCreditWithdrawalTransitionAction;
use drive::state_transition_action::StateTransitionAction;
use std::collections::BTreeMap;

pub(in crate::execution::validation::state_transition::state_transitions::address_credit_withdrawal) trait AddressCreditWithdrawalStateTransitionTransformIntoActionValidationV0
{
    fn transform_into_action_v0(
        &self,
        block_info: &BlockInfo,
        inputs_with_remaining_balance: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl AddressCreditWithdrawalStateTransitionTransformIntoActionValidationV0
    for AddressCreditWithdrawalTransition
{
    fn transform_into_action_v0(
        &self,
        block_info: &BlockInfo,
        inputs_with_remaining_balance: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        _platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        // Get the creation time from block info for the withdrawal document
        let creation_time_ms = block_info.time_ms;

        let result = AddressCreditWithdrawalTransitionAction::try_from_transition(
            self,
            inputs_with_remaining_balance,
            creation_time_ms,
        );

        Ok(result.map(|action| action.into()))
    }
}
