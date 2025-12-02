use crate::state_transition_action::address_funds::address_funds_transfer::v0::AddressFundsTransferTransitionActionV0;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::prelude::{AddressNonce, ConsensusValidationResult};
use dpp::state_transition::state_transitions::address_funds::address_funds_transfer_transition::v0::AddressFundsTransferTransitionV0;
use std::collections::BTreeMap;

impl AddressFundsTransferTransitionActionV0 {
    /// Transforms the state transition into an action using pre-validated inputs with remaining balances.
    ///
    /// # Arguments
    /// * `value` - The state transition
    /// * `inputs_with_remaining_balance` - Pre-validated inputs with remaining balances after spending
    pub fn try_from_transition(
        value: &AddressFundsTransferTransitionV0,
        inputs_with_remaining_balance: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    ) -> ConsensusValidationResult<Self> {
        let AddressFundsTransferTransitionV0 {
            outputs,
            fee_strategy,
            user_fee_increase,
            ..
        } = value;

        ConsensusValidationResult::new_with_data(AddressFundsTransferTransitionActionV0 {
            inputs_with_remaining_balance,
            outputs: outputs.clone(),
            fee_strategy: fee_strategy.clone(),
            user_fee_increase: *user_fee_increase,
        })
    }
}
