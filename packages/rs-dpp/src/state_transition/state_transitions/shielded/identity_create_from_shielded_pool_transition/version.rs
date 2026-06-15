use crate::state_transition::identity_create_from_shielded_pool_transition::IdentityCreateFromShieldedPoolTransition;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for IdentityCreateFromShieldedPoolTransition {
    fn feature_version(&self) -> FeatureVersion {
        match self {
            IdentityCreateFromShieldedPoolTransition::V0(v0) => v0.feature_version(),
        }
    }
}
