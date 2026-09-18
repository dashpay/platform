use crate::version::drive_versions::drive_contract_method_versions::v3::DRIVE_CONTRACT_METHOD_VERSIONS_V3;
use crate::version::drive_versions::drive_contract_method_versions::{
    DriveContractInsertMethodVersions, DriveContractMethodVersions,
    DriveContractModerationMethodVersions, DriveContractUpdateMethodVersions,
};

/// Drive contract methods for protocol v14+.
///
/// Relative to [`super::v3::DRIVE_CONTRACT_METHOD_VERSIONS_V3`]:
///
/// * `insert.add_contract_to_storage` is bumped to `1`. The v1 storage writer stores, beside
///   the contract, a four-byte item holding the contract's version number (`[64, id] / 2`) on
///   every contract create and update. That item is what lets
///   `getDataContractsLatestVersions` read and prove a contract's version without the contract
///   bytes. Contracts stored before this version get their item on the first block of protocol
///   version 14 (`Drive::add_version_items_to_all_contracts`).
/// * `insert.insert_contract` and `update.update_contract` are bumped to `2`: a contract whose
///   config declares moderation gets its banlist (`[64, id] / 3`) and suspension list
///   (`[64, id] / 4`) trees created at insertion, or by the update that turns a list on.
/// * The `moderation` table is new: the ban and suspension entry writers, readers and provers.
pub const DRIVE_CONTRACT_METHOD_VERSIONS_V4: DriveContractMethodVersions =
    DriveContractMethodVersions {
        insert: DriveContractInsertMethodVersions {
            add_contract_to_storage: 1,
            insert_contract: 2,
            ..DRIVE_CONTRACT_METHOD_VERSIONS_V3.insert
        },
        update: DriveContractUpdateMethodVersions {
            update_contract: 2,
            ..DRIVE_CONTRACT_METHOD_VERSIONS_V3.update
        },
        moderation: DriveContractModerationMethodVersions {
            add_contract_ban: 0,
            remove_contract_ban: 0,
            add_contract_suspension: 0,
            remove_contract_suspension: 0,
            fetch_contract_moderation_status: 0,
            fetch_contract_moderation_entries: 0,
            prove_contract_moderation_status: 0,
            prove_contract_moderation_entries: 0,
            insert_contract_moderation_trees: 0,
            add_estimation_costs_for_contract_moderation_trees: 0,
        },
        ..DRIVE_CONTRACT_METHOD_VERSIONS_V3
    };
