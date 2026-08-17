use crate::version::drive_abci_versions::drive_abci_method_versions::{
    DriveAbciBlockEndMethodVersions, DriveAbciBlockFeeProcessingMethodVersions,
    DriveAbciBlockStartMethodVersions, DriveAbciCoreBasedUpdatesMethodVersions,
    DriveAbciCoreChainLockMethodVersionsAndConstants, DriveAbciCoreInstantSendLockMethodVersions,
    DriveAbciEngineMethodVersions, DriveAbciEpochMethodVersions,
    DriveAbciFeePoolInwardsDistributionMethodVersions,
    DriveAbciFeePoolOutwardsDistributionMethodVersions,
    DriveAbciIdentityCreditWithdrawalMethodVersions, DriveAbciInitializationMethodVersions,
    DriveAbciMasternodeIdentitiesUpdatesMethodVersions, DriveAbciMethodVersions,
    DriveAbciPlatformStateStorageMethodVersions, DriveAbciProtocolUpgradeMethodVersions,
    DriveAbciStateTransitionProcessingMethodVersions, DriveAbciTokensProcessingMethodVersions,
    DriveAbciVotingMethodVersions,
};

// Introduced in Protocol version 14.
//
// Identical to DRIVE_ABCI_METHOD_VERSIONS_V9 (the protocol-v13 method set) except
// `process_raw_state_transitions` is bumped from 0 to 1: the v1 outer loop wraps every
// executed state transition in a GroveDB savepoint and rolls back on a result that
// strips the transition from the block (InternalError / UnpaidConsensusError). Under
// v0, execution could write into the shared block transaction before failing
// (apply-then-check fee flow), so a dropped transition's writes leaked into the
// proposer's app hash while the gossiped block omitted the transition — no validator
// could reproduce the hash and the chain stalled (mainnet evo1, 2026-08-14/15, after
// heights 415652 and 415661). Rolling back changes the app hash of any block that
// drops such a transition, so the bump MUST stay inactive on v13 (see
// DRIVE_ABCI_METHOD_VERSIONS_V9, which keeps the field at 0).
pub const DRIVE_ABCI_METHOD_VERSIONS_V10: DriveAbciMethodVersions = DriveAbciMethodVersions {
    engine: DriveAbciEngineMethodVersions {
        init_chain: 0,
        check_tx: 0,
        run_block_proposal: 0,
        finalize_block_proposal: 0,
        consensus_params_update: 1,
    },
    initialization: DriveAbciInitializationMethodVersions {
        initial_core_height_and_time: 0,
        create_genesis_state: 1,
    },
    core_based_updates: DriveAbciCoreBasedUpdatesMethodVersions {
        update_core_info: 0,
        update_masternode_list: 0,
        update_quorum_info: 0,
        masternode_updates: DriveAbciMasternodeIdentitiesUpdatesMethodVersions {
            get_voter_identity_key: 0,
            get_operator_identity_keys: 0,
            get_owner_identity_withdrawal_key: 0,
            get_owner_identity_owner_key: 0,
            get_voter_identifier_from_masternode_list_item: 0,
            get_operator_identifier_from_masternode_list_item: 0,
            create_operator_identity: 0,
            create_owner_identity: 1,
            create_voter_identity: 0,
            disable_identity_keys: 0,
            update_masternode_identities: 0,
            update_operator_identity: 0,
            update_owner_withdrawal_address: 1,
            update_voter_identity: 0,
        },
    },
    protocol_upgrade: DriveAbciProtocolUpgradeMethodVersions {
        check_for_desired_protocol_upgrade: 1,
        upgrade_protocol_version_on_epoch_change: 0,
        perform_events_on_first_block_of_protocol_change: Some(1),
        protocol_version_upgrade_percentage_needed: 67,
    },
    block_fee_processing: DriveAbciBlockFeeProcessingMethodVersions {
        add_process_epoch_change_operations: 0,
        process_block_fees_and_validate_sum_trees: 1,
    },
    tokens_processing: DriveAbciTokensProcessingMethodVersions {
        validate_token_aggregated_balance: 0,
    },
    core_chain_lock: DriveAbciCoreChainLockMethodVersionsAndConstants {
        choose_quorum: 0,
        verify_chain_lock: 0,
        verify_chain_lock_locally: 0,
        verify_chain_lock_through_core: 0,
        make_sure_core_is_synced_to_chain_lock: 0,
        recent_block_count_amount: 2,
    },
    core_instant_send_lock: DriveAbciCoreInstantSendLockMethodVersions {
        verify_recent_signature_locally: 0,
    },
    fee_pool_inwards_distribution: DriveAbciFeePoolInwardsDistributionMethodVersions {
        add_distribute_block_fees_into_pools_operations: 0,
        add_distribute_storage_fee_to_epochs_operations: 0,
    },
    fee_pool_outwards_distribution: DriveAbciFeePoolOutwardsDistributionMethodVersions {
        add_distribute_fees_from_oldest_unpaid_epoch_pool_to_proposers_operations: 1,
        add_epoch_pool_to_proposers_payout_operations: 0,
        find_oldest_epoch_needing_payment: 0,
        fetch_reward_shares_list_for_masternode: 0,
    },
    withdrawals: DriveAbciIdentityCreditWithdrawalMethodVersions {
        build_untied_withdrawal_transactions_from_documents: 0,
        dequeue_and_build_unsigned_withdrawal_transactions: 0,
        fetch_transactions_block_inclusion_status: 0,
        pool_withdrawals_into_transactions_queue: 1,
        update_broadcasted_withdrawal_statuses: 0,
        rebroadcast_expired_withdrawal_documents: 1,
        append_signatures_and_broadcast_withdrawal_transactions: 0,
        cleanup_expired_locks_of_withdrawal_amounts: 0,
    },
    voting: DriveAbciVotingMethodVersions {
        keep_record_of_finished_contested_resource_vote_poll: 0,
        clean_up_after_vote_poll_end: 0,
        clean_up_after_contested_resources_vote_poll_end: 1,
        check_for_ended_vote_polls: 0,
        tally_votes_for_contested_document_resource_vote_poll: 0,
        award_document_to_winner: 0,
        delay_vote_poll: 0,
        run_dao_platform_events: 0,
        remove_votes_for_removed_masternodes: 0,
    },
    state_transition_processing: DriveAbciStateTransitionProcessingMethodVersions {
        execute_event: 0,
        // changed: v1 wraps each executed state transition in a savepoint and rolls back when the
        // result strips the transition from the block, so dropped transitions leave no trace in
        // the app hash. Kept in a real _v1 so _v0 stays byte-identical for pre-v14 nodes.
        process_raw_state_transitions: 1,
        // changed: v1 records the balance effects of paid-INVALID / unsuccessful-paid transitions
        // (charged fees, adjusted outputs, applied chargeable-failure credits) that v0 dropped. Same
        // v13 recorded-set expansion as `record_added_balance_outputs` below; kept in a real _v1 so
        // no version conditional lives inside the _v0 helper.
        process_validation_result: 1,
        decode_raw_state_transitions: 0,
        validate_fees_of_event: 0,
        store_address_balances_to_recent_block_storage: Some(0),
        cleanup_recent_block_storage_address_balances: Some(0),
        // changed: v1 records shielded-spend transparent credits (Unshield net output,
        // ShieldFromAssetLock surplus, identity-create fallback) folded at the executor. The storage
        // method above is unchanged — only which credits feed its map did.
        record_added_balance_outputs: 1,
    },
    epoch: DriveAbciEpochMethodVersions {
        gather_epoch_info: 0,
        get_genesis_time: 0,
    },
    block_start: DriveAbciBlockStartMethodVersions {
        clear_drive_block_cache: 0,
    },
    block_end: DriveAbciBlockEndMethodVersions {
        update_state_cache: 0,
        update_drive_cache: 0,
        validator_set_update: 2,
        should_checkpoint: Some(0),
        update_checkpoints: Some(0),
        record_shielded_pool_anchor: Some(0),
        prune_shielded_pool_anchors: Some(0),
    },
    platform_state_storage: DriveAbciPlatformStateStorageMethodVersions {
        fetch_platform_state: 0,
        store_platform_state: 0,
    },
};
