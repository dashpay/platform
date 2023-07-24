use crate::state_transition::documents_batch_transition::DocumentsBatchTransition;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for DocumentsBatchTransition {
    fn feature_version(&self) -> FeatureVersion {
        match self {
            DocumentsBatchTransition::V0(v0) => v0.feature_version(),
        }
    }
}
