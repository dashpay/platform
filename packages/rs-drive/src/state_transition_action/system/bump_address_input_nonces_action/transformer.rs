use crate::state_transition_action::address_funds::address_funds_transfer::AddressFundsTransferTransitionAction;
use crate::state_transition_action::identity::identity_create_from_addresses::IdentityCreateFromAddressesTransitionAction;
use crate::state_transition_action::identity::identity_topup_from_addresses::IdentityTopUpFromAddressesTransitionAction;
use crate::state_transition_action::system::bump_address_input_nonces_action::{
    BumpAddressInputNoncesAction, BumpAddressInputNoncesActionV0,
};
use dpp::fee::Credits;
use dpp::state_transition::state_transitions::address_funds::address_funds_transfer_transition::AddressFundsTransferTransition;
use dpp::state_transition::state_transitions::identity::identity_create_from_addresses_transition::IdentityCreateFromAddressesTransition;
use dpp::state_transition::state_transitions::identity::identity_topup_from_addresses_transition::IdentityTopUpFromAddressesTransition;

impl BumpAddressInputNoncesAction {
    // IdentityCreateFromAddresses transformers

    /// from IdentityCreateFromAddresses transition
    pub fn from_identity_create_from_addresses_transition(
        value: IdentityCreateFromAddressesTransition,
        penalty_credits: Credits,
    ) -> Self {
        match value {
            IdentityCreateFromAddressesTransition::V0(ref v0) => {
                BumpAddressInputNoncesActionV0::from_borrowed_identity_create_from_addresses_transition(v0, penalty_credits)
                    .into()
            }
        }
    }

    /// from borrowed IdentityCreateFromAddresses transition
    pub fn from_borrowed_identity_create_from_addresses_transition(
        value: &IdentityCreateFromAddressesTransition,
        penalty_credits: Credits,
    ) -> Self {
        match value {
            IdentityCreateFromAddressesTransition::V0(v0) => {
                BumpAddressInputNoncesActionV0::from_borrowed_identity_create_from_addresses_transition(v0, penalty_credits)
                    .into()
            }
        }
    }

    /// from IdentityCreateFromAddresses transition action
    pub fn from_identity_create_from_addresses_transition_action(
        value: IdentityCreateFromAddressesTransitionAction,
        penalty_credits: Credits,
    ) -> Self {
        match value {
            IdentityCreateFromAddressesTransitionAction::V0(ref v0) => {
                BumpAddressInputNoncesActionV0::from_borrowed_identity_create_from_addresses_transition_action(v0, penalty_credits)
                    .into()
            }
        }
    }

    /// from borrowed IdentityCreateFromAddresses transition action
    pub fn from_borrowed_identity_create_from_addresses_transition_action(
        value: &IdentityCreateFromAddressesTransitionAction,
        penalty_credits: Credits,
    ) -> Self {
        match value {
            IdentityCreateFromAddressesTransitionAction::V0(v0) => {
                BumpAddressInputNoncesActionV0::from_borrowed_identity_create_from_addresses_transition_action(v0, penalty_credits)
                    .into()
            }
        }
    }

    // IdentityTopUpFromAddresses transformers

    /// from IdentityTopUpFromAddresses transition
    pub fn from_identity_topup_from_addresses_transition(
        value: IdentityTopUpFromAddressesTransition,
        penalty_credits: Credits,
    ) -> Self {
        match value {
            IdentityTopUpFromAddressesTransition::V0(ref v0) => {
                BumpAddressInputNoncesActionV0::from_borrowed_identity_topup_from_addresses_transition(v0, penalty_credits)
                    .into()
            }
        }
    }

    /// from borrowed IdentityTopUpFromAddresses transition
    pub fn from_borrowed_identity_topup_from_addresses_transition(
        value: &IdentityTopUpFromAddressesTransition,
        penalty_credits: Credits,
    ) -> Self {
        match value {
            IdentityTopUpFromAddressesTransition::V0(v0) => {
                BumpAddressInputNoncesActionV0::from_borrowed_identity_topup_from_addresses_transition(v0, penalty_credits)
                    .into()
            }
        }
    }

    /// from IdentityTopUpFromAddresses transition action
    pub fn from_identity_topup_from_addresses_transition_action(
        value: IdentityTopUpFromAddressesTransitionAction,
        penalty_credits: Credits,
    ) -> Self {
        match value {
            IdentityTopUpFromAddressesTransitionAction::V0(ref v0) => {
                BumpAddressInputNoncesActionV0::from_borrowed_identity_topup_from_addresses_transition_action(v0, penalty_credits)
                    .into()
            }
        }
    }

    /// from borrowed IdentityTopUpFromAddresses transition action
    pub fn from_borrowed_identity_topup_from_addresses_transition_action(
        value: &IdentityTopUpFromAddressesTransitionAction,
        penalty_credits: Credits,
    ) -> Self {
        match value {
            IdentityTopUpFromAddressesTransitionAction::V0(v0) => {
                BumpAddressInputNoncesActionV0::from_borrowed_identity_topup_from_addresses_transition_action(v0, penalty_credits)
                    .into()
            }
        }
    }

    // AddressFundsTransfer transformers

    /// from AddressFundsTransfer transition
    pub fn from_address_funds_transfer_transition(
        value: AddressFundsTransferTransition,
        penalty_credits: Credits,
    ) -> Self {
        match value {
            AddressFundsTransferTransition::V0(ref v0) => {
                BumpAddressInputNoncesActionV0::from_borrowed_address_funds_transfer_transition(v0, penalty_credits).into()
            }
        }
    }

    /// from borrowed AddressFundsTransfer transition
    pub fn from_borrowed_address_funds_transfer_transition(
        value: &AddressFundsTransferTransition,
        penalty_credits: Credits,
    ) -> Self {
        match value {
            AddressFundsTransferTransition::V0(v0) => {
                BumpAddressInputNoncesActionV0::from_borrowed_address_funds_transfer_transition(v0, penalty_credits)
                    .into()
            }
        }
    }

    /// from AddressFundsTransfer transition action
    pub fn from_address_funds_transfer_transition_action(
        value: AddressFundsTransferTransitionAction,
        penalty_credits: Credits,
    ) -> Self {
        match value {
            AddressFundsTransferTransitionAction::V0(ref v0) => {
                BumpAddressInputNoncesActionV0::from_borrowed_address_funds_transfer_transition_action(v0, penalty_credits)
                    .into()
            }
        }
    }

    /// from borrowed AddressFundsTransfer transition action
    pub fn from_borrowed_address_funds_transfer_transition_action(
        value: &AddressFundsTransferTransitionAction,
        penalty_credits: Credits,
    ) -> Self {
        match value {
            AddressFundsTransferTransitionAction::V0(v0) => {
                BumpAddressInputNoncesActionV0::from_borrowed_address_funds_transfer_transition_action(v0, penalty_credits)
                    .into()
            }
        }
    }
}
