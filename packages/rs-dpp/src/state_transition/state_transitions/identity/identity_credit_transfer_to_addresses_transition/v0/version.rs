use crate::state_transition::identity_credit_transfer_to_addresses_transition::v0::IdentityCreditTransferToAddressesTransitionV0;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for IdentityCreditTransferToAddressesTransitionV0 {
    fn feature_version(&self) -> FeatureVersion {
        0
    }
}
