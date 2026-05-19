use crate::state_transition::shield_transition::v0::ShieldTransitionV0;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for ShieldTransitionV0 {
    fn feature_version(&self) -> FeatureVersion {
        0
    }
}
