use crate::state_transition::address_funds_transfer_transition::v0::AddressFundsTransferTransitionV0;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for AddressFundsTransferTransitionV0 {
    fn feature_version(&self) -> FeatureVersion {
        0
    }
}
