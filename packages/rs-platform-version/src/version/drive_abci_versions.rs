use crate::version::FeatureVersion;

#[derive(Clone, Copy, Debug, Default)]
pub struct DriveAbciVersion {
    pub methods: DriveAbciMethodVersions,
    pub validation_and_processing: DriveAbciValidationVersions,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DriveAbciMethodVersions {
    pub engine: DriveAbciEngineMethodVersions,
    pub initialization: DriveAbciInitializationMethodVersions,
    pub core_based_updates: DriveAbciCoreBasedUpdatesMethodVersions,
    pub protocol_upgrade: DriveAbciProtocolUpgradeMethodVersions,
    pub block_fee_processing: DriveAbciBlockFeeProcessingMethodVersions,
    pub core_subsidy: DriveAbciCoreSubsidyMethodVersions,
    pub fee_pool_inwards_distribution: DriveAbciFeePoolInwardsDistributionMethodVersions,
    pub fee_pool_outwards_distribution: DriveAbciFeePoolOutwardsDistributionMethodVersions,
    pub identity_credit_withdrawal: DriveAbciIdentityCreditWithdrawalMethodVersions,
    pub state_transition_processing: DriveAbciStateTransitionProcessingMethodVersions,
    pub withdrawals: DriveAbciWithdrawalsMethodVersions,
    pub epoch: DriveAbciEpochMethodVersions,
    pub block_end: DriveAbciBlockEndMethodVersions,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DriveAbciValidationVersions {
    pub state_transitions: DriveAbciStateTransitionValidationVersions,
    pub process_state_transition: FeatureVersion,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DriveAbciDocumentsStateTransitionValidationVersions {
    pub structure: FeatureVersion,
    pub identity_signatures: FeatureVersion,
    pub state: FeatureVersion,
    pub transform_into_action: FeatureVersion,
    pub data_triggers: DriveAbciValidationDataTriggerAndBindingVersions,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DriveAbciValidationDataTriggerAndBindingVersions {
    pub bindings: FeatureVersion,
    pub triggers: DriveAbciValidationDataTriggerVersions,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DriveAbciValidationDataTriggerVersions {
    pub create_contact_request_data_trigger: FeatureVersion,
    pub create_domain_data_trigger: FeatureVersion,
    pub create_identity_data_trigger: FeatureVersion,
    pub create_feature_flag_data_trigger: FeatureVersion,
    pub create_masternode_reward_shares_data_trigger: FeatureVersion,
    pub delete_withdrawal_data_trigger: FeatureVersion,
    pub reject_data_trigger: FeatureVersion,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DriveAbciStateTransitionValidationVersion {
    pub structure: FeatureVersion,
    pub identity_signatures: FeatureVersion,
    pub state: FeatureVersion,
    pub transform_into_action: FeatureVersion,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DriveAbciStateTransitionValidationVersions {
    pub identity_create_state_transition: DriveAbciStateTransitionValidationVersion,
    pub identity_update_state_transition: DriveAbciStateTransitionValidationVersion,
    pub identity_top_up_state_transition: DriveAbciStateTransitionValidationVersion,
    pub identity_credit_withdrawal_state_transition: DriveAbciStateTransitionValidationVersion,
    pub identity_credit_transfer_state_transition: DriveAbciStateTransitionValidationVersion,
    pub contract_create_state_transition: DriveAbciStateTransitionValidationVersion,
    pub contract_update_state_transition: DriveAbciStateTransitionValidationVersion,
    pub documents_batch_state_transition: DriveAbciDocumentsStateTransitionValidationVersions,
    // TODO: We might want to add data triggers to action transitions. BTW, they aren't using atm.
    pub document_base_state_transition: DriveAbciStateTransitionValidationVersion,
    pub document_create_state_transition: DriveAbciStateTransitionValidationVersion,
    pub document_replace_state_transition: DriveAbciStateTransitionValidationVersion,
    pub document_delete_state_transition: DriveAbciStateTransitionValidationVersion,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DriveAbciEngineMethodVersions {
    pub init_chain: FeatureVersion,
    pub check_tx: FeatureVersion,
    pub run_block_proposal: FeatureVersion,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DriveAbciCoreBasedUpdatesMethodVersions {
    pub update_core_info: FeatureVersion,
    pub update_masternode_list: FeatureVersion,
    pub update_quorum_info: FeatureVersion,
    pub masternode_updates: DriveAbciMasternodeIdentitiesUpdatesMethodVersions,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DriveAbciMasternodeIdentitiesUpdatesMethodVersions {
    pub disable_identity_keys: FeatureVersion,
    pub update_masternode_identities: FeatureVersion,
    pub update_operator_identity: FeatureVersion,
    pub update_owner_withdrawal_address: FeatureVersion,
    pub update_voter_identity: FeatureVersion,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DriveAbciInitializationMethodVersions {
    pub initial_core_height: FeatureVersion,
    pub create_genesis_state: FeatureVersion,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DriveAbciBlockFeeProcessingMethodVersions {
    pub add_process_epoch_change_operations: FeatureVersion,
    pub process_block_fees: FeatureVersion,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DriveAbciCoreSubsidyMethodVersions {
    pub epoch_core_reward_credits_for_distribution: FeatureVersion,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DriveAbciFeePoolInwardsDistributionMethodVersions {
    pub add_distribute_block_fees_into_pools_operations: FeatureVersion,
    pub add_distribute_storage_fee_to_epochs_operations: FeatureVersion,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DriveAbciFeePoolOutwardsDistributionMethodVersions {
    pub add_distribute_fees_from_oldest_unpaid_epoch_pool_to_proposers_operations: FeatureVersion,
    pub add_epoch_pool_to_proposers_payout_operations: FeatureVersion,
    pub find_oldest_epoch_needing_payment: FeatureVersion,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DriveAbciEpochMethodVersions {
    pub gather_epoch_info: FeatureVersion,
    pub get_genesis_time: FeatureVersion,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DriveAbciBlockEndMethodVersions {
    pub store_ephemeral_state: FeatureVersion,
    pub update_state_cache: FeatureVersion,
    pub validator_set_update: FeatureVersion,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DriveAbciIdentityCreditWithdrawalMethodVersions {
    pub build_withdrawal_transactions_from_documents: FeatureVersion,
    pub fetch_and_prepare_unsigned_withdrawal_transactions: FeatureVersion,
    pub fetch_core_block_transactions: FeatureVersion,
    pub pool_withdrawals_into_transactions_queue: FeatureVersion,
    pub update_broadcasted_withdrawal_transaction_statuses: FeatureVersion,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DriveAbciProtocolUpgradeMethodVersions {
    pub check_for_desired_protocol_upgrade: FeatureVersion,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DriveAbciStateTransitionProcessingMethodVersions {
    pub execute_event: FeatureVersion,
    pub process_raw_state_transitions: FeatureVersion,
    pub validate_fees_of_event: FeatureVersion,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DriveAbciWithdrawalsMethodVersions {
    pub check_withdrawals: FeatureVersion,
}
