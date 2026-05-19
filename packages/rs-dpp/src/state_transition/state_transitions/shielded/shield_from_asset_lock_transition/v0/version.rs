use crate::state_transition::shield_from_asset_lock_transition::v0::ShieldFromAssetLockTransitionV0;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for ShieldFromAssetLockTransitionV0 {
    fn feature_version(&self) -> FeatureVersion {
        0
    }
}
