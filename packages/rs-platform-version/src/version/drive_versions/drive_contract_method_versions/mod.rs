use versioned_feature_core::FeatureVersion;

pub mod v1;
pub mod v2;

#[derive(Clone, Debug, Default)]
pub struct DriveContractMethodVersions {
    pub prove: DriveContractProveMethodVersions,
    pub apply: DriveContractApplyMethodVersions,
    pub insert: DriveContractInsertMethodVersions,
    pub update: DriveContractUpdateMethodVersions,
    pub costs: DriveContractCostsMethodVersions,
    pub get: DriveContractGetMethodVersions,
}

#[derive(Clone, Debug, Default)]
pub struct DriveContractProveMethodVersions {
    pub prove_contract: FeatureVersion,
    pub prove_contract_history: FeatureVersion,
    pub prove_contracts: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveContractApplyMethodVersions {
    pub apply_contract: FeatureVersion,
    pub apply_contract_with_serialization: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveContractInsertMethodVersions {
    pub add_contract_to_storage: FeatureVersion,
    pub insert_contract: FeatureVersion,
    pub add_description: FeatureVersion,
    pub add_keywords: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveContractUpdateMethodVersions {
    pub update_contract: FeatureVersion,
    pub update_description: FeatureVersion,
    pub update_keywords: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveContractGetMethodVersions {
    pub fetch_contract: FeatureVersion,
    pub fetch_contract_with_history: FeatureVersion,
    pub get_cached_contract_with_fetch_info: FeatureVersion,
    pub get_contract_with_fetch_info: FeatureVersion,
    pub get_contracts_with_fetch_info: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveContractQueryMethodVersions {
    pub fetch_contract_query: FeatureVersion,
    pub fetch_contract_with_history_latest_query: FeatureVersion,
    pub fetch_contracts_query: FeatureVersion,
    pub fetch_contract_history_query: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveContractCostsMethodVersions {
    pub add_estimation_costs_for_contract_insertion: FeatureVersion,
}
