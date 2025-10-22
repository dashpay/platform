use crate::state_transition::identity_credit_transfer_to_single_key_transition::v0::IdentityCreditTransferToSingleKeyTransitionV0;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for IdentityCreditTransferToSingleKeyTransitionV0 {
    fn feature_version(&self) -> FeatureVersion {
        0
    }
}
