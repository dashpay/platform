use crate::state_transition::shield_from_asset_lock_transition::ShieldFromAssetLockTransition;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for ShieldFromAssetLockTransition {
    fn feature_version(&self) -> FeatureVersion {
        match self {
            ShieldFromAssetLockTransition::V0(v0) => v0.feature_version(),
        }
    }
}
