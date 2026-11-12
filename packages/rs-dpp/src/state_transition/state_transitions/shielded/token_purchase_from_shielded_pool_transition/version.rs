use crate::state_transition::token_purchase_from_shielded_pool_transition::TokenPurchaseFromShieldedPoolTransition;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for TokenPurchaseFromShieldedPoolTransition {
    fn feature_version(&self) -> FeatureVersion {
        match self {
            TokenPurchaseFromShieldedPoolTransition::V0(v0) => v0.feature_version(),
        }
    }
}
