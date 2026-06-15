use crate::state_transition::identity_create_from_shielded_pool_transition::v0::IdentityCreateFromShieldedPoolTransitionV0;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for IdentityCreateFromShieldedPoolTransitionV0 {
    fn feature_version(&self) -> FeatureVersion {
        0
    }
}
