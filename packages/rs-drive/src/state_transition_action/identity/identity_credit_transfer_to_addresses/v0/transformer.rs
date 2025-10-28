use crate::state_transition_action::identity::identity_credit_transfer_to_addresses::v0::IdentityCreditTransferToAddressesTransitionActionV0;
use dpp::state_transition::state_transitions::identity::identity_credit_transfer_to_addresses_transition::v0::IdentityCreditTransferToAddressesTransitionV0;

impl From<IdentityCreditTransferToAddressesTransitionV0>
    for IdentityCreditTransferToAddressesTransitionActionV0
{
    fn from(value: IdentityCreditTransferToAddressesTransitionV0) -> Self {
        let IdentityCreditTransferToAddressesTransitionV0 {
            identity_id,
            recipient_keys,
            nonce,
            user_fee_increase,
            ..
        } = value;
        IdentityCreditTransferToAddressesTransitionActionV0 {
            identity_id,
            recipient_keys,
            nonce,
            user_fee_increase,
        }
    }
}

impl From<&IdentityCreditTransferToAddressesTransitionV0>
    for IdentityCreditTransferToAddressesTransitionActionV0
{
    fn from(value: &IdentityCreditTransferToAddressesTransitionV0) -> Self {
        let IdentityCreditTransferToAddressesTransitionV0 {
            identity_id,
            recipient_keys,
            nonce,
            user_fee_increase,
            ..
        } = value;
        IdentityCreditTransferToAddressesTransitionActionV0 {
            identity_id: *identity_id,
            recipient_keys: recipient_keys.clone(),
            nonce: *nonce,
            user_fee_increase: *user_fee_increase,
        }
    }
}
