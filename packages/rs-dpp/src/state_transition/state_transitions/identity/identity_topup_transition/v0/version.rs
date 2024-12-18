use crate::state_transition::state_transitions::identity::identity_topup_transition::v0::IdentityTopUpTransitionV0;
use crate::state_transition::FeatureVersioned;
use versioned_feature_core::FeatureVersion;

impl FeatureVersioned for IdentityTopUpTransitionV0 {
    fn feature_version(&self) -> FeatureVersion {
        0
    }
}
