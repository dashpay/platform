// MIT LICENSE
//
// Copyright (c) 2021 Dash Core Group
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
//

use crate::drive::batch::GroveDbOpBatch;
use crate::drive::fee_pools::pools_vec_path;
use crate::error::Error;
use crate::fee::credits::{Creditable, Credits};
use crate::fee::epoch::{EpochIndex, GENESIS_EPOCH_INDEX, PERPETUAL_STORAGE_EPOCHS};
use crate::fee_pools::epochs::Epoch;
use crate::fee_pools::epochs_root_tree_key_constants::{
    KEY_PENDING_EPOCH_REFUNDS, KEY_STORAGE_FEE_POOL, KEY_UNPAID_EPOCH_INDEX,
};
use grovedb::batch::GroveDbOp;
use grovedb::Element;

/// Epochs module
pub mod epochs;
/// Epochs root tree key constants module
pub mod epochs_root_tree_key_constants;

/// Adds the operations to groveDB op batch to create the fee pool trees
pub fn add_create_fee_pool_trees_operations(batch: &mut GroveDbOpBatch) -> Result<(), Error> {
    // Init storage credit pool
    batch.push(update_storage_fee_distribution_pool_operation(0)?);

    // Init next epoch to pay
    batch.push(update_unpaid_epoch_index_operation(GENESIS_EPOCH_INDEX));

    add_create_pending_epoch_refunds_tree_operations(batch);

    // We need to insert 50 years worth of epochs,
    // with 20 epochs per year that's 1000 epochs
    for i in GENESIS_EPOCH_INDEX..PERPETUAL_STORAGE_EPOCHS {
        let epoch = Epoch::new(i);
        epoch.add_init_empty_operations(batch)?;
    }

    Ok(())
}

/// Adds operations to batch to create pending pool updates tree
pub fn add_create_pending_epoch_refunds_tree_operations(batch: &mut GroveDbOpBatch) {
    batch.add_insert_empty_sum_tree(pools_vec_path(), KEY_PENDING_EPOCH_REFUNDS.to_vec());
}

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

/// Updates the unpaid epoch index
pub fn update_unpaid_epoch_index_operation(epoch_index: EpochIndex) -> GroveDbOp {
    GroveDbOp::insert_op(
        pools_vec_path(),
        KEY_UNPAID_EPOCH_INDEX.to_vec(),
        Element::new_item(epoch_index.to_be_bytes().to_vec()),
    )
}

// TODD: Find tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::helpers::setup::setup_drive_with_initial_state_structure;

    mod add_create_fee_pool_trees_operations {
        use super::*;

        #[test]
        fn test_values_are_set() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let storage_fee_pool = drive
                .get_storage_fees_from_distribution_pool(Some(&transaction))
                .expect("should get storage fee pool");

            assert_eq!(storage_fee_pool, 0u64);
        }

        #[test]
        fn test_epoch_trees_are_created() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            for epoch_index in 0..1000 {
                let epoch = Epoch::new(epoch_index);

                let storage_fee = drive
                    .get_epoch_storage_credits_for_distribution(&epoch, Some(&transaction))
                    .expect("should get storage fee");

                assert_eq!(storage_fee, 0);
            }

            let epoch = Epoch::new(1000); // 1001th epochs pool

            let result =
                drive.get_epoch_storage_credits_for_distribution(&epoch, Some(&transaction));

            assert!(matches!(result, Err(Error::GroveDB(_))));
        }
    }

    mod update_storage_fee_distribution_pool_operation {
        use super::*;

        #[test]
        fn test_update_and_get_value() {
            let drive = setup_drive_with_initial_state_structure();
            let transaction = drive.grove.start_transaction();

            let storage_fee = 42;

            let mut batch = GroveDbOpBatch::new();

            batch.push(
                update_storage_fee_distribution_pool_operation(storage_fee)
                    .expect("should return operation"),
            );

            drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let stored_storage_fee = drive
                .get_storage_fees_from_distribution_pool(Some(&transaction))
                .expect("should get storage fee pool");

            assert_eq!(storage_fee, stored_storage_fee);
        }
    }
}
