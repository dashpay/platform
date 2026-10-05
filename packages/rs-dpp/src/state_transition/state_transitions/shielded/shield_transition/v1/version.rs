use crate::state_transition::shield_transition::v1::ShieldTransitionV1;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for ShieldTransitionV1 {
    fn feature_version(&self) -> FeatureVersion {
        1
    }
}
