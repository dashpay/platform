use crate::identity::{KeyID, Purpose, SecurityLevel};
use crate::state_transition::identity_credit_transfer_to_addresses_transition::IdentityCreditTransferToAddressesTransition;
use crate::state_transition::StateTransitionIdentitySigned;

impl StateTransitionIdentitySigned for IdentityCreditTransferToAddressesTransition {
    fn signature_public_key_id(&self) -> KeyID {
        match self {
            IdentityCreditTransferToAddressesTransition::V0(transition) => {
                transition.signature_public_key_id()
            }
        }
    }

    fn set_signature_public_key_id(&mut self, key_id: KeyID) {
        match self {
            IdentityCreditTransferToAddressesTransition::V0(transition) => {
                transition.set_signature_public_key_id(key_id)
            }
        }
    }

    fn security_level_requirement(&self, purpose: Purpose) -> Vec<SecurityLevel> {
        match self {
            IdentityCreditTransferToAddressesTransition::V0(transition) => {
                transition.security_level_requirement(purpose)
            }
        }
    }

    fn purpose_requirement(&self) -> Vec<Purpose> {
        match self {
            IdentityCreditTransferToAddressesTransition::V0(transition) => {
                transition.purpose_requirement()
            }
        }
    }
}
