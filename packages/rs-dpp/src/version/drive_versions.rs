use crate::version::{FeatureVersion, FeatureVersionBounds};

#[derive(Clone, Copy, Debug, Default)]
pub struct DriveVersion {
    pub structure: DriveStructureVersion,
    pub methods: DriveMethodVersions,
    pub grove_methods: DriveGroveMethodVersions,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DriveMethodVersions {
    pub initialization: DriveInitializationMethodVersions,
    pub credit_pools: DriveCreditPoolMethodVersions,
    pub protocol_upgrade: DriveProtocolUpgradeVersions,
    pub balances: DriveBalancesMethodVersions,
    pub document: DriveDocumentMethodVersions,
    pub contract: DriveContractMethodVersions,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DriveGroveMethodVersions {
    pub basic: DriveGroveBasicMethodVersions,
    pub batch: DriveGroveBatchMethodVersions,
    pub apply: DriveGroveApplyMethodVersions,
    pub costs: DriveGroveCostMethodVersions,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DriveBalancesMethodVersions {
    pub add_to_system_credits: FeatureVersion,
    pub add_to_system_credits_operations: FeatureVersion,
    pub remove_from_system_credits: FeatureVersion,
    pub remove_from_system_credits_operations: FeatureVersion,
    pub calculate_total_credits_balance: FeatureVersion,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DriveDocumentMethodVersions {
    pub delete: DriveDocumentDeleteMethodVersions,
    pub insert: DriveDocumentInsertMethodVersions,
    pub update: DriveDocumentUpdateMethodVersions,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DriveContractMethodVersions {
    pub prove: DriveContractProveMethodVersions,
    pub costs: DriveContractCostsMethodVersions,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DriveContractProveMethodVersions {
    pub prove_contract: FeatureVersion,
    pub prove_contract_history: FeatureVersion,
    pub prove_contracts: FeatureVersion,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DriveContractQueryMethodVersions {
    pub fetch_contract_query: FeatureVersion,
    pub fetch_contract_with_history_latest_query: FeatureVersion,
    pub fetch_contracts_query: FeatureVersion,
    pub fetch_contract_history_query: FeatureVersion,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DriveContractCostsMethodVersions {
    pub add_estimation_costs_for_contract_insertion: FeatureVersion,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DriveDocumentInsertMethodVersions {
    pub add_document: FeatureVersion,
    pub add_document_for_contract: FeatureVersion,
    pub add_document_for_contract_apply_and_add_to_operations: FeatureVersion,
    pub add_document_for_contract_operations: FeatureVersion,
    pub add_document_to_primary_storage: FeatureVersion,
    pub add_indices_for_index_level_for_contract_operations: FeatureVersion,
    pub add_indices_for_top_index_level_for_contract_operations: FeatureVersion,
    pub add_reference_for_index_level_for_contract_operations: FeatureVersion,
    pub add_serialized_document_for_contract: FeatureVersion,
    pub add_serialized_document_for_contract_id: FeatureVersion,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DriveDocumentUpdateMethodVersions {
    pub add_update_multiple_documents_operations: FeatureVersion,
    pub update_document_for_contract: FeatureVersion,
    pub update_document_for_contract_apply_and_add_to_operations: FeatureVersion,
    pub update_document_for_contract_id: FeatureVersion,
    pub update_document_for_contract_operations: FeatureVersion,
    pub update_document_with_serialization_for_contract: FeatureVersion,
    pub update_serialized_document_for_contract: FeatureVersion,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DriveDocumentDeleteMethodVersions {
    pub add_estimation_costs_for_remove_document_to_primary_storage: FeatureVersion,
    pub delete_document_for_contract: FeatureVersion,
    pub delete_document_for_contract_id: FeatureVersion,
    pub delete_document_for_contract_apply_and_add_to_operations: FeatureVersion,
    pub remove_document_from_primary_storage: FeatureVersion,
    pub remove_reference_for_index_level_for_contract_operations: FeatureVersion,
    pub remove_indices_for_index_level_for_contract_operations: FeatureVersion,
    pub remove_indices_for_top_index_level_for_contract_operations: FeatureVersion,
    pub delete_document_for_contract_id_with_named_type_operations: FeatureVersion,
    pub delete_document_for_contract_with_named_type_operations: FeatureVersion,
    pub delete_document_for_contract_operations: FeatureVersion,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DriveGroveBasicMethodVersions {
    pub grove_insert: FeatureVersion,
    pub grove_insert_empty_tree: FeatureVersion,
    pub grove_insert_empty_sum_tree: FeatureVersion,
    pub grove_insert_if_not_exists: FeatureVersion,
    pub grove_delete: FeatureVersion,
    pub grove_get_raw: FeatureVersion,
    pub grove_get_raw_optional: FeatureVersion,
    pub grove_get_raw_value_u64_from_encoded_var_vec: FeatureVersion,
    pub grove_get: FeatureVersion,
    pub grove_get_path_query_serialized_results: FeatureVersion,
    pub grove_get_path_query: FeatureVersion,
    pub grove_get_path_query_with_optional: FeatureVersion,
    pub grove_get_raw_path_query_with_optional: FeatureVersion,
    pub grove_get_raw_path_query: FeatureVersion,
    pub grove_get_proved_path_query: FeatureVersion,
    pub grove_get_sum_tree_total_value: FeatureVersion,
    pub grove_has_raw: FeatureVersion,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DriveGroveBatchMethodVersions {
    pub batch_insert_empty_tree: FeatureVersion,
    pub batch_insert_empty_tree_if_not_exists: FeatureVersion,
    pub batch_insert_empty_tree_if_not_exists_check_existing_operations: FeatureVersion,
    pub batch_insert: FeatureVersion,
    pub batch_insert_if_not_exists: FeatureVersion,
    pub batch_insert_if_changed_value: FeatureVersion,
    pub batch_delete: FeatureVersion,
    pub batch_remove_raw: FeatureVersion,
    pub batch_delete_up_tree_while_empty: FeatureVersion,
    pub batch_refresh_reference: FeatureVersion,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DriveGroveApplyMethodVersions {
    pub grove_apply_operation: FeatureVersion,
    pub grove_apply_batch: FeatureVersion,
    pub grove_apply_batch_with_add_costs: FeatureVersion,
    pub grove_apply_partial_batch: FeatureVersion,
    pub grove_apply_partial_batch_with_add_costs: FeatureVersion,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DriveGroveCostMethodVersions {
    pub grove_batch_operations_costs: FeatureVersion,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DriveInitializationMethodVersions {
    pub create_initial_state_structure: FeatureVersion,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DriveCreditPoolMethodVersions {
    pub get_storage_credits_for_distribution_for_epochs_in_range: FeatureVersion
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DriveProtocolUpgradeVersions {
    pub clear_version_information: FeatureVersion,
    pub change_to_new_version_and_clear_version_information: FeatureVersion,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DriveStructureVersion {
    pub document_indexes: FeatureVersionBounds,
    pub identity_indexes: FeatureVersionBounds,
    pub pools: FeatureVersionBounds,
}