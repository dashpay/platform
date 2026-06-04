use crate::state_transition_action::identity::identity_create_from_addresses::v0::IdentityCreateFromAddressesTransitionActionV0;
use dpp::address_funds::PlatformAddress;
use dpp::consensus::basic::overflow_error::OverflowError;
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
            inputs,
            output,
            fee_strategy,
            public_keys,
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
                        "Input sum overflow in identity create transformer".to_string(),
                    )
                    .into(),
                )
            }
        };

        // Subtract the output amount if present
        let fund_identity_amount = match output {
            Some((_, output_amount)) => match total_inputs.checked_sub(*output_amount) {
                Some(diff) => diff,
                None => {
                    return ConsensusValidationResult::new_with_error(
                        OverflowError::new(
                            "Output exceeds input sum in identity create transformer".to_string(),
                        )
                        .into(),
                    )
                }
            },
            None => total_inputs,
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
