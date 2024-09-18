use crate::state_transition::identity_credit_withdrawal_transition::v0::IdentityCreditWithdrawalTransitionV0;
use crate::state_transition::FeatureVersioned;
use crate::state_transition::identity_credit_withdrawal_transition::v1::IdentityCreditWithdrawalTransitionV1;
use crate::version::FeatureVersion;

impl FeatureVersioned for IdentityCreditWithdrawalTransitionV1 {
    fn feature_version(&self) -> FeatureVersion {
        1
    }
}
