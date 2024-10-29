use versioned_feature_core::FeatureVersionBounds;

pub mod v1;

#[derive(Clone, Debug, Default)]
pub struct DPPAssetLockVersions {
    pub reduced_asset_lock_value: FeatureVersionBounds,
}
