use crate::state_transition::public_key_in_creation::v1::IdentityPublicKeyInCreationV1;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for IdentityPublicKeyInCreationV1 {
    fn feature_version(&self) -> FeatureVersion {
        1
    }
}
