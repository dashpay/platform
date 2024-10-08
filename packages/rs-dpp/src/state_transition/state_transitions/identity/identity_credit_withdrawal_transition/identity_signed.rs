use crate::identity::{KeyID, Purpose, SecurityLevel};
use crate::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;
use crate::state_transition::StateTransitionIdentitySigned;

impl StateTransitionIdentitySigned for IdentityCreditWithdrawalTransition {
    fn signature_public_key_id(&self) -> KeyID {
        match self {
            IdentityCreditWithdrawalTransition::V0(transition) => {
                transition.signature_public_key_id()
            }
            IdentityCreditWithdrawalTransition::V1(transition) => {
                transition.signature_public_key_id()
            }
        }
    }

    fn set_signature_public_key_id(&mut self, key_id: KeyID) {
        match self {
            IdentityCreditWithdrawalTransition::V0(transition) => {
                transition.set_signature_public_key_id(key_id)
            }
            IdentityCreditWithdrawalTransition::V1(transition) => {
                transition.set_signature_public_key_id(key_id)
            }
        }
    }

    fn security_level_requirement(&self, purpose: Purpose) -> Vec<SecurityLevel> {
        match self {
            IdentityCreditWithdrawalTransition::V0(transition) => {
                transition.security_level_requirement(purpose)
            }
            IdentityCreditWithdrawalTransition::V1(transition) => {
                transition.security_level_requirement(purpose)
            }
        }
    }

    fn purpose_requirement(&self) -> Vec<Purpose> {
        match self {
            IdentityCreditWithdrawalTransition::V0(transition) => transition.purpose_requirement(),
            IdentityCreditWithdrawalTransition::V1(transition) => transition.purpose_requirement(),
        }
    }
}
