use crate::state_transition::identity_topup_transition::v0::IdentityTopUpTransitionV0;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for IdentityTopUpTransitionV0 {
    fn feature_version(&self) -> FeatureVersion {
        0
    }
}
