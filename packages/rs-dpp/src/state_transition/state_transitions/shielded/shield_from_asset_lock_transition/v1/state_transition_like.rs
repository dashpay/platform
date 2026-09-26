use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use platform_value::BinaryData;

use crate::state_transition::shield_from_asset_lock_transition::v1::ShieldFromAssetLockTransitionV1;
use crate::state_transition::shield_from_asset_lock_transition::ShieldFromAssetLockTransition;
use crate::state_transition::{StateTransition, StateTransitionSingleSigned};
use crate::version::FeatureVersion;
use crate::{
    prelude::Identifier,
    state_transition::{StateTransitionLike, StateTransitionType},
};

impl From<ShieldFromAssetLockTransitionV1> for StateTransition {
    fn from(value: ShieldFromAssetLockTransitionV1) -> Self {
        let transition: ShieldFromAssetLockTransition = value.into();
        transition.into()
    }
}

impl StateTransitionLike for ShieldFromAssetLockTransitionV1 {
    fn state_transition_protocol_version(&self) -> FeatureVersion {
        1
    }

    /// returns the type of State Transition
    fn state_transition_type(&self) -> StateTransitionType {
        StateTransitionType::ShieldFromAssetLock
    }

    /// Returns IDs of modified data (none for shielded transitions)
    fn modified_data_ids(&self) -> Vec<Identifier> {
        vec![]
    }

    /// Returns unique identifier based on the asset lock proof.
    /// The asset lock can only be consumed once, making it the natural deduplication key.
    fn unique_identifiers(&self) -> Vec<String> {
        match self.asset_lock_proof.create_identifier() {
            Ok(identifier) => vec![BASE64_STANDARD.encode(identifier)],
            Err(_) => vec![String::default()],
        }
    }
}

impl StateTransitionSingleSigned for ShieldFromAssetLockTransitionV1 {
    /// returns the signature as a byte-array
    fn signature(&self) -> &BinaryData {
        &self.signature
    }

    /// set a new signature
    fn set_signature(&mut self, signature: BinaryData) {
        self.signature = signature
    }

    fn set_signature_bytes(&mut self, signature: Vec<u8>) {
        self.signature = BinaryData::new(signature)
    }
}
