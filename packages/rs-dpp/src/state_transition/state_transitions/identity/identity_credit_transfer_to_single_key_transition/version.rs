use crate::state_transition::identity_credit_transfer_to_single_key_transition::IdentityCreditTransferToSingleKeyTransition;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for IdentityCreditTransferToSingleKeyTransition {
    fn feature_version(&self) -> FeatureVersion {
        match self {
            IdentityCreditTransferToSingleKeyTransition::V0(v0) => v0.feature_version(),
        }
    }
}
