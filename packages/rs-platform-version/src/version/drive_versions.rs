use crate::version::{FeatureVersion, FeatureVersionBounds, OptionalFeatureVersion};

#[derive(Clone, Debug, Default)]
pub struct DriveVersion {
    pub structure: DriveStructureVersion,
    pub methods: DriveMethodVersions,
    pub grove_methods: DriveGroveMethodVersions,
}

impl DriveVersion {
    pub fn latest() -> DriveVersion {
        DriveVersion::default()
    }
}

#[derive(Clone, Debug, Default)]
pub struct DriveMethodVersions {
    pub initialization: DriveInitializationMethodVersions,
    pub credit_pools: DriveCreditPoolMethodVersions,
    pub protocol_upgrade: DriveProtocolUpgradeVersions,
    pub balances: DriveBalancesMethodVersions,
    pub document: DriveDocumentMethodVersions,
    pub contract: DriveContractMethodVersions,
    pub fees: DriveFeesMethodVersions,
    pub estimated_costs: DriveEstimatedCostsMethodVersions,
    pub asset_lock: DriveAssetLockMethodVersions,
    pub verify: DriveVerifyMethodVersions,
    pub identity: DriveIdentityMethodVersions,
    pub platform_system: DrivePlatformSystemMethodVersions,
    pub operations: DriveOperationsMethodVersion,
    pub batch_operations: DriveBatchOperationsMethodVersion,
    pub prove: DriveProveMethodVersions,
    pub state_transitions: DriveStateTransitionMethodVersions,
    pub platform_state: DrivePlatformStateMethodVersions,
}

