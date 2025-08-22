use versioned_feature_core::FeatureVersion;
pub mod v1;

#[derive(Clone, Debug, Default)]
pub struct DriveCreditPoolMethodVersions {
    pub epochs: DriveCreditPoolEpochsMethodVersions,
    pub pending_epoch_refunds: DriveCreditPoolPendingEpochRefundsMethodVersions,
    pub storage_fee_distribution_pool: DriveCreditPoolStorageFeeDistributionPoolMethodVersions,
    pub unpaid_epoch: DriveCreditPoolUnpaidEpochMethodVersions,
}

#[derive(Clone, Debug, Default)]
pub struct DriveCreditPoolEpochsMethodVersions {
    pub get_epochs_infos: FeatureVersion,
    pub get_epochs_protocol_versions: FeatureVersion,
    pub prove_epochs_infos: FeatureVersion,
    pub prove_finalized_epoch_infos: FeatureVersion,
    pub get_epoch_fee_multiplier: FeatureVersion,
    pub get_epoch_processing_credits_for_distribution: FeatureVersion,
    pub get_epoch_storage_credits_for_distribution: FeatureVersion,
    pub get_epoch_total_credits_for_distribution: FeatureVersion,
    pub get_storage_credits_for_distribution_for_epochs_in_range: FeatureVersion,
    pub get_epoch_start_time: FeatureVersion,
    pub get_epoch_start_block_core_height: FeatureVersion,
    pub get_epoch_start_block_height: FeatureVersion,
    pub get_first_epoch_start_block_info_between_epochs: FeatureVersion,
    pub fetch_epoch_proposers: FeatureVersion,
    pub prove_epoch_proposers: FeatureVersion,
    pub get_epochs_proposer_block_count: FeatureVersion,
    pub add_update_pending_epoch_refunds_operations: FeatureVersion,
    pub is_epochs_proposers_tree_empty: FeatureVersion,
    pub add_epoch_processing_credits_for_distribution_operation: FeatureVersion,
    pub add_epoch_final_info_operation: FeatureVersion,
    pub get_epoch_protocol_version: FeatureVersion,
    pub get_finalized_epoch_infos: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveCreditPoolUnpaidEpochMethodVersions {
    pub get_unpaid_epoch_index: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveCreditPoolPendingEpochRefundsMethodVersions {
    pub add_delete_pending_epoch_refunds_except_specified: FeatureVersion,
    pub fetch_and_add_pending_epoch_refunds_to_collection: FeatureVersion,
    pub fetch_pending_epoch_refunds: FeatureVersion,
    pub add_update_pending_epoch_refunds_operations: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveCreditPoolStorageFeeDistributionPoolMethodVersions {
    pub get_storage_fees_from_distribution_pool: FeatureVersion,
}
