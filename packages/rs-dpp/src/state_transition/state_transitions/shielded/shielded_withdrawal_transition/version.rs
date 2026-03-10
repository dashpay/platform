use crate::state_transition::shielded_withdrawal_transition::ShieldedWithdrawalTransition;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for ShieldedWithdrawalTransition {
    fn feature_version(&self) -> FeatureVersion {
        match self {
            ShieldedWithdrawalTransition::V0(v0) => v0.feature_version(),
        }
    }
}
