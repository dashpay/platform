use crate::state_transition::batch_transition::BatchTransitionV0;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for BatchTransitionV0 {
    fn feature_version(&self) -> FeatureVersion {
        0
    }
}
