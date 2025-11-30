use std::collections::BTreeMap;
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
/// The penalty is distributed across inputs in order, deducting as much as possible from each.
fn deduct_penalty_from_inputs(
    inputs: &BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    penalty_credits: Credits,
) -> BTreeMap<PlatformAddress, (AddressNonce, Credits)> {
    let mut remaining_penalty = penalty_credits;
    inputs
        .iter()
        .map(|(key, (nonce, balance))| {
            let deduction = remaining_penalty.min(*balance);
            remaining_penalty -= deduction;
            (key.clone(), (*nonce, balance - deduction))
        })
        .collect()
}

impl BumpAddressInputNoncesActionV0 {
    /// Helper to create action with penalty deduction
    fn new_with_penalty(
        inputs: &BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        penalty_credits: Credits,
        user_fee_increase: UserFeeIncrease,
    ) -> Self {
        BumpAddressInputNoncesActionV0 {
            inputs_with_remaining_balance: deduct_penalty_from_inputs(inputs, penalty_credits),
            user_fee_increase,
        }
    }

    // IdentityCreateFromAddresses transformers

    /// from borrowed IdentityCreateFromAddresses transition
    /// Subtracts penalty_credits from the input balances (distributed across inputs in order)
    pub fn from_borrowed_identity_create_from_addresses_transition(
        value: &IdentityCreateFromAddressesTransitionV0,
        penalty_credits: Credits,
    ) -> Self {
        Self::new_with_penalty(&value.inputs, penalty_credits, value.user_fee_increase)
    }

    /// from borrowed IdentityCreateFromAddresses transition action
    pub fn from_borrowed_identity_create_from_addresses_transition_action(
        value: &IdentityCreateFromAddressesTransitionActionV0,
        penalty_credits: Credits,
    ) -> Self {
        Self::new_with_penalty(
            &value.inputs_with_remaining_balance,
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
        Self::new_with_penalty(&value.inputs, penalty_credits, value.user_fee_increase)
    }

    /// from borrowed IdentityTopUpFromAddresses transition action
    pub fn from_borrowed_identity_topup_from_addresses_transition_action(
        value: &IdentityTopUpFromAddressesTransitionActionV0,
        penalty_credits: Credits,
    ) -> Self {
        Self::new_with_penalty(
            &value.inputs_with_remaining_balance,
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
        Self::new_with_penalty(&value.inputs, penalty_credits, value.user_fee_increase)
    }

    /// from borrowed AddressFundsTransfer transition action
    pub fn from_borrowed_address_funds_transfer_transition_action(
        value: &AddressFundsTransferTransitionActionV0,
        penalty_credits: Credits,
    ) -> Self {
        Self::new_with_penalty(
            &value.inputs_with_remaining_balance,
            penalty_credits,
            value.user_fee_increase,
        )
    }
}
