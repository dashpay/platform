use crate::state_transition::state_transitions::identity::identity_topup_transition::IdentityTopUpTransition;
use crate::state_transition::FeatureVersioned;
use platform_version::version::FeatureVersion;

impl FeatureVersioned for IdentityTopUpTransition {
    fn feature_version(&self) -> FeatureVersion {
        match self {
            IdentityTopUpTransition::V0(v0) => v0.feature_version(),
        }
    }
}
