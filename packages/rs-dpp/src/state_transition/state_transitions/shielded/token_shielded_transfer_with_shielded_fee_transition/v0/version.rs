use crate::state_transition::token_shielded_transfer_with_shielded_fee_transition::v0::TokenShieldedTransferWithShieldedFeeTransitionV0;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for TokenShieldedTransferWithShieldedFeeTransitionV0 {
    fn feature_version(&self) -> FeatureVersion {
        0
    }
}
