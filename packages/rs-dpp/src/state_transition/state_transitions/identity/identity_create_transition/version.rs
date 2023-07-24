use crate::state_transition::identity_create_transition::IdentityCreateTransition;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for IdentityCreateTransition {
    fn feature_version(&self) -> FeatureVersion {
        match self {
            IdentityCreateTransition::V0(v0) => v0.feature_version(),
        }
    }
}
