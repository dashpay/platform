use crate::state_transition::address_funding_from_asset_lock_transition::AddressFundingFromAssetLockTransition;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for AddressFundingFromAssetLockTransition {
    fn feature_version(&self) -> FeatureVersion {
        match self {
            AddressFundingFromAssetLockTransition::V0(v0) => v0.feature_version(),
        }
    }
}
