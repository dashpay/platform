use crate::state_transition_action::identity::identity_topup_from_addresses::v0::IdentityTopUpFromAddressesTransitionActionV0;
use dpp::address_funds::PlatformAddress;
use dpp::consensus::basic::overflow_error::OverflowError;
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
            inputs,
            identity_id,
            output,
            fee_strategy,
            user_fee_increase,
            ..
        } = value;

        // Sum all balances from inputs (checked to prevent overflow)
        let total_inputs: Credits = match inputs
            .values()
            .try_fold(0u64, |acc, (_, balance)| acc.checked_add(*balance))
        {
            Some(sum) => sum,
            None => {
                return ConsensusValidationResult::new_with_error(
                    OverflowError::new(
                        "Input sum overflow in identity topup transformer".to_string(),
                    )
                    .into(),
                )
            }
        };

        // Subtract the output amount if present to get the topup amount
        let topup_amount = match output {
            Some((_, output_amount)) => match total_inputs.checked_sub(*output_amount) {
                Some(diff) => diff,
                None => {
                    return ConsensusValidationResult::new_with_error(
                        OverflowError::new(
                            "Output exceeds input sum in identity topup transformer".to_string(),
                        )
                        .into(),
                    )
                }
            },
            None => total_inputs,
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
