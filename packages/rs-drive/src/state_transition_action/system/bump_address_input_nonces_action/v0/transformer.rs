use std::collections::{BTreeMap, HashSet};
use dpp::address_funds::fee_strategy::{AddressFundsFeeStrategy, AddressFundsFeeStrategyStep};
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::prelude::{AddressNonce, UserFeeIncrease};
use crate::state_transition_action::address_funds::address_funds_transfer::v0::AddressFundsTransferTransitionActionV0;
use crate::state_transition_action::identity::identity_create_from_addresses::v0::IdentityCreateFromAddressesTransitionActionV0;
use crate::state_transition_action::identity::identity_topup_from_addresses::v0::IdentityTopUpFromAddressesTransitionActionV0;
use crate::state_transition_action::system::bump_address_input_nonces_action::BumpAddressInputNoncesActionV0;
use dpp::state_transition::state_transitions::address_funds::address_funds_transfer_transition::v0::AddressFundsTransferTransitionV0;
use dpp::state_transition::state_transitions::identity::identity_create_from_addresses_transition::v0::IdentityCreateFromAddressesTransitionV0;
use dpp::state_transition::state_transitions::identity::identity_topup_from_addresses_transition::v0::IdentityTopUpFromAddressesTransitionV0;

/// Helper function to subtract penalty credits from input balances.
/// The penalty is distributed across inputs according to the fee strategy order,
/// deducting as much as possible from each input before moving to the next.
fn deduct_penalty_from_inputs(
    inputs: &BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    fee_strategy: &AddressFundsFeeStrategy,
    penalty_credits: Credits,
) -> BTreeMap<PlatformAddress, (AddressNonce, Credits)> {
    let mut remaining_penalty = penalty_credits;
    let mut result = inputs.clone();

    // Convert inputs to a Vec for index-based access
    let input_keys: Vec<_> = inputs.keys().collect();

    // Track which input indices we've already processed
    let mut processed_indices = HashSet::new();

    // First, process inputs in fee strategy order
    for step in fee_strategy {
        if let AddressFundsFeeStrategyStep::DeductFromInput(idx) = step {
            let idx = *idx as usize;
            if idx < input_keys.len() && !processed_indices.contains(&idx) {
                processed_indices.insert(idx);
                let key = input_keys[idx];
                if let Some((_nonce, balance)) = result.get_mut(key) {
                    let deduction = remaining_penalty.min(*balance);
                    remaining_penalty -= deduction;
                    *balance -= deduction;
                }
            }
        }
    }

    // Then, process any remaining inputs not covered by the fee strategy
    for (idx, key) in input_keys.iter().enumerate() {
        if !processed_indices.contains(&idx) {
            if let Some((_, balance)) = result.get_mut(*key) {
                let deduction = remaining_penalty.min(*balance);
                remaining_penalty -= deduction;
                *balance -= deduction;
            }
        }
    }

    result
}

impl BumpAddressInputNoncesActionV0 {
    /// Helper to create action with penalty deduction
    fn new_with_penalty(
        inputs: &BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        fee_strategy: &AddressFundsFeeStrategy,
        penalty_credits: Credits,
        user_fee_increase: UserFeeIncrease,
    ) -> Self {
        BumpAddressInputNoncesActionV0 {
            inputs_with_remaining_balance: deduct_penalty_from_inputs(
                inputs,
                fee_strategy,
                penalty_credits,
            ),
            fee_strategy: fee_strategy.clone(),
            user_fee_increase,
        }
    }

    // IdentityCreateFromAddresses transformers

    /// from borrowed IdentityCreateFromAddresses transition
    /// Subtracts penalty_credits from the input balances (distributed across inputs according to fee strategy)
    pub fn from_borrowed_identity_create_from_addresses_transition(
        value: &IdentityCreateFromAddressesTransitionV0,
        penalty_credits: Credits,
    ) -> Self {
        Self::new_with_penalty(
            &value.inputs,
            &value.fee_strategy,
            penalty_credits,
            value.user_fee_increase,
        )
    }

    /// from borrowed IdentityCreateFromAddresses transition action
    pub fn from_borrowed_identity_create_from_addresses_transition_action(
        value: &IdentityCreateFromAddressesTransitionActionV0,
        penalty_credits: Credits,
    ) -> Self {
        Self::new_with_penalty(
            &value.inputs_with_remaining_balance,
            &value.fee_strategy,
            penalty_credits,
            value.user_fee_increase,
        )
    }

    // IdentityTopUpFromAddresses transformers

    /// from borrowed IdentityTopUpFromAddresses transition
    pub fn from_borrowed_identity_topup_from_addresses_transition(
        value: &IdentityTopUpFromAddressesTransitionV0,
        penalty_credits: Credits,
    ) -> Self {
        Self::new_with_penalty(
            &value.inputs,
            &value.fee_strategy,
            penalty_credits,
            value.user_fee_increase,
        )
    }

    /// from borrowed IdentityTopUpFromAddresses transition action
    pub fn from_borrowed_identity_topup_from_addresses_transition_action(
        value: &IdentityTopUpFromAddressesTransitionActionV0,
        penalty_credits: Credits,
    ) -> Self {
        Self::new_with_penalty(
            &value.inputs_with_remaining_balance,
            &value.fee_strategy,
            penalty_credits,
            value.user_fee_increase,
        )
    }

    // AddressFundsTransfer transformers

    /// from borrowed AddressFundsTransfer transition
    pub fn from_borrowed_address_funds_transfer_transition(
        value: &AddressFundsTransferTransitionV0,
        penalty_credits: Credits,
    ) -> Self {
        Self::new_with_penalty(
            &value.inputs,
            &value.fee_strategy,
            penalty_credits,
            value.user_fee_increase,
        )
    }

    /// from borrowed AddressFundsTransfer transition action
    pub fn from_borrowed_address_funds_transfer_transition_action(
        value: &AddressFundsTransferTransitionActionV0,
        penalty_credits: Credits,
    ) -> Self {
        Self::new_with_penalty(
            &value.inputs_with_remaining_balance,
            &value.fee_strategy,
            penalty_credits,
            value.user_fee_increase,
        )
    }
}
