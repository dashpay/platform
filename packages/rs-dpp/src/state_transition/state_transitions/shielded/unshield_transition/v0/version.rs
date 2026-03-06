use crate::state_transition::unshield_transition::v0::UnshieldTransitionV0;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for UnshieldTransitionV0 {
    fn feature_version(&self) -> FeatureVersion {
        0
    }
}
