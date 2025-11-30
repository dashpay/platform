use crate::state_transition_action::identity::identity_create_from_addresses::v0::IdentityCreateFromAddressesTransitionActionV0;
use dpp::address_funds::PlatformAddress;
use dpp::consensus::basic::value_error::ValueError;
use dpp::consensus::state::address_funds::address_does_not_exist_error::AddressDoesNotExistError;
use dpp::consensus::state::address_funds::address_not_enough_funds_error::AddressNotEnoughFundsError;
use dpp::consensus::ConsensusError;
use dpp::fee::Credits;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::state_transitions::identity::identity_create_from_addresses_transition::v0::IdentityCreateFromAddressesTransitionV0;
use dpp::state_transition::StateTransitionIdentityIdFromInputs;
use std::collections::BTreeMap;

impl IdentityCreateFromAddressesTransitionActionV0 {
    /// Transforms the state transition into an action by validating inputs against provided balances.
    ///
    /// For each input address:
    /// 1. Validates the address exists in the provided balances
    /// 2. Validates there is sufficient balance for the claimed spend amount
    /// 3. Computes the remaining balance after the transfer
    pub fn try_from_transition(
        value: &IdentityCreateFromAddressesTransitionV0,
        input_balances: BTreeMap<PlatformAddress, Credits>,
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
            public_keys,
            user_fee_increase,
            ..
        } = value;

        // Validate each input and compute remaining balances
        let mut inputs_with_remaining_balance = BTreeMap::new();
        let mut fund_identity_amount: Credits = 0;

        for (address, (expected_nonce, spend_amount)) in inputs {
            match input_balances.get(address) {
                Some(actual_balance) => {
                    // Address exists, check if there's enough balance
                    if *actual_balance < *spend_amount {
                        return ConsensusValidationResult::new_with_error(
                            AddressNotEnoughFundsError::new(
                                *address,
                                *actual_balance,
                                *spend_amount,
                            )
                            .into(),
                        );
                    }

                    // Compute remaining balance after the transfer
                    let remaining_balance = actual_balance - spend_amount;
                    inputs_with_remaining_balance
                        .insert(*address, (*expected_nonce, remaining_balance));

                    fund_identity_amount += spend_amount;
                }
                None => {
                    // Address does not exist
                    return ConsensusValidationResult::new_with_error(
                        AddressDoesNotExistError::new(*address).into(),
                    );
                }
            }
        }

        // Subtract the output from fund_identity_amount if present
        if let Some((_, output_amount)) = output {
            fund_identity_amount -= output_amount;
        }

        ConsensusValidationResult::new_with_data(IdentityCreateFromAddressesTransitionActionV0 {
            inputs_with_remaining_balance,
            output: *output,
            public_keys: public_keys.iter().map(|key| key.into()).collect(),
            identity_id,
            fund_identity_amount,
            user_fee_increase: *user_fee_increase,
        })
    }
}
