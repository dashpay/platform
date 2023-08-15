use crate::state_transition::FeatureVersioned;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use crate::version::FeatureVersion;

impl FeatureVersioned for IdentityPublicKeyInCreation {
    fn feature_version(&self) -> FeatureVersion {
        match self {
            IdentityPublicKeyInCreation::V0(v0) => v0.feature_version(),
        }
    }
}
