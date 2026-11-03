use crate::state_transition::shield_from_identity_transition::v0::ShieldFromIdentityTransitionV0;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for ShieldFromIdentityTransitionV0 {
    fn feature_version(&self) -> FeatureVersion {
        0
    }
}
