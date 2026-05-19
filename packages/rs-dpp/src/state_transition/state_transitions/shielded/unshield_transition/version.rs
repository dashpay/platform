use crate::state_transition::unshield_transition::UnshieldTransition;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for UnshieldTransition {
    fn feature_version(&self) -> FeatureVersion {
        match self {
            UnshieldTransition::V0(v0) => v0.feature_version(),
        }
    }
}
