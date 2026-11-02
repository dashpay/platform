use crate::state_transition::token_unshield_with_shielded_fee_transition::TokenUnshieldWithShieldedFeeTransition;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for TokenUnshieldWithShieldedFeeTransition {
    fn feature_version(&self) -> FeatureVersion {
        match self {
            TokenUnshieldWithShieldedFeeTransition::V0(v0) => v0.feature_version(),
        }
    }
}
