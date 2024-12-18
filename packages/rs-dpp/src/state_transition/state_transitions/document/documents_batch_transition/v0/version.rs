use crate::state_transition::state_transitions::document::documents_batch_transition::DocumentsBatchTransitionV0;
use crate::state_transition::FeatureVersioned;
use versioned_feature_core::FeatureVersion;

impl FeatureVersioned for DocumentsBatchTransitionV0 {
    fn feature_version(&self) -> FeatureVersion {
        0
    }
}
