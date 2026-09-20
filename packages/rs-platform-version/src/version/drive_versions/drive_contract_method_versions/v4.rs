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
///   the contract, a four-byte item holding the contract's version number (`[64, id, 2] / 64`) on
///   every contract create and update. That item is what lets
///   `getDataContractsLatestVersions` read and prove a contract's version without the contract
///   bytes. Contracts stored before this version get their item on the first block of protocol
///   version 14 (`Drive::add_version_items_to_all_contracts`).
/// * `update.update_contract` is bumped to `2`. The v2 contract update creates the perpetual
///   and pre-programmed distribution storage of a token the update adds, as the contract
///   insert always has for a token present at registration. v1 created none of it, so a claim
///   on such a token failed as an internal error and the distribution was unclaimable. There
///   is no backfill for tokens added by an update before this version: mainnet has none
///   (checked at block 436796, where no contract update ever carried a token and every
///   token's contract is still at version 1). It also mints the base supply of a token the
///   update adds, to the identity the contract insert credits at registration (the token's
///   new tokens destination identity, else the contract owner), and starts the token's total
///   supply at that amount. v1 left such a token at a total supply of zero with nobody
///   holding any of it. Nothing is minted retroactively for a token added by an update
///   before this version.
/// * `insert.insert_contract` is bumped to `2`: a contract whose config declares moderation
///   gets its banlist (`[64, id, 2] / 128`) and suspension list (`[64, id, 2] / 192`) trees created at
///   insertion. Which lists a contract keeps never changes afterwards, so a contract update
///   creates none.
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
