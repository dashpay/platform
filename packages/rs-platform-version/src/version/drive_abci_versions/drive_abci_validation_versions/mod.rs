pub mod v1;
pub mod v2;
pub mod v3;
pub mod v4;
pub mod v5;
pub mod v6;
pub mod v7;

use versioned_feature_core::{FeatureVersion, OptionalFeatureVersion};

#[derive(Clone, Debug, Default)]
pub struct DriveAbciValidationVersions {
    pub state_transitions: DriveAbciStateTransitionValidationVersions,
    pub has_nonce_validation: FeatureVersion,
    pub process_state_transition: FeatureVersion,
    pub state_transition_to_execution_event_for_check_tx: FeatureVersion,
    pub penalties: PenaltyAmounts,
    pub event_constants: DriveAbciValidationConstants,
}

#[derive(Clone, Debug, Default)]
pub struct DriveAbciValidationConstants {
    pub maximum_vote_polls_to_process: u16,
    pub maximum_contenders_to_consider: u16,
}

#[derive(Clone, Debug, Default)]
pub struct DriveAbciStateTransitionValidationVersion {
    pub basic_structure: OptionalFeatureVersion,
    pub advanced_structure: OptionalFeatureVersion,
    pub identity_signatures: OptionalFeatureVersion,
    pub advanced_minimum_balance_pre_check: OptionalFeatureVersion,
    pub nonce: OptionalFeatureVersion,
    pub state: FeatureVersion,
    pub transform_into_action: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveAbciStateTransitionValidationVersions {
    pub common_validation_methods: DriveAbciStateTransitionCommonValidationVersions,
    pub max_asset_lock_usage_attempts: u16,
    pub identity_create_state_transition: DriveAbciStateTransitionValidationVersion,
    pub identity_update_state_transition: DriveAbciStateTransitionValidationVersion,
    pub identity_top_up_state_transition: DriveAbciStateTransitionValidationVersion,
    pub identity_credit_withdrawal_state_transition: DriveAbciStateTransitionValidationVersion,
    pub identity_credit_withdrawal_state_transition_purpose_matches_requirements: FeatureVersion,
    pub identity_credit_transfer_state_transition: DriveAbciStateTransitionValidationVersion,
    pub masternode_vote_state_transition: DriveAbciStateTransitionValidationVersion,
    pub contract_create_state_transition: DriveAbciStateTransitionValidationVersion,
    pub contract_update_state_transition: DriveAbciStateTransitionValidationVersion,
    pub batch_state_transition: DriveAbciDocumentsStateTransitionValidationVersions,
}

#[derive(Clone, Debug, Default)]
pub struct DriveAbciStateTransitionCommonValidationVersions {
    pub asset_locks: DriveAbciAssetLockValidationVersions,
    pub validate_identity_public_key_contract_bounds: FeatureVersion,
    pub validate_identity_public_key_ids_dont_exist_in_state: FeatureVersion,
    pub validate_identity_public_key_ids_exist_in_state: FeatureVersion,
    pub validate_state_transition_identity_signed: FeatureVersion,
    pub validate_unique_identity_public_key_hashes_in_state: FeatureVersion,
    pub validate_master_key_uniqueness: FeatureVersion,
    pub validate_simple_pre_check_balance: FeatureVersion,
    pub validate_non_masternode_identity_exists: FeatureVersion,
    pub validate_identity_exists: FeatureVersion,
}

/// All of these penalty amounts are in credits
#[derive(Clone, Debug, Default)]
pub struct PenaltyAmounts {
    pub identity_id_not_correct: u64,
    pub unique_key_already_present: u64,
    pub validation_of_added_keys_structure_failure: u64,
    pub validation_of_added_keys_proof_of_possession_failure: u64,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DriveAbciAssetLockValidationVersions {
    pub fetch_asset_lock_transaction_output_sync: FeatureVersion,
    pub verify_asset_lock_is_not_spent_and_has_enough_balance: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveAbciDocumentsStateTransitionValidationVersions {
    pub balance_pre_check: FeatureVersion,
    pub basic_structure: FeatureVersion,
    pub advanced_structure: FeatureVersion,
    pub revision: FeatureVersion,
    pub state: FeatureVersion,
    pub transform_into_action: FeatureVersion,
    pub data_triggers: DriveAbciValidationDataTriggerAndBindingVersions,
    pub is_allowed: FeatureVersion,
    pub document_create_transition_structure_validation: FeatureVersion,
    pub document_delete_transition_structure_validation: FeatureVersion,
    pub document_replace_transition_structure_validation: FeatureVersion,
    pub document_transfer_transition_structure_validation: FeatureVersion,
    pub document_purchase_transition_structure_validation: FeatureVersion,
    pub document_update_price_transition_structure_validation: FeatureVersion,
    pub document_base_transition_state_validation: FeatureVersion,
    pub document_create_transition_state_validation: FeatureVersion,
    pub document_delete_transition_state_validation: FeatureVersion,
    pub document_replace_transition_state_validation: FeatureVersion,
    pub document_transfer_transition_state_validation: FeatureVersion,
    pub document_purchase_transition_state_validation: FeatureVersion,
    pub document_update_price_transition_state_validation: FeatureVersion,
    pub token_mint_transition_structure_validation: FeatureVersion,
    pub token_burn_transition_structure_validation: FeatureVersion,
    pub token_transfer_transition_structure_validation: FeatureVersion,
    pub token_mint_transition_state_validation: FeatureVersion,
    pub token_burn_transition_state_validation: FeatureVersion,
    pub token_transfer_transition_state_validation: FeatureVersion,
    pub token_base_transition_structure_validation: FeatureVersion,
    pub token_base_transition_state_validation: FeatureVersion,
    pub token_freeze_transition_structure_validation: FeatureVersion,
    pub token_unfreeze_transition_structure_validation: FeatureVersion,
    pub token_freeze_transition_state_validation: FeatureVersion,
    pub token_unfreeze_transition_state_validation: FeatureVersion,
    pub token_destroy_frozen_funds_transition_structure_validation: FeatureVersion,
    pub token_destroy_frozen_funds_transition_state_validation: FeatureVersion,
    pub token_emergency_action_transition_structure_validation: FeatureVersion,
    pub token_emergency_action_transition_state_validation: FeatureVersion,
    pub token_config_update_transition_structure_validation: FeatureVersion,
    pub token_config_update_transition_state_validation: FeatureVersion,
    pub token_base_transition_group_action_validation: FeatureVersion,
    pub token_claim_transition_structure_validation: FeatureVersion,
    pub token_claim_transition_state_validation: FeatureVersion,
    pub token_direct_purchase_transition_structure_validation: FeatureVersion,
    pub token_direct_purchase_transition_state_validation: FeatureVersion,
    pub token_set_price_for_direct_purchase_transition_structure_validation: FeatureVersion,
    pub token_set_price_for_direct_purchase_transition_state_validation: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveAbciValidationDataTriggerAndBindingVersions {
    pub bindings: FeatureVersion,
    pub triggers: DriveAbciValidationDataTriggerVersions,
}

#[derive(Clone, Debug, Default)]
pub struct DriveAbciValidationDataTriggerVersions {
    pub create_contact_request_data_trigger: FeatureVersion,
    pub create_domain_data_trigger: FeatureVersion,
    pub create_identity_data_trigger: FeatureVersion,
    pub create_feature_flag_data_trigger: FeatureVersion,
    pub create_masternode_reward_shares_data_trigger: FeatureVersion,
    pub delete_withdrawal_data_trigger: FeatureVersion,
    pub reject_data_trigger: FeatureVersion,
}
