use versioned_feature_core::FeatureVersionBounds;
use crate::version::drive_versions::drive_structure_version::DriveStructureVersion;

pub const DRIVE_STRUCTURE_V1: DriveStructureVersion = DriveStructureVersion {
    document_indexes: FeatureVersionBounds {
        min_version: 0,
        max_version: 0,
        default_current_version: 0,
    },
    identity_indexes: FeatureVersionBounds {
        min_version: 0,
        max_version: 0,
        default_current_version: 0,
    },
    pools: FeatureVersionBounds {
        min_version: 0,
        max_version: 0,
        default_current_version: 0,
    },
};