use crate::state_transition::batch_transition::BatchTransitionV2;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for BatchTransitionV2 {
    fn feature_version(&self) -> FeatureVersion {
        2
    }
}
