use crate::version::drive_versions::drive_contract_method_versions::{
    DriveContractApplyMethodVersions, DriveContractCostsMethodVersions,
    DriveContractGetMethodVersions, DriveContractInsertMethodVersions, DriveContractMethodVersions,
    DriveContractProveMethodVersions, DriveContractUpdateMethodVersions,
};

pub const DRIVE_CONTRACT_METHOD_VERSIONS_V1: DriveContractMethodVersions =
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
            insert_contract: 0,
        },
        update: DriveContractUpdateMethodVersions { update_contract: 0 },
        costs: DriveContractCostsMethodVersions {
            add_estimation_costs_for_contract_insertion: 0,
        },
        get: DriveContractGetMethodVersions {
            fetch_contract: 0,
            fetch_contract_with_history: 0,
            get_cached_contract_with_fetch_info: 0,
            get_contract_with_fetch_info: 0,
            get_contracts_with_fetch_info: 0,
        },
    };
