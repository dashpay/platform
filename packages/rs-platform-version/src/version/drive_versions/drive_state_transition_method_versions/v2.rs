use crate::version::drive_versions::drive_state_transition_method_versions::{
    DriveDocumentFromActionVersions,
    DriveStateTransitionActionConvertToHighLevelOperationsMethodVersions,
    DriveStateTransitionMethodVersions, DriveStateTransitionOperationMethodVersions,
};
use crate::version::drive_versions::DriveDataContractOperationMethodVersions;

// This started at protocol 11 (2.2)
pub const DRIVE_STATE_TRANSITION_METHOD_VERSIONS_V2: DriveStateTransitionMethodVersions =
    DriveStateTransitionMethodVersions {
        operations: DriveStateTransitionOperationMethodVersions {
            finalization_tasks: 0,
            contracts: DriveDataContractOperationMethodVersions {
                finalization_tasks: 0,
            },
        },
        convert_to_high_level_operations:
            DriveStateTransitionActionConvertToHighLevelOperationsMethodVersions {
                data_contract_create_transition: 0,
                data_contract_update_transition: 0,
                document_create_transition: 0,
                document_delete_transition: 0,
                document_index_only_delete_transition: 0,
                document_purchase_transition: 0,
                document_replace_transition: 0,
                document_transfer_transition: 0,
                document_update_price_transition: 0,
                token_burn_transition: 0,
                token_mint_transition: 0,
                token_transfer_transition: 0,
                documents_batch_transition: 0,
                identity_create_transition: 0,
                identity_create_from_addresses_transition: 0,
                identity_credit_transfer_transition: 0,
                identity_credit_withdrawal_transition: 0,
                identity_top_up_transition: 0,
                identity_top_up_from_addresses_transition: 0,
                identity_update_transition: 1, //changed
                masternode_vote_transition: 0,
                bump_identity_data_contract_nonce: 0,
                bump_identity_nonce: 0,
                partially_use_asset_lock: 0,
                token_freeze_transition: 0,
                token_unfreeze_transition: 0,
                token_emergency_action_transition: 0,
                token_destroy_frozen_funds_transition: 0,
                token_config_update_transition: 0,
                token_claim_transition: 0,
                token_direct_purchase_transition: 0,
                token_set_price_for_direct_purchase_transition: 0,
                identity_credit_transfer_to_addresses_transition: 0,
                address_funds_transfer_transition: 0,
                address_credit_withdrawal_transition: 0,
                address_funding_from_asset_lock_transition: 0,
                shield_transition: 0,
                shield_from_asset_lock_transition: 0,
                shielded_transfer_transition: 0,
                unshield_transition: 0,
                shielded_withdrawal_transition: 0,
                identity_create_from_shielded_pool_transition: 0,
            },
        document_from_action: DriveDocumentFromActionVersions {
            document_from_create_transition_action: 0,
            document_from_replace_transition_action: 0,
        },
    };
