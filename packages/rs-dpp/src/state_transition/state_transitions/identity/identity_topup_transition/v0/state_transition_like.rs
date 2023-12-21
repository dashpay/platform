use platform_value::BinaryData;

use crate::prelude::AssetLockProof;
use crate::state_transition::identity_topup_transition::IdentityTopUpTransition;
use crate::{
    prelude::Identifier,
    state_transition::{StateTransitionLike, StateTransitionType},
};

use crate::state_transition::identity_topup_transition::v0::IdentityTopUpTransitionV0;

use crate::state_transition::StateTransition;
use crate::state_transition::StateTransitionType::IdentityTopUp;
use crate::version::FeatureVersion;

impl From<IdentityTopUpTransitionV0> for StateTransition {
    fn from(value: IdentityTopUpTransitionV0) -> Self {
        let transition: IdentityTopUpTransition = value.into();
        transition.into()
    }
}

impl StateTransitionLike for IdentityTopUpTransitionV0 {
    fn state_transition_protocol_version(&self) -> FeatureVersion {
        0
    }

    /// returns the type of State Transition
    fn state_transition_type(&self) -> StateTransitionType {
        IdentityTopUp
    }
    /// returns the signature as a byte-array
    fn signature(&self) -> &BinaryData {
        &self.signature
    }
    /// set a new signature
    fn set_signature(&mut self, signature: BinaryData) {
        self.signature = signature
    }
    /// Returns ID of the topUpd contract
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

    fn asset_lock(&self) -> Option<&AssetLockProof> {
        Some(&self.asset_lock_proof)
    }
}
