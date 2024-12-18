use crate::state_transition::state_transitions::identity::public_key_in_creation::v0::IdentityPublicKeyInCreationV0;
use crate::state_transition::FeatureVersioned;
use versioned_feature_core::FeatureVersion;

impl FeatureVersioned for IdentityPublicKeyInCreationV0 {
    fn feature_version(&self) -> FeatureVersion {
        0
    }
}
