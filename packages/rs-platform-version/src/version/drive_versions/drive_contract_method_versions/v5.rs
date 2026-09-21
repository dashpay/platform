use super::v4::DRIVE_CONTRACT_METHOD_VERSIONS_V4;
use super::{
    DriveContractInsertMethodVersions, DriveContractMethodVersions,
    DriveContractUpdateMethodVersions,
};

/// Protocol 15 creates shielded pool storage for opted-in tokens at creation and update.
pub const DRIVE_CONTRACT_METHOD_VERSIONS_V5: DriveContractMethodVersions =
    DriveContractMethodVersions {
        insert: DriveContractInsertMethodVersions {
            insert_contract: 3,
            ..DRIVE_CONTRACT_METHOD_VERSIONS_V4.insert
        },
        update: DriveContractUpdateMethodVersions {
            update_contract: 3,
            ..DRIVE_CONTRACT_METHOD_VERSIONS_V4.update
        },
        ..DRIVE_CONTRACT_METHOD_VERSIONS_V4
    };
