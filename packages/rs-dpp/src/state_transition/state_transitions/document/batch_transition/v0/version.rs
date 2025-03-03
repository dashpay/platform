use crate::state_transition::state_transitions::document::batch_transition::BatchTransitionV0;
use crate::state_transition::FeatureVersioned;
use versioned_feature_core::FeatureVersion;

impl FeatureVersioned for BatchTransitionV0 {
    fn feature_version(&self) -> FeatureVersion {
        0
    }
}
