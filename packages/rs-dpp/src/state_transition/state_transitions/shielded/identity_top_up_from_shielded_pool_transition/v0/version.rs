use crate::state_transition::identity_top_up_from_shielded_pool_transition::v0::IdentityTopUpFromShieldedPoolTransitionV0;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for IdentityTopUpFromShieldedPoolTransitionV0 {
    fn feature_version(&self) -> FeatureVersion {
        0
    }
}
