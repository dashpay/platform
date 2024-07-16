use crate::drive::credit_pools::epochs::epochs_root_tree_key_constants::{
    KEY_PENDING_EPOCH_REFUNDS, KEY_STORAGE_FEE_POOL, KEY_UNPAID_EPOCH_INDEX,
};
use crate::drive::credit_pools::pools_vec_path;
use crate::error::Error;
use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
use crate::util::batch::GroveDbOpBatch;
use dpp::balances::credits::Creditable;
use dpp::block::epoch::EpochIndex;
use dpp::fee::Credits;
use grovedb::batch::GroveDbOp;
use grovedb::Element;

#[cfg(feature = "server")]
/// Adds operations to batch to create pending pool updates tree
pub fn add_create_pending_epoch_refunds_tree_operations(batch: &mut GroveDbOpBatch) {
    batch.add_insert_empty_sum_tree(pools_vec_path(), KEY_PENDING_EPOCH_REFUNDS.to_vec());
}

#[cfg(feature = "server")]
/// Updates the storage fee distribution pool with a new storage fee
pub fn update_storage_fee_distribution_pool_operation(
    storage_fee: Credits,
) -> Result<GroveDbOp, Error> {
    Ok(GroveDbOp::insert_op(
        pools_vec_path(),
        KEY_STORAGE_FEE_POOL.to_vec(),
        Element::new_sum_item(storage_fee.to_signed()?),
    ))
}

#[cfg(feature = "server")]
/// Updates the unpaid epoch index
pub fn update_unpaid_epoch_index_operation(epoch_index: EpochIndex) -> GroveDbOp {
    GroveDbOp::insert_op(
        pools_vec_path(),
        KEY_UNPAID_EPOCH_INDEX.to_vec(),
        Element::new_item(epoch_index.to_be_bytes().to_vec()),
    )
}

#[cfg(test)]
mod tests {
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;

    mod add_create_fee_pool_trees_operations {
        use super::*;
        use crate::error::Error;
        use dpp::block::epoch::Epoch;
        use dpp::fee::epoch::perpetual_storage_epochs;
        use dpp::version::PlatformVersion;

        #[test]
        fn test_values_are_set() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();
            let platform_version = PlatformVersion::latest();

            let storage_fee_pool = drive
                .get_storage_fees_from_distribution_pool(Some(&transaction), platform_version)
                .expect("should get storage fee pool");

            assert_eq!(storage_fee_pool, 0u64);
        }

        #[test]
        fn test_epoch_trees_are_created() {
            let drive = setup_drive_with_initial_state_structure();
            let platform_version = PlatformVersion::latest();
            let transaction = drive.grove.start_transaction();

            let perpetual_storage_epochs = perpetual_storage_epochs(drive.config.epochs_per_era);

            for epoch_index in 0..perpetual_storage_epochs {
                let epoch = Epoch::new(epoch_index).unwrap();

                let storage_fee = drive
                    .get_epoch_storage_credits_for_distribution(
                        &epoch,
                        Some(&transaction),
                        platform_version,
                    )
                    .expect("should get storage fee");

                assert_eq!(storage_fee, 0);
            }

            let epoch = Epoch::new(perpetual_storage_epochs).unwrap(); // 1001th epochs pool

            let result = drive.get_epoch_storage_credits_for_distribution(
                &epoch,
                Some(&transaction),
                platform_version,
            );

            assert!(matches!(result, Err(Error::GroveDB(_))));
        }
    }

    mod update_storage_fee_distribution_pool_operation {
        use super::*;
        use crate::drive::credit_pools::operations;
        use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
        use crate::util::batch::GroveDbOpBatch;
        use dpp::version::PlatformVersion;

        #[test]
        fn test_update_and_get_value() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let platform_version = PlatformVersion::latest();

            let storage_fee = 42;

            let mut batch = GroveDbOpBatch::new();

            batch.push(
                operations::update_storage_fee_distribution_pool_operation(storage_fee)
                    .expect("should return operation"),
            );

            drive
                .grove_apply_batch(batch, false, Some(&transaction), &platform_version.drive)
                .expect("should apply batch");

            let stored_storage_fee = drive
                .get_storage_fees_from_distribution_pool(Some(&transaction), platform_version)
                .expect("should get storage fee pool");

            assert_eq!(storage_fee, stored_storage_fee);
        }
    }
}
