use crate::state_transition::shield_from_asset_lock_transition::v1::ShieldFromAssetLockTransitionV1;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for ShieldFromAssetLockTransitionV1 {
    fn feature_version(&self) -> FeatureVersion {
        1
    }
}
