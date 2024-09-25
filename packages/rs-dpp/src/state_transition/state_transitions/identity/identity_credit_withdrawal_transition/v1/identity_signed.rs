use crate::identity::SecurityLevel::{CRITICAL, MASTER};
use crate::identity::{KeyID, Purpose, SecurityLevel};
use crate::state_transition::identity_credit_withdrawal_transition::v1::IdentityCreditWithdrawalTransitionV1;
use crate::state_transition::StateTransitionIdentitySigned;

impl StateTransitionIdentitySigned for IdentityCreditWithdrawalTransitionV1 {
    fn signature_public_key_id(&self) -> KeyID {
        self.signature_public_key_id
    }

    fn set_signature_public_key_id(&mut self, key_id: KeyID) {
        self.signature_public_key_id = key_id
    }

    fn security_level_requirement(&self, purpose: Purpose) -> Vec<SecurityLevel> {
        if purpose == Purpose::AUTHENTICATION {
            vec![MASTER]
        } else {
            // for transfer
            vec![CRITICAL]
        }
    }

    fn purpose_requirement(&self) -> Vec<Purpose> {
        vec![Purpose::TRANSFER, Purpose::AUTHENTICATION]
    }
}
