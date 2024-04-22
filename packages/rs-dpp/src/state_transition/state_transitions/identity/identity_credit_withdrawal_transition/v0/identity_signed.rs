use crate::identity::SecurityLevel::CRITICAL;
use crate::identity::{KeyID, Purpose, SecurityLevel};

use crate::state_transition::identity_credit_withdrawal_transition::v0::IdentityCreditWithdrawalTransitionV0;
use crate::state_transition::StateTransitionIdentitySigned;

impl StateTransitionIdentitySigned for IdentityCreditWithdrawalTransitionV0 {
    fn signature_public_key_id(&self) -> KeyID {
        self.signature_public_key_id
    }

    fn set_signature_public_key_id(&mut self, key_id: KeyID) {
        self.signature_public_key_id = key_id
    }

    fn security_level_requirement(&self) -> Vec<SecurityLevel> {
        vec![CRITICAL]
    }

    fn purpose_requirement(&self) -> Purpose {
        Purpose::TRANSFER
    }
}
