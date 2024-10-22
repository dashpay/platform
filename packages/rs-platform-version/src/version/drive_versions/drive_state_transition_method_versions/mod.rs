pub mod v1;

use versioned_feature_core::FeatureVersion;
use crate::version::drive_versions::DriveDataContractOperationMethodVersions;

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
}

#[derive(Clone, Debug, Default)]
pub struct DriveStateTransitionOperationMethodVersions {
    pub finalization_tasks: FeatureVersion,
    pub contracts: DriveDataContractOperationMethodVersions,
}