use crate::state_transition_action::address_funds::address_credit_withdrawal::v0::AddressCreditWithdrawalTransitionActionV0;
use crate::state_transition_action::address_funds::address_credit_withdrawal::AddressCreditWithdrawalTransitionAction;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::prelude::{AddressNonce, ConsensusValidationResult};
use dpp::state_transition::address_credit_withdrawal_transition::AddressCreditWithdrawalTransition;
use std::collections::BTreeMap;

impl AddressCreditWithdrawalTransitionAction {
    /// Transforms the state transition into an action using pre-validated inputs with remaining balances.
    pub fn try_from_transition(
        value: &AddressCreditWithdrawalTransition,
        inputs_with_remaining_balance: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        creation_time_ms: u64,
    ) -> ConsensusValidationResult<Self> {
        match value {
            AddressCreditWithdrawalTransition::V0(v0) => {
                let result = AddressCreditWithdrawalTransitionActionV0::try_from_transition(
                    v0,
                    inputs_with_remaining_balance,
                    creation_time_ms,
                );
                result.map(|action| action.into())
            }
        }
    }
}
