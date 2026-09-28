use crate::identity::{KeyID, Purpose, SecurityLevel};
use crate::state_transition::contract_fee_claim_transition::ContractFeeClaimTransition;
use crate::state_transition::StateTransitionIdentitySigned;

impl StateTransitionIdentitySigned for ContractFeeClaimTransition {
    fn signature_public_key_id(&self) -> KeyID {
        match self {
            ContractFeeClaimTransition::V0(transition) => transition.signature_public_key_id(),
        }
    }

    fn set_signature_public_key_id(&mut self, key_id: KeyID) {
        match self {
            ContractFeeClaimTransition::V0(transition) => {
                transition.set_signature_public_key_id(key_id)
            }
        }
    }

    fn security_level_requirement(&self, purpose: Purpose) -> Vec<SecurityLevel> {
        match self {
            ContractFeeClaimTransition::V0(transition) => {
                transition.security_level_requirement(purpose)
            }
        }
    }
}
