use crate::state_transition_action::identity::identity_topup_from_addresses::v0::IdentityTopUpFromAddressesTransitionActionV0;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::prelude::{AddressNonce, ConsensusValidationResult};
use dpp::state_transition::state_transitions::identity::identity_topup_from_addresses_transition::v0::IdentityTopUpFromAddressesTransitionV0;
use std::collections::BTreeMap;

impl IdentityTopUpFromAddressesTransitionActionV0 {
    /// Transforms the state transition into an action using pre-validated inputs with remaining balances.
    pub fn try_from_transition(
        value: &IdentityTopUpFromAddressesTransitionV0,
        inputs_with_remaining_balance: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    ) -> ConsensusValidationResult<Self> {
        let IdentityTopUpFromAddressesTransitionV0 {
            identity_id,
            output,
            fee_strategy,
            user_fee_increase,
            ..
        } = value;

        // Sum all remaining balances from inputs
        let total_remaining: Credits = inputs_with_remaining_balance
            .values()
            .map(|(_, balance)| *balance)
            .sum();

        // Subtract the output amount if present to get the topup amount
        let topup_amount = match output {
            Some((_, output_amount)) => total_remaining - output_amount,
            None => total_remaining,
        };

        ConsensusValidationResult::new_with_data(IdentityTopUpFromAddressesTransitionActionV0 {
            inputs_with_remaining_balance,
            output: *output,
            fee_strategy: fee_strategy.clone(),
            identity_id: *identity_id,
            topup_amount,
            user_fee_increase: *user_fee_increase,
        })
    }
}
