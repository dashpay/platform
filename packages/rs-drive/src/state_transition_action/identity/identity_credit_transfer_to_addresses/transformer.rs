use crate::state_transition_action::identity::identity_credit_transfer_to_addresses::v0::IdentityCreditTransferToAddressesTransitionActionV0;
use crate::state_transition_action::identity::identity_credit_transfer_to_addresses::IdentityCreditTransferToAddressesTransitionAction;
use dpp::state_transition::identity_credit_transfer_to_addresses_transition::IdentityCreditTransferToAddressesTransition;

impl From<IdentityCreditTransferToAddressesTransition>
    for IdentityCreditTransferToAddressesTransitionAction
{
    fn from(value: IdentityCreditTransferToAddressesTransition) -> Self {
        match value {
            IdentityCreditTransferToAddressesTransition::V0(v0) => {
                IdentityCreditTransferToAddressesTransitionActionV0::from(v0).into()
            }
        }
    }
}

impl From<&IdentityCreditTransferToAddressesTransition>
    for IdentityCreditTransferToAddressesTransitionAction
{
    fn from(value: &IdentityCreditTransferToAddressesTransition) -> Self {
        match value {
            IdentityCreditTransferToAddressesTransition::V0(v0) => {
                IdentityCreditTransferToAddressesTransitionActionV0::from(v0).into()
            }
        }
    }
}
