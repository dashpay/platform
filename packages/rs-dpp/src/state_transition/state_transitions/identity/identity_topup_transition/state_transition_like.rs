use crate::prelude::AssetLockProof;
use crate::state_transition::identity_topup_transition::IdentityTopUpTransition;
use crate::state_transition::{StateTransitionLike, StateTransitionType};
use crate::version::FeatureVersion;
use platform_value::{BinaryData, Identifier};

impl StateTransitionLike for IdentityTopUpTransition {
    /// Returns ID of the topupd contract
    fn modified_data_ids(&self) -> Vec<Identifier> {
        match self {
            IdentityTopUpTransition::V0(transition) => transition.modified_data_ids(),
        }
    }

    fn state_transition_protocol_version(&self) -> FeatureVersion {
        match self {
            IdentityTopUpTransition::V0(_) => 0,
        }
    }
    /// returns the type of State Transition
    fn state_transition_type(&self) -> StateTransitionType {
        match self {
            IdentityTopUpTransition::V0(transition) => transition.state_transition_type(),
        }
    }
    /// returns the signature as a byte-array
    fn signature(&self) -> &BinaryData {
        match self {
            IdentityTopUpTransition::V0(transition) => transition.signature(),
        }
    }
    /// set a new signature
    fn set_signature(&mut self, signature: BinaryData) {
        match self {
            IdentityTopUpTransition::V0(transition) => transition.set_signature(signature),
        }
    }

    fn set_signature_bytes(&mut self, signature: Vec<u8>) {
        match self {
            IdentityTopUpTransition::V0(transition) => transition.set_signature_bytes(signature),
        }
    }

    fn owner_id(&self) -> Identifier {
        match self {
            IdentityTopUpTransition::V0(transition) => transition.owner_id(),
        }
    }

    fn asset_lock(&self) -> Option<&AssetLockProof> {
        match self {
            IdentityTopUpTransition::V0(transition) => transition.asset_lock(),
        }
    }
}
