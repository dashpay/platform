use crate::state_transition::documents_batch_transition::DocumentsBatchTransitionV0;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for DocumentsBatchTransitionV0 {
    fn feature_version(&self) -> FeatureVersion {
        0
    }
}
