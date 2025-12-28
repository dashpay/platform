use crate::state_transition::identity_credit_transfer_to_addresses_transition::IdentityCreditTransferToAddressesTransition;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for IdentityCreditTransferToAddressesTransition {
    fn feature_version(&self) -> FeatureVersion {
        match self {
            IdentityCreditTransferToAddressesTransition::V0(v0) => v0.feature_version(),
        }
    }
}
