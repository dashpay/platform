use crate::state_transition::shield_from_identity_transition::ShieldFromIdentityTransition;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for ShieldFromIdentityTransition {
    fn feature_version(&self) -> FeatureVersion {
        match self {
            ShieldFromIdentityTransition::V0(v0) => v0.feature_version(),
        }
    }
}