#[derive(Clone, Debug, Default)]
pub struct DrivePlatformStateMethodVersions {
    pub fetch_platform_state_bytes: FeatureVersion,
    pub store_platform_state_bytes: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveStateTransitionMethodVersions {
    pub operations: DriveStateTransitionOperationMethodVersions,
}

#[derive(Clone, Debug, Default)]
pub struct DriveStateTransitionOperationMethodVersions {
    pub finalization_tasks: FeatureVersion,
    pub contracts: DriveDataContractOperationMethodVersions,
}

#[derive(Clone, Debug, Default)]
pub struct DriveDataContractOperationMethodVersions {
    pub finalization_tasks: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveProveMethodVersions {
    pub prove_multiple: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveVerifyMethodVersions {
    pub contract: DriveVerifyContractMethodVersions,
    pub document: DriveVerifyDocumentMethodVersions,
    pub identity: DriveVerifyIdentityMethodVersions,
    pub single_document: DriveVerifySingleDocumentMethodVersions,
    pub system: DriveVerifySystemMethodVersions,
    pub state_transition: DriveVerifyStateTransitionMethodVersions,
}

#[derive(Clone, Debug, Default)]
pub struct DriveVerifyContractMethodVersions {
    pub verify_contract: FeatureVersion,
    pub verify_contract_history: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveVerifyDocumentMethodVersions {
    pub verify_proof: FeatureVersion,
    pub verify_proof_keep_serialized: FeatureVersion,
    pub verify_start_at_document_in_proof: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveVerifyIdentityMethodVersions {
    pub verify_full_identities_by_public_key_hashes: FeatureVersion,
    pub verify_full_identity_by_identity_id: FeatureVersion,
    pub verify_full_identity_by_public_key_hash: FeatureVersion,
    pub verify_identity_balance_for_identity_id: FeatureVersion,
    pub verify_identity_balances_for_identity_ids: FeatureVersion,
    pub verify_identity_id_by_public_key_hash: FeatureVersion,
    pub verify_identity_ids_by_public_key_hashes: FeatureVersion,
    pub verify_identity_keys_by_identity_id: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveVerifySystemMethodVersions {
    pub verify_epoch_infos: FeatureVersion,
    pub verify_upgrade_state: FeatureVersion,
    pub verify_upgrade_vote_status: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveVerifySingleDocumentMethodVersions {
    pub verify_proof: FeatureVersion,
    pub verify_proof_keep_serialized: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveVerifyStateTransitionMethodVersions {
    pub verify_state_transition_was_executed_with_proof: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveGroveMethodVersions {
    pub basic: DriveGroveBasicMethodVersions,
    pub batch: DriveGroveBatchMethodVersions,
    pub apply: DriveGroveApplyMethodVersions,
    pub costs: DriveGroveCostMethodVersions,
}

#[derive(Clone, Debug, Default)]
pub struct DriveBalancesMethodVersions {
    pub add_to_system_credits: FeatureVersion,
    pub add_to_system_credits_operations: FeatureVersion,
    pub remove_from_system_credits: FeatureVersion,
    pub remove_from_system_credits_operations: FeatureVersion,
    pub calculate_total_credits_balance: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveAssetLockMethodVersions {
    pub add_asset_lock_outpoint: FeatureVersion,
    pub add_estimation_costs_for_adding_asset_lock: FeatureVersion,
    pub has_asset_lock_outpoint: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveFeesMethodVersions {
    pub calculate_fee: FeatureVersion,
}

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
}

#[derive(Clone, Debug, Default)]
pub struct DriveContractUpdateMethodVersions {
    pub update_contract: FeatureVersion,
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

#[derive(Clone, Debug, Default)]
pub struct DrivePlatformSystemMethodVersions {
    pub protocol_version: DriveSystemProtocolVersionMethodVersions,
    pub estimation_costs: DriveSystemEstimationCostsMethodVersions,
}

#[derive(Clone, Debug, Default)]
pub struct DriveOperationsMethodVersion {
    pub rollback_transaction: FeatureVersion,
    pub drop_cache: FeatureVersion,
    pub commit_transaction: FeatureVersion,
    pub apply_partial_batch_low_level_drive_operations: FeatureVersion,
    pub apply_partial_batch_grovedb_operations: FeatureVersion,
    pub apply_batch_low_level_drive_operations: FeatureVersion,
    pub apply_batch_grovedb_operations: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveBatchOperationsMethodVersion {
    pub convert_drive_operations_to_grove_operations: FeatureVersion,
    pub apply_drive_operations: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveSystemProtocolVersionMethodVersions {
    pub fetch_next_protocol_version: FeatureVersion,
    pub set_next_protocol_version_operations: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveSystemEstimationCostsMethodVersions {
    pub for_total_system_credits_update: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveDocumentMethodVersions {
    pub query: DriveDocumentQueryMethodVersions,
    pub delete: DriveDocumentDeleteMethodVersions,
    pub insert: DriveDocumentInsertMethodVersions,
    pub update: DriveDocumentUpdateMethodVersions,
    pub estimation_costs: DriveDocumentEstimationCostsMethodVersions,
    pub index_uniqueness: DriveDocumentIndexUniquenessMethodVersions,
}

#[derive(Clone, Debug, Default)]
pub struct DriveDocumentQueryMethodVersions {
    pub query_documents: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveDocumentEstimationCostsMethodVersions {
    pub add_estimation_costs_for_add_document_to_primary_storage: FeatureVersion,
    pub stateless_delete_of_non_tree_for_costs: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveDocumentInsertMethodVersions {
    pub add_document: FeatureVersion,
    pub add_document_for_contract: FeatureVersion,
    pub add_document_for_contract_apply_and_add_to_operations: FeatureVersion,
    pub add_document_for_contract_operations: FeatureVersion,
    pub add_document_to_primary_storage: FeatureVersion,
    pub add_indices_for_index_level_for_contract_operations: FeatureVersion,
    pub add_indices_for_top_index_level_for_contract_operations: FeatureVersion,
    pub add_reference_for_index_level_for_contract_operations: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveDocumentUpdateMethodVersions {
    pub add_update_multiple_documents_operations: FeatureVersion,
    pub update_document_for_contract: FeatureVersion,
    pub update_document_for_contract_apply_and_add_to_operations: FeatureVersion,
    pub update_document_for_contract_id: FeatureVersion,
    pub update_document_for_contract_operations: FeatureVersion,
    pub update_document_with_serialization_for_contract: FeatureVersion,
    pub update_serialized_document_for_contract: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
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

#[derive(Clone, Debug, Default)]
pub struct DriveDocumentIndexUniquenessMethodVersions {
    pub validate_document_uniqueness: FeatureVersion,
    pub validate_document_create_transition_action_uniqueness: FeatureVersion,
    pub validate_document_replace_transition_action_uniqueness: FeatureVersion,
    pub validate_uniqueness_of_data: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveGroveBasicMethodVersions {
    pub grove_insert: FeatureVersion,
    pub grove_insert_empty_tree: FeatureVersion,
    pub grove_insert_empty_sum_tree: FeatureVersion,
    pub grove_insert_if_not_exists: FeatureVersion,
    pub grove_clear: FeatureVersion,
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

#[derive(Clone, Debug, Default)]
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

#[derive(Clone, Debug, Default)]
pub struct DriveGroveApplyMethodVersions {
    pub grove_apply_operation: FeatureVersion,
    pub grove_apply_batch: FeatureVersion,
    pub grove_apply_batch_with_add_costs: FeatureVersion,
    pub grove_apply_partial_batch: FeatureVersion,
    pub grove_apply_partial_batch_with_add_costs: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveGroveCostMethodVersions {
    pub grove_batch_operations_costs: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveInitializationMethodVersions {
    pub create_initial_state_structure: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveCreditPoolMethodVersions {
    pub epochs: DriveCreditPoolEpochsMethodVersions,
    pub pending_epoch_refunds: DriveCreditPoolPendingEpochRefundsMethodVersions,
    pub storage_fee_distribution_pool: DriveCreditPoolStorageFeeDistributionPoolMethodVersions,
}

#[derive(Clone, Debug, Default)]
pub struct DriveCreditPoolEpochsMethodVersions {
    pub get_epochs_infos: FeatureVersion,
    pub prove_epochs_infos: FeatureVersion,
    pub get_epoch_fee_multiplier: FeatureVersion,
    pub get_epoch_processing_credits_for_distribution: FeatureVersion,
    pub get_epoch_storage_credits_for_distribution: FeatureVersion,
    pub get_epoch_total_credits_for_distribution: FeatureVersion,
    pub get_storage_credits_for_distribution_for_epochs_in_range: FeatureVersion,
    pub get_epoch_start_time: FeatureVersion,
    pub get_epoch_start_block_core_height: FeatureVersion,
    pub get_epoch_start_block_height: FeatureVersion,
    pub get_first_epoch_start_block_info_between_epochs: FeatureVersion,
    pub get_epoch_proposers: FeatureVersion,
    pub get_epochs_proposer_block_count: FeatureVersion,
    pub add_update_pending_epoch_refunds_operations: FeatureVersion,
    pub is_epochs_proposers_tree_empty: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveCreditPoolPendingEpochRefundsMethodVersions {
    pub add_delete_pending_epoch_refunds_except_specified: FeatureVersion,
    pub fetch_and_add_pending_epoch_refunds_to_collection: FeatureVersion,
    pub fetch_pending_epoch_refunds: FeatureVersion,
    pub add_update_pending_epoch_refunds_operations: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveCreditPoolStorageFeeDistributionPoolMethodVersions {
    pub get_storage_fees_from_distribution_pool: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveProtocolUpgradeVersions {
    pub clear_version_information: FeatureVersion,
    pub change_to_new_version_and_clear_version_information: FeatureVersion,
    pub fetch_versions_with_counter: FeatureVersion,
    pub fetch_proved_versions_with_counter: FeatureVersion,
    pub fetch_validator_version_votes: FeatureVersion,
    pub fetch_proved_validator_version_votes: FeatureVersion,
    pub remove_validators_proposed_app_versions: FeatureVersion,
    pub update_validator_proposed_app_version: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveStructureVersion {
    pub document_indexes: FeatureVersionBounds,
    pub identity_indexes: FeatureVersionBounds,
    pub pools: FeatureVersionBounds,
}

#[derive(Clone, Debug, Default)]
pub struct DriveIdentityMethodVersions {
    pub fetch: DriveIdentityFetchMethodVersions,
    pub prove: DriveIdentityProveMethodVersions,
    pub keys: DriveIdentityKeysMethodVersions,
    pub update: DriveIdentityUpdateMethodVersions,
    pub insert: DriveIdentityInsertMethodVersions,
    pub contract_info: DriveIdentityContractInfoMethodVersions,
    pub cost_estimation: DriveIdentityCostEstimationMethodVersions,
}

#[derive(Clone, Debug, Default)]
pub struct DriveIdentityContractInfoMethodVersions {
    pub add_potential_contract_info_for_contract_bounded_key: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveIdentityCostEstimationMethodVersions {
    pub for_authentication_keys_security_level_in_key_reference_tree: FeatureVersion,
    pub for_balances: FeatureVersion,
    pub for_contract_info: FeatureVersion,
    pub for_contract_info_group: FeatureVersion,
    pub for_contract_info_group_key_purpose: FeatureVersion,
    pub for_keys_for_identity_id: FeatureVersion,
    pub for_negative_credit: FeatureVersion,
    pub for_purpose_in_key_reference_tree: FeatureVersion,
    pub for_root_key_reference_tree: FeatureVersion,
    pub for_update_revision: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveIdentityFetchMethodVersions {
    pub public_key_hashes: DriveIdentityFetchPublicKeyHashesMethodVersions,
    pub attributes: DriveIdentityFetchAttributesMethodVersions,
    pub partial_identity: DriveIdentityFetchPartialIdentityMethodVersions,
    pub full_identity: DriveIdentityFetchFullIdentityMethodVersions,
}

#[derive(Clone, Debug, Default)]
pub struct DriveIdentityFetchPublicKeyHashesMethodVersions {
    pub fetch_full_identities_by_unique_public_key_hashes: FeatureVersion,
    pub fetch_full_identity_by_unique_public_key_hash: FeatureVersion,
    pub fetch_identity_id_by_unique_public_key_hash: FeatureVersion,
    pub fetch_identity_ids_by_non_unique_public_key_hash: FeatureVersion,
    pub fetch_identity_ids_by_unique_public_key_hashes: FeatureVersion,
    pub fetch_serialized_full_identity_by_unique_public_key_hash: FeatureVersion,
    pub has_any_of_unique_public_key_hashes: FeatureVersion,
    pub has_non_unique_public_key_hash: FeatureVersion,
    pub has_non_unique_public_key_hash_already_for_identity: FeatureVersion,
    pub has_unique_public_key_hash: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveIdentityFetchAttributesMethodVersions {
    pub revision: FeatureVersion,
    pub balance: FeatureVersion,
    pub balance_include_debt: FeatureVersion,
    pub negative_balance: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveIdentityFetchFullIdentityMethodVersions {
    pub fetch_full_identity: OptionalFeatureVersion,
    pub fetch_full_identities: OptionalFeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveIdentityFetchPartialIdentityMethodVersions {
    pub fetch_identity_balance_with_keys: FeatureVersion,
    pub fetch_identity_balance_with_keys_and_revision: FeatureVersion,
    pub fetch_identity_with_balance: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveIdentityProveMethodVersions {
    pub full_identity: FeatureVersion,
    pub full_identities: FeatureVersion,
    pub prove_full_identities_by_unique_public_key_hashes: FeatureVersion,
    pub prove_full_identity_by_unique_public_key_hash: FeatureVersion,
    pub prove_identity_id_by_unique_public_key_hash: FeatureVersion,
    pub prove_identity_ids_by_unique_public_key_hashes: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveIdentityKeysMethodVersions {
    pub fetch: DriveIdentityKeysFetchMethodVersions,
    pub prove: DriveIdentityKeysProveMethodVersions,
    pub insert: DriveIdentityKeysInsertMethodVersions,
    pub insert_key_hash_identity_reference: DriveIdentityKeyHashesToIdentityInsertMethodVersions,
}

#[derive(Clone, Debug, Default)]
pub struct DriveIdentityKeysFetchMethodVersions {
    pub fetch_all_current_identity_keys: FeatureVersion,
    pub fetch_all_identity_keys: FeatureVersion,
    pub fetch_identities_all_keys: FeatureVersion,
    pub fetch_identity_keys: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveIdentityKeysProveMethodVersions {
    pub prove_identities_all_keys: FeatureVersion,
    pub prove_identity_keys: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveIdentityKeysInsertMethodVersions {
    pub create_key_tree_with_keys: FeatureVersion,
    pub create_new_identity_key_query_trees: FeatureVersion,
    pub insert_key_searchable_references: FeatureVersion,
    pub insert_key_to_storage: FeatureVersion,
    pub insert_new_non_unique_key: FeatureVersion,
    pub insert_new_unique_key: FeatureVersion,
    pub replace_key_in_storage: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveIdentityKeyHashesToIdentityInsertMethodVersions {
    pub add_estimation_costs_for_insert_non_unique_public_key_hash_reference: FeatureVersion,
    pub add_estimation_costs_for_insert_unique_public_key_hash_reference: FeatureVersion,
    pub insert_non_unique_public_key_hash_reference_to_identity: FeatureVersion,
    pub insert_reference_to_non_unique_key: FeatureVersion,
    pub insert_reference_to_unique_key: FeatureVersion,
    pub insert_unique_public_key_hash_reference_to_identity: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveIdentityInsertMethodVersions {
    pub add_new_identity: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveIdentityUpdateMethodVersions {
    pub update_identity_revision: FeatureVersion,
    pub update_identity_negative_credit_operation: FeatureVersion,
    pub initialize_identity_revision: FeatureVersion,
    pub disable_identity_keys: FeatureVersion,
    pub re_enable_identity_keys: FeatureVersion,
    pub add_new_non_unique_keys_to_identity: FeatureVersion,
    pub add_new_unique_keys_to_identity: FeatureVersion,
    pub add_new_keys_to_identity: FeatureVersion,
    pub insert_identity_balance: FeatureVersion,
    pub initialize_negative_identity_balance: FeatureVersion,
    pub add_to_identity_balance: FeatureVersion,
    pub add_to_previous_balance: FeatureVersion,
    pub apply_balance_change_from_fee_to_identity: FeatureVersion,
    pub remove_from_identity_balance: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveEstimatedCostsMethodVersions {
    pub add_estimation_costs_for_levels_up_to_contract: FeatureVersion,
    pub add_estimation_costs_for_levels_up_to_contract_document_type_excluded: FeatureVersion,
}
