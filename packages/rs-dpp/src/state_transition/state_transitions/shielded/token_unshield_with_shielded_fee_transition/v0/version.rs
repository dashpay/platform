use crate::state_transition::token_unshield_with_shielded_fee_transition::v0::TokenUnshieldWithShieldedFeeTransitionV0;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for TokenUnshieldWithShieldedFeeTransitionV0 {
    fn feature_version(&self) -> FeatureVersion {
        0
    }
}
