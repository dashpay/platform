use crate::state_transition_action::identity::identity_topup_from_addresses::v0::IdentityTopUpFromAddressesTransitionActionV0;
use dpp::address_funds::PlatformAddress;
use dpp::consensus::state::address_funds::address_does_not_exist_error::AddressDoesNotExistError;
use dpp::consensus::state::address_funds::address_not_enough_funds_error::AddressNotEnoughFundsError;
use dpp::fee::Credits;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::state_transitions::identity::identity_topup_from_addresses_transition::v0::IdentityTopUpFromAddressesTransitionV0;
use std::collections::BTreeMap;

impl IdentityTopUpFromAddressesTransitionActionV0 {
    /// Transforms the state transition into an action by validating inputs against provided balances.
    ///
    /// For each input address:
    /// 1. Validates the address exists in the provided balances
    /// 2. Validates there is sufficient balance for the claimed spend amount
    /// 3. Computes the remaining balance after the transfer
    pub fn try_from_transition(
        value: &IdentityTopUpFromAddressesTransitionV0,
        input_balances: BTreeMap<PlatformAddress, Credits>,
    ) -> ConsensusValidationResult<Self> {
        let IdentityTopUpFromAddressesTransitionV0 {
            identity_id,
            inputs,
            output,
            fee_strategy,
            user_fee_increase,
            ..
        } = value;

        // Validate each input and compute remaining balances
        let mut inputs_with_remaining_balance = BTreeMap::new();

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
                }
                None => {
                    // Address does not exist
                    return ConsensusValidationResult::new_with_error(
                        AddressDoesNotExistError::new(*address).into(),
                    );
                }
            }
        }

        ConsensusValidationResult::new_with_data(IdentityTopUpFromAddressesTransitionActionV0 {
            inputs_with_remaining_balance,
            output: *output,
            fee_strategy: fee_strategy.clone(),
            identity_id: *identity_id,
            user_fee_increase: *user_fee_increase,
        })
    }
}
