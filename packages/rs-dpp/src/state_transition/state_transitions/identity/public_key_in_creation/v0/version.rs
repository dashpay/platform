use crate::state_transition::public_key_in_creation::v0::IdentityPublicKeyInCreationV0;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for IdentityPublicKeyInCreationV0 {
    fn feature_version(&self) -> FeatureVersion {
        0
    }
}
