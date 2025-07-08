use crate::version::drive_versions::drive_credit_pool_method_versions::{
    DriveCreditPoolEpochsMethodVersions, DriveCreditPoolMethodVersions,
    DriveCreditPoolPendingEpochRefundsMethodVersions,
    DriveCreditPoolStorageFeeDistributionPoolMethodVersions,
    DriveCreditPoolUnpaidEpochMethodVersions,
};

pub const CREDIT_POOL_METHOD_VERSIONS_V1: DriveCreditPoolMethodVersions =
    DriveCreditPoolMethodVersions {
        epochs: DriveCreditPoolEpochsMethodVersions {
            get_epochs_infos: 0,
            get_epochs_protocol_versions: 0,
            prove_epochs_infos: 0,
            prove_finalized_epoch_infos: 0,
            get_epoch_fee_multiplier: 0,
            get_epoch_processing_credits_for_distribution: 0,
            get_epoch_storage_credits_for_distribution: 0,
            get_epoch_total_credits_for_distribution: 0,
            get_storage_credits_for_distribution_for_epochs_in_range: 0,
            get_epoch_start_time: 0,
            get_epoch_start_block_core_height: 0,
            get_epoch_start_block_height: 0,
            get_first_epoch_start_block_info_between_epochs: 0,
            fetch_epoch_proposers: 0,
            prove_epoch_proposers: 0,
            get_epochs_proposer_block_count: 0,
            add_update_pending_epoch_refunds_operations: 0,
            is_epochs_proposers_tree_empty: 0,
            add_epoch_processing_credits_for_distribution_operation: 0,
            add_epoch_final_info_operation: 0,
            get_epoch_protocol_version: 0,
            get_finalized_epoch_infos: 0,
        },
        pending_epoch_refunds: DriveCreditPoolPendingEpochRefundsMethodVersions {
            add_delete_pending_epoch_refunds_except_specified: 0,
            fetch_and_add_pending_epoch_refunds_to_collection: 0,
            fetch_pending_epoch_refunds: 0,
            add_update_pending_epoch_refunds_operations: 0,
        },
        storage_fee_distribution_pool: DriveCreditPoolStorageFeeDistributionPoolMethodVersions {
            get_storage_fees_from_distribution_pool: 0,
        },
        unpaid_epoch: DriveCreditPoolUnpaidEpochMethodVersions {
            get_unpaid_epoch_index: 0,
        },
    };
