use crate::state_transition::state_transitions::identity::public_key_in_creation::IdentityPublicKeyInCreation;
use crate::state_transition::FeatureVersioned;
use versioned_feature_core::FeatureVersion;

impl FeatureVersioned for IdentityPublicKeyInCreation {
    fn feature_version(&self) -> FeatureVersion {
        match self {
            IdentityPublicKeyInCreation::V0(v0) => v0.feature_version(),
        }
    }
}
