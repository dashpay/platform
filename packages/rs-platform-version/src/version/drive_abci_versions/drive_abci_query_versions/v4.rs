use crate::version::drive_abci_versions::drive_abci_query_versions::v3::DRIVE_ABCI_QUERY_VERSIONS_V3;
use crate::version::drive_abci_versions::drive_abci_query_versions::DriveAbciQueryVersions;

/// Version 4 of the Drive ABCI query versions.
///
/// Differs from v3 in one slot: `document_history_processing` is 1, the
/// history handler that reports the deleted and erasing lifecycle states of a
/// keep-history document with their times. The wire version of the request and
/// response is unchanged.
pub const DRIVE_ABCI_QUERY_VERSIONS_V4: DriveAbciQueryVersions = DriveAbciQueryVersions {
    document_history_processing: 1,
    ..DRIVE_ABCI_QUERY_VERSIONS_V3
};
