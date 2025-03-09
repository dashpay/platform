use crate::state_transition::state_transitions::identity::identity_credit_withdrawal_transition::v0::IdentityCreditWithdrawalTransitionV0;
use crate::state_transition::FeatureVersioned;
use versioned_feature_core::FeatureVersion;

impl FeatureVersioned for IdentityCreditWithdrawalTransitionV0 {
    fn feature_version(&self) -> FeatureVersion {
        0
    }
}
