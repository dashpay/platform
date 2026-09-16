use grovedb_version::version::FeatureVersion;

pub mod v1;

/// Drive method versions for contract groups: identity-owned sets of contracts, contract
/// document types and contract tokens under the `ContractGroups` root tree.
#[derive(Clone, Debug, Default)]
pub struct DriveContractGroupMethodVersions {
    pub insert: DriveContractGroupInsertMethodVersions,
    pub fetch: DriveContractGroupFetchMethodVersions,
    pub prove: DriveContractGroupProveMethodVersions,
    pub cost_estimation: DriveContractGroupCostEstimationMethodVersions,
}

#[derive(Clone, Debug, Default)]
pub struct DriveContractGroupInsertMethodVersions {
    pub insert_contract_group: FeatureVersion,
    pub insert_contract_group_memberships: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveContractGroupFetchMethodVersions {
    pub fetch_contract_group_info: FeatureVersion,
    pub fetch_contract_group: FeatureVersion,
    pub fetch_contract_group_memberships_for_contract: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveContractGroupProveMethodVersions {
    pub prove_contract_group: FeatureVersion,
    pub prove_contract_group_memberships_for_contract: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveContractGroupCostEstimationMethodVersions {
    pub for_insert_contract_group: FeatureVersion,
    pub for_insert_contract_group_memberships: FeatureVersion,
}
