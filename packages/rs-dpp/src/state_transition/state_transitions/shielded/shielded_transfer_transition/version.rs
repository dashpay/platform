use crate::state_transition::shielded_transfer_transition::ShieldedTransferTransition;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for ShieldedTransferTransition {
    fn feature_version(&self) -> FeatureVersion {
        match self {
            ShieldedTransferTransition::V0(v0) => v0.feature_version(),
        }
    }
}
