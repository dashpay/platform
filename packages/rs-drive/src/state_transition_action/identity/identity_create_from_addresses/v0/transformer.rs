use crate::state_transition_action::identity::identity_create_from_addresses::v0::IdentityCreateFromAddressesTransitionActionV0;
use dpp::address_funds::PlatformAddress;
use dpp::consensus::basic::value_error::ValueError;
use dpp::consensus::ConsensusError;
use dpp::fee::Credits;
use dpp::prelude::{AddressNonce, ConsensusValidationResult};
use dpp::state_transition::state_transitions::identity::identity_create_from_addresses_transition::v0::IdentityCreateFromAddressesTransitionV0;
use dpp::state_transition::StateTransitionIdentityIdFromInputs;
use std::collections::BTreeMap;

impl IdentityCreateFromAddressesTransitionActionV0 {
    /// Transforms the state transition into an action using pre-validated inputs with remaining balances.
    pub fn try_from_transition(
        value: &IdentityCreateFromAddressesTransitionV0,
        inputs_with_remaining_balance: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    ) -> ConsensusValidationResult<Self> {
        let identity_id = match value.identity_id_from_inputs() {
            Ok(id) => id,
            Err(e) => {
                return ConsensusValidationResult::new_with_error(ConsensusError::from(
                    ValueError::new_from_string(format!(
                        "Failed to calculate identity id from inputs: {}",
                        e
                    )),
                ));
            }
        };

        let IdentityCreateFromAddressesTransitionV0 {
            output,
            fee_strategy,
            public_keys,
            user_fee_increase,
            ..
        } = value;

        // Sum all remaining balances from inputs
        let total_remaining: Credits = inputs_with_remaining_balance
            .values()
            .map(|(_, balance)| *balance)
            .sum();

        // Subtract the output amount if present
        let fund_identity_amount = match output {
            Some((_, output_amount)) => total_remaining - output_amount,
            None => total_remaining,
        };

        ConsensusValidationResult::new_with_data(IdentityCreateFromAddressesTransitionActionV0 {
            inputs_with_remaining_balance,
            output: *output,
            fee_strategy: fee_strategy.clone(),
            public_keys: public_keys.iter().map(|key| key.into()).collect(),
            identity_id,
            fund_identity_amount,
            user_fee_increase: *user_fee_increase,
        })
    }
}
