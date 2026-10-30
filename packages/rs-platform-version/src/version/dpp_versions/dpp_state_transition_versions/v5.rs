use crate::version::dpp_versions::dpp_state_transition_versions::{
    AddressFundsTransitionVersions, ContractTransitionVersions, DPPStateTransitionVersions,
    DocumentTransitionVersions, DocumentsBatchTransitionValidationVersions,
    DocumentsBatchTransitionVersions, IdentityCreditWithdrawalTransitionVersions,
    IdentityTransitionAssetLockVersions, IdentityTransitionVersions, ShieldedTransitionVersions,
};

/// Protocol version 15. Identical to [`super::v4::STATE_TRANSITION_VERSIONS_V4`] except that
/// `max_asset_lock_transaction_inputs` moves from the asset lock transaction structure check to
/// the instant asset lock proof, the one proof that embeds the transaction, whose size the cap
/// bounds: `validate_asset_lock_transaction_structure` 1 no longer counts inputs and
/// `validate_instant_asset_lock_proof_structure` 1 does. A chain asset lock proof, which names
/// only the outpoint, can use a mined asset lock transaction with more inputs.
pub const STATE_TRANSITION_VERSIONS_V5: DPPStateTransitionVersions = DPPStateTransitionVersions {
    documents: DocumentTransitionVersions {
        documents_batch_transition: DocumentsBatchTransitionVersions {
            validation: DocumentsBatchTransitionValidationVersions {
                find_duplicates_by_id: 0,
                validate_base_structure: 0,
            },
        },
    },
    identities: IdentityTransitionVersions {
        max_public_keys_in_creation: 6,
        asset_locks: IdentityTransitionAssetLockVersions {
            required_asset_lock_duff_balance_for_processing_start_for_identity_create: 200000,
            required_asset_lock_duff_balance_for_processing_start_for_identity_top_up: 50000,
            required_asset_lock_duff_balance_for_processing_start_for_address_funding: 50000,
            validate_asset_lock_transaction_structure: 1, // v15: inputs are not counted here
            validate_instant_asset_lock_proof_structure: 1, // v15: the instant proof holds its transaction to the input cap
            max_asset_lock_transaction_inputs: 100,
        },
        credit_withdrawal: IdentityCreditWithdrawalTransitionVersions {
            default_constructor: 1,
        },
        calculate_min_required_fee_on_identity_create_transition: 1, // updated in v3
        calculate_min_required_fee_on_identity_top_up_transition: 1, // updated in v3
    },
    contract: ContractTransitionVersions {
        contract_create_transition_default_version: 0,
        contract_update_transition_default_version: 0,
    },
    address_funds: AddressFundsTransitionVersions {
        address_funds_transition_default_version: 0,
        credit_withdrawal: 0,
        validate_credit_withdrawal_structure: 1, // v14: Core fee-rate cap + fee-inclusive minimum
        min_output_amount: 500_000,
        min_input_amount: 100_000,
        min_identity_funding_amount: 200_000,
    },
    shielded: ShieldedTransitionVersions {
        validate_withdrawal_structure: 1, // v14: Core fee-rate cap + fee-inclusive reserved range
    },
    max_address_inputs: 16,
    max_address_outputs: 128,
    max_address_fee_strategies: 4,
};
