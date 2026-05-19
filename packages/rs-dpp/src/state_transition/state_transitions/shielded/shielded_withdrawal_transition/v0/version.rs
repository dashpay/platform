use crate::state_transition::shielded_withdrawal_transition::v0::ShieldedWithdrawalTransitionV0;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for ShieldedWithdrawalTransitionV0 {
    fn feature_version(&self) -> FeatureVersion {
        0
    }
}
