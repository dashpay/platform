use crate::state_transition::state_transitions::identity::identity_create_transition::IdentityCreateTransition;
use crate::state_transition::FeatureVersioned;
use platform_version::version::protocol_version::FeatureVersion;

impl FeatureVersioned for IdentityCreateTransition {
    fn feature_version(&self) -> FeatureVersion {
        match self {
            IdentityCreateTransition::V0(v0) => v0.feature_version(),
        }
    }
}
