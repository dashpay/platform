use crate::state_transition::identity_top_up_from_shielded_pool_transition::IdentityTopUpFromShieldedPoolTransition;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for IdentityTopUpFromShieldedPoolTransition {
    fn feature_version(&self) -> FeatureVersion {
        match self {
            IdentityTopUpFromShieldedPoolTransition::V0(v0) => v0.feature_version(),
        }
    }
}
