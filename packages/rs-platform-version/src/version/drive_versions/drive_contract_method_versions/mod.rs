use versioned_feature_core::FeatureVersion;

pub mod v1;
pub mod v2;
pub mod v3;
pub mod v4;

#[derive(Clone, Debug, Default)]
pub struct DriveContractMethodVersions {
    pub prove: DriveContractProveMethodVersions,
    pub apply: DriveContractApplyMethodVersions,
    pub insert: DriveContractInsertMethodVersions,
    pub update: DriveContractUpdateMethodVersions,
    pub costs: DriveContractCostsMethodVersions,
    pub get: DriveContractGetMethodVersions,
    pub moderation: DriveContractModerationMethodVersions,
}

/// Drive methods for contract moderation: the banlist and the suspension list a moderated
/// contract keeps under its own subtree (keys `3` and `4`). Protocol version 14.
#[derive(Clone, Debug, Default)]
pub struct DriveContractModerationMethodVersions {
    pub add_contract_ban: FeatureVersion,
    pub remove_contract_ban: FeatureVersion,
    pub add_contract_suspension: FeatureVersion,
    pub remove_contract_suspension: FeatureVersion,
    pub fetch_contract_moderation_status: FeatureVersion,
    pub fetch_contract_moderation_entries: FeatureVersion,
    pub prove_contract_moderation_status: FeatureVersion,
    pub prove_contract_moderation_entries: FeatureVersion,
    pub insert_contract_moderation_trees: FeatureVersion,
    pub add_estimation_costs_for_contract_moderation_trees: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveContractProveMethodVersions {
    pub prove_contract: FeatureVersion,
    pub prove_contract_history: FeatureVersion,
    pub prove_contracts: FeatureVersion,
    pub prove_contracts_by_range: FeatureVersion,
    pub prove_contracts_versions: FeatureVersion,
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
    pub fetch_contract_ids: FeatureVersion,
    pub fetch_contract_version: FeatureVersion,
    pub fetch_contracts: FeatureVersion,
    pub fetch_contract_with_history: FeatureVersion,
    pub get_cached_contract_with_fetch_info: FeatureVersion,
    pub get_contract_with_fetch_info: FeatureVersion,
    pub get_contracts_with_fetch_info: FeatureVersion,
    pub get_system_or_user_contract_with_fee: FeatureVersion,
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
