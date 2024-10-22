use crate::version::dpp_versions::dpp_state_transition_versions::{
    DPPStateTransitionVersions, DocumentTransitionVersions,
    DocumentsBatchTransitionValidationVersions, DocumentsBatchTransitionVersions,
    IdentityCreditWithdrawalTransitionVersions, IdentityTransitionAssetLockVersions,
    IdentityTransitionVersions,
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
            validate_asset_lock_transaction_structure: 0,
            validate_instant_asset_lock_proof_structure: 0,
        },
        credit_withdrawal: IdentityCreditWithdrawalTransitionVersions {
            default_constructor: 1,
        },
    },
};
