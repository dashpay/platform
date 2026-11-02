use crate::state_transition::token_purchase_from_shielded_pool_transition::v0::TokenPurchaseFromShieldedPoolTransitionV0;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for TokenPurchaseFromShieldedPoolTransitionV0 {
    fn feature_version(&self) -> FeatureVersion {
        0
    }
}
