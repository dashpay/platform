use crate::state_transition::shielded_transfer_transition::v0::ShieldedTransferTransitionV0;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for ShieldedTransferTransitionV0 {
    fn feature_version(&self) -> FeatureVersion {
        0
    }
}
