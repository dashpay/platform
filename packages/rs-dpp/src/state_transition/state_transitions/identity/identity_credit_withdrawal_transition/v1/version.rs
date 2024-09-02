use crate::state_transition::identity_credit_withdrawal_transition::v0::IdentityCreditWithdrawalTransitionV0;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for IdentityCreditWithdrawalTransitionV0 {
    fn feature_version(&self) -> FeatureVersion {
        0
    }
}
