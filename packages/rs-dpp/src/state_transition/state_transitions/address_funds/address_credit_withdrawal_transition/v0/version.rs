use crate::state_transition::address_credit_withdrawal_transition::v0::AddressCreditWithdrawalTransitionV0;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for AddressCreditWithdrawalTransitionV0 {
    fn feature_version(&self) -> FeatureVersion {
        0
    }
}
