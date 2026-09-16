use crate::version::drive_versions::drive_contract_method_versions::{
    DriveContractApplyMethodVersions, DriveContractCostsMethodVersions,
    DriveContractGetMethodVersions, DriveContractInsertMethodVersions, DriveContractMethodVersions,
    DriveContractProveMethodVersions, DriveContractUpdateMethodVersions,
};

/// Drive contract methods for the 5.0 protocol version (17).
///
/// Identical to [`super::v3::DRIVE_CONTRACT_METHOD_VERSIONS_V3`] except
/// `insert.insert_contract` is bumped to `2`: a contract that issues tokens
/// gets its lifecycle record (the per-issuer supply rollup) in the same batch
/// that creates its token trees, seeded with the sum of the base supplies.
pub const DRIVE_CONTRACT_METHOD_VERSIONS_V4: DriveContractMethodVersions =
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
            insert_contract: 2, // changed in v4: writes the issuer's lifecycle record
            add_description: 0,
            add_keywords: 0,
        },
        update: DriveContractUpdateMethodVersions {
            update_contract: 1,
            update_description: 0,
            update_keywords: 0,
        },
        costs: DriveContractCostsMethodVersions {
            add_estimation_costs_for_contract_insertion: 1,
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
