use crate::state_transition::FeatureVersioned;
use crate::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;
use crate::version::FeatureVersion;

impl FeatureVersioned for IdentityCreditWithdrawalTransition {
    fn feature_version(&self) -> FeatureVersion {
        match self { IdentityCreditWithdrawalTransition::V0(v0) => v0.feature_version() }
    }
}
