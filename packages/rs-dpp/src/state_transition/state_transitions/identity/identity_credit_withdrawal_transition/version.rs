use crate::state_transition::state_transitions::identity::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;
use crate::state_transition::FeatureVersioned;
use platform_version::version::FeatureVersion;

impl FeatureVersioned for IdentityCreditWithdrawalTransition {
    fn feature_version(&self) -> FeatureVersion {
        match self {
            IdentityCreditWithdrawalTransition::V0(v0) => v0.feature_version(),
        }
    }
}
