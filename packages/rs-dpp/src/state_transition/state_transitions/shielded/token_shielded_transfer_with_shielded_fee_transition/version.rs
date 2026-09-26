use crate::state_transition::token_shielded_transfer_with_shielded_fee_transition::TokenShieldedTransferWithShieldedFeeTransition;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for TokenShieldedTransferWithShieldedFeeTransition {
    fn feature_version(&self) -> FeatureVersion {
        match self {
            TokenShieldedTransferWithShieldedFeeTransition::V0(v0) => v0.feature_version(),
        }
    }
}
