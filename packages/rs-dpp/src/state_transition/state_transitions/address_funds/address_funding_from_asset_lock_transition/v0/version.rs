use crate::state_transition::address_funding_from_asset_lock_transition::v0::AddressFundingFromAssetLockTransitionV0;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for AddressFundingFromAssetLockTransitionV0 {
    fn feature_version(&self) -> FeatureVersion {
        0
    }
}
