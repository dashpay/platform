use crate::state_transition::identity_key_limits_update_transition::IdentityKeyLimitsUpdateTransition;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for IdentityKeyLimitsUpdateTransition {
    fn feature_version(&self) -> FeatureVersion {
        match self {
            IdentityKeyLimitsUpdateTransition::V0(v0) => v0.feature_version(),
        }
    }
}
