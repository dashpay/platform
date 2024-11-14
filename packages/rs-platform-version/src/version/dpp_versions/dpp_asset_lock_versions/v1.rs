use crate::version::dpp_versions::dpp_asset_lock_versions::DPPAssetLockVersions;
use versioned_feature_core::FeatureVersionBounds;

pub const DPP_ASSET_LOCK_VERSIONS_V1: DPPAssetLockVersions = DPPAssetLockVersions {
    reduced_asset_lock_value: FeatureVersionBounds {
        min_version: 0,
        max_version: 0,
        default_current_version: 0,
    },
};
