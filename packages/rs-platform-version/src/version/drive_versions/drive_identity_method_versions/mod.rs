use versioned_feature_core::{FeatureVersion, OptionalFeatureVersion};

pub mod v1;

#[derive(Clone, Debug, Default)]
pub struct DriveIdentityMethodVersions {
    pub fetch: DriveIdentityFetchMethodVersions,
    pub prove: DriveIdentityProveMethodVersions,
    pub keys: DriveIdentityKeysMethodVersions,
    pub update: DriveIdentityUpdateMethodVersions,
    pub insert: DriveIdentityInsertMethodVersions,
    pub contract_info: DriveIdentityContractInfoMethodVersions,
    pub cost_estimation: DriveIdentityCostEstimationMethodVersions,
    pub withdrawals: DriveIdentityWithdrawalMethodVersions,
}

#[derive(Clone, Debug, Default)]
pub struct DriveIdentityWithdrawalMethodVersions {
    pub document: DriveIdentityWithdrawalDocumentMethodVersions,
    pub transaction: DriveIdentityWithdrawalTransactionMethodVersions,
    pub calculate_current_withdrawal_limit: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveIdentityWithdrawalDocumentMethodVersions {
    pub fetch_oldest_withdrawal_documents_by_status: FeatureVersion,
    pub find_withdrawal_documents_by_status_and_transaction_indices: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveIdentityWithdrawalTransactionMethodVersions {
    pub index: DriveIdentityWithdrawalTransactionIndexMethodVersions,
    pub queue: DriveIdentityWithdrawalTransactionQueueMethodVersions,
}

#[derive(Clone, Debug, Default)]
pub struct DriveIdentityWithdrawalTransactionIndexMethodVersions {
    pub fetch_next_withdrawal_transaction_index: FeatureVersion,
    pub add_update_next_withdrawal_transaction_index_operation: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveIdentityWithdrawalTransactionQueueMethodVersions {
    pub add_enqueue_untied_withdrawal_transaction_operations: FeatureVersion,
    pub dequeue_untied_withdrawal_transactions: FeatureVersion,
    pub remove_broadcasted_withdrawal_transactions_after_completion_operations: FeatureVersion,
    pub move_broadcasted_withdrawal_transactions_back_to_queue_operations: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveIdentityContractInfoMethodVersions {
    pub add_potential_contract_info_for_contract_bounded_key: FeatureVersion,
    pub refresh_potential_contract_info_key_references: FeatureVersion,
    pub merge_identity_contract_nonce: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveIdentityCostEstimationMethodVersions {
    pub for_authentication_keys_security_level_in_key_reference_tree: FeatureVersion,
    pub for_balances: FeatureVersion,
    pub for_token_balances: FeatureVersion,
    pub for_token_total_supply: FeatureVersion,
    pub for_contract_info: FeatureVersion,
    pub for_contract_info_group: FeatureVersion,
    pub for_contract_info_group_keys: FeatureVersion,
    pub for_contract_info_group_key_purpose: FeatureVersion,
    pub for_keys_for_identity_id: FeatureVersion,
    pub for_negative_credit: FeatureVersion,
    pub for_purpose_in_key_reference_tree: FeatureVersion,
    pub for_root_key_reference_tree: FeatureVersion,
    pub for_update_revision: FeatureVersion,
    pub for_token_identity_infos: FeatureVersion,
    pub for_token_pre_programmed_distribution: FeatureVersion,
    pub for_root_token_ms_interval_distribution: FeatureVersion,
    pub for_token_perpetual_distribution: FeatureVersion,
    pub for_token_selling_prices: FeatureVersion,
    pub for_token_contract_infos: FeatureVersion,
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
    pub fetch_full_identity_by_non_unique_public_key_hash: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveIdentityFetchAttributesMethodVersions {
    pub revision: FeatureVersion,
    pub nonce: FeatureVersion,
    pub identity_contract_nonce: FeatureVersion,
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
    pub fetch_identity_revision_with_keys: FeatureVersion,
    pub fetch_identity_balance_with_keys: FeatureVersion,
    pub fetch_identity_balance_with_keys_and_revision: FeatureVersion,
    pub fetch_identity_with_balance: FeatureVersion,
    pub fetch_identity_keys: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveIdentityProveMethodVersions {
    pub full_identity: FeatureVersion,
    pub full_identities: FeatureVersion,
    pub identity_nonce: FeatureVersion,
    pub identity_contract_nonce: FeatureVersion,
    pub identities_contract_keys: FeatureVersion,
    pub prove_full_identities_by_unique_public_key_hashes: FeatureVersion,
    pub prove_full_identity_by_unique_public_key_hash: FeatureVersion,
    pub prove_identity_id_by_unique_public_key_hash: FeatureVersion,
    pub prove_identity_ids_by_unique_public_key_hashes: FeatureVersion,
    pub prove_full_identity_by_non_unique_public_key_hash: FeatureVersion,
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
    pub fetch_identities_contract_keys: FeatureVersion,
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
    pub merge_identity_nonce: FeatureVersion,
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
    pub refresh_identity_key_reference_operations: FeatureVersion,
}
