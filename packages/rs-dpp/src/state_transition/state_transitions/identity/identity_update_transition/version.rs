use crate::state_transition::identity_create_transition::IdentityCreateTransition;
use crate::state_transition::identity_update_transition::IdentityUpdateTransition;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for IdentityUpdateTransition {
    fn feature_version(&self) -> FeatureVersion {
        match self {
            IdentityUpdateTransition::V0(v0) => v0.feature_version(),
        }
    }
}
