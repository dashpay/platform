use platform_value::BinaryData;

use crate::{
    prelude::Identifier,
    state_transition::{StateTransitionLike, StateTransitionType},
};

use crate::state_transition::identity_update_transition::v0::IdentityUpdateTransitionV0;
use crate::state_transition::identity_update_transition::IdentityUpdateTransition;

use crate::state_transition::StateTransition;
use crate::state_transition::StateTransitionType::IdentityUpdate;
use crate::version::FeatureVersion;

impl From<IdentityUpdateTransitionV0> for StateTransition {
    fn from(value: IdentityUpdateTransitionV0) -> Self {
        let identity_update_transition: IdentityUpdateTransition = value.into();
        identity_update_transition.into()
    }
}

impl StateTransitionLike for IdentityUpdateTransitionV0 {
    fn state_transition_protocol_version(&self) -> FeatureVersion {
        0
    }

    /// returns the type of State Transition
    fn state_transition_type(&self) -> StateTransitionType {
        IdentityUpdate
    }
    /// returns the signature as a byte-array
    fn signature(&self) -> &BinaryData {
        &self.signature
    }
    /// set a new signature
    fn set_signature(&mut self, signature: BinaryData) {
        self.signature = signature
    }
    /// Returns ID of the created contract
    fn modified_data_ids(&self) -> Vec<Identifier> {
        vec![self.identity_id]
    }

    fn set_signature_bytes(&mut self, signature: Vec<u8>) {
        self.signature = BinaryData::new(signature)
    }

    /// Get owner ID
    fn owner_id(&self) -> Identifier {
        self.identity_id
    }
}
