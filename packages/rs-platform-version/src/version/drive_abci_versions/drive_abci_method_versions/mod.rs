use versioned_feature_core::{FeatureVersion, OptionalFeatureVersion};

pub mod v1;
pub mod v2;
pub mod v3;
pub mod v4;
pub mod v5;
pub mod v6;

#[derive(Clone, Debug, Default)]
pub struct DriveAbciMethodVersions {
    pub engine: DriveAbciEngineMethodVersions,
    pub initialization: DriveAbciInitializationMethodVersions,
    pub core_based_updates: DriveAbciCoreBasedUpdatesMethodVersions,
    pub protocol_upgrade: DriveAbciProtocolUpgradeMethodVersions,
    pub block_fee_processing: DriveAbciBlockFeeProcessingMethodVersions,
    pub tokens_processing: DriveAbciTokensProcessingMethodVersions,
    pub core_chain_lock: DriveAbciCoreChainLockMethodVersionsAndConstants,
    pub core_instant_send_lock: DriveAbciCoreInstantSendLockMethodVersions,
    pub fee_pool_inwards_distribution: DriveAbciFeePoolInwardsDistributionMethodVersions,
    pub fee_pool_outwards_distribution: DriveAbciFeePoolOutwardsDistributionMethodVersions,
    pub withdrawals: DriveAbciIdentityCreditWithdrawalMethodVersions,
    pub voting: DriveAbciVotingMethodVersions,
    pub state_transition_processing: DriveAbciStateTransitionProcessingMethodVersions,
    pub epoch: DriveAbciEpochMethodVersions,
    pub block_start: DriveAbciBlockStartMethodVersions,
    pub block_end: DriveAbciBlockEndMethodVersions,
    pub platform_state_storage: DriveAbciPlatformStateStorageMethodVersions,
    pub platform_reduced_state_storage: DriveAbciReducedPlatformStateStorageMethodVersions,
    pub last_block_info_storage: DriveAbciLastBlockInfoStorageMethodVersions,
}

