pub mod v1;
pub mod v2;
pub mod v3;
pub mod v4;
pub mod v5;
pub mod v6;
pub mod v7;
pub mod v8;
pub mod v9;

use versioned_feature_core::{FeatureVersion, OptionalFeatureVersion};

#[derive(Clone, Debug, Default)]
pub struct DriveAbciValidationVersions {
    pub state_transitions: DriveAbciStateTransitionValidationVersions,
    pub has_nonce_validation: FeatureVersion,
    pub has_address_witness_validation: FeatureVersion,
    pub validate_address_witnesses: FeatureVersion,
    pub validate_shielded_proof: FeatureVersion,
    pub validate_minimum_shielded_fee: FeatureVersion,
    pub process_state_transition: FeatureVersion,
    pub state_transition_to_execution_event_for_check_tx: FeatureVersion,
    pub penalties: PenaltyAmounts,
    pub event_constants: DriveAbciValidationConstants,
}

#[derive(Clone, Debug, Default)]
pub struct DriveAbciValidationConstants {
    pub maximum_vote_polls_to_process: u16,
    pub maximum_contenders_to_consider: u16,
    /// Minimum number of encrypted notes in the shielded pool before outgoing
    /// transitions (Unshield, ShieldedWithdrawal) are allowed. This ensures a
    /// sufficient anonymity set before funds can leave the pool.
    pub minimum_pool_notes_for_outgoing: u64,
    /// Number of blocks of anchors to retain. Anchors older than this are
    /// pruned at the end of each block. Clients must use an anchor no older
    /// than this many blocks when building shielded transactions.
    pub shielded_anchor_retention_blocks: u64,
    /// Anchor pruning is only performed every N blocks to avoid unnecessary
    /// GroveDB work on every block. Must evenly divide
    /// `shielded_anchor_retention_blocks`.
    pub shielded_anchor_pruning_interval: u64,
    /// Per-bundle fee (in credits) for Halo 2 ZK proof verification.
    /// Benchmarked at ~30x per-action signature verification cost.
    pub shielded_proof_verification_fee: u64,
    /// Per-action fee (in credits) for processing: RedPallas spend auth signature
    /// verification, nullifier duplicate check, and tree insertion.
    pub shielded_per_action_processing_fee: u64,
    /// Maximum surplus (in credits) that a `ShieldFromAssetLock` may implicitly
    /// donate to the fee pools when no `surplus_output` address is set. Above this
    /// cap the transition is rejected so a client cannot accidentally forfeit a
    /// large asset-lock remainder. 20,000,000,000 credits = 0.2 Dash.
    pub shielded_implicit_fee_cap: u64,
    /// Allowed exit denominations (in credits) for `IdentityCreateFromShieldedPool`.
    /// 0.1, 0.3, 0.5, 1.0 DASH = {10, 30, 50, 100} × 10^9 credits. The exit amount is
    /// restricted to this small fixed set so every identity-creation exit of a given size
    /// is indistinguishable on-chain, maximizing the anonymity set (mirroring the exact-fee
    /// uniformity already enforced for `ShieldedTransfer`). Empty pre-v12 so the transition
    /// is gated off until the shielded family activates.
    pub shielded_identity_create_denominations: &'static [u64],
}

#[derive(Clone, Debug, Default)]
pub struct DriveAbciStateTransitionValidationVersion {
    pub basic_structure: OptionalFeatureVersion,
    pub advanced_structure: OptionalFeatureVersion,
    pub identity_signatures: OptionalFeatureVersion,
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
    pub identity_credit_transfer_to_addresses_state_transition:
        DriveAbciStateTransitionValidationVersion,
    pub masternode_vote_state_transition: DriveAbciStateTransitionValidationVersion,
    pub masternode_vote_state_transition_balance_pre_check: FeatureVersion,
    pub contract_create_state_transition: DriveAbciStateTransitionValidationVersion,
    pub contract_update_state_transition: DriveAbciStateTransitionValidationVersion,
    pub batch_state_transition: DriveAbciDocumentsStateTransitionValidationVersions,
    pub identity_create_from_addresses_state_transition: DriveAbciStateTransitionValidationVersion,
    pub identity_top_up_from_addresses_state_transition: DriveAbciStateTransitionValidationVersion,

    pub address_credit_withdrawal: DriveAbciStateTransitionValidationVersion,
    pub address_funds_from_asset_lock: DriveAbciStateTransitionValidationVersion,
    pub address_funds_transfer: DriveAbciStateTransitionValidationVersion,

    pub shield_state_transition: DriveAbciStateTransitionValidationVersion,
    pub shielded_transfer_state_transition: DriveAbciStateTransitionValidationVersion,
    pub unshield_state_transition: DriveAbciStateTransitionValidationVersion,
    pub shield_from_asset_lock_state_transition: DriveAbciStateTransitionValidationVersion,
    pub shielded_withdrawal_state_transition: DriveAbciStateTransitionValidationVersion,
    pub identity_create_from_shielded_pool_state_transition:
        DriveAbciStateTransitionValidationVersion,
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
    /// Penalty for address funding with insufficient funds for outputs
    pub address_funds_insufficient_balance: u64,
    /// Penalty for submitting a shield transition with an invalid ZK proof
    pub shielded_proof_verification_failure: u64,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DriveAbciAssetLockValidationVersions {
    pub fetch_asset_lock_transaction_output_sync: FeatureVersion,
    pub verify_asset_lock_is_not_spent_and_has_enough_balance: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveAbciDocumentsStateTransitionValidationVersions {
    pub basic_structure: FeatureVersion,
    pub advanced_structure: FeatureVersion,
    pub revision: FeatureVersion,
    pub state: FeatureVersion,
    pub transform_into_action: FeatureVersion,
    /// Versions the action emitted when a per-transition validation fails
    /// inside [`transform_document_transition`].
    ///
    /// - `0` (PROTOCOL_VERSION_11 and below): errors-only, no action data.
    ///   The empty action flowed through the legacy
    ///   `flatten` / `merge_many` aggregators as `Some(empty_vec)` and was
    ///   accounted as `PaidConsensusError`, but no `BumpIdentityDataContractNonce`
    ///   drive op was created — so the user only paid the bare-bump fee
    ///   and the contract nonce never advanced.
    /// - `1` (PROTOCOL_VERSION_12+): emit a `BumpIdentityDataContractNonce`
    ///   action so the user pays for the validation work that already ran
    ///   (fetch + ownership/revision check) and the contract nonce advances.
    ///
    /// [`transform_document_transition`]: crate
    pub failed_per_transition_action: FeatureVersion,
    /// Versions the
    /// `fetch_documents_for_transitions_knowing_contract_and_document_type`
    /// helper. v0 (PROTOCOL_VERSION_11 and below) passes `epoch=None`
    /// to `query_documents` and doesn't bill the cost. v1
    /// (PROTOCOL_VERSION_12+) passes `Some(epoch)` and bills via
    /// `execution_context.add_operation`.
    pub fetch_documents_for_transitions_knowing_contract_and_document_type: FeatureVersion,
    /// Versions the `fetch_document_with_id` helper. Same v0 vs v1
    /// semantics as
    /// `fetch_documents_for_transitions_knowing_contract_and_document_type`.
    pub fetch_document_with_id: FeatureVersion,
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
