use crate::state_transition_action::identity::identity_credit_transfer::v0::IdentityCreditTransferTransitionActionV0;
use dpp::state_transition::state_transitions::identity::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0;

impl From<IdentityCreditTransferTransitionV0> for IdentityCreditTransferTransitionActionV0 {
    fn from(value: IdentityCreditTransferTransitionV0) -> Self {
        let IdentityCreditTransferTransitionV0 {
            identity_id,
            recipient_id,
            amount,
            ..
        } = value;
        IdentityCreditTransferTransitionActionV0 {
            identity_id,
            recipient_id,
            transfer_amount: amount,
        }
    }
}

impl From<&IdentityCreditTransferTransitionV0> for IdentityCreditTransferTransitionActionV0 {
    fn from(value: &IdentityCreditTransferTransitionV0) -> Self {
        let IdentityCreditTransferTransitionV0 {
            identity_id,
            recipient_id,
            amount,
            ..
        } = value;
        IdentityCreditTransferTransitionActionV0 {
            identity_id: *identity_id,
            recipient_id: *recipient_id,
            transfer_amount: *amount,
        }
    }
}
