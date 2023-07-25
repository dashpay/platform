use crate::identity::SecurityLevel;
use crate::state_transition::identity_credit_transfer_transition::v0::v0_methods::IdentityCreditTransferTransitionV0Methods;
use crate::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
use crate::state_transition::{StateTransitionLike, StateTransitionType};
use platform_value::Identifier;

impl IdentityCreditTransferTransitionV0Methods for IdentityCreditTransferTransition {
    fn set_amount(&mut self, amount: u64) {
        match self {
            IdentityCreditTransferTransition::V0(transition) => transition.set_amount(amount),
        }
    }

    fn amount(&self) -> u64 {
        match self {
            IdentityCreditTransferTransition::V0(transition) => transition.amount(),
        }
    }

    fn identity_id(&self) -> Identifier {
        match self {
            IdentityCreditTransferTransition::V0(transition) => transition.identity_id(),
        }
    }

    fn set_identity_id(&mut self, identity_id: Identifier) {
        match self {
            IdentityCreditTransferTransition::V0(transition) => {
                transition.set_identity_id(identity_id)
            }
        }
    }

    fn recipient_id(&self) -> Identifier {
        match self {
            IdentityCreditTransferTransition::V0(transition) => transition.recipient_id(),
        }
    }

    fn set_recipient_id(&mut self, recipient_id: Identifier) {
        match self {
            IdentityCreditTransferTransition::V0(transition) => {
                transition.set_recipient_id(recipient_id)
            }
        }
    }

    fn security_level_requirement(&self) -> Vec<SecurityLevel> {
        match self {
            IdentityCreditTransferTransition::V0(transition) => {
                transition.security_level_requirement()
            }
        }
    }
}
