use crate::state_transition::identity_credit_withdrawal_transition::v1::IdentityCreditWithdrawalTransitionV1;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for IdentityCreditWithdrawalTransitionV1 {
    fn feature_version(&self) -> FeatureVersion {
        1
    }
}
