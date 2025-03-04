use crate::state_transition::state_transitions::document::batch_transition::BatchTransitionV1;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for BatchTransitionV1 {
    fn feature_version(&self) -> FeatureVersion {
        1
    }
}
