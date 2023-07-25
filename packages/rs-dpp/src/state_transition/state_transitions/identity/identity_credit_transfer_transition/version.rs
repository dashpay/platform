use crate::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for IdentityCreditTransferTransition {
    fn feature_version(&self) -> FeatureVersion {
        match self {
            IdentityCreditTransferTransition::V0(v0) => v0.feature_version(),
        }
    }
}
