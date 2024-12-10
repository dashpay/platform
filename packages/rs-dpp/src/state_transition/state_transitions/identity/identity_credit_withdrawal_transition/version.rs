use crate::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for IdentityCreditWithdrawalTransition {
    fn feature_version(&self) -> FeatureVersion {
        match self {
            IdentityCreditWithdrawalTransition::V0(v0) => v0.feature_version(),
            IdentityCreditWithdrawalTransition::V1(v1) => v1.feature_version(),
        }
    }
}
