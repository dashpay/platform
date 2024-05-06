use crate::state_transition::state_transitions::identity::identity_update_transition::v0::IdentityUpdateTransitionV0;
use crate::state_transition::FeatureVersioned;
use platform_version::version::FeatureVersion;

impl FeatureVersioned for IdentityUpdateTransitionV0 {
    fn feature_version(&self) -> FeatureVersion {
        0
    }
}
