use crate::state_transition::identity_topup_transition::IdentityTopUpTransition;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for IdentityTopUpTransition {
    fn feature_version(&self) -> FeatureVersion {
        match self {
            IdentityTopUpTransition::V0(v0) => v0.feature_version(),
        }
    }
}
