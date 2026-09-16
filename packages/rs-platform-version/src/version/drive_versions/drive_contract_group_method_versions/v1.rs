use crate::version::drive_versions::drive_contract_group_method_versions::{
    DriveContractGroupCostEstimationMethodVersions, DriveContractGroupFetchMethodVersions,
    DriveContractGroupInsertMethodVersions, DriveContractGroupMethodVersions,
    DriveContractGroupProveMethodVersions,
};

pub const DRIVE_CONTRACT_GROUP_METHOD_VERSIONS_V1: DriveContractGroupMethodVersions =
    DriveContractGroupMethodVersions {
        insert: DriveContractGroupInsertMethodVersions {
            insert_contract_group: 0,
            insert_contract_group_memberships: 0,
        },
        fetch: DriveContractGroupFetchMethodVersions {
            fetch_contract_group_info: 0,
            fetch_contract_group: 0,
            fetch_contract_group_memberships_for_contract: 0,
        },
        prove: DriveContractGroupProveMethodVersions {
            prove_contract_group: 0,
            prove_contract_group_memberships_for_contract: 0,
        },
        cost_estimation: DriveContractGroupCostEstimationMethodVersions {
            for_insert_contract_group: 0,
            for_insert_contract_group_memberships: 0,
        },
    };
