use crate::state_transition::identity_credit_transfer_to_address_transition::IdentityCreditTransferToAddressTransition;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for IdentityCreditTransferToAddressTransition {
    fn feature_version(&self) -> FeatureVersion {
        match self {
            IdentityCreditTransferToAddressTransition::V0(v0) => v0.feature_version(),
        }
    }
}
