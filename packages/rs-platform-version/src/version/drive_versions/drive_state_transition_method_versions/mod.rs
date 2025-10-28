pub mod v1;

use crate::version::drive_versions::DriveDataContractOperationMethodVersions;
use versioned_feature_core::FeatureVersion;

#[derive(Clone, Debug, Default)]
pub struct DriveStateTransitionMethodVersions {
    pub operations: DriveStateTransitionOperationMethodVersions,
    pub convert_to_high_level_operations:
        DriveStateTransitionActionConvertToHighLevelOperationsMethodVersions,
}

#[derive(Clone, Debug, Default)]
pub struct DriveStateTransitionActionConvertToHighLevelOperationsMethodVersions {
    pub data_contract_create_transition: FeatureVersion,
    pub data_contract_update_transition: FeatureVersion,
    pub document_create_transition: FeatureVersion,
    pub document_delete_transition: FeatureVersion,
    pub document_purchase_transition: FeatureVersion,
    pub document_replace_transition: FeatureVersion,
    pub document_transfer_transition: FeatureVersion,
    pub document_update_price_transition: FeatureVersion,
    pub token_burn_transition: FeatureVersion,
    pub token_mint_transition: FeatureVersion,
    pub token_transfer_transition: FeatureVersion,
    pub documents_batch_transition: FeatureVersion,
    pub identity_create_transition: FeatureVersion,
    pub identity_credit_transfer_transition: FeatureVersion,
    pub identity_credit_withdrawal_transition: FeatureVersion,
    pub identity_top_up_transition: FeatureVersion,
    pub identity_update_transition: FeatureVersion,
    pub masternode_vote_transition: FeatureVersion,
    pub bump_identity_data_contract_nonce: FeatureVersion,
    pub bump_identity_nonce: FeatureVersion,
    pub partially_use_asset_lock: FeatureVersion,
    pub token_freeze_transition: FeatureVersion,
    pub token_unfreeze_transition: FeatureVersion,
    pub token_emergency_action_transition: FeatureVersion,
    pub token_destroy_frozen_funds_transition: FeatureVersion,
    pub token_config_update_transition: FeatureVersion,
    pub token_claim_transition: FeatureVersion,
    pub token_direct_purchase_transition: FeatureVersion,
    pub token_set_price_for_direct_purchase_transition: FeatureVersion,
    pub identity_credit_transfer_to_addresses_transition: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveStateTransitionOperationMethodVersions {
    pub finalization_tasks: FeatureVersion,
    pub contracts: DriveDataContractOperationMethodVersions,
}
