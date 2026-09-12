use crate::version::dpp_versions::dpp_state_transition_versions::{
    AddressFundsTransitionVersions, ContractTransitionVersions, DPPStateTransitionVersions,
    DocumentTransitionVersions, DocumentsBatchTransitionValidationVersions,
    DocumentsBatchTransitionVersions, IdentityCreditWithdrawalTransitionVersions,
    IdentityTransitionAssetLockVersions, IdentityTransitionVersions,
    IdentityUpdateTransitionVersions,
};

pub const STATE_TRANSITION_VERSIONS_V2: DPPStateTransitionVersions = DPPStateTransitionVersions {
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
            validate_asset_lock_transaction_structure: 0,
            validate_instant_asset_lock_proof_structure: 0,
            max_asset_lock_transaction_inputs: u16::MAX,
        },
        credit_withdrawal: IdentityCreditWithdrawalTransitionVersions {
            default_constructor: 1,
        },
        identity_update: IdentityUpdateTransitionVersions {
            basic_structure: Some(0),
        },
        calculate_min_required_fee_on_identity_create_transition: 0,
        calculate_min_required_fee_on_identity_top_up_transition: 0,
    },
    contract: ContractTransitionVersions {
        contract_create_transition_default_version: 0,
        contract_update_transition_default_version: 0,
    },
    address_funds: AddressFundsTransitionVersions {
        address_funds_transition_default_version: 0,
        credit_withdrawal: 0,
        min_output_amount: 500_000,
        min_input_amount: 100_000,
        min_identity_funding_amount: 200_000,
    },
    max_address_inputs: 16,
    max_address_outputs: 128,
    max_address_fee_strategies: 4,
};
