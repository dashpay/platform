use crate::version::drive_abci_versions::drive_abci_query_versions::v2::DRIVE_ABCI_QUERY_VERSIONS_V2;
use crate::version::drive_abci_versions::drive_abci_query_versions::DriveAbciQueryVersions;
use versioned_feature_core::FeatureVersionBounds;

/// Composite history queries for the per-type retained history layout.
pub const DRIVE_ABCI_QUERY_VERSIONS_V3: DriveAbciQueryVersions = DriveAbciQueryVersions {
    document_history: FeatureVersionBounds {
        min_version: 1,
        max_version: 1,
        default_current_version: 1,
    },
    ..DRIVE_ABCI_QUERY_VERSIONS_V2
};
