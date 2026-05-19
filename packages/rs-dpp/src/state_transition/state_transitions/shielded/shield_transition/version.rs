use crate::state_transition::shield_transition::ShieldTransition;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for ShieldTransition {
    fn feature_version(&self) -> FeatureVersion {
        match self {
            ShieldTransition::V0(v0) => v0.feature_version(),
        }
    }
}
