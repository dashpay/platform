use crate::state_transition::identity_key_limits_update_transition::v0::IdentityKeyLimitsUpdateTransitionV0;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for IdentityKeyLimitsUpdateTransitionV0 {
    fn feature_version(&self) -> FeatureVersion {
        0
    }
}
