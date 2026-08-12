use crate::version::drive_abci_versions::drive_abci_validation_versions::{
    DriveAbciAssetLockValidationVersions, DriveAbciDocumentsStateTransitionValidationVersions,
    DriveAbciStateTransitionCommonValidationVersions, DriveAbciStateTransitionValidationVersion,
    DriveAbciStateTransitionValidationVersions, DriveAbciValidationConstants,
    DriveAbciValidationDataTriggerAndBindingVersions, DriveAbciValidationDataTriggerVersions,
    DriveAbciValidationVersions, PenaltyAmounts,
};

// PROTOCOL_VERSION_13: enable DPNS username transfers and sales and activate
// the token authorization/configuration validators that bind confirmations to
// stored intent and resolve group authority fail-closed. Also revises the
// shielded identity-create exit-denomination set (adds 0.03 and 0.25 DASH,
// retires 0.3 DASH). v8 remains unchanged for PROTOCOL_VERSION_12 chain
// replay.
pub const DRIVE_ABCI_VALIDATION_VERSIONS_V9: DriveAbciValidationVersions =
    DriveAbciValidationVersions {
        state_transitions: DriveAbciStateTransitionValidationVersions {
            common_validation_methods: DriveAbciStateTransitionCommonValidationVersions {
                asset_locks: DriveAbciAssetLockValidationVersions {
                    fetch_asset_lock_transaction_output_sync: 0,
                    verify_asset_lock_is_not_spent_and_has_enough_balance: 0,
                },
                validate_identity_public_key_contract_bounds: 1,
                validate_identity_public_key_ids_dont_exist_in_state: 0,
                validate_identity_public_key_ids_exist_in_state: 0,
                validate_state_transition_identity_signed: 0,
                validate_unique_identity_public_key_hashes_in_state: 1,
                validate_master_key_uniqueness: 0,
                validate_payment_key_uniqueness: 0,
                validate_non_masternode_identity_exists: 0,
                validate_identity_exists: 0,
            },
            max_asset_lock_usage_attempts: 16,
            identity_create_state_transition: DriveAbciStateTransitionValidationVersion {
                basic_structure: Some(0),
                advanced_structure: Some(0),
                identity_signatures: Some(0),
                nonce: None,
                state: 0,
                transform_into_action: 0,
            },
            identity_update_state_transition: DriveAbciStateTransitionValidationVersion {
                basic_structure: Some(0),
                advanced_structure: Some(0),
                identity_signatures: Some(0),
                nonce: Some(0),
                state: 0,
                transform_into_action: 0,
            },
            identity_top_up_state_transition: DriveAbciStateTransitionValidationVersion {
                basic_structure: Some(0),
                advanced_structure: None,
                identity_signatures: None,
                nonce: None,
                state: 0,
                transform_into_action: 0,
            },
            identity_credit_withdrawal_state_transition:
                DriveAbciStateTransitionValidationVersion {
                    // v1 adds config min_version enforcement: since protocol version 12, V0 config is no longer
                    // accepted because it lacks sized_integer_types support.
                    basic_structure: Some(1),
                    advanced_structure: None,
                    identity_signatures: None,
                    nonce: Some(0),
                    state: 0,
                    transform_into_action: 0,
                },
            identity_credit_withdrawal_state_transition_purpose_matches_requirements: 0,
            identity_credit_transfer_state_transition: DriveAbciStateTransitionValidationVersion {
                basic_structure: Some(0),
                advanced_structure: None,
                identity_signatures: None,
                nonce: Some(0),
                state: 0,
                transform_into_action: 0,
            },
            identity_credit_transfer_to_addresses_state_transition:
                DriveAbciStateTransitionValidationVersion {
                    basic_structure: Some(0),
                    advanced_structure: None,
                    identity_signatures: None,
                    nonce: Some(0),
                    state: 0,
                    transform_into_action: 0,
                },
            masternode_vote_state_transition: DriveAbciStateTransitionValidationVersion {
                basic_structure: None,
                advanced_structure: Some(0),
                identity_signatures: None,
                nonce: Some(1),
                state: 0,
                transform_into_action: 0,
            },
            masternode_vote_state_transition_balance_pre_check: 0,
            contract_create_state_transition: DriveAbciStateTransitionValidationVersion {
                basic_structure: Some(1),
                advanced_structure: Some(1),
                identity_signatures: None,
                nonce: Some(0),
                state: 0,
                transform_into_action: 0,
            },
            contract_update_state_transition: DriveAbciStateTransitionValidationVersion {
                basic_structure: Some(1),
                advanced_structure: None,
                identity_signatures: None,
                nonce: Some(0),
                state: 0,
                transform_into_action: 0,
            },
            batch_state_transition: DriveAbciDocumentsStateTransitionValidationVersions {
                basic_structure: 0,
                advanced_structure: 0,
                state: 0,
                revision: 0,
                // PROTOCOL_VERSION_12 (v3.1 hard fork): batch state transition
                // fee accounting fixes. This single field gates multiple
                // related billing changes so they all activate together at
                // the same hard fork. On v0 every behavior below is the
                // legacy under-billing, preserved verbatim for
                // PROTOCOL_VERSION_11 chain replay.
                //
                // Gated by `transform_into_action: 1`:
                //   * B7 — outer `execution_context` is threaded through the
                //     batch transformer (was a dropped local) so per-
                //     transition fee_results in `try_into_action_v0` are
                //     billed.
                //   * B4 — `query_documents` cost in
                //     `fetch_documents_for_transitions_knowing_contract_and_document_type`
                //     is added to `execution_context`.
                //   * B5 — `query_documents` cost in `fetch_document_with_id`
                //     is added to `execution_context`.
                //   * T1 — DPNS data trigger parent-domain
                //     `query_documents` cost.
                //   * T2 — DPNS data trigger preorder `query_documents` cost.
                //   * T3 — DashPay data trigger recipient identity-balance
                //     fetch cost (switched to `fetch_identity_balance_with_costs`).
                //   * T4 — withdrawals data trigger `query_documents` cost.
                transform_into_action: 1,
                // PROTOCOL_VERSION_12 (v3.1 hard fork): per-transition
                // failure paths in `transform_document_transition` now emit
                // a `BumpIdentityDataContractNonce` action so the user pays
                // for the validation work that already ran (fetch +
                // ownership/revision check). v0 stays for chain
                // reproducibility on PROTOCOL_VERSION_11 and below.
                failed_per_transition_action: 1,
                // PROTOCOL_VERSION_12 (v3.1 hard fork): fetch_documents
                // helpers bumped to v1 which bill the grovedb cost of
                // their query_documents calls. v0 stays for PV11 chain
                // replay (the v0 helpers pass epoch=None and never call
                // add_operation — byte-identical to pre-PR behavior).
                fetch_documents_for_transitions_knowing_contract_and_document_type: 1,
                fetch_document_with_id: 1,
                data_triggers: DriveAbciValidationDataTriggerAndBindingVersions {
                    // PROTOCOL_VERSION_13: v1 drops the reject bindings for
                    // Transfer, Purchase and UpdatePrice on DPNS `domain`
                    // documents, enabling username transfers and sales.
                    bindings: 1,
                    triggers: DriveAbciValidationDataTriggerVersions {
                        // PROTOCOL_VERSION_12 (v3.1 hard fork): triggers
                        // that perform drive reads now have `_v1` versions
                        // that bill the cost via add_operation on the
                        // outer execution_context. v0 versions remain
                        // byte-identical to PV11 (don't bill).
                        create_contact_request_data_trigger: 1,
                        validate_profile_payment_addresses_data_trigger: 0,
                        create_domain_data_trigger: 1,
                        create_identity_data_trigger: 0,
                        create_feature_flag_data_trigger: 0,
                        create_masternode_reward_shares_data_trigger: 0,
                        delete_withdrawal_data_trigger: 1,
                        // Reject does no drive reads — stays at v0.
                        reject_data_trigger: 0,
                    },
                },
                is_allowed: 0,
                document_create_transition_structure_validation: 0,
                document_delete_transition_structure_validation: 0,
                document_replace_transition_structure_validation: 0,
                document_transfer_transition_structure_validation: 0,
                document_purchase_transition_structure_validation: 0,
                document_update_price_transition_structure_validation: 0,
                document_base_transition_state_validation: 0,
                document_create_transition_state_validation: 1,
                document_delete_transition_state_validation: 0,
                document_replace_transition_state_validation: 0,
                document_transfer_transition_state_validation: 0,
                document_purchase_transition_state_validation: 0,
                document_update_price_transition_state_validation: 0,
                token_mint_transition_structure_validation: 0,
                token_burn_transition_structure_validation: 0,
                token_transfer_transition_structure_validation: 0,
                token_mint_transition_state_validation: 0,
                token_burn_transition_state_validation: 0,
                token_transfer_transition_state_validation: 0,
                token_base_transition_structure_validation: 0,
                token_base_transition_state_validation: 1,
                token_freeze_transition_structure_validation: 0,
                token_unfreeze_transition_structure_validation: 0,
                token_freeze_transition_state_validation: 0,
                token_unfreeze_transition_state_validation: 0,
                token_destroy_frozen_funds_transition_structure_validation: 0,
                token_destroy_frozen_funds_transition_state_validation: 0,
                token_emergency_action_transition_structure_validation: 0,
                token_emergency_action_transition_state_validation: 0,
                token_config_update_transition_structure_validation: 0,
                token_config_update_transition_state_validation: 1,
                token_base_transition_group_action_validation: 1,
                token_claim_transition_structure_validation: 0,
                token_claim_transition_state_validation: 0,
                token_direct_purchase_transition_structure_validation: 0,
                token_direct_purchase_transition_state_validation: 0,
                token_set_price_for_direct_purchase_transition_structure_validation: 0,
                token_set_price_for_direct_purchase_transition_state_validation: 0,
            },
            identity_create_from_addresses_state_transition:
                DriveAbciStateTransitionValidationVersion {
                    basic_structure: Some(0),
                    advanced_structure: Some(0),
                    identity_signatures: Some(0),
                    nonce: Some(0),
                    state: 0,
                    transform_into_action: 0,
                },
            identity_top_up_from_addresses_state_transition:
                DriveAbciStateTransitionValidationVersion {
                    basic_structure: Some(0),
                    advanced_structure: None,
                    identity_signatures: None,
                    nonce: Some(0),
                    state: 0,
                    transform_into_action: 0,
                },
            address_credit_withdrawal: DriveAbciStateTransitionValidationVersion {
                basic_structure: Some(0),
                advanced_structure: None,
                identity_signatures: None,
                nonce: Some(0),
                state: 0,
                transform_into_action: 0,
            },
            address_funds_from_asset_lock: DriveAbciStateTransitionValidationVersion {
                basic_structure: Some(0),
                advanced_structure: Some(0),
                identity_signatures: None,
                nonce: Some(0),
                state: 0,
                transform_into_action: 0,
            },
            address_funds_transfer: DriveAbciStateTransitionValidationVersion {
                basic_structure: Some(0),
                advanced_structure: None,
                identity_signatures: None,
                nonce: Some(0),
                state: 0,
                transform_into_action: 0,
            },
            shield_state_transition: DriveAbciStateTransitionValidationVersion {
                basic_structure: Some(0),
                advanced_structure: None,
                identity_signatures: None,
                nonce: None,
                state: 0,
                transform_into_action: 0,
            },
            shielded_transfer_state_transition: DriveAbciStateTransitionValidationVersion {
                basic_structure: Some(0),
                advanced_structure: None,
                identity_signatures: None,
                nonce: None,
                state: 0,
                transform_into_action: 0,
            },
            unshield_state_transition: DriveAbciStateTransitionValidationVersion {
                basic_structure: Some(0),
                advanced_structure: None,
                identity_signatures: None,
                nonce: None,
                state: 0,
                transform_into_action: 0,
            },
            shield_from_asset_lock_state_transition: DriveAbciStateTransitionValidationVersion {
                basic_structure: Some(0),
                advanced_structure: None,
                identity_signatures: None,
                nonce: None,
                state: 0,
                transform_into_action: 0,
            },
            shielded_withdrawal_state_transition: DriveAbciStateTransitionValidationVersion {
                basic_structure: Some(0),
                advanced_structure: None,
                identity_signatures: None,
                nonce: None,
                state: 0,
                transform_into_action: 0,
            },
            identity_create_from_shielded_pool_state_transition:
                DriveAbciStateTransitionValidationVersion {
                    basic_structure: Some(0),
                    advanced_structure: None,
                    identity_signatures: None,
                    nonce: None,
                    state: 0,
                    transform_into_action: 0,
                },
        },
        has_nonce_validation: 1,
        has_address_witness_validation: 0,
        validate_address_witnesses: 0,
        validate_shielded_proof: 0,
        validate_minimum_shielded_fee: 0,
        process_state_transition: 0,
        state_transition_to_execution_event_for_check_tx: 0,
        penalties: PenaltyAmounts {
            identity_id_not_correct: 50000000,
            unique_key_already_present: 10000000,
            validation_of_added_keys_structure_failure: 10000000,
            validation_of_added_keys_proof_of_possession_failure: 50000000,
            address_funds_insufficient_balance: 10000000,
            shielded_proof_verification_failure: 50000000,
        },
        event_constants: DriveAbciValidationConstants {
            maximum_vote_polls_to_process: 2,
            maximum_contenders_to_consider: 100,
            minimum_pool_notes_for_outgoing: 250,
            shielded_anchor_retention_blocks: 1000,
            shielded_anchor_pruning_interval: 100,
            shielded_proof_verification_fee: 100_000_000,
            // Per-action processing prices the ~1.1 ms/action Halo 2 verification CPU at the
            // same rate the flat fee prices the ~5 ms base (100M ≈ 4.5× this), so the fee
            // tracks the per-action cost and the margin stays uniform as actions grow.
            shielded_per_action_processing_fee: 22_000_000,
            shielded_implicit_fee_cap: 20_000_000_000,
            // 0.1, 0.3, 0.5, 1.0 DASH in credits (1 DASH = 10^8 duffs, CREDITS_PER_DUFF = 1000).
            // v13 revises the v8 set: adds 0.03 and 0.25 DASH, retires 0.3 DASH.
            shielded_identity_create_denominations: &[
                3_000_000_000,   // 0.03 DASH
                10_000_000_000,  // 0.1 DASH
                25_000_000_000,  // 0.25 DASH
                50_000_000_000,  // 0.5 DASH
                100_000_000_000, // 1 DASH
            ],
        },
    };
