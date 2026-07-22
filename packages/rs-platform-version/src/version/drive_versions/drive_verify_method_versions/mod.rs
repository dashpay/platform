use versioned_feature_core::FeatureVersion;

pub mod v1;

#[derive(Clone, Debug, Default)]
pub struct DriveVerifyMethodVersions {
    pub contract: DriveVerifyContractMethodVersions,
    pub document: DriveVerifyDocumentMethodVersions,
    pub document_count: DriveVerifyDocumentCountMethodVersions,
    pub document_sum: DriveVerifyDocumentSumMethodVersions,
    pub identity: DriveVerifyIdentityMethodVersions,
    pub group: DriveVerifyGroupMethodVersions,
    pub token: DriveVerifyTokenMethodVersions,
    pub single_document: DriveVerifySingleDocumentMethodVersions,
    pub system: DriveVerifySystemMethodVersions,
    pub voting: DriveVerifyVoteMethodVersions,
    pub address_funds: DriveVerifyAddressFundsMethodVersions,
    pub state_transition: DriveVerifyStateTransitionMethodVersions,
    pub shielded: DriveVerifyShieldedMethodVersions,
}

#[derive(Clone, Debug, Default)]
pub struct DriveVerifyShieldedMethodVersions {
    pub verify_shielded_pool_state: FeatureVersion,
    pub verify_shielded_anchors: FeatureVersion,
    pub verify_most_recent_shielded_anchor: FeatureVersion,
    pub verify_shielded_encrypted_notes: FeatureVersion,
    pub verify_shielded_notes_count: FeatureVersion,
    pub verify_shielded_nullifiers: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveVerifyContractMethodVersions {
    pub verify_contract: FeatureVersion,
    pub verify_contract_history: FeatureVersion,
    pub verify_contract_return_serialization: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveVerifyDocumentMethodVersions {
    pub verify_proof: FeatureVersion,
    pub verify_proof_keep_serialized: FeatureVersion,
    pub verify_document_history: FeatureVersion,
    pub verify_start_at_document_in_proof: FeatureVersion,
}

/// Versions for the `GetDocumentsCount` prove-path verifiers
/// (grovedb-level — the tenderdash composition layer lives in
/// rs-drive-proof-verifier). All three methods are implemented on
/// `DriveDocumentCountQuery` and return `(RootHash, T)`.
#[derive(Clone, Debug, Default)]
pub struct DriveVerifyDocumentCountMethodVersions {
    pub verify_aggregate_count_proof: FeatureVersion,
    pub verify_carrier_aggregate_count_proof: FeatureVersion,
    pub verify_distinct_count_proof: FeatureVersion,
    pub verify_point_lookup_count_proof: FeatureVersion,
    pub verify_primary_key_count_tree_proof: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveVerifyDocumentSumMethodVersions {
    pub verify_aggregate_sum_proof: FeatureVersion,
    pub verify_carrier_aggregate_sum_proof: FeatureVersion,
    pub verify_carrier_aggregate_count_and_sum_proof: FeatureVersion,
    pub verify_aggregate_count_and_sum_proof: FeatureVersion,
    pub verify_primary_key_sum_tree_proof: FeatureVersion,
    pub verify_primary_key_count_sum_tree_proof: FeatureVersion,
    pub verify_point_lookup_sum_proof: FeatureVersion,
    pub verify_distinct_sum_proof: FeatureVersion,
    pub verify_distinct_count_and_sum_proof: FeatureVersion,
    pub verify_point_lookup_count_and_sum_proof: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveVerifyIdentityMethodVersions {
    pub verify_full_identities_by_public_key_hashes: FeatureVersion,
    pub verify_full_identity_by_identity_id: FeatureVersion,
    pub verify_full_identity_by_public_key_hash: FeatureVersion,
    pub verify_identity_balance_for_identity_id: FeatureVersion,
    pub verify_identity_balances_for_identity_ids: FeatureVersion,
    pub verify_identity_balance_revision_and_addresses_from_inputs: FeatureVersion,
    pub verify_identity_id_by_unique_public_key_hash: FeatureVersion,
    pub verify_identity_ids_by_unique_public_key_hashes: FeatureVersion,
    pub verify_identity_keys_by_identity_id: FeatureVersion,
    pub verify_identity_nonce: FeatureVersion,
    pub verify_identity_contract_nonce: FeatureVersion,
    pub verify_identities_contract_keys: FeatureVersion,
    pub verify_identity_revision_for_identity_id: FeatureVersion,
    pub verify_full_identity_by_non_unique_public_key_hash: FeatureVersion,
    pub verify_identity_id_by_non_unique_public_key_hash: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveVerifyGroupMethodVersions {
    pub verify_group_info: FeatureVersion,
    pub verify_group_infos_in_contract: FeatureVersion,
    pub verify_action_infos: FeatureVersion,
    pub verify_action_signers: FeatureVersion,
    pub verify_action_signers_total_power: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveVerifyTokenMethodVersions {
    pub verify_token_balances_for_identity_ids: FeatureVersion,
    pub verify_token_balances_for_identity_id: FeatureVersion,
    pub verify_token_infos_for_identity_ids: FeatureVersion,
    pub verify_token_infos_for_identity_id: FeatureVersion,
    pub verify_token_statuses: FeatureVersion,
    pub verify_token_total_supply_and_aggregated_identity_balance: FeatureVersion,
    pub verify_token_balance_for_identity_id: FeatureVersion,
    pub verify_token_info_for_identity_id: FeatureVersion,
    pub verify_token_status: FeatureVersion,
    pub verify_token_pre_programmed_distributions: FeatureVersion,
    pub verify_token_direct_selling_price: FeatureVersion,
    pub verify_token_direct_selling_prices: FeatureVersion,
    pub verify_token_perpetual_distribution_last_paid_time: FeatureVersion,
    pub verify_token_contract_info: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveVerifyVoteMethodVersions {
    pub verify_masternode_vote: FeatureVersion,
    pub verify_vote_poll_votes_proof: FeatureVersion,
    pub verify_identity_votes_given_proof: FeatureVersion,
    pub verify_vote_poll_vote_state_proof: FeatureVersion,
    pub verify_contests_proof: FeatureVersion,
    pub verify_vote_polls_by_end_date_proof: FeatureVersion,
    pub verify_specialized_balance: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveVerifySystemMethodVersions {
    pub verify_epoch_infos: FeatureVersion,
    pub verify_epoch_proposers: FeatureVersion,
    pub verify_elements: FeatureVersion,
    pub verify_total_credits_in_system: FeatureVersion,
    pub verify_upgrade_state: FeatureVersion,
    pub verify_upgrade_vote_status: FeatureVersion,
    pub verify_finalized_epoch_infos: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveVerifySingleDocumentMethodVersions {
    pub verify_proof: FeatureVersion,
    pub verify_proof_keep_serialized: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveVerifyStateTransitionMethodVersions {
    pub verify_state_transition_was_executed_with_proof: FeatureVersion,
    pub verify_state_transition_affected_state_with_proof: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveVerifyAddressFundsMethodVersions {
    pub verify_address_info: FeatureVersion,
    pub verify_addresses_infos: FeatureVersion,
    pub verify_address_funds_trunk_query: FeatureVersion,
    pub verify_address_funds_branch_query: FeatureVersion,
    pub verify_recent_address_balance_changes: FeatureVersion,
    pub verify_compacted_address_balance_changes: FeatureVersion,
}
