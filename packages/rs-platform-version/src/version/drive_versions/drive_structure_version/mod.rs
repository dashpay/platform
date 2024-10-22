use versioned_feature_core::FeatureVersionBounds;

pub mod v1;

#[derive(Clone, Debug, Default)]
pub struct DriveStructureVersion {
    pub document_indexes: FeatureVersionBounds,
    pub identity_indexes: FeatureVersionBounds,
    pub pools: FeatureVersionBounds,
}