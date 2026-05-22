use crate::version::drive_versions::drive_contract_method_versions::{
    DriveContractApplyMethodVersions, DriveContractCostsMethodVersions,
    DriveContractGetMethodVersions, DriveContractInsertMethodVersions, DriveContractMethodVersions,
    DriveContractProveMethodVersions, DriveContractUpdateMethodVersions,
};

/// Drive contract methods for protocol v12+.
///
/// Identical to [`super::v2::DRIVE_CONTRACT_METHOD_VERSIONS_V2`] except
/// `costs.add_estimation_costs_for_contract_insertion` is bumped to `1`.
///
/// The v1 estimation method makes the per-doctype layer info reflect the
/// actual mix of count-bearing vs normal child subtrees — required for fee
/// accuracy once `documentsCountable` / `rangeCountable` doctypes (a v12+
/// feature) are exposed. For pre-v12 contracts (no countable flags) the v0
/// and v1 methods produce byte-identical results, so the bump only changes
/// observable fees for contracts that opt into the new flags.
pub const DRIVE_CONTRACT_METHOD_VERSIONS_V3: DriveContractMethodVersions =
    DriveContractMethodVersions {
        prove: DriveContractProveMethodVersions {
            prove_contract: 0,
            prove_contract_history: 0,
            prove_contracts: 0,
        },
        apply: DriveContractApplyMethodVersions {
            apply_contract: 0,
            apply_contract_with_serialization: 0,
        },
        insert: DriveContractInsertMethodVersions {
            add_contract_to_storage: 0,
            insert_contract: 1,
            add_description: 0,
            add_keywords: 0,
        },
        update: DriveContractUpdateMethodVersions {
            update_contract: 1,
            update_description: 0,
            update_keywords: 0,
        },
        costs: DriveContractCostsMethodVersions {
            add_estimation_costs_for_contract_insertion: 1, // <--- v12: count-tree-aware
        },
        get: DriveContractGetMethodVersions {
            fetch_contract: 0,
            fetch_contract_ids: 0,
            fetch_contracts: 0,
            fetch_contract_with_history: 0,
            get_cached_contract_with_fetch_info: 0,
            get_contract_with_fetch_info: 0,
            get_contracts_with_fetch_info: 0,
            get_system_or_user_contract_with_fee: 0,
        },
    };
