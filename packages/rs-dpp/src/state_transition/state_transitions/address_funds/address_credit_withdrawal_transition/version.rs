use crate::state_transition::address_credit_withdrawal_transition::AddressCreditWithdrawalTransition;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for AddressCreditWithdrawalTransition {
    fn feature_version(&self) -> FeatureVersion {
        match self {
            AddressCreditWithdrawalTransition::V0(v0) => v0.feature_version(),
        }
    }
}
