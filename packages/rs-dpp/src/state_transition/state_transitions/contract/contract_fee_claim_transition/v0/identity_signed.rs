use crate::identity::SecurityLevel::CRITICAL;
use crate::identity::{KeyID, Purpose, SecurityLevel};
use crate::state_transition::contract_fee_claim_transition::v0::ContractFeeClaimTransitionV0;
use crate::state_transition::StateTransitionIdentitySigned;

impl StateTransitionIdentitySigned for ContractFeeClaimTransitionV0 {
    fn signature_public_key_id(&self) -> KeyID {
        self.signature_public_key_id
    }

    fn set_signature_public_key_id(&mut self, key_id: KeyID) {
        self.signature_public_key_id = key_id
    }

    /// A claim moves credits out of a contract's pot: a CRITICAL key only, as for moderation.
    fn security_level_requirement(&self, _purpose: Purpose) -> Vec<SecurityLevel> {
        vec![CRITICAL]
    }
}
