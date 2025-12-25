use crate::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for AddressFundsTransferTransition {
    fn feature_version(&self) -> FeatureVersion {
        match self {
            AddressFundsTransferTransition::V0(v0) => v0.feature_version(),
        }
    }
}
