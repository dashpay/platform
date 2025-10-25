use crate::state_transition::identity_credit_transfer_to_address_transition::v0::IdentityCreditTransferToAddressTransitionV0;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for IdentityCreditTransferToAddressTransitionV0 {
    fn feature_version(&self) -> FeatureVersion {
        0
    }
}
