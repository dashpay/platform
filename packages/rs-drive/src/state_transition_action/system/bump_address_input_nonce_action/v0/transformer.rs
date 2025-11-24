use crate::state_transition_action::address_funds::address_funds_transfer::v0::AddressFundsTransferTransitionActionV0;
use crate::state_transition_action::identity::identity_create_from_addresses::v0::IdentityCreateFromAddressesTransitionActionV0;
use crate::state_transition_action::identity::identity_topup_from_addresses::v0::IdentityTopUpFromAddressesTransitionActionV0;
use crate::state_transition_action::system::bump_address_input_nonce_action::BumpAddressInputNonceActionV0;
use dpp::state_transition::state_transitions::address_funds::address_funds_transfer_transition::v0::AddressFundsTransferTransitionV0;
use dpp::state_transition::state_transitions::identity::identity_create_from_addresses_transition::v0::IdentityCreateFromAddressesTransitionV0;
use dpp::state_transition::state_transitions::identity::identity_topup_from_addresses_transition::v0::IdentityTopUpFromAddressesTransitionV0;

impl BumpAddressInputNonceActionV0 {
    // IdentityCreateFromAddresses transformers

    /// from IdentityCreateFromAddresses transition
    pub fn from_identity_create_from_addresses_transition(
        value: IdentityCreateFromAddressesTransitionV0,
    ) -> Self {
        BumpAddressInputNonceActionV0 {
            inputs_with_remaining_balance: value.inputs,
            user_fee_increase: value.user_fee_increase,
        }
    }

    /// from borrowed IdentityCreateFromAddresses transition
    pub fn from_borrowed_identity_create_from_addresses_transition(
        value: &IdentityCreateFromAddressesTransitionV0,
    ) -> Self {
        BumpAddressInputNonceActionV0 {
            inputs_with_remaining_balance: value.inputs.clone(),
            user_fee_increase: value.user_fee_increase,
        }
    }

    /// from IdentityCreateFromAddresses transition action
    pub fn from_identity_create_from_addresses_transition_action(
        value: IdentityCreateFromAddressesTransitionActionV0,
    ) -> Self {
        BumpAddressInputNonceActionV0 {
            inputs_with_remaining_balance: value.inputs_with_remaining_balance,
            user_fee_increase: value.user_fee_increase,
        }
    }

    /// from borrowed IdentityCreateFromAddresses transition action
    pub fn from_borrowed_identity_create_from_addresses_transition_action(
        value: &IdentityCreateFromAddressesTransitionActionV0,
    ) -> Self {
        BumpAddressInputNonceActionV0 {
            inputs_with_remaining_balance: value.inputs_with_remaining_balance.clone(),
            user_fee_increase: value.user_fee_increase,
        }
    }

    // IdentityTopUpFromAddresses transformers

    /// from IdentityTopUpFromAddresses transition
    pub fn from_identity_topup_from_addresses_transition(
        value: IdentityTopUpFromAddressesTransitionV0,
    ) -> Self {
        BumpAddressInputNonceActionV0 {
            inputs_with_remaining_balance: value.inputs,
            user_fee_increase: value.user_fee_increase,
        }
    }

    /// from borrowed IdentityTopUpFromAddresses transition
    pub fn from_borrowed_identity_topup_from_addresses_transition(
        value: &IdentityTopUpFromAddressesTransitionV0,
    ) -> Self {
        BumpAddressInputNonceActionV0 {
            inputs_with_remaining_balance: value.inputs.clone(),
            user_fee_increase: value.user_fee_increase,
        }
    }

    /// from IdentityTopUpFromAddresses transition action
    pub fn from_identity_topup_from_addresses_transition_action(
        value: IdentityTopUpFromAddressesTransitionActionV0,
    ) -> Self {
        BumpAddressInputNonceActionV0 {
            inputs_with_remaining_balance: value.inputs_with_remaining_balance,
            user_fee_increase: value.user_fee_increase,
        }
    }

    /// from borrowed IdentityTopUpFromAddresses transition action
    pub fn from_borrowed_identity_topup_from_addresses_transition_action(
        value: &IdentityTopUpFromAddressesTransitionActionV0,
    ) -> Self {
        BumpAddressInputNonceActionV0 {
            inputs_with_remaining_balance: value.inputs_with_remaining_balance.clone(),
            user_fee_increase: value.user_fee_increase,
        }
    }

    // AddressFundsTransfer transformers

    /// from AddressFundsTransfer transition
    pub fn from_address_funds_transfer_transition(value: AddressFundsTransferTransitionV0) -> Self {
        BumpAddressInputNonceActionV0 {
            inputs_with_remaining_balance: value.inputs,
            user_fee_increase: value.user_fee_increase,
        }
    }

    /// from borrowed AddressFundsTransfer transition
    pub fn from_borrowed_address_funds_transfer_transition(
        value: &AddressFundsTransferTransitionV0,
    ) -> Self {
        BumpAddressInputNonceActionV0 {
            inputs_with_remaining_balance: value.inputs.clone(),
            user_fee_increase: value.user_fee_increase,
        }
    }

    /// from AddressFundsTransfer transition action
    pub fn from_address_funds_transfer_transition_action(
        value: AddressFundsTransferTransitionActionV0,
    ) -> Self {
        BumpAddressInputNonceActionV0 {
            inputs_with_remaining_balance: value.inputs_with_remaining_balance,
            user_fee_increase: value.user_fee_increase,
        }
    }

    /// from borrowed AddressFundsTransfer transition action
    pub fn from_borrowed_address_funds_transfer_transition_action(
        value: &AddressFundsTransferTransitionActionV0,
    ) -> Self {
        BumpAddressInputNonceActionV0 {
            inputs_with_remaining_balance: value.inputs_with_remaining_balance.clone(),
            user_fee_increase: value.user_fee_increase,
        }
    }
}
