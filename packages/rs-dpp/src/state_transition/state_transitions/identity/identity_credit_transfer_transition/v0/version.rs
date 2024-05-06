use crate::state_transition::state_transitions::identity::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0;
use crate::state_transition::FeatureVersioned;
use platform_version::version::FeatureVersion;

impl FeatureVersioned for IdentityCreditTransferTransitionV0 {
    fn feature_version(&self) -> FeatureVersion {
        0
    }
}