#[derive(Clone, Debug, Default)]
pub struct DriveAbciPlatformStateStorageMethodVersions {
    pub fetch_platform_state: FeatureVersion,
    pub store_platform_state: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveAbciReducedPlatformStateStorageMethodVersions {
    pub fetch_reduced_platform_state: FeatureVersion,
    pub store_reduced_platform_state: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveAbciLastBlockInfoStorageMethodVersions {
    pub fetch_last_block_info: FeatureVersion,
    pub store_last_block_info: FeatureVersion,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DriveAbciEngineMethodVersions {
    pub init_chain: FeatureVersion,
    pub check_tx: FeatureVersion,
    pub run_block_proposal: FeatureVersion,
    pub finalize_block_proposal: FeatureVersion,
    pub consensus_params_update: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveAbciCoreBasedUpdatesMethodVersions {
    pub update_core_info: FeatureVersion,
    pub update_masternode_list: FeatureVersion,
    pub update_quorum_info: FeatureVersion,
    pub masternode_updates: DriveAbciMasternodeIdentitiesUpdatesMethodVersions,
}

#[derive(Clone, Debug, Default)]
pub struct DriveAbciMasternodeIdentitiesUpdatesMethodVersions {
    pub get_voter_identity_key: FeatureVersion,
    pub get_operator_identity_keys: FeatureVersion,
    pub get_owner_identity_withdrawal_key: FeatureVersion,
    pub get_owner_identity_owner_key: FeatureVersion,
    pub get_voter_identifier_from_masternode_list_item: FeatureVersion,
    pub get_operator_identifier_from_masternode_list_item: FeatureVersion,
    pub create_operator_identity: FeatureVersion,
    pub create_owner_identity: FeatureVersion,
    pub create_voter_identity: FeatureVersion,
    pub disable_identity_keys: FeatureVersion,
    pub update_masternode_identities: FeatureVersion,
    pub update_operator_identity: FeatureVersion,
    pub update_owner_withdrawal_address: FeatureVersion,
    pub update_voter_identity: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveAbciInitializationMethodVersions {
    pub initial_core_height_and_time: FeatureVersion,
    pub create_genesis_state: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveAbciBlockFeeProcessingMethodVersions {
    pub add_process_epoch_change_operations: FeatureVersion,
    pub process_block_fees_and_validate_sum_trees: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveAbciTokensProcessingMethodVersions {
    pub validate_token_aggregated_balance: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveAbciCoreInstantSendLockMethodVersions {
    pub verify_recent_signature_locally: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveAbciCoreChainLockMethodVersionsAndConstants {
    pub choose_quorum: FeatureVersion,
    pub verify_chain_lock: FeatureVersion,
    pub verify_chain_lock_locally: FeatureVersion,
    pub verify_chain_lock_through_core: FeatureVersion,
    pub make_sure_core_is_synced_to_chain_lock: FeatureVersion,
    pub recent_block_count_amount: u32, //what constitutes a recent block, for v0 it's 2.
}

#[derive(Clone, Debug, Default)]
pub struct DriveAbciFeePoolInwardsDistributionMethodVersions {
    pub add_distribute_block_fees_into_pools_operations: FeatureVersion,
    pub add_distribute_storage_fee_to_epochs_operations: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveAbciFeePoolOutwardsDistributionMethodVersions {
    pub add_distribute_fees_from_oldest_unpaid_epoch_pool_to_proposers_operations: FeatureVersion,
    pub add_epoch_pool_to_proposers_payout_operations: FeatureVersion,
    pub find_oldest_epoch_needing_payment: FeatureVersion,
    pub fetch_reward_shares_list_for_masternode: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveAbciEpochMethodVersions {
    pub gather_epoch_info: FeatureVersion,
    pub get_genesis_time: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveAbciBlockStartMethodVersions {
    pub clear_drive_block_cache: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveAbciBlockEndMethodVersions {
    pub update_state_cache: FeatureVersion,
    pub update_drive_cache: FeatureVersion,
    pub validator_set_update: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveAbciVotingMethodVersions {
    pub keep_record_of_finished_contested_resource_vote_poll: FeatureVersion,
    pub clean_up_after_vote_poll_end: FeatureVersion,
    pub clean_up_after_contested_resources_vote_poll_end: FeatureVersion,
    pub check_for_ended_vote_polls: FeatureVersion,
    pub tally_votes_for_contested_document_resource_vote_poll: FeatureVersion,
    pub award_document_to_winner: FeatureVersion,
    pub delay_vote_poll: FeatureVersion,
    pub run_dao_platform_events: FeatureVersion,
    pub remove_votes_for_removed_masternodes: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveAbciIdentityCreditWithdrawalMethodVersions {
    pub build_untied_withdrawal_transactions_from_documents: FeatureVersion,
    pub dequeue_and_build_unsigned_withdrawal_transactions: FeatureVersion,
    pub fetch_transactions_block_inclusion_status: FeatureVersion,
    pub pool_withdrawals_into_transactions_queue: FeatureVersion,
    pub update_broadcasted_withdrawal_statuses: FeatureVersion,
    pub rebroadcast_expired_withdrawal_documents: FeatureVersion,
    pub append_signatures_and_broadcast_withdrawal_transactions: FeatureVersion,
    pub cleanup_expired_locks_of_withdrawal_amounts: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveAbciProtocolUpgradeMethodVersions {
    pub check_for_desired_protocol_upgrade: FeatureVersion,
    pub upgrade_protocol_version_on_epoch_change: FeatureVersion,
    pub perform_events_on_first_block_of_protocol_change: OptionalFeatureVersion,
    pub protocol_version_upgrade_percentage_needed: u64,
}

#[derive(Clone, Debug, Default)]
pub struct DriveAbciStateTransitionProcessingMethodVersions {
    pub execute_event: FeatureVersion,
    pub process_raw_state_transitions: FeatureVersion,
    pub decode_raw_state_transitions: FeatureVersion,
    pub validate_fees_of_event: FeatureVersion,
}
