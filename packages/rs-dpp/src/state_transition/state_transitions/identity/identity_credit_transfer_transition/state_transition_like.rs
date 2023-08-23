use crate::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
use crate::state_transition::{StateTransitionLike, StateTransitionType};
use crate::version::FeatureVersion;
use platform_value::{BinaryData, Identifier};

impl StateTransitionLike for IdentityCreditTransferTransition {
    /// Returns ID of the credit_transferred contract
    fn modified_data_ids(&self) -> Vec<Identifier> {
        match self {
            IdentityCreditTransferTransition::V0(transition) => transition.modified_data_ids(),
        }
    }

    fn state_transition_protocol_version(&self) -> FeatureVersion {
        match self {
            IdentityCreditTransferTransition::V0(_) => 0,
        }
    }
    /// returns the type of State Transition
    fn state_transition_type(&self) -> StateTransitionType {
        match self {
            IdentityCreditTransferTransition::V0(transition) => transition.state_transition_type(),
        }
    }
    /// returns the signature as a byte-array
    fn signature(&self) -> &BinaryData {
        match self {
            IdentityCreditTransferTransition::V0(transition) => transition.signature(),
        }
    }
    /// set a new signature
    fn set_signature(&mut self, signature: BinaryData) {
        match self {
            IdentityCreditTransferTransition::V0(transition) => transition.set_signature(signature),
        }
    }

    fn set_signature_bytes(&mut self, signature: Vec<u8>) {
        match self {
            IdentityCreditTransferTransition::V0(transition) => {
                transition.set_signature_bytes(signature)
            }
        }
    }

    fn owner_id(&self) -> Identifier {
        match self {
            IdentityCreditTransferTransition::V0(transition) => transition.owner_id(),
        }
    }
}
