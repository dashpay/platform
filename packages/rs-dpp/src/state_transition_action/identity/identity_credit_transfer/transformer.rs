use crate::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
use crate::state_transition_action::identity::identity_credit_transfer::IdentityCreditTransferTransitionAction;
use crate::state_transition_action::identity::identity_credit_transfer::v0::IdentityCreditTransferTransitionActionV0;

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