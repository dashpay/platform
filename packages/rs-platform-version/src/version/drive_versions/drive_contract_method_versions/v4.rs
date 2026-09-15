use crate::version::drive_versions::drive_contract_method_versions::v3::DRIVE_CONTRACT_METHOD_VERSIONS_V3;
use crate::version::drive_versions::drive_contract_method_versions::{
    DriveContractInsertMethodVersions, DriveContractMethodVersions,
    DriveContractUpdateMethodVersions,
};

/// Drive contract methods for protocol v14+.
///
/// Identical to [`super::v3::DRIVE_CONTRACT_METHOD_VERSIONS_V3`] except
/// `insert.add_contract_to_storage` is bumped to `1`, and `insert_contract`
/// and `update_contract` are bumped to `2`.
///
/// The v1 storage writer stores, beside the contract, a four-byte item holding
/// the contract's version number (`[64, id] / 2`) on every contract create and
/// update. That item is what lets `getDataContractsLatestVersions` read and
/// prove a contract's version without the contract bytes. Contracts stored
/// before this version get their item on the first block of protocol version
/// 14 (`Drive::add_version_items_to_all_contracts`).
pub const DRIVE_CONTRACT_METHOD_VERSIONS_V4: DriveContractMethodVersions =
    DriveContractMethodVersions {
        insert: DriveContractInsertMethodVersions {
            add_contract_to_storage: 1,
            // Creates the per-type history tree for keep-history document types.
            insert_contract: 2,
            ..DRIVE_CONTRACT_METHOD_VERSIONS_V3.insert
        },
        update: DriveContractUpdateMethodVersions {
            // Creates the per-type history tree for keep-history document types
            // a contract update adds.
            update_contract: 2,
            ..DRIVE_CONTRACT_METHOD_VERSIONS_V3.update
        },
        ..DRIVE_CONTRACT_METHOD_VERSIONS_V3
    };
