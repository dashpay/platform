use crate::state_transition_action::identity::identity_credit_transfer::v0::IdentityCreditTransferTransitionActionV0;
use crate::state_transition_action::identity::identity_credit_transfer::IdentityCreditTransferTransitionAction;
use dpp::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;

impl From<IdentityCreditTransferTransition> for IdentityCreditTransferTransitionAction {
    fn from(value: IdentityCreditTransferTransition) -> Self {
        match value {
            IdentityCreditTransferTransition::V0(v0) => {
                IdentityCreditTransferTransitionActionV0::from(v0).into()
            }
        }
    }
}

impl From<&IdentityCreditTransferTransition> for IdentityCreditTransferTransitionAction {
    fn from(value: &IdentityCreditTransferTransition) -> Self {
        match value {
            IdentityCreditTransferTransition::V0(v0) => {
                IdentityCreditTransferTransitionActionV0::from(v0).into()
            }
        }
    }
}
