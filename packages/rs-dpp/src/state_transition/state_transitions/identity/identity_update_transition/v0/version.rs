use crate::state_transition::identity_update_transition::v0::IdentityUpdateTransitionV0;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for IdentityUpdateTransitionV0 {
    fn feature_version(&self) -> FeatureVersion {
        0
    }
}
