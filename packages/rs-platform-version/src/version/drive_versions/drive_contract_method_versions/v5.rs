use super::v4::DRIVE_CONTRACT_METHOD_VERSIONS_V4;
use super::{
    DriveContractInsertMethodVersions, DriveContractMethodVersions,
    DriveContractUpdateMethodVersions,
};

/// Drive contract methods for protocol v15+.
///
/// Creates the per-type history tree when a keep-history document type is
/// introduced by a contract create or update.
pub const DRIVE_CONTRACT_METHOD_VERSIONS_V5: DriveContractMethodVersions =
    DriveContractMethodVersions {
        insert: DriveContractInsertMethodVersions {
            insert_contract: 2,
            ..DRIVE_CONTRACT_METHOD_VERSIONS_V4.insert
        },
        update: DriveContractUpdateMethodVersions {
            update_contract: 2,
            ..DRIVE_CONTRACT_METHOD_VERSIONS_V4.update
        },
        ..DRIVE_CONTRACT_METHOD_VERSIONS_V4
    };
