use crate::state_transition::state_transitions::identity::identity_create_transition::v0::IdentityCreateTransitionV0;
use crate::state_transition::FeatureVersioned;
use platform_version::version::protocol_version::FeatureVersion;

impl FeatureVersioned for IdentityCreateTransitionV0 {
    fn feature_version(&self) -> FeatureVersion {
        0
    }
}
