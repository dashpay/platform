use crate::state_transition::batch_transition::BatchTransition;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for BatchTransition {
    fn feature_version(&self) -> FeatureVersion {
        match self {
            BatchTransition::V0(v0) => v0.feature_version(),
            BatchTransition::V1(v1) => v1.feature_version(),
        }
    }
}
