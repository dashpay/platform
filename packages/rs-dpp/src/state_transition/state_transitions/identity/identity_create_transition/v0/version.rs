use crate::state_transition::identity_create_transition::v0::IdentityCreateTransitionV0;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for IdentityCreateTransitionV0 {
    fn feature_version(&self) -> FeatureVersion {
        0
    }
}
